mod commands;
mod hardware;
mod model;
mod traits;

use anyhow::Result;
use futures::future::FutureExt;
use futures::{Stream, StreamExt, pin_mut, stream};
use ipnet::IpNet;
use rand::seq::SliceRandom;
use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use std::collections::HashSet;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time::timeout;

use super::commands::MinerCommand;
use super::util::{send_rpc_command, send_web_command};
use crate::data::device::{MinerFirmware, MinerMake, MinerModel};
use crate::miners::backends::antminer::AntMiner;
use crate::miners::backends::avalonminer::AvalonMiner;
use crate::miners::backends::bitaxe::Bitaxe;
use crate::miners::backends::braiins::Braiins;
use crate::miners::backends::epic::PowerPlay;
use crate::miners::backends::luxminer::LuxMiner;
use crate::miners::backends::marathon::Marathon;
use crate::miners::backends::nerdaxe::NerdAxe;
use crate::miners::backends::traits::*;
use crate::miners::backends::vnish::Vnish;
use crate::miners::backends::whatsminer::WhatsMiner;
use crate::miners::factory::traits::VersionSelection;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use traits::{DiscoveryCommands, ModelSelection};

const IDENTIFICATION_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECTIVITY_TIMEOUT: Duration = Duration::from_secs(1);
const CONNECTIVITY_RETRIES: u32 = 3;

fn calculate_optimal_concurrency(ip_count: usize) -> usize {
    // Adaptive concurrency based on scale
    match ip_count {
        0..=1000 => 1000,     // Medium networks - moderate
        1001..=5000 => 2500,  // Large networks - aggressive
        5001..=10000 => 5000, // Very large networks - high throughput
        _ => 10000,           // Massive mining operations - maximum throughput
    }
}

/// Fast port connectivity check with TCP optimizations
async fn check_port_open(ip: IpAddr, port: u16, connectivity_timeout: Duration) -> bool {
    let addr: SocketAddr = (ip, port).into();

    let stream = match timeout(connectivity_timeout, TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        _ => return false,
    };

    // disable Nagle's algorithm for immediate transmission
    let _ = stream.set_nodelay(true);

    true
}

async fn get_miner_type_from_command(
    ip: IpAddr,
    command: MinerCommand,
) -> Option<(Option<MinerMake>, Option<MinerFirmware>)> {
    match command {
        MinerCommand::RPC {
            command,
            parameters: _,
        } => {
            let response = send_rpc_command(&ip, command).await?;
            parse_type_from_socket(response)
        }
        MinerCommand::WebAPI {
            command,
            parameters: _,
        } => {
            let response = send_web_command(&ip, command).await?;
            parse_type_from_web(response)
        }
        _ => None,
    }
}

fn parse_type_from_socket(
    response: serde_json::Value,
) -> Option<(Option<MinerMake>, Option<MinerFirmware>)> {
    let json_string = response.to_string().to_uppercase();
    match () {
        _ if json_string.contains("BOSMINER") || json_string.contains("BOSER") => {
            Some((None, Some(MinerFirmware::BraiinsOS)))
        }
        _ if json_string.contains("LUXMINER") => Some((None, Some(MinerFirmware::LuxOS))),
        _ if json_string.contains("MARAFW") || json_string.contains("KAONSU") => {
            Some((None, Some(MinerFirmware::Marathon)))
        }
        _ if json_string.contains("VNISH") => Some((None, Some(MinerFirmware::VNish))),
        _ if json_string.contains("BITMICRO") || json_string.contains("BTMINER") => {
            Some((Some(MinerMake::WhatsMiner), Some(MinerFirmware::Stock)))
        }
        _ if json_string.contains("ANTMINER") => {
            Some((Some(MinerMake::AntMiner), Some(MinerFirmware::Stock)))
        }
        _ if json_string.contains("AVALON") => {
            Some((Some(MinerMake::AvalonMiner), Some(MinerFirmware::Stock)))
        }
        _ => None,
    }
}

fn parse_type_from_web(
    response: (String, HeaderMap, StatusCode),
) -> Option<(Option<MinerMake>, Option<MinerFirmware>)> {
    let (resp_text, resp_headers, resp_status) = response;
    let auth_header = match resp_headers.get("www-authenticate") {
        Some(header) => header.to_str().unwrap(),
        None => "",
    };
    let algo_header = match resp_headers.get("algorithm") {
        Some(header) => header.to_str().unwrap(),
        None => "",
    };
    let redirect_header = match resp_headers.get("location") {
        Some(header) => header.to_str().unwrap(),
        None => "",
    };
    match () {
        _ if resp_status == 401 && algo_header.contains("MD5") => {
            Some((None, Some(MinerFirmware::Marathon)))
        }
        _ if resp_status == 401 && auth_header.contains("realm=\"antMiner") => {
            Some((Some(MinerMake::AntMiner), Some(MinerFirmware::Stock)))
        }
        _ if resp_text.contains("Braiins OS") => Some((None, Some(MinerFirmware::BraiinsOS))),
        _ if resp_text.contains("Luxor Firmware") => Some((None, Some(MinerFirmware::LuxOS))),
        _ if resp_text.contains("Nerd") => {
            Some((Some(MinerMake::NerdAxe), Some(MinerFirmware::Stock)))
        }
        _ if resp_text.contains("AxeOS") => {
            Some((Some(MinerMake::Bitaxe), Some(MinerFirmware::Stock)))
        }
        _ if resp_text.contains("Miner Web Dashboard") => Some((None, Some(MinerFirmware::EPic))),
        _ if resp_text.contains("Avalon") => {
            Some((Some(MinerMake::AvalonMiner), Some(MinerFirmware::Stock)))
        }
        _ if resp_text.contains("AnthillOS") => Some((None, Some(MinerFirmware::VNish))),
        _ if redirect_header.contains("https://") && resp_status == 307
            || resp_text.contains("/cgi-bin/luci") =>
        {
            Some((Some(MinerMake::WhatsMiner), Some(MinerFirmware::Stock)))
        }
        _ => None,
    }
}

#[tracing::instrument(level = "debug")]
pub fn select_backend(
    ip: IpAddr,
    model: MinerModel,
    firmware: Option<MinerFirmware>,
    version: Option<semver::Version>,
) -> Option<Box<dyn Miner>> {
    match (model, firmware) {
        (MinerModel::WhatsMiner(_), Some(MinerFirmware::Stock)) => {
            Some(WhatsMiner::new(ip, model, version))
        }
        (MinerModel::Bitaxe(_), Some(MinerFirmware::Stock)) => {
            Some(Bitaxe::new(ip, model, version))
        }
        (MinerModel::NerdAxe(_), Some(MinerFirmware::Stock)) => {
            Some(NerdAxe::new(ip, model, version))
        }
        (MinerModel::AvalonMiner(_), Some(MinerFirmware::Stock)) => {
            Some(AvalonMiner::new(ip, model, version))
        }
        (MinerModel::AntMiner(_), Some(MinerFirmware::Stock)) => {
            Some(AntMiner::new(ip, model, version))
        }
        (_, Some(MinerFirmware::VNish)) => Some(Vnish::new(ip, model, version)),
        (_, Some(MinerFirmware::EPic)) => Some(PowerPlay::new(ip, model, version)),
        (_, Some(MinerFirmware::Marathon)) => Some(Marathon::new(ip, model, version)),
        (_, Some(MinerFirmware::LuxOS)) => Some(LuxMiner::new(ip, model, version)),
        (_, Some(MinerFirmware::BraiinsOS)) => Some(Braiins::new(ip, model, version)),
        _ => {
            tracing::debug!("no valid backend found");
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct MinerFactory {
    search_makes: Option<Vec<MinerMake>>,
    search_firmwares: Option<Vec<MinerFirmware>>,
    ips: Vec<IpAddr>,
    identification_timeout: Duration,
    connectivity_timeout: Duration,
    connectivity_retries: u32,
    concurrent: Option<usize>,
    check_port: bool,
}

impl Default for MinerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl MinerFactory {
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn scan_miner(&self, ip: IpAddr) -> Result<Option<Box<dyn Miner>>> {
        // Quick port check first to avoid wasting time on dead IPs
        if (1..self.connectivity_retries).next().is_some() {
            if !self.check_port {
                return self.get_miner(ip).await;
            }
            // Check for web UI
            if check_port_open(ip, 80, self.connectivity_timeout).await {
                return self.get_miner(ip).await;
            }
            // Check for CGMiner RPC API
            if check_port_open(ip, 4028, self.connectivity_timeout).await {
                return self.get_miner(ip).await;
            }
            // Check for alternate CGMiner RPC API
            if check_port_open(ip, 4029, self.connectivity_timeout).await {
                return self.get_miner(ip).await;
            }
            // Check for whatsminer tool API
            if check_port_open(ip, 8889, self.connectivity_timeout).await {
                return self.get_miner(ip).await;
            }
        }
        tracing::trace!("no response from any miner-specific ports");
        Ok(None)
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn get_miner(&self, ip: IpAddr) -> Result<Option<Box<dyn Miner>>> {
        let search_makes = self.search_makes.clone().unwrap_or(vec![
            MinerMake::AntMiner,
            MinerMake::WhatsMiner,
            MinerMake::AvalonMiner,
            MinerMake::EPic,
            MinerMake::Braiins,
            MinerMake::Bitaxe,
            MinerMake::NerdAxe,
        ]);
        let search_firmwares = self.search_firmwares.clone().unwrap_or(vec![
            MinerFirmware::Stock,
            MinerFirmware::BraiinsOS,
            MinerFirmware::VNish,
            MinerFirmware::EPic,
            MinerFirmware::HiveOS,
            MinerFirmware::LuxOS,
            MinerFirmware::Marathon,
        ]);
        let mut commands: HashSet<MinerCommand> = HashSet::new();

        for make in search_makes {
            for command in make.get_discovery_commands() {
                commands.insert(command);
            }
        }
        for firmware in search_firmwares {
            for command in firmware.get_discovery_commands() {
                commands.insert(command);
            }
        }

        let mut discovery_tasks = JoinSet::new();
        for command in commands {
            let _ = discovery_tasks.spawn(get_miner_type_from_command(ip, command));
        }

        let timeout = tokio::time::sleep(self.identification_timeout).fuse();
        pin_mut!(timeout);

        let mut found: Option<(Option<MinerMake>, Option<MinerFirmware>)> = None;

        loop {
            if discovery_tasks.is_empty() {
                break;
            }

            tokio::select! {
                _ = &mut timeout => break,
                r = discovery_tasks.join_next() => {
                    match r.unwrap_or(Ok(None)) {
                        Ok(Some(result @ (_, Some(fw)))) if fw != MinerFirmware::Stock => {
                            found = Some(result);
                            break;
                        }
                        Ok(Some(result)) => {
                            found = Some(result);
                            break;
                        }
                        _ => continue,
                    }
                }
            }
        }

        if let Some((_make, Some(MinerFirmware::Stock))) = found {
            let upgrade_window = tokio::time::sleep(Duration::from_millis(300)).fuse();
            pin_mut!(upgrade_window);

            loop {
                if discovery_tasks.is_empty() {
                    break;
                }
                tokio::select! {
                    _ = &mut timeout => break,
                    _ = &mut upgrade_window => break,
                    r = discovery_tasks.join_next() => {
                        if let Ok(Some(result @ (_, Some(fw)))) = r.unwrap_or(Ok(None))
                            && fw != MinerFirmware::Stock {
                                found = Some(result);
                                break;
                            }
                    }
                }
            }
        }

        discovery_tasks.abort_all();
        while discovery_tasks.join_next().await.is_some() {}

        let miner_info = found;

        match miner_info {
            Some((Some(make), Some(MinerFirmware::Stock))) => {
                let model = make.get_model(ip).await?;
                let version = make.get_version(ip).await;

                Ok(select_backend(
                    ip,
                    model,
                    Some(MinerFirmware::Stock),
                    version,
                ))
            }
            Some((_, Some(firmware))) => {
                let model = firmware.get_model(ip).await?;
                let version = firmware.get_version(ip).await;

                Ok(select_backend(ip, model, Some(firmware), version))
            }
            Some((Some(make), firmware)) => {
                let model = make.get_model(ip).await?;
                let version = make.get_version(ip).await;

                Ok(select_backend(ip, model, firmware, version))
            }
            _ => {
                tracing::debug!("failed to identify {ip}");
                Ok(None)
            }
        }
    }

    pub fn new() -> MinerFactory {
        MinerFactory {
            search_makes: None,
            search_firmwares: None,
            ips: Vec::new(),
            identification_timeout: IDENTIFICATION_TIMEOUT,
            connectivity_timeout: CONNECTIVITY_TIMEOUT,
            connectivity_retries: CONNECTIVITY_RETRIES,
            concurrent: None,
            check_port: true, // Enable port checking by default
        }
    }

    // Port checking
    pub fn with_port_check(mut self, enabled: bool) -> Self {
        self.check_port = enabled;
        self
    }

    // Concurrency limiting
    pub fn with_concurrent_limit(mut self, limit: usize) -> Self {
        self.concurrent = Some(limit);
        self
    }

    pub fn with_adaptive_concurrency(mut self) -> Self {
        self.concurrent = Some(calculate_optimal_concurrency(self.ips.len()));
        self
    }

    pub fn update_adaptive_concurrency(&mut self) {
        if self.concurrent.is_none() {
            self.concurrent = Some(calculate_optimal_concurrency(self.ips.len()));
        }
    }

    // Timeout
    pub fn with_identification_timeout(mut self, timeout: Duration) -> Self {
        self.identification_timeout = timeout;
        self
    }

    pub fn with_identification_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.identification_timeout = Duration::from_secs(timeout_secs);
        self
    }

    pub fn with_connectivity_timeout(mut self, timeout: Duration) -> Self {
        self.connectivity_timeout = timeout;
        self
    }

    pub fn with_connectivity_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.connectivity_timeout = Duration::from_secs(timeout_secs);
        self
    }

    pub fn with_connectivity_retries(mut self, retries: u32) -> Self {
        self.connectivity_retries = retries;
        self
    }

    // Makes
    pub fn with_search_makes(mut self, search_makes: Vec<MinerMake>) -> Self {
        self.search_makes = Some(search_makes);
        self
    }

    pub fn with_makes(mut self, makes: Vec<MinerMake>) -> Self {
        self.search_makes = Some(makes);
        self
    }

    pub fn add_search_make(mut self, search_make: MinerMake) -> Self {
        if self.search_makes.is_none() {
            self.search_makes = Some(vec![search_make]);
        } else {
            self.search_makes.as_mut().unwrap().push(search_make);
        }
        self
    }

    pub fn remove_search_make(mut self, search_make: MinerMake) -> Self {
        if let Some(makes) = self.search_makes.as_mut() {
            makes.retain(|val| *val != search_make);
        }
        self
    }

    // Firmwares
    pub fn with_search_firmwares(mut self, search_firmwares: Vec<MinerFirmware>) -> Self {
        self.search_firmwares = Some(search_firmwares);
        self
    }

    pub fn with_firmwares(mut self, firmwares: Vec<MinerFirmware>) -> Self {
        self.search_firmwares = Some(firmwares);
        self
    }

    pub fn add_search_firmware(mut self, search_firmware: MinerFirmware) -> Self {
        if self.search_firmwares.is_none() {
            self.search_firmwares = Some(vec![search_firmware]);
        } else {
            self.search_firmwares
                .as_mut()
                .unwrap()
                .push(search_firmware);
        }
        self
    }

    pub fn remove_search_firmware(mut self, search_firmware: MinerFirmware) -> Self {
        if let Some(firmwares) = self.search_firmwares.as_mut() {
            firmwares.retain(|val| *val != search_firmware);
        }
        self
    }

    // Subnet handlers
    /// Create a new `MinerFactory` with a subnet
    pub fn from_subnet(subnet: &str) -> Result<Self> {
        Self::new().with_subnet(subnet)
    }

    /// Add a subnet to the IP range
    pub fn with_subnet(mut self, subnet: &str) -> Result<Self> {
        let ips = self.hosts_from_subnet(subnet)?;
        self.ips.extend(ips);
        self.shuffle_ips();
        Ok(self)
    }

    /// Set the subnet range to use, removing all other IPs
    pub fn set_subnet(&mut self, subnet: &str) -> Result<&Self> {
        let ips = self.hosts_from_subnet(subnet)?;
        self.ips = ips;
        self.shuffle_ips();
        Ok(self)
    }

    fn hosts_from_subnet(&self, subnet: &str) -> Result<Vec<IpAddr>> {
        let network = IpNet::from_str(subnet)?;
        Ok(network.hosts().collect())
    }

    /// Randomize IP order to avoid bursts to a single switch/segment
    fn shuffle_ips(&mut self) {
        let mut rng = rand::rng();
        self.ips.shuffle(&mut rng);
    }

    // Octet handlers
    /// Create a new `MinerFactory` with an octet range
    pub fn from_octets(octet1: &str, octet2: &str, octet3: &str, octet4: &str) -> Result<Self> {
        Self::new().with_octets(octet1, octet2, octet3, octet4)
    }

    /// Add an octet range to the IP range
    pub fn with_octets(
        mut self,
        octet1: &str,
        octet2: &str,
        octet3: &str,
        octet4: &str,
    ) -> Result<Self> {
        let ips = self.hosts_from_octets(octet1, octet2, octet3, octet4)?;
        self.ips.extend(ips);
        self.shuffle_ips();
        Ok(self)
    }

    /// Set the octet range to use, removing all other IPs
    pub fn set_octets(
        &mut self,
        octet1: &str,
        octet2: &str,
        octet3: &str,
        octet4: &str,
    ) -> Result<&Self> {
        let ips = self.hosts_from_octets(octet1, octet2, octet3, octet4)?;
        self.ips = ips;
        self.shuffle_ips();
        Ok(self)
    }

    fn hosts_from_octets(
        &self,
        octet1: &str,
        octet2: &str,
        octet3: &str,
        octet4: &str,
    ) -> Result<Vec<IpAddr>> {
        let octet1_range = parse_octet_range(octet1)?;
        let octet2_range = parse_octet_range(octet2)?;
        let octet3_range = parse_octet_range(octet3)?;
        let octet4_range = parse_octet_range(octet4)?;

        Ok(generate_ips_from_ranges(
            &octet1_range,
            &octet2_range,
            &octet3_range,
            &octet4_range,
        ))
    }

    // Range handlers
    /// Create a new `MinerFactory` with a range string in the format "10.1-199.0.1-199"
    pub fn from_range(range_str: &str) -> Result<Self> {
        Self::new().with_range(range_str)
    }

    /// Add a range string in the format "10.1-199.0.1-199"
    pub fn with_range(mut self, range_str: &str) -> Result<Self> {
        let ips = self.hosts_from_range(range_str)?;
        self.ips.extend(ips);
        self.shuffle_ips();
        Ok(self)
    }

    /// Set the range string in the format "10.1-199.0.1-199", replacing all other IPs
    pub fn set_range(&mut self, range_str: &str) -> Result<&Self> {
        let ips = self.hosts_from_range(range_str)?;
        self.ips = ips;
        self.shuffle_ips();
        Ok(self)
    }

    fn hosts_from_range(&self, range_str: &str) -> Result<Vec<IpAddr>> {
        let parts: Vec<&str> = range_str.split('.').collect();
        if parts.len() != 4 {
            return Err(anyhow::anyhow!(
                "Invalid IP range format. Expected format: 10.1-199.0.1-199"
            ));
        }

        let octet1_range = parse_octet_range(parts[0])?;
        let octet2_range = parse_octet_range(parts[1])?;
        let octet3_range = parse_octet_range(parts[2])?;
        let octet4_range = parse_octet_range(parts[3])?;

        Ok(generate_ips_from_ranges(
            &octet1_range,
            &octet2_range,
            &octet3_range,
            &octet4_range,
        ))
    }

    /// Return current scan IPs
    pub fn hosts(&self) -> Vec<IpAddr> {
        self.ips.clone()
    }

    /// Get current count of scan IPs
    pub fn len(&self) -> usize {
        self.ips.len()
    }

    /// Check if the list of IPs is empty
    pub fn is_empty(&self) -> bool {
        self.ips.is_empty()
    }

    /// Scan the IPs specified in the factory
    pub async fn scan(&self) -> Result<Vec<Box<dyn Miner>>> {
        if self.ips.is_empty() {
            return Err(anyhow::anyhow!(
                "No IPs to scan. Use with_subnet, with_octets, or with_range to set IPs."
            ));
        }

        let concurrency = self
            .concurrent
            .unwrap_or(calculate_optimal_concurrency(self.ips.len()));

        let miners: Vec<Box<dyn Miner>> = stream::iter(self.ips.iter().copied())
            .map(|ip| async move { self.scan_miner(ip).await.ok().flatten() })
            .buffer_unordered(concurrency)
            .filter_map(|miner_opt| async move { miner_opt })
            .collect()
            .await;

        Ok(miners)
    }

    pub fn scan_stream(&self) -> Pin<Box<impl Stream<Item = Box<dyn Miner>> + Send + use<>>> {
        let concurrency = self
            .concurrent
            .unwrap_or(calculate_optimal_concurrency(self.ips.len()));

        let factory = Arc::new(self.clone());
        let ips: Arc<[IpAddr]> = Arc::from(self.ips.as_slice());

        let ip_count = ips.len();
        let stream = stream::iter(0..ip_count)
            .map(move |i| {
                let factory = Arc::clone(&factory);
                let ips = Arc::clone(&ips);
                async move { factory.scan_miner(ips[i]).await.ok().flatten() }
            })
            .buffer_unordered(concurrency)
            .filter_map(|miner_opt| async move { miner_opt });

        Box::pin(stream)
    }

    pub fn scan_stream_with_ip(
        &self,
    ) -> Pin<Box<impl Stream<Item = (IpAddr, Option<Box<dyn Miner>>)> + Send + use<>>> {
        let concurrency = self
            .concurrent
            .unwrap_or(calculate_optimal_concurrency(self.ips.len()));

        let factory = Arc::new(self.clone());
        let ips: Arc<[IpAddr]> = Arc::from(self.ips.as_slice());

        let ip_count = ips.len();
        let stream = stream::iter(0..ip_count)
            .map(move |i| {
                let factory = Arc::clone(&factory);
                let ips = Arc::clone(&ips);
                async move { (ips[i], factory.scan_miner(ips[i]).await.ok().flatten()) }
            })
            .buffer_unordered(concurrency);

        Box::pin(stream)
    }

    /// Scan for miners by specific octets
    pub async fn scan_by_octets(
        self,
        octet1: &str,
        octet2: &str,
        octet3: &str,
        octet4: &str,
    ) -> Result<Vec<Box<dyn Miner>>> {
        self.with_octets(octet1, octet2, octet3, octet4)?
            .scan()
            .await
    }

    /// Scan for miners by IP range in the format "10.1-199.0.1-199"
    pub async fn scan_by_range(self, range_str: &str) -> Result<Vec<Box<dyn Miner>>> {
        self.with_range(range_str)?.scan().await
    }
}

/// Helper function to parse an octet range string like "1-199" into a vector of u8 values
fn parse_octet_range(range_str: &str) -> Result<Vec<u8>> {
    if range_str.contains('-') {
        let parts: Vec<&str> = range_str.split('-').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid range format: {}", range_str));
        }

        let start: u8 = parts[0].parse()?;
        let end: u8 = parts[1].parse()?;

        if start > end {
            return Err(anyhow::anyhow!(
                "Invalid range: start > end in {}",
                range_str
            ));
        }

        Ok((start..=end).collect())
    } else {
        // Single value
        let value: u8 = range_str.parse()?;
        Ok(vec![value])
    }
}

/// Generate all IPv4 addresses from octet ranges
fn generate_ips_from_ranges(
    octet1_range: &[u8],
    octet2_range: &[u8],
    octet3_range: &[u8],
    octet4_range: &[u8],
) -> Vec<IpAddr> {
    let mut ips = Vec::new();

    for &o1 in octet1_range {
        for &o2 in octet2_range {
            for &o3 in octet3_range {
                for &o4 in octet4_range {
                    ips.push(IpAddr::V4(Ipv4Addr::new(o1, o2, o3, o4)));
                }
            }
        }
    }

    ips
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_type_from_socket_whatsminer_2024_09_30() {
        const RAW_DATA: &str = r#"{"STATUS": [{"STATUS": "S", "Msg": "Device Details"}], "DEVDETAILS": [{"DEVDETAILS": 0, "Name": "SM", "ID": 0, "Driver": "bitmicro", "Kernel": "", "Model": "M30S+_VE40"}, {"DEVDETAILS": 1, "Name": "SM", "ID": 1, "Driver": "bitmicro", "Kernel": "", "Model": "M30S+_VE40"}, {"DEVDETAILS": 2, "Name": "SM", "ID": 2, "Driver": "bitmicro", "Kernel": "", "Model": "M30S+_VE40"}], "id": 1}"#;
        let parsed_data = serde_json::from_str(RAW_DATA).unwrap();
        let result = parse_type_from_socket(parsed_data);
        assert_eq!(
            result,
            Some((Some(MinerMake::WhatsMiner), Some(MinerFirmware::Stock)))
        )
    }

    #[test]
    fn test_parse_type_from_web_whatsminer_2024_09_30() {
        let mut headers = HeaderMap::new();
        headers.insert("location", "https://example.com/".parse().unwrap());

        let response_data = (String::from(""), headers, StatusCode::TEMPORARY_REDIRECT);

        let result = parse_type_from_web(response_data);
        assert_eq!(
            result,
            Some((Some(MinerMake::WhatsMiner), Some(MinerFirmware::Stock)))
        )
    }

    #[test]
    fn test_parse_octet_range() {
        // Test single value
        let result = parse_octet_range("10").unwrap();
        assert_eq!(result, vec![10]);

        // Test range
        let result = parse_octet_range("1-5").unwrap();
        assert_eq!(result, vec![1, 2, 3, 4, 5]);

        // Test larger range
        let result = parse_octet_range("200-255").unwrap();
        assert_eq!(result, (200..=255).collect::<Vec<u8>>());

        // Test invalid range (start > end)
        let result = parse_octet_range("200-100");
        assert!(result.is_err());

        // Test invalid format
        let result = parse_octet_range("1-5-10");
        assert!(result.is_err());

        // Test invalid value
        let result = parse_octet_range("300");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_ips_from_ranges() {
        let octet1 = vec![192];
        let octet2 = vec![168];
        let octet3 = vec![1];
        let octet4 = vec![1, 2];

        let ips = generate_ips_from_ranges(&octet1, &octet2, &octet3, &octet4);

        assert_eq!(ips.len(), 2);
        assert!(ips.contains(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(ips.contains(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2))));
    }

    #[test]
    fn parse_type_from_web_nerdaxe() {
        #[track_caller]
        fn case(body: &str) {
            let response = (body.to_string(), HeaderMap::new(), StatusCode::OK);
            assert_eq!(
                parse_type_from_web(response),
                Some((Some(MinerMake::NerdAxe), Some(MinerFirmware::Stock)))
            );
        }

        case("<html><title>NerdAxe</title></html>");
        case("<html><title>NerdQAxe</title></html>");
        case("<html><title>NerdMiner</title></html>");
        case("<html><title>Nerd* Dashboard</title></html>");
    }

    #[test]
    fn parse_type_from_web_bitaxe_not_nerdaxe() {
        let response = (
            "<html><title>AxeOS</title></html>".to_string(),
            HeaderMap::new(),
            StatusCode::OK,
        );
        assert_eq!(
            parse_type_from_web(response),
            Some((Some(MinerMake::Bitaxe), Some(MinerFirmware::Stock)))
        );
    }
}

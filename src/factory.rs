use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    panic::AssertUnwindSafe,
    pin::Pin,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use asic_rs_core::{
    data::command::MinerCommand,
    traits::{
        entry::FirmwareEntry,
        identification::WebResponse,
        miner::{Miner, MinerAuth},
    },
    util::{send_rpc_command, send_web_command},
};
use futures::{Stream, StreamExt, future::FutureExt, pin_mut, stream};
use ipnet::IpNet;
use rand::seq::SliceRandom;
use tokio::{net::TcpStream, task::JoinSet, time::timeout};

const IDENTIFICATION_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECTIVITY_TIMEOUT: Duration = Duration::from_secs(1);
const CONNECTIVITY_RETRIES: u32 = 3;
const NOFILE_PER_CONCURRENCY: u64 = 8;
const MIN_NOFILE_LIMIT: u64 = 2048;

fn calculate_optimal_concurrency(ip_count: usize) -> usize {
    match ip_count {
        0..=1000 => 1000,
        1001..=5000 => 2500,
        5001..=10000 => 5000,
        _ => 10000,
    }
}

fn calculate_desired_nofile_limit(concurrency: usize) -> u64 {
    (concurrency as u64)
        .saturating_mul(NOFILE_PER_CONCURRENCY)
        .max(MIN_NOFILE_LIMIT)
}

async fn check_port_open(ip: IpAddr, port: u16, connectivity_timeout: Duration) -> bool {
    let addr: SocketAddr = (ip, port).into();
    let stream = match timeout(connectivity_timeout, TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        _ => return false,
    };
    let _ = stream.set_nodelay(true);
    true
}

async fn get_miner_type_from_command(
    ip: IpAddr,
    command: MinerCommand,
    registry: Arc<[Arc<dyn FirmwareEntry>]>,
) -> Option<Arc<dyn FirmwareEntry>> {
    match command {
        MinerCommand::RPC { command, .. } => {
            let response = send_rpc_command(&ip, command).await?;
            let upper = response.to_string().to_uppercase();
            registry.iter().find(|fw| fw.identify_rpc(&upper)).cloned()
        }
        MinerCommand::WebAPI { command, .. } => {
            let (body, headers, status) = send_web_command(&ip, command).await?;
            let auth_header = headers
                .get("www-authenticate")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");
            let algo_header = headers
                .get("algorithm")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");
            let redirect_header = headers
                .get("location")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");
            let web_resp = WebResponse {
                body: &body,
                auth_header,
                algo_header,
                redirect_header,
                status: status.as_u16(),
            };
            registry
                .iter()
                .find(|fw| fw.identify_web(&web_resp))
                .cloned()
        }
        _ => None,
    }
}

/// Build the default firmware registry, gated by feature flags.
///
/// Non-stock firmwares are listed first so they take priority over stock
/// when multiple responses are received for the same device.
#[allow(clippy::vec_init_then_push)]
pub fn default_firmware_registry() -> Vec<Arc<dyn FirmwareEntry>> {
    let mut registry: Vec<Arc<dyn FirmwareEntry>> = vec![];

    #[cfg(feature = "braiins")]
    registry.push(Arc::new(
        asic_rs_firmwares_braiins::firmware::BraiinsFirmware::default(),
    ));

    #[cfg(feature = "luxminer")]
    registry.push(Arc::new(
        asic_rs_firmwares_luxminer::firmware::LuxMinerFirmware::default(),
    ));

    #[cfg(feature = "marathon")]
    registry.push(Arc::new(
        asic_rs_firmwares_marathon::firmware::MarathonFirmware::default(),
    ));

    #[cfg(feature = "vnish")]
    registry.push(Arc::new(
        asic_rs_firmwares_vnish::firmware::VnishFirmware::default(),
    ));

    #[cfg(feature = "epic")]
    registry.push(Arc::new(
        asic_rs_firmwares_epic::firmware::EPicFirmware::default(),
    ));

    // Stock firmwares — checked last so non-stock take priority
    #[cfg(feature = "whatsminer")]
    registry.push(Arc::new(
        asic_rs_firmwares_whatsminer::firmware::WhatsMinerFirmware::default(),
    ));

    #[cfg(feature = "antminer")]
    registry.push(Arc::new(
        asic_rs_firmwares_antminer::firmware::AntMinerStockFirmware::default(),
    ));

    #[cfg(feature = "avalonminer")]
    registry.push(Arc::new(
        asic_rs_firmwares_avalonminer::firmware::AvalonStockFirmware::default(),
    ));

    // NerdAxe before Bitaxe — both check web root but NerdAxe is more specific
    #[cfg(feature = "nerdaxe")]
    registry.push(Arc::new(
        asic_rs_firmwares_nerdaxe::firmware::NerdAxeFirmware::default(),
    ));

    #[cfg(feature = "bitaxe")]
    registry.push(Arc::new(
        asic_rs_firmwares_bitaxe::firmware::BitaxeFirmware::default(),
    ));

    registry
}

#[derive(Clone)]
pub struct MinerFactory {
    search_firmwares: Option<Vec<Arc<dyn FirmwareEntry>>>,
    ips: Vec<IpAddr>,
    discovery_auth_by_firmware: HashMap<String, MinerAuth>,
    identification_timeout: Duration,
    connectivity_timeout: Duration,
    connectivity_retries: u32,
    concurrent: Option<usize>,
    nofile_limit: Option<u64>,
    nofile_adjustment: bool,
    check_port: bool,
}

impl std::fmt::Debug for MinerFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinerFactory")
            .field("ips", &self.ips.len())
            .field(
                "search_firmwares",
                &self.search_firmwares.as_ref().map(|v| v.len()),
            )
            .field(
                "discovery_auth_by_firmware",
                &self.discovery_auth_by_firmware.len(),
            )
            .field("identification_timeout", &self.identification_timeout)
            .field("connectivity_timeout", &self.connectivity_timeout)
            .field("connectivity_retries", &self.connectivity_retries)
            .field("concurrent", &self.concurrent)
            .field("nofile_limit", &self.nofile_limit)
            .field("nofile_adjustment", &self.nofile_adjustment)
            .field("check_port", &self.check_port)
            .finish()
    }
}

impl Default for MinerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl MinerFactory {
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn scan_miner(&self, ip: IpAddr) -> Result<Option<Box<dyn Miner>>> {
        if (1..self.connectivity_retries).next().is_some() {
            if !self.check_port {
                return self.get_miner(ip).await;
            }
            if check_port_open(ip, 80, self.connectivity_timeout).await {
                return self.get_miner(ip).await;
            }
            if check_port_open(ip, 4028, self.connectivity_timeout).await {
                return self.get_miner(ip).await;
            }
            if check_port_open(ip, 4029, self.connectivity_timeout).await {
                return self.get_miner(ip).await;
            }
            if check_port_open(ip, 8889, self.connectivity_timeout).await {
                return self.get_miner(ip).await;
            }
        }
        tracing::trace!("no response from any miner-specific ports");
        Ok(None)
    }

    /// Discover and construct a miner at the given IP.
    ///
    /// Uses backend default credentials during discovery/build unless
    /// overridden via [`Self::with_firmware_discovery_auth`].
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn get_miner(&self, ip: IpAddr) -> Result<Option<Box<dyn Miner>>> {
        match AssertUnwindSafe(self.get_miner_inner(ip))
            .catch_unwind()
            .await
        {
            Ok(result) => result,
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                tracing::error!("panic during miner discovery for {ip}: {msg}");
                Err(anyhow::anyhow!(
                    "internal panic during miner discovery: {msg}"
                ))
            }
        }
    }

    async fn get_miner_inner(&self, ip: IpAddr) -> Result<Option<Box<dyn Miner>>> {
        let registry: Arc<[Arc<dyn FirmwareEntry>]> = Arc::from(
            self.search_firmwares
                .clone()
                .unwrap_or_else(default_firmware_registry)
                .as_slice(),
        );

        let mut commands: HashSet<MinerCommand> = HashSet::new();
        for fw in registry.iter() {
            for cmd in fw.get_discovery_commands() {
                commands.insert(cmd);
            }
        }

        let mut discovery_tasks = JoinSet::new();
        for command in commands {
            let reg = registry.clone();
            discovery_tasks.spawn(get_miner_type_from_command(ip, command, reg));
        }

        let id_timeout = tokio::time::sleep(self.identification_timeout).fuse();
        pin_mut!(id_timeout);

        let mut found: Option<Arc<dyn FirmwareEntry>> = None;

        loop {
            if discovery_tasks.is_empty() {
                break;
            }
            tokio::select! {
                _ = &mut id_timeout => break,
                r = discovery_tasks.join_next() => {
                    let fw = match r {
                        Some(Ok(v)) => v,
                        Some(Err(e)) => {
                            tracing::warn!("discovery task failed for {ip}: {e}");
                            continue;
                        }
                        None => continue,
                    };
                    match fw {
                        Some(fw) if !fw.is_stock() => {
                            found = Some(fw);
                            break;
                        }
                        Some(fw) => {
                            found = Some(fw);
                            break;
                        }
                        _ => continue,
                    }
                }
            }
        }

        // If we found a stock firmware, wait a short window for non-stock to respond
        if found.as_ref().map(|f| f.is_stock()).unwrap_or(false) {
            let upgrade_window = tokio::time::sleep(Duration::from_millis(300)).fuse();
            pin_mut!(upgrade_window);

            loop {
                if discovery_tasks.is_empty() {
                    break;
                }
                tokio::select! {
                    _ = &mut id_timeout => break,
                    _ = &mut upgrade_window => break,
                    r = discovery_tasks.join_next() => {
                        if let Some(Ok(Some(fw))) = r
                            && !fw.is_stock()
                        {
                            found = Some(fw);
                            break;
                        }
                    }
                }
            }
        }

        discovery_tasks.abort_all();
        while discovery_tasks.join_next().await.is_some() {}

        match found {
            Some(fw) => {
                let auth = self.discovery_auth_by_firmware.get(&fw.to_string());
                match fw.build_miner(ip, auth).await {
                    Ok(miner) => Ok(Some(miner)),
                    Err(e) => {
                        tracing::debug!("failed to build miner for {ip}: {e}");
                        Ok(None)
                    }
                }
            }
            None => {
                tracing::debug!("failed to identify {ip}");
                Ok(None)
            }
        }
    }

    pub fn new() -> MinerFactory {
        MinerFactory {
            search_firmwares: None,
            ips: Vec::new(),
            discovery_auth_by_firmware: HashMap::new(),
            identification_timeout: IDENTIFICATION_TIMEOUT,
            connectivity_timeout: CONNECTIVITY_TIMEOUT,
            connectivity_retries: CONNECTIVITY_RETRIES,
            concurrent: None,
            nofile_limit: None,
            nofile_adjustment: true,
            check_port: true,
        }
    }

    pub fn with_port_check(mut self, enabled: bool) -> Self {
        self.check_port = enabled;
        self
    }

    /// Set credentials for a specific firmware entry to use during
    /// miner construction after identification.
    pub fn with_firmware_discovery_auth(
        mut self,
        firmware: &dyn FirmwareEntry,
        auth: MinerAuth,
    ) -> Self {
        self.discovery_auth_by_firmware
            .insert(firmware.to_string(), auth);
        self
    }

    pub fn with_concurrent_limit(mut self, limit: usize) -> Self {
        self.concurrent = Some(limit);
        self
    }

    pub fn with_nofile_limit(mut self, limit: u64) -> Self {
        self.nofile_limit = Some(limit);
        self
    }

    pub fn with_nofile_adjustment(mut self, enabled: bool) -> Self {
        self.nofile_adjustment = enabled;
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

    /// Override the firmware registry with a custom list.
    pub fn with_firmwares(mut self, firmwares: Vec<Arc<dyn FirmwareEntry>>) -> Self {
        self.search_firmwares = Some(firmwares);
        self
    }

    // Subnet handlers
    pub fn from_subnet(subnet: &str) -> Result<Self> {
        Self::new().with_subnet(subnet)
    }

    pub fn with_subnet(mut self, subnet: &str) -> Result<Self> {
        let ips = self.hosts_from_subnet(subnet)?;
        self.ips.extend(ips);
        self.shuffle_ips();
        Ok(self)
    }

    pub fn set_subnet(&mut self, subnet: &str) -> Result<&Self> {
        let ips = self.hosts_from_subnet(subnet)?;
        self.ips = ips;
        self.shuffle_ips();
        Ok(self)
    }

    fn hosts_from_subnet(&self, subnet: &str) -> Result<Vec<IpAddr>> {
        let network = IpNet::from_str(subnet)?;
        let hosts = match network {
            IpNet::V4(network_v4) => {
                let start = u32::from(network_v4.network());
                let end = u32::from(network_v4.broadcast());

                (start..=end)
                    .map(Ipv4Addr::from)
                    .map(IpAddr::V4)
                    .collect::<Vec<IpAddr>>()
            }
            IpNet::V6(network_v6) => network_v6.hosts().map(IpAddr::V6).collect(),
        };

        Ok(hosts)
    }

    fn shuffle_ips(&mut self) {
        let mut rng = rand::rng();
        self.ips.shuffle(&mut rng);
    }

    // Octet handlers
    pub fn from_octets(octet1: &str, octet2: &str, octet3: &str, octet4: &str) -> Result<Self> {
        Self::new().with_octets(octet1, octet2, octet3, octet4)
    }

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
    pub fn from_range(range_str: &str) -> Result<Self> {
        Self::new().with_range(range_str)
    }

    pub fn with_range(mut self, range_str: &str) -> Result<Self> {
        let ips = self.hosts_from_range(range_str)?;
        self.ips.extend(ips);
        self.shuffle_ips();
        Ok(self)
    }

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

    pub fn hosts(&self) -> Vec<IpAddr> {
        self.ips.clone()
    }

    pub fn len(&self) -> usize {
        self.ips.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ips.is_empty()
    }

    pub async fn scan(&self) -> Result<Vec<Box<dyn Miner>>> {
        if self.ips.is_empty() {
            return Err(anyhow::anyhow!(
                "No IPs to scan. Use with_subnet, with_octets, or with_range to set IPs."
            ));
        }

        let concurrency = self
            .concurrent
            .unwrap_or(calculate_optimal_concurrency(self.ips.len()));

        if let Some(desired_nofile) = self.nofile_limit.or_else(|| {
            self.nofile_adjustment
                .then(|| calculate_desired_nofile_limit(concurrency))
        }) {
            maybe_adjust_nofile_limit(desired_nofile);
        }

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

        if let Some(desired_nofile) = self.nofile_limit.or_else(|| {
            self.nofile_adjustment
                .then(|| calculate_desired_nofile_limit(concurrency))
        }) {
            maybe_adjust_nofile_limit(desired_nofile);
        }

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

    #[allow(clippy::type_complexity)]
    pub fn scan_stream_with_ip(
        &self,
    ) -> Pin<Box<impl Stream<Item = (IpAddr, Option<Box<dyn Miner>>)> + Send + use<>>> {
        let concurrency = self
            .concurrent
            .unwrap_or(calculate_optimal_concurrency(self.ips.len()));

        if let Some(desired_nofile) = self.nofile_limit.or_else(|| {
            self.nofile_adjustment
                .then(|| calculate_desired_nofile_limit(concurrency))
        }) {
            maybe_adjust_nofile_limit(desired_nofile);
        }

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

    pub async fn scan_by_range(self, range_str: &str) -> Result<Vec<Box<dyn Miner>>> {
        self.with_range(range_str)?.scan().await
    }
}

#[cfg(unix)]
fn maybe_adjust_nofile_limit(desired: u64) {
    if let Err(err) = rlimit::increase_nofile_limit(desired) {
        tracing::warn!("failed to raise RLIMIT_NOFILE to {desired}: {err}");
    }
}

#[cfg(windows)]
fn maybe_adjust_nofile_limit(desired: u64) {
    let current = rlimit::getmaxstdio() as u64;
    if current >= desired {
        return;
    }

    let target = desired.min(u32::MAX as u64) as u32;
    if let Err(err) = rlimit::setmaxstdio(target) {
        tracing::warn!("failed to raise maxstdio from {current} to {target}: {err}");
    }
}

#[cfg(not(any(unix, windows)))]
fn maybe_adjust_nofile_limit(_desired: u64) {}

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
        let value: u8 = range_str.parse()?;
        Ok(vec![value])
    }
}

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
    #[cfg(feature = "whatsminer")]
    fn test_identify_whatsminer_rpc() {
        use asic_rs_core::traits::identification::FirmwareIdentification;
        use asic_rs_firmwares_whatsminer::firmware::WhatsMinerFirmware;

        const RAW_DATA: &str = r#"{"STATUS": [{"STATUS": "S", "Msg": "Device Details"}], "DEVDETAILS": [{"DEVDETAILS": 0, "Name": "SM", "ID": 0, "Driver": "bitmicro", "Kernel": "", "Model": "M30S+_VE40"}, {"DEVDETAILS": 1, "Name": "SM", "ID": 1, "Driver": "bitmicro", "Kernel": "", "Model": "M30S+_VE40"}, {"DEVDETAILS": 2, "Name": "SM", "ID": 2, "Driver": "bitmicro", "Kernel": "", "Model": "M30S+_VE40"}], "id": 1}"#;
        let fw = WhatsMinerFirmware::default();
        assert!(fw.identify_rpc(&RAW_DATA.to_uppercase()));
        assert!(fw.is_stock());
    }

    #[test]
    #[cfg(feature = "whatsminer")]
    fn test_identify_whatsminer_web_redirect() {
        use asic_rs_core::traits::identification::{FirmwareIdentification, WebResponse};
        use asic_rs_firmwares_whatsminer::firmware::WhatsMinerFirmware;

        let web_resp = WebResponse {
            body: "",
            auth_header: "",
            algo_header: "",
            redirect_header: "https://example.com/",
            status: 307,
        };
        let fw = WhatsMinerFirmware::default();
        assert!(fw.identify_web(&web_resp));
    }

    #[test]
    fn test_parse_octet_range() {
        let result = parse_octet_range("10").unwrap();
        assert_eq!(result, vec![10]);

        let result = parse_octet_range("1-5").unwrap();
        assert_eq!(result, vec![1, 2, 3, 4, 5]);

        let result = parse_octet_range("200-255").unwrap();
        assert_eq!(result, (200..=255).collect::<Vec<u8>>());

        let result = parse_octet_range("200-100");
        assert!(result.is_err());

        let result = parse_octet_range("1-5-10");
        assert!(result.is_err());

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
    #[cfg(feature = "nerdaxe")]
    fn identify_nerdaxe_web() {
        use asic_rs_core::traits::identification::{FirmwareIdentification, WebResponse};
        use asic_rs_firmwares_nerdaxe::firmware::NerdAxeFirmware;

        #[track_caller]
        fn case(body: &str) {
            let response = WebResponse {
                body,
                auth_header: "",
                algo_header: "",
                redirect_header: "",
                status: 200,
            };
            assert!(NerdAxeFirmware::default().identify_web(&response));
        }

        case("<html><title>NerdAxe</title></html>");
        case("<html><title>NerdQAxe</title></html>");
        case("<html><title>NerdMiner</title></html>");
    }

    #[test]
    #[cfg(all(feature = "bitaxe", feature = "nerdaxe"))]
    fn identify_bitaxe_not_nerdaxe() {
        use asic_rs_core::traits::identification::{FirmwareIdentification, WebResponse};
        use asic_rs_firmwares_bitaxe::firmware::BitaxeFirmware;
        use asic_rs_firmwares_nerdaxe::firmware::NerdAxeFirmware;

        let response = WebResponse {
            body: "<html><title>AxeOS</title></html>",
            auth_header: "",
            algo_header: "",
            redirect_header: "",
            status: 200,
        };
        assert!(BitaxeFirmware::default().identify_web(&response));
        assert!(!NerdAxeFirmware::default().identify_web(&response));
    }
}

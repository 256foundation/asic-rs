use std::{
    any::Any,
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
use futures::{
    Stream, StreamExt,
    future::FutureExt,
    pin_mut,
    stream::{self, FuturesUnordered},
};
use ipnet::IpNet;
use rand::seq::SliceRandom;
use tokio::{net::TcpStream, time::timeout};

const IDENTIFICATION_TIMEOUT: Duration = Duration::from_secs(3);
const MINER_CONSTRUCTION_TIMEOUT: Duration = Duration::from_secs(5);
const IDENTIFICATION_CONCURRENCY: usize = 128;
const CONNECTIVITY_TIMEOUT: Duration = Duration::from_millis(500);
const CONNECTIVITY_RETRIES: u32 = 0;
const CONNECTIVITY_CONCURRENCY: usize = 256;
const CONNECTIVITY_RETRY_CONCURRENCY: usize = 64;
const CONNECTIVITY_RETRY_BACKOFF: Duration = Duration::from_millis(100);
const SCAN_RETRIES: u32 = 0;
const SCAN_RETRY_BACKOFF: Duration = Duration::from_millis(250);
const IDENTIFICATION_RETRY_CONCURRENCY: usize = 8;
const NOFILE_PER_CONCURRENCY: u64 = 8;
const MIN_NOFILE_LIMIT: u64 = 2048;
const MINER_PORTS: [u16; 4] = [80, 4028, 4029, 8889];

fn calculate_optimal_concurrency(ip_count: usize) -> usize {
    ip_count.clamp(1, IDENTIFICATION_CONCURRENCY)
}

fn calculate_desired_nofile_limit(concurrency: usize) -> u64 {
    (concurrency as u64)
        .saturating_mul(NOFILE_PER_CONCURRENCY)
        .max(MIN_NOFILE_LIMIT)
}

fn calculate_retry_concurrency(concurrency: usize) -> usize {
    concurrency.clamp(1, IDENTIFICATION_RETRY_CONCURRENCY)
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

async fn check_miner_ports(ip: IpAddr, connectivity_timeout: Duration) -> bool {
    let mut probes = stream::iter(MINER_PORTS)
        .map(|port| check_port_open(ip, port, connectivity_timeout))
        .buffer_unordered(MINER_PORTS.len());

    while let Some(open) = probes.next().await {
        if open {
            return true;
        }
    }
    false
}

async fn probe_ip_batch(
    ips: Vec<IpAddr>,
    connectivity_timeout: Duration,
    concurrency: usize,
) -> (Vec<IpAddr>, Vec<IpAddr>) {
    let mut reachable = Vec::new();
    let mut unreachable = Vec::new();
    let mut probes = stream::iter(ips)
        .map(|ip| async move { (ip, check_miner_ports(ip, connectivity_timeout).await) })
        .buffer_unordered(concurrency.max(1));

    while let Some((ip, is_reachable)) = probes.next().await {
        if is_reachable {
            reachable.push(ip);
        } else {
            unreachable.push(ip);
        }
    }
    (reachable, unreachable)
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

fn panic_message(panic_info: &(dyn Any + Send)) -> &str {
    if let Some(message) = panic_info.downcast_ref::<&str>() {
        message
    } else if let Some(message) = panic_info.downcast_ref::<String>() {
        message.as_str()
    } else {
        "unknown panic"
    }
}

async fn get_miner_type_from_command_catch_unwind(
    ip: IpAddr,
    command: MinerCommand,
    registry: Arc<[Arc<dyn FirmwareEntry>]>,
) -> Option<Arc<dyn FirmwareEntry>> {
    match AssertUnwindSafe(get_miner_type_from_command(ip, command, registry))
        .catch_unwind()
        .await
    {
        Ok(result) => result,
        Err(panic_info) => {
            tracing::warn!(
                "discovery command panicked for {ip}: {}",
                panic_message(&*panic_info)
            );
            None
        }
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

    #[cfg(feature = "volcminer")]
    registry.push(Arc::new(
        asic_rs_firmwares_volcminer::firmware::VolcMinerStockFirmware::default(),
    ));

    #[cfg(feature = "elphapex")]
    registry.push(Arc::new(
        asic_rs_firmwares_elphapex::firmware::ElphapexStockFirmware::default(),
    ));

    #[cfg(feature = "epic")]
    registry.push(Arc::new(
        asic_rs_firmwares_epic::firmware::EPicFirmware::default(),
    ));

    // Stock firmwares — checked last so non-stock take priority
    #[cfg(feature = "futurebit")]
    registry.push(Arc::new(
        asic_rs_firmwares_futurebit::firmware::ApolloFirmware::default(),
    ));

    #[cfg(feature = "whatsminer")]
    registry.push(Arc::new(
        asic_rs_firmwares_whatsminer::firmware::WhatsMinerFirmware::default(),
    ));

    #[cfg(feature = "antminer")]
    registry.push(Arc::new(
        asic_rs_firmwares_antminer::firmware::AntMinerStockFirmware::default(),
    ));

    #[cfg(feature = "sealminer")]
    registry.push(Arc::new(
        asic_rs_firmwares_sealminer::firmware::SealMinerStockFirmware::default(),
    ));

    #[cfg(feature = "avalonminer")]
    registry.push(Arc::new(
        asic_rs_firmwares_avalonminer::firmware::AvalonStockFirmware::default(),
    ));

    #[cfg(feature = "auradine")]
    registry.push(Arc::new(
        asic_rs_firmwares_auradine::firmware::AuradineFirmware::default(),
    ));

    // NerdAxe before Bitaxe — both check web root but NerdAxe is more specific
    #[cfg(feature = "nerdaxe")]
    registry.push(Arc::new(
        asic_rs_firmwares_nerdaxe::firmware::NerdAxeFirmware::default(),
    ));

    #[cfg(feature = "proto")]
    registry.push(Arc::new(
        asic_rs_firmwares_proto::firmware::ProtoFirmware::default(),
    ));

    #[cfg(feature = "bitaxe")]
    registry.push(Arc::new(
        asic_rs_firmwares_bitaxe::firmware::BitaxeFirmware::default(),
    ));

    registry
}

#[derive(Clone)]
/// Discovers ASIC miners and constructs firmware-specific miner handles.
///
/// A factory owns the IP addresses to scan, the firmware registry used for
/// identification, and discovery tuning such as timeouts and concurrency.
/// Constructors like [`Self::from_subnet`], [`Self::from_octets`], and
/// [`Self::from_range`] create a factory with an initial search range. The
/// matching `with_*` methods append additional addresses and return the updated
/// factory for chaining.
pub struct MinerFactory {
    search_firmwares: Option<Vec<Arc<dyn FirmwareEntry>>>,
    ips: Vec<IpAddr>,
    discovery_auth_by_firmware: HashMap<String, MinerAuth>,
    identification_timeout: Duration,
    miner_construction_timeout: Duration,
    connectivity_timeout: Duration,
    connectivity_retries: u32,
    connectivity_concurrent: usize,
    connectivity_retry_concurrent: usize,
    connectivity_retry_backoff: Duration,
    scan_retries: u32,
    scan_retry_backoff: Duration,
    retry_concurrent: Option<usize>,
    concurrent: Option<usize>,
    nofile_limit: Option<u64>,
    nofile_adjustment: bool,
    check_port: bool,
    strict_port_check: bool,
}

#[allow(clippy::type_complexity)]
fn identify_ip_stream(
    factory: Arc<MinerFactory>,
    ips: Vec<IpAddr>,
    concurrency: usize,
) -> impl Stream<Item = (IpAddr, Option<Box<dyn Miner>>)> + Send {
    stream::iter(ips)
        .map(move |ip| {
            let factory = Arc::clone(&factory);
            async move {
                let miner = factory.get_miner(ip).await.unwrap_or_else(|error| {
                    tracing::debug!(%ip, %error, "miner identification failed");
                    None
                });
                (ip, miner)
            }
        })
        .buffer_unordered(concurrency.max(1))
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
            .field(
                "miner_construction_timeout",
                &self.miner_construction_timeout,
            )
            .field("connectivity_timeout", &self.connectivity_timeout)
            .field("connectivity_retries", &self.connectivity_retries)
            .field("connectivity_concurrent", &self.connectivity_concurrent)
            .field(
                "connectivity_retry_concurrent",
                &self.connectivity_retry_concurrent,
            )
            .field(
                "connectivity_retry_backoff",
                &self.connectivity_retry_backoff,
            )
            .field("scan_retries", &self.scan_retries)
            .field("scan_retry_backoff", &self.scan_retry_backoff)
            .field("retry_concurrent", &self.retry_concurrent)
            .field("concurrent", &self.concurrent)
            .field("nofile_limit", &self.nofile_limit)
            .field("nofile_adjustment", &self.nofile_adjustment)
            .field("check_port", &self.check_port)
            .field("strict_port_check", &self.strict_port_check)
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
        if !self.check_port {
            return self.get_miner(ip).await;
        }

        for attempt in 0..=self.connectivity_retries {
            if attempt > 0 {
                let multiplier = 1_u32 << (attempt - 1).min(10);
                tokio::time::sleep(self.connectivity_retry_backoff.saturating_mul(multiplier))
                    .await;
            }
            if check_miner_ports(ip, self.connectivity_timeout).await {
                return self.get_miner(ip).await;
            }
        }
        tracing::trace!("no response from any miner-specific ports");
        if self.strict_port_check {
            Ok(None)
        } else {
            self.get_miner(ip).await
        }
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
                let msg = panic_message(&*panic_info);
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

        let found = {
            let mut commands: HashSet<MinerCommand> = HashSet::new();
            for fw in registry.iter() {
                for cmd in fw.get_discovery_commands() {
                    commands.insert(cmd);
                }
            }

            let mut discovery_tasks = FuturesUnordered::new();
            for command in commands {
                let reg = registry.clone();
                discovery_tasks.push(get_miner_type_from_command_catch_unwind(ip, command, reg));
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
                    r = discovery_tasks.next() => {
                        if let Some(Some(fw)) = r {
                            found = Some(fw);
                            break;
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
                        r = discovery_tasks.next() => {
                            if let Some(Some(fw)) = r
                                && !fw.is_stock()
                            {
                                found = Some(fw);
                                break;
                            }
                        }
                    }
                }
            }

            found
        };

        match found {
            Some(fw) => {
                let firmware = fw.to_string();
                let auth = self.discovery_auth_by_firmware.get(&fw.to_string());
                let build_result =
                    timeout(self.miner_construction_timeout, fw.build_miner(ip, auth)).await;

                match build_result {
                    Ok(Ok(miner)) => Ok(Some(miner)),
                    Ok(Err(e)) => {
                        tracing::debug!("failed to build miner for {ip}: {e}");
                        Ok(None)
                    }
                    Err(_) => {
                        tracing::warn!(
                            %ip,
                            %firmware,
                            timeout_ms = self.miner_construction_timeout.as_millis(),
                            "miner construction timed out"
                        );
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

    /// Create an empty factory.
    ///
    /// Use one of the `with_*` range methods before calling [`Self::scan`], or
    /// call [`Self::get_miner`] directly when a single IP address is known.
    pub fn new() -> MinerFactory {
        MinerFactory {
            search_firmwares: None,
            ips: Vec::new(),
            discovery_auth_by_firmware: HashMap::new(),
            identification_timeout: IDENTIFICATION_TIMEOUT,
            miner_construction_timeout: MINER_CONSTRUCTION_TIMEOUT,
            connectivity_timeout: CONNECTIVITY_TIMEOUT,
            connectivity_retries: CONNECTIVITY_RETRIES,
            connectivity_concurrent: CONNECTIVITY_CONCURRENCY,
            connectivity_retry_concurrent: CONNECTIVITY_RETRY_CONCURRENCY,
            connectivity_retry_backoff: CONNECTIVITY_RETRY_BACKOFF,
            scan_retries: SCAN_RETRIES,
            scan_retry_backoff: SCAN_RETRY_BACKOFF,
            retry_concurrent: None,
            concurrent: None,
            nofile_limit: None,
            nofile_adjustment: true,
            check_port: true,
            strict_port_check: true,
        }
    }

    /// Enable or disable TCP reachability prioritization before identification.
    ///
    /// Responsive hosts are identified after the reachability stage. By
    /// default, hosts that miss the probe are excluded; call
    /// `with_strict_port_check(false)` to enable the conservative fallback.
    pub fn with_port_check(mut self, enabled: bool) -> Self {
        self.check_port = enabled;
        self
    }

    /// Exclude hosts that miss all TCP reachability probes.
    ///
    /// This can reduce scan time on sparse networks, but may produce false
    /// negatives when ARP resolution, switches, or embedded TCP stacks drop
    /// probe bursts. The default is `true`; set this to `false` to run one
    /// application-identification fallback pass for probe-negative hosts.
    pub fn with_strict_port_check(mut self, enabled: bool) -> Self {
        self.strict_port_check = enabled;
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

    /// Set the maximum number of responsive addresses identified concurrently.
    ///
    /// TCP reachability probing has an independent concurrency limit. If this
    /// is unset, identification uses a default of 128 addresses.
    pub fn with_concurrent_limit(mut self, limit: usize) -> Self {
        self.concurrent = Some(limit.max(1));
        self
    }

    /// Set the number of additional attempts for addresses that fail scanning.
    ///
    /// Retries run as a separate lower-concurrency batch after the primary
    /// identification pass. The default is zero retries.
    pub fn with_scan_retries(mut self, retries: u32) -> Self {
        self.scan_retries = retries;
        self
    }

    /// Set the maximum number of failed addresses retried concurrently.
    ///
    /// If unset, retry concurrency is capped at 8. This conservative cap
    /// prioritizes recovery on saturated networks; callers can explicitly
    /// select a higher limit.
    pub fn with_retry_concurrent_limit(mut self, limit: usize) -> Self {
        self.retry_concurrent = Some(limit.max(1));
        self
    }

    /// Set the initial delay before retrying a failed address.
    ///
    /// Additional retries use exponential backoff. Concurrency remains bounded
    /// by [`Self::with_retry_concurrent_limit`].
    pub fn with_scan_retry_backoff(mut self, backoff: Duration) -> Self {
        self.scan_retry_backoff = backoff;
        self
    }

    /// Set the initial scan retry delay in milliseconds.
    pub fn with_scan_retry_backoff_millis(mut self, backoff_millis: u64) -> Self {
        self.scan_retry_backoff = Duration::from_millis(backoff_millis);
        self
    }

    /// Set the desired process file descriptor limit before large scans.
    ///
    /// This is best-effort. If the operating system rejects the requested
    /// value, scanning continues with the existing limit.
    pub fn with_nofile_limit(mut self, limit: u64) -> Self {
        self.nofile_limit = Some(limit);
        self
    }

    /// Enable or disable automatic file descriptor limit adjustment.
    ///
    /// Automatic adjustment is enabled by default and is only attempted before
    /// scans. The operation is fail-open.
    pub fn with_nofile_adjustment(mut self, enabled: bool) -> Self {
        self.nofile_adjustment = enabled;
        self
    }

    /// Choose scan concurrency from the number of queued hosts.
    ///
    /// This is normally unnecessary because [`Self::scan`] and streaming scans
    /// already use adaptive concurrency when no explicit limit is set.
    pub fn with_adaptive_concurrency(mut self) -> Self {
        self.concurrent = Some(calculate_optimal_concurrency(self.ips.len()));
        self
    }

    /// Populate the concurrency limit if it has not already been set.
    pub fn update_adaptive_concurrency(&mut self) {
        if self.concurrent.is_none() {
            self.concurrent = Some(calculate_optimal_concurrency(self.ips.len()));
        }
    }

    /// Set the maximum time spent identifying a miner once connectivity exists.
    pub fn with_identification_timeout(mut self, timeout: Duration) -> Self {
        self.identification_timeout = timeout;
        self
    }

    /// Set the identification timeout in seconds.
    pub fn with_identification_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.identification_timeout = Duration::from_secs(timeout_secs);
        self
    }

    /// Set the maximum time allowed to construct a miner after identification.
    pub fn with_miner_construction_timeout(mut self, timeout: Duration) -> Self {
        self.miner_construction_timeout = timeout;
        self
    }

    /// Set the miner construction timeout in seconds.
    pub fn with_miner_construction_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.miner_construction_timeout = Duration::from_secs(timeout_secs);
        self
    }

    /// Set the timeout for quick connectivity probes during scans.
    pub fn with_connectivity_timeout(mut self, timeout: Duration) -> Self {
        self.connectivity_timeout = timeout;
        self
    }

    /// Set the connectivity probe timeout in seconds.
    pub fn with_connectivity_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.connectivity_timeout = Duration::from_secs(timeout_secs);
        self
    }

    /// Set the connectivity probe timeout in milliseconds.
    pub fn with_connectivity_timeout_millis(mut self, timeout_millis: u64) -> Self {
        self.connectivity_timeout = Duration::from_millis(timeout_millis);
        self
    }

    /// Set the number of connectivity retries after the initial attempt.
    ///
    /// A value of zero performs one initial connectivity pass without retries,
    /// which is the default.
    pub fn with_connectivity_retries(mut self, retries: u32) -> Self {
        self.connectivity_retries = retries;
        self
    }

    /// Set the maximum number of addresses probed concurrently on the first pass.
    pub fn with_connectivity_concurrent_limit(mut self, limit: usize) -> Self {
        self.connectivity_concurrent = limit.max(1);
        self
    }

    /// Set the maximum number of addresses probed concurrently on retry passes.
    pub fn with_connectivity_retry_concurrent_limit(mut self, limit: usize) -> Self {
        self.connectivity_retry_concurrent = limit.max(1);
        self
    }

    /// Set the initial delay before retrying connectivity probes.
    pub fn with_connectivity_retry_backoff(mut self, backoff: Duration) -> Self {
        self.connectivity_retry_backoff = backoff;
        self
    }

    /// Set the initial connectivity retry delay in milliseconds.
    pub fn with_connectivity_retry_backoff_millis(mut self, backoff_millis: u64) -> Self {
        self.connectivity_retry_backoff = Duration::from_millis(backoff_millis);
        self
    }

    /// Override the firmware registry with a custom list.
    pub fn with_firmwares(mut self, firmwares: Vec<Arc<dyn FirmwareEntry>>) -> Self {
        self.search_firmwares = Some(firmwares);
        self
    }

    /// Create a factory populated with all addresses from a CIDR subnet.
    ///
    /// Both IPv4 and IPv6 CIDR strings are supported.
    pub fn from_subnet(subnet: &str) -> Result<Self> {
        Self::new().with_subnet(subnet)
    }

    /// Append all addresses from a CIDR subnet to this factory.
    pub fn with_subnet(mut self, subnet: &str) -> Result<Self> {
        let ips = self.hosts_from_subnet(subnet)?;
        self.ips.extend(ips);
        self.shuffle_ips();
        Ok(self)
    }

    /// Replace this factory's queued addresses with all addresses from a CIDR subnet.
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

    /// Create a factory from four IPv4 octet selectors.
    ///
    /// Each octet may be a single value such as `"192"` or an inclusive range
    /// such as `"1-254"`.
    pub fn from_octets(octet1: &str, octet2: &str, octet3: &str, octet4: &str) -> Result<Self> {
        Self::new().with_octets(octet1, octet2, octet3, octet4)
    }

    /// Append addresses generated from four IPv4 octet selectors.
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

    /// Replace this factory's queued addresses with four IPv4 octet selectors.
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

    /// Create a factory from an IPv4 range string.
    ///
    /// Range strings use dotted octets where any octet may be a single value or
    /// an inclusive range, for example `"192.168.1.1-254"`.
    pub fn from_range(range_str: &str) -> Result<Self> {
        Self::new().with_range(range_str)
    }

    /// Append addresses generated from an IPv4 range string.
    pub fn with_range(mut self, range_str: &str) -> Result<Self> {
        let ips = self.hosts_from_range(range_str)?;
        self.ips.extend(ips);
        self.shuffle_ips();
        Ok(self)
    }

    /// Replace this factory's queued addresses with an IPv4 range string.
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

    /// Return the queued scan addresses.
    pub fn hosts(&self) -> Vec<IpAddr> {
        self.ips.clone()
    }

    /// Return the number of queued scan addresses.
    pub fn len(&self) -> usize {
        self.ips.len()
    }

    /// Return whether this factory has no queued scan addresses.
    pub fn is_empty(&self) -> bool {
        self.ips.is_empty()
    }

    #[allow(clippy::type_complexity)]
    fn scan_outcomes_stream(
        &self,
    ) -> Pin<Box<impl Stream<Item = (IpAddr, Option<Box<dyn Miner>>)> + Send + use<>>> {
        let identification_concurrency = self
            .concurrent
            .unwrap_or(calculate_optimal_concurrency(self.ips.len()))
            .max(1);
        let identification_retry_concurrency = self
            .retry_concurrent
            .unwrap_or_else(|| calculate_retry_concurrency(identification_concurrency));
        let descriptor_concurrency = self
            .connectivity_concurrent
            .saturating_mul(MINER_PORTS.len())
            .max(identification_concurrency);

        if let Some(desired_nofile) = self.nofile_limit.or_else(|| {
            self.nofile_adjustment
                .then(|| calculate_desired_nofile_limit(descriptor_concurrency))
        }) {
            maybe_adjust_nofile_limit(desired_nofile);
        }

        // The returned stream must own the factory configuration rather than
        // borrow `self`, and every in-flight identification task shares it.
        let factory = Arc::new(self.clone());
        let queued_ips = self.ips.clone();
        Box::pin(async_stream::stream! {
            let (mut candidates, unreachable) = if factory.check_port {
                let (mut reachable, mut unreachable) = probe_ip_batch(
                    queued_ips,
                    factory.connectivity_timeout,
                    factory.connectivity_concurrent,
                ).await;

                for retry_index in 0..factory.connectivity_retries {
                    if unreachable.is_empty() {
                        break;
                    }
                    let multiplier = 1_u32 << retry_index.min(10);
                    tokio::time::sleep(
                        factory
                            .connectivity_retry_backoff
                            .saturating_mul(multiplier),
                    )
                    .await;
                    let (recovered, still_unreachable) = probe_ip_batch(
                        unreachable,
                        factory.connectivity_timeout,
                        factory.connectivity_retry_concurrent,
                    )
                    .await;
                    reachable.extend(recovered);
                    unreachable = still_unreachable;
                }
                (reachable, unreachable)
            } else {
                (queued_ips, Vec::new())
            };

            for attempt in 0..=factory.scan_retries {
                if candidates.is_empty() {
                    break;
                }
                if attempt > 0 {
                    let multiplier = 1_u32 << (attempt - 1).min(10);
                    tokio::time::sleep(factory.scan_retry_backoff.saturating_mul(multiplier)).await;
                }
                let concurrency = if attempt == 0 {
                    identification_concurrency
                } else {
                    identification_retry_concurrency
                };
                let mut unidentified = Vec::new();
                let mut identification = Box::pin(identify_ip_stream(
                    Arc::clone(&factory),
                    candidates,
                    concurrency,
                ));
                while let Some((ip, miner)) = identification.next().await {
                    if miner.is_some() {
                        yield (ip, miner);
                    } else {
                        unidentified.push(ip);
                    }
                }
                candidates = unidentified;
            }

            for ip in candidates {
                yield (ip, None);
            }

            if factory.strict_port_check {
                for ip in unreachable {
                    yield (ip, None);
                }
            } else {
                // A TCP probe is an optimization, not proof that a host is
                // absent. Preserve correctness with one conservative fallback
                // pass after the responsive-host fast path. Fallback misses are
                // not retried because most are genuinely unused addresses.
                let mut fallback = Box::pin(identify_ip_stream(
                    Arc::clone(&factory),
                    unreachable,
                    identification_concurrency,
                ));
                while let Some(outcome) = fallback.next().await {
                    yield outcome;
                }
            }
        })
    }

    /// Scan all queued addresses and return every successfully identified miner.
    ///
    /// Unsupported hosts and failed identification attempts are skipped. An
    /// error is returned only when the factory has no queued IP addresses.
    pub async fn scan(&self) -> Result<Vec<Box<dyn Miner>>> {
        if self.ips.is_empty() {
            return Err(anyhow::anyhow!(
                "No IPs to scan. Use with_subnet, with_octets, or with_range to set IPs."
            ));
        }
        Ok(self.scan_stream().collect().await)
    }

    /// Scan queued addresses as a stream of successfully identified miners.
    ///
    /// The TCP reachability pass completes before application identification.
    /// Responsive addresses are identified with a separate, lower concurrency
    /// limit so embedded web servers are not overloaded.
    pub fn scan_stream(&self) -> Pin<Box<impl Stream<Item = Box<dyn Miner>> + Send + use<>>> {
        Box::pin(
            self.scan_outcomes_stream()
                .filter_map(|(_, miner)| async move { miner }),
        )
    }

    /// Scan queued addresses as a stream that preserves every attempted IP.
    ///
    /// Stream items are `(ip, miner)` pairs. `miner` is `None` when the host was
    /// unreachable or did not identify as a supported miner.
    #[allow(clippy::type_complexity)]
    pub fn scan_stream_with_ip(
        &self,
    ) -> Pin<Box<impl Stream<Item = (IpAddr, Option<Box<dyn Miner>>)> + Send + use<>>> {
        self.scan_outcomes_stream()
    }

    /// Append an octet range, scan it, and return identified miners.
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

    /// Append an IPv4 range string, scan it, and return identified miners.
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
    fn retry_concurrency_scales_below_primary_concurrency() {
        assert_eq!(calculate_retry_concurrency(0), 1);
        assert_eq!(calculate_retry_concurrency(1), 1);
        assert_eq!(calculate_retry_concurrency(8), 8);
        assert_eq!(calculate_retry_concurrency(9), 8);
        assert_eq!(calculate_retry_concurrency(64), 8);
        assert_eq!(calculate_retry_concurrency(512), 8);
        assert_eq!(calculate_retry_concurrency(10_000), 8);
    }

    #[test]
    fn staged_scan_defaults_are_bounded() {
        let factory = MinerFactory::new();
        assert_eq!(calculate_optimal_concurrency(1024), 128);
        assert_eq!(factory.identification_timeout, Duration::from_secs(3));
        assert_eq!(factory.miner_construction_timeout, Duration::from_secs(5));
        assert_eq!(factory.connectivity_timeout, Duration::from_millis(500));
        assert_eq!(factory.connectivity_retries, 0);
        assert_eq!(factory.scan_retries, 0);
        assert_eq!(factory.connectivity_concurrent, 256);
        assert_eq!(factory.connectivity_retry_concurrent, 64);
        assert!(factory.strict_port_check);
        assert_eq!(
            factory.connectivity_retry_backoff,
            Duration::from_millis(100)
        );
    }

    #[test]
    fn connectivity_retries_can_be_disabled_without_disabling_initial_attempt() {
        let factory = MinerFactory::new().with_connectivity_retries(0);
        assert_eq!(factory.connectivity_retries, 0);
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

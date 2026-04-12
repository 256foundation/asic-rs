use std::{collections::HashMap, net::IpAddr, str::FromStr, time::Duration};

use anyhow;
use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigField, ConfigLocation},
        pools::PoolGroupConfig,
    },
    data::{
        board::{BoardData, ChipData, MinerControlBoard},
        collector::{
            DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
        },
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        hashrate::{HashRate, HashRateUnit},
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_antminer::hardware::AntMinerControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use serde_json::{Value, json};
use web::VnishWebAPI;

use crate::firmware::VnishFirmware;

mod web;

#[derive(Debug)]
pub struct VnishV120 {
    ip: IpAddr,
    web: VnishWebAPI,
    device_info: DeviceInfo,
}

impl VnishV120 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        let auth = Self::default_auth();
        VnishV120 {
            ip,
            web: VnishWebAPI::new(ip, 80, auth),
            device_info: DeviceInfo::new(model, VnishFirmware::default(), HashAlgorithm::SHA256),
        }
    }
}

#[async_trait]
impl APIClient for VnishV120 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!("Unsupported command type for Vnish API")),
        }
    }
}

impl GetConfigsLocations for VnishV120 {
    #[allow(unused_variables)]
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}

impl CollectConfigs for VnishV120 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for VnishV120 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const WEB_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "info",
            parameters: None,
        };
        const WEB_STATUS: MinerCommand = MinerCommand::WebAPI {
            command: "status",
            parameters: None,
        };
        const WEB_SUMMARY: MinerCommand = MinerCommand::WebAPI {
            command: "summary",
            parameters: None,
        };
        const WEB_CHAINS: MinerCommand = MinerCommand::WebAPI {
            command: "chains",
            parameters: None,
        };
        const WEB_FACTORY_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "chains/factory-info",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system/network_status/mac"),
                    tag: None,
                },
            )],
            DataField::SerialNumber => vec![
                (
                    WEB_FACTORY_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/psu_serial"),
                        tag: None,
                    },
                ),
                (
                    WEB_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/serial"),
                        tag: None,
                    },
                ),
            ],
            DataField::Hostname => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system/network_status/hostname"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/fw_version"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/fw_version"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/platform"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system/uptime"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner/hr_realtime"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![
                (
                    WEB_FACTORY_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/hr_stock"),
                        tag: None,
                    },
                ),
                (
                    WEB_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/miner/hr_stock"),
                        tag: None,
                    },
                ),
            ],
            DataField::Wattage => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner/power_consumption"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner/cooling/fans"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![
                (
                    WEB_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/miner/chains"),
                        tag: None,
                    },
                ),
                (
                    WEB_CHAINS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: None,
                    },
                ),
            ],
            DataField::Pools => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner/pools"),
                    tag: None,
                },
            )],
            DataField::IsMining => vec![(
                WEB_STATUS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner_state"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                WEB_STATUS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/find_miner"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for VnishV120 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for VnishV120 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for VnishV120 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for VnishV120 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetSerialNumber for VnishV120 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::SerialNumber)
    }
}

impl GetHostname for VnishV120 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for VnishV120 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for VnishV120 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetControlBoardVersion for VnishV120 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| AntMinerControlBoard::parse(&s).map(|cb| cb.into()))
    }
}

impl GetHashboards for VnishV120 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let all_chains = data.get(&DataField::Hashboards).and_then(|v| v.as_array());
        let Some(all_chains) = all_chains else {
            return Vec::new();
        };

        let board_count = self.device_info.hardware.boards.unwrap_or(0) as u64;

        // Both /summary and /chains endpoints are concatenated into all_chains.
        // Iterate by expected board count, merge entries matching each board ID.
        (1..=board_count)
            .filter_map(|id| {
                let mut merged = serde_json::Map::new();
                for entry in all_chains
                    .iter()
                    .filter(|c| c.pointer("/id").and_then(|v| v.as_u64()) == Some(id))
                {
                    if let Value::Object(obj) = entry {
                        for (k, v) in obj {
                            merged.entry(k.clone()).or_insert_with(|| v.clone());
                        }
                    }
                }
                if merged.is_empty() {
                    return None;
                }
                let chain = Value::Object(merged);

                let hashrate = Self::extract_hashrate(&chain, &["/hashrate_rt", "/hr_realtime"]);
                let expected_hashrate =
                    Self::extract_hashrate(&chain, &["/hashrate_ideal", "/hr_nominal"]);
                let frequency = Self::extract_frequency(&chain);
                let voltage = Self::extract_voltage(&chain);
                let (board_temperature, chip_temperature) = Self::extract_temperatures(&chain);
                let working_chips = Self::extract_working_chips(&chain);
                let active = Self::extract_chain_active_status(&chain, &hashrate);
                let serial_number = Self::extract_chain_serial(&chain, data);
                let tuned = Self::extract_tuned_status(&chain, data);
                let chips = Self::extract_chips(&chain);

                Some(BoardData {
                    position: id as u8,
                    hashrate,
                    expected_hashrate,
                    board_temperature,
                    intake_temperature: chip_temperature,
                    outlet_temperature: chip_temperature,
                    expected_chips: self.device_info.hardware.chips,
                    working_chips,
                    serial_number,
                    chips,
                    voltage,
                    frequency,
                    tuned,
                    active,
                })
            })
            .collect()
    }
}

impl GetHashrate for VnishV120 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(),
        })
    }
}

impl GetExpectedHashrate for VnishV120 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(),
        })
    }
}

impl GetFans for VnishV120 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans: Vec<FanData> = Vec::new();

        if let Some(fans_data) = data.get(&DataField::Fans)
            && let Some(fans_array) = fans_data.as_array()
        {
            for (idx, fan) in fans_array.iter().enumerate() {
                if let Some(rpm) = fan.pointer("/rpm").and_then(|v| v.as_i64()) {
                    fans.push(FanData {
                        position: idx as i16,
                        rpm: Some(AngularVelocity::from_rpm(rpm as f64)),
                    });
                }
            }
        }

        fans
    }
}

impl GetPsuFans for VnishV120 {}

impl GetFluidTemperature for VnishV120 {}

impl GetWattage for VnishV120 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<i64, _>(DataField::Wattage, |w| Power::from_watts(w as f64))
    }
}

impl GetTuningTarget for VnishV120 {}

impl GetLightFlashing for VnishV120 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing)
    }
}

impl GetMessages for VnishV120 {}

impl GetUptime for VnishV120 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract::<String>(DataField::Uptime)
            .and_then(|uptime_str| {
                // Parse uptime strings like "10 days, 18:00"
                let trimmed = uptime_str.trim();

                // Try to parse format like "X days, HH:MM" or "X days"
                if trimmed.contains("days") {
                    let mut total_seconds = 0u64;

                    // Extract days
                    if let Some(days_part) = trimmed.split("days").next()
                        && let Ok(days) = days_part.trim().parse::<u64>()
                    {
                        total_seconds += days * 24 * 60 * 60;
                    }

                    // Extract hours and minutes if present (after comma)
                    if let Some(time_part) = trimmed.split(',').nth(1) {
                        let time_part = time_part.trim();
                        if let Some((hours_str, minutes_str)) = time_part.split_once(':')
                            && let (Ok(hours), Ok(minutes)) = (
                                hours_str.trim().parse::<u64>(),
                                minutes_str.trim().parse::<u64>(),
                            )
                        {
                            total_seconds += hours * 60 * 60 + minutes * 60;
                        }
                    }

                    return Some(Duration::from_secs(total_seconds));
                }

                // Handle "H:MM" or "HH:MM" format (uptime < 1 day)
                if let Some((hours_str, minutes_str)) = trimmed.split_once(':')
                    && let (Ok(hours), Ok(minutes)) = (
                        hours_str.trim().parse::<u64>(),
                        minutes_str.trim().parse::<u64>(),
                    )
                {
                    return Some(Duration::from_secs(hours * 3600 + minutes * 60));
                }

                None
            })
    }
}

impl GetIsMining for VnishV120 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        data.extract::<String>(DataField::IsMining)
            .map(|state| state == "mining" || state == "auto-tuning")
            .unwrap_or(false)
    }
}

impl GetPools for VnishV120 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let mut pools: Vec<PoolData> = Vec::new();

        if let Some(pools_data) = data.get(&DataField::Pools)
            && let Some(pools_array) = pools_data.as_array()
        {
            for (idx, pool) in pools_array.iter().enumerate() {
                let url = pool
                    .pointer("/url")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .map(PoolURL::from);

                let user = pool
                    .pointer("/user")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let accepted_shares = pool.pointer("/accepted").and_then(|v| v.as_u64());
                let rejected_shares = pool.pointer("/rejected").and_then(|v| v.as_u64());
                let pool_status = pool.pointer("/status").and_then(|v| v.as_str());
                let (active, alive) = Self::parse_pool_status(pool_status);

                pools.push(PoolData {
                    position: Some(idx as u16),
                    url,
                    accepted_shares,
                    rejected_shares,
                    active,
                    alive,
                    user,
                });
            }
        }

        vec![PoolGroupData {
            name: String::new(),
            quota: 1,
            pools,
        }]
    }
}

// Helper methods for data extraction
impl VnishV120 {
    fn extract_hashrate(chain: &Value, paths: &[&str]) -> Option<HashRate> {
        paths
            .iter()
            .find_map(|&path| chain.pointer(path).and_then(|v| v.as_f64()))
            .map(|f| HashRate {
                value: f,
                unit: HashRateUnit::GigaHash,
                algo: "SHA256".to_string(),
            })
    }

    fn extract_frequency(chain: &Value) -> Option<Frequency> {
        chain
            .pointer("/frequency")
            .or_else(|| chain.pointer("/freq"))
            .and_then(|v| v.as_f64())
            .map(Frequency::from_megahertz)
    }

    fn extract_voltage(chain: &Value) -> Option<Voltage> {
        chain
            .pointer("/voltage")
            .and_then(|v| v.as_i64())
            .map(|v| Voltage::from_millivolts(v as f64))
    }

    fn extract_temperatures(chain: &Value) -> (Option<Temperature>, Option<Temperature>) {
        let board_temp = chain
            .pointer("/pcb_temp/max")
            .and_then(|v| v.as_i64())
            .map(|t| Temperature::from_celsius(t as f64));

        let chip_temp = chain
            .pointer("/chip_temp/max")
            .and_then(|v| v.as_i64())
            .map(|t| Temperature::from_celsius(t as f64));

        (board_temp, chip_temp)
    }

    fn extract_working_chips(chain: &Value) -> Option<u16> {
        // Prefer individual chip data: count chips with hashrate > 0
        chain
            .pointer("/chips")
            .and_then(|v| v.as_array())
            .map(|chips| {
                chips
                    .iter()
                    .filter(|c| c.pointer("/hr").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.0)
                    .count() as u16
            })
    }

    fn extract_chain_active_status(chain: &Value, hashrate: &Option<HashRate>) -> Option<bool> {
        chain
            .pointer("/status/state")
            .and_then(|v| v.as_str())
            .map(|s| s == "mining")
            .or_else(|| hashrate.as_ref().map(|h| h.value > 0.0))
    }

    fn parse_pool_status(status: Option<&str>) -> (Option<bool>, Option<bool>) {
        match status {
            Some("active" | "working") => (Some(true), Some(true)),
            Some("offline" | "disabled") => (Some(false), Some(false)),
            Some("rejecting") => (Some(false), Some(true)),
            _ => (None, None),
        }
    }

    fn extract_chain_serial(chain: &Value, data: &HashMap<DataField, Value>) -> Option<String> {
        // Try to get serial from chain-specific data first (factory-info)
        chain
            .pointer("/serial")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                // Fallback to miner-wide serial number
                data.extract::<String>(DataField::SerialNumber)
            })
    }

    fn extract_tuned_status(_chain: &Value, data: &HashMap<DataField, Value>) -> Option<bool> {
        // Check miner state to determine tuning status
        if let Some(miner_state) = data.extract::<String>(DataField::IsMining) {
            match miner_state.as_str() {
                "auto-tuning" => Some(false), // Currently tuning, not yet tuned
                "mining" => Some(true),       // Tuned and mining
                _ => None,
            }
        } else {
            None
        }
    }

    fn extract_chips(chain: &Value) -> Vec<ChipData> {
        let mut chips: Vec<ChipData> = Vec::new();

        if let Some(chips_array) = chain.pointer("/chips").and_then(|v| v.as_array()) {
            for (idx, chip) in chips_array.iter().enumerate() {
                let hashrate = chip
                    .pointer("/hr")
                    .and_then(|v| v.as_f64())
                    .map(|f| HashRate {
                        value: f,
                        unit: HashRateUnit::GigaHash,
                        algo: "SHA256".to_string(),
                    });

                let temperature = chip
                    .pointer("/temp")
                    .and_then(|v| v.as_f64())
                    .map(Temperature::from_celsius);

                let voltage = chip
                    .pointer("/volt")
                    .and_then(|v| v.as_i64())
                    .map(|v| Voltage::from_millivolts(v as f64));

                let frequency = chip
                    .pointer("/freq")
                    .and_then(|v| v.as_i64())
                    .map(|f| Frequency::from_megahertz(f as f64));

                let working = hashrate.as_ref().map(|hr| hr.value > 0.0);

                chips.push(ChipData {
                    position: chip
                        .pointer("/id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(idx as u64) as u16,
                    hashrate,
                    temperature,
                    voltage,
                    frequency,
                    tuned: None,
                    working,
                });
            }
        }

        chips
    }
}

#[async_trait]
impl SetFaultLight for VnishV120 {
    fn supports_set_fault_light(&self) -> bool {
        true
    }

    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        if self.get_light_flashing().await == Some(fault) {
            return Ok(true);
        }

        let response = self.web.find_miner(fault).await?;
        Ok(response
            .pointer("/on")
            .and_then(|v| v.as_bool())
            .map(|on| on == fault)
            .unwrap_or(true))
    }
}

#[async_trait]
impl SetPowerLimit for VnishV120 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsPoolsConfig for VnishV120 {
    async fn get_pools_config(&self) -> anyhow::Result<Vec<PoolGroupConfig>> {
        Ok(self
            .get_pools()
            .await
            .iter()
            .map(|g| g.clone().into())
            .collect())
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> anyhow::Result<bool> {
        let pools: Vec<Value> = config
            .iter()
            .flat_map(|group| group.pools.iter())
            .enumerate()
            .map(|(idx, pool)| {
                json!({
                    "url": format!("{}:{}", pool.url.host, pool.url.port),
                    "user": pool.username.as_str(),
                    "pass": pool.password.as_str(),
                    "order": idx,
                    "id": idx,
                })
            })
            .collect();

        Ok(self
            .web
            .set_settings(json!({ "miner": { "pools": pools } }))
            .await
            .is_ok())
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for VnishV120 {
    async fn restart(&self) -> anyhow::Result<bool> {
        Ok(self.web.restart().await.is_ok())
    }

    fn supports_restart(&self) -> bool {
        true
    }
}

#[async_trait]
impl Pause for VnishV120 {
    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        Ok(self.web.stop().await.is_ok())
    }

    fn supports_pause(&self) -> bool {
        true
    }
}

#[async_trait]
impl Resume for VnishV120 {
    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        Ok(self.web.start().await.is_ok())
    }

    fn supports_resume(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsScalingConfig for VnishV120 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for VnishV120 {
    fn supports_upgrade_firmware(&self) -> bool {
        false
    }
}

impl HasDefaultAuth for VnishV120 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("", "admin")
    }
}

impl HasAuth for VnishV120 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

#[async_trait]
impl SupportsTuningConfig for VnishV120 {
    fn supports_tuning_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsFanConfig for VnishV120 {
    fn supports_fan_config(&self) -> bool {
        false
    }
}

use std::{collections::HashMap, net::IpAddr, str::FromStr, time::Duration};

use anyhow::Result;
use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigExtractor, ConfigField, ConfigLocation},
        pools::{PoolConfig, PoolGroupConfig},
    },
    data::{
        board::{BoardData, MinerControlBoard},
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
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Temperature};
use serde_json::Value;

use crate::firmware::VolcMinerStockFirmware;
use asic_rs_makes_volcminer::hardware::VolcMinerControlBoard;

pub mod web;

mod config_form;
mod rpc;
mod status_parser;

use rpc::VolcMinerRPCAPI;
use web::VolcMinerWebAPI;

#[derive(Debug)]
pub struct VolcMinerV1 {
    ip: IpAddr,
    rpc: VolcMinerRPCAPI,
    web: VolcMinerWebAPI,
    device_info: DeviceInfo,
}

impl VolcMinerV1 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        Self {
            ip,
            rpc: VolcMinerRPCAPI::new(ip),
            web: VolcMinerWebAPI::new(ip, Self::default_auth()),
            device_info: DeviceInfo::new(
                model,
                VolcMinerStockFirmware::default(),
                HashAlgorithm::Scrypt,
            ),
        }
    }

    #[cfg(test)]
    fn web_auth(&self) -> MinerAuth {
        self.web.auth()
    }

    fn parse_number_string(value: &str) -> Option<f64> {
        value.trim().replace(',', "").parse::<f64>().ok()
    }

    fn parse_f64(value: &Value) -> Option<f64> {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(Self::parse_number_string))
    }

    fn parse_fan_rpm(value: &Value) -> Option<f64> {
        Self::parse_f64(value).or_else(|| {
            value
                .as_str()
                .filter(|s| s.contains("Socket connect failed"))
                .map(|_| 0.0)
        })
    }

    fn parse_u64(value: &Value) -> Option<u64> {
        value.as_u64().or_else(|| {
            value
                .as_str()
                .map(|s| {
                    s.trim()
                        .chars()
                        .take_while(|ch| ch.is_ascii_digit() || *ch == ',')
                        .collect::<String>()
                        .replace(',', "")
                })
                .filter(|s| !s.is_empty())
                .and_then(|s| s.parse::<u64>().ok())
        })
    }

    fn parse_number_tokens(value: &str) -> Vec<f64> {
        value
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == ','))
            .filter_map(Self::parse_number_string)
            .filter(|value| *value > 0.0)
            .collect()
    }

    fn parse_temperature(value: &Value) -> Option<Temperature> {
        let temps = match value {
            Value::Array(values) => values
                .iter()
                .filter_map(Self::parse_f64)
                .collect::<Vec<_>>(),
            Value::String(value) => Self::parse_number_tokens(value),
            _ => Self::parse_f64(value).into_iter().collect(),
        }
        .into_iter()
        .filter(|value| *value > 0.0)
        .collect::<Vec<_>>();

        if temps.is_empty() {
            return None;
        }

        Some(Temperature::from_celsius(
            temps.iter().sum::<f64>() / temps.len() as f64,
        ))
    }
}

#[async_trait]
impl APIClient for VolcMinerV1 {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for VolcMiner API"
            )),
        }
    }
}

impl GetConfigsLocations for VolcMinerV1 {
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        const WEB_GET_MINER_CONF: MinerCommand = MinerCommand::WebAPI {
            command: "get_miner_conf",
            parameters: None,
        };
        match data_field {
            ConfigField::Pools => vec![(
                WEB_GET_MINER_CONF,
                ConfigExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl CollectConfigs for VolcMinerV1 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for VolcMinerV1 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const RPC_VERSION: MinerCommand = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };

        const RPC_STATS: MinerCommand = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };

        const RPC_SUMMARY: MinerCommand = MinerCommand::RPC {
            command: "summary",
            parameters: None,
        };

        const RPC_POOLS: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };

        const WEB_SYSTEM_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "get_system_info",
            parameters: None,
        };

        const WEB_MINER_STATUS: MinerCommand = MinerCommand::WebAPI {
            command: "get_miner_status",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/macaddr"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/hostname"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system_filesystem_version"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![
                (
                    WEB_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/cgminer_version"),
                        tag: None,
                    },
                ),
                (
                    RPC_VERSION,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/VERSION/0/API"),
                        tag: None,
                    },
                ),
            ],
            DataField::ControlBoardVersion => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system_kernel_version"),
                    tag: None,
                },
            )],
            DataField::Hashrate | DataField::IsMining => vec![
                (
                    WEB_MINER_STATUS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/summary"),
                        tag: None,
                    },
                ),
                (
                    RPC_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/SUMMARY/0"),
                        tag: None,
                    },
                ),
            ],
            DataField::Uptime => vec![
                (
                    WEB_MINER_STATUS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/summary/elapsed"),
                        tag: None,
                    },
                ),
                (
                    RPC_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/SUMMARY/0/Elapsed"),
                        tag: None,
                    },
                ),
            ],
            DataField::Fans | DataField::Hashboards => vec![
                (
                    WEB_MINER_STATUS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: None,
                    },
                ),
                (
                    RPC_STATS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/STATS/1"),
                        tag: None,
                    },
                ),
            ],
            DataField::Pools => vec![
                (
                    WEB_MINER_STATUS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: None,
                    },
                ),
                (
                    RPC_POOLS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/POOLS"),
                        tag: None,
                    },
                ),
            ],
            _ => vec![],
        }
    }
}

impl GetIP for VolcMinerV1 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for VolcMinerV1 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for VolcMinerV1 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for VolcMinerV1 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetHostname for VolcMinerV1 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for VolcMinerV1 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for VolcMinerV1 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetHashboards for VolcMinerV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let Some(stats_data) = data.get(&DataField::Hashboards).and_then(Value::as_object) else {
            return vec![];
        };

        let mut chain_indexes = stats_data
            .keys()
            .filter_map(|key| key.strip_prefix("chain_acn"))
            .filter_map(|idx| idx.parse::<u8>().ok())
            .collect::<Vec<_>>();
        chain_indexes.sort_unstable();

        if !chain_indexes.is_empty() {
            let mut hashboards = Vec::with_capacity(chain_indexes.len());
            for idx in chain_indexes {
                let chips = stats_data
                    .get(&format!("chain_acn{idx}"))
                    .and_then(Self::parse_u64)
                    .and_then(|chips| u16::try_from(chips).ok());

                let hashrate = stats_data
                    .get(&format!("chain_rate{idx}"))
                    .and_then(Self::parse_f64)
                    .map(|rate| HashRate {
                        value: rate,
                        unit: HashRateUnit::MegaHash,
                        algo: "Scrypt".to_string(),
                    });

                let temperature = stats_data
                    .get(&format!("temp{idx}"))
                    .and_then(Self::parse_temperature);

                let chain_acs = stats_data
                    .get(&format!("chain_acs{idx}"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim();

                let active = chips
                    .map(|chips| chips > 0)
                    .or_else(|| hashrate.as_ref().map(|hashrate| hashrate.value > 0.0));
                if active != Some(true) && temperature.is_none() && chain_acs.is_empty() {
                    continue;
                }

                let mut board = BoardData::new(idx - 1, None);
                board.working_chips = chips;
                board.frequency = stats_data
                    .get("frequency")
                    .and_then(Self::parse_f64)
                    .map(Frequency::from_megahertz);
                board.board_temperature = temperature;
                board.hashrate = hashrate;
                board.active = active;
                board.tuned = active;
                hashboards.push(board);
            }

            return hashboards;
        }

        let Some(devs) = stats_data.get("devs").and_then(Value::as_array) else {
            return vec![];
        };

        let mut hashboards = Vec::with_capacity(devs.len());
        for (position, dev) in devs.iter().enumerate() {
            let position = dev
                .get("index")
                .and_then(Self::parse_u64)
                .and_then(|idx| u8::try_from(idx.saturating_sub(1)).ok())
                .unwrap_or(position as u8);

            let mut board = BoardData::new(position, None);
            board.working_chips = dev
                .get("chain_acn")
                .and_then(Self::parse_u64)
                .and_then(|chips| u16::try_from(chips).ok());
            board.frequency = dev
                .get("freq")
                .and_then(|value| match value {
                    Value::String(value) => Self::parse_number_tokens(value).into_iter().next(),
                    _ => Self::parse_f64(value),
                })
                .map(Frequency::from_megahertz);
            board.board_temperature = dev.get("temp").and_then(Self::parse_temperature);
            board.hashrate = dev
                .get("chain_rate")
                .and_then(Self::parse_f64)
                .map(|rate| HashRate {
                    value: rate,
                    unit: HashRateUnit::MegaHash,
                    algo: "Scrypt".to_string(),
                });
            let active = board
                .working_chips
                .map(|chips| chips > 0)
                .or_else(|| board.hashrate.as_ref().map(|hashrate| hashrate.value > 0.0));
            board.active = active;
            board.tuned = active;
            hashboards.push(board);
        }

        hashboards
    }
}

impl GetHashrate for VolcMinerV1 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        let summary = data.get(&DataField::Hashrate)?;
        let hashrate = summary
            .get("MHS 5s")
            .or_else(|| summary.get("ghs5s"))
            .and_then(Self::parse_f64)?;
        Some(HashRate {
            value: hashrate,
            unit: HashRateUnit::MegaHash,
            algo: "Scrypt".to_string(),
        })
    }
}

impl GetExpectedHashrate for VolcMinerV1 {}

impl GetFans for VolcMinerV1 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans = Vec::new();

        if let Some(status) = data.get(&DataField::Fans) {
            for idx in 1..=self.device_info.hardware.fans.unwrap_or(4) {
                let key = format!("fan{idx}");
                if let Some(rpm) = status.get(key.as_str()).and_then(Self::parse_fan_rpm)
                    && rpm > 0.0
                {
                    fans.push(FanData {
                        position: (idx - 1) as i16,
                        rpm: Some(AngularVelocity::from_rpm(rpm)),
                    });
                }
            }
        }

        fans
    }
}

impl GetLightFlashing for VolcMinerV1 {}

impl GetUptime for VolcMinerV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.get(&DataField::Uptime)
            .and_then(Self::parse_u64)
            .map(Duration::from_secs)
    }
}

impl GetIsMining for VolcMinerV1 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        self.parse_hashrate(data)
            .map(|hashrate| hashrate.value > 0.0)
            .unwrap_or(false)
    }
}

impl GetPools for VolcMinerV1 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let Some(pools_data) = data.get(&DataField::Pools) else {
            return vec![];
        };

        let Some(pools_array) = pools_data
            .as_array()
            .or_else(|| pools_data.get("pools").and_then(Value::as_array))
            .or_else(|| pools_data.get("POOLS").and_then(Value::as_array))
        else {
            return vec![PoolGroupData {
                name: "default".to_string(),
                quota: 1,
                pools: vec![],
            }];
        };

        let mut pools = Vec::with_capacity(pools_array.len());
        for (idx, pool_info) in pools_array.iter().enumerate() {
            let url = pool_info
                .get("URL")
                .or_else(|| pool_info.get("url"))
                .and_then(Value::as_str)
                .filter(|url| !url.is_empty())
                .map(|url| PoolURL::from(url.to_string()));

            let position = pool_info
                .get("POOL")
                .or_else(|| pool_info.get("index"))
                .and_then(Self::parse_u64)
                .and_then(|idx| u16::try_from(idx).ok())
                .or_else(|| u16::try_from(idx).ok());

            let accepted_shares = pool_info
                .get("Accepted")
                .or_else(|| pool_info.get("accepted"))
                .and_then(Self::parse_u64);

            let rejected_shares = pool_info
                .get("Rejected")
                .or_else(|| pool_info.get("rejected"))
                .and_then(Self::parse_u64);

            let status = pool_info
                .get("Status")
                .or_else(|| pool_info.get("status"))
                .and_then(Value::as_str)
                .map(|status| status.eq_ignore_ascii_case("alive"));

            let user = pool_info
                .get("User")
                .or_else(|| pool_info.get("user"))
                .and_then(Value::as_str)
                .filter(|user| !user.is_empty())
                .map(str::to_string);

            pools.push(PoolData {
                position,
                url,
                accepted_shares,
                rejected_shares,
                active: status,
                alive: status,
                user,
            });
        }

        vec![PoolGroupData {
            name: "default".to_string(),
            quota: 1,
            pools,
        }]
    }
}

impl GetSerialNumber for VolcMinerV1 {}
impl GetControlBoardVersion for VolcMinerV1 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.get(&DataField::ControlBoardVersion)
            .and_then(Value::as_str)
            .and_then(|kernel| VolcMinerControlBoard::parse(kernel).map(Into::into))
    }
}

impl GetWattage for VolcMinerV1 {}
impl GetTuningTarget for VolcMinerV1 {}
impl GetScaledTuningTarget for VolcMinerV1 {}
impl GetFluidTemperature for VolcMinerV1 {}
impl GetPsuFans for VolcMinerV1 {}
impl GetTuningCapabilities for VolcMinerV1 {}
impl GetMessages for VolcMinerV1 {}

impl SetFaultLight for VolcMinerV1 {
    fn supports_set_fault_light(&self) -> bool {
        false
    }
}

#[async_trait]
impl SetPowerLimit for VolcMinerV1 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsPoolsConfig for VolcMinerV1 {
    fn parse_pools_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> Result<Vec<PoolGroupConfig>> {
        let Some(pools_data) = data.get(&ConfigField::Pools) else {
            return Ok(vec![]);
        };

        let Some(pools_array) = pools_data.get("pools").and_then(Value::as_array) else {
            return Ok(vec![PoolGroupConfig {
                name: "default".to_string(),
                quota: 1,
                pools: vec![],
            }]);
        };

        let mut pools = Vec::with_capacity(pools_array.len());
        for pool in pools_array {
            let Some(url) = pool.get("url").and_then(Value::as_str) else {
                continue;
            };
            if url.is_empty() {
                continue;
            }

            let username = pool
                .get("user")
                .and_then(Value::as_str)
                .map(String::from)
                .unwrap_or_default();
            let password = pool
                .get("pass")
                .and_then(Value::as_str)
                .map(String::from)
                .unwrap_or_default();

            pools.push(PoolConfig {
                url: PoolURL::from(url.to_string()),
                username,
                password,
            });
        }

        pools.truncate(3);

        Ok(vec![PoolGroupConfig {
            name: "default".to_string(),
            quota: 1,
            pools,
        }])
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> Result<bool> {
        let mut pools = config
            .into_iter()
            .flat_map(|group| group.pools)
            .collect::<Vec<_>>();
        pools.truncate(3);
        self.web.set_pools_config(&pools).await
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for VolcMinerV1 {
    fn supports_restart(&self) -> bool {
        true
    }
    async fn restart(&self) -> anyhow::Result<bool> {
        Ok(self.web.reboot().await.is_ok())
    }
}

#[async_trait]
impl Pause for VolcMinerV1 {
    fn supports_pause(&self) -> bool {
        false
    }
}

#[async_trait]
impl Resume for VolcMinerV1 {
    fn supports_resume(&self) -> bool {
        false
    }
}

#[async_trait]
impl ChangePassword for VolcMinerV1 {
    async fn change_password(&mut self, password: &str) -> anyhow::Result<bool> {
        let original_auth = self.web.auth();
        let new_auth = MinerAuth::new(original_auth.username().to_string(), password);
        let result = self.web.change_password(password).await;

        match result {
            Ok(false) => Ok(false),
            Ok(true) => {
                self.set_auth(new_auth);
                if self.web.get_miner_conf().await.is_ok() {
                    Ok(true)
                } else {
                    self.set_auth(original_auth);
                    Ok(false)
                }
            }
            Err(err) => {
                self.set_auth(new_auth);
                if self.web.get_miner_conf().await.is_ok() {
                    Ok(true)
                } else {
                    self.set_auth(original_auth);
                    Err(err)
                }
            }
        }
    }

    fn supports_change_password(&self) -> bool {
        true
    }
}

impl ReadLogs for VolcMinerV1 {
    fn supports_read_logs(&self) -> bool {
        false
    }
}

impl FactoryReset for VolcMinerV1 {
    fn supports_factory_reset(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsScalingConfig for VolcMinerV1 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

impl UpgradeFirmware for VolcMinerV1 {}

impl HasDefaultAuth for VolcMinerV1 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("root", "ltc@dog")
    }
}

impl HasAuth for VolcMinerV1 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

#[async_trait]
impl SupportsTuningConfig for VolcMinerV1 {}

#[async_trait]
impl SupportsFanConfig for VolcMinerV1 {}

impl SupportsTemperatureConfig for VolcMinerV1 {}
impl SupportsTimezoneConfig for VolcMinerV1 {}
impl GetTuningPercent for VolcMinerV1 {}
impl SetTuningPercent for VolcMinerV1 {}

#[cfg(test)]
mod tests;

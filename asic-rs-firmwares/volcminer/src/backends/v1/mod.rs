use std::{collections::HashMap, net::IpAddr, str::FromStr, time::Duration};

use anyhow::Result;
use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigExtractor, ConfigField, ConfigLocation},
        pools::{PoolConfig, PoolGroupConfig},
    },
    data::{
        board::BoardData,
        collector::{DataCollector, DataExtractor, DataField, DataLocation, get_by_pointer},
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

use crate::firmware::VolcMinerFirmware;

pub mod web;

mod config_form;
mod status_parser;

use web::VolcMinerWebAPI;

const WEB_STATUS: MinerCommand = MinerCommand::WebAPI {
    command: "get_miner_status",
    parameters: None,
};
const WEB_CONFIG: MinerCommand = MinerCommand::WebAPI {
    command: "get_miner_conf",
    parameters: None,
};
const WEB_SYSTEM_INFO: MinerCommand = MinerCommand::WebAPI {
    command: "get_system_info",
    parameters: None,
};

#[derive(Debug)]
pub struct VolcMinerV1 {
    ip: IpAddr,
    web: VolcMinerWebAPI,
    device_info: DeviceInfo,
}

impl VolcMinerV1 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        Self {
            ip,
            web: VolcMinerWebAPI::new(ip, Self::default_auth()),
            device_info: DeviceInfo::new(
                model,
                VolcMinerFirmware::default(),
                HashAlgorithm::Scrypt,
            ),
        }
    }

    #[cfg(test)]
    fn web_auth(&self) -> &MinerAuth {
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

    fn scrypt_hashrate(value: f64, unit: HashRateUnit) -> HashRate {
        HashRate {
            value,
            unit,
            algo: "Scrypt".to_string(),
        }
    }

    fn status(data: &HashMap<DataField, Value>, field: DataField) -> Option<&Value> {
        data.get(&field)
    }

    fn configured_pools(config: &Value) -> Vec<PoolConfig> {
        config
            .get("pools")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|pool| {
                let url = pool.get("url").and_then(Value::as_str).unwrap_or_default();
                if url.is_empty() {
                    return None;
                }
                Some(PoolConfig {
                    url: PoolURL::from(url.to_string()),
                    username: pool
                        .get("user")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    password: pool
                        .get("pass")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                })
            })
            .collect()
    }
}

#[async_trait]
impl APIClient for VolcMinerV1 {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for VolcMiner API"
            )),
        }
    }
}

impl GetConfigsLocations for VolcMinerV1 {
    fn get_configs_locations(&self, config_field: ConfigField) -> Vec<ConfigLocation> {
        match config_field {
            ConfigField::Pools => vec![(
                WEB_CONFIG,
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
            DataField::ApiVersion => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/cgminer_version"),
                    tag: None,
                },
            )],
            DataField::Hashrate | DataField::Uptime | DataField::IsMining => vec![(
                WEB_STATUS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/summary"),
                    tag: None,
                },
            )],
            DataField::Fans | DataField::Hashboards | DataField::Pools => vec![(
                WEB_STATUS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
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
        data.get(&DataField::Mac)
            .and_then(Value::as_str)
            .and_then(|s| MacAddr::from_str(s).ok())
    }
}

impl GetHostname for VolcMinerV1 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::Hostname)
            .and_then(Value::as_str)
            .map(str::to_string)
    }
}

impl GetApiVersion for VolcMinerV1 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::ApiVersion)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }
}

impl GetFirmwareVersion for VolcMinerV1 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::FirmwareVersion)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }
}

impl GetHashrate for VolcMinerV1 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        let hashrate = Self::status(data, DataField::Hashrate)?
            .get("ghs5s")
            .and_then(Self::parse_f64)?;
        Some(Self::scrypt_hashrate(hashrate, HashRateUnit::MegaHash))
    }
}

impl GetHashboards for VolcMinerV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        Self::status(data, DataField::Hashboards)
            .and_then(|status| status.get("devs"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(position, dev)| {
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
                    .map(|rate| Self::scrypt_hashrate(rate, HashRateUnit::MegaHash));
                let active = board
                    .working_chips
                    .map(|chips| chips > 0)
                    .or_else(|| board.hashrate.as_ref().map(|hashrate| hashrate.value > 0.0));
                board.active = active;
                board.tuned = active;
                board
            })
            .collect()
    }
}

impl GetFans for VolcMinerV1 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let Some(status) = Self::status(data, DataField::Fans) else {
            return vec![];
        };

        (1..=4)
            .filter_map(|idx| {
                let key = format!("fan{idx}");
                let rpm = status.get(key.as_str()).and_then(Self::parse_fan_rpm)?;
                Some(FanData {
                    position: (idx - 1) as i16,
                    rpm: Some(AngularVelocity::from_rpm(rpm)),
                })
            })
            .collect()
    }
}

impl GetPools for VolcMinerV1 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let pools = Self::status(data, DataField::Pools)
            .and_then(|status| status.get("pools"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|pool| {
                let url = pool
                    .get("url")
                    .and_then(Value::as_str)
                    .filter(|url| !url.is_empty())
                    .map(|url| PoolURL::from(url.to_string()));
                PoolData {
                    position: pool
                        .get("index")
                        .and_then(Self::parse_u64)
                        .and_then(|idx| u16::try_from(idx).ok()),
                    url,
                    accepted_shares: pool.get("accepted").and_then(Self::parse_u64),
                    rejected_shares: pool.get("rejected").and_then(Self::parse_u64),
                    active: pool
                        .get("status")
                        .and_then(Value::as_str)
                        .map(|status| status.eq_ignore_ascii_case("alive")),
                    alive: pool
                        .get("status")
                        .and_then(Value::as_str)
                        .map(|status| status.eq_ignore_ascii_case("alive")),
                    user: pool
                        .get("user")
                        .and_then(Value::as_str)
                        .filter(|user| !user.is_empty())
                        .map(str::to_string),
                }
            })
            .collect::<Vec<_>>();

        if pools.is_empty() {
            vec![]
        } else {
            vec![PoolGroupData {
                name: "default".to_string(),
                quota: 1,
                pools,
            }]
        }
    }
}

impl GetUptime for VolcMinerV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        Self::status(data, DataField::Uptime)
            .and_then(|summary| summary.get("elapsed"))
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

#[async_trait]
impl SupportsPoolsConfig for VolcMinerV1 {
    fn parse_pools_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> Result<Vec<PoolGroupConfig>> {
        let Some(config) = data.get(&ConfigField::Pools) else {
            return Ok(vec![]);
        };
        let pools = Self::configured_pools(config);
        if pools.is_empty() {
            Ok(vec![])
        } else {
            Ok(vec![PoolGroupConfig {
                name: "default".to_string(),
                quota: 1,
                pools,
            }])
        }
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> Result<bool> {
        let pools = config
            .into_iter()
            .flat_map(|group| group.pools)
            .take(3)
            .collect::<Vec<_>>();
        self.web.set_pools_config(&pools).await
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

impl GetSerialNumber for VolcMinerV1 {}
impl GetControlBoardVersion for VolcMinerV1 {}
impl GetExpectedHashrate for VolcMinerV1 {}
impl GetPsuFans for VolcMinerV1 {}
impl GetFluidTemperature for VolcMinerV1 {}
impl GetWattage for VolcMinerV1 {}
impl GetTuningPercent for VolcMinerV1 {}
impl GetTuningTarget for VolcMinerV1 {}
impl GetScaledTuningTarget for VolcMinerV1 {}
impl GetTuningCapabilities for VolcMinerV1 {}
impl GetLightFlashing for VolcMinerV1 {}
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
impl Restart for VolcMinerV1 {
    fn supports_restart(&self) -> bool {
        false
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

impl ChangePassword for VolcMinerV1 {
    fn supports_change_password(&self) -> bool {
        false
    }
}

impl FactoryReset for VolcMinerV1 {
    fn supports_factory_reset(&self) -> bool {
        false
    }
}

impl ReadLogs for VolcMinerV1 {
    fn supports_read_logs(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsScalingConfig for VolcMinerV1 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsTuningConfig for VolcMinerV1 {}

#[async_trait]
impl SupportsFanConfig for VolcMinerV1 {}

impl SupportsTemperatureConfig for VolcMinerV1 {}
impl UpgradeFirmware for VolcMinerV1 {}
impl SetTuningPercent for VolcMinerV1 {}

impl HasAuth for VolcMinerV1 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

impl HasDefaultAuth for VolcMinerV1 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("root", "ltc@dog")
    }
}

#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    net::IpAddr,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use asic_rs_core::{
    config::{
        collector::{
            ConfigCollector, ConfigExtractor, ConfigField, ConfigLocation,
            get_by_pointer as cfg_by_pointer,
        },
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
        message::{MessageSeverity, MinerComponent, MinerMessage},
        miner::{MiningMode, TuningTarget},
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_elphapex::hardware::ElphapexControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature};
use serde_json::Value;

use crate::firmware::ElphapexStockFirmware;

pub mod web;

use web::ElphapexWebAPI;

#[derive(Debug)]
pub struct ElphapexV1 {
    ip: IpAddr,
    web: ElphapexWebAPI,
    device_info: DeviceInfo,
}

impl ElphapexV1 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        Self {
            ip,
            web: ElphapexWebAPI::new(ip, Self::default_auth()),
            device_info: DeviceInfo::new(
                model,
                ElphapexStockFirmware::default(),
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

    fn average_temperature(values: &Value) -> Option<Temperature> {
        let readings = values
            .as_array()?
            .iter()
            .filter_map(Self::parse_f64)
            .filter(|value| *value > 0.0)
            .collect::<Vec<_>>();

        if readings.is_empty() {
            return None;
        }

        Some(Temperature::from_celsius(
            readings.iter().sum::<f64>() / readings.len() as f64,
        ))
    }

    fn average_chip_temperature(values: &Value) -> Option<Temperature> {
        let readings = values
            .as_array()?
            .iter()
            .filter_map(|value| match value {
                Value::String(value) if !value.is_empty() => {
                    value.parse::<f64>().ok().map(|temp| temp / 1000.0)
                }
                Value::Number(_) => value.as_f64().map(|temp| temp / 1000.0),
                _ => None,
            })
            .collect::<Vec<_>>();

        if readings.is_empty() {
            return None;
        }

        Some(Temperature::from_celsius(
            readings.iter().sum::<f64>() / readings.len() as f64,
        ))
    }

    fn minimum_chip_temperature(values: &Value) -> Option<Temperature> {
        values
            .as_array()?
            .iter()
            .filter_map(|value| match value {
                Value::String(value) if !value.is_empty() => {
                    value.parse::<f64>().ok().map(|temp| temp / 1000.0)
                }
                Value::Number(_) => value.as_f64().map(|temp| temp / 1000.0),
                _ => None,
            })
            .min_by(f64::total_cmp)
            .map(Temperature::from_celsius)
    }

    fn parse_work_mode(value: &Value) -> Option<MiningMode> {
        let mode = value
            .as_i64()
            .or_else(|| value.as_str().and_then(|mode| mode.parse::<i64>().ok()))?;

        match mode {
            0 => Some(MiningMode::Normal),
            2 => Some(MiningMode::High),
            3 => Some(MiningMode::Low),
            _ => None,
        }
    }
}

impl Validate for ElphapexV1 {
    type Firmware = ElphapexStockFirmware;
}

#[async_trait]
impl APIClient for ElphapexV1 {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!("Unsupported command type for Elphapex")),
        }
    }
}

impl GetConfigsLocations for ElphapexV1 {
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        const WEB_GET_MINER_CONF: MinerCommand = MinerCommand::WebAPI {
            command: "get_miner_conf",
            parameters: None,
        };
        match data_field {
            ConfigField::Pools => vec![(
                WEB_GET_MINER_CONF,
                ConfigExtractor {
                    func: cfg_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl CollectConfigs for ElphapexV1 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for ElphapexV1 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const WEB_SYSTEM_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "get_system_info",
            parameters: None,
        };
        const WEB_NETWORK_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "get_network_info",
            parameters: None,
        };
        const WEB_STATS: MinerCommand = MinerCommand::WebAPI {
            command: "stats",
            parameters: None,
        };
        const WEB_SUMMARY: MinerCommand = MinerCommand::WebAPI {
            command: "summary",
            parameters: None,
        };
        const WEB_POOLS: MinerCommand = MinerCommand::WebAPI {
            command: "pools",
            parameters: None,
        };
        const WEB_BLINK: MinerCommand = MinerCommand::WebAPI {
            command: "get_blink_status",
            parameters: None,
        };
        const WEB_MINER_CONF: MinerCommand = MinerCommand::WebAPI {
            command: "get_miner_conf",
            parameters: None,
        };
        const WEB_POWER: MinerCommand = MinerCommand::WebAPI {
            command: "get_power",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![
                (
                    WEB_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/macaddr"),
                        tag: None,
                    },
                ),
                (
                    WEB_NETWORK_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/macaddr"),
                        tag: None,
                    },
                ),
            ],
            DataField::SerialNumber => vec![(
                WEB_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/INFO/dev_sn"),
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
            DataField::ApiVersion => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATUS/api_version"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![
                (
                    WEB_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/system_filesystem_version"),
                        tag: None,
                    },
                ),
                (
                    WEB_STATS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/INFO/miner_version"),
                        tag: None,
                    },
                ),
            ],
            DataField::ControlBoardVersion => vec![(
                WEB_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/INFO/hw_version"),
                    tag: None,
                },
            )],
            DataField::Hashboards
            | DataField::Hashrate
            | DataField::ExpectedHashrate
            | DataField::Fans
            | DataField::Uptime => vec![(
                WEB_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            DataField::Messages => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                WEB_BLINK,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/blink"),
                    tag: None,
                },
            )],
            DataField::IsMining => vec![(
                WEB_MINER_CONF,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/fc-work-mode"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                WEB_POWER,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/power_output"),
                    tag: None,
                },
            )],
            DataField::TuningPercent => vec![(
                WEB_MINER_CONF,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/fc-freq-level"),
                    tag: None,
                },
            )],
            DataField::TuningTarget => vec![(
                WEB_MINER_CONF,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            DataField::Pools => vec![(
                WEB_POOLS,
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

impl GetIP for ElphapexV1 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for ElphapexV1 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for ElphapexV1 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for ElphapexV1 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetSerialNumber for ElphapexV1 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::SerialNumber)
    }
}

impl GetHostname for ElphapexV1 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for ElphapexV1 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for ElphapexV1 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
            .map(|version| {
                version
                    .to_ascii_uppercase()
                    .split('V')
                    .next_back()
                    .unwrap_or("")
                    .to_string()
            })
            .filter(|version| !version.is_empty())
    }
}

impl GetControlBoardVersion for ElphapexV1 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .map(|version| {
                ElphapexControlBoard::parse(&version)
                    .map(Into::into)
                    .unwrap_or_else(|| MinerControlBoard::unknown(version))
            })
    }
}

impl GetHashboards for ElphapexV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let Some(stats) = data.get(&DataField::Hashboards) else {
            return vec![];
        };
        let stats0 = stats
            .pointer("/STATS/0")
            .or_else(|| stats.get("STATS").and_then(|stats| stats.get(0)))
            .unwrap_or(stats);

        let chains = stats0
            .get("chain")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let expected = self
            .device_info
            .hardware
            .board_count()
            .map(usize::from)
            .or_else(|| {
                stats0
                    .get("chain_num")
                    .and_then(Self::parse_u64)
                    .and_then(|count| usize::try_from(count).ok())
            })
            .unwrap_or(chains.len());

        let board_count = expected.max(chains.len());
        let unit = stats0
            .get("rate_unit")
            .and_then(Value::as_str)
            .and_then(|unit| HashRateUnit::from_str(unit).ok())
            .unwrap_or(HashRateUnit::MegaHash);
        let mut hashboards = (0..board_count)
            .filter_map(|idx| {
                let position = u8::try_from(idx).ok()?;
                Some(BoardData::new(
                    position,
                    self.device_info.hardware.chips_for_board(idx),
                ))
            })
            .collect::<Vec<_>>();

        for chain in chains {
            let Some(index) = chain
                .get("index")
                .and_then(Self::parse_u64)
                .and_then(|index| usize::try_from(index).ok())
            else {
                continue;
            };
            let Some(board) = hashboards.get_mut(index) else {
                continue;
            };

            board.hashrate = chain
                .get("rate_real")
                .and_then(Self::parse_f64)
                .map(|rate| HashRate {
                    value: rate,
                    unit,
                    algo: self.device_info.algo,
                });
            board.expected_hashrate =
                chain
                    .get("rate_ideal")
                    .and_then(Self::parse_f64)
                    .map(|rate| HashRate {
                        value: rate,
                        unit,
                        algo: self.device_info.algo,
                    });
            board.working_chips = chain
                .get("asic_num")
                .and_then(Self::parse_u64)
                .and_then(|chips| u16::try_from(chips).ok());
            board.board_temperature = chain
                .get("temp_pcb")
                .or_else(|| chain.get("temp_pic"))
                .and_then(Self::average_temperature);
            board.inlet_chip_temperature = chain
                .get("temp_chip")
                .and_then(Self::minimum_chip_temperature);
            board.outlet_chip_temperature = chain
                .get("temp_chip")
                .and_then(Self::average_chip_temperature);
            board.serial_number = chain
                .get("sn")
                .and_then(Value::as_str)
                .filter(|sn| !sn.is_empty())
                .map(str::to_string);
            board.frequency = chain
                .get("freq_avg")
                .and_then(Self::parse_f64)
                .filter(|freq| *freq > 0.0)
                .map(|freq| Frequency::from_megahertz(freq / 1000.0));
            let active = board
                .working_chips
                .map(|chips| chips > 0)
                .or_else(|| board.hashrate.as_ref().map(|hashrate| hashrate.value > 0.0));
            board.active = active;
            board.tuned = active;
        }

        hashboards
    }
}

impl GetHashrate for ElphapexV1 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        let stats = data.get(&DataField::Hashrate)?;
        let stats0 = stats
            .pointer("/STATS/0")
            .or_else(|| stats.get("STATS").and_then(|stats| stats.get(0)))
            .unwrap_or(stats);
        let rate = stats0
            .get("rate_avg")
            .or_else(|| stats0.get("rate_5s"))
            .and_then(Self::parse_f64)?;
        let unit = stats0
            .get("rate_unit")
            .and_then(Value::as_str)
            .and_then(|unit| HashRateUnit::from_str(unit).ok())
            .unwrap_or(HashRateUnit::MegaHash);

        Some(HashRate {
            value: rate,
            unit,
            algo: self.device_info.algo,
        })
    }
}

impl GetExpectedHashrate for ElphapexV1 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        let stats = data.get(&DataField::ExpectedHashrate)?;
        let stats0 = stats
            .pointer("/STATS/0")
            .or_else(|| stats.get("STATS").and_then(|stats| stats.get(0)))
            .unwrap_or(stats);
        let rate = stats0
            .get("total_rateideal")
            .or_else(|| stats0.get("rate_ideal"))
            .and_then(Self::parse_f64)?;
        let unit = stats0
            .get("rate_unit")
            .and_then(Value::as_str)
            .and_then(|unit| HashRateUnit::from_str(unit).ok())
            .unwrap_or(HashRateUnit::MegaHash);

        Some(HashRate {
            value: rate,
            unit,
            algo: self.device_info.algo,
        })
    }
}

impl GetFans for ElphapexV1 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let Some(stats) = data.get(&DataField::Fans) else {
            return vec![];
        };
        let stats0 = stats
            .pointer("/STATS/0")
            .or_else(|| stats.get("STATS").and_then(|stats| stats.get(0)))
            .unwrap_or(stats);

        stats0
            .get("fan")
            .and_then(Value::as_array)
            .map(|fans| {
                fans.iter()
                    .enumerate()
                    .filter_map(|(idx, fan)| {
                        let rpm = Self::parse_f64(fan)?;
                        if rpm <= 0.0 {
                            return None;
                        }
                        Some(FanData {
                            position: idx as i16,
                            rpm: Some(AngularVelocity::from_rpm(rpm)),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl GetPsuFans for ElphapexV1 {}
impl GetFluidTemperature for ElphapexV1 {}
impl GetWattage for ElphapexV1 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.get(&DataField::Wattage)
            .and_then(Self::parse_f64)
            .map(Power::from_watts)
    }
}

impl GetTuningPercent for ElphapexV1 {
    fn parse_tuning_percent(&self, data: &HashMap<DataField, Value>) -> Option<u8> {
        data.get(&DataField::TuningPercent)
            .and_then(Self::parse_u64)
            .and_then(|percent| u8::try_from(percent).ok())
    }
}

impl GetTuningTarget for ElphapexV1 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        data.get(&DataField::TuningTarget)
            .and_then(|config| config.get("fc-work-mode"))
            .and_then(Self::parse_work_mode)
            .map(TuningTarget::MiningMode)
    }
}

impl GetScaledTuningTarget for ElphapexV1 {
    fn parse_scaled_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        self.parse_tuning_target(data)
    }
}
impl GetTuningCapabilities for ElphapexV1 {}

impl GetLightFlashing for ElphapexV1 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing)
    }
}

impl GetMessages for ElphapexV1 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let Some(statuses) = data
            .get(&DataField::Messages)
            .and_then(|summary| summary.pointer("/SUMMARY/0/status"))
            .and_then(Value::as_array)
        else {
            return vec![];
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;

        statuses
            .iter()
            .filter_map(|status| {
                let state = status.get("status").and_then(Value::as_str)?;
                if state.eq_ignore_ascii_case("s") {
                    return None;
                }
                let message = status
                    .get("msg")
                    .and_then(Value::as_str)
                    .unwrap_or("Elphapex status warning")
                    .to_string();
                Some(MinerMessage {
                    timestamp,
                    code: 0,
                    message,
                    severity: MessageSeverity::Error,
                    component: Some(MinerComponent::control_board()),
                })
            })
            .collect()
    }
}

impl GetUptime for ElphapexV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        let stats = data.get(&DataField::Uptime)?;
        let stats0 = stats
            .pointer("/STATS/0")
            .or_else(|| stats.get("STATS").and_then(|stats| stats.get(0)))
            .unwrap_or(stats);

        stats0
            .get("elapsed")
            .and_then(Self::parse_u64)
            .map(Duration::from_secs)
    }
}

impl GetIsMining for ElphapexV1 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        data.get(&DataField::IsMining)
            .and_then(|mode| {
                mode.as_i64()
                    .or_else(|| mode.as_str().and_then(|mode| mode.parse::<i64>().ok()))
            })
            .map(|mode| mode != 1)
            .unwrap_or_else(|| {
                self.parse_hashrate(data)
                    .map(|hashrate| hashrate.value > 0.0)
                    .unwrap_or(false)
            })
    }
}

impl GetPools for ElphapexV1 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let Some(pools_array) = data
            .get(&DataField::Pools)
            .and_then(|pools| pools.get("POOLS"))
            .and_then(Value::as_array)
        else {
            return vec![];
        };

        let active_index = pools_array
            .iter()
            .filter(|pool| {
                pool.get("status")
                    .and_then(Value::as_str)
                    .map(|status| status.eq_ignore_ascii_case("alive"))
                    .unwrap_or(false)
            })
            .filter_map(|pool| {
                let priority = pool.get("priority").and_then(Self::parse_u64)?;
                let index = pool.get("index").and_then(Self::parse_u64)?;
                Some((priority, index))
            })
            .min_by_key(|(priority, _)| *priority)
            .map(|(_, index)| index);

        let pools = pools_array
            .iter()
            .enumerate()
            .map(|(idx, pool)| {
                let index = pool
                    .get("index")
                    .and_then(Self::parse_u64)
                    .and_then(|index| u16::try_from(index).ok())
                    .or_else(|| u16::try_from(idx).ok());
                let alive = pool
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|status| status.eq_ignore_ascii_case("alive"));
                let raw_index = pool.get("index").and_then(Self::parse_u64);

                PoolData {
                    position: index,
                    url: pool
                        .get("url")
                        .and_then(Value::as_str)
                        .filter(|url| !url.is_empty())
                        .map(|url| PoolURL::from(url.to_string())),
                    accepted_shares: pool.get("accepted").and_then(Self::parse_u64),
                    rejected_shares: pool.get("rejected").and_then(Self::parse_u64),
                    active: raw_index.map(|index| Some(index) == active_index).or(alive),
                    alive,
                    user: pool
                        .get("user")
                        .and_then(Value::as_str)
                        .filter(|user| !user.is_empty())
                        .map(str::to_string),
                }
            })
            .collect();

        vec![PoolGroupData {
            name: "default".to_string(),
            quota: 1,
            pools,
        }]
    }
}

#[async_trait]
impl SetFaultLight for ElphapexV1 {
    fn supports_set_fault_light(&self) -> bool {
        true
    }

    async fn set_fault_light(&self, flashing: bool) -> Result<bool> {
        let code = self
            .web
            .blink(flashing)
            .await?
            .get("code")
            .and_then(Value::as_str)
            .map(str::to_string);
        Ok(matches!(
            (flashing, code.as_deref()),
            (true, Some("B000")) | (false, Some("B100"))
        ))
    }
}

#[async_trait]
impl SetPowerLimit for ElphapexV1 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl Restart for ElphapexV1 {
    fn supports_restart(&self) -> bool {
        true
    }

    async fn restart(&self) -> Result<bool> {
        self.web.reboot().await
    }
}

#[async_trait]
impl Pause for ElphapexV1 {
    fn supports_pause(&self) -> bool {
        false
    }
}

#[async_trait]
impl Resume for ElphapexV1 {
    fn supports_resume(&self) -> bool {
        false
    }
}

#[async_trait]
impl ChangePassword for ElphapexV1 {
    fn supports_change_password(&self) -> bool {
        false
    }
}

impl ReadLogs for ElphapexV1 {
    fn supports_read_logs(&self) -> bool {
        false
    }
}

impl FactoryReset for ElphapexV1 {
    fn supports_factory_reset(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsPoolsConfig for ElphapexV1 {
    fn parse_pools_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> Result<Vec<PoolGroupConfig>> {
        let Some(config) = data.get(&ConfigField::Pools) else {
            return Ok(vec![]);
        };

        if let Some(pools_array) = config.get("pools").and_then(Value::as_array) {
            let mut pools = Vec::new();
            for pool in pools_array {
                let Some(url) = pool
                    .get("url")
                    .and_then(Value::as_str)
                    .filter(|url| !url.is_empty())
                else {
                    continue;
                };

                pools.push(PoolConfig {
                    url: PoolURL::from(url.to_string()),
                    username: pool
                        .get("user")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    password: pool
                        .get("pass")
                        .and_then(Value::as_str)
                        .unwrap_or("x")
                        .to_string(),
                });
            }
            pools.truncate(3);

            return Ok(vec![PoolGroupConfig {
                name: "default".to_string(),
                quota: 1,
                pools,
            }]);
        }

        let mut pools = Vec::new();
        for idx in 0..3 {
            let url_keys = [format!("pool{idx}url"), format!("pool{idx}_url")];
            let user_keys = [format!("pool{idx}user"), format!("pool{idx}_user")];
            let pass_keys = [
                format!("pool{idx}pw"),
                format!("pool{idx}pass"),
                format!("pool{idx}_pass"),
            ];

            let url = url_keys
                .iter()
                .find_map(|key| config.get(key).and_then(Value::as_str))
                .filter(|url| !url.is_empty());
            let Some(url) = url else {
                continue;
            };
            let username = user_keys
                .iter()
                .find_map(|key| config.get(key).and_then(Value::as_str))
                .unwrap_or_default()
                .to_string();
            let password = pass_keys
                .iter()
                .find_map(|key| config.get(key).and_then(Value::as_str))
                .unwrap_or("x")
                .to_string();
            pools.push(PoolConfig {
                url: PoolURL::from(url.to_string()),
                username,
                password,
            });
        }

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
impl SupportsScalingConfig for ElphapexV1 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}
impl SupportsTuningConfig for ElphapexV1 {}
impl SupportsFanConfig for ElphapexV1 {}
impl SupportsTemperatureConfig for ElphapexV1 {}
impl SupportsPresets for ElphapexV1 {}
impl SupportsTimezoneConfig for ElphapexV1 {}
impl UpgradeFirmware for ElphapexV1 {}
impl SetTuningPercent for ElphapexV1 {}

impl HasDefaultAuth for ElphapexV1 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("root", "root")
    }
}

impl HasAuth for ElphapexV1 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use asic_rs_core::{
        data::{command::MinerCommand, hashrate::HashRateUnit},
        test::api::MockAPIClient,
    };
    use asic_rs_makes_elphapex::models::ElphapexModel;
    use serde_json::Value;

    use super::*;
    use crate::test::json::v1;

    const WEB_SYSTEM_INFO: MinerCommand = MinerCommand::WebAPI {
        command: "get_system_info",
        parameters: None,
    };
    const WEB_STATS: MinerCommand = MinerCommand::WebAPI {
        command: "stats",
        parameters: None,
    };
    const WEB_SUMMARY: MinerCommand = MinerCommand::WebAPI {
        command: "summary",
        parameters: None,
    };
    const WEB_POOLS: MinerCommand = MinerCommand::WebAPI {
        command: "pools",
        parameters: None,
    };
    const WEB_BLINK: MinerCommand = MinerCommand::WebAPI {
        command: "get_blink_status",
        parameters: None,
    };
    const WEB_MINER_CONF: MinerCommand = MinerCommand::WebAPI {
        command: "get_miner_conf",
        parameters: None,
    };
    const WEB_POWER: MinerCommand = MinerCommand::WebAPI {
        command: "get_power",
        parameters: None,
    };

    fn fixture_json(data: &str) -> anyhow::Result<Value> {
        serde_json::from_str(data).context("fixture JSON is invalid")
    }

    #[test]
    fn set_auth_updates_web_client_auth() {
        let mut miner = ElphapexV1::new(IpAddr::from([127, 0, 0, 1]), ElphapexModel::DG1Home);
        let auth = MinerAuth::new("admin", "secret");

        miner.set_auth(auth);

        assert_eq!(miner.web_auth().username(), "admin");
        assert_eq!(miner.web_auth().password(), "secret");
    }

    #[tokio::test]
    async fn test_elphapex_parse_dg_home1_data() -> anyhow::Result<()> {
        let miner = ElphapexV1::new(IpAddr::from([127, 0, 0, 1]), ElphapexModel::DG1Home);
        let mut results = HashMap::new();
        results.insert(WEB_SYSTEM_INFO, fixture_json(v1::GET_SYSTEM_INFO)?);
        results.insert(WEB_STATS, fixture_json(v1::STATS)?);
        results.insert(WEB_SUMMARY, fixture_json(v1::SUMMARY)?);
        results.insert(WEB_POOLS, fixture_json(v1::POOLS)?);
        results.insert(WEB_BLINK, fixture_json(v1::GET_BLINK_STATUS)?);
        results.insert(WEB_MINER_CONF, fixture_json(v1::GET_MINER_CONF)?);
        results.insert(WEB_POWER, fixture_json(v1::GET_POWER)?);

        let mock_api = MockAPIClient::new(results);
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;
        let miner_data = miner.parse_data(data);

        assert_eq!(
            miner_data.mac,
            Some(MacAddr::from_str("b8:4c:87:e0:66:b2")?)
        );
        assert_eq!(
            miner_data.serial_number.as_deref(),
            Some("13HY24B056N300037C11B")
        );
        assert_eq!(miner_data.firmware_version.as_deref(), Some("1.0.5"));
        assert_eq!(
            miner_data.control_board_version,
            Some(ElphapexControlBoard::DGHome1.into())
        );
        assert_eq!(miner_data.api_version.as_deref(), Some("1.0.0"));
        assert_eq!(miner_data.hashrate.as_ref().map(|h| h.value), Some(0.0));
        assert_eq!(
            miner_data.hashrate.as_ref().map(|h| h.unit),
            Some(HashRateUnit::MegaHash)
        );
        assert_eq!(
            miner_data.expected_hashrate.as_ref().map(|h| h.value),
            Some(3003.75)
        );
        assert_eq!(miner_data.wattage, Some(Power::from_watts(25.0)));
        assert_eq!(miner_data.tuning_percent, Some(100));
        assert_eq!(
            miner_data.tuning_target,
            Some(TuningTarget::MiningMode(MiningMode::Normal))
        );
        assert_eq!(miner_data.scaled_tuning_target, miner_data.tuning_target);
        assert_eq!(
            miner_data.average_temperature.map(|temp| temp.as_celsius()),
            Some(56.5)
        );
        assert_eq!(
            miner_data.efficiency, None,
            "zero hashrate fixture cannot produce efficiency"
        );
        assert_eq!(miner_data.uptime, Some(Duration::from_secs(0)));
        assert!(miner_data.is_mining);
        assert_eq!(miner_data.fans.len(), 4);
        assert_eq!(miner_data.messages.len(), 2);

        assert_eq!(miner_data.hashboards.len(), 4);
        assert_eq!(miner_data.hashboards[0].expected_chips, Some(120));
        assert_eq!(miner_data.hashboards[3].expected_chips, Some(120));
        assert_eq!(
            miner_data.hashboards[3]
                .expected_hashrate
                .as_ref()
                .map(|h| h.value),
            Some(3003.75)
        );
        assert_eq!(miner_data.hashboards[0].working_chips, Some(0));
        assert_eq!(miner_data.hashboards[0].active, Some(false));
        assert_eq!(miner_data.hashboards[3].working_chips, Some(120));
        assert_eq!(miner_data.hashboards[3].active, Some(true));
        let inlet_chip_temp = miner_data.hashboards[3]
            .inlet_chip_temperature
            .map(|temp| temp.as_celsius())
            .context("missing inlet chip temperature")?;
        assert!((inlet_chip_temp - 25.437).abs() < 1e-9);
        assert_eq!(
            miner_data.hashboards[3].serial_number.as_deref(),
            Some("10HY24B046N300036H10JC53")
        );
        assert_eq!(
            miner_data.hashboards[3].hashrate.as_ref().map(|h| h.value),
            Some(0.0)
        );

        let pools = miner_data
            .pools
            .first()
            .context("missing default pool group")?;
        assert_eq!(pools.pools.len(), 3);
        assert_eq!(
            pools.pools[0]
                .url
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("stratum+tcp://pool.invalid:3333")
        );
        assert_eq!(pools.pools[0].user.as_deref(), Some("worker"));
        assert_eq!(pools.pools[0].accepted_shares, Some(0));
        assert_eq!(pools.pools[0].rejected_shares, Some(0));
        assert_eq!(pools.pools[0].active, Some(false));
        assert_eq!(pools.pools[1].active, Some(false));

        Ok(())
    }

    #[test]
    fn test_parse_pools_config() -> anyhow::Result<()> {
        let miner = ElphapexV1::new(IpAddr::from([127, 0, 0, 1]), ElphapexModel::DG1Home);
        let mut data = HashMap::new();
        data.insert(ConfigField::Pools, fixture_json(v1::GET_MINER_CONF)?);

        let groups = miner.parse_pools_config(&data)?;

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].pools.len(), 3);
        assert_eq!(
            groups[0].pools[0].url.to_string(),
            "stratum+tcp://pool.invalid:3333"
        );
        assert_eq!(groups[0].pools[0].username, "worker");
        assert_eq!(groups[0].pools[0].password, "x");
        assert_eq!(
            groups[0].pools[1].url.to_string(),
            "stratum+tcp://backup.invalid:3333"
        );
        assert_eq!(groups[0].pools[1].username, "worker");
        assert_eq!(groups[0].pools[1].password, "x");

        Ok(())
    }
}

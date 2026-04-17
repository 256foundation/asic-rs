use std::{collections::HashMap, net::IpAddr, str::FromStr, time::Duration};

use anyhow;
use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigField, ConfigLocation},
        pools::PoolGroupConfig,
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
        message::{MessageSeverity, MinerMessage},
        miner::TuningTarget,
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_antminer::hardware::AntMinerControlBoard;
use asic_rs_makes_braiins::hardware::BraiinsControlBoard;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use reqwest::Method;
use serde_json::{Value, json};
use web::BraiinsWebAPI;

use crate::firmware::BraiinsFirmware;

pub mod web;

#[derive(Debug)]
pub struct BraiinsV2507 {
    pub ip: IpAddr,
    pub web: BraiinsWebAPI,
    pub device_info: DeviceInfo,
}

impl BraiinsV2507 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        let auth = Self::default_auth();
        BraiinsV2507 {
            ip,
            web: BraiinsWebAPI::new(ip, auth),
            device_info: DeviceInfo::new(model, BraiinsFirmware::default(), HashAlgorithm::SHA256),
        }
    }
}

#[async_trait]
impl APIClient for BraiinsV2507 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!("Unsupported command type for Braiins API")),
        }
    }
}

impl GetConfigsLocations for BraiinsV2507 {
    #[allow(unused_variables)]
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}

impl CollectConfigs for BraiinsV2507 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for BraiinsV2507 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const WEB_NETWORK: MinerCommand = MinerCommand::WebAPI {
            command: "network",
            parameters: None,
        };
        const WEB_VERSION: MinerCommand = MinerCommand::WebAPI {
            command: "version",
            parameters: None,
        };
        const WEB_MINER_DETAILS: MinerCommand = MinerCommand::WebAPI {
            command: "miner/details",
            parameters: None,
        };
        const WEB_LOCATE: MinerCommand = MinerCommand::WebAPI {
            command: "actions/locate",
            parameters: None,
        };
        const WEB_MINER_STATS: MinerCommand = MinerCommand::WebAPI {
            command: "miner/stats",
            parameters: None,
        };
        const WEB_PERFORMANCE_TUNER_STATE: MinerCommand = MinerCommand::WebAPI {
            command: "performance/tuner-state",
            parameters: None,
        };
        const WEB_MINER_ERRORS: MinerCommand = MinerCommand::WebAPI {
            command: "miner/errors",
            parameters: None,
        };
        const WEB_POOLS: MinerCommand = MinerCommand::WebAPI {
            command: "pools",
            parameters: None,
        };
        const WEB_COOLING_STATE: MinerCommand = MinerCommand::WebAPI {
            command: "cooling/state",
            parameters: None,
        };
        const WEB_HASHBOARDS: MinerCommand = MinerCommand::WebAPI {
            command: "miner/hw/hashboards",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                WEB_NETWORK,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/mac_address"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                WEB_NETWORK,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/hostname"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![(
                WEB_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                WEB_MINER_DETAILS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bos_version/current"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                WEB_MINER_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner_stats/real_hashrate/last_5s/gigahash_per_second"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                WEB_MINER_DETAILS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/sticker_hashrate/gigahash_per_second"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                WEB_COOLING_STATE,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/fans"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![(
                WEB_HASHBOARDS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/hashboards"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                WEB_LOCATE,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            DataField::IsMining => vec![(
                WEB_MINER_DETAILS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/status"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                WEB_MINER_DETAILS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system_uptime_s"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                WEB_MINER_DETAILS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/control_board_soc_family"),
                    tag: None,
                },
            )],
            DataField::Pools => vec![(
                WEB_POOLS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/0/pools"), // assuming there is 1 pool group
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                WEB_MINER_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/power_stats/approximated_consumption/watt"),
                    tag: None,
                },
            )],
            DataField::TuningTarget => vec![(
                WEB_PERFORMANCE_TUNER_STATE,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/mode_state/powertargetmodestate/current_target/watt"),
                    tag: None,
                },
            )],
            DataField::SerialNumber => vec![(
                WEB_MINER_DETAILS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/serial_number"),
                    tag: None,
                },
            )],
            DataField::Messages => vec![(
                WEB_MINER_ERRORS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/errors"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for BraiinsV2507 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for BraiinsV2507 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for BraiinsV2507 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for BraiinsV2507 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetHostname for BraiinsV2507 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for BraiinsV2507 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        let major = data.extract_nested::<f64>(DataField::ApiVersion, "major");
        let minor = data.extract_nested::<f64>(DataField::ApiVersion, "minor");
        let patch = data.extract_nested::<f64>(DataField::ApiVersion, "patch");

        Some(format!("{}.{}.{}", major?, minor?, patch?))
    }
}

impl GetFirmwareVersion for BraiinsV2507 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetHashboards for BraiinsV2507 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> = (0..self.device_info.hardware.boards.unwrap_or(0))
            .map(|idx| BoardData::new(idx, self.device_info.hardware.chips))
            .collect();

        let Some(chains_array) = data.get(&DataField::Hashboards).and_then(|v| v.as_array()) else {
            return hashboards;
        };

        for board in hashboards.iter_mut() {
            let Some(chain) = chains_array.iter().find(|c| {
                c.pointer("/id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u8>().ok())
                    .is_some_and(|id| id == board.position + 1)
            }) else {
                continue;
            };

            let chip_temperature = chain
                .pointer("/highest_chip_temp/temperature/degree_c")
                .and_then(|v| v.as_f64())
                .map(Temperature::from_celsius);

            board.hashrate = chain
                .pointer("/stats/real_hashrate/last_5s/gigahash_per_second")
                .and_then(|v| v.as_f64())
                .map(|f| {
                    HashRate {
                        value: f,
                        unit: HashRateUnit::GigaHash,
                        algo: "SHA256".to_string(),
                    }
                    .as_unit(HashRateUnit::default())
                });
            board.expected_hashrate = chain
                .pointer("/stats/nominal_hashrate/gigahash_per_second")
                .and_then(|v| v.as_f64())
                .map(|f| {
                    HashRate {
                        value: f,
                        unit: HashRateUnit::GigaHash,
                        algo: "SHA256".to_string(),
                    }
                    .as_unit(HashRateUnit::default())
                });
            board.board_temperature = chain
                .pointer("/board_temp/degree_c")
                .and_then(|v| v.as_f64())
                .map(Temperature::from_celsius);
            board.intake_temperature = chip_temperature;
            board.outlet_temperature = chip_temperature;
            board.working_chips = chain
                .pointer("/chips_count")
                .and_then(|v| v.as_u64())
                .map(|u| u as u16);
            board.serial_number = chain
                .pointer("/serial_number")
                .and_then(|v| v.as_str())
                .map(|u| u.to_string());
            board.voltage = chain
                .pointer("/current_voltage/volt")
                .and_then(|v| v.as_f64())
                .map(Voltage::from_volts);
            board.frequency = chain
                .pointer("/current_frequency/hertz")
                .and_then(|v| v.as_f64())
                .map(Frequency::from_hertz);
            board.active = chain.pointer("/enabled").and_then(|v| v.as_bool());
        }

        hashboards
    }
}

impl GetHashrate for BraiinsV2507 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(),
        })
    }
}

impl GetExpectedHashrate for BraiinsV2507 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(),
        })
    }
}

impl GetFans for BraiinsV2507 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans: Vec<FanData> = Vec::new();

        if let Some(fans_data) = data.get(&DataField::Fans)
            && let Some(fans_array) = fans_data.as_array()
        {
            for (idx, fan) in fans_array.iter().enumerate() {
                if let Some(rpm) = fan.pointer("/rpm").and_then(|v| v.as_i64()) {
                    let pos = fan
                        .pointer("/position")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(idx as i64);
                    fans.push(FanData {
                        position: pos as i16,
                        rpm: Some(AngularVelocity::from_rpm(rpm as f64)),
                    });
                }
            }
        }

        fans
    }
}

impl GetLightFlashing for BraiinsV2507 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing)
    }
}

impl GetUptime for BraiinsV2507 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetIsMining for BraiinsV2507 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        // 1 -> Not Started
        // 2 -> Normal
        // 3 -> Paused
        // 4 -> Suspended
        // See: https://github.com/braiins/bos-plus-api/blob/ef28e752f80711c54d5587ec8f2cd838fdb34042/proto/bos/v1/miner.proto#L117-L124
        data.extract::<u64>(DataField::IsMining) == Some(2)
    }
}

impl GetPools for BraiinsV2507 {
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

                let accepted_shares = pool
                    .pointer("/stats/accepted_shares")
                    .and_then(|v| v.as_u64());
                let rejected_shares = pool
                    .pointer("/stats/rejected_shares")
                    .and_then(|v| v.as_u64());
                let active = pool.pointer("/active").and_then(|v| v.as_bool());
                let alive = pool.pointer("/alive").and_then(|v| v.as_bool());

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

impl GetSerialNumber for BraiinsV2507 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::SerialNumber)
    }
}

impl GetControlBoardVersion for BraiinsV2507 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        let cb_type = data.extract::<u64>(DataField::ControlBoardVersion)?;
        match cb_type {
            0 => None,
            1 => Some(AntMinerControlBoard::CVITek).map(|cb| cb.into()),
            2 => Some(AntMinerControlBoard::BeagleBoneBlack).map(|cb| cb.into()),
            3 => Some(AntMinerControlBoard::AMLogic).map(|cb| cb.into()),
            4 => Some(AntMinerControlBoard::Xilinx).map(|cb| cb.into()),
            5 => Some(BraiinsControlBoard::BraiinsCB).map(|cb| cb.into()),
            _ => None,
        }
    }
}

impl GetWattage for BraiinsV2507 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<i64, _>(DataField::Wattage, |w| Power::from_watts(w as f64))
    }
}

impl GetTuningTarget for BraiinsV2507 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        data.extract_map::<i64, _>(DataField::TuningTarget, |w| Power::from_watts(w as f64))
            .map(TuningTarget::Power)
    }
}

impl GetFluidTemperature for BraiinsV2507 {}

impl GetPsuFans for BraiinsV2507 {}

impl GetMessages for BraiinsV2507 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let mut messages: Vec<MinerMessage> = Vec::new();

        if let Some(errors_data) = data.get(&DataField::Messages)
            && let Some(errors_array) = errors_data.as_array()
        {
            for error in errors_array.iter() {
                let timestamp = error
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|dt| dt.parse::<DateTime<Utc>>().ok())
                    .map(|dt| dt.timestamp() as u32);
                let message = error.get("message").and_then(|v| v.as_str());
                if let Some(ts) = timestamp {
                    messages.push(MinerMessage::new(
                        ts,
                        0, // They have codes, but they include a string
                        message.unwrap_or("Unknown error").to_string(),
                        MessageSeverity::Error,
                    ))
                }
            }
        };

        messages
    }
}

#[async_trait]
impl SetFaultLight for BraiinsV2507 {
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        Ok(self
            .web
            .send_command("actions/locate", true, Some(json!(fault)), Method::PUT)
            .await
            .is_ok())
    }
    fn supports_set_fault_light(&self) -> bool {
        true
    }
}

#[async_trait]
impl SetPowerLimit for BraiinsV2507 {
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        Ok(self
            .web
            .send_command(
                "performance/power-target",
                true,
                Some(json!({"watt": limit.as_watts() as u64})),
                Method::PUT,
            )
            .await
            .is_ok())
    }
    fn supports_set_power_limit(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsPoolsConfig for BraiinsV2507 {
    async fn get_pools_config(&self) -> anyhow::Result<Vec<PoolGroupConfig>> {
        Ok(self
            .get_pools()
            .await
            .iter()
            .map(|g| g.clone().into())
            .collect())
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> anyhow::Result<bool> {
        let groups: Vec<Value> = config
            .iter()
            .map(|group| {
                let pools: Vec<Value> = group
                    .pools
                    .iter()
                    .map(|pool| {
                        json!({
                            "url": pool.url.to_string(),
                            "user": pool.username.as_str(),
                            "password": pool.password.as_str(),
                        })
                    })
                    .collect();
                json!({
                    "name": group.name,
                    "pools": pools,
                    "load_balance_strategy": {
                        "quota": { "value": group.quota }
                    },
                })
            })
            .collect();

        Ok(self
            .web
            .send_command("pools/batch", true, Some(json!(groups)), Method::PUT)
            .await
            .is_ok())
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for BraiinsV2507 {
    async fn restart(&self) -> anyhow::Result<bool> {
        Ok(self
            .web
            .send_command("actions/reboot", true, None, Method::PUT)
            .await
            .is_ok())
    }
    fn supports_restart(&self) -> bool {
        true
    }
}

#[async_trait]
impl Pause for BraiinsV2507 {
    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        Ok(self
            .web
            .send_command("actions/pause", true, None, Method::PUT)
            .await
            .is_ok())
    }
    fn supports_pause(&self) -> bool {
        true
    }
}

#[async_trait]
impl Resume for BraiinsV2507 {
    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        Ok(self
            .web
            .send_command("actions/resume", true, None, Method::PUT)
            .await
            .is_ok())
    }
    fn supports_resume(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsScalingConfig for BraiinsV2507 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for BraiinsV2507 {
    fn supports_upgrade_firmware(&self) -> bool {
        false
    }
}

impl HasDefaultAuth for BraiinsV2507 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("root", "root")
    }
}

impl HasAuth for BraiinsV2507 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

#[async_trait]
impl SupportsTuningConfig for BraiinsV2507 {
    fn supports_tuning_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsFanConfig for BraiinsV2507 {
    fn supports_fan_config(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use asic_rs_core::{data::collector::DataCollector, test::api::MockAPIClient};
    use asic_rs_makes_antminer::models::AntMinerModel;
    use macaddr::MacAddr;
    use measurements::Power;

    use super::*;
    use crate::test::json::v25_07::{
        WEB_COOLING_STATE_COMMAND as V07_COOLING, WEB_HASHBOARDS_COMMAND as V07_BOARDS,
        WEB_LOCATE_COMMAND as V07_LOCATE, WEB_MINER_DETAILS_COMMAND as V07_DETAILS,
        WEB_MINER_ERRORS_COMMAND as V07_ERRORS, WEB_MINER_STATS_COMMAND as V07_STATS,
        WEB_NETWORK_COMMAND as V07_NETWORK, WEB_PERFORMANCE_TUNER_STATE_COMMAND as V07_TUNER,
        WEB_POOLS_COMMAND as V07_POOLS, WEB_VERSION_COMMAND as V07_VERSION,
    };
    use crate::test::json::v25_11::{
        WEB_COOLING_STATE_COMMAND as V11_COOLING, WEB_HASHBOARDS_COMMAND as V11_BOARDS,
        WEB_LOCATE_COMMAND as V11_LOCATE, WEB_MINER_DETAILS_COMMAND as V11_DETAILS,
        WEB_MINER_ERRORS_COMMAND as V11_ERRORS, WEB_MINER_STATS_COMMAND as V11_STATS,
        WEB_NETWORK_COMMAND as V11_NETWORK, WEB_PERFORMANCE_TUNER_STATE_COMMAND as V11_TUNER,
        WEB_POOLS_COMMAND as V11_POOLS, WEB_VERSION_COMMAND as V11_VERSION,
    };

    #[tokio::test]
    async fn test_braiins_v25_07() {
        let miner = BraiinsV2507::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP);

        let mut results = HashMap::new();
        results.insert(
            MinerCommand::WebAPI {
                command: "network",
                parameters: None,
            },
            Value::from_str(V07_NETWORK).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "version",
                parameters: None,
            },
            Value::from_str(V07_VERSION).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "miner/details",
                parameters: None,
            },
            Value::from_str(V07_DETAILS).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "miner/stats",
                parameters: None,
            },
            Value::from_str(V07_STATS).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "performance/tuner-state",
                parameters: None,
            },
            Value::from_str(V07_TUNER).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "miner/errors",
                parameters: None,
            },
            Value::from_str(V07_ERRORS).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "pools",
                parameters: None,
            },
            Value::from_str(V07_POOLS).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "cooling/state",
                parameters: None,
            },
            Value::from_str(V07_COOLING).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "miner/hw/hashboards",
                parameters: None,
            },
            Value::from_str(V07_BOARDS).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "actions/locate",
                parameters: None,
            },
            Value::from_str(V07_LOCATE).unwrap(),
        );

        let mock_api = MockAPIClient::new(results);
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let miner_data = miner.parse_data(collector.collect_all().await);

        assert_eq!(miner_data.ip.to_string(), "127.0.0.1");
        assert_eq!(
            miner_data.mac,
            Some(MacAddr::from_str("02:ee:da:a9:b6:4a").unwrap())
        );
        assert_eq!(miner_data.hostname, Some("Antminer".to_owned()));
        assert_eq!(miner_data.api_version, Some("1.0.0".to_owned()));
        assert_eq!(
            miner_data.firmware_version,
            Some("2025-08-06-0-3881c51d-25.07-plus".to_owned())
        );
        assert_eq!(miner_data.serial_number, None);
        assert_eq!(miner_data.hashboards.len(), 3);
        assert_eq!(miner_data.fans.len(), 4);
        assert_eq!(miner_data.light_flashing, Some(false));
        assert!(miner_data.is_mining);
        assert_eq!(miner_data.wattage, Some(Power::from_watts(3137.0)));
        assert_eq!(
            miner_data.tuning_target,
            Some(TuningTarget::Power(Power::from_watts(3031.0)))
        );
        assert_eq!(miner_data.pools.len(), 1);
        assert_eq!(miner_data.pools[0].len(), 2);
        assert!(miner_data.hashrate.is_some());
        assert!(miner_data.expected_hashrate.is_some());
    }

    #[tokio::test]
    async fn test_braiins_v25_11() {
        let miner = BraiinsV2507::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP);

        let mut results = HashMap::new();
        results.insert(
            MinerCommand::WebAPI {
                command: "network",
                parameters: None,
            },
            Value::from_str(V11_NETWORK).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "version",
                parameters: None,
            },
            Value::from_str(V11_VERSION).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "miner/details",
                parameters: None,
            },
            Value::from_str(V11_DETAILS).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "miner/stats",
                parameters: None,
            },
            Value::from_str(V11_STATS).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "performance/tuner-state",
                parameters: None,
            },
            Value::from_str(V11_TUNER).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "miner/errors",
                parameters: None,
            },
            Value::from_str(V11_ERRORS).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "pools",
                parameters: None,
            },
            Value::from_str(V11_POOLS).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "cooling/state",
                parameters: None,
            },
            Value::from_str(V11_COOLING).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "miner/hw/hashboards",
                parameters: None,
            },
            Value::from_str(V11_BOARDS).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "actions/locate",
                parameters: None,
            },
            Value::from_str(V11_LOCATE).unwrap(),
        );

        let mock_api = MockAPIClient::new(results);
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let miner_data = miner.parse_data(collector.collect_all().await);

        assert_eq!(miner_data.ip.to_string(), "127.0.0.1");
        assert_eq!(
            miner_data.mac,
            Some(MacAddr::from_str("90:34:86:54:14:c0").unwrap())
        );
        assert_eq!(miner_data.hostname, Some("Antminer".to_owned()));
        assert_eq!(miner_data.api_version, Some("1.1.0".to_owned()));
        assert_eq!(
            miner_data.firmware_version,
            Some("2025-11-21-0-eb658dcd-25.11-plus".to_owned())
        );
        assert_eq!(
            miner_data.serial_number,
            Some("YNAHEAUBCJCBA03GL".to_owned())
        );
        assert_eq!(miner_data.hashboards.len(), 3);
        assert_eq!(miner_data.fans.len(), 4);
        assert_eq!(miner_data.light_flashing, Some(false));
        assert!(miner_data.is_mining);
        assert_eq!(miner_data.wattage, Some(Power::from_watts(2472.0)));
        assert_eq!(
            miner_data.tuning_target,
            Some(TuningTarget::Power(Power::from_watts(2431.0)))
        );
        assert_eq!(miner_data.pools.len(), 1);
        assert_eq!(miner_data.pools[0].len(), 2);
        assert!(miner_data.hashrate.is_some());
        assert!(miner_data.expected_hashrate.is_some());
    }
}

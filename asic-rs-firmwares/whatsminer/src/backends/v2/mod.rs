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
use asic_rs_makes_whatsminer::hardware::WhatsMinerControlBoard;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature};
use rpc::WhatsMinerRPCAPI;
use serde_json::{Value, json};

use crate::firmware::WhatsMinerFirmware;

mod rpc;

#[derive(Debug)]
pub struct WhatsMinerV2 {
    pub ip: IpAddr,
    pub rpc: WhatsMinerRPCAPI,
    pub device_info: DeviceInfo,
}

impl WhatsMinerV2 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        WhatsMinerV2 {
            ip,
            rpc: WhatsMinerRPCAPI::new(ip, None),
            device_info: DeviceInfo::new(
                model,
                WhatsMinerFirmware::default(),
                HashAlgorithm::SHA256,
            ),
        }
    }
}

impl GetConfigsLocations for WhatsMinerV2 {
    #[allow(unused_variables)]
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}

impl CollectConfigs for WhatsMinerV2 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

#[async_trait]
impl APIClient for WhatsMinerV2 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for WhatsMiner API"
            )),
        }
    }
}

impl GetDataLocations for WhatsMinerV2 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const RPC_GET_MINER_INFO: MinerCommand = MinerCommand::RPC {
            command: "get_miner_info",
            parameters: None,
        };
        const RPC_SUMMARY: MinerCommand = MinerCommand::RPC {
            command: "summary",
            parameters: None,
        };
        const RPC_DEVS: MinerCommand = MinerCommand::RPC {
            command: "devs",
            parameters: None,
        };
        const RPC_POOLS: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };
        const RPC_STATUS: MinerCommand = MinerCommand::RPC {
            command: "status",
            parameters: None,
        };
        const RPC_GET_VERSION: MinerCommand = MinerCommand::RPC {
            command: "get_version",
            parameters: None,
        };
        const RPC_GET_PSU: MinerCommand = MinerCommand::RPC {
            command: "get_psu",
            parameters: None,
        };
        const RPC_GET_ERROR_CODE: MinerCommand = MinerCommand::RPC {
            command: "get_error_code",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                RPC_GET_MINER_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Msg/mac"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![(
                RPC_GET_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Msg/api_ver"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                RPC_GET_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Msg/fw_ver"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                RPC_GET_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Msg/platform"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                RPC_GET_MINER_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Msg/hostname"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                RPC_GET_MINER_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Msg/ledstat"),
                    tag: None,
                },
            )],
            DataField::TuningTarget => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/Power Limit"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0"),
                    tag: None,
                },
            )],
            DataField::PsuFans => vec![(
                RPC_GET_PSU,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Msg/fan_speed"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![(
                RPC_DEVS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            DataField::Pools => vec![(
                RPC_POOLS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/POOLS"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/Elapsed"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/Power"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/HS RT"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/Factory GHS"),
                    tag: None,
                },
            )],
            DataField::FluidTemperature => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/Env Temp"),
                    tag: None,
                },
            )],
            DataField::IsMining => vec![
                (
                    RPC_STATUS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/SUMMARY/0/btmineroff"),
                        tag: None,
                    },
                ),
                (
                    RPC_STATUS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/Msg/mineroff"),
                        tag: None,
                    },
                ),
            ],
            DataField::Messages => vec![(
                RPC_GET_ERROR_CODE,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Msg/error_code"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for WhatsMinerV2 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}
impl GetDeviceInfo for WhatsMinerV2 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for WhatsMinerV2 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for WhatsMinerV2 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetSerialNumber for WhatsMinerV2 {}
impl GetHostname for WhatsMinerV2 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}
impl GetApiVersion for WhatsMinerV2 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}
impl GetFirmwareVersion for WhatsMinerV2 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}
impl GetControlBoardVersion for WhatsMinerV2 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| WhatsMinerControlBoard::parse(&s).map(|cb| cb.into()))
    }
}
impl GetHashboards for WhatsMinerV2 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> = Vec::new();
        let board_count = self.device_info.hardware.boards.unwrap_or(3);
        let hashboard_data = data.get(&DataField::Hashboards);

        for idx in 0..board_count {
            let hashrate = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{idx}/MHS av")))
                .and_then(|val| val.as_f64())
                .map(|f| {
                    HashRate {
                        value: f,
                        unit: HashRateUnit::MegaHash,
                        algo: String::from("SHA256"),
                    }
                    .as_unit(HashRateUnit::TeraHash)
                });
            let expected_hashrate = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{idx}/Factory GHS")))
                .and_then(|val| val.as_f64())
                .map(|f| {
                    HashRate {
                        value: f,
                        unit: HashRateUnit::GigaHash,
                        algo: String::from("SHA256"),
                    }
                    .as_unit(HashRateUnit::TeraHash)
                });
            let board_temperature = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{idx}/Temperature")))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let intake_temperature = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{idx}/Chip Temp Min")))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let outlet_temperature = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{idx}/Chip Temp Max")))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let serial_number = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{idx}/PCB SN")))
                .and_then(|val| val.as_str())
                .map(String::from);
            let working_chips = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{idx}/Effective Chips")))
                .and_then(|val| val.as_u64())
                .map(|u| u as u16);
            let frequency = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{idx}/Frequency")))
                .and_then(|val| val.as_f64())
                .map(Frequency::from_megahertz);

            let active = Some(hashrate.clone().map(|h| h.value).unwrap_or(0f64) > 0f64);
            hashboards.push(BoardData {
                hashrate,
                position: idx,
                expected_hashrate,
                board_temperature,
                intake_temperature,
                outlet_temperature,
                expected_chips: self.device_info.hardware.chips,
                working_chips,
                serial_number,
                chips: vec![],
                voltage: None, // TODO
                frequency,
                tuned: Some(true),
                active,
            });
        }
        hashboards
    }
}
impl GetHashrate for WhatsMinerV2 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| {
            HashRate {
                value: f,
                unit: HashRateUnit::MegaHash,
                algo: String::from("SHA256"),
            }
            .as_unit(HashRateUnit::TeraHash)
        })
    }
}
impl GetExpectedHashrate for WhatsMinerV2 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| {
            HashRate {
                value: f,
                unit: HashRateUnit::GigaHash,
                algo: String::from("SHA256"),
            }
            .as_unit(HashRateUnit::TeraHash)
        })
    }
}
impl GetFans for WhatsMinerV2 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans: Vec<FanData> = Vec::new();
        for (idx, direction) in ["In", "Out"].iter().enumerate() {
            let fan = data.extract_nested_map::<f64, _>(
                DataField::Fans,
                &format!("Fan Speed {direction}"),
                |rpm| FanData {
                    position: idx as i16,
                    rpm: Some(AngularVelocity::from_rpm(rpm)),
                },
            );
            if let Some(f) = fan {
                fans.push(f)
            }
        }
        fans
    }
}
impl GetPsuFans for WhatsMinerV2 {
    fn parse_psu_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut psu_fans: Vec<FanData> = Vec::new();

        let psu_fan = data.extract_map::<String, _>(DataField::PsuFans, |rpm| FanData {
            position: 0i16,
            rpm: Some(AngularVelocity::from_rpm(rpm.parse().unwrap())),
        });
        if let Some(f) = psu_fan {
            psu_fans.push(f)
        }
        psu_fans
    }
}
impl GetFluidTemperature for WhatsMinerV2 {
    fn parse_fluid_temperature(&self, data: &HashMap<DataField, Value>) -> Option<Temperature> {
        data.extract_map::<f64, _>(DataField::FluidTemperature, Temperature::from_celsius)
    }
}
impl GetWattage for WhatsMinerV2 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}
impl GetTuningTarget for WhatsMinerV2 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        data.extract_map::<f64, _>(DataField::TuningTarget, Power::from_watts)
            .map(TuningTarget::Power)
    }
}
impl GetLightFlashing for WhatsMinerV2 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract_map::<String, _>(DataField::LightFlashing, |l| l != "auto")
    }
}
impl GetMessages for WhatsMinerV2 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let mut messages = Vec::new();

        let errors_raw = data.get(&DataField::Messages);

        if let Some(errors_response) = errors_raw {
            for obj in errors_response.as_array().unwrap_or(&Vec::new()).iter() {
                let object = obj.as_object();
                if let Some(obj) = object {
                    for (code, time) in obj.iter() {
                        let timestamp = NaiveDateTime::parse_from_str(
                            time.as_str().unwrap(),
                            "%Y-%m-%d %H:%M:%S",
                        )
                        .map(|t| DateTime::<Utc>::from_naive_utc_and_offset(t, Utc))
                        .map(|dt| dt.timestamp_millis() as u32);

                        if let Ok(ts) = timestamp {
                            messages.push(MinerMessage {
                                timestamp: ts,
                                code: code.parse::<u64>().unwrap_or(0),
                                message: "".to_string(),
                                severity: MessageSeverity::Error,
                            })
                        }
                    }
                }
            }
        }

        messages
    }
}
impl GetUptime for WhatsMinerV2 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}
impl GetIsMining for WhatsMinerV2 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        // Raw field is "mineroff" / "btmineroff": "true" means mining is OFF.
        let miner_off = data.extract::<String>(DataField::IsMining);
        miner_off.as_deref() != Some("true")
    }
}
impl GetPools for WhatsMinerV2 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let mut pools: Vec<PoolData> = Vec::new();
        let pools_raw = data.get(&DataField::Pools);
        if let Some(pools_response) = pools_raw {
            for (idx, _) in pools_response
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .enumerate()
            {
                let user = pools_raw
                    .and_then(|val| val.pointer(&format!("/{idx}/User")))
                    .map(|val| String::from(val.as_str().unwrap_or("")));

                let alive = pools_raw
                    .and_then(|val| val.pointer(&format!("/{idx}/Status")))
                    .map(|val| val.as_str())
                    .map(|val| val == Some("Alive"));

                let active = pools_raw
                    .and_then(|val| val.pointer(&format!("/{idx}/Stratum Active")))
                    .and_then(|val| val.as_bool());

                let url = pools_raw
                    .and_then(|val| val.pointer(&format!("/{idx}/URL")))
                    .map(|val| PoolURL::from(String::from(val.as_str().unwrap_or(""))));

                let accepted_shares = pools_raw
                    .and_then(|val| val.pointer(&format!("/{idx}/Accepted")))
                    .and_then(|val| val.as_u64());

                let rejected_shares = pools_raw
                    .and_then(|val| val.pointer(&format!("/{idx}/Rejected")))
                    .and_then(|val| val.as_u64());

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

#[async_trait]
impl SetFaultLight for WhatsMinerV2 {
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        let parameters = match fault {
            false => Some(json!({"param": "auto"})),
            true => Some(json!({"color": "red", "period": 200, "duration": 100, "start": 0})),
        };

        let data = self.rpc.send_command("set_led", true, parameters).await;
        Ok(data.is_ok())
    }
    fn supports_set_fault_light(&self) -> bool {
        true
    }
}

#[async_trait]
impl SetPowerLimit for WhatsMinerV2 {
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        let parameters = Some(json!({"power_limit": limit.as_watts().to_string()}));
        let data = self
            .rpc
            .send_command("adjust_power_limit", true, parameters)
            .await;
        Ok(data.is_ok())
    }
    fn supports_set_power_limit(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsPoolsConfig for WhatsMinerV2 {
    async fn get_pools_config(&self) -> anyhow::Result<Vec<PoolGroupConfig>> {
        Ok(self
            .get_pools()
            .await
            .iter()
            .map(|g| g.clone().into())
            .collect())
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> anyhow::Result<bool> {
        let group = config
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No pool groups provided"))?;

        let mut params = serde_json::Map::new();
        for n in 1..=3 {
            let pool = group.pools.get(n - 1);
            params.insert(
                format!("pool{n}"),
                json!(pool.map(|p| p.url.to_string()).unwrap_or_default()),
            );
            params.insert(
                format!("worker{n}"),
                json!(pool.map(|p| p.username.as_str()).unwrap_or_default()),
            );
            params.insert(
                format!("passwd{n}"),
                json!(pool.map(|p| p.password.as_str()).unwrap_or_default()),
            );
        }

        Ok(self
            .rpc
            .send_command("update_pools", true, Some(json!(params)))
            .await
            .is_ok())
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for WhatsMinerV2 {
    async fn restart(&self) -> anyhow::Result<bool> {
        let data = self.rpc.send_command("reboot", true, None).await;
        Ok(data.is_ok())
    }
    fn supports_restart(&self) -> bool {
        true
    }
}

#[async_trait]
impl Pause for WhatsMinerV2 {
    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        let data = self
            .rpc
            .send_command("power_off", true, Some(json!({"respbefore": "true"}))) // Has to be string for some reason
            .await;
        Ok(data.is_ok())
    }
    fn supports_pause(&self) -> bool {
        true
    }
}

#[async_trait]
impl Resume for WhatsMinerV2 {
    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        let data = self.rpc.send_command("power_on", true, None).await;
        Ok(data.is_ok())
    }
    fn supports_resume(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsScalingConfig for WhatsMinerV2 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsTuningConfig for WhatsMinerV2 {
    fn supports_tuning_config(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use asic_rs_core::test::api::MockAPIClient;
    use asic_rs_makes_whatsminer::models::WhatsMinerModel;

    use super::*;
    use crate::test::json::v2::{
        DEVS_COMMAND, GET_ERROR_CODE_COMMAND, GET_MINER_INFO_COMMAND, GET_PSU_COMMAND,
        GET_VERSION_COMMAND, POOLS_COMMAND, STATUS_COMMAND, SUMMARY_COMMAND,
    };

    #[tokio::test]
    async fn test_whatsminer_v2_data_parsers() -> anyhow::Result<()> {
        // Arrange
        let miner = WhatsMinerV2::new(IpAddr::from([127, 0, 0, 1]), WhatsMinerModel::M50SVH50);
        let mut results = HashMap::new();

        results.insert(
            MinerCommand::RPC {
                command: "summary",
                parameters: None,
            },
            Value::from_str(SUMMARY_COMMAND)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "status",
                parameters: None,
            },
            Value::from_str(STATUS_COMMAND)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "pools",
                parameters: None,
            },
            Value::from_str(POOLS_COMMAND)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "devs",
                parameters: None,
            },
            Value::from_str(DEVS_COMMAND)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "get_version",
                parameters: None,
            },
            Value::from_str(GET_VERSION_COMMAND)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "get_psu",
                parameters: None,
            },
            Value::from_str(GET_PSU_COMMAND)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "get_miner_info",
                parameters: None,
            },
            Value::from_str(GET_MINER_INFO_COMMAND)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "get_error_code",
                parameters: None,
            },
            Value::from_str(GET_ERROR_CODE_COMMAND)?,
        );

        let mock_api = MockAPIClient::new(results);

        // Act
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;
        let miner_data = miner.parse_data(data);

        // Assert
        assert_eq!(&miner_data.ip, &miner.ip);
        assert_eq!(
            miner_data.mac,
            Some(MacAddr::from_str("D4:01:02:03:04:05")?)
        );
        assert_eq!(miner_data.api_version, Some("2.0.5".to_string()));
        assert_eq!(
            miner_data.firmware_version,
            Some("20230803.11.REL".to_string())
        );
        assert_eq!(miner_data.hostname, Some("WhatsMiner".to_string()));
        assert_eq!(
            miner_data.hashrate,
            Some(HashRate {
                value: 124.5,
                unit: HashRateUnit::TeraHash,
                algo: String::from("SHA256"),
            })
        );
        assert_eq!(
            miner_data.expected_hashrate,
            Some(HashRate {
                value: 126.0,
                unit: HashRateUnit::TeraHash,
                algo: String::from("SHA256"),
            })
        );
        assert_eq!(miner_data.wattage, Some(Power::from_watts(3200.0)));
        assert_eq!(
            miner_data.tuning_target,
            Some(TuningTarget::Power(Power::from_watts(3300.0)))
        );
        assert_eq!(miner_data.uptime, Some(Duration::from_secs(25000)));
        assert!(miner_data.is_mining);
        assert_eq!(miner_data.fans.len(), 2);
        assert_eq!(miner_data.pools[0].len(), 3);

        Ok(())
    }

    #[test]
    fn test_parse_is_mining_when_miner_off() {
        // Arrange - mineroff="true" means the miner is off
        let miner = WhatsMinerV2::new(IpAddr::from([127, 0, 0, 1]), WhatsMinerModel::M30SV10);
        let mut data = HashMap::new();
        data.insert(DataField::IsMining, Value::String("true".to_string()));

        // Act
        let is_mining = miner.parse_is_mining(&data);

        // Assert
        assert!(!is_mining);
    }

    #[test]
    fn test_parse_is_mining_when_miner_on() {
        // Arrange - mineroff="false" means the miner is running
        let miner = WhatsMinerV2::new(IpAddr::from([127, 0, 0, 1]), WhatsMinerModel::M30SV10);
        let mut data = HashMap::new();
        data.insert(DataField::IsMining, Value::String("false".to_string()));

        // Act
        let is_mining = miner.parse_is_mining(&data);

        // Assert
        assert!(is_mining);
    }

    #[test]
    fn test_parse_is_mining_missing_defaults_to_mining() {
        // Arrange - no status data available
        let miner = WhatsMinerV2::new(IpAddr::from([127, 0, 0, 1]), WhatsMinerModel::M30SV10);
        let data = HashMap::new();

        // Act
        let is_mining = miner.parse_is_mining(&data);

        // Assert
        assert!(is_mining);
    }
}

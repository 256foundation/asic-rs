use anyhow;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature};
use serde_json::Value;
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

use crate::data::board::BoardData;
use crate::data::device::{DeviceInfo, HashAlgorithm, MinerFirmware, MinerModel};
use crate::data::device::{MinerControlBoard, MinerMake};
use crate::data::fan::FanData;
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::message::{MessageSeverity, MinerMessage};
use crate::data::pool::{PoolData, PoolURL};
use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;
use crate::miners::data::{
    DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
};

use rpc::WhatsMinerRPCAPI;

mod rpc;

#[derive(Debug)]
pub struct WhatsMinerV1 {
    pub ip: IpAddr,
    pub rpc: WhatsMinerRPCAPI,
    pub device_info: DeviceInfo,
}

impl WhatsMinerV1 {
    pub fn new(ip: IpAddr, model: MinerModel) -> Self {
        WhatsMinerV1 {
            ip,
            rpc: WhatsMinerRPCAPI::new(ip, None),
            device_info: DeviceInfo::new(
                MinerMake::WhatsMiner,
                model,
                MinerFirmware::Stock,
                HashAlgorithm::SHA256,
            ),
        }
    }
}

#[async_trait]
impl APIClient for WhatsMinerV1 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for WhatsMiner API"
            )),
        }
    }
}

impl GetDataLocations for WhatsMinerV1 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
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

        match data_field {
            DataField::Mac => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/MAC"),
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
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/CB Platform"),
                    tag: None,
                },
            )],
            DataField::WattageLimit => vec![(
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
            DataField::IsMining => vec![],
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
            DataField::MiningMode => vec![(
                RPC_STATUS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/btmineroff"),
                    tag: None,
                },
            )],
            DataField::Messages => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for WhatsMinerV1 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}
impl GetDeviceInfo for WhatsMinerV1 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info
    }
}

impl CollectData for WhatsMinerV1 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for WhatsMinerV1 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetSerialNumber for WhatsMinerV1 {}
impl GetHostname for WhatsMinerV1 {}
impl GetApiVersion for WhatsMinerV1 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
            .and_then(|s| Some(s.strip_prefix("whatsminer v")?.to_string()))
    }
}
impl GetFirmwareVersion for WhatsMinerV1 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}
impl GetControlBoardVersion for WhatsMinerV1 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| {
                MinerControlBoard::from_str(s.to_uppercase().strip_prefix("ALLWINNER_")?).ok()
            })
    }
}
impl GetHashboards for WhatsMinerV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> = Vec::new();
        let board_count = self.device_info.hardware.boards.unwrap_or(3);
        let hashboard_data = data.get(&DataField::Hashboards);

        for idx in 0..board_count {
            let hashrate = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{}/MHS av", idx)))
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
                .and_then(|val| val.pointer(&format!("/DEVS/{}/Factory GHS", idx)))
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
                .and_then(|val| val.pointer(&format!("/DEVS/{}/Temperature", idx)))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let intake_temperature = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{}/Chip Temp Min", idx)))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let outlet_temperature = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{}/Chip Temp Max", idx)))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let serial_number = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{}/PCB SN", idx)))
                .and_then(|val| val.as_str())
                .map(String::from);
            let working_chips = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{}/Effective Chips", idx)))
                .and_then(|val| val.as_u64())
                .map(|u| u as u16);
            let frequency = hashboard_data
                .and_then(|val| val.pointer(&format!("/DEVS/{}/Frequency", idx)))
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
impl GetHashrate for WhatsMinerV1 {
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

impl GetIsMining for WhatsMinerV1 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        self.parse_hashrate(data)
            .map(|hr| hr.value > 0.0)
            .unwrap_or(false)
    }
}

impl GetExpectedHashrate for WhatsMinerV1 {
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
impl GetFans for WhatsMinerV1 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans: Vec<FanData> = Vec::new();
        for (idx, direction) in ["In", "Out"].iter().enumerate() {
            let fan = data.extract_nested_map::<f64, _>(
                DataField::Fans,
                &format!("Fan Speed {}", direction),
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
impl GetPsuFans for WhatsMinerV1 {
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
impl GetFluidTemperature for WhatsMinerV1 {
    fn parse_fluid_temperature(&self, data: &HashMap<DataField, Value>) -> Option<Temperature> {
        data.extract_map::<f64, _>(DataField::FluidTemperature, Temperature::from_celsius)
    }
}
impl GetWattage for WhatsMinerV1 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}
impl GetWattageLimit for WhatsMinerV1 {
    fn parse_wattage_limit(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::WattageLimit, Power::from_watts)
    }
}
impl GetLightFlashing for WhatsMinerV1 {}
impl GetMessages for WhatsMinerV1 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let mut messages = Vec::new();

        let error_count = data
            .get(&DataField::Messages)
            .and_then(|val| {
                val.pointer("/Error Code Count")
                    .and_then(|val| val.as_u64())
            })
            .unwrap_or(0u64) as usize;
        for idx in 0..error_count {
            let e_code = data
                .get(&DataField::Messages)
                .and_then(|val| val.pointer(&format!("/Error Code {}", idx)))
                .and_then(|val| val.as_u64());
            if let Some(code) = e_code {
                messages.push(MinerMessage::new(
                    0,
                    code,
                    "".to_string(), // TODO: parse message from mapping
                    MessageSeverity::Error,
                ));
            }
        }

        messages
    }
}
impl GetUptime for WhatsMinerV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}
impl GetMiningMode for WhatsMinerV1 {
    fn parse_mining_mode(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> crate::data::miner::MiningMode {
        let enabled = data
            .extract_map::<String, _>(DataField::MiningMode, |l| l != "false")
            .unwrap_or(true);
        enabled.into()
    }
}
impl GetPools for WhatsMinerV1 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolData> {
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
                    .and_then(|val| val.pointer(&format!("/{}/User", idx)))
                    .map(|val| String::from(val.as_str().unwrap_or("")));

                let alive = pools_raw
                    .and_then(|val| val.pointer(&format!("/{}/Status", idx)))
                    .map(|val| val.as_str())
                    .map(|val| val == Some("Alive"));

                let active = pools_raw
                    .and_then(|val| val.pointer(&format!("/{}/Stratum Active", idx)))
                    .and_then(|val| val.as_bool());

                let url = pools_raw
                    .and_then(|val| val.pointer(&format!("/{}/URL", idx)))
                    .map(|val| PoolURL::from(String::from(val.as_str().unwrap_or(""))));

                let accepted_shares = pools_raw
                    .and_then(|val| val.pointer(&format!("/{}/Accepted", idx)))
                    .and_then(|val| val.as_u64());

                let rejected_shares = pools_raw
                    .and_then(|val| val.pointer(&format!("/{}/Rejected", idx)))
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
        pools
    }
}

#[async_trait]
impl SetFaultLight for WhatsMinerV1 {
    #[allow(unused_variables)]
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

#[async_trait]
impl SetPowerLimit for WhatsMinerV1 {
    #[allow(unused_variables)]
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

#[async_trait]
impl Restart for WhatsMinerV1 {
    async fn restart(&self) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

#[async_trait]
impl Pause for WhatsMinerV1 {
    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

#[async_trait]
impl Resume for WhatsMinerV1 {
    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::device::models::whatsminer::WhatsMinerModel;
    use crate::test::api::MockAPIClient;
    use crate::test::json::btminer::v1::{
        DEVS_COMMAND, GET_PSU_COMMAND, GET_VERSION_COMMAND, POOLS_COMMAND, STATUS_COMMAND,
        SUMMARY_COMMAND,
    };

    #[tokio::test]
    async fn test_whatsminer_v1_data_parsers() -> anyhow::Result<()> {
        let miner = WhatsMinerV1::new(
            IpAddr::from([127, 0, 0, 1]),
            MinerModel::WhatsMiner(WhatsMinerModel::M20SV10),
        );
        let mut results = HashMap::new();
        let summary_command: MinerCommand = MinerCommand::RPC {
            command: "summary",
            parameters: None,
        };
        let status_command: MinerCommand = MinerCommand::RPC {
            command: "status",
            parameters: None,
        };
        let pools_command: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };
        let devs_command: MinerCommand = MinerCommand::RPC {
            command: "devs",
            parameters: None,
        };
        let get_version_command: MinerCommand = MinerCommand::RPC {
            command: "get_version",
            parameters: None,
        };
        let get_psu_command: MinerCommand = MinerCommand::RPC {
            command: "get_psu",
            parameters: None,
        };

        results.insert(summary_command, Value::from_str(SUMMARY_COMMAND)?);
        results.insert(status_command, Value::from_str(STATUS_COMMAND)?);
        results.insert(pools_command, Value::from_str(POOLS_COMMAND)?);
        results.insert(devs_command, Value::from_str(DEVS_COMMAND)?);
        results.insert(get_version_command, Value::from_str(GET_VERSION_COMMAND)?);
        results.insert(get_psu_command, Value::from_str(GET_PSU_COMMAND)?);

        let mock_api = MockAPIClient::new(results);

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;

        let miner_data = miner.parse_data(data);

        assert_eq!(&miner_data.ip, &miner.ip);
        assert_eq!(
            miner_data.mac,
            Some(MacAddr::from_str("C4:08:28:00:A4:19")?)
        );
        assert_eq!(miner_data.api_version, Some("1.4.0".to_string()));
        assert_eq!(
            miner_data.firmware_version,
            Some("20210322.22.REL".to_string())
        );
        assert_eq!(
            miner_data.control_board_version,
            Some(MinerControlBoard::H3)
        );
        assert_eq!(
            miner_data.hashrate,
            Some(HashRate {
                value: 67.39480097,
                unit: HashRateUnit::TeraHash,
                algo: String::from("SHA256"),
            })
        );
        assert_eq!(
            miner_data.expected_hashrate,
            Some(HashRate {
                value: 68.796,
                unit: HashRateUnit::TeraHash,
                algo: String::from("SHA256"),
            })
        );
        assert_eq!(miner_data.wattage, Some(Power::from_watts(3417f64)));
        assert_eq!(miner_data.wattage_limit, Some(Power::from_watts(3500f64)));
        assert_eq!(miner_data.uptime, Some(Duration::from_secs(10154)));
        assert_eq!(miner_data.fans.len(), 2);
        assert_eq!(miner_data.pools.len(), 3);

        Ok(())
    }
}

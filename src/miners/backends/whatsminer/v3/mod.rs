use anyhow;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

use crate::data::board::BoardData;
use crate::data::device::{DeviceInfo, HashAlgorithm, MinerFirmware, MinerModel};
use crate::data::device::{MinerControlBoard, MinerMake};
use crate::data::fan::FanData;
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::pool::{PoolData, PoolGroupData, PoolURL};
use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;
use crate::miners::data::{
    DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_key,
    get_by_pointer,
};

pub(crate) use rpc::WhatsMinerRPCAPI;

mod rpc;

#[derive(Debug)]
pub struct WhatsMinerV3 {
    pub ip: IpAddr,
    pub rpc: WhatsMinerRPCAPI,
    pub device_info: DeviceInfo,
}

impl WhatsMinerV3 {
    pub fn new(ip: IpAddr, model: MinerModel) -> Self {
        WhatsMinerV3 {
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
impl APIClient for WhatsMinerV3 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for WhatsMiner API"
            )),
        }
    }
}

impl GetDataLocations for WhatsMinerV3 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const RPC_GET_DEVICE_INFO: MinerCommand = MinerCommand::RPC {
            command: "get.device.info",
            parameters: None,
        };
        let rpc_get_miner_status_summary: MinerCommand = MinerCommand::RPC {
            command: "get.miner.status",
            parameters: Some(json!("summary")),
        };
        let rpc_get_miner_status_pools: MinerCommand = MinerCommand::RPC {
            command: "get.miner.status",
            parameters: Some(json!("pools")),
        };
        let rpc_get_miner_status_edevs: MinerCommand = MinerCommand::RPC {
            command: "get.miner.status",
            parameters: Some(json!("edevs")),
        };

        match data_field {
            DataField::Mac => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/network/mac"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/system/api"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/system/fwversion"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/system/platform"),
                    tag: None,
                },
            )],
            DataField::SerialNumber => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/miner/miner-sn"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/network/hostname"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/system/ledstatus"),
                    tag: None,
                },
            )],
            DataField::WattageLimit => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary/power-limit"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary"),
                    tag: None,
                },
            )],
            DataField::PsuFans => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/power/fanspeed"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![
                (
                    RPC_GET_DEVICE_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/msg/miner"),
                        tag: None,
                    },
                ),
                (
                    rpc_get_miner_status_edevs,
                    DataExtractor {
                        func: get_by_key,
                        key: Some("msg"),
                        tag: None,
                    },
                ),
            ],
            DataField::Pools => vec![(
                rpc_get_miner_status_pools,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/pools"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary/elapsed"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary/power-realtime"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary/hash-realtime"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary/factory-hash"),
                    tag: None,
                },
            )],
            DataField::FluidTemperature => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary/environment-temperature"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for WhatsMinerV3 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}
impl GetDeviceInfo for WhatsMinerV3 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info
    }
}

impl CollectData for WhatsMinerV3 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for WhatsMinerV3 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetSerialNumber for WhatsMinerV3 {}
impl GetHostname for WhatsMinerV3 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}
impl GetApiVersion for WhatsMinerV3 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}
impl GetFirmwareVersion for WhatsMinerV3 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}
impl GetControlBoardVersion for WhatsMinerV3 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| MinerControlBoard::from_str(&s).ok())
    }
}
impl GetHashboards for WhatsMinerV3 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> = Vec::new();
        let board_count = self.device_info.hardware.boards.unwrap_or(3);
        for idx in 0..board_count {
            let hashrate = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/hash-average")))
                .and_then(|val| val.as_f64())
                .map(|f| HashRate {
                    value: f,
                    unit: HashRateUnit::TeraHash,
                    algo: String::from("SHA256"),
                });
            let expected_hashrate = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/factory-hash")))
                .and_then(|val| val.as_f64())
                .map(|f| HashRate {
                    value: f,
                    unit: HashRateUnit::TeraHash,
                    algo: String::from("SHA256"),
                });
            let board_temperature = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/chip-temp-min")))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let intake_temperature = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/chip-temp-min")))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let outlet_temperature = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/chip-temp-max")))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let serial_number =
                data.extract_nested::<String>(DataField::Hashboards, &format!("pcbsn{idx}"));

            let working_chips = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/effective-chips")))
                .and_then(|val| val.as_u64())
                .map(|u| u as u16);
            let frequency = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/freq")))
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
impl GetHashrate for WhatsMinerV3 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::TeraHash,
            algo: String::from("SHA256"),
        })
    }
}
impl GetExpectedHashrate for WhatsMinerV3 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::TeraHash,
            algo: String::from("SHA256"),
        })
    }
}
impl GetFans for WhatsMinerV3 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans: Vec<FanData> = Vec::new();
        for (idx, direction) in ["in", "out"].iter().enumerate() {
            let fan = data.extract_nested_map::<f64, _>(
                DataField::Fans,
                &format!("fan-speed-{direction}"),
                |rpm| FanData {
                    position: idx as i16,
                    rpm: Some(AngularVelocity::from_rpm(rpm)),
                },
            );
            if let Some(fan_data) = fan {
                fans.push(fan_data);
            }
        }
        fans
    }
}
impl GetPsuFans for WhatsMinerV3 {
    fn parse_psu_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut psu_fans: Vec<FanData> = Vec::new();

        let psu_fan = data.extract_map::<f64, _>(DataField::PsuFans, |rpm| FanData {
            position: 0i16,
            rpm: Some(AngularVelocity::from_rpm(rpm)),
        });
        if let Some(fan_data) = psu_fan {
            psu_fans.push(fan_data);
        }
        psu_fans
    }
}
impl GetFluidTemperature for WhatsMinerV3 {
    fn parse_fluid_temperature(&self, data: &HashMap<DataField, Value>) -> Option<Temperature> {
        data.extract_map::<f64, _>(DataField::FluidTemperature, Temperature::from_celsius)
    }
}
impl GetWattage for WhatsMinerV3 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}
impl GetWattageLimit for WhatsMinerV3 {
    fn parse_wattage_limit(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::WattageLimit, Power::from_watts)
    }
}
impl GetLightFlashing for WhatsMinerV3 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract_map::<String, _>(DataField::LightFlashing, |l| l != "auto")
    }
}
impl GetMessages for WhatsMinerV3 {}
impl GetUptime for WhatsMinerV3 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}
impl GetIsMining for WhatsMinerV3 {}
impl GetPools for WhatsMinerV3 {
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
                let user = data
                    .get(&DataField::Pools)
                    .and_then(|val| val.pointer(&format!("/{idx}/account")))
                    .map(|val| String::from(val.as_str().unwrap_or("")));

                let alive = data
                    .get(&DataField::Pools)
                    .and_then(|val| val.pointer(&format!("/{idx}/status")))
                    .map(|val| val.as_str())
                    .map(|val| val == Some("alive"));

                let active = data
                    .get(&DataField::Pools)
                    .and_then(|val| val.pointer(&format!("/{idx}/stratum-active")))
                    .and_then(|val| val.as_bool());

                let url = data
                    .get(&DataField::Pools)
                    .and_then(|val| val.pointer(&format!("/{idx}/url")))
                    .map(|val| PoolURL::from(String::from(val.as_str().unwrap_or(""))));

                pools.push(PoolData {
                    position: Some(idx as u16),
                    url,
                    accepted_shares: None,
                    rejected_shares: None,
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
impl SetFaultLight for WhatsMinerV3 {
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        let parameters = match fault {
            false => Some(json!("auto")),
            true => Some(json!([{"color": "red", "period": 200, "duration": 100, "start": 0}])),
        };

        let data = self
            .rpc
            .send_command("set.system.led", true, parameters)
            .await;

        Ok(data.is_ok())
    }
    fn supports_set_fault_light(&self) -> bool {
        true
    }
}

#[async_trait]
impl SetPowerLimit for WhatsMinerV3 {
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        let data = self
            .rpc
            .send_command("set.miner.power_limit", true, Some(json!(limit.as_watts())))
            .await;

        Ok(data.is_ok())
    }
    fn supports_set_power_limit(&self) -> bool {
        true
    }
}

#[async_trait]
impl SetPools for WhatsMinerV3 {
    fn supports_set_pools(&self) -> bool {
        false
    }
}

#[async_trait]
impl Restart for WhatsMinerV3 {
    async fn restart(&self) -> anyhow::Result<bool> {
        let data = self.rpc.send_command("set.system.reboot", true, None).await;

        Ok(data.is_ok())
    }
    fn supports_restart(&self) -> bool {
        true
    }
}

#[async_trait]
impl Pause for WhatsMinerV3 {
    async fn pause(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        // might not work as intended, if issues are found then switch to "enable" + "disable"
        // see api docs - https://apidoc.whatsminer.com/#api-Miner-btminer_service_set
        let data = self
            .rpc
            .send_command("set.miner.service", true, Some(json!("stop")))
            .await;

        Ok(data.is_ok())
    }
    fn supports_pause(&self) -> bool {
        true
    }
}

#[async_trait]
impl Resume for WhatsMinerV3 {
    async fn resume(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        let data = self
            .rpc
            .send_command("set.miner.service", true, Some(json!("start")))
            .await;

        Ok(data.is_ok())
    }
    fn supports_resume(&self) -> bool {
        true
    }
}

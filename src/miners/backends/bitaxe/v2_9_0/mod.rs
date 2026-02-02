use anyhow;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use serde_json::Value;
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::data::board::{BoardData, ChipData};
use crate::data::device::{DeviceInfo, HashAlgorithm, MinerFirmware, MinerModel};
use crate::data::device::{MinerControlBoard, MinerMake};
use crate::data::fan::FanData;
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::message::{MessageSeverity, MinerMessage};
use crate::data::pool::{PoolData, PoolScheme, PoolURL};
use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;
use crate::miners::data::{
    DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_key,
    get_by_pointer,
};
use web::BitaxeWebAPI;

mod web;

#[derive(Debug)]
pub struct Bitaxe290 {
    ip: IpAddr,
    web: BitaxeWebAPI,
    device_info: DeviceInfo,
}

impl Bitaxe290 {
    pub fn new(ip: IpAddr, model: MinerModel) -> Self {
        Bitaxe290 {
            ip,
            web: BitaxeWebAPI::new(ip, 80),
            device_info: DeviceInfo::new(
                MinerMake::Bitaxe,
                model,
                MinerFirmware::Stock,
                HashAlgorithm::SHA256,
            ),
        }
    }
}

#[async_trait]
impl APIClient for Bitaxe290 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!("Unsupported command type for Bitaxe API")),
        }
    }
}

#[async_trait]
impl GetDataLocations for Bitaxe290 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const WEB_SYSTEM_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "system/info",
            parameters: None,
        };
        const WEB_ASIC_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "system/asic",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("macAddr"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("hostname"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("version"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("version"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("boardVersion"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("expectedHashrate"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![
                (
                    WEB_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: None,
                    },
                ),
                (
                    WEB_ASIC_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: None,
                    },
                ),
            ],
            DataField::Hashrate => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("hashRate"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("fanrpm"),
                    tag: None,
                },
            )],
            DataField::AverageTemperature => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("temp"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("power"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("uptimeSeconds"),
                    tag: None,
                },
            )],
            DataField::Pools => vec![(
                WEB_SYSTEM_INFO,
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

impl GetIP for Bitaxe290 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}
impl GetDeviceInfo for Bitaxe290 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info
    }
}

impl CollectData for Bitaxe290 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for Bitaxe290 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetSerialNumber for Bitaxe290 {
    // N/A
}
impl GetHostname for Bitaxe290 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}
impl GetApiVersion for Bitaxe290 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}
impl GetFirmwareVersion for Bitaxe290 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}
impl GetControlBoardVersion for Bitaxe290 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| MinerControlBoard::from_str(&s).ok())
    }
}
impl GetHashboards for Bitaxe290 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        // Extract nested values with type conversion
        let board_voltage = data.extract_nested_map::<f64, _>(
            DataField::Hashboards,
            "voltage",
            Voltage::from_millivolts,
        );

        let board_temperature = data.extract_nested_map::<f64, _>(
            DataField::Hashboards,
            "vrTemp",
            Temperature::from_celsius,
        );

        let board_frequency = data.extract_nested_map::<f64, _>(
            DataField::Hashboards,
            "frequency",
            Frequency::from_megahertz,
        );

        let chip_temperature = data.extract_nested_map::<f64, _>(
            DataField::Hashboards,
            "temp",
            Temperature::from_celsius,
        );

        let expected_hashrate = Some(HashRate {
            value: data.extract_nested_or::<f64>(DataField::Hashboards, "expectedHashrate", 0.0),
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(),
        });

        let board_hashrate = Some(HashRate {
            value: data.extract_nested_or::<f64>(DataField::Hashboards, "hashRate", 0.0),
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(),
        });

        let total_chips =
            data.extract_nested_map::<u64, _>(DataField::Hashboards, "asicCount", |u| u as u16);

        let chip_info = ChipData {
            position: 0,
            temperature: chip_temperature,
            voltage: board_voltage,
            frequency: board_frequency,
            tuned: Some(true),
            working: Some(true),
            hashrate: board_hashrate.clone(),
        };

        let board_data = BoardData {
            position: 0,
            hashrate: board_hashrate,
            expected_hashrate,
            board_temperature,
            intake_temperature: board_temperature,
            outlet_temperature: board_temperature,
            expected_chips: self.device_info.hardware.chips,
            working_chips: total_chips,
            serial_number: None,
            chips: vec![chip_info],
            voltage: board_voltage,
            frequency: board_frequency,
            tuned: Some(true),
            active: Some(true),
        };

        vec![board_data]
    }
}
impl GetHashrate for Bitaxe290 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: String::from("SHA256"),
        })
    }
}

impl GetExpectedHashrate for Bitaxe290 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: String::from("SHA256"),
        })
    }
}
impl GetFans for Bitaxe290 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        data.extract_map_or::<f64, _>(DataField::Fans, Vec::new(), |f| {
            vec![FanData {
                position: 0,
                rpm: Some(AngularVelocity::from_rpm(f)),
            }]
        })
    }
}
impl GetPsuFans for Bitaxe290 {
    // N/A
}
impl GetFluidTemperature for Bitaxe290 {
    // N/A
}
impl GetWattage for Bitaxe290 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}
impl GetWattageLimit for Bitaxe290 {
    // N/A
}
impl GetLightFlashing for Bitaxe290 {
    // N/A
}
impl GetMessages for Bitaxe290 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let mut messages = Vec::new();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_secs();

        let is_overheating = data.extract_nested::<bool>(DataField::Hashboards, "overheat_mode");

        if let Some(true) = is_overheating {
            messages.push(MinerMessage {
                timestamp: timestamp as u32,
                code: 0u64,
                message: "Overheat Mode is Enabled!".to_string(),
                severity: MessageSeverity::Warning,
            });
        };
        messages
    }
}
impl GetUptime for Bitaxe290 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}
impl GetIsMining for Bitaxe290 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        let hashrate = self.parse_hashrate(data);
        hashrate.as_ref().is_some_and(|hr| hr.value > 0.0)
    }
}
impl GetPools for Bitaxe290 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolData> {
        let main_url =
            data.extract_nested_or::<String>(DataField::Pools, "stratumURL", String::new());
        let main_port = data.extract_nested_or::<u64>(DataField::Pools, "stratumPort", 0);
        let accepted_share = data.extract_nested::<u64>(DataField::Pools, "sharesAccepted");
        let rejected_share = data.extract_nested::<u64>(DataField::Pools, "sharesRejected");
        let main_user = data.extract_nested::<String>(DataField::Pools, "stratumUser");

        let is_using_fallback =
            data.extract_nested_or::<bool>(DataField::Pools, "isUsingFallbackStratum", false);

        let main_pool_url = PoolURL {
            scheme: PoolScheme::StratumV1,
            host: main_url,
            port: main_port as u16,
            pubkey: None,
        };

        let main_pool_data = PoolData {
            position: Some(0),
            url: Some(main_pool_url),
            accepted_shares: accepted_share,
            rejected_shares: rejected_share,
            active: Some(!is_using_fallback),
            alive: None,
            user: main_user,
        };

        // Extract fallback pool data
        let fallback_url =
            data.extract_nested_or::<String>(DataField::Pools, "fallbackStratumURL", String::new());
        let fallback_port =
            data.extract_nested_or::<u64>(DataField::Pools, "fallbackStratumPort", 0);
        let fallback_user = data.extract_nested(DataField::Pools, "fallbackStratumUser");
        let fallback_pool_url = PoolURL {
            scheme: PoolScheme::StratumV1,
            host: fallback_url,
            port: fallback_port as u16,
            pubkey: None,
        };

        let fallback_pool_data = PoolData {
            position: Some(1),
            url: Some(fallback_pool_url),
            accepted_shares: accepted_share,
            rejected_shares: rejected_share,
            active: Some(is_using_fallback),
            alive: None,
            user: fallback_user,
        };

        vec![main_pool_data, fallback_pool_data]
    }
}

#[async_trait]
impl SetFaultLight for Bitaxe290 {
    fn supports_set_fault_light(&self) -> bool {
        false
    }
}

#[async_trait]
impl SetPowerLimit for Bitaxe290 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl Restart for Bitaxe290 {
    fn supports_restart(&self) -> bool {
        false
    }
}

#[async_trait]
impl Pause for Bitaxe290 {
    fn supports_pause(&self) -> bool {
        false
    }
}

#[async_trait]
impl Resume for Bitaxe290 {
    fn supports_resume(&self) -> bool {
        false
    }
}

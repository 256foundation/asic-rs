use std::{
    collections::HashMap,
    net::IpAddr,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow;
use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigField, ConfigLocation},
        pools::PoolGroupConfig,
    },
    data::{
        board::{BoardData, ChipData, MinerControlBoard},
        collector::{
            DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_key,
            get_by_pointer,
        },
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        hashrate::{HashRate, HashRateUnit},
        message::{MessageSeverity, MinerMessage},
        pool::{PoolData, PoolGroupData, PoolScheme, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_nerdaxe::hardware::NerdAxeControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use serde_json::Value;

use crate::{backends::v1::web::NerdAxeWebAPI, firmware::NerdAxeFirmware};

pub(crate) mod web;

#[derive(Debug)]
pub struct NerdAxeV1 {
    ip: IpAddr,
    web: NerdAxeWebAPI,
    device_info: DeviceInfo,
}

impl NerdAxeV1 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        NerdAxeV1 {
            ip,
            web: NerdAxeWebAPI::new(ip, 80),
            device_info: DeviceInfo::new(model, NerdAxeFirmware::default(), HashAlgorithm::SHA256),
        }
    }
}

#[async_trait]
impl APIClient for NerdAxeV1 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!("Unsupported command type for NerdAxe API")),
        }
    }
}

impl GetConfigsLocations for NerdAxeV1 {
    #[allow(unused_variables)]
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}

impl CollectConfigs for NerdAxeV1 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

#[async_trait]
impl GetDataLocations for NerdAxeV1 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const WEB_SYSTEM_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "system/info",
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
            DataField::Hashboards => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("hashRate"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
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

impl GetIP for NerdAxeV1 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}
impl GetDeviceInfo for NerdAxeV1 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for NerdAxeV1 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for NerdAxeV1 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetSerialNumber for NerdAxeV1 {}
impl GetHostname for NerdAxeV1 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}
impl GetApiVersion for NerdAxeV1 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}
impl GetFirmwareVersion for NerdAxeV1 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}
impl GetControlBoardVersion for NerdAxeV1 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| NerdAxeControlBoard::parse(&s).map(|cb| cb.into()))
    }
}
impl GetHashboards for NerdAxeV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let board_voltage = data.extract_nested_map::<f64, _>(
            DataField::Hashboards,
            "coreVoltageActual",
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

        let board_hashrate = Some(HashRate {
            value: data.extract_nested_or::<f64>(DataField::Hashboards, "hashRate", 0.0),
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(),
        });

        let total_chips =
            data.extract_nested_map::<u64, _>(DataField::Hashboards, "asicCount", |u| u as u16);

        let core_count =
            data.extract_nested_or::<u64>(DataField::Hashboards, "smallCoreCount", 0u64);

        let expected_hashrate = Some(HashRate {
            value: core_count as f64
                * total_chips.unwrap_or(0) as f64
                * board_frequency
                    .unwrap_or(Frequency::from_megahertz(0f64))
                    .as_gigahertz(),
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(),
        });

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
impl GetHashrate for NerdAxeV1 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(),
        })
    }
}
impl GetExpectedHashrate for NerdAxeV1 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        let total_chips =
            data.extract_nested_map::<u64, _>(DataField::ExpectedHashrate, "asicCount", |u| {
                u as u16
            });

        let core_count =
            data.extract_nested_or::<u64>(DataField::ExpectedHashrate, "smallCoreCount", 0u64);

        let board_frequency = data.extract_nested_map::<f64, _>(
            DataField::Hashboards,
            "frequency",
            Frequency::from_megahertz,
        );

        Some(HashRate {
            value: core_count as f64
                * total_chips.unwrap_or(0) as f64
                * board_frequency
                    .unwrap_or(Frequency::from_megahertz(0f64))
                    .as_gigahertz(),
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(),
        })
    }
}
impl GetFans for NerdAxeV1 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        data.extract_map_or::<f64, _>(DataField::Fans, Vec::new(), |f| {
            vec![FanData {
                position: 0,
                rpm: Some(AngularVelocity::from_rpm(f)),
            }]
        })
    }
}
impl GetPsuFans for NerdAxeV1 {}
impl GetFluidTemperature for NerdAxeV1 {}
impl GetWattage for NerdAxeV1 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}
impl GetTuningTarget for NerdAxeV1 {}
impl GetLightFlashing for NerdAxeV1 {}
impl GetMessages for NerdAxeV1 {
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

impl GetUptime for NerdAxeV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}
impl GetIsMining for NerdAxeV1 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        let hashrate = self.parse_hashrate(data);
        hashrate.as_ref().is_some_and(|hr| hr.value > 0.0)
    }
}
impl GetPools for NerdAxeV1 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
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
            accepted_shares: None,
            rejected_shares: None,
            active: Some(is_using_fallback),
            alive: None,
            user: fallback_user,
        };

        vec![PoolGroupData {
            name: String::new(),
            quota: 1,
            pools: vec![main_pool_data, fallback_pool_data],
        }]
    }
}

#[async_trait]
impl SetFaultLight for NerdAxeV1 {
    fn supports_set_fault_light(&self) -> bool {
        false
    }
}

#[async_trait]
impl SetPowerLimit for NerdAxeV1 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsPoolsConfig for NerdAxeV1 {
    async fn get_pools_config(&self) -> anyhow::Result<Vec<PoolGroupConfig>> {
        Ok(self
            .get_pools()
            .await
            .iter()
            .map(|g| g.clone().into())
            .collect())
    }

    fn supports_pools_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl Restart for NerdAxeV1 {
    fn supports_restart(&self) -> bool {
        false
    }
}

#[async_trait]
impl Pause for NerdAxeV1 {
    fn supports_pause(&self) -> bool {
        false
    }
}

#[async_trait]
impl Resume for NerdAxeV1 {
    fn supports_resume(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsScalingConfig for NerdAxeV1 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for NerdAxeV1 {
    fn supports_upgrade_firmware(&self) -> bool {
        false
    }
}

impl HasAuth for NerdAxeV1 {}
impl HasDefaultAuth for NerdAxeV1 {}

#[async_trait]
impl SupportsTuningConfig for NerdAxeV1 {
    fn supports_tuning_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsFanConfig for NerdAxeV1 {
    fn supports_fan_config(&self) -> bool {
        false
    }
}

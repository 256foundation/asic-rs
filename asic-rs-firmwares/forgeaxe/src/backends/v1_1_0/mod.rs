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
            DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_key,
            get_by_pointer,
        },
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        hashrate::{HashRate, HashRateUnit},
        message::{MessageSeverity, MinerMessage},
        operating_state::OperatingState,
        pool::{PoolData, PoolGroupData, PoolScheme, PoolURL},
        share::parse_share_difficulty,
    },
    traits::{miner::*, model::MinerModel},
    util::unix_timestamp_secs,
};
use asic_rs_makes_forgeaxe::hardware::ForgeaxeControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use semver::Version;
use serde_json::Value;
use web::ForgeaxeWebAPI;

use crate::firmware::ForgeaxeFirmware;

pub(crate) mod web;

/// ForgeOS v1.1 backend, initially copied from the ESP-Miner 2.0 implementation.
#[derive(Debug)]
pub struct Forgeaxe110 {
    ip: IpAddr,
    web: ForgeaxeWebAPI,
    device_info: DeviceInfo,
}

impl Forgeaxe110 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        Forgeaxe110 {
            ip,
            web: ForgeaxeWebAPI::new(ip, 80),
            device_info: DeviceInfo::new(model, ForgeaxeFirmware::default(), HashAlgorithm::SHA256),
        }
    }
}

#[async_trait]
impl APIClient for Forgeaxe110 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for {} API",
                ForgeaxeFirmware::default()
            )),
        }
    }
}

impl GetConfigsLocations for Forgeaxe110 {
    #[allow(unused_variables)]
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}

impl CollectConfigs for Forgeaxe110 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

#[async_trait]
impl GetDataLocations for Forgeaxe110 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const WEB_SYSTEM_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "system/info",
            parameters: None,
        };

        match data_field {
            DataField::OperatingState => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("miningPaused"),
                    tag: None,
                },
            )],
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
            DataField::Chips => vec![(
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
            DataField::BestShare => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("bestDiff"),
                    tag: None,
                },
            )],
            DataField::SessionBestShare => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("bestSessionDiff"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for Forgeaxe110 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}
impl GetDeviceInfo for Forgeaxe110 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for Forgeaxe110 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for Forgeaxe110 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetSerialNumber for Forgeaxe110 {
    // N/A
}
impl GetHostname for Forgeaxe110 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}
impl GetApiVersion for Forgeaxe110 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}
impl GetFirmwareVersion for Forgeaxe110 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}
impl GetControlBoardVersion for Forgeaxe110 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| ForgeaxeControlBoard::parse(&s).map(|cb| cb.into()))
    }
}
impl GetHashboards for Forgeaxe110 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut board = BoardData::new(0, self.device_info.hardware.chips_for_board(0));

        let Some(api_data) = data.get(&DataField::Hashboards) else {
            return vec![board];
        };
        let chip_data = data.get(&DataField::Chips);

        board.hashrate = api_data.get("hashRate").and_then(|v| v.as_f64()).map(|f| {
            HashRate {
                value: f,
                unit: HashRateUnit::GigaHash,
                algo: self.device_info.algo,
            }
            .as_default_unit()
        });
        board.expected_hashrate = api_data
            .get("smallCoreCount")
            .and_then(|v| v.as_u64())
            .zip(board.working_chips)
            .zip(board.frequency)
            .map(|((core_count, chips), freq)| {
                HashRate {
                    value: core_count as f64 * chips as f64 * freq.as_gigahertz(),
                    unit: HashRateUnit::GigaHash,
                    algo: self.device_info.algo,
                }
                .as_default_unit()
            });
        // `vrTemp` is the VR/board sensor; `temp` is the ASIC chip sensor.
        let chip_temp = api_data
            .get("temp")
            .and_then(|v| v.as_f64())
            .filter(|t| *t > -50.0)
            .or_else(|| {
                chip_data
                    .and_then(|c| c.get("temp"))
                    .and_then(|v| v.as_f64())
                    .filter(|t| *t > -50.0)
            })
            .map(Temperature::from_celsius);
        board.board_temperature = api_data
            .get("vrTemp")
            .and_then(|v| v.as_f64())
            .map(Temperature::from_celsius);
        board.inlet_chip_temperature = chip_temp;
        board.outlet_chip_temperature = chip_temp;
        board.working_chips = api_data
            .get("asicCount")
            .and_then(|v| v.as_u64())
            .map(|u| u as u16);
        board.voltage = api_data
            .get("voltage")
            .and_then(|v| v.as_f64())
            .map(Voltage::from_millivolts);
        board.frequency = api_data
            .get("frequency")
            .and_then(|v| v.as_f64())
            .map(Frequency::from_megahertz);
        if chip_data.is_some() {
            board.chips = vec![ChipData {
                position: 0,
                temperature: chip_temp,
                voltage: board.voltage,
                frequency: board.frequency,
                tuned: Some(true),
                working: Some(true),
                hashrate: board.hashrate.clone(),
            }];
        }
        board.active = Some(true);

        vec![board]
    }
}
impl GetHashrate for Forgeaxe110 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| {
            HashRate {
                value: f,
                unit: HashRateUnit::GigaHash,
                algo: self.device_info.algo,
            }
            .as_default_unit()
        })
    }
}
impl GetExpectedHashrate for Forgeaxe110 {
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

        Some(
            HashRate {
                value: core_count as f64
                    * total_chips.unwrap_or(0) as f64
                    * board_frequency
                        .unwrap_or(Frequency::from_megahertz(0f64))
                        .as_gigahertz(),
                unit: HashRateUnit::GigaHash,
                algo: self.device_info.algo,
            }
            .as_default_unit(),
        )
    }
}
impl GetFans for Forgeaxe110 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        data.extract_map_or::<f64, _>(DataField::Fans, Vec::new(), |f| {
            vec![FanData {
                position: 0,
                rpm: Some(AngularVelocity::from_rpm(f)),
            }]
        })
    }
}
impl GetPsuFans for Forgeaxe110 {
    // N/A
}
impl GetFluidTemperature for Forgeaxe110 {
    // N/A
}
impl GetWattage for Forgeaxe110 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}
impl GetTuningTarget for Forgeaxe110 {
    // N/A
}
impl GetScaledTuningTarget for Forgeaxe110 {
    // N/A
}
impl GetTuningCapabilities for Forgeaxe110 {}
impl GetLightFlashing for Forgeaxe110 {
    // N/A
}
impl GetMessages for Forgeaxe110 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let mut messages = Vec::new();
        let timestamp = unix_timestamp_secs();

        let is_overheating = data.extract_nested::<bool>(DataField::Hashboards, "overheat_mode");

        if let Some(true) = is_overheating {
            messages.push(MinerMessage {
                timestamp: timestamp as u32,
                code: 0u64,
                message: "Overheat Mode is Enabled!".to_string(),
                severity: MessageSeverity::Warning,
                component: None,
            });
        };
        messages
    }
}

impl GetUptime for Forgeaxe110 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetBestShare for Forgeaxe110 {
    fn parse_best_share(&self, data: &HashMap<DataField, Value>) -> Option<f64> {
        data.get(&DataField::BestShare)
            .and_then(parse_share_difficulty)
    }
}

impl GetSessionBestShare for Forgeaxe110 {
    fn parse_session_best_share(&self, data: &HashMap<DataField, Value>) -> Option<f64> {
        data.get(&DataField::SessionBestShare)
            .and_then(parse_share_difficulty)
    }
}
impl GetOperatingState for Forgeaxe110 {
    fn parse_operating_state(&self, data: &HashMap<DataField, Value>) -> Option<OperatingState> {
        data.get(&DataField::OperatingState)
            .and_then(Value::as_bool)
            .map(|paused| {
                if paused {
                    OperatingState::Paused {}
                } else {
                    OperatingState::Mining {}
                }
            })
    }
}

impl GetDevFeeConnected for Forgeaxe110 {}

impl GetIsMining for Forgeaxe110 {}
impl GetPools for Forgeaxe110 {
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
            last_share_time: None,
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
            last_share_time: None,
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
impl SetFaultLight for Forgeaxe110 {
    fn supports_set_fault_light(&self) -> bool {
        false
    }
}

#[async_trait]
impl SetPowerLimit for Forgeaxe110 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsPoolsConfig for Forgeaxe110 {
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
impl Restart for Forgeaxe110 {
    fn supports_restart(&self) -> bool {
        false
    }
}

#[async_trait]
impl Pause for Forgeaxe110 {
    fn supports_pause(&self) -> bool {
        false
    }
}

#[async_trait]
impl Resume for Forgeaxe110 {
    fn supports_resume(&self) -> bool {
        false
    }
}

impl ChangePassword for Forgeaxe110 {
    fn supports_change_password(&self) -> bool {
        false
    }
}

impl ReadLogs for Forgeaxe110 {
    fn supports_read_logs(&self) -> bool {
        false
    }
}

impl FactoryReset for Forgeaxe110 {
    fn supports_factory_reset(&self) -> bool {
        false
    }
}

impl RestoreStockOs for Forgeaxe110 {}

#[async_trait]
impl SupportsScalingConfig for Forgeaxe110 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for Forgeaxe110 {
    fn supports_upgrade_firmware(&self) -> bool {
        false
    }
}

impl HasAuth for Forgeaxe110 {}
impl SupportsTimezoneConfig for Forgeaxe110 {}
impl HasDefaultAuth for Forgeaxe110 {}

impl Validate for Forgeaxe110 {
    type Firmware = ForgeaxeFirmware;

    fn validate(version: Option<&semver::Version>) -> bool {
        version.is_some_and(|v| *v >= Version::new(1, 1, 0))
    }
}

#[async_trait]
impl SupportsTuningConfig for Forgeaxe110 {
    fn supports_tuning_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsFanConfig for Forgeaxe110 {
    fn supports_fan_config(&self) -> bool {
        false
    }
}

impl SupportsTemperatureConfig for Forgeaxe110 {}
impl GetTuningPercent for Forgeaxe110 {}
impl SetHashboardsEnabled for Forgeaxe110 {}

impl SetTuningPercent for Forgeaxe110 {}

impl SupportsPresets for Forgeaxe110 {}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, net::IpAddr, time::Duration};

    use asic_rs_core::{
        data::{
            collector::{DataCollector, DataField},
            command::MinerCommand,
        },
        test::api::MockAPIClient,
    };
    use asic_rs_makes_forgeaxe::models::ForgeaxeModel;
    use serde_json::Value;

    use super::*;

    #[tokio::test]
    async fn collects_forgeos_v1_1_telemetry() -> anyhow::Result<()> {
        // Synthetic response using the fields and types emitted by ForgeOS v1.1:
        // https://github.com/WantClue/forge-os/blob/v1.1/main/http_server/http_server.c
        let response: Value =
            serde_json::from_str(include_str!("../../test/json/v1_1_0/system_info.json"))?;
        let command = MinerCommand::WebAPI {
            command: "system/info",
            parameters: None,
        };
        let api = MockAPIClient::new(HashMap::from([(command, response)]));
        let miner = Forgeaxe110::new(IpAddr::from([127, 0, 0, 1]), ForgeaxeModel::BitForgeNano);
        let mut collector = DataCollector::new_with_client(&miner, &api);
        let data = miner.parse_data(collector.collect_all().await);

        assert_eq!(data.device_info.firmware, "ForgeOS Stock");
        assert_eq!(data.device_info.make, "Forgeaxe");
        assert_eq!(data.device_info.model, "BitForgeNano");
        assert_eq!(data.total_chips, Some(2));
        assert_eq!(
            data.control_board_version.map(|cb| cb.to_string()),
            Some("B800".into())
        );
        assert_eq!(data.firmware_version.as_deref(), Some("v1.1"));
        assert_eq!(data.hostname.as_deref(), Some("BitForge"));
        assert_eq!(data.hashrate.map(|h| h.value), Some(1.2));
        assert!(
            data.expected_hashrate
                .is_some_and(|h| (h.value - 2.448).abs() < 1e-9)
        );
        assert_eq!(data.wattage.map(|w| w.as_watts()), Some(42.67));
        assert_eq!(data.uptime, Some(Duration::from_secs(38)));
        assert_eq!(data.best_share, Some(666_000_000_000.0));
        assert_eq!(data.session_best_share, Some(33_000_000.0));
        assert!(
            data.fans[0]
                .rpm
                .is_some_and(|r| (r.as_rpm() - 3333.0).abs() < 1e-9)
        );
        assert_eq!(data.hashboards[0].working_chips, Some(2));
        assert_eq!(
            data.hashboards[0]
                .inlet_chip_temperature
                .map(|t| t.as_celsius()),
            Some(60.0)
        );
        assert_eq!(
            data.hashboards[0].board_temperature.map(|t| t.as_celsius()),
            Some(55.0)
        );
        let pools = data.pools;
        assert_eq!(pools[0].pools[0].user.as_deref(), Some("asic-rs.nano-1"));
        assert_eq!(
            pools[0].pools[0].url.as_ref().map(|url| url.port),
            Some(34255)
        );
        assert_eq!(pools[0].pools[0].active, Some(true));
        assert_eq!(pools[0].pools[1].active, Some(false));
        Ok(())
    }

    #[test]
    fn numeric_fallback_flag_selects_backup_pool() -> anyhow::Result<()> {
        let mut response: Value =
            serde_json::from_str(include_str!("../../test/json/v1_1_0/system_info.json"))?;
        response["isUsingFallbackStratum"] = Value::from(1);
        let miner = Forgeaxe110::new(IpAddr::from([127, 0, 0, 1]), ForgeaxeModel::BitForgeNano);
        let pools = miner.parse_pools(&HashMap::from([(DataField::Pools, response)]));
        assert_eq!(pools[0].pools[0].active, Some(false));
        assert_eq!(pools[0].pools[1].active, Some(true));
        assert_eq!(
            pools[0].pools[1].url.as_ref().map(|url| url.port),
            Some(3333)
        );
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires a ForgeOS miner; set FORGEAXE_IP (read-only)"]
    async fn live_discovery_and_telemetry() -> anyhow::Result<()> {
        use asic_rs_core::traits::entry::FirmwareEntry;

        let ip: IpAddr = std::env::var("FORGEAXE_IP")?.parse()?;
        let miner = ForgeaxeFirmware::default().build_miner(ip, None).await?;
        assert!(miner.revalidate().await?);
        let data = miner.get_data().await;
        assert_eq!(data.device_info.firmware, "ForgeOS Stock");
        assert!(data.firmware_version.is_some());
        assert!(data.hashrate.is_some());
        assert!(!data.pools.is_empty());
        Ok(())
    }

    #[test]
    fn forgeos_has_an_independent_version_range() {
        assert!(!Forgeaxe110::validate(Some(&Version::new(1, 0, 0))));
        assert!(Forgeaxe110::validate(Some(&Version::new(1, 1, 0))));
        assert!(Forgeaxe110::validate(Some(&Version::new(1, 7, 0))));
        assert!(!Forgeaxe110::validate(None));
    }
}

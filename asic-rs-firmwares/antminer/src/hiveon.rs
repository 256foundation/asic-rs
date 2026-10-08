// SPDX-License-Identifier: Apache-2.0
// Contract source: native pyasic HiveonModern and its exact VERSION/Type mapping.

use std::{
    collections::HashMap,
    fmt::{self, Display},
    net::IpAddr,
    str::FromStr,
    time::Duration,
};

use asic_rs_core::{
    config::collector::{ConfigCollector, ConfigField, ConfigLocation},
    data::{
        board::{BoardData, MinerControlBoard},
        collector::{DataCollector, DataExtractor, DataField, DataLocation, get_by_pointer},
        command::{DiscoveryCommand, MinerCommand},
        device::DeviceInfo,
        fan::FanData,
        hashrate::HashRate,
        message::MinerMessage,
        miner::TuningTarget,
        pool::PoolGroupData,
    },
    discovery::{HTTP_WEB_ROOT, RPC_VERSION},
    errors::ModelSelectionError,
    traits::{
        discovery::DiscoveryCommands,
        entry::FirmwareEntry,
        firmware::MinerFirmware,
        identification::{FirmwareIdentification, WebResponse},
        miner::*,
        model::MinerModel,
    },
};
use asic_rs_makes_antminer::models::AntMinerModel;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{Power, Temperature};
use serde_json::Value;

use crate::backends::v2020::AntMinerV2020;

#[derive(Default, Debug)]
pub struct HiveonFirmware;

impl Display for HiveonFirmware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Hiveon")
    }
}

impl DiscoveryCommands for HiveonFirmware {
    fn get_discovery_commands(&self) -> Vec<DiscoveryCommand> {
        vec![RPC_VERSION, HTTP_WEB_ROOT]
    }
}

impl FirmwareIdentification for HiveonFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        response.to_ascii_uppercase().contains("HIVEON")
    }
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.to_ascii_uppercase().contains("HIVEON")
    }
}

fn model_from_version(value: &Value) -> Result<AntMinerModel, ModelSelectionError> {
    if !HiveonFirmware.identify_rpc(&value.to_string()) {
        return Err(ModelSelectionError::UnexpectedModelResponse);
    }
    let Some(raw) = value
        .pointer("/VERSION/0/Type")
        .and_then(Value::as_str)
        .filter(|raw| !raw.trim().is_empty())
    else {
        // Firmware identity is proven above; an absent product name is not
        // evidence for any stock algorithm or board/chip template.
        return Ok(AntMinerModel::Unknown("Hiveon".to_owned()));
    };
    AntMinerModel::from_str(&raw.to_uppercase())
}

#[async_trait]
impl MinerFirmware for HiveonFirmware {
    type Model = AntMinerModel;
    async fn get_model(ip: IpAddr) -> Result<Self::Model, ModelSelectionError> {
        let client = AntMinerV2020::new(ip, AntMinerModel::Unknown("Hiveon".to_owned()));
        let value = client
            .get_api_result(&MinerCommand::RPC {
                command: "version",
                parameters: None,
            })
            .await
            .map_err(|_| ModelSelectionError::NoModelResponse)?;
        if !Self.identify_rpc(&value.to_string()) {
            return Err(ModelSelectionError::UnexpectedModelResponse);
        }
        model_from_version(&value)
    }
    async fn get_version(_ip: IpAddr) -> Option<semver::Version> {
        None
    }
}

#[async_trait]
impl FirmwareEntry for HiveonFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = Self::get_model(ip).await?;
        let mut miner = HiveonMiner::new(ip, model);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(Box::new(miner))
    }
}

/// Reuses the evidenced BMMiner telemetry schema without inheriting stock controls.
#[derive(Debug)]
pub struct HiveonMiner {
    inner: AntMinerV2020,
}

impl HiveonMiner {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        let mut inner = AntMinerV2020::new(ip, model);
        inner.device_info.firmware = HiveonFirmware.to_string();
        Self { inner }
    }
}

#[async_trait]
impl APIClient for HiveonMiner {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        let readable = match command {
            MinerCommand::RPC {
                command,
                parameters,
            } => {
                parameters.is_none()
                    && matches!(*command, "version" | "stats" | "summary" | "pools")
            }
            MinerCommand::WebAPI {
                command,
                parameters,
            } => {
                parameters.is_none()
                    && matches!(
                        *command,
                        "get_system_info"
                            | "get_network_info"
                            | "miner_type"
                            | "get_blink_status"
                            | "get_miner_conf"
                            | "stats"
                            | "summary"
                    )
            }
            _ => false,
        };
        if !readable {
            anyhow::bail!("Hiveon telemetry supports read-only commands only");
        }
        self.inner.get_api_result(command).await
    }
}

impl GetDataLocations for HiveonMiner {
    fn get_locations(&self, field: DataField) -> Vec<DataLocation> {
        if field == DataField::FirmwareVersion {
            return vec![(
                MinerCommand::RPC {
                    command: "version",
                    parameters: None,
                },
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/VERSION/0/CompileTime"),
                    tag: None,
                },
            )];
        }
        self.inner
            .get_locations(field)
            .into_iter()
            .filter(|(command, _)| {
                !matches!(
                    command,
                    MinerCommand::RPC {
                        parameters: Some(_),
                        ..
                    }
                )
            })
            .collect()
    }
}
impl GetIP for HiveonMiner {
    fn get_ip(&self) -> IpAddr {
        self.inner.get_ip()
    }
}
impl GetDeviceInfo for HiveonMiner {
    fn get_device_info(&self) -> DeviceInfo {
        self.inner.get_device_info()
    }
}
impl CollectData for HiveonMiner {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}
impl GetConfigsLocations for HiveonMiner {
    fn get_configs_locations(&self, _field: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}
impl CollectConfigs for HiveonMiner {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}
impl HasAuth for HiveonMiner {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.inner.set_auth(auth);
    }
}
impl HasDefaultAuth for HiveonMiner {
    fn default_auth() -> MinerAuth {
        AntMinerV2020::default_auth()
    }
}
impl Validate for HiveonMiner {
    type Firmware = HiveonFirmware;
}

macro_rules! forward_read {
    ($trait:ident, $method:ident, $result:ty) => {
        impl $trait for HiveonMiner {
            fn $method(&self, data: &HashMap<DataField, Value>) -> $result {
                self.inner.$method(data)
            }
        }
    };
}
forward_read!(GetMAC, parse_mac, Option<MacAddr>);
forward_read!(GetHostname, parse_hostname, Option<String>);
forward_read!(GetApiVersion, parse_api_version, Option<String>);
forward_read!(GetFirmwareVersion, parse_firmware_version, Option<String>);
forward_read!(
    GetControlBoardVersion,
    parse_control_board_version,
    Option<MinerControlBoard>
);
forward_read!(GetSerialNumber, parse_serial_number, Option<String>);
forward_read!(GetHashboards, parse_hashboards, Vec<BoardData>);
forward_read!(GetHashrate, parse_hashrate, Option<HashRate>);
forward_read!(
    GetExpectedHashrate,
    parse_expected_hashrate,
    Option<HashRate>
);
forward_read!(GetFans, parse_fans, Vec<FanData>);
impl GetWattage for HiveonMiner {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        self.inner.parse_wattage(data)
    }
    fn parse_wattage_source(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        self.inner
            .parse_wattage_source(data)
            .map(|source| source.replacen("antminer.", "hiveon.", 1))
    }
}
forward_read!(GetTuningTarget, parse_tuning_target, Option<TuningTarget>);
forward_read!(
    GetScaledTuningTarget,
    parse_scaled_tuning_target,
    Option<TuningTarget>
);
forward_read!(GetLightFlashing, parse_light_flashing, Option<bool>);
forward_read!(GetMessages, parse_messages, Vec<MinerMessage>);
forward_read!(GetUptime, parse_uptime, Option<Duration>);
forward_read!(GetIsMining, parse_is_mining, bool);
forward_read!(GetPools, parse_pools, Vec<PoolGroupData>);
impl GetFluidTemperature for HiveonMiner {
    fn parse_fluid_temperature(&self, data: &HashMap<DataField, Value>) -> Option<Temperature> {
        self.inner.parse_fluid_temperature(data)
    }
    fn parse_outlet_fluid_temperature(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<Temperature> {
        self.inner.parse_outlet_fluid_temperature(data)
    }
}
impl GetPsuFans for HiveonMiner {}
impl GetTuningCapabilities for HiveonMiner {}
impl GetTuningPercent for HiveonMiner {}
impl GetBestShare for HiveonMiner {}
impl GetSessionBestShare for HiveonMiner {}
impl GetOperatingState for HiveonMiner {}
impl GetDevFeeConnected for HiveonMiner {}

macro_rules! unsupported_control {
    ($trait:ident, $method:ident) => {
        impl $trait for HiveonMiner {
            fn $method(&self) -> bool {
                false
            }
        }
    };
}
unsupported_control!(SetFaultLight, supports_set_fault_light);
unsupported_control!(SetPowerLimit, supports_set_power_limit);
unsupported_control!(Restart, supports_restart);
unsupported_control!(Pause, supports_pause);
unsupported_control!(Resume, supports_resume);
unsupported_control!(ChangePassword, supports_change_password);
unsupported_control!(FactoryReset, supports_factory_reset);
unsupported_control!(ReadLogs, supports_read_logs);
unsupported_control!(SupportsPoolsConfig, supports_pools_config);
unsupported_control!(SupportsScalingConfig, supports_scaling_config);
impl SetHashboardsEnabled for HiveonMiner {}
impl SetTuningPercent for HiveonMiner {}
impl SupportsTemperatureConfig for HiveonMiner {}
impl SupportsTuningConfig for HiveonMiner {}
impl SupportsFanConfig for HiveonMiner {}
impl SupportsTimezoneConfig for HiveonMiner {}
impl SupportsPresets for HiveonMiner {}
impl UpgradeFirmware for HiveonMiner {}
impl RestoreStockOs for HiveonMiner {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::json::v2020::AM_STATS;
    use asic_rs_core::test::api::MockAPIClient;
    use serde_json::json;

    #[test]
    fn hiveon_suffix_preserves_exact_identity_and_unknown_models() {
        assert_eq!(
            model_from_version(&json!({"VERSION": [{"Type": "Antminer S19x88 HIVEON"}]})).unwrap(),
            AntMinerModel::S19NoPIC
        );
        assert_eq!(
            model_from_version(&json!({"VERSION": [{"Type": "Antminer S99 HIVEON"}]})).unwrap(),
            AntMinerModel::Unknown("ANTMINER S99 HIVEON".to_owned())
        );
        assert!(!HiveonFirmware.identify_rpc("ANTMINER S19"));
        assert!(HiveonFirmware.identify_rpc("ANTMINER S19 HIVEON"));
    }

    #[test]
    fn missing_or_empty_product_keeps_only_proven_firmware_identity() {
        for version in [
            json!({"Firmware": "HIVEON"}),
            json!({"Firmware": "HIVEON", "Type": ""}),
            json!({"Firmware": "HIVEON", "Type": "  "}),
        ] {
            let model = model_from_version(&json!({"VERSION": [version]})).unwrap();
            assert_eq!(model, AntMinerModel::Unknown("Hiveon".to_owned()));
            let miner = HiveonMiner::new("127.0.0.1".parse().unwrap(), model);
            let info = miner.get_device_info();
            assert_eq!(info.firmware, "Hiveon");
            assert_eq!(
                info.algo,
                asic_rs_core::data::device::HashAlgorithm::Unknown
            );
            assert!(info.hardware.boards.is_none());
            assert!(info.hardware.fans.is_none());
        }
        assert!(model_from_version(&json!({"VERSION": [{}]})).is_err());
        assert!(model_from_version(&json!({"VERSION": [{"Type": ""}]})).is_err());
        assert!(model_from_version(&json!({"VERSION": [{"Type": "Antminer S19"}]})).is_err());
    }

    #[tokio::test]
    async fn shared_cgminer_schema_collects_telemetry_without_control_capabilities() {
        let miner = HiveonMiner::new("127.0.0.1".parse().unwrap(), AntMinerModel::S19NoPIC);
        // A captured stock cgminer response proves the shared parser contract;
        // this is not represented as live Hiveon acceptance.
        let mock = MockAPIClient::new(HashMap::from([(
            MinerCommand::RPC {
                command: "stats",
                parameters: None,
            },
            serde_json::from_str(AM_STATS).unwrap(),
        )]));
        let mut collector = DataCollector::new_with_client(&miner, &mock);
        let data = miner.parse_data(collector.collect_all().await);
        assert_eq!(data.device_info.firmware, "Hiveon");
        assert!(data.hashrate.is_some());
        assert_eq!(data.hashboards.len(), 3);
        assert!(!miner.supports_pause());
        assert!(!miner.supports_resume());
        assert!(!miner.supports_restart());
        assert!(!miner.supports_set_fault_light());
        assert!(!miner.supports_set_power_limit());
        assert!(!miner.supports_pools_config());
        assert!(!miner.supports_upgrade_firmware());
        assert!(!miner.supports_factory_reset());
        assert!(!miner.supports_change_password());
        assert!(
            miner
                .get_api_result(&MinerCommand::RPC {
                    command: "restart",
                    parameters: None
                })
                .await
                .is_err()
        );
        assert!(
            miner
                .get_api_result(&MinerCommand::WebAPI {
                    command: "set_miner_conf",
                    parameters: Some(json!({}))
                })
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn captured_hiveon_reports_total_power_with_raw_field_provenance() {
        use crate::test::json::v2023_07::HIVEON_S19X88_STATS_CAPTURED;
        let miner = HiveonMiner::new("127.0.0.1".parse().unwrap(), AntMinerModel::S19NoPIC);
        let raw: Value = serde_json::from_str(HIVEON_S19X88_STATS_CAPTURED).unwrap();
        let mock = MockAPIClient::new(HashMap::from([(
            MinerCommand::RPC {
                command: "stats",
                parameters: None,
            },
            raw,
        )]));
        let mut collector = DataCollector::new_with_client(&miner, &mock);
        let data = miner.parse_data(collector.collect_all().await);
        assert_eq!(data.wattage.unwrap().as_watts(), 1345.0);
        assert_eq!(
            data.wattage_source.as_deref(),
            Some("hiveon.rpc.stats:total_power")
        );
        // The field is firmware-reported power. Its producer does not establish
        // whether this is measured or estimated, and board sums round differently.
        assert!(data.wattage_is_estimated.is_none());
        assert!(!miner.supports_set_power_limit());
    }
}

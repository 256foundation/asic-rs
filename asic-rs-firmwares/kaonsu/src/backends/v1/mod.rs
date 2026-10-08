// SPDX-License-Identifier: Apache-2.0
use crate::{firmware::KaonsuFirmware, telemetry, web::KaonsuWebAPI};
use asic_rs_core::{
    config::collector::{ConfigCollector, ConfigField, ConfigLocation},
    data::{
        board::{BoardData, MinerControlBoard},
        collector::{DataCollector, DataExtractor, DataField, DataLocation},
        command::MinerCommand,
        device::{DeviceInfo, MinerHardware},
        fan::FanData,
        hashrate::HashRate,
        message::MinerMessage,
        operating_state::OperatingState,
    },
    traits::{miner::*, model::MinerModelAlgorithm},
};
use asic_rs_makes_antminer::models::AntMinerModel;
use async_trait::async_trait;
use measurements::{Power, Temperature};
use serde_json::Value;
use std::{collections::HashMap, net::IpAddr, time::Duration};

#[derive(Debug)]
pub struct KaonsuMiner {
    ip: IpAddr,
    device_info: DeviceInfo,
    web: KaonsuWebAPI,
}
impl KaonsuMiner {
    #[cfg(test)]
    pub fn new(ip: IpAddr, model: AntMinerModel) -> Self {
        Self::with_auth(ip, model, Self::default_auth())
    }
    pub fn with_auth(ip: IpAddr, model: AntMinerModel, auth: MinerAuth) -> Self {
        let algo = model.hash_algorithm();
        let mut device_info = DeviceInfo::new(model, KaonsuFirmware, algo);
        // Mara reports its own board shape; stock model capacity is not authoritative.
        device_info.hardware = MinerHardware::default();
        Self {
            ip,
            device_info,
            web: KaonsuWebAPI::new(ip, auth),
        }
    }
}
#[async_trait]
impl APIClient for KaonsuMiner {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command,
                parameters: None,
            } => self.web.get(command).await,
            _ => anyhow::bail!("KaonSu telemetry supports read-only GET commands only"),
        }
    }
}
fn location(command: &'static str, tag: Option<&'static str>) -> DataLocation {
    (
        MinerCommand::WebAPI {
            command,
            parameters: None,
        },
        DataExtractor {
            func: telemetry::payload,
            key: None,
            tag,
        },
    )
}
impl GetDataLocations for KaonsuMiner {
    fn get_locations(&self, field: DataField) -> Vec<DataLocation> {
        match field {
            DataField::FirmwareVersion | DataField::ControlBoardVersion => {
                vec![location("overview", None)]
            }
            DataField::Hashrate
            | DataField::ExpectedHashrate
            | DataField::Wattage
            | DataField::Uptime
            | DataField::IsMining
            | DataField::OperatingState => vec![location("brief", None)],
            DataField::Hashboards => vec![
                location("hashboards", Some("boards")),
                location("brief", Some("brief")),
            ],
            DataField::Chips => vec![location("hashboards", None)],
            DataField::Fans => vec![location("fans", None)],
            DataField::Messages => vec![
                location("brief", Some("brief")),
                location("hashboards", Some("boards")),
            ],
            _ => vec![],
        }
    }
}
impl GetIP for KaonsuMiner {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}
impl GetDeviceInfo for KaonsuMiner {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}
impl CollectData for KaonsuMiner {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}
impl GetConfigsLocations for KaonsuMiner {
    fn get_configs_locations(&self, _field: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}
impl CollectConfigs for KaonsuMiner {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}
impl HasAuth for KaonsuMiner {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}
impl HasDefaultAuth for KaonsuMiner {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("root", "root")
    }
}
#[async_trait]
impl Validate for KaonsuMiner {
    type Firmware = KaonsuFirmware;

    async fn revalidate(&self) -> anyhow::Result<bool> {
        let Ok(model) = crate::firmware::read_identity_with_api(&self.web).await else {
            return Ok(false);
        };
        Ok(model.to_string() == self.device_info.model)
    }
}
impl GetFirmwareVersion for KaonsuMiner {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::FirmwareVersion)?
            .get("version_firmware")?
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    }
}
impl GetHashrate for KaonsuMiner {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        telemetry::hashrate(data.get(&DataField::Hashrate)?, self.device_info.algo)
    }
}
impl GetExpectedHashrate for KaonsuMiner {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        telemetry::expected_hashrate(
            data.get(&DataField::ExpectedHashrate)?,
            self.device_info.algo,
        )
    }
}
impl GetHashboards for KaonsuMiner {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        data.get(&DataField::Hashboards)
            .map(|value| {
                telemetry::boards_with_unit(
                    value
                        .get("boards")
                        .filter(|boards| boards.is_object())
                        .unwrap_or(value),
                    self.device_info.algo,
                    value
                        .get("brief")
                        .and_then(|brief| brief.get("hashrate_unit")),
                )
            })
            .unwrap_or_default()
    }
    fn parse_expected_hashboards(&self, data: &HashMap<DataField, Value>) -> Option<u8> {
        let value = data.get(&DataField::Hashboards)?;
        telemetry::expected_boards(value.get("boards").unwrap_or(value))
    }
    fn parse_reported_max_temperature(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<Temperature> {
        telemetry::reported_max_temperature(data.get(&DataField::Hashboards)?.get("brief")?)
    }
}
impl GetFans for KaonsuMiner {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        data.get(&DataField::Fans)
            .map(telemetry::fans)
            .unwrap_or_default()
    }
}
impl GetWattage for KaonsuMiner {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        telemetry::power(data.get(&DataField::Wattage)?)
    }
    fn parse_wattage_source(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        self.parse_wattage(data)
            .map(|_| "kaonsu.brief:power_consumption_estimated".to_owned())
    }
    fn parse_wattage_is_estimated(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        self.parse_wattage(data).map(|_| true)
    }
    fn parse_wattage_firmware_source(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::Wattage)?
            .get("power_source")?
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    }
    fn parse_wattage_indicator(&self, data: &HashMap<DataField, Value>) -> Option<i64> {
        let value = data.get(&DataField::Wattage)?.get("power_indicator")?;
        value
            .as_i64()
            .or_else(|| value.as_str()?.trim().parse().ok())
    }
}
impl GetIsMining for KaonsuMiner {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        data.get(&DataField::IsMining)
            .is_some_and(|value| telemetry::is_mining(value, self.device_info.algo))
    }
}
impl GetOperatingState for KaonsuMiner {
    fn parse_operating_state(&self, data: &HashMap<DataField, Value>) -> Option<OperatingState> {
        telemetry::operating_state(data.get(&DataField::OperatingState)?)
    }
}
impl GetMessages for KaonsuMiner {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let Some(value) = data.get(&DataField::Messages) else {
            return vec![];
        };
        let mut messages = value
            .get("brief")
            .map(telemetry::messages)
            .unwrap_or_default();
        if let Some(rows) = value
            .get("boards")
            .and_then(|boards| {
                ["hashboards", "boards", "chains"]
                    .into_iter()
                    .find_map(|key| boards.get(key))
            })
            .and_then(Value::as_array)
        {
            for board in rows {
                let identity = ["index", "id", "slot", "chain_id"]
                    .into_iter()
                    .find_map(|key| board.get(key));
                for mut message in telemetry::messages(board) {
                    if let Some(identity) = identity {
                        message.message = format!("Hashboard {identity}: {}", message.message);
                    }
                    messages.push(message);
                }
            }
        }
        messages
    }
}
impl GetMAC for KaonsuMiner {}
impl GetHostname for KaonsuMiner {}
impl GetApiVersion for KaonsuMiner {}
impl GetControlBoardVersion for KaonsuMiner {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.get(&DataField::ControlBoardVersion)?
            .get("control_board")?
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .map(MinerControlBoard::unknown)
    }
}
impl GetSerialNumber for KaonsuMiner {}
impl GetFluidTemperature for KaonsuMiner {}
impl GetPsuFans for KaonsuMiner {}
impl GetLightFlashing for KaonsuMiner {}
impl GetTuningCapabilities for KaonsuMiner {}
impl GetTuningPercent for KaonsuMiner {}
impl GetTuningTarget for KaonsuMiner {}
impl GetScaledTuningTarget for KaonsuMiner {}
impl GetUptime for KaonsuMiner {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        telemetry::uptime(data.get(&DataField::Uptime)?)
    }
}
impl GetBestShare for KaonsuMiner {}
impl GetSessionBestShare for KaonsuMiner {}
impl GetDevFeeConnected for KaonsuMiner {}
impl GetPools for KaonsuMiner {}

macro_rules! unsupported_control {
    ($trait:ident, $method:ident) => {
        impl $trait for KaonsuMiner {
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
impl SetHashboardsEnabled for KaonsuMiner {}
impl SetTuningPercent for KaonsuMiner {}
impl SupportsTemperatureConfig for KaonsuMiner {}
impl SupportsTuningConfig for KaonsuMiner {}
impl SupportsFanConfig for KaonsuMiner {}
impl SupportsTimezoneConfig for KaonsuMiner {}
impl SupportsPresets for KaonsuMiner {}
impl UpgradeFirmware for KaonsuMiner {}
impl RestoreStockOs for KaonsuMiner {}

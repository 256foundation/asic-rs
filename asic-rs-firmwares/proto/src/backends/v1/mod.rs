use std::{collections::HashMap, net::IpAddr, time::Duration};

use anyhow::anyhow;
use asic_rs_core::{
    config::{
        collector::{
            ConfigCollector, ConfigExtractor, ConfigField, ConfigLocation,
            get_by_pointer as get_config_by_pointer,
        },
        fan::FanConfig,
        pools::{PoolConfig, PoolGroupConfig},
        scaling::ScalingConfig,
        tuning::TuningConfig,
    },
    data::{
        board::{BoardData, MinerControlBoard},
        collector::{
            DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
        },
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm, MinerHardware},
        fan::FanData,
        hashrate::{HashRate, HashRateUnit},
        message::{MessageSeverity, MinerComponent, MinerMessage},
        miner::TuningTarget,
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{
        auth::{HasAuth, HasDefaultAuth, MinerAuth},
        miner::*,
        model::MinerModel,
    },
};
use asic_rs_makes_proto::hardware::ProtoControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Power, Temperature};
use serde_json::{Value, json};

use super::v1::web::ProtoWebAPI;
use crate::firmware::ProtoFirmware;

pub mod web;

#[derive(Debug)]
pub struct ProtoV1 {
    ip: IpAddr,
    auth: MinerAuth,
    device_info: DeviceInfo,
    web: ProtoWebAPI,
}

impl ProtoV1 {
    pub fn new(
        ip: IpAddr,
        model: impl MinerModel,
        _version: Option<semver::Version>,
        hardware: MinerHardware,
    ) -> Self {
        let auth = Self::default_auth();
        let web = ProtoWebAPI::new(ip, auth.clone());
        let mut device_info =
            DeviceInfo::new(model, ProtoFirmware::default(), HashAlgorithm::SHA256);
        // Layout is discovered from the device, not model-derived.
        device_info.hardware = hardware;
        Self {
            ip,
            auth,
            device_info,
            web,
        }
    }

    /// Discover the hardware layout from the authenticated `/api/v1/hardware`
    /// endpoint; empty if unreachable.
    pub async fn discover_hardware(ip: IpAddr, auth: &MinerAuth) -> MinerHardware {
        const WEB_HARDWARE: MinerCommand = MinerCommand::WebAPI {
            command: "/api/v1/hardware",
            parameters: None,
        };
        let web = ProtoWebAPI::new(ip, auth.clone());
        match web.get_api_result(&WEB_HARDWARE).await {
            Ok(hardware) => Self::hardware_from_response(&hardware),
            Err(_) => MinerHardware::default(),
        }
    }

    /// Parse board/fan counts from a `/api/v1/hardware` response. `boards` is
    /// slot-indexed by chassis position, since slots can be sparse.
    fn hardware_from_response(hardware: &Value) -> MinerHardware {
        let boards = hardware
            .pointer("/hardware-info/hashboards-info")
            .and_then(Value::as_array);
        let fans = hardware
            .pointer("/hardware-info/fans-info")
            .and_then(Value::as_array);

        MinerHardware {
            boards: boards.map(|b| {
                let board_count = b
                    .iter()
                    .filter_map(|board| board.get("slot").and_then(Value::as_u64))
                    .max()
                    .unwrap_or(b.len() as u64) as usize;
                let mut boards = vec![None; board_count];
                for board in b {
                    let slot = board.get("slot").and_then(Value::as_u64).unwrap_or(0);
                    let Some(position) = slot.checked_sub(1).map(|slot| slot as usize) else {
                        continue;
                    };
                    if let Some(expected_chips) = board
                        .get("mining_asic_count")
                        .and_then(Value::as_u64)
                        .and_then(|count| u16::try_from(count).ok())
                        && let Some(board) = boards.get_mut(position)
                    {
                        *board = Some(expected_chips);
                    }
                }
                boards
            }),
            fans: fans.map(|f| f.len() as u8),
        }
    }
}

#[async_trait]
impl APIClient for ProtoV1 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow!("Unsupported command type for Proto API")),
        }
    }
}

impl GetConfigsLocations for ProtoV1 {
    fn get_configs_locations(&self, config_field: ConfigField) -> Vec<ConfigLocation> {
        const WEB_POOLS: MinerCommand = MinerCommand::WebAPI {
            command: "/api/v1/pools",
            parameters: None,
        };
        const WEB_COOLING: MinerCommand = MinerCommand::WebAPI {
            command: "/api/v1/cooling",
            parameters: None,
        };
        const WEB_MINING_TARGET: MinerCommand = MinerCommand::WebAPI {
            command: "/api/v1/mining/target",
            parameters: None,
        };

        match config_field {
            ConfigField::Pools => vec![(
                WEB_POOLS,
                ConfigExtractor {
                    func: get_config_by_pointer,
                    key: Some("/pools"),
                    tag: None,
                },
            )],
            ConfigField::Fan => vec![(
                WEB_COOLING,
                ConfigExtractor {
                    func: get_config_by_pointer,
                    key: Some("/cooling-status"),
                    tag: None,
                },
            )],
            ConfigField::Tuning => vec![(
                WEB_MINING_TARGET,
                ConfigExtractor {
                    func: get_config_by_pointer,
                    key: Some("/power_target_watts"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl CollectConfigs for ProtoV1 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for ProtoV1 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const WEB_SYSTEM: MinerCommand = MinerCommand::WebAPI {
            command: "/api/v1/system",
            parameters: None,
        };
        const WEB_NETWORK: MinerCommand = MinerCommand::WebAPI {
            command: "/api/v1/network",
            parameters: None,
        };
        const WEB_MINING: MinerCommand = MinerCommand::WebAPI {
            command: "/api/v1/mining",
            parameters: None,
        };
        const WEB_HARDWARE: MinerCommand = MinerCommand::WebAPI {
            command: "/api/v1/hardware",
            parameters: None,
        };
        const WEB_POOLS: MinerCommand = MinerCommand::WebAPI {
            command: "/api/v1/pools",
            parameters: None,
        };
        const WEB_COOLING: MinerCommand = MinerCommand::WebAPI {
            command: "/api/v1/cooling",
            parameters: None,
        };
        const WEB_ERRORS: MinerCommand = MinerCommand::WebAPI {
            command: "/api/v1/errors",
            parameters: None,
        };
        // Carries a query param, so it's a `let`, not a `const` like the others.
        let web_telemetry_full = MinerCommand::WebAPI {
            command: "/api/v1/telemetry",
            parameters: Some(json!({ "level": "miner,hashboard,psu" })),
        };

        match data_field {
            DataField::Mac => vec![(
                WEB_NETWORK,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/network-info/mac"),
                    tag: None,
                },
            )],
            DataField::SerialNumber => vec![(
                WEB_HARDWARE,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/hardware-info/cb-info/serial_number"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                WEB_NETWORK,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/network-info/hostname"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![(
                WEB_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system-info/web_server/version"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                WEB_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system-info/os/version"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                WEB_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system-info/board"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![
                (
                    WEB_HARDWARE,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/hardware-info/hashboards-info"),
                        tag: Some("hardware"),
                    },
                ),
                (
                    WEB_MINING,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/mining-status"),
                        tag: Some("mining"),
                    },
                ),
                (
                    web_telemetry_full.clone(),
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/hashboards"),
                        tag: Some("telemetry"),
                    },
                ),
            ],
            DataField::Hashrate => vec![(
                web_telemetry_full.clone(),
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner/hashrate/value"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                WEB_MINING,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/mining-status/ideal_hashrate_ghs"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                WEB_COOLING,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/cooling-status/fans"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                web_telemetry_full,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner/power/value"),
                    tag: None,
                },
            )],
            DataField::TuningTarget => vec![(
                WEB_MINING,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/mining-status/power_target_watts"),
                    tag: None,
                },
            )],
            DataField::Messages => vec![(
                WEB_ERRORS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                WEB_MINING,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/mining-status/reboot_uptime_s"),
                    tag: None,
                },
            )],
            DataField::IsMining => vec![(
                WEB_MINING,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/mining-status/status"),
                    tag: None,
                },
            )],
            DataField::Pools => vec![(
                WEB_POOLS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/pools"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for ProtoV1 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for ProtoV1 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for ProtoV1 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for ProtoV1 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.get(&DataField::Mac)
            .and_then(Value::as_str)
            .and_then(|raw| raw.parse().ok())
    }
}

impl GetSerialNumber for ProtoV1 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::SerialNumber)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    }
}

impl GetHostname for ProtoV1 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::Hostname)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    }
}

impl GetApiVersion for ProtoV1 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for ProtoV1 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetControlBoardVersion for ProtoV1 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<ProtoControlBoard>(DataField::ControlBoardVersion)
            .map(Into::into)
    }
}

#[async_trait]
impl GetHashboards for ProtoV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let Some(hashboards_payload) = data.get(&DataField::Hashboards) else {
            return Vec::new();
        };
        let hardware_boards = hashboards_payload
            .get("hardware")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let telemetry_boards = hashboards_payload
            .get("telemetry")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mining = hashboards_payload.get("mining");
        let ideal_hashrate = mining
            .and_then(|v| v.get("ideal_hashrate_ghs"))
            .and_then(Value::as_f64);
        let status = mining
            .and_then(|v| v.get("status").and_then(Value::as_str))
            .unwrap_or("Unknown");
        let mining_boards = mining
            .and_then(|v| v.get("hashboards_mining"))
            .and_then(Value::as_u64);

        let board_count = hardware_boards.len().max(1) as f64;

        // Only populated slots are reported, positioned by slot, so a sparse
        // chassis (e.g. slots 1,3,4,6,7,9) yields just those boards.
        let mut hashboards: Vec<BoardData> = hardware_boards
            .iter()
            .enumerate()
            .map(|(idx, board)| {
                let slot = board
                    .get("slot")
                    .and_then(Value::as_u64)
                    .unwrap_or((idx + 1) as u64);
                let asic_count = board
                    .get("mining_asic_count")
                    .and_then(Value::as_u64)
                    .map(|v| v as u16);
                let position = slot.saturating_sub(1) as u8;
                let mut hashboard = BoardData::new(
                    position,
                    self.device_info.hardware.chips_for_board(position as usize),
                );
                hashboard.expected_chips = asic_count.or(hashboard.expected_chips);
                hashboard.working_chips = asic_count;
                hashboard.serial_number = board
                    .get("hb_sn")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                hashboard
            })
            .collect();

        for board in &mut hashboards {
            let telemetry = board
                .serial_number
                .as_deref()
                .and_then(|serial| {
                    telemetry_boards.iter().find(|entry| {
                        entry.get("serial_number").and_then(Value::as_str) == Some(serial)
                    })
                })
                .or_else(|| {
                    telemetry_boards.iter().find(|entry| {
                        entry.get("index").and_then(Value::as_u64) == Some(board.position as u64)
                    })
                });

            let Some(telemetry) = telemetry else {
                continue;
            };

            let telemetry_hashrate = telemetry.pointer("/hashrate/value").and_then(Value::as_f64);

            board.hashrate = telemetry_hashrate.map(|ths| {
                HashRate {
                    value: ths,
                    unit: HashRateUnit::TeraHash,
                    algo: "SHA256".to_string(),
                }
                .as_unit(HashRateUnit::default())
            });

            board.expected_hashrate = ideal_hashrate
                .map(|ghs| HashRate {
                    value: ghs / board_count,
                    unit: HashRateUnit::GigaHash,
                    algo: "SHA256".to_string(),
                })
                .map(|hr| hr.as_unit(HashRateUnit::default()));

            board.board_temperature = telemetry
                .pointer("/temperature/average")
                .and_then(Value::as_f64)
                .map(Temperature::from_celsius);
            board.inlet_chip_temperature = telemetry
                .pointer("/temperature/inlet")
                .and_then(Value::as_f64)
                .map(Temperature::from_celsius);
            board.outlet_chip_temperature = telemetry
                .pointer("/temperature/outlet")
                .and_then(Value::as_f64)
                .map(Temperature::from_celsius);
            board.voltage = telemetry
                .pointer("/voltage/value")
                .and_then(Value::as_f64)
                .map(measurements::Voltage::from_volts);

            let board_active = telemetry_hashrate.map(|v| v > 0.0).unwrap_or(false)
                || (matches!(status, "Mining" | "DegradedMining")
                    && mining_boards
                        .map(|v| v > board.position as u64)
                        .unwrap_or(false));
            board.active = Some(board_active);
        }

        hashboards
    }
}

impl GetHashrate for ProtoV1 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |ths| {
            HashRate {
                value: ths,
                unit: HashRateUnit::TeraHash,
                algo: "SHA256".to_string(),
            }
            .as_unit(HashRateUnit::default())
        })
    }
}

impl GetExpectedHashrate for ProtoV1 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |ghs| {
            HashRate {
                value: ghs,
                unit: HashRateUnit::GigaHash,
                algo: "SHA256".to_string(),
            }
            .as_unit(HashRateUnit::default())
        })
    }
}

impl GetFans for ProtoV1 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        data.get(&DataField::Fans)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|fan| FanData {
                position: fan.get("slot").and_then(Value::as_i64).unwrap_or_default() as i16,
                rpm: fan
                    .get("rpm")
                    .and_then(Value::as_f64)
                    .map(AngularVelocity::from_rpm),
            })
            .collect()
    }
}

impl GetPsuFans for ProtoV1 {}
impl GetFluidTemperature for ProtoV1 {}

impl GetWattage for ProtoV1 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}

impl GetTuningTarget for ProtoV1 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        data.extract::<f64>(DataField::TuningTarget)
            .map(|watts| TuningTarget::Power(Power::from_watts(watts)))
    }
}

impl GetScaledTuningTarget for ProtoV1 {}
impl GetTuningCapabilities for ProtoV1 {}
// No endpoint reports locate-LED state, so it's set-only.
impl GetLightFlashing for ProtoV1 {}

/// Map a Proto error `source` (plus its slot) onto the shared component type.
fn message_component(source: &str, slot: u64) -> Option<MinerComponent> {
    let idx = slot as u16;
    match source.trim().to_ascii_lowercase().as_str() {
        "hashboard" | "hb" => Some(MinerComponent::hashboard(idx)),
        "fan" => Some(MinerComponent::fan(idx)),
        "psu" | "power" | "power_supply" | "powersupply" => Some(MinerComponent::power_supply(idx)),
        "controlboard" | "control_board" | "control" | "cb" => {
            Some(MinerComponent::control_board())
        }
        _ => None,
    }
}

impl GetMessages for ProtoV1 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        data.get(&DataField::Messages)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|item| {
                let source = item.get("source").and_then(Value::as_str).unwrap_or("rig");
                let slot = item.get("slot").and_then(Value::as_u64).unwrap_or_default();
                // `error_code` is a string in the MDK spec. Put it in the u64
                // `code` when numeric; if it's non-numeric, surface it in the
                // message instead so it isn't lost.
                let error_code = item
                    .get("error_code")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let raw_message = item
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let parsed = error_code.parse::<u64>().ok();
                let message = match parsed {
                    Some(_) => raw_message.to_string(),
                    None if error_code.is_empty() => raw_message.to_string(),
                    None => format!("{error_code}: {raw_message}"),
                };
                let code = parsed.unwrap_or_default();
                let timestamp = item
                    .get("timestamp")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as u32;
                MinerMessage::with_component(
                    timestamp,
                    code,
                    message,
                    MessageSeverity::Warning,
                    message_component(source, slot),
                )
            })
            .collect()
    }
}

impl GetUptime for ProtoV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetIsMining for ProtoV1 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        matches!(
            data.extract::<String>(DataField::IsMining).as_deref(),
            Some("Mining" | "DegradedMining" | "PoweringOn")
        )
    }
}

impl GetPools for ProtoV1 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let pools = data
            .get(&DataField::Pools)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|pool| PoolData {
                position: pool
                    .get("priority")
                    .and_then(Value::as_u64)
                    .map(|v| v as u16),
                url: pool
                    .get("url")
                    .and_then(Value::as_str)
                    .map(|raw| PoolURL::from(raw.to_string())),
                accepted_shares: pool.get("accepted").and_then(Value::as_u64),
                rejected_shares: pool.get("rejected").and_then(Value::as_u64),
                active: pool
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|status| status == "Active"),
                alive: pool
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|status| status != "Dead"),
                user: pool
                    .get("user")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
            })
            .collect::<Vec<_>>();

        if pools.is_empty() {
            vec![]
        } else {
            vec![PoolGroupData {
                name: "Default".to_string(),
                quota: 1,
                pools,
            }]
        }
    }
}

#[async_trait]
impl SetFaultLight for ProtoV1 {
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        // The locate LED auto-expires and can't be turned off early; disabling
        // is a no-op.
        if fault {
            self.web.locate().await?;
        }
        Ok(true)
    }

    fn supports_set_fault_light(&self) -> bool {
        true
    }
}

#[async_trait]
impl SetPowerLimit for ProtoV1 {
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        self.web
            .set_miner_target(limit.as_watts().round() as u64)
            .await?;
        Ok(true)
    }

    fn supports_set_power_limit(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for ProtoV1 {
    async fn restart(&self) -> anyhow::Result<bool> {
        self.web.reboot().await?;
        Ok(true)
    }

    fn supports_restart(&self) -> bool {
        true
    }
}

#[async_trait]
impl Pause for ProtoV1 {
    async fn pause(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        self.web.mining_stop().await?;
        Ok(true)
    }

    fn supports_pause(&self) -> bool {
        true
    }
}

#[async_trait]
impl Resume for ProtoV1 {
    async fn resume(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        self.web.mining_start().await?;
        Ok(true)
    }

    fn supports_resume(&self) -> bool {
        true
    }
}

#[async_trait]
impl UpgradeFirmware for ProtoV1 {}

#[async_trait]
impl ChangePassword for ProtoV1 {
    async fn change_password(&mut self, password: &str) -> anyhow::Result<bool> {
        let success = self.web.set_password(password).await?;
        if success {
            let username = self.auth.username().to_string();
            self.set_auth(MinerAuth::new(username, password));
        }
        Ok(success)
    }

    fn supports_change_password(&self) -> bool {
        true
    }
}

impl FactoryReset for ProtoV1 {
    fn supports_factory_reset(&self) -> bool {
        false
    }
}

#[async_trait]
impl ReadLogs for ProtoV1 {
    async fn read_logs(&self) -> anyhow::Result<String> {
        self.web.read_logs().await
    }

    fn supports_read_logs(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsPoolsConfig for ProtoV1 {
    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> anyhow::Result<bool> {
        let payload = config
            .into_iter()
            .flat_map(|group| group.pools.into_iter())
            .enumerate()
            .map(|(idx, pool)| {
                json!({
                    "name": format!("Pool {}", idx + 1),
                    "url": pool.url.to_string(),
                    "username": pool.username,
                    "password": pool.password,
                    "priority": idx,
                })
            })
            .collect::<Vec<_>>();

        self.web.set_pools(json!(payload)).await?;
        Ok(true)
    }

    fn parse_pools_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<Vec<PoolGroupConfig>> {
        let pools = data
            .get(&ConfigField::Pools)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|pool| {
                Some(PoolConfig {
                    url: PoolURL::from(pool.get("url")?.as_str()?.to_string()),
                    username: pool.get("user")?.as_str()?.to_string(),
                    password: "x".to_string(),
                })
            })
            .collect::<Vec<_>>();

        Ok(if pools.is_empty() {
            vec![]
        } else {
            vec![PoolGroupConfig {
                name: "Default".to_string(),
                quota: 1,
                pools,
            }]
        })
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsScalingConfig for ProtoV1 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsTuningConfig for ProtoV1 {
    async fn set_tuning_config(
        &self,
        config: TuningConfig,
        _scaling_config: Option<ScalingConfig>,
    ) -> anyhow::Result<bool> {
        if let TuningTarget::Power(power) = config.target {
            return self.set_power_limit(power).await;
        }
        Err(anyhow!(
            "Proto tuning currently supports power targets only"
        ))
    }

    fn parse_tuning_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<TuningConfig> {
        let watts = data
            .get(&ConfigField::Tuning)
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("missing power_target_watts"))?;
        Ok(TuningConfig::new(TuningTarget::Power(Power::from_watts(
            watts,
        ))))
    }

    fn supports_tuning_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsFanConfig for ProtoV1 {
    async fn set_fan_config(&self, config: FanConfig) -> anyhow::Result<bool> {
        let payload = match config {
            FanConfig::Auto { target_temp, .. } => json!({
                "mode": "Auto",
                "target_temperature_c": target_temp,
            }),
            FanConfig::Manual { fan_speed } => json!({
                "mode": "Manual",
                "speed_percentage": fan_speed,
            }),
        };
        self.web.set_cooling(payload).await?;
        Ok(true)
    }

    fn parse_fan_config(&self, data: &HashMap<ConfigField, Value>) -> anyhow::Result<FanConfig> {
        let status = data
            .get(&ConfigField::Fan)
            .ok_or_else(|| anyhow!("missing cooling config"))?;
        let mode = status
            .get("fan_mode")
            .and_then(Value::as_str)
            .unwrap_or("Auto");

        match mode {
            "Manual" => Ok(FanConfig::manual(
                status
                    .get("speed_percentage")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
            )),
            _ => Ok(FanConfig::auto(
                status
                    .get("target_temperature_c")
                    .and_then(Value::as_f64)
                    .unwrap_or(50.0),
                None,
            )),
        }
    }

    fn supports_fan_config(&self) -> bool {
        true
    }
}

impl HasDefaultAuth for ProtoV1 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("", "")
    }
}

impl HasAuth for ProtoV1 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth.clone();
        self.web.set_auth(auth);
    }
}

impl SupportsTemperatureConfig for ProtoV1 {}
impl GetTuningPercent for ProtoV1 {}
impl SetTuningPercent for ProtoV1 {}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use anyhow::Result;
    use asic_rs_core::{
        data::{
            collector::DataField,
            command::MinerCommand,
            hashrate::{HashRate, HashRateUnit},
        },
        test::api::MockAPIClient,
        traits::{
            identification::{FirmwareIdentification, WebResponse},
            miner::{GetHashboards, GetMessages, GetMinerData},
        },
    };
    use serde_json::{Value, json};

    use super::*;
    use crate::test::json::v1::{
        COOLING, ERRORS, HARDWARE, MINING, NETWORK, POOLS, SYSTEM, TELEMETRY_FULL,
    };

    fn parse_fixture(raw: &str) -> Value {
        Value::from_str(raw).expect("fixture parses")
    }

    fn command_results() -> HashMap<MinerCommand, Value> {
        let mut results = HashMap::new();

        let commands = [
            ("/api/v1/system", SYSTEM),
            ("/api/v1/network", NETWORK),
            ("/api/v1/hardware", HARDWARE),
            ("/api/v1/mining", MINING),
            ("/api/v1/pools", POOLS),
            ("/api/v1/cooling", COOLING),
            ("/api/v1/errors", ERRORS),
        ];

        for (command, data) in commands {
            results.insert(
                MinerCommand::WebAPI {
                    command,
                    parameters: None,
                },
                parse_fixture(data),
            );
        }

        // Telemetry carries query parameters, so it can't go through the loop.
        results.insert(
            MinerCommand::WebAPI {
                command: "/api/v1/telemetry",
                parameters: Some(json!({ "level": "miner,hashboard,psu" })),
            },
            parse_fixture(TELEMETRY_FULL),
        );
        results
    }

    #[test]
    fn identify_web_matches_dashboard_root() {
        // Identify from the dashboard `/` page title.
        let root_page =
            r#"<!doctype html><html><head><title>Proto OS</title></head><body></body></html>"#;
        let web = WebResponse {
            body: root_page,
            auth_header: "",
            algo_header: "",
            redirect_header: "",
            status: 200,
        };

        assert!(ProtoFirmware::default().identify_web(&web));
        // The system JSON must not match.
        let system = WebResponse {
            body: SYSTEM,
            auth_header: "",
            algo_header: "",
            redirect_header: "",
            status: 200,
        };
        assert!(!ProtoFirmware::default().identify_web(&system));
    }

    #[tokio::test]
    async fn parse_data_test_proto_rig() -> Result<()> {
        // Hardware shape is discovered and passed into the constructor.
        let miner = ProtoV1::new(
            IpAddr::from([127, 0, 0, 1]),
            asic_rs_makes_proto::models::ProtoModel::Rig,
            Some(semver::Version::parse("1.8.0").expect("version parses")),
            ProtoV1::hardware_from_response(&parse_fixture(HARDWARE)),
        );
        assert_eq!(miner.device_info.hardware.board_count(), Some(4));
        assert_eq!(miner.device_info.hardware.total_chips(), Some(480));
        assert_eq!(
            miner.device_info.hardware.boards,
            Some(vec![Some(120), Some(120), Some(120), Some(120)])
        );
        assert_eq!(miner.device_info.hardware.fans, Some(4));

        let mock_api = MockAPIClient::new(command_results());

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect(&[DataField::Hashboards]).await;

        let hashboards = miner.parse_hashboards(&data);
        assert_eq!(hashboards.len(), 4);
        // Per-chip detail comes from an undocumented endpoint and is no longer
        // collected; board-level data is still populated.
        assert!(hashboards[0].chips.is_empty());
        assert_eq!(
            hashboards[0].serial_number.as_deref(),
            Some("HB-PROTO-SIM-b6bdaf86-0")
        );
        assert!(hashboards[0].hashrate.is_some());
        assert!(hashboards[0].board_temperature.is_some());
        assert!(hashboards[0].inlet_chip_temperature.is_some());
        assert!(hashboards[0].outlet_chip_temperature.is_some());
        assert!(hashboards[0].voltage.is_some());
        assert_eq!(hashboards[0].expected_chips, Some(120));
        assert_eq!(hashboards[0].working_chips, Some(120));

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;

        let parsed = miner.parse_data(data);

        assert_eq!(parsed.hostname.as_deref(), Some("proto-miner-af86"));
        assert_eq!(
            parsed.mac.map(|m| m.to_string()),
            Some("02:00:00:AD:66:39".to_string())
        );
        assert_eq!(parsed.serial_number.as_deref(), Some("PROTO-SIM-b6bdaf86"));
        assert_eq!(parsed.expected_hashboards, Some(4));
        assert_eq!(parsed.expected_fans, Some(4));
        assert_eq!(parsed.expected_chips, Some(480));
        assert_eq!(parsed.total_chips, Some(480));
        assert_eq!(parsed.hashboards.len(), 4);
        assert!(parsed.hashboards[0].chips.is_empty());
        assert_eq!(
            parsed.hashboards[0]
                .expected_hashrate
                .as_ref()
                .expect("board expected hashrate")
                .unit,
            HashRateUnit::TeraHash
        );
        assert_eq!(
            parsed.hashboards[0]
                .expected_hashrate
                .as_ref()
                .expect("board expected hashrate")
                .value,
            36.25
        );
        assert!(
            parsed.hashboards[0]
                .board_temperature
                .expect("board temperature")
                .as_celsius()
                > 40.0
        );
        assert!(
            parsed.hashboards[0]
                .inlet_chip_temperature
                .expect("intake temperature")
                .as_celsius()
                > 30.0
        );
        assert!(
            parsed.hashboards[0]
                .outlet_chip_temperature
                .expect("outlet temperature")
                .as_celsius()
                > 50.0
        );
        assert!(
            parsed.hashboards[0]
                .voltage
                .expect("board voltage")
                .as_volts()
                > 10.0
        );
        assert_eq!(parsed.fans.len(), 4);
        assert!(parsed.is_mining);
        assert!(parsed.uptime.expect("uptime") > Duration::from_secs(0));
        assert_eq!(parsed.pools.len(), 1);
        assert_eq!(parsed.pools[0].pools.len(), 2);
        assert_eq!(parsed.pools[0].pools[0].position, Some(0));
        assert_eq!(
            parsed.pools[0].pools[0]
                .url
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("stratum+tcp://pool-a.example.com:3333")
        );
        assert_eq!(parsed.pools[0].pools[0].user.as_deref(), Some("worker-a"));
        assert_eq!(parsed.pools[0].pools[0].accepted_shares, Some(12345));
        assert_eq!(parsed.pools[0].pools[0].rejected_shares, Some(10));
        assert_eq!(parsed.pools[0].pools[0].active, Some(true));
        assert_eq!(parsed.pools[0].pools[0].alive, Some(true));
        assert_eq!(
            parsed.expected_hashrate.expect("expected hashrate"),
            HashRate {
                value: 145.0,
                unit: HashRateUnit::TeraHash,
                algo: "SHA256".to_string(),
            }
        );
        assert!(
            parsed
                .hashrate
                .as_ref()
                .expect("hashrate")
                .clone()
                .as_unit(HashRateUnit::TeraHash)
                .value
                > 100.0
        );
        assert!(parsed.wattage.expect("wattage").as_watts() > 3000.0);
        Ok(())
    }

    #[test]
    fn sparse_hardware_slots_preserve_capacity_and_parse_populated_boards() {
        let hardware = json!({
            "hardware-info": {
                "hashboards-info": [
                    {"slot": 1, "hb_sn": "HB-1", "mining_asic_count": 120},
                    {"slot": 3, "hb_sn": "HB-3", "mining_asic_count": 118},
                    {"slot": 4, "hb_sn": "HB-4", "mining_asic_count": 119},
                    {"slot": 6, "hb_sn": "HB-6", "mining_asic_count": 120},
                    {"slot": 7, "hb_sn": "HB-7", "mining_asic_count": 117},
                    {"slot": 9, "hb_sn": "HB-9", "mining_asic_count": 116}
                ],
                "fans-info": [
                    {"slot": 1},
                    {"slot": 2},
                    {"slot": 3},
                    {"slot": 4}
                ]
            }
        });
        let discovered = ProtoV1::hardware_from_response(&hardware);

        assert_eq!(discovered.board_count(), Some(9));
        assert_eq!(
            discovered.boards,
            Some(vec![
                Some(120),
                None,
                Some(118),
                Some(119),
                None,
                Some(120),
                Some(117),
                None,
                Some(116),
            ])
        );
        assert_eq!(discovered.total_chips(), Some(710));
        assert_eq!(discovered.fans, Some(4));

        let miner = ProtoV1::new(
            IpAddr::from([127, 0, 0, 1]),
            asic_rs_makes_proto::models::ProtoModel::Rig,
            Some(semver::Version::parse("1.8.0").expect("version parses")),
            discovered,
        );
        let mut data = HashMap::new();
        data.insert(
            DataField::Hashboards,
            json!({
                "hardware": hardware.pointer("/hardware-info/hashboards-info").expect("boards"),
                "mining": {
                    "ideal_hashrate_ghs": 900_000.0,
                    "status": "Mining",
                    "hashboards_mining": 6
                },
                "telemetry": []
            }),
        );

        let hashboards = miner.parse_hashboards(&data);
        assert_eq!(hashboards.len(), 6);
        assert_eq!(
            hashboards
                .iter()
                .map(|board| board.position)
                .collect::<Vec<_>>(),
            vec![0, 2, 3, 5, 6, 8]
        );
        assert_eq!(
            hashboards
                .iter()
                .map(|board| board.expected_chips)
                .collect::<Vec<_>>(),
            vec![
                Some(120),
                Some(118),
                Some(119),
                Some(120),
                Some(117),
                Some(116),
            ]
        );
        assert_eq!(
            hashboards
                .iter()
                .map(|board| board.working_chips)
                .collect::<Vec<_>>(),
            vec![
                Some(120),
                Some(118),
                Some(119),
                Some(120),
                Some(117),
                Some(116),
            ]
        );
    }

    #[test]
    fn parse_messages_maps_component_and_code() {
        // Shaped per the MDK `NotificationError` schema (error_code is a
        // string); a healthy rig returns `[]`, so drive the parser directly.
        let miner = ProtoV1::new(
            IpAddr::from([127, 0, 0, 1]),
            asic_rs_makes_proto::models::ProtoModel::Rig,
            None,
            MinerHardware::default(),
        );
        let mut data = HashMap::new();
        data.insert(
            DataField::Messages,
            json!([
                {"source": "hashboard", "slot": 2, "error_code": "1024",
                 "message": "Hashboard 2 over temperature", "timestamp": 1718000000},
                {"source": "psu", "slot": 0, "error_code": "PSU_FAULT",
                 "message": "PSU voltage out of range", "timestamp": 1718000100}
            ]),
        );

        let messages = miner.parse_messages(&data);
        assert_eq!(messages.len(), 2);
        // Numeric code goes in `code` only (not duplicated into the message);
        // component is derived from source/slot.
        assert_eq!(messages[0].code, 1024);
        assert_eq!(messages[0].message, "Hashboard 2 over temperature");
        assert_eq!(messages[0].component, Some(MinerComponent::hashboard(2)));
        // Non-numeric error_code can't fit the u64 `code` (stays 0), so it's
        // surfaced in the message instead.
        assert_eq!(messages[1].code, 0);
        assert_eq!(messages[1].message, "PSU_FAULT: PSU voltage out of range");
        assert_eq!(messages[1].component, Some(MinerComponent::power_supply(0)));
    }
}

impl SupportsPresets for ProtoV1 {}

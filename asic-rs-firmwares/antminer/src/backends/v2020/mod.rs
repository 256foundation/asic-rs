use std::{collections::HashMap, fmt::Display, net::IpAddr, str::FromStr, time::Duration};

use anyhow;
use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigExtractor, ConfigField, ConfigLocation},
        fan::FanConfig,
        pools::{PoolConfig, PoolGroupConfig},
        tuning::TuningConfig,
    },
    data::{
        board::{BoardData, MinerControlBoard},
        collector::{
            DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
        },
        command::MinerCommand,
        device::DeviceInfo,
        fan::FanData,
        firmware::FirmwareImage,
        hashrate::HashRate,
        message::MinerMessage,
        miner::{MiningMode, TuningTarget},
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_antminer::hardware::AntMinerControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{Power, Temperature};
use rpc::AntMinerRPCAPI;
use semver::Version;
use serde_json::{Value, json};
use web::AntMinerWebAPI;

use self::firmware::resolve_firmware_image;
use crate::{backends::telemetry, firmware::AntMinerStockFirmware};

mod firmware;
mod rpc;
pub(crate) mod web;

#[derive(Debug)]
pub struct AntMinerV2020 {
    pub ip: IpAddr,
    pub rpc: AntMinerRPCAPI,
    pub web: AntMinerWebAPI,
    pub device_info: DeviceInfo,
}

#[allow(dead_code)]
enum MinerMode {
    Sleep,
    Low,
    Normal,
    High,
}

impl Display for MinerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MinerMode::Sleep => "1",
            MinerMode::Low => "3",
            MinerMode::High => "2",
            _ => "0",
        };
        f.write_str(s)
    }
}

impl MinerMode {
    fn as_web_value(&self) -> u8 {
        match self {
            MinerMode::Sleep => 1,
            MinerMode::Low => 3,
            MinerMode::Normal => 0,
            MinerMode::High => 2,
        }
    }
}

fn miner_mode_config_key(miner_conf: &Value) -> Option<&'static str> {
    if miner_conf.get("miner-mode").is_some() {
        Some("miner-mode")
    } else if miner_conf.get("bitmain-work-mode").is_some() {
        Some("bitmain-work-mode")
    } else {
        None
    }
}

fn browser_miner_conf_payload(miner_conf: &Value) -> serde_json::Map<String, Value> {
    let mut payload = serde_json::Map::new();
    payload.insert(
        "bitmain-fan-ctrl".to_string(),
        miner_conf
            .get("bitmain-fan-ctrl")
            .cloned()
            .unwrap_or(Value::Bool(false)),
    );
    payload.insert(
        "bitmain-fan-pwm".to_string(),
        miner_conf
            .get("bitmain-fan-pwm")
            .cloned()
            .unwrap_or(Value::String("100".to_string())),
    );

    if let Some(mode_key) = miner_mode_config_key(miner_conf)
        && let Some(mode) = miner_conf.get(mode_key)
    {
        payload.insert(mode_key.to_string(), mode.clone());
    }
    payload.insert(
        "freq-level".to_string(),
        miner_conf
            .get("freq-level")
            .or_else(|| miner_conf.get("bitmain-freq-level"))
            .cloned()
            .unwrap_or(Value::String("100".to_string())),
    );
    payload.insert(
        "pools".to_string(),
        miner_conf
            .get("pools")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
    );

    payload
}

/// `set_miner_conf.cgi` only overwrites the pool slots it is sent, so a list
/// shorter than the miner's three slots leaves the old trailing pools in
/// place. Blank the unused slots so they are removed.
fn pools_payload(config: Vec<PoolGroupConfig>) -> Vec<Value> {
    let mut pools: Vec<Value> = config
        .into_iter()
        .flat_map(|group| group.pools.into_iter())
        .map(|pool| {
            json!({
                "url": pool.url.to_string(),
                "user": pool.username,
                "pass": pool.password,
            })
        })
        .collect();

    pools.resize(3, json!({ "url": "", "user": "", "pass": "" }));
    pools
}

fn miner_conf_with_pools(miner_conf: &Value, pools: Vec<Value>) -> Value {
    let mut payload = browser_miner_conf_payload(miner_conf);
    payload.insert("pools".to_string(), Value::Array(pools));
    Value::Object(payload)
}

fn miner_conf_with_miner_mode(miner_conf: &Value, mode: MinerMode) -> Option<Value> {
    miner_mode_config_key(miner_conf)?;
    let mut payload = browser_miner_conf_payload(miner_conf);
    let mode = Value::from(mode.as_web_value());
    payload.insert("miner-mode".to_string(), mode.clone());
    payload.insert("bitmain-work-mode".to_string(), mode);
    Some(Value::Object(payload))
}

fn miner_mode_from_value(mode: &Value) -> Option<MiningMode> {
    let mode = mode
        .as_str()
        .map(str::to_owned)
        .or_else(|| mode.as_i64().map(|mode| mode.to_string()))?;

    match mode.as_str() {
        "0" => Some(MiningMode::Normal),
        "2" => Some(MiningMode::High),
        "3" => Some(MiningMode::Low),
        _ => None,
    }
}

fn miner_conf_mining_mode(miner_conf: &Value) -> Option<MiningMode> {
    ["miner-mode", "bitmain-work-mode"]
        .iter()
        .filter_map(|key| miner_conf.get(key))
        .find_map(miner_mode_from_value)
}

fn bool_from_value(value: &Value) -> Option<bool> {
    value.as_bool().or_else(|| {
        value.as_str().and_then(|s| match s {
            "1" => Some(true),
            "0" => Some(false),
            _ if s.eq_ignore_ascii_case("true") => Some(true),
            _ if s.eq_ignore_ascii_case("false") => Some(false),
            _ => None,
        })
    })
}

fn u64_from_value(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<u64>().ok()))
}

fn miner_conf_with_fan_config(miner_conf: &Value, config: FanConfig) -> Value {
    let mut payload = browser_miner_conf_payload(miner_conf);
    match config {
        FanConfig::Auto { .. } => {
            payload.insert("bitmain-fan-ctrl".to_string(), Value::Bool(false));
        }
        FanConfig::Manual { fan_speed } => {
            payload.insert("bitmain-fan-ctrl".to_string(), Value::Bool(true));
            payload.insert(
                "bitmain-fan-pwm".to_string(),
                Value::String(fan_speed.min(100).to_string()),
            );
        }
    }
    Value::Object(payload)
}

impl AntMinerV2020 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        let auth = Self::default_auth();
        let algo = model.hash_algorithm();
        AntMinerV2020 {
            ip,
            rpc: AntMinerRPCAPI::new(ip),
            web: AntMinerWebAPI::new(ip, auth),
            device_info: DeviceInfo::new(model, AntMinerStockFirmware::default(), algo),
        }
    }
}

#[async_trait]
impl APIClient for AntMinerV2020 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for {} API",
                AntMinerStockFirmware::default()
            )),
        }
    }
}

impl GetConfigsLocations for AntMinerV2020 {
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        const WEB_GET_MINER_CONF: MinerCommand = MinerCommand::WebAPI {
            command: "get_miner_conf",
            parameters: None,
        };
        match data_field {
            ConfigField::Pools => vec![(
                WEB_GET_MINER_CONF,
                ConfigExtractor {
                    func: get_by_pointer,
                    key: Some("/pools"),
                    tag: None,
                },
            )],
            ConfigField::Tuning | ConfigField::Fan => vec![(
                WEB_GET_MINER_CONF,
                ConfigExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl CollectConfigs for AntMinerV2020 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for AntMinerV2020 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const RPC_VERSION: MinerCommand = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };

        const RPC_POOLS: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };

        const WEB_SYSTEM_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "get_system_info",
            parameters: None,
        };

        const WEB_BLINK_STATUS: MinerCommand = MinerCommand::WebAPI {
            command: "get_blink_status",
            parameters: None,
        };

        const WEB_MINER_CONF: MinerCommand = MinerCommand::WebAPI {
            command: "get_miner_conf",
            parameters: None,
        };

        const WEB_SUMMARY: MinerCommand = MinerCommand::WebAPI {
            command: "summary",
            parameters: None,
        };

        const WEB_MINER_TYPE: MinerCommand = MinerCommand::WebAPI {
            command: "miner_type",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/macaddr"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![(
                RPC_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/VERSION/0/API"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system_filesystem_version"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/hostname"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![
                (
                    WEB_MINER_TYPE,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/subtype"),
                        tag: Some("subtype"),
                    },
                ),
                (
                    WEB_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/system_kernel_version"),
                        tag: Some("kernel_version"),
                    },
                ),
            ],
            DataField::Hashrate => telemetry::rate_locations(),
            DataField::ExpectedHashrate
            | DataField::Fans
            | DataField::Hashboards
            | DataField::FluidTemperature
            | DataField::OutletFluidTemperature => telemetry::stats_locations(),
            DataField::LightFlashing => vec![(
                WEB_BLINK_STATUS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/blink"),
                    tag: None,
                },
            )],
            DataField::IsMining => {
                let mut locations = telemetry::rate_locations();
                locations.extend(["bitmain-work-mode", "miner-mode"].into_iter().map(|key| {
                    (
                        WEB_MINER_CONF,
                        DataExtractor {
                            func: get_by_pointer,
                            key: Some(if key == "miner-mode" {
                                "/miner-mode"
                            } else {
                                "/bitmain-work-mode"
                            }),
                            tag: Some(if key == "miner-mode" {
                                "miner_mode"
                            } else {
                                "work_mode"
                            }),
                        },
                    )
                }));
                locations
            }
            DataField::Uptime => telemetry::stats_locations(),
            DataField::Pools => vec![(
                RPC_POOLS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/POOLS"),
                    tag: None,
                },
            )],
            DataField::Wattage => telemetry::stats_locations(),
            DataField::SerialNumber => vec![
                (
                    WEB_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/serial_no"), // Cant find on 2022 firmware, does exist on 2025 firmware for XP
                        tag: None,
                    },
                ),
                (
                    WEB_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/serinum"), // exist on 2025 firmware for s21
                        tag: None,
                    },
                ),
            ],
            DataField::Messages => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            DataField::TuningTarget => vec![(
                WEB_MINER_CONF,
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

impl GetIP for AntMinerV2020 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for AntMinerV2020 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for AntMinerV2020 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for AntMinerV2020 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetHostname for AntMinerV2020 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for AntMinerV2020 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for AntMinerV2020 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetHashboards for AntMinerV2020 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        data.get(&DataField::Hashboards)
            .map(|value| {
                telemetry::hashboards(
                    value,
                    &self.device_info.model.to_string(),
                    self.device_info.algo,
                    &self.device_info.hardware,
                )
            })
            .unwrap_or_else(|| {
                (0..self.device_info.hardware.board_count().unwrap_or(0))
                    .map(|position| {
                        BoardData::new(
                            position,
                            self.device_info.hardware.chips_for_board(position as usize),
                        )
                    })
                    .collect()
            })
    }
}

impl GetHashrate for AntMinerV2020 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        telemetry::hashrate_for_model(
            data.get(&DataField::Hashrate)?,
            self.device_info.algo,
            Some(&self.device_info.model),
        )
    }
}

impl GetExpectedHashrate for AntMinerV2020 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        telemetry::expected_hashrate(
            data.get(&DataField::ExpectedHashrate)?,
            self.device_info.algo,
        )
    }
}

impl GetFans for AntMinerV2020 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        data.get(&DataField::Fans)
            .map(|value| telemetry::fans_for_model(value, Some(&self.device_info.model)))
            .unwrap_or_default()
    }
}

impl GetLightFlashing for AntMinerV2020 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing).or_else(|| {
            data.extract::<String>(DataField::LightFlashing)
                .map(|s| s.to_lowercase() == "true" || s == "1")
        })
    }
}

impl GetUptime for AntMinerV2020 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        telemetry::uptime(data.get(&DataField::Uptime)?)
    }
}

impl GetBestShare for AntMinerV2020 {}
impl GetSessionBestShare for AntMinerV2020 {}

impl GetOperatingState for AntMinerV2020 {}

impl GetDevFeeConnected for AntMinerV2020 {}

impl GetIsMining for AntMinerV2020 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        let field = data.get(&DataField::IsMining);
        let rate = field
            .and_then(|value| {
                telemetry::current_rate_for_model(
                    value,
                    self.device_info.algo,
                    Some(&self.device_info.model),
                )
            })
            .or_else(|| {
                data.get(&DataField::Hashrate).and_then(|value| {
                    telemetry::current_rate_for_model(
                        value,
                        self.device_info.algo,
                        Some(&self.device_info.model),
                    )
                })
            });
        telemetry::is_mining(field, rate)
    }
}

impl GetPools for AntMinerV2020 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let Some(pools_data) = data.get(&DataField::Pools) else {
            return vec![];
        };

        let Some(pools_array) = pools_data.as_array() else {
            return vec![PoolGroupData {
                name: String::new(),
                quota: 1,
                pools: vec![],
            }];
        };

        let mut rpc_pools: Vec<PoolData> = Vec::with_capacity(pools_array.len());
        for (idx, pool_info) in pools_array.iter().enumerate() {
            let url = pool_info
                .get("URL")
                .and_then(|v| v.as_str())
                .map(|s| PoolURL::from(s.to_string()));

            let accepted_shares = pool_info.get("Accepted").and_then(|v| v.as_u64());

            let rejected_shares = pool_info.get("Rejected").and_then(|v| v.as_u64());

            let last_share_time = pool_info
                .get("Last Share Time")
                .and_then(asic_rs_core::util::parse_last_share_time);

            let active = pool_info.get("Stratum Active").and_then(|v| v.as_bool());

            let alive = pool_info
                .get("Status")
                .and_then(|v| v.as_str())
                .map(|s| s == "Alive");

            let user = pool_info
                .get("User")
                .and_then(|v| v.as_str())
                .map(String::from);

            rpc_pools.push(PoolData {
                position: Some(idx as u16),
                url,
                accepted_shares,
                rejected_shares,
                last_share_time,
                active,
                alive,
                user,
            });
        }

        vec![PoolGroupData {
            name: String::new(),
            quota: 1,
            pools: rpc_pools,
        }]
    }
}

impl GetSerialNumber for AntMinerV2020 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::SerialNumber)
    }
}

impl GetControlBoardVersion for AntMinerV2020 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        let subtype = data.extract_nested::<String>(DataField::ControlBoardVersion, "subtype");
        let board = subtype.as_deref().and_then(|cb_type| {
            if cb_type.to_ascii_uppercase().contains("AML") {
                Some(AntMinerControlBoard::AMLogic)
            } else {
                AntMinerControlBoard::parse(cb_type.split('_').next()?)
            }
        });

        board
            .or_else(|| {
                let kernel_version = data
                    .extract_nested::<String>(DataField::ControlBoardVersion, "kernel_version")?;
                kernel_version
                    .to_ascii_lowercase()
                    .contains("-xilinx")
                    .then_some(AntMinerControlBoard::Xilinx)
            })
            .map(Into::into)
    }
}

impl GetWattage for AntMinerV2020 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        telemetry::wattage(data.get(&DataField::Wattage)?)
    }

    fn parse_wattage_source(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        telemetry::wattage_source(data.get(&DataField::Wattage)?)
    }
}

impl GetTuningTarget for AntMinerV2020 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        data.get(&DataField::TuningTarget)
            .and_then(miner_conf_mining_mode)
            .map(TuningTarget::MiningMode)
    }
}

impl GetScaledTuningTarget for AntMinerV2020 {
    fn parse_scaled_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        self.parse_tuning_target(data)
    }
}

impl GetFluidTemperature for AntMinerV2020 {
    fn parse_fluid_temperature(&self, data: &HashMap<DataField, Value>) -> Option<Temperature> {
        let field = data
            .get(&DataField::FluidTemperature)
            .or_else(|| data.get(&DataField::Hashboards))?;
        let boards = telemetry::hashboards(
            field,
            &self.device_info.model.to_string(),
            self.device_info.algo,
            &self.device_info.hardware,
        );
        telemetry::fluid_temperature(&boards, false)
    }

    fn parse_outlet_fluid_temperature(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<Temperature> {
        let field = data
            .get(&DataField::OutletFluidTemperature)
            .or_else(|| data.get(&DataField::Hashboards))?;
        let boards = telemetry::hashboards(
            field,
            &self.device_info.model.to_string(),
            self.device_info.algo,
            &self.device_info.hardware,
        );
        telemetry::fluid_temperature(&boards, true)
    }
}

impl GetPsuFans for AntMinerV2020 {}
impl GetTuningCapabilities for AntMinerV2020 {}
impl SupportsTimezoneConfig for AntMinerV2020 {}

impl GetMessages for AntMinerV2020 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        data.get(&DataField::Messages)
            .map(telemetry::messages)
            .unwrap_or_default()
    }
}

#[async_trait]
impl SetFaultLight for AntMinerV2020 {
    fn supports_set_fault_light(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        Ok(self.web.blink(fault).await.is_ok())
    }
}

#[async_trait]
impl SetPowerLimit for AntMinerV2020 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsPoolsConfig for AntMinerV2020 {
    fn parse_pools_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<Vec<PoolGroupConfig>> {
        let Some(pools_data) = data.get(&ConfigField::Pools) else {
            return Ok(vec![]);
        };

        let Some(pools_array) = pools_data.as_array() else {
            return Ok(vec![PoolGroupConfig {
                name: String::new(),
                quota: 1,
                pools: vec![],
            }]);
        };

        let mut pools: Vec<PoolConfig> = Vec::with_capacity(pools_array.len());
        for pool in pools_array {
            let Some(url) = pool.get("url").and_then(|v| v.as_str()) else {
                continue;
            };
            if url.is_empty() {
                continue;
            }

            let username = pool
                .get("user")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default();
            let password = pool
                .get("pass")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default();

            pools.push(PoolConfig {
                url: PoolURL::from(url.to_string()),
                username,
                password,
            });
        }

        pools.truncate(3);

        Ok(vec![PoolGroupConfig {
            name: String::new(),
            quota: 1,
            pools,
        }])
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> anyhow::Result<bool> {
        let pools = pools_payload(config);
        let miner_conf = self.web.get_miner_conf().await?;
        let miner_conf = miner_conf_with_pools(&miner_conf, pools);

        Ok(self.web.set_miner_conf(miner_conf).await.is_ok())
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for AntMinerV2020 {
    fn supports_restart(&self) -> bool {
        true
    }
    async fn restart(&self) -> anyhow::Result<bool> {
        Ok(self.web.reboot().await.is_ok())
    }
}

#[async_trait]
impl Pause for AntMinerV2020 {
    fn supports_pause(&self) -> bool {
        true
    }
    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        let pre = self.web.get_miner_conf().await?;
        let Some(miner_conf) = miner_conf_with_miner_mode(&pre, MinerMode::Sleep) else {
            return Ok(false);
        };

        let response = self.web.set_miner_conf(miner_conf).await?;
        if response.get("stats").and_then(Value::as_str) != Some("success") {
            return Ok(false);
        }

        // set_miner_conf.cgi writes the config before starting a miner
        // reload/restart in the background; an immediate follow-up read can
        // race the device becoming temporarily unavailable.
        Ok(true)
    }
}

#[async_trait]
impl Resume for AntMinerV2020 {
    fn supports_resume(&self) -> bool {
        true
    }
    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        let pre = self.web.get_miner_conf().await?;
        let Some(miner_conf) = miner_conf_with_miner_mode(&pre, MinerMode::Normal) else {
            return Ok(false);
        };

        let response = self.web.set_miner_conf(miner_conf).await?;
        if response.get("stats").and_then(Value::as_str) != Some("success") {
            return Ok(false);
        }

        Ok(true)
    }
}

#[async_trait]
impl ChangePassword for AntMinerV2020 {
    async fn change_password(&mut self, password: &str) -> anyhow::Result<bool> {
        let original_auth = self.web.auth();
        let new_auth = MinerAuth::new(original_auth.username().to_string(), password);
        let result = self.web.change_password(password).await;

        match result {
            Ok(false) => Ok(false),
            Ok(true) => {
                self.set_auth(new_auth);
                if self.web.get_miner_conf().await.is_ok() {
                    Ok(true)
                } else {
                    self.set_auth(original_auth);
                    Ok(false)
                }
            }
            Err(err) => {
                self.set_auth(new_auth);
                if self.web.get_miner_conf().await.is_ok() {
                    Ok(true)
                } else {
                    self.set_auth(original_auth);
                    Err(err)
                }
            }
        }
    }

    fn supports_change_password(&self) -> bool {
        true
    }
}

#[async_trait]
impl ReadLogs for AntMinerV2020 {
    async fn read_logs(&self) -> anyhow::Result<String> {
        self.web.read_logs().await
    }

    fn supports_read_logs(&self) -> bool {
        true
    }
}

#[async_trait]
impl FactoryReset for AntMinerV2020 {
    async fn factory_reset(&self) -> anyhow::Result<bool> {
        self.web.factory_reset().await
    }

    fn supports_factory_reset(&self) -> bool {
        true
    }
}

impl RestoreStockOs for AntMinerV2020 {}

#[async_trait]
impl SupportsScalingConfig for AntMinerV2020 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for AntMinerV2020 {
    async fn prepare_firmware(&self, image: FirmwareImage) -> anyhow::Result<FirmwareImage> {
        let miner = self.get_miner_type_info().await?;
        resolve_firmware_image(image, &miner).await
    }

    fn supports_prepare_firmware(&self) -> bool {
        true
    }

    async fn upgrade_firmware(&self, image: FirmwareImage) -> anyhow::Result<bool> {
        let image = self.prepare_firmware(image).await?;
        self.web.upgrade_firmware(image).await?;
        Ok(true)
    }

    fn supports_upgrade_firmware(&self) -> bool {
        true
    }
}

impl HasDefaultAuth for AntMinerV2020 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("root", "root")
    }
}

impl HasAuth for AntMinerV2020 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

impl Validate for AntMinerV2020 {
    type Firmware = AntMinerStockFirmware;

    fn validate(version: Option<&semver::Version>) -> bool {
        version.is_some_and(|v| *v < Version::new(2023, 7, 0))
    }
}

#[async_trait]
impl SupportsTuningConfig for AntMinerV2020 {
    async fn set_tuning_config(
        &self,
        config: TuningConfig,
        _scaling_config: Option<asic_rs_core::config::scaling::ScalingConfig>,
    ) -> anyhow::Result<bool> {
        let mode = match config.target {
            TuningTarget::Manual { .. } => {
                anyhow::bail!(
                    "Manual tuning target is not supported on {} firmware",
                    AntMinerStockFirmware::default()
                )
            }
            TuningTarget::MiningMode(MiningMode::Low) => MinerMode::Low,
            TuningTarget::MiningMode(MiningMode::Normal) => MinerMode::Normal,
            TuningTarget::MiningMode(MiningMode::High) => MinerMode::High,
            TuningTarget::Power(_) => {
                anyhow::bail!(
                    "Power tuning target is not supported on {} firmware",
                    AntMinerStockFirmware::default()
                )
            }
            TuningTarget::HashRate(_) => {
                anyhow::bail!(
                    "Hashrate tuning target is not supported on {} firmware",
                    AntMinerStockFirmware::default()
                )
            }
            TuningTarget::Preset(_) => {
                anyhow::bail!(
                    "Preset tuning target is not supported on {} firmware",
                    AntMinerStockFirmware::default()
                )
            }
        };

        let pre = self.web.get_miner_conf().await?;
        let Some(miner_conf) = miner_conf_with_miner_mode(&pre, mode) else {
            anyhow::bail!(
                "No {} mining mode field found in miner config",
                AntMinerStockFirmware::default()
            )
        };

        self.web.set_miner_conf(miner_conf).await?;
        Ok(true)
    }

    fn parse_tuning_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<TuningConfig> {
        data.get(&ConfigField::Tuning)
            .and_then(miner_conf_mining_mode)
            .map(|mode| TuningConfig::new(TuningTarget::MiningMode(mode)))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No {} mining mode found in tuning config",
                    AntMinerStockFirmware::default()
                )
            })
    }

    fn supports_tuning_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsFanConfig for AntMinerV2020 {
    async fn set_fan_config(&self, config: FanConfig) -> anyhow::Result<bool> {
        let pre = self.web.get_miner_conf().await?;
        self.web
            .set_miner_conf(miner_conf_with_fan_config(&pre, config))
            .await?;
        Ok(true)
    }

    fn parse_fan_config(&self, data: &HashMap<ConfigField, Value>) -> anyhow::Result<FanConfig> {
        let fan = data
            .get(&ConfigField::Fan)
            .ok_or_else(|| anyhow::anyhow!("No fan config data"))?;

        let manual = fan
            .get("bitmain-fan-ctrl")
            .and_then(bool_from_value)
            .unwrap_or(false);
        let fan_speed = fan
            .get("bitmain-fan-pwm")
            .and_then(u64_from_value)
            .unwrap_or(100);

        if manual {
            Ok(FanConfig::manual(fan_speed))
        } else {
            Ok(FanConfig::auto(0.0, Some(fan_speed)))
        }
    }

    fn supports_fan_config(&self) -> bool {
        true
    }
}

impl SupportsTemperatureConfig for AntMinerV2020 {}
impl GetTuningPercent for AntMinerV2020 {}
impl SetHashboardsEnabled for AntMinerV2020 {}

impl SetTuningPercent for AntMinerV2020 {}

impl SupportsPresets for AntMinerV2020 {}

#[cfg(test)]
mod tests {
    use asic_rs_core::data::{device::HashAlgorithm, hashrate::HashRateUnit};
    use std::sync::Arc;

    use anyhow::{self, Context};
    use asic_rs_core::test::{api::MockAPIClient, util::get_miner};
    use asic_rs_makes_antminer::models::AntMinerModel;

    use super::*;
    use crate::test::json::v2020::{
        AM_DEVS, AM_POOLS, AM_STATS, AM_SUMMARY, AM_SYSTEM_INFO, AM_VERSION,
    };

    #[tokio::test]
    async fn control_board_falls_back_to_kernel_and_prefers_subtype() {
        let miner = AntMinerV2020::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::Z15);

        for (subtype, kernel, expected) in [
            (
                None,
                "Linux 4.6.0-xilinx-g0387054-dirty #45 SMP PREEMPT",
                Some("Xilinx"),
            ),
            (
                Some("BeagleBoneBlack_B"),
                "Linux 4.6.0-xilinx",
                Some("BeagleBoneBlack"),
            ),
            (None, "Linux 4.9.113 #1 SMP PREEMPT", None),
        ] {
            let mut results = HashMap::new();
            results.insert(
                MinerCommand::WebAPI {
                    command: "get_system_info",
                    parameters: None,
                },
                json!({"system_kernel_version": kernel}),
            );
            if let Some(subtype) = subtype {
                results.insert(
                    MinerCommand::WebAPI {
                        command: "miner_type",
                        parameters: None,
                    },
                    json!({"subtype": subtype}),
                );
            }

            let mock_api = MockAPIClient::new(results);
            let mut collector = DataCollector::new_with_client(&miner, &mock_api);
            let data = collector.collect(&[DataField::ControlBoardVersion]).await;
            let board = miner.parse_control_board_version(&data);

            assert_eq!(board.as_ref().map(|board| board.name.as_str()), expected);
        }
    }

    #[test]
    fn pools_payload_blanks_the_slots_a_shorter_config_leaves_unused() {
        let pool = |n: u8| PoolConfig {
            url: PoolURL::from(format!("stratum+tcp://pool{n}.example:3333")),
            username: format!("worker.{n}"),
            password: "x".to_string(),
        };

        assert_eq!(
            pools_payload(vec![PoolGroupConfig {
                name: String::new(),
                quota: 1,
                pools: vec![pool(1), pool(2)],
            }]),
            vec![
                json!({"url": "stratum+tcp://pool1.example:3333", "user": "worker.1", "pass": "x"}),
                json!({"url": "stratum+tcp://pool2.example:3333", "user": "worker.2", "pass": "x"}),
                json!({"url": "", "user": "", "pass": ""}),
            ]
        );
    }

    #[test]
    fn set_miner_conf_payload_matches_cgi_contract_for_pause_resume() {
        let pre = json!({
            "pools": [
                {"url": "stratum+tcp://pool1.example:3333", "user": "worker.1", "pass": "x"},
                {"url": "stratum+tcp://pool2.example:3333", "user": "worker.2", "pass": "x"},
                {"url": "stratum+tcp://pool3.example:3333", "user": "worker.3", "pass": "x"}
            ],
            "bitmain-fan-ctrl": true,
            "bitmain-fan-pwm": "70",
            "bitmain-work-mode": "0",
            "bitmain-freq-level": "100",
            "api-listen": true
        });

        assert_eq!(
            miner_conf_with_miner_mode(&pre, MinerMode::Sleep).unwrap(),
            json!({
                "pools": [
                    {"url": "stratum+tcp://pool1.example:3333", "user": "worker.1", "pass": "x"},
                    {"url": "stratum+tcp://pool2.example:3333", "user": "worker.2", "pass": "x"},
                    {"url": "stratum+tcp://pool3.example:3333", "user": "worker.3", "pass": "x"}
                ],
                "bitmain-fan-ctrl": true,
                "bitmain-fan-pwm": "70",
                "freq-level": "100",
                "miner-mode": 1,
                "bitmain-work-mode": 1
            })
        );

        assert_eq!(
            miner_conf_with_miner_mode(&pre, MinerMode::Normal).unwrap()["miner-mode"],
            json!(0)
        );
    }

    /// An L9 is a Scrypt miner reporting in MH/s. Before this was wired up it
    /// came back as a SHA-256 miner and every figure was read as GH/s, putting
    /// the hashrate out by a factor of 1000.
    #[tokio::test]
    async fn scrypt_model_is_read_in_the_unit_it_reports() {
        let miner = AntMinerV2020::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::L9);

        let mut results = HashMap::new();
        results.insert(
            MinerCommand::RPC {
                command: "summary",
                parameters: None,
            },
            json!({ "SUMMARY": [{ "MHS 5s": 16200.0, "MHS av": 16150.0 }] }),
        );
        results.insert(
            MinerCommand::RPC {
                command: "stats",
                parameters: None,
            },
            json!({
                "STATS": [
                    {},
                    {
                        "rate_unit": "MH",
                        "total_rateideal": 16000.0,
                        "chain_rate1": 5400.0,
                        "chain_rate2": 5400.0,
                        "chain_rate3": 5400.0
                    }
                ]
            }),
        );

        let mock_api = MockAPIClient::new(results);
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let miner_data = miner.parse_data(collector.collect_all().await);

        let hashrate = miner_data.hashrate.expect("hashrate");
        assert_eq!(hashrate.algo, HashAlgorithm::Scrypt);
        assert_eq!(hashrate.unit, HashRateUnit::GigaHash);
        assert_mega_hash(hashrate, 16200.0);

        let expected = miner_data.expected_hashrate.expect("expected hashrate");
        assert_eq!(expected.algo, HashAlgorithm::Scrypt);
        assert_eq!(expected.unit, HashRateUnit::GigaHash);
        assert_mega_hash(expected, 16000.0);

        let board = miner_data.hashboards[0].hashrate.clone().expect("board 0");
        assert_eq!(board.algo, HashAlgorithm::Scrypt);
        assert_eq!(board.unit, HashRateUnit::GigaHash);
        assert_mega_hash(board, 5400.0);
    }

    #[track_caller]
    fn assert_mega_hash(hashrate: HashRate, expected: f64) {
        let value = hashrate.as_unit(HashRateUnit::MegaHash).value;
        assert!(
            (value - expected).abs() < 1e-6,
            "expected {expected} MH/s, got {value}"
        );
    }

    #[tokio::test]
    async fn test_antminer() {
        let miner = AntMinerV2020::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19Pro);

        let mut results = HashMap::new();

        let stats_cmd = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };

        let version_cmd = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };

        let summary_cmd = MinerCommand::RPC {
            command: "summary",
            parameters: None,
        };

        let system_info_cmd = MinerCommand::WebAPI {
            command: "get_system_info",
            parameters: None,
        };

        let devs_cmd = MinerCommand::RPC {
            command: "devs",
            parameters: None,
        };

        let pools_cmd = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };

        results.insert(stats_cmd, Value::from_str(AM_STATS).unwrap());
        results.insert(version_cmd, Value::from_str(AM_VERSION).unwrap());
        results.insert(summary_cmd, Value::from_str(AM_SUMMARY).unwrap());
        results.insert(system_info_cmd, Value::from_str(AM_SYSTEM_INFO).unwrap());
        results.insert(devs_cmd, Value::from_str(AM_DEVS).unwrap());
        results.insert(pools_cmd, Value::from_str(AM_POOLS).unwrap());

        let mock_api = MockAPIClient::new(results);

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;

        let miner_data = miner.parse_data(data);

        assert_eq!(miner_data.ip.to_string(), "127.0.0.1".to_owned());
        assert_eq!(
            miner_data.firmware_version.as_deref(),
            Some("FR-1.12(251009-S21)")
        );
        assert_eq!(miner_data.hashboards.len(), 3);
        assert_eq!(miner_data.light_flashing, None);
        assert_eq!(miner_data.fans.len(), 4);
        let last_share_time = miner_data.pools[0].pools[0]
            .last_share_time
            .expect("active pool last share time");
        assert!(
            last_share_time.abs_diff(miner_data.timestamp.saturating_sub(3)) <= 1,
            "expected the reported three-second age to be normalized"
        );
        assert_eq!(miner_data.pools[0].pools[1].last_share_time, None);
        assert_eq!(
            miner_data.expected_hashrate.unwrap(),
            HashRate {
                value: 110.0,
                unit: HashRateUnit::TeraHash,
                algo: HashAlgorithm::SHA256,
            }
        );
        assert_eq!(
            miner_data.hashrate.unwrap(),
            HashRate {
                value: 110.56689,
                unit: HashRateUnit::TeraHash,
                algo: HashAlgorithm::SHA256,
            }
        );
    }

    #[tokio::test]
    #[ignore = "requires live miner; set MINER_IP"]
    async fn parse_data_live_test_auto_detect() -> anyhow::Result<()> {
        let ip_str = std::env::var("MINER_IP").context("MINER_IP is not set")?;
        let ip =
            IpAddr::from_str(&ip_str).with_context(|| format!("invalid MINER_IP: {ip_str}"))?;

        let miner = get_miner(ip, Arc::new(AntMinerStockFirmware::default()))
            .await?
            .context("no miner detected at MINER_IP")?;
        let miner_data = miner.get_data().await;
        let mut miner_data_print = miner_data.clone();
        for hashboard in &mut miner_data_print.hashboards {
            hashboard.chips.clear();
        }
        println!("data {}", serde_json::to_string_pretty(&miner_data_print)?);

        println!(
            "pools {}",
            serde_json::to_string_pretty(&miner.get_pools_config().await?)?
        );

        assert_eq!(miner_data.ip, ip);
        assert!(miner_data.timestamp > 0);
        assert!(!miner_data.schema_version.is_empty());

        Ok(())
    }
}

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
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        firmware::FirmwareImage,
        hashrate::{HashRate, HashRateUnit},
        miner::{MiningMode, TuningTarget},
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_antminer::hardware::AntMinerControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature};
use rpc::AntMinerRPCAPI;
use semver::Version;
use serde_json::{Value, json};
use web::AntMinerWebAPI;

use self::firmware::resolve_firmware_image;
use crate::firmware::AntMinerStockFirmware;

mod firmware;
mod rpc;
pub(crate) mod web;

#[derive(Debug)]
pub struct AntMinerV2018 {
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
fn pools_payload(config: Vec<PoolGroupConfig>) -> anyhow::Result<Vec<Value>> {
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

    if pools.len() > 3 {
        anyhow::bail!("stock Antminer supports at most three pools");
    }
    pools.resize(3, json!({ "url": "", "user": "", "pass": "" }));
    Ok(pools)
}

fn pool_slots_match(expected: &[Value], actual_config: &Value) -> bool {
    let Some(actual) = actual_config["pools"].as_array() else {
        return false;
    };
    expected.iter().enumerate().all(|(index, pool)| {
        ["url", "user", "pass"].into_iter().all(|key| {
            pool[key].as_str().unwrap_or_default()
                == actual
                    .get(index)
                    .and_then(|pool| pool[key].as_str())
                    .unwrap_or_default()
        })
    })
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

impl AntMinerV2018 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        let auth = Self::default_auth();
        let algo = model.hash_algorithm();
        AntMinerV2018 {
            ip,
            rpc: AntMinerRPCAPI::new(ip),
            web: AntMinerWebAPI::new(ip, auth),
            device_info: DeviceInfo::new(model, AntMinerStockFirmware::default(), algo),
        }
    }

    /// Read a numeric field that some firmware generations emit as a JSON
    /// number and others as a quoted string.
    fn parse_f64_field(value: &Value) -> Option<f64> {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
    }

    fn parse_temp_string(temp_str: &str) -> Option<Temperature> {
        let temps: Vec<f64> = temp_str
            .split('-')
            .filter_map(|s| s.parse().ok())
            .filter(|&temp| temp > 0.0)
            .collect();

        if !temps.is_empty() {
            let avg = temps.iter().sum::<f64>() / temps.len() as f64;
            Some(Temperature::from_celsius(avg))
        } else {
            None
        }
    }

    fn _calculate_average_temp_s21_hyd(chain: &Value) -> Option<Temperature> {
        let mut temps = Vec::new();

        if let Some(temp_pic) = chain.get("temp_pic").and_then(|v| v.as_array()) {
            for i in 1..=3 {
                if let Some(temp) = temp_pic.get(i).and_then(|v| v.as_f64())
                    && temp != 0.0
                {
                    temps.push(temp);
                }
            }
        }

        if let Some(temp_pcb) = chain.get("temp_pcb").and_then(|v| v.as_array()) {
            if let Some(temp) = temp_pcb.get(1).and_then(|v| v.as_f64())
                && temp != 0.0
            {
                temps.push(temp);
            }
            if let Some(temp) = temp_pcb.get(3).and_then(|v| v.as_f64())
                && temp != 0.0
            {
                temps.push(temp);
            }
        }

        if !temps.is_empty() {
            let avg = temps.iter().sum::<f64>() / temps.len() as f64;
            Some(Temperature::from_celsius(avg))
        } else {
            None
        }
    }

    fn _calculate_average_temp_pcb(chain: &Value) -> Option<Temperature> {
        if let Some(temp_pcb) = chain.get("temp_pcb").and_then(|v| v.as_array()) {
            let temps: Vec<f64> = temp_pcb
                .iter()
                .filter_map(|v| v.as_f64())
                .filter(|&temp| temp != 0.0)
                .collect();

            if !temps.is_empty() {
                let avg = temps.iter().sum::<f64>() / temps.len() as f64;
                Some(Temperature::from_celsius(avg))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn _calculate_average_temp_chip(chain: &Value) -> Option<Temperature> {
        if let Some(temp_chip) = chain.get("temp_chip").and_then(|v| v.as_array()) {
            let temps: Vec<f64> = temp_chip
                .iter()
                .filter_map(|v| v.as_f64())
                .filter(|&temp| temp != 0.0)
                .collect();

            if !temps.is_empty() {
                let avg = temps.iter().sum::<f64>() / temps.len() as f64;
                Some(Temperature::from_celsius(avg))
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[async_trait]
impl APIClient for AntMinerV2018 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            MinerCommand::WebAPI {
                command: "miner_stats",
                ..
            } => match self.web.get_api_result(command).await {
                Ok(stats) => Ok(stats),
                Err(_) => self.rpc.stats(false).await,
            },
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for {} API",
                AntMinerStockFirmware::default()
            )),
        }
    }
}

impl GetConfigsLocations for AntMinerV2018 {
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

impl CollectConfigs for AntMinerV2018 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for AntMinerV2018 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const RPC_VERSION: MinerCommand = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };

        const WEB_MINER_STATS: MinerCommand = MinerCommand::WebAPI {
            command: "miner_stats",
            parameters: None,
        };

        const RPC_SUMMARY: MinerCommand = MinerCommand::RPC {
            command: "summary",
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

        const WEB_MINER_CONF: MinerCommand = MinerCommand::WebAPI {
            command: "get_miner_conf",
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
            DataField::ControlBoardVersion => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system_kernel_version"),
                    tag: Some("kernel_version"),
                },
            )],
            // cgminer names its summary keys after the unit they are in —
            // `GHS 5s` on SHA-256 models, `MHS 5s` on Scrypt ones — so keep
            // the whole entry and let the parse pick the key that is present.
            DataField::Hashrate => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0"),
                    tag: None,
                },
            )],
            // `total_rateideal`, by contrast, says nothing about its own
            // scale, so carry STATS' `rate_unit` alongside it.
            DataField::ExpectedHashrate => vec![
                (
                    WEB_MINER_STATS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/STATS/1/total_rateideal"),
                        tag: Some("hashrate"),
                    },
                ),
                (
                    WEB_MINER_STATS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/STATS/1/rate_unit"),
                        tag: Some("unit"),
                    },
                ),
            ],
            DataField::Fans => vec![(
                WEB_MINER_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/1"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![(
                WEB_MINER_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/1"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![],
            DataField::IsMining => vec![
                (
                    WEB_MINER_CONF,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/bitmain-work-mode"),
                        tag: Some("mode"),
                    },
                ),
                (
                    RPC_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/SUMMARY/0"),
                        tag: Some("summary"),
                    },
                ),
            ],
            DataField::Uptime => vec![(
                WEB_MINER_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/1/Elapsed"),
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
            DataField::Wattage => vec![(
                WEB_MINER_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/1"),
                    tag: None,
                },
            )],
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

impl GetIP for AntMinerV2018 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for AntMinerV2018 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for AntMinerV2018 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for AntMinerV2018 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetHostname for AntMinerV2018 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for AntMinerV2018 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for AntMinerV2018 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetHashboards for AntMinerV2018 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> =
            (0..self.device_info.hardware.board_count().unwrap_or(0))
                .map(|idx| {
                    BoardData::new(idx, self.device_info.hardware.chips_for_board(idx as usize))
                })
                .collect();

        let Some(stats_data) = data.get(&DataField::Hashboards).and_then(|v| v.as_object()) else {
            return hashboards;
        };

        let algo = self.device_info.algo;
        // These Equihash builds report chain rates in KSol/s without a
        // rate_unit field, even though their summary calls Sol/s "GHS".
        let unit = if algo == HashAlgorithm::Equihash {
            HashRateUnit::KiloSol
        } else {
            stats_data
                .get("rate_unit")
                .and_then(|v| v.as_str())
                .and_then(|s| HashRateUnit::from_str(s).ok())
                .unwrap_or(HashRateUnit::GigaHash)
        };

        for board in hashboards.iter_mut() {
            let idx = board.position + 1;

            board.hashrate = stats_data
                .get(&format!("chain_rate{idx}"))
                .and_then(Self::parse_f64_field)
                .map(|r| {
                    HashRate {
                        value: r,
                        unit,
                        algo,
                    }
                    .as_default_unit()
                });

            // `temp_pcb{idx}` on older generations; newer ones split the
            // reading into inlet/outlet pairs and omit the combined key.
            board.board_temperature = stats_data
                .get(&format!("temp_pcb{idx}"))
                .and_then(|v| v.as_str())
                .and_then(Self::parse_temp_string)
                .or_else(|| {
                    stats_data
                        .get(&format!("temp_out_pcb_{idx}"))
                        .and_then(Self::parse_f64_field)
                        .filter(|&t| t > 0.0)
                        .map(Temperature::from_celsius)
                })
                .or_else(|| {
                    stats_data
                        .get(&format!("temp{idx}"))
                        .and_then(Self::parse_f64_field)
                        .filter(|&t| t > 0.0)
                        .map(Temperature::from_celsius)
                });

            board.outlet_chip_temperature = stats_data
                .get(&format!("temp2_{idx}"))
                .and_then(Self::parse_f64_field)
                .filter(|&t| t > 0.0)
                .map(Temperature::from_celsius);

            board.working_chips = stats_data
                .get(&format!("chain_acn{idx}"))
                .and_then(|v| v.as_u64())
                .map(|u| u as u16);

            board.frequency = stats_data
                .get(&format!("freq{idx}"))
                .and_then(|v| v.as_u64())
                .map(|f| Frequency::from_megahertz(f as f64));

            let has_hashrate = board
                .hashrate
                .as_ref()
                .map(|h| h.value > 0.0)
                .unwrap_or(false);
            let has_chips = board.working_chips.map(|chips| chips > 0).unwrap_or(false);

            board.active = Some(has_hashrate || has_chips);
        }

        hashboards
    }
}

impl GetHashrate for AntMinerV2018 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        let summary = data.get(&DataField::Hashrate)?.as_object()?;

        // cgminer names its rate keys `<unit> <window>` — `GHS 5s` on SHA-256
        // models, `MHS 5s` on Scrypt ones — so the key states its own unit.
        // Prefer the 5-second window and fall back to the average.
        let (value, mut unit) = ["5s", "av"].into_iter().find_map(|window| {
            summary.iter().find_map(|(key, raw)| {
                let (unit, key_window) = key.split_once(' ')?;
                if key_window != window {
                    return None;
                }
                Some((
                    Self::parse_f64_field(raw)?,
                    HashRateUnit::from_str(unit).ok()?,
                ))
            })
        })?;

        // On the observed Z11j/Z15/Z15j stock builds, "GHS" contains Sol/s.
        if self.device_info.algo == HashAlgorithm::Equihash && unit == HashRateUnit::GigaHash {
            unit = HashRateUnit::Hash;
        }

        Some(
            HashRate {
                value,
                unit,
                algo: self.device_info.algo,
            }
            .as_default_unit(),
        )
    }
}

impl GetExpectedHashrate for AntMinerV2018 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        let field = data.get(&DataField::ExpectedHashrate);
        let value = field?
            .pointer("/hashrate")
            .and_then(Self::parse_f64_field)?;
        Some(
            HashRate {
                value,
                // `total_rateideal` does not name its own scale, so take it
                // from STATS' `rate_unit` (`"GH"` / `"MH"`).
                unit: if self.device_info.algo == HashAlgorithm::Equihash {
                    HashRateUnit::Hash
                } else {
                    field
                        .and_then(|v| v.pointer("/unit"))
                        .and_then(|v| v.as_str())
                        .and_then(|s| HashRateUnit::from_str(s).ok())
                        .unwrap_or(HashRateUnit::GigaHash)
                },
                algo: self.device_info.algo,
            }
            .as_default_unit(),
        )
    }
}

impl GetFans for AntMinerV2018 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let Some(stats_data) = data.get(&DataField::Fans).and_then(Value::as_object) else {
            return Vec::new();
        };
        // The old control board can populate fan3/fan4 while fan1/fan2 are
        // zero, even on models with two physical fans.
        let mut fans: Vec<FanData> = stats_data
            .iter()
            .filter_map(|(key, value)| {
                let index = key.strip_prefix("fan")?.parse::<i16>().ok()?;
                let rpm = Self::parse_f64_field(value).filter(|&rpm| rpm > 0.0)?;
                Some(FanData {
                    position: index - 1,
                    rpm: Some(AngularVelocity::from_rpm(rpm)),
                })
            })
            .collect();
        fans.sort_by_key(|fan| fan.position);
        fans
    }
}

impl GetLightFlashing for AntMinerV2018 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing).or_else(|| {
            data.extract::<String>(DataField::LightFlashing)
                .map(|s| s.to_lowercase() == "true" || s == "1")
        })
    }
}

impl GetUptime for AntMinerV2018 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetBestShare for AntMinerV2018 {}
impl GetSessionBestShare for AntMinerV2018 {}

impl GetOperatingState for AntMinerV2018 {}

impl GetDevFeeConnected for AntMinerV2018 {}

impl GetIsMining for AntMinerV2018 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        let Some(status) = data.get(&DataField::IsMining) else {
            return false;
        };
        let mode = status
            .get("mode")
            .and_then(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| value.as_i64().map(|value| value.to_string()))
            })
            .map(|value| value.to_ascii_lowercase());
        if matches!(mode.as_deref(), Some("stopped" | "idle" | "sleep" | "1")) {
            return false;
        }

        status
            .get("summary")
            .and_then(Value::as_object)
            .and_then(|summary| {
                summary.iter().find_map(|(key, value)| {
                    let (unit, window) = key.split_once(' ')?;
                    if window == "5s" && HashRateUnit::from_str(unit).is_ok() {
                        Self::parse_f64_field(value)
                    } else {
                        None
                    }
                })
            })
            .is_some_and(|rate| rate > 0.0)
    }
}

impl GetPools for AntMinerV2018 {
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

impl GetSerialNumber for AntMinerV2018 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::SerialNumber)
    }
}

impl GetControlBoardVersion for AntMinerV2018 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        let kernel_version =
            data.extract_nested::<String>(DataField::ControlBoardVersion, "kernel_version")?;
        kernel_version
            .to_ascii_lowercase()
            .contains("-xilinx")
            .then_some(AntMinerControlBoard::Xilinx)
            .map(Into::into)
    }
}

impl GetWattage for AntMinerV2018 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        if let Some(stats_data) = data.get(&DataField::Wattage) {
            if let Some(chain_power) = stats_data.get("chain_power")
                && let Some(power_str) = chain_power.as_str()
            {
                // Parse "3250 W" format
                if let Some(watt_part) = power_str.split_whitespace().next()
                    && let Ok(watts) = watt_part.parse::<f64>()
                {
                    return Some(Power::from_watts(watts));
                }
            }

            if let Some(power) = stats_data
                .get("power")
                .or_else(|| stats_data.get("Power"))
                // Same firmware version spells this differently per model: the
                // L9 reports `power`, the L11 `watt`.
                .or_else(|| stats_data.get("watt"))
                .and_then(|v| v.as_f64())
            {
                return Some(Power::from_watts(power));
            }
        }
        None
    }
}

impl GetTuningTarget for AntMinerV2018 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        data.get(&DataField::TuningTarget)
            .and_then(miner_conf_mining_mode)
            .map(TuningTarget::MiningMode)
    }
}

impl GetScaledTuningTarget for AntMinerV2018 {
    fn parse_scaled_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        self.parse_tuning_target(data)
    }
}

impl GetFluidTemperature for AntMinerV2018 {
    fn parse_fluid_temperature(&self, data: &HashMap<DataField, Value>) -> Option<Temperature> {
        // For S21+ Hyd models, use inlet/outlet temperature average
        if self.device_info.model.to_string().contains("S21+ Hyd")
            && let Some(hashboards_data) = data.get(&DataField::Hashboards)
            && let Some(chains) = hashboards_data.as_array()
        {
            let mut temps = Vec::new();

            for chain in chains {
                if let Some(temp_pcb) = chain.get("temp_pcb").and_then(|v| v.as_array()) {
                    // Inlet temp (index 0) and outlet temp (index 2)
                    if let Some(inlet) = temp_pcb.first().and_then(|v| v.as_f64())
                        && inlet != 0.0
                    {
                        temps.push(inlet);
                    }
                    if let Some(outlet) = temp_pcb.get(2).and_then(|v| v.as_f64())
                        && outlet != 0.0
                    {
                        temps.push(outlet);
                    }
                }
            }

            if !temps.is_empty() {
                let avg = temps.iter().sum::<f64>() / temps.len() as f64;
                return Some(Temperature::from_celsius(avg));
            }
        }
        None
    }
}

impl GetPsuFans for AntMinerV2018 {}
impl GetPsus for AntMinerV2018 {}
impl GetTuningCapabilities for AntMinerV2018 {}
impl SupportsTimezoneConfig for AntMinerV2018 {}

impl GetMessages for AntMinerV2018 {}

#[async_trait]
impl SetFaultLight for AntMinerV2018 {
    fn supports_set_fault_light(&self) -> bool {
        true
    }

    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        Ok(super::blink_response_matches(
            &self.web.blink(fault).await?,
            fault,
        ))
    }
}

#[async_trait]
impl SetPowerLimit for AntMinerV2018 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsPoolsConfig for AntMinerV2018 {
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
        let pools = pools_payload(config)?;
        self.web.set_pools_form(&pools).await?;

        // The setter can answer while mining is restarting. Confirm the
        // persisted slots instead of treating an HTTP success as a good write.
        for _ in 0..5 {
            if let Ok(actual) = self.web.get_miner_conf().await
                && pool_slots_match(&pools, &actual)
            {
                return Ok(true);
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        anyhow::bail!("stock miner did not report the requested pool URLs, workers, and passwords")
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for AntMinerV2018 {
    fn supports_restart(&self) -> bool {
        true
    }
    async fn restart(&self) -> anyhow::Result<bool> {
        Ok(self.web.reboot().await.is_ok())
    }
}

#[async_trait]
impl Pause for AntMinerV2018 {
    fn supports_pause(&self) -> bool {
        false
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
impl Resume for AntMinerV2018 {
    fn supports_resume(&self) -> bool {
        false
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
impl ChangePassword for AntMinerV2018 {
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
impl ReadLogs for AntMinerV2018 {
    async fn read_logs(&self) -> anyhow::Result<String> {
        anyhow::bail!("legacy Antminer firmware does not expose log.cgi")
    }

    fn supports_read_logs(&self) -> bool {
        false
    }
}

#[async_trait]
impl FactoryReset for AntMinerV2018 {
    async fn factory_reset(&self) -> anyhow::Result<bool> {
        self.web.factory_reset().await
    }

    fn supports_factory_reset(&self) -> bool {
        true
    }
}

impl RestoreStockOs for AntMinerV2018 {}

#[async_trait]
impl SupportsScalingConfig for AntMinerV2018 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for AntMinerV2018 {
    async fn prepare_firmware(&self, image: FirmwareImage) -> anyhow::Result<FirmwareImage> {
        if !firmware::is_bmu(&image) {
            return Ok(image);
        }
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

impl HasDefaultAuth for AntMinerV2018 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("root", "root")
    }
}

impl HasAuth for AntMinerV2018 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

impl Validate for AntMinerV2018 {
    type Firmware = AntMinerStockFirmware;

    fn validate(version: Option<&semver::Version>) -> bool {
        version.is_some_and(|v| *v < Version::new(2021, 1, 1))
    }
}

#[async_trait]
impl SupportsTuningConfig for AntMinerV2018 {
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
        false
    }
}

#[async_trait]
impl SupportsFanConfig for AntMinerV2018 {
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
        false
    }
}

impl SupportsTemperatureConfig for AntMinerV2018 {}
impl GetTuningPercent for AntMinerV2018 {}
impl SetHashboardsEnabled for AntMinerV2018 {}

impl SetTuningPercent for AntMinerV2018 {}

impl SupportsPresets for AntMinerV2018 {}

#[cfg(test)]
mod tests {
    use asic_rs_core::data::device::HashAlgorithm;
    use std::sync::Arc;

    use anyhow::{self, Context};
    use asic_rs_core::test::{api::MockAPIClient, util::get_miner};
    use asic_rs_makes_antminer::models::AntMinerModel;

    use super::*;
    use crate::test::json::v2020::{
        AM_DEVS, AM_POOLS, AM_STATS, AM_SUMMARY, AM_SYSTEM_INFO, AM_VERSION,
    };

    #[test]
    fn legacy_mode_controls_are_not_advertised() {
        let miner = AntMinerV2018::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::Z15);
        assert!(!miner.supports_pause());
        assert!(!miner.supports_resume());
        assert!(!miner.supports_tuning_config());
    }

    #[tokio::test]
    async fn legacy_logs_are_not_advertised_or_requested() {
        let miner = AntMinerV2018::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::Z15);
        assert!(!miner.supports_read_logs());
        assert!(
            miner
                .read_logs()
                .await
                .unwrap_err()
                .to_string()
                .contains("log.cgi")
        );
    }

    #[test]
    fn legacy_polls_skip_missing_blink_status_endpoint() {
        let miner = AntMinerV2018::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::Z15);
        assert!(miner.get_locations(DataField::LightFlashing).is_empty());
    }

    #[tokio::test]
    async fn legacy_mining_status_uses_current_rate_when_mode_is_missing() {
        let miner = AntMinerV2018::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::Z15);
        for (rate, config, expected) in [
            (100.0, json!({"pools": []}), true),
            (0.0, json!({"pools": []}), false),
            (100.0, json!({"bitmain-work-mode": "sleep"}), false),
        ] {
            let results = HashMap::from([
                (
                    MinerCommand::WebAPI {
                        command: "get_miner_conf",
                        parameters: None,
                    },
                    config,
                ),
                (
                    MinerCommand::RPC {
                        command: "summary",
                        parameters: None,
                    },
                    json!({"SUMMARY": [{"GHS 5s": rate, "GHS av": 100.0}]}),
                ),
            ]);
            let mock_api = MockAPIClient::new(results);
            let mut collector = DataCollector::new_with_client(&miner, &mock_api);
            let data = collector.collect(&[DataField::IsMining]).await;
            assert_eq!(miner.parse_is_mining(&data), expected, "rate: {rate}");
        }
        assert!(!miner.parse_is_mining(&HashMap::new()));
    }

    #[tokio::test]
    async fn control_board_uses_system_info_without_querying_miner_type() {
        let miner = AntMinerV2018::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::Z15);

        for (kernel, expected) in [
            (
                "Linux 4.6.0-xilinx-g0387054-dirty #45 SMP PREEMPT",
                Some("Xilinx"),
            ),
            ("Linux 4.9.113 #1 SMP PREEMPT", None),
        ] {
            let mut results = HashMap::new();
            results.insert(
                MinerCommand::WebAPI {
                    command: "get_system_info",
                    parameters: None,
                },
                json!({"system_kernel_version": kernel}),
            );
            let mock_api = MockAPIClient::new(results);
            let mut collector = DataCollector::new_with_client(&miner, &mock_api);
            let report = collector
                .collect_report(&[DataField::ControlBoardVersion])
                .await;
            let board = miner.parse_control_board_version(&report.values);

            assert_eq!(board.as_ref().map(|board| board.name.as_str()), expected);
            assert_eq!(report.diagnostics.failed_commands, 0);
        }
    }

    #[tokio::test]
    async fn legacy_wattage_uses_http_stats_without_tcp_stats_request() {
        let miner = AntMinerV2018::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::Z15);
        let results = HashMap::from([(
            MinerCommand::WebAPI {
                command: "miner_stats",
                parameters: None,
            },
            json!({"STATS": [{}, {"chain_power": "3250 W"}]}),
        )]);
        let mock_api = MockAPIClient::new(results);
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let report = collector.collect_report(&[DataField::Wattage]).await;

        assert_eq!(report.diagnostics.failed_commands, 0);
        assert_eq!(
            miner
                .parse_wattage(&report.values)
                .map(|power| power.as_watts()),
            Some(3250.0)
        );
    }

    #[tokio::test]
    async fn legacy_http_stats_supply_temperatures_fans_and_uptime() {
        let miner = AntMinerV2018::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::Z11j);
        let mut results = HashMap::new();
        results.insert(
            MinerCommand::WebAPI {
                command: "miner_stats",
                parameters: None,
            },
            json!({"STATS": [{}, {
                "Elapsed": 3274,
                "chain_rate1": "22.94",
                "chain_acn1": 2,
                "temp1": 49,
                "temp2_1": 71,
                "fan1": 0,
                "fan2": 0,
                "fan3": 3840,
                "fan4": 2880,
                "total_rateideal": 320000.0
            }]}),
        );
        results.insert(
            MinerCommand::RPC {
                command: "summary",
                parameters: None,
            },
            json!({"SUMMARY": [{"GHS 5s": "92399.72"}]}),
        );

        let mock_api = MockAPIClient::new(results);
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector
            .collect(&[
                DataField::Hashboards,
                DataField::Fans,
                DataField::Uptime,
                DataField::Hashrate,
                DataField::ExpectedHashrate,
            ])
            .await;

        let boards = miner.parse_hashboards(&data);
        assert_eq!(boards[0].board_temperature.unwrap().as_celsius(), 49.0);
        assert_eq!(
            boards[0].outlet_chip_temperature.unwrap().as_celsius(),
            71.0
        );
        assert_eq!(
            boards[0].hashrate.as_ref().unwrap().unit,
            HashRateUnit::KiloSol
        );
        assert!((boards[0].hashrate.as_ref().unwrap().value - 22.94).abs() < 0.001);
        assert_eq!(miner.parse_fans(&data).len(), 2);
        assert_eq!(miner.parse_uptime(&data), Some(Duration::from_secs(3274)));

        let hashrate = miner.parse_hashrate(&data).unwrap();
        assert_eq!(hashrate.unit, HashRateUnit::KiloSol);
        assert!((hashrate.value - 92.39972).abs() < 0.001);
        let expected = miner.parse_expected_hashrate(&data).unwrap();
        assert_eq!(expected.unit, HashRateUnit::KiloSol);
        assert!((expected.value - 320.0).abs() < 0.001);
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
            }])
            .unwrap(),
            vec![
                json!({"url": "stratum+tcp://pool1.example:3333", "user": "worker.1", "pass": "x"}),
                json!({"url": "stratum+tcp://pool2.example:3333", "user": "worker.2", "pass": "x"}),
                json!({"url": "", "user": "", "pass": ""}),
            ]
        );
    }

    #[test]
    fn pools_payload_rejects_more_than_three_pools_across_groups() {
        let pool = || PoolConfig {
            url: PoolURL::from("stratum+tcp://pool.example:3333".to_string()),
            username: "worker".to_string(),
            password: "x".to_string(),
        };
        let groups = (0..2)
            .map(|_| PoolGroupConfig {
                name: String::new(),
                quota: 1,
                pools: vec![pool(), pool()],
            })
            .collect();
        assert!(
            pools_payload(groups)
                .unwrap_err()
                .to_string()
                .contains("at most three pools")
        );
    }

    #[test]
    fn saved_pool_password_must_match() {
        let expected = vec![json!({"url": "pool.example:3333", "user": "worker", "pass": "new"})];
        let actual =
            json!({"pools": [{"url": "pool.example:3333", "user": "worker", "pass": "old"}]});

        assert!(!pool_slots_match(&expected, &actual));
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
        let miner = AntMinerV2018::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::L9);

        let mut results = HashMap::new();
        results.insert(
            MinerCommand::RPC {
                command: "summary",
                parameters: None,
            },
            json!({ "SUMMARY": [{ "MHS 5s": 16200.0, "MHS av": 16150.0 }] }),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "miner_stats",
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
        let miner = AntMinerV2018::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19Pro);

        let mut results = HashMap::new();

        let stats_cmd = MinerCommand::WebAPI {
            command: "miner_stats",
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

    struct MockHttpServer {
        port: u16,
        task: tokio::task::JoinHandle<anyhow::Result<Vec<String>>>,
    }

    /// Answers each request in turn with the next `(status, reason, body)` and
    /// hands back the requests it saw.
    async fn mock_http_server(
        responses: Vec<(u16, &'static str, &'static str)>,
    ) -> anyhow::Result<MockHttpServer> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, reason, body) in responses {
                let (mut socket, _) =
                    tokio::time::timeout(Duration::from_secs(2), listener.accept())
                        .await
                        .context("timed out waiting for HTTP request")??;

                let mut request = Vec::new();
                let mut chunk = [0_u8; 1024];
                loop {
                    if let Some(header_end) = request
                        .windows(4)
                        .position(|window| window == b"\r\n\r\n")
                        .map(|position| position + 4)
                    {
                        let headers = std::str::from_utf8(&request[..header_end])?;
                        let content_length = headers
                            .lines()
                            .filter_map(|line| line.split_once(':'))
                            .find_map(|(name, value)| {
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or_default();
                        if request.len() >= header_end + content_length {
                            break;
                        }
                    }
                    let bytes_read = socket.read(&mut chunk).await?;
                    if bytes_read == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..bytes_read]);
                }
                requests.push(String::from_utf8(request)?);

                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await?;
            }
            Ok(requests)
        });

        Ok(MockHttpServer { port, task })
    }

    fn miner_with_mock_web(port: u16) -> AntMinerV2018 {
        let ip = IpAddr::from([127, 0, 0, 1]);
        let mut miner = AntMinerV2018::new(ip, AntMinerModel::S19);
        miner.web = AntMinerWebAPI::new_with_port(ip, port, MinerAuth::new("root", "root"));
        miner
    }

    #[tokio::test]
    async fn raw_firmware_preparation_does_not_query_the_miner() -> anyhow::Result<()> {
        let miner = miner_with_mock_web(0);
        let image = FirmwareImage::new("stock.bin".to_string(), vec![1, 2, 3, 4]);

        assert_eq!(miner.prepare_firmware(image.clone()).await?, image);
        Ok(())
    }

    #[tokio::test]
    async fn bmu_preparation_uses_system_info_when_miner_type_is_missing() -> anyhow::Result<()> {
        let MockHttpServer { port, task } = mock_http_server(vec![
            (404, "Not Found", ""),
            (200, "OK", r#"{"minertype":"Antminer S19"}"#),
        ])
        .await?;
        let miner = miner_with_mock_web(port);
        let image = FirmwareImage::new(
            "bundle.bmu".to_string(),
            0xABABABAB_u32.to_le_bytes().to_vec(),
        );

        let error = miner.prepare_firmware(image).await.unwrap_err();
        assert!(error.to_string().contains("BMU header is truncated"));

        let requests = task.await??;
        assert!(requests[0].starts_with("GET /cgi-bin/miner_type.cgi HTTP/1.1\r\n"));
        assert!(requests[1].starts_with("GET /cgi-bin/get_system_info.cgi HTTP/1.1\r\n"));
        Ok(())
    }

    #[tokio::test]
    async fn set_fault_light_reports_why_the_miner_refused() -> anyhow::Result<()> {
        let MockHttpServer { port, task } = mock_http_server(vec![(
            500,
            "Internal Server Error",
            "blink.cgi: cannot reach bmminer",
        )])
        .await?;
        let miner = miner_with_mock_web(port);

        let error = miner.set_fault_light(true).await.unwrap_err();

        let requests = task.await??;
        let request = requests.last().unwrap();
        assert!(request.starts_with("POST /cgi-bin/blink.cgi HTTP/1.1\r\n"));
        let error = error.to_string();
        assert!(error.contains("500"), "{error}");
        assert!(error.contains("blink.cgi: cannot reach bmminer"), "{error}");

        Ok(())
    }

    #[tokio::test]
    async fn set_fault_light_still_returns_true_when_accepted() -> anyhow::Result<()> {
        let MockHttpServer { port, task } =
            mock_http_server(vec![(200, "OK", r#"{"code":"B000"}"#)]).await?;
        let miner = miner_with_mock_web(port);

        assert!(miner.set_fault_light(true).await?);

        let requests = task.await??;
        let request = requests.last().unwrap();
        assert!(request.starts_with("POST /cgi-bin/blink.cgi HTTP/1.1\r\n"));

        Ok(())
    }

    #[tokio::test]
    async fn set_fault_light_reports_false_when_blink_state_does_not_match() -> anyhow::Result<()> {
        let MockHttpServer { port, task } =
            mock_http_server(vec![(200, "OK", r#"{"code":"B100"}"#)]).await?;
        let miner = miner_with_mock_web(port);

        assert!(!miner.set_fault_light(true).await?);
        task.await??;
        Ok(())
    }

    #[tokio::test]
    async fn set_pools_config_reports_why_the_miner_refused() -> anyhow::Result<()> {
        let MockHttpServer { port, task } = mock_http_server(vec![(
            500,
            "Internal Server Error",
            "set_miner_conf.cgi: config is locked",
        )])
        .await?;
        let miner = miner_with_mock_web(port);

        let error = miner.set_pools_config(vec![]).await.unwrap_err();

        let requests = task.await??;
        let request = requests.last().unwrap();
        assert!(request.starts_with("POST /cgi-bin/set_miner_conf.cgi HTTP/1.1\r\n"));
        assert!(request.contains("application/x-www-form-urlencoded"));
        let error = error.to_string();
        assert!(error.contains("500"), "{error}");
        assert!(
            error.contains("set_miner_conf.cgi: config is locked"),
            "{error}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn legacy_form_repairs_a_malformed_pool_config() -> anyhow::Result<()> {
        let MockHttpServer { port, task } = mock_http_server(vec![
            (200, "OK", "<html>miner restarting</html>"),
            (
                200,
                "OK",
                r#"{"pools":[{"url":"stratum+tcp://pool.example:3333","user":"worker","pass":"x"},{"url":"","user":"","pass":""},{"url":"","user":"","pass":""}]}"#,
            ),
        ])
        .await?;
        let miner = miner_with_mock_web(port);
        let pools = vec![PoolGroupConfig {
            name: "default".to_string(),
            quota: 1,
            pools: vec![PoolConfig {
                url: PoolURL::from("stratum+tcp://pool.example:3333".to_string()),
                username: "worker".to_string(),
                password: "x".to_string(),
            }],
        }];

        assert!(miner.set_pools_config(pools).await?);

        let requests = task.await??;
        assert!(requests[0].starts_with("POST /cgi-bin/set_miner_conf.cgi "));
        assert!(requests[0].contains("application/x-www-form-urlencoded"));
        assert!(requests[0].contains("stratum%2Btcp%3A%2F%2Fpool.example%3A3333"));

        Ok(())
    }
}

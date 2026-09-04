use std::{collections::HashMap, net::IpAddr, str::FromStr, time::Duration};

use anyhow;
use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigExtractor, ConfigField, ConfigLocation},
        fan::FanConfig,
        pools::{PoolConfig, PoolGroupConfig},
        scaling::ScalingConfig,
        tuning::TuningConfig,
    },
    data::{
        board::{BoardData, ChipData, MinerControlBoard},
        collector::{
            DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
        },
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        firmware::FirmwareImage,
        hashrate::{HashRate, HashRateUnit},
        message::{MessageSeverity, MinerMessage},
        miner::TuningTarget,
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
    util::unix_timestamp_secs,
};
use asic_rs_makes_antminer::hardware::AntMinerControlBoard;
use asic_rs_makes_epic::hardware::EPicControlBoard;
use asic_rs_makes_volcminer::{hardware::VolcMinerControlBoard, make::VolcMinerMake};
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use reqwest::Method;
use serde_json::{Value, json};
use web::PowerPlayWebAPI;

use crate::firmware::EPicFirmware;

mod web;

#[derive(Debug)]
pub struct PowerPlayV1 {
    ip: IpAddr,
    web: PowerPlayWebAPI,
    device_info: DeviceInfo,
}

impl PowerPlayV1 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        let auth = Self::default_auth();
        let hash_algorithm = model.hash_algorithm();
        PowerPlayV1 {
            ip,
            web: PowerPlayWebAPI::new(ip, 4028, auth),
            device_info: DeviceInfo::new(model, EPicFirmware::default(), hash_algorithm),
        }
    }

    fn coin_type_for_hash_algorithm(hash_algorithm: HashAlgorithm) -> &'static str {
        match hash_algorithm {
            HashAlgorithm::Scrypt => "LTC",
            _ => "BTC",
        }
    }

    // this gets used twice
    fn parse_stratum_configs(configs: &Value) -> Vec<PoolConfig> {
        let mut pools: Vec<PoolConfig> = configs
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|pool| {
                let url = pool.get("pool").and_then(Value::as_str)?;
                if url.is_empty() {
                    return None;
                }

                Some(PoolConfig {
                    url: PoolURL::from(url.to_string()),
                    username: pool
                        .get("login")
                        .and_then(Value::as_str)
                        .map(String::from)
                        .unwrap_or_default(),
                    password: pool
                        .get("password")
                        .and_then(Value::as_str)
                        .map(String::from)
                        .unwrap_or_default(),
                })
            })
            .collect();

        pools.truncate(3);

        pools
    }

    fn parse_pool_url(value: &Value) -> Option<PoolURL> {
        value.as_str().and_then(|s| {
            if s.is_empty() {
                None
            } else {
                Some(PoolURL::from(s.to_string()))
            }
        })
    }

    fn parse_pool_user(value: &Value) -> Option<String> {
        value.as_str().map(String::from)
    }

    fn pool_data_from_config(position: Option<u16>, config: &Value) -> PoolData {
        PoolData {
            position,
            url: config.get("pool").and_then(Self::parse_pool_url),
            accepted_shares: None,
            rejected_shares: None,
            active: Some(false),
            alive: None,
            user: config.get("login").and_then(Self::parse_pool_user),
        }
    }

    fn apply_pool_status(
        pool: &mut PoolData,
        status: &Value,
        pool_key: &str,
        user_key: &str,
        accepted_key: &str,
        rejected_key: &str,
        alive_key: &str,
    ) {
        pool.active = Some(true);
        pool.alive = status.get(alive_key).and_then(Value::as_bool);
        pool.user = status.get(user_key).and_then(Self::parse_pool_user);
        pool.url = status.get(pool_key).and_then(Self::parse_pool_url);
        pool.accepted_shares = status.get(accepted_key).and_then(Value::as_u64);
        pool.rejected_shares = status.get(rejected_key).and_then(Value::as_u64);
    }

    fn parse_hashrate_split_pools(
        pools_object: &serde_json::Map<String, Value>,
    ) -> Vec<PoolGroupData> {
        let statuses = pools_object
            .get("Hashratesplit Status")
            .and_then(Value::as_array);

        let mut groups: Vec<PoolGroupData> = pools_object
            .get("Hashratesplit Config")
            .and_then(|v| v.get("hashrate_splits"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(group_idx, split)| {
                let active_index = split
                    .get("sc_index")
                    .and_then(Value::as_u64)
                    .and_then(|idx| u16::try_from(idx).ok());
                let status = statuses.and_then(|statuses| statuses.get(group_idx));
                let name = format!("group{}", group_idx + 1);
                let quota = split
                    .get("ratio")
                    .and_then(Value::as_u64)
                    .and_then(|ratio| u32::try_from(ratio).ok())
                    .unwrap_or(1);
                let pools = split
                    .get("stratum_configs")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .map(|(pool_idx, config)| {
                        let position = u16::try_from(pool_idx).ok();
                        let active = position == active_index;

                        let mut pool = Self::pool_data_from_config(position, config);
                        pool.active = Some(active);

                        if active && let Some(status) = status {
                            Self::apply_pool_status(
                                &mut pool,
                                status,
                                "pool",
                                "login",
                                "shares_accepted",
                                "shares_rejected",
                                "is_stratum_connected",
                            );

                            pool.url = pool
                                .url
                                .or_else(|| config.get("pool").and_then(Self::parse_pool_url));
                            pool.user = pool
                                .user
                                .or_else(|| config.get("login").and_then(Self::parse_pool_user));
                        }

                        pool
                    })
                    .collect();

                PoolGroupData { name, quota, pools }
            })
            .collect();

        groups.truncate(3);
        groups
    }

    fn parse_standard_pools(pools_object: &serde_json::Map<String, Value>) -> Vec<PoolGroupData> {
        let mut pools_vec: Vec<PoolData> = Vec::new();

        if let Some(configs) = pools_object
            .get("StratumConfigs")
            .and_then(|v| v.as_array())
        {
            for (idx, config) in configs.iter().enumerate() {
                pools_vec.push(Self::pool_data_from_config(Some(idx as u16), config));
            }
        }

        if let Some(stratum) = pools_object.get("Stratum") {
            for pool in pools_vec.iter_mut() {
                if pool.position
                    == stratum
                        .get("Config Id")
                        .and_then(|v| v.as_u64().map(|v| v as u16))
                {
                    Self::apply_pool_status(
                        pool,
                        stratum,
                        "Current Pool",
                        "Current User",
                        "Accepted",
                        "Rejected",
                        "IsPoolConnected",
                    );

                    if let Some(session) = pools_object.get("Session").and_then(|v| v.as_object()) {
                        pool.accepted_shares = session.get("Accepted").and_then(|v| v.as_u64());
                        pool.rejected_shares = session.get("Rejected").and_then(|v| v.as_u64());
                    }
                }
            }
        }

        vec![PoolGroupData {
            name: String::new(),
            quota: 1,
            pools: pools_vec,
        }]
    }

    fn to_stratum_configs(group: &PoolGroupConfig) -> Vec<Value> {
        group
            .pools
            .iter()
            .map(|pool| {
                json!({
                    "pool": pool.url.to_string(),
                    "login": pool.username.as_str(),
                    "password": pool.password.as_str(),
                })
            })
            .collect()
    }
}

#[async_trait]
impl APIClient for PowerPlayV1 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for ePIC PowerPlay API"
            )),
        }
    }
}

impl GetConfigsLocations for PowerPlayV1 {
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        const WEB_SUMMARY: MinerCommand = MinerCommand::WebAPI {
            command: "summary",
            parameters: None,
        };
        const WEB_HASHRATESPLIT_CONFIG: MinerCommand = MinerCommand::WebAPI {
            command: "hashratesplit/config",
            parameters: None,
        };
        match data_field {
            ConfigField::Fan => vec![(
                WEB_SUMMARY,
                ConfigExtractor {
                    func: get_by_pointer,
                    key: Some("/Fans/Fan Mode"),
                    tag: None,
                },
            )],
            ConfigField::Pools => vec![
                (
                    WEB_SUMMARY,
                    ConfigExtractor {
                        func: get_by_pointer,
                        key: Some("/StratumConfigs"),
                        tag: Some("summary"),
                    },
                ),
                (
                    WEB_HASHRATESPLIT_CONFIG,
                    ConfigExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("Hashratesplit Config"),
                    },
                ),
            ],
            ConfigField::Scaling => vec![(
                WEB_SUMMARY,
                ConfigExtractor {
                    func: get_by_pointer,
                    key: Some("/PerpetualTune/Algorithm"),
                    tag: None,
                },
            )],
            ConfigField::Tuning => vec![(
                WEB_SUMMARY,
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

impl CollectConfigs for PowerPlayV1 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for PowerPlayV1 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const WEB_SUMMARY: MinerCommand = MinerCommand::WebAPI {
            command: "summary",
            parameters: None,
        };
        const WEB_NETWORK: MinerCommand = MinerCommand::WebAPI {
            command: "network",
            parameters: None,
        };
        const WEB_CAPABILITIES: MinerCommand = MinerCommand::WebAPI {
            command: "capabilities",
            parameters: None,
        };
        const WEB_CHIP_TEMPS: MinerCommand = MinerCommand::WebAPI {
            command: "temps/chip",
            parameters: None,
        };
        const WEB_CHIP_VOLTAGES: MinerCommand = MinerCommand::WebAPI {
            command: "voltages",
            parameters: None,
        };
        const WEB_CHIP_HASHRATES: MinerCommand = MinerCommand::WebAPI {
            command: "hashrate",
            parameters: None,
        };
        const WEB_CHIP_CLOCKS: MinerCommand = MinerCommand::WebAPI {
            command: "clocks",
            parameters: None,
        };
        const WEB_TEMPS: MinerCommand = MinerCommand::WebAPI {
            command: "temps",
            parameters: None,
        };
        const WEB_HASHRATESPLIT_CONFIG: MinerCommand = MinerCommand::WebAPI {
            command: "hashratesplit/config",
            parameters: None,
        };
        const WEB_HASHRATESPLIT_STATUS: MinerCommand = MinerCommand::WebAPI {
            command: "hashratesplit/status",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                WEB_NETWORK,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Hostname"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Session/Uptime"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Power Supply Stats/Input Power"),
                    tag: None,
                },
            )],
            DataField::TuningTarget => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Fans Rpm"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![
                (
                    WEB_TEMPS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("Board Temps"),
                    },
                ),
                (
                    WEB_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("Summary"),
                    },
                ),
                (
                    WEB_CAPABILITIES,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("Capabilities"),
                    },
                ),
            ],
            DataField::Chips => vec![
                (
                    WEB_CHIP_TEMPS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("Chip Temps"),
                    },
                ),
                (
                    WEB_CHIP_VOLTAGES,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("Chip Voltages"),
                    },
                ),
                (
                    WEB_CHIP_HASHRATES,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("Chip Hashrates"),
                    },
                ),
                (
                    WEB_CHIP_CLOCKS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("Chip Clocks"),
                    },
                ),
            ],
            DataField::Pools => vec![
                (
                    WEB_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: None,
                    },
                ),
                (
                    WEB_HASHRATESPLIT_CONFIG,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("Hashratesplit Config"),
                    },
                ),
                (
                    WEB_HASHRATESPLIT_STATUS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("Hashratesplit Status"),
                    },
                ),
            ],
            DataField::IsMining => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Status/Operating State"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Misc/Locate Miner State"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![
                (
                    WEB_CAPABILITIES,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/Control Board Version/cpuHardware"),
                        tag: Some("cpu"),
                    },
                ),
                (
                    WEB_CAPABILITIES,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/Control Board Version/platform"),
                        tag: Some("platform"),
                    },
                ),
            ],
            // Tagged rather than relying on location order: `Miner Serial
            // Number` is frequently present-but-null, and `get_by_pointer`
            // returns `Some(Value::Null)` for that, which an untagged fallback
            // would merge over the control board serial and lose both.
            DataField::SerialNumber => vec![
                (
                    WEB_CAPABILITIES,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/Miner Serial Number"),
                        tag: Some("miner"),
                    },
                ),
                (
                    WEB_CAPABILITIES,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/Control Board Version/cpuSerial"),
                        tag: Some("control_board"),
                    },
                ),
            ],
            DataField::ExpectedHashrate => vec![(
                WEB_CAPABILITIES,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Default Hashrate"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Software"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/HBs"),
                    tag: None,
                },
            )],
            DataField::Messages => vec![(
                WEB_SUMMARY,
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

impl GetIP for PowerPlayV1 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for PowerPlayV1 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for PowerPlayV1 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for PowerPlayV1 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        match serde_json::from_value::<HashMap<String, Value>>(data.get(&DataField::Mac)?.clone())
            .ok()
            .and_then(|inner| inner.get("dhcp").or_else(|| inner.get("static")).cloned())
            .and_then(|obj| {
                obj.get("mac_address")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            }) {
            Some(mac_str) => MacAddr::from_str(&mac_str).ok(),
            None => None,
        }
    }
}

impl GetSerialNumber for PowerPlayV1 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        // The manufacturer serial is the meaningful identifier, but PowerPlay
        // only reports it on some units; fall back to the control board CPU
        // serial so the field stays populated everywhere it used to be.
        data.extract_nested::<String>(DataField::SerialNumber, "miner")
            .or_else(|| data.extract_nested::<String>(DataField::SerialNumber, "control_board"))
    }
}

impl GetHostname for PowerPlayV1 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for PowerPlayV1 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for PowerPlayV1 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetControlBoardVersion for PowerPlayV1 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        if let Some(cb) =
            data.extract_nested::<EPicControlBoard>(DataField::ControlBoardVersion, "platform")
        {
            return Some(cb.into());
        }

        let is_volcminer_model = self.device_info.make == VolcMinerMake::default().to_string();

        if let Some(platform) =
            data.extract_nested::<String>(DataField::ControlBoardVersion, "platform")
        {
            let is_volcminer_platform = platform.trim().eq_ignore_ascii_case("TVXilinx");

            if (is_volcminer_platform || is_volcminer_model)
                && let Some(cb) = VolcMinerControlBoard::parse(&platform)
            {
                return Some(cb.into());
            }
        }

        if let Some(cb) =
            data.extract_nested::<AntMinerControlBoard>(DataField::ControlBoardVersion, "platform")
        {
            return Some(cb.into());
        }

        // Fallback for older versions that do not have platform.
        let cb_type = data.extract_nested::<String>(DataField::ControlBoardVersion, "cpu")?;

        match cb_type.as_str() {
            s if s.to_uppercase().contains("AMLOGIC") => {
                Some(AntMinerControlBoard::AMLogic).map(|cb| cb.into())
            }
            s if s.to_uppercase().contains("GENERIC AM33XX") => {
                Some(AntMinerControlBoard::BeagleBoneBlack).map(|cb| cb.into())
            }
            s if s.to_uppercase().contains("XILINX") => {
                if is_volcminer_model && let Some(cb) = VolcMinerControlBoard::parse(s) {
                    return Some(cb.into());
                }

                Some(AntMinerControlBoard::Xilinx).map(|cb| cb.into())
            }
            _ => Some(EPicControlBoard::EPicUMC).map(|cb| cb.into()),
        }
    }
}

impl GetHashboards for PowerPlayV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> =
            (0..self.device_info.hardware.board_count().unwrap_or(0))
                .map(|idx| {
                    BoardData::new(idx, self.device_info.hardware.chips_for_board(idx as usize))
                })
                .collect();

        let Some(api_data) = data.get(&DataField::Hashboards) else {
            return hashboards;
        };
        let chip_data = data.get(&DataField::Chips);
        let board_chip_count = api_data
            .pointer("/Capabilities/Performance Estimator/Chip Count")
            .and_then(|v| v.as_u64())
            .and_then(|chips| u16::try_from(chips).ok())
            .or_else(|| self.device_info.hardware.chips_for_board(0));
        let hashes_per_clock = api_data
            .pointer("/Capabilities/Performance Estimator/Hashes Per Second Per Chip")
            .and_then(|v| v.as_f64());

        // Active status
        if let Some(boards) = api_data
            .pointer("/Summary/HBStatus")
            .and_then(|v| v.as_array())
        {
            for board in boards {
                if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                    && let Some(hashboard) = hashboards.get_mut(idx as usize)
                {
                    hashboard.active = board.get("Enabled").and_then(|v| v.as_bool());
                    if chip_data.is_none()
                        && let Some(active) = hashboard.active
                    {
                        hashboard.working_chips = Some(if active {
                            board_chip_count.unwrap_or_default()
                        } else {
                            0
                        });
                    }
                }
            }
        }

        if chip_data.is_some() {
            for board in &mut hashboards {
                if board.active.unwrap_or(false) {
                    board.chips = vec![
                        ChipData::default();
                        board.expected_chips.unwrap_or_default() as usize
                    ];
                }
            }
        }

        // Serial numbers (unindexed array, assigned to first active board without one)
        if let Some(serial_numbers) = api_data
            .pointer("/Capabilities/Board Serial Numbers")
            .and_then(|v| v.as_array())
        {
            for serial in serial_numbers {
                for hb in hashboards.iter_mut() {
                    if hb.serial_number.is_none() && hb.active.unwrap_or(false) {
                        hb.serial_number = serial.as_str().map(String::from);
                        break;
                    }
                }
            }
        }

        // Summary data
        if let Some(boards) = api_data.pointer("/Summary/HBs").and_then(|v| v.as_array()) {
            for board in boards {
                if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                    && let Some(hashboard) = hashboards.get_mut(idx as usize)
                {
                    let hashrate_arr = board.get("Hashrate").and_then(|v| v.as_array());
                    hashboard.hashrate = hashrate_arr
                        .and_then(|arr| arr.first().and_then(|v| v.as_f64()))
                        .map(|h| {
                            HashRate {
                                value: h,
                                unit: HashRateUnit::MegaHash,
                                algo: self.device_info.algo,
                            }
                            .as_unit(HashRateUnit::default())
                        });
                    hashboard.voltage = board
                        .get("Input Voltage")
                        .and_then(|v| v.as_f64())
                        .map(Voltage::from_volts);
                    hashboard.frequency = board
                        .get("Core Clock Avg")
                        .and_then(|v| v.as_f64())
                        .map(Frequency::from_megahertz);
                    hashboard.expected_hashrate = board_chip_count
                        .zip(hashes_per_clock)
                        .zip(hashboard.frequency)
                        .map(|((chips, hashes_per_clock), frequency)| {
                            HashRate {
                                value: chips as f64 * hashes_per_clock * frequency.as_megahertz(),
                                unit: HashRateUnit::MegaHash,
                                algo: self.device_info.algo,
                            }
                            .as_unit(HashRateUnit::default())
                        });
                    hashboard.board_temperature = board
                        .get("Temperature")
                        .and_then(|v| v.as_f64())
                        .map(Temperature::from_celsius);
                }
            }
        }

        // Tuned status (applies to all boards uniformly)
        if let Some(tuned) = api_data
            .pointer("/Summary/PerpetualTune/Algorithm")
            .and_then(|v| v.as_object())
            .and_then(|algorithms| {
                algorithms
                    .values()
                    .find_map(|algo| algo.get("Optimized").and_then(|v| v.as_bool()))
            })
        {
            for hashboard in &mut hashboards {
                hashboard.tuned = Some(tuned);
            }
        }

        // Board temperatures (outlet = max sensor, intake = min sensor)
        if let Some(boards) = api_data.pointer("/Board Temps").and_then(|v| v.as_array()) {
            for board in boards {
                if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                    && let Some(hashboard) = hashboards.get_mut(idx as usize)
                {
                    let temps = board.get("Data").and_then(|v| v.as_array());
                    hashboard.outlet_chip_temperature = temps
                        .and_then(|arr| {
                            arr.iter().filter_map(|v| v.as_f64()).max_by(f64::total_cmp)
                        })
                        .map(Temperature::from_celsius);
                    hashboard.inlet_chip_temperature = temps
                        .and_then(|arr| {
                            arr.iter().filter_map(|v| v.as_f64()).min_by(f64::total_cmp)
                        })
                        .map(Temperature::from_celsius);
                }
            }
        }

        // Chip temperatures
        if let Some(boards) = chip_data
            .and_then(|data| data.pointer("/Chip Temps"))
            .and_then(|v| v.as_array())
        {
            for board in boards {
                if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                    && let Some(hashboard) = hashboards.get_mut(idx as usize)
                    && let Some(temps) = board.get("Data").and_then(|v| v.as_array())
                {
                    for (chip_no, temp) in temps.iter().filter_map(|v| v.as_f64()).enumerate() {
                        if let Some(chip) = hashboard.chips.get_mut(chip_no) {
                            chip.position = chip_no as u16;
                            chip.temperature = Some(Temperature::from_celsius(temp));
                        }
                    }
                }
            }
        }

        // Chip voltages
        if let Some(boards) = chip_data
            .and_then(|data| data.pointer("/Chip Voltages"))
            .and_then(|v| v.as_array())
        {
            for board in boards {
                if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                    && let Some(hashboard) = hashboards.get_mut(idx as usize)
                    && let Some(voltages) = board.get("Data").and_then(|v| v.as_array())
                {
                    for (chip_no, voltage) in voltages.iter().filter_map(|v| v.as_f64()).enumerate()
                    {
                        if let Some(chip) = hashboard.chips.get_mut(chip_no) {
                            chip.position = chip_no as u16;
                            chip.voltage = Some(Voltage::from_millivolts(voltage));
                        }
                    }
                }
            }
        }

        // Chip frequencies
        if let Some(boards) = chip_data
            .and_then(|data| data.pointer("/Chip Clocks"))
            .and_then(|v| v.as_array())
        {
            for board in boards {
                if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                    && let Some(hashboard) = hashboards.get_mut(idx as usize)
                    && let Some(freqs) = board.get("Data").and_then(|v| v.as_array())
                {
                    for (chip_no, freq) in freqs.iter().filter_map(|v| v.as_f64()).enumerate() {
                        if let Some(chip) = hashboard.chips.get_mut(chip_no) {
                            chip.position = chip_no as u16;
                            chip.frequency = Some(Frequency::from_megahertz(freq));
                        }
                    }
                }
            }
        }

        // Chip hashrates — presence implies the chip is working
        if let Some(boards) = chip_data
            .and_then(|data| data.pointer("/Chip Hashrates"))
            .and_then(|v| v.as_array())
        {
            for board in boards {
                if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                    && let Some(hashboard) = hashboards.get_mut(idx as usize)
                    && let Some(hashrates) = board.get("Data").and_then(|v| v.as_array())
                {
                    for (chip_no, hr) in hashrates
                        .iter()
                        .filter_map(|inner| inner.as_array())
                        .filter_map(|inner| inner.first().and_then(|v| v.as_f64()))
                        .enumerate()
                    {
                        if let Some(chip) = hashboard.chips.get_mut(chip_no) {
                            chip.position = chip_no as u16;
                            chip.working = Some(true);
                            chip.hashrate = Some(
                                HashRate {
                                    value: hr,
                                    unit: HashRateUnit::MegaHash,
                                    algo: self.device_info.algo,
                                }
                                .as_unit(HashRateUnit::default()),
                            );
                        }
                    }
                }
            }
        }

        if chip_data.is_some() {
            for board in &mut hashboards {
                board.working_chips = Some(
                    board
                        .chips
                        .iter()
                        .filter(|c| c.working.unwrap_or(false))
                        .count() as u16,
                );
            }
        }

        hashboards
    }
}

impl GetHashrate for PowerPlayV1 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        let mut total_hashrate: f64 = 0.0;

        data.get(&DataField::Hashrate).and_then(|v| {
            v.as_array().map(|boards| {
                boards.iter().for_each(|board| {
                    if let Some(_idx) = board.get("Index").and_then(|v| v.as_u64()) {
                        // Hashrate
                        if let Some(h) = board
                            .get("Hashrate")
                            .and_then(|v| v.as_array())
                            .and_then(|v| v.first().and_then(|f| f.as_f64()))
                        {
                            total_hashrate += h;
                        };
                    }
                })
            })
        });

        Some(
            HashRate {
                value: total_hashrate,
                unit: HashRateUnit::MegaHash,
                algo: self.device_info.algo,
            }
            .as_unit(HashRateUnit::default()),
        )
    }
}

impl GetExpectedHashrate for PowerPlayV1 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| {
            HashRate {
                value: f,
                unit: HashRateUnit::TeraHash,
                algo: self.device_info.algo,
            }
            .as_unit(HashRateUnit::default())
        })
    }
}

impl GetFans for PowerPlayV1 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans: Vec<FanData> = Vec::new();

        if let Some(fans_data) = data.get(&DataField::Fans)
            && let Some(obj) = fans_data.as_object()
        {
            for (key, value) in obj {
                if let Some(num) = value.as_f64() {
                    // Extract the number from the key (e.g. "Fans Speed 3" -> 3)
                    if let Some(pos_str) = key.strip_prefix("Fans Speed ")
                        && let Ok(pos) = pos_str.parse::<i16>()
                    {
                        fans.push(FanData {
                            position: pos,
                            rpm: Some(AngularVelocity::from_rpm(num)),
                        });
                    }
                }
            }
        }

        fans
    }
}

impl GetPsuFans for PowerPlayV1 {}
impl GetTuningCapabilities for PowerPlayV1 {}

impl GetFluidTemperature for PowerPlayV1 {}

impl GetWattage for PowerPlayV1 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}

fn tuning_value_as_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|v| v as f64))
        .or_else(|| value.as_u64().map(|v| v as f64))
}

fn first_perpetual_tune_algorithm(summary: &Value) -> Option<(&str, &Value)> {
    summary
        .pointer("/PerpetualTune/Algorithm")
        .and_then(Value::as_object)?
        .iter()
        .next()
        .map(|(algorithm, stats)| (algorithm.as_str(), stats))
}

fn parse_tuning_target_from_stats(
    algorithm: &str,
    stats: &Value,
    algorithm_drives_power: bool,
    algo: HashAlgorithm,
) -> Option<TuningTarget> {
    let target = tuning_value_as_f64(stats.get("Target")?)?;
    parse_tuning_target_value_from_stats(algorithm, stats, target, algorithm_drives_power, algo)
}

fn parse_scaled_tuning_target_from_stats(
    algorithm: &str,
    stats: &Value,
    algorithm_drives_power: bool,
    algo: HashAlgorithm,
) -> Option<TuningTarget> {
    let throttle_target = stats.get("Throttle Target").and_then(tuning_value_as_f64);
    let error_throttle_target = stats
        .get("Error Throttle Target")
        .and_then(tuning_value_as_f64);
    let target = [throttle_target, error_throttle_target]
        .into_iter()
        .flatten()
        .min_by(f64::total_cmp)?;

    parse_tuning_target_value_from_stats(algorithm, stats, target, algorithm_drives_power, algo)
}

fn parse_tuning_target_value_from_stats(
    algorithm: &str,
    stats: &Value,
    target: f64,
    algorithm_drives_power: bool,
    algo: HashAlgorithm,
) -> Option<TuningTarget> {
    let unit = stats
        .get("Unit")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let unit_is_power = unit.trim().to_ascii_uppercase().contains('W');
    let algorithm_is_power = algorithm.to_ascii_lowercase().contains("power");
    if unit_is_power || (algorithm_drives_power && algorithm_is_power) {
        return Some(TuningTarget::Power(Power::from_watts(target)));
    }

    let hr_unit = unit.parse::<HashRateUnit>().ok()?;
    Some(TuningTarget::HashRate(
        HashRate {
            value: target,
            unit: hr_unit,
            algo,
        }
        .as_unit(HashRateUnit::default()),
    ))
}

fn to_non_negative_u32_target(value: f64, label: &str) -> anyhow::Result<u32> {
    anyhow::ensure!(value.is_finite(), "{label} target is not finite");
    anyhow::ensure!(value >= 0.0, "{label} target must be non-negative");

    let rounded = value.round();
    anyhow::ensure!(
        rounded <= u32::MAX as f64,
        "{label} target exceeds maximum supported value"
    );

    Ok(rounded as u32)
}

impl GetTuningTarget for PowerPlayV1 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        data.get(&DataField::TuningTarget).and_then(|summary| {
            if !summary
                .pointer("/PerpetualTune/Running")
                .and_then(Value::as_bool)?
            {
                return None;
            }

            let (algorithm, stats) = first_perpetual_tune_algorithm(summary)?;
            parse_tuning_target_from_stats(algorithm, stats, true, self.device_info.algo)
        })
    }
}

impl GetScaledTuningTarget for PowerPlayV1 {
    fn parse_scaled_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        data.get(&DataField::TuningTarget).and_then(|summary| {
            if !summary
                .pointer("/PerpetualTune/Running")
                .and_then(Value::as_bool)?
            {
                return None;
            }

            let (algorithm, stats) = first_perpetual_tune_algorithm(summary)?;
            parse_scaled_tuning_target_from_stats(algorithm, stats, true, self.device_info.algo)
        })
    }
}

impl GetLightFlashing for PowerPlayV1 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing)
    }
}

impl GetMessages for PowerPlayV1 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let mut messages = Vec::new();
        let timestamp = unix_timestamp_secs();

        if let Some(last_error) = data
            .get(&DataField::Messages)
            .and_then(|v| v.pointer("/Status/Last Error"))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            messages.push(MinerMessage::new(
                timestamp as u32,
                0,
                last_error.to_string(),
                MessageSeverity::Error,
            ));
        }

        messages
    }
}

impl GetUptime for PowerPlayV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetBestShare for PowerPlayV1 {}
impl GetSessionBestShare for PowerPlayV1 {}

impl GetIsMining for PowerPlayV1 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        data.extract::<String>(DataField::IsMining)
            .map(|state| state != "Idling")
            .unwrap_or(true)
    }
}

impl GetPools for PowerPlayV1 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let Some(pools_data) = data.get(&DataField::Pools) else {
            return vec![];
        };

        let Some(pools_object) = pools_data.as_object() else {
            return vec![];
        };

        let split_enabled = pools_object
            .get("Hashratesplit Config")
            .and_then(|v| v.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if split_enabled {
            Self::parse_hashrate_split_pools(pools_object)
        } else {
            Self::parse_standard_pools(pools_object)
        }
    }
}

#[async_trait]
impl SetFaultLight for PowerPlayV1 {
    #[allow(unused_variables)]
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        self.web
            .send_command(
                "identify",
                false,
                Some(json!({ "param": fault })),
                Method::POST,
            )
            .await
            .map(|v| v.get("result").and_then(Value::as_bool).unwrap_or(false))
    }
    fn supports_set_fault_light(&self) -> bool {
        true
    }
}

#[async_trait]
impl SetPowerLimit for PowerPlayV1 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsPoolsConfig for PowerPlayV1 {
    fn parse_pools_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<Vec<PoolGroupConfig>> {
        let Some(pools_data) = data.get(&ConfigField::Pools) else {
            return Ok(vec![]);
        };

        if pools_data.is_array() {
            return Ok(vec![]);
        }

        let Some(pools_object) = pools_data.as_object() else {
            return Ok(vec![]);
        };

        let split_enabled = pools_object
            .get("Hashratesplit Config")
            .and_then(|v| v.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if split_enabled {
            let mut groups: Vec<PoolGroupConfig> = Vec::new();

            if let Some(splits) = pools_object
                .get("Hashratesplit Config")
                .and_then(|v| v.get("hashrate_splits"))
                .and_then(Value::as_array)
            {
                for (idx, split) in splits.iter().enumerate() {
                    let name = format!("group{}", idx + 1);
                    let quota = split
                        .get("ratio")
                        .and_then(Value::as_u64)
                        .and_then(|ratio| u32::try_from(ratio).ok())
                        .unwrap_or(1);
                    let pools = split
                        .get("stratum_configs")
                        .map(Self::parse_stratum_configs)
                        .unwrap_or_default();

                    groups.push(PoolGroupConfig { name, quota, pools });
                }
            }

            groups.truncate(3);

            Ok(groups)
        } else {
            let groups = vec![PoolGroupConfig {
                name: String::new(),
                quota: 1,
                pools: pools_object
                    .get("summary")
                    .map(Self::parse_stratum_configs)
                    .unwrap_or_default(),
            }];

            Ok(groups)
        }
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> anyhow::Result<bool> {
        let response_ok = |v: &Value| v.get("result").and_then(Value::as_bool).unwrap_or(false);

        let groups: Vec<PoolGroupConfig> = config
            .into_iter()
            .filter(|group| !group.pools.is_empty())
            .collect();

        anyhow::ensure!(!groups.is_empty(), "No non-empty pool groups provided");
        anyhow::ensure!(groups.len() <= 3, "ePIC supports up to 3 pool groups");

        let coin = Self::coin_type_for_hash_algorithm(self.device_info.algo);
        let unique_id_enabled = false;

        if let [group] = groups.as_slice() {
            let set_coin = self
                .web
                .send_command(
                    "coin",
                    false,
                    Some(json!({
                        "param": {
                            "coin": coin,
                            "stratum_configs": Self::to_stratum_configs(group),
                            "unique_id": unique_id_enabled,
                        }
                    })),
                    Method::POST,
                )
                .await?;

            if !response_ok(&set_coin) {
                return Ok(false);
            }

            let disable_split = self
                .web
                .send_command(
                    "hashratesplit/enable",
                    false,
                    Some(json!({ "param": false })),
                    Method::POST,
                )
                .await?;

            Ok(response_ok(&disable_split))
        } else {
            let total_quota: u32 = groups.iter().map(|g| g.quota.max(1)).sum();
            let mut allocated_ratio = 0_u32;

            let split: Vec<Value> = groups
                .iter()
                .enumerate()
                .map(|(idx, group)| {
                    let remaining = 100_u32.saturating_sub(allocated_ratio);
                    let ratio = if idx + 1 == groups.len() {
                        remaining
                    } else {
                        let share = ((group.quota.max(1) as f64 / total_quota as f64) * 100.0)
                            .round() as u32;
                        let bounded = share.min(remaining);
                        allocated_ratio += bounded;
                        bounded
                    };

                    json!({
                        "coin": coin,
                        "ratio": ratio,
                        "sc_index": idx,
                        "stratum_configs": Self::to_stratum_configs(group),
                        "unique_id": unique_id_enabled,
                        // this needs to be set since it's not an option
                        "unique_worker_id_variant": "MacAddress",
                    })
                })
                .collect();

            let set_split = self
                .web
                .send_command(
                    "hashratesplit",
                    false,
                    Some(json!({ "param": split })),
                    Method::POST,
                )
                .await?;

            if !response_ok(&set_split) {
                return Ok(false);
            }

            let enable_split = self
                .web
                .send_command(
                    "hashratesplit/enable",
                    false,
                    Some(json!({ "param": true })),
                    Method::POST,
                )
                .await?;

            Ok(response_ok(&enable_split))
        }
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsScalingConfig for PowerPlayV1 {
    async fn set_scaling_config(&self, config: ScalingConfig) -> anyhow::Result<bool> {
        // PowerPlay has no scaling-only endpoint: `min_throttle` and
        // `throttle_step` are written by the same `perpetualtune/algo` call that
        // sets the tuning target, so the current target is read and resent to
        // avoid clobbering it.
        let current = self.get_tuning_config().await?;
        self.set_tuning_config(current, Some(config)).await
    }

    fn parse_scaling_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<ScalingConfig> {
        data.get(&ConfigField::Scaling)
            .and_then(Value::as_object)
            .and_then(|algorithms| {
                algorithms.values().find_map(|stats| {
                    let minimum = stats
                        .get("Min Throttle Target")
                        .or_else(|| stats.get("min"))
                        .and_then(|v| u32::try_from(v.as_u64()?).ok())?;
                    let step = stats
                        .get("Throttle Step")
                        .or_else(|| stats.get("step"))
                        .and_then(|v| u32::try_from(v.as_u64()?).ok())?;
                    Some(ScalingConfig::new(step, minimum))
                })
            })
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to parse scaling config from summary perpetual tune data")
            })
    }

    fn supports_scaling_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsTuningConfig for PowerPlayV1 {
    async fn set_tuning_config(
        &self,
        config: TuningConfig,
        scaling_config: Option<ScalingConfig>,
    ) -> anyhow::Result<bool> {
        let parse_algorithm = |algorithm_input: &str| -> anyhow::Result<&'static str> {
            let normalized_algorithm = algorithm_input
                .trim()
                .to_ascii_lowercase()
                .replace([' ', '_', '-'], "");

            match normalized_algorithm.as_str() {
                "chiptune" => Ok("ChipTune"),
                "voltageoptimizer" => Ok("VoltageOptimizer"),
                "power" | "powertune" => Ok("PowerTune"),
                "boardtune" => Ok("BoardTune"),
                _ => anyhow::bail!(
                    "Unsupported perpetual tune algorithm '{algorithm_input}' for ePIC PowerPlay"
                ),
            }
        };

        let (algorithm, target) = match &config.target {
            TuningTarget::Power(power) => (
                "PowerTune",
                to_non_negative_u32_target(power.as_watts(), "power")?,
            ),
            TuningTarget::HashRate(hashrate) => {
                let algorithm_input = config.algorithm.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("TuningConfig.algorithm is required for hashrate tuning")
                })?;
                let algorithm = parse_algorithm(algorithm_input)?;
                anyhow::ensure!(
                    algorithm != "PowerTune",
                    "Hashrate tuning target cannot be used with PowerTune algorithm"
                );
                let target = to_non_negative_u32_target(
                    hashrate.clone().as_unit(HashRateUnit::TeraHash).value,
                    "hashrate",
                )?;
                (algorithm, target)
            }
            TuningTarget::MiningMode(_) => {
                anyhow::bail!("MiningMode tuning target is not supported on ePIC PowerPlay")
            }
            TuningTarget::Preset(_) => {
                anyhow::bail!("Preset tuning target is not supported on ePIC PowerPlay")
            }
        };

        let Some(scaling) = scaling_config else {
            anyhow::bail!("ScalingConfig is required for ePIC PowerPlay")
        };

        let payload = json!({
            "param": {
                "algo": algorithm,
                "target": target,
                "min_throttle": scaling.minimum,
                "throttle_step": scaling.step,
            }
        });

        self.web
            .send_command("perpetualtune/algo", false, Some(payload), Method::POST)
            .await
            .map(|v| v.get("result").and_then(Value::as_bool).unwrap_or(false))
    }

    fn parse_tuning_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<TuningConfig> {
        data.get(&ConfigField::Tuning)
            .and_then(|summary| {
                let (algorithm, stats) = first_perpetual_tune_algorithm(summary)?;
                let tuning_target =
                    parse_tuning_target_from_stats(algorithm, stats, false, self.device_info.algo)?;
                Some(TuningConfig::new(tuning_target).with_algorithm(algorithm))
            })
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to parse tuning config from summary perpetual tune data")
            })
    }

    fn supports_tuning_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsFanConfig for PowerPlayV1 {
    fn parse_fan_config(&self, data: &HashMap<ConfigField, Value>) -> anyhow::Result<FanConfig> {
        let fan_mode = data
            .get(&ConfigField::Fan)
            .ok_or_else(|| anyhow::anyhow!("No fan mode data in summary response"))?;

        if let Some(auto) = fan_mode.get("Auto") {
            let Some(target_temp) = auto.get("Target Temperature").and_then(Value::as_f64) else {
                anyhow::bail!("Missing Auto/Target Temperature in fan mode data");
            };
            let idle_speed = auto.get("Idle Speed").and_then(Value::as_u64);

            Ok(FanConfig::auto(target_temp, idle_speed))
        } else if let Some(manual_speed) = fan_mode.get("Manual").and_then(Value::as_u64) {
            Ok(FanConfig::manual(manual_speed))
        } else {
            anyhow::bail!("Failed to parse fan mode as Auto or Manual")
        }
    }

    async fn set_fan_config(&self, config: FanConfig) -> anyhow::Result<bool> {
        let payload = match config {
            FanConfig::Auto {
                target_temp,
                idle_speed,
            } => {
                let idle_speed = idle_speed.unwrap_or(20);
                let target_temp = target_temp.round().max(0.0) as u64;

                json!({
                    "param": {
                        "Auto": {
                            "Target Temperature": target_temp,
                            "Idle Speed": idle_speed,
                        }
                    }
                })
            }
            FanConfig::Manual { fan_speed } => json!({
                "param": {
                    "Manual": fan_speed,
                }
            }),
        };

        self.web
            .send_command("fanspeed", false, Some(payload), Method::POST)
            .await
            .map(|v| v.get("result").and_then(Value::as_bool).unwrap_or(false))
    }

    fn supports_fan_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for PowerPlayV1 {
    async fn restart(&self) -> anyhow::Result<bool> {
        self.web
            .send_command("reboot", false, Some(json!({"param": "0"})), Method::POST)
            .await
            .map(|v| v.get("result").and_then(Value::as_bool).unwrap_or(false))
    }
    fn supports_restart(&self) -> bool {
        true
    }
}

#[async_trait]
impl Pause for PowerPlayV1 {
    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        self.web
            .send_command("miner", false, Some(json!({"param": "Stop"})), Method::POST)
            .await
            .map(|v| v.get("result").and_then(Value::as_bool).unwrap_or(false))
    }
    fn supports_pause(&self) -> bool {
        true
    }
}

#[async_trait]
impl Resume for PowerPlayV1 {
    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        self.web
            .send_command(
                "miner",
                false,
                Some(json!({ "param": "Autostart" })),
                Method::POST,
            )
            .await
            .map(|v| v.get("result").and_then(Value::as_bool).unwrap_or(false))
    }
    fn supports_resume(&self) -> bool {
        true
    }
}

#[async_trait]
impl ChangePassword for PowerPlayV1 {
    async fn change_password(&mut self, password: &str) -> anyhow::Result<bool> {
        let success = self.web.change_password(password).await?;
        if success {
            let username = self.web.username().to_string();
            self.set_auth(MinerAuth::new(username, password));
        }
        Ok(success)
    }

    fn supports_change_password(&self) -> bool {
        true
    }
}

#[async_trait]
impl ReadLogs for PowerPlayV1 {
    async fn read_logs(&self) -> anyhow::Result<String> {
        self.web.read_logs().await
    }

    fn supports_read_logs(&self) -> bool {
        true
    }
}

impl FactoryReset for PowerPlayV1 {
    fn supports_factory_reset(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for PowerPlayV1 {
    async fn upgrade_firmware(&self, image: FirmwareImage) -> anyhow::Result<bool> {
        self.web.upgrade_firmware(image).await
    }

    fn supports_upgrade_firmware(&self) -> bool {
        true
    }
}

impl HasDefaultAuth for PowerPlayV1 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("", "letmein")
    }
}

impl HasAuth for PowerPlayV1 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

impl Validate for PowerPlayV1 {
    type Firmware = EPicFirmware;
}

impl SupportsTemperatureConfig for PowerPlayV1 {}
impl GetTuningPercent for PowerPlayV1 {}
impl SetTuningPercent for PowerPlayV1 {}

impl SupportsPresets for PowerPlayV1 {}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::{self, Context};
    use asic_rs_core::{
        test::{api::MockAPIClient, util::get_miner},
        traits::firmware::MinerFirmware,
    };
    use asic_rs_makes_antminer::models::AntMinerModel;

    use super::*;
    use crate::test::json::v1::{
        CAPABILITIES, CHIP_CLOCKS, CHIP_HASHRATES, CHIP_TEMPS, CHIP_VOLTAGES, NETWORK, SUMMARY,
        TEMPS,
    };

    #[tokio::test]
    async fn parse_data_test_antminer_s19xp() -> anyhow::Result<()> {
        let miner = PowerPlayV1::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP);

        let mut results = HashMap::new();

        let commands = vec![
            ("summary", SUMMARY),
            ("capabilities", CAPABILITIES),
            ("temps", TEMPS),
            ("network", NETWORK),
            ("clocks", CHIP_CLOCKS),
            ("temps/chip", CHIP_TEMPS),
            ("voltages", CHIP_VOLTAGES),
            ("hashrate", CHIP_HASHRATES),
        ];

        for (command, data) in commands {
            let cmd: MinerCommand = MinerCommand::WebAPI {
                command,
                parameters: None,
            };
            results.insert(cmd, Value::from_str(data)?);
        }

        let mock_api = MockAPIClient::new(results);

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect(&[DataField::Hashboards]).await;
        assert!(!data.contains_key(&DataField::Chips));

        let hashboards_without_chips = miner.parse_hashboards(&data);
        assert_eq!(hashboards_without_chips.len(), 3);
        assert_eq!(hashboards_without_chips[0].active, Some(false));
        assert_eq!(hashboards_without_chips[0].working_chips, Some(0));
        assert!(hashboards_without_chips[1].chips.is_empty());
        assert_eq!(hashboards_without_chips[1].working_chips, Some(110));
        assert!(hashboards_without_chips[1].serial_number.is_some());
        assert!(hashboards_without_chips[1].hashrate.is_some());
        assert!(hashboards_without_chips[1].board_temperature.is_some());
        assert!(hashboards_without_chips[1].inlet_chip_temperature.is_some());
        assert!(
            hashboards_without_chips[1]
                .outlet_chip_temperature
                .is_some()
        );
        assert!(hashboards_without_chips[1].tuned.is_some());

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector
            .collect(&[DataField::Hashboards, DataField::Chips])
            .await;
        assert!(data.contains_key(&DataField::Chips));

        let hashboards_with_chips = miner.parse_hashboards(&data);
        assert_eq!(hashboards_with_chips[1].chips.len(), 110);
        assert_eq!(
            hashboards_without_chips[1].active,
            hashboards_with_chips[1].active
        );
        assert_eq!(
            hashboards_without_chips[1].serial_number,
            hashboards_with_chips[1].serial_number
        );
        assert_eq!(
            hashboards_without_chips[1].hashrate,
            hashboards_with_chips[1].hashrate
        );
        assert_eq!(
            hashboards_without_chips[1].board_temperature,
            hashboards_with_chips[1].board_temperature
        );
        assert_eq!(
            hashboards_without_chips[1].inlet_chip_temperature,
            hashboards_with_chips[1].inlet_chip_temperature
        );
        assert_eq!(
            hashboards_without_chips[1].outlet_chip_temperature,
            hashboards_with_chips[1].outlet_chip_temperature
        );
        assert_eq!(
            hashboards_without_chips[1].tuned,
            hashboards_with_chips[1].tuned
        );

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;

        let miner_data = miner.parse_data(data);

        assert_eq!(miner_data.uptime, Some(Duration::from_secs(23170)));
        assert_eq!(miner_data.wattage, Some(Power::from_watts(2166.6174)));
        assert_eq!(miner_data.hashboards.len(), 3);
        assert_eq!(miner_data.hashboards[0].active, Some(false));
        assert_eq!(miner_data.hashboards[1].chips.len(), 110);
        assert_eq!(
            miner_data.hashboards[1].chips[69].hashrate,
            Some(HashRate {
                value: 305937.8,
                unit: HashRateUnit::MegaHash,
                algo: HashAlgorithm::SHA256,
            })
        );
        assert_eq!(
            miner_data.hashboards[2].chips[72].hashrate,
            Some(HashRate {
                value: 487695.28,
                unit: HashRateUnit::MegaHash,
                algo: HashAlgorithm::SHA256,
            })
        );

        Ok(())
    }

    #[test]
    fn parse_scaling_config_test() -> anyhow::Result<()> {
        let summary = Value::from_str(SUMMARY)?;
        let algorithm = summary
            .pointer("/PerpetualTune/Algorithm")
            .cloned()
            .context("missing /PerpetualTune/Algorithm")?;

        let miner = PowerPlayV1::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP);
        let data = HashMap::from([(ConfigField::Scaling, algorithm)]);
        let config = miner.parse_scaling_config(&data)?;

        assert_eq!(config.minimum, 50);
        assert_eq!(config.step, 5);

        Ok(())
    }

    #[test]
    fn parse_messages_uses_summary_status_last_error() {
        let miner = PowerPlayV1::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP);
        let summary = serde_json::json!({
            "Status": {
                "Operating State": "AdjustingClockVoltage",
                "Last Command": "autostart",
                "Last Command Result": null,
                "Last Error": "Clock voltage adjustment failed"
            }
        });
        let data = HashMap::from([(DataField::Messages, summary)]);

        let messages = miner.parse_messages(&data);

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message, "Clock voltage adjustment failed");
        assert_eq!(messages[0].severity, MessageSeverity::Error);
    }

    #[test]
    fn parse_scaled_tuning_target_uses_lower_throttle_target() {
        let miner = PowerPlayV1::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP);
        let summary = serde_json::json!({
            "Mining": {
                "Algorithm": "SHA-256"
            },
            "PerpetualTune": {
                "Running": true,
                "Algorithm": {
                    "VoltageOptimizer": {
                        "Target": 143,
                        "Throttle Target": 120,
                        "Error Throttle Target": 95,
                        "Unit": "TH/s"
                    }
                }
            }
        });
        let data = HashMap::from([(DataField::TuningTarget, summary)]);

        assert_eq!(
            miner.parse_scaled_tuning_target(&data),
            Some(TuningTarget::HashRate(HashRate {
                value: 95.0,
                unit: HashRateUnit::TeraHash,
                algo: HashAlgorithm::SHA256,
            }))
        );
    }

    #[test]
    fn parse_scaled_tuning_target_uses_lower_power_throttle_target() {
        let miner = PowerPlayV1::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP);
        let summary = serde_json::json!({
            "Mining": {
                "Algorithm": "SHA-256"
            },
            "PerpetualTune": {
                "Running": true,
                "Algorithm": {
                    "PowerTune": {
                        "Target": 3000,
                        "Throttle Target": 2600,
                        "Error Throttle Target": 2800,
                        "Unit": "W"
                    }
                }
            }
        });
        let data = HashMap::from([(DataField::TuningTarget, summary)]);

        assert_eq!(
            miner.parse_scaled_tuning_target(&data),
            Some(TuningTarget::Power(Power::from_watts(2600.0)))
        );
    }

    #[test]
    fn parse_fan_config_test() -> anyhow::Result<()> {
        let summary = Value::from_str(SUMMARY)?;
        let fan_mode = summary
            .pointer("/Fans/Fan Mode")
            .cloned()
            .context("missing /Fans/Fan Mode")?;

        let miner = PowerPlayV1::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP);
        let data = HashMap::from([(ConfigField::Fan, fan_mode)]);
        let config = miner.parse_fan_config(&data)?;

        assert_eq!(config.mode(), asic_rs_core::config::fan::FanMode::Auto);
        assert_eq!(config.target_temp(), Some(60.0));
        assert_eq!(config.idle_speed(), Some(20));
        assert_eq!(config.fan_speed(), None);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires live miner; set MINER_IP"]
    async fn parse_data_live_test_auto_detect() -> anyhow::Result<()> {
        let ip_str = std::env::var("MINER_IP").context("MINER_IP is not set")?;
        let ip =
            IpAddr::from_str(&ip_str).with_context(|| format!("invalid MINER_IP: {ip_str}"))?;

        let miner = get_miner(ip, Arc::new(EPicFirmware::default()))
            .await?
            .context("no miner detected at MINER_IP")?;
        let miner_data = miner.get_data().await;
        let mut miner_data_print = miner_data.clone();
        for hashboard in &mut miner_data_print.hashboards {
            hashboard.chips.clear();
        }
        println!("{}", serde_json::to_string_pretty(&miner_data_print)?);

        println!(
            "pools {}",
            serde_json::to_string_pretty(&miner.get_pools_config().await?)?
        );

        let scaling_config = miner.get_scaling_config().await?;
        println!(
            "scalingconfig {}",
            serde_json::to_string_pretty(&scaling_config)?
        );

        let tuning_config = miner.get_tuning_config().await?;
        println!(
            "tuningconfig {}",
            serde_json::to_string_pretty(&tuning_config)?
        );

        println!(
            "fanconfig {}",
            serde_json::to_string_pretty(&miner.get_fan_config().await?)?
        );

        println!(
            "messages {}",
            serde_json::to_string_pretty(&miner.get_messages().await)?
        );

        assert_eq!(miner_data.ip, ip);
        assert!(miner_data.timestamp > 0);
        assert!(!miner_data.schema_version.is_empty());

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires live miner and writes perpetual tune config; set MINER_IP and EPIC_TUNE_MODE=chiptune|powertune"]
    async fn set_perpetual_tune_live_test() -> anyhow::Result<()> {
        let ip_str = std::env::var("MINER_IP").context("MINER_IP is not set")?;
        let ip =
            IpAddr::from_str(&ip_str).with_context(|| format!("invalid MINER_IP: {ip_str}"))?;
        let tune_mode = std::env::var("EPIC_TUNE_MODE")
            .context("EPIC_TUNE_MODE is not set (expected chiptune or powertune)")?;

        let miner = get_miner(ip, Arc::new(EPicFirmware::default()))
            .await?
            .context("no miner detected at MINER_IP")?;

        let current_tuning = miner.get_tuning_config().await?;
        let current_scaling = miner.get_scaling_config().await?;
        println!(
            "current perpetual tune settings: tuning={} scaling={}",
            serde_json::to_string_pretty(&current_tuning)?,
            serde_json::to_string_pretty(&current_scaling)?,
        );

        let target_tuning = match tune_mode.trim().to_ascii_lowercase().as_str() {
            "chiptune" => {
                let target = std::env::var("EPIC_HASHRATE_TARGET_TH")
                    .unwrap_or_else(|_| "103".to_string())
                    .parse::<f64>()
                    .context("EPIC_HASHRATE_TARGET_TH must be a number in TH/s")?;
                TuningConfig::new(TuningTarget::HashRate(HashRate {
                    value: target,
                    unit: HashRateUnit::TeraHash,
                    algo: HashAlgorithm::SHA256,
                }))
                .with_algorithm("ChipTune")
            }
            "powertune" => {
                let target = std::env::var("EPIC_POWER_TARGET_W")
                    .unwrap_or_else(|_| "3000".to_string())
                    .parse::<f64>()
                    .context("EPIC_POWER_TARGET_W must be a number in watts")?;
                TuningConfig::new(TuningTarget::Power(Power::from_watts(target)))
            }
            _ => anyhow::bail!(
                "invalid EPIC_TUNE_MODE '{tune_mode}' (expected chiptune or powertune)"
            ),
        };

        println!(
            "target perpetual tune settings: tuning={} scaling={}",
            serde_json::to_string_pretty(&target_tuning)?,
            serde_json::to_string_pretty(&current_scaling)?,
        );
        anyhow::ensure!(
            miner
                .set_tuning_config(target_tuning, Some(current_scaling))
                .await?,
            "miner rejected the perpetual tune settings"
        );

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires live miner and writes perpetual tune config; set MINER_IP"]
    async fn set_scaling_config_live_test() -> anyhow::Result<()> {
        let ip_str = std::env::var("MINER_IP").context("MINER_IP is not set")?;
        let ip =
            IpAddr::from_str(&ip_str).with_context(|| format!("invalid MINER_IP: {ip_str}"))?;

        let miner = get_miner(ip, Arc::new(EPicFirmware::default()))
            .await?
            .context("no miner detected at MINER_IP")?;

        let original_tuning = miner.get_tuning_config().await?;
        let original_scaling = miner.get_scaling_config().await?;
        println!(
            "original: tuning={} scaling={}",
            serde_json::to_string_pretty(&original_tuning)?,
            serde_json::to_string_pretty(&original_scaling)?,
        );

        let target = ScalingConfig::new(original_scaling.step + 1, original_scaling.minimum);
        anyhow::ensure!(
            miner.set_scaling_config(target).await?,
            "miner rejected the scaling config"
        );

        // The tuning target must survive a scaling-only write. Compared as JSON
        // because TuningConfig is not PartialEq.
        let before = serde_json::to_string(&original_tuning)?;
        let after = serde_json::to_string(&miner.get_tuning_config().await?)?;
        anyhow::ensure!(
            before == after,
            "scaling write changed the tuning target: {before} -> {after}"
        );

        miner.set_scaling_config(original_scaling).await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires live miner and writes pool config; set MINER_IP"]
    async fn set_pools_config_live_test_increment_worker() -> anyhow::Result<()> {
        let ip_str = std::env::var("MINER_IP").context("MINER_IP is not set")?;
        let ip =
            IpAddr::from_str(&ip_str).with_context(|| format!("invalid MINER_IP: {ip_str}"))?;

        let model = EPicFirmware::get_model(ip).await?;
        let miner = PowerPlayV1::new(ip, model);

        let current = miner.get_pools_config().await?;
        println!("current pools {}", serde_json::to_string_pretty(&current)?);
        anyhow::ensure!(!current.is_empty(), "miner has no pool groups configured");

        let target = current
            .iter()
            .map(|group| PoolGroupConfig {
                name: group.name.clone(),
                quota: group.quota,
                pools: group
                    .pools
                    .iter()
                    .map(|pool| PoolConfig {
                        url: pool.url.clone(),
                        username: format!("{}1", pool.username),
                        password: pool.password.clone(),
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();

        assert!(miner.set_pools_config(target.clone()).await?);

        let updated = miner.get_pools_config().await?;
        println!("updated pools {}", serde_json::to_string_pretty(&updated)?);

        for expected in target.iter().flat_map(|group| group.pools.iter()) {
            let updated_pool = updated
                .iter()
                .flat_map(|group| group.pools.iter())
                .find(|pool| pool.url == expected.url && pool.username == expected.username)
                .with_context(|| {
                    format!(
                        "target pool config was not written: {} {}",
                        expected.url, expected.username
                    )
                })?;

            assert_eq!(updated_pool.password, expected.password);
        }

        Ok(())
    }

    /// Builds a collector over the v1 fixtures with `capabilities` overridden,
    /// then returns whatever `parse_serial_number` makes of it.
    async fn serial_number_for(capabilities: Value) -> Option<String> {
        let miner = PowerPlayV1::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP);

        let mut results = HashMap::new();
        results.insert(
            MinerCommand::WebAPI {
                command: "capabilities",
                parameters: None,
            },
            capabilities,
        );

        let mock_api = MockAPIClient::new(results);
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect(&[DataField::SerialNumber]).await;

        miner.parse_serial_number(&data)
    }

    #[tokio::test]
    async fn serial_number_prefers_the_miner_serial() -> anyhow::Result<()> {
        let mut capabilities = Value::from_str(CAPABILITIES)?;
        capabilities["Miner Serial Number"] = Value::from("YDFTDXUBBJDJH067C");

        assert_eq!(
            serial_number_for(capabilities).await,
            Some("YDFTDXUBBJDJH067C".to_string())
        );

        Ok(())
    }

    #[tokio::test]
    async fn serial_number_falls_back_when_miner_serial_is_absent() -> anyhow::Result<()> {
        // The stock fixture carries no `Miner Serial Number` key at all.
        let capabilities = Value::from_str(CAPABILITIES)?;
        assert!(capabilities.get("Miner Serial Number").is_none());

        assert_eq!(
            serial_number_for(capabilities).await,
            Some("002000353232510933363631".to_string())
        );

        Ok(())
    }

    #[tokio::test]
    async fn serial_number_falls_back_when_miner_serial_is_null() -> anyhow::Result<()> {
        // The common case in the field: the key is present but explicitly null.
        let mut capabilities = Value::from_str(CAPABILITIES)?;
        capabilities["Miner Serial Number"] = Value::Null;

        assert_eq!(
            serial_number_for(capabilities).await,
            Some("002000353232510933363631".to_string())
        );

        Ok(())
    }
}

use std::{collections::HashMap, net::IpAddr, str::FromStr, time::Duration};

use anyhow;
use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigExtractor, ConfigField, ConfigLocation},
        pools::{PoolConfig, PoolGroupConfig},
    },
    data::{
        board::{BoardData, ChipData, MinerControlBoard},
        collector::{
            DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
        },
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        hashrate::{HashRate, HashRateUnit},
        message::{MessageSeverity, MinerMessage},
        miner::TuningTarget,
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_antminer::hardware::AntMinerControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use serde_json::Value;

use crate::{backends::v1::rpc::LUXMinerRPCAPI, firmware::LuxMinerFirmware};

mod rpc;

#[derive(Debug)]
pub struct LuxMinerV1 {
    pub ip: IpAddr,
    pub rpc: LUXMinerRPCAPI,
    pub device_info: DeviceInfo,
}

impl LuxMinerV1 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        LuxMinerV1 {
            ip,
            rpc: LUXMinerRPCAPI::new(ip),
            device_info: DeviceInfo::new(model, LuxMinerFirmware::default(), HashAlgorithm::SHA256),
        }
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

    fn parse_quota(value: Option<&Value>) -> u32 {
        value
            .and_then(Value::as_u64)
            .and_then(|quota| u32::try_from(quota).ok())
            .or_else(|| {
                value.and_then(Value::as_f64).and_then(|quota| {
                    if quota.is_finite() && quota >= 0.0 {
                        Some(quota.round() as u32)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_default()
    }

    fn parse_pool_config(config: &Value) -> Vec<PoolGroupConfig> {
        let groups = config
            .pointer("/groups/0/GROUPS")
            .or_else(|| config.pointer("/groups"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let pools = config
            .pointer("/pools/0/POOLS")
            .or_else(|| config.pointer("/pools"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut ordered_groups: Vec<(u64, PoolGroupConfig)> = groups
            .iter()
            .filter_map(|group| {
                let group_id = group.get("GROUP").and_then(Value::as_u64)?;
                Some((
                    group_id,
                    PoolGroupConfig {
                        name: group
                            .get("Name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        quota: Self::parse_quota(group.get("Quota")),
                        pools: vec![],
                    },
                ))
            })
            .collect();

        for pool in pools {
            let Some(url) = pool.get("URL").and_then(Value::as_str) else {
                continue;
            };
            if url.is_empty() {
                continue;
            }

            let group_id = pool
                .get("GROUP")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let pool_config = PoolConfig {
                url: PoolURL::from(url.to_string()),
                username: pool
                    .get("User")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                password: pool
                    .get("Password")
                    .or_else(|| pool.get("Pass"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            };

            if let Some((_, group)) = ordered_groups
                .iter_mut()
                .find(|(existing_id, _)| *existing_id == group_id)
            {
                group.pools.push(pool_config);
            } else {
                ordered_groups.push((
                    group_id,
                    PoolGroupConfig {
                        name: String::new(),
                        quota: Self::parse_quota(pool.get("Quota")),
                        pools: vec![pool_config],
                    },
                ));
            }
        }

        ordered_groups.sort_by_key(|(group_id, _)| *group_id);
        ordered_groups.into_iter().map(|(_, group)| group).collect()
    }
}

#[async_trait]
impl APIClient for LuxMinerV1 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            _ => Err(anyhow::anyhow!("Unsupported command type for LuxMiner API")),
        }
    }
}

impl GetConfigsLocations for LuxMinerV1 {
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        const RPC_GROUPS: MinerCommand = MinerCommand::RPC {
            command: "groups",
            parameters: None,
        };

        const RPC_POOLS: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };

        match data_field {
            ConfigField::Pools => vec![
                (
                    RPC_GROUPS,
                    ConfigExtractor {
                        func: get_by_pointer,
                        key: Some("/GROUPS"),
                        tag: Some("groups"),
                    },
                ),
                (
                    RPC_POOLS,
                    ConfigExtractor {
                        func: get_by_pointer,
                        key: Some("/POOLS"),
                        tag: Some("pools"),
                    },
                ),
            ],
            _ => vec![],
        }
    }
}

impl CollectConfigs for LuxMinerV1 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for LuxMinerV1 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const RPC_VERSION: MinerCommand = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };

        const RPC_STATS: MinerCommand = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };

        const RPC_SUMMARY: MinerCommand = MinerCommand::RPC {
            command: "summary",
            parameters: None,
        };

        const RPC_EVENTS: MinerCommand = MinerCommand::RPC {
            command: "events",
            parameters: None,
        };

        const RPC_POOLS: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };

        const RPC_CONFIG: MinerCommand = MinerCommand::RPC {
            command: "config",
            parameters: None,
        };

        const RPC_FANS: MinerCommand = MinerCommand::RPC {
            command: "fans",
            parameters: None,
        };

        const RPC_POWER: MinerCommand = MinerCommand::RPC {
            command: "power",
            parameters: None,
        };

        const RPC_PROFILES: MinerCommand = MinerCommand::RPC {
            command: "profiles",
            parameters: None,
        };

        const RPC_TEMPS: MinerCommand = MinerCommand::RPC {
            command: "temps",
            parameters: None,
        };

        const RPC_DEVS: MinerCommand = MinerCommand::RPC {
            command: "devs",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                RPC_CONFIG,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/CONFIG/0/MACAddr"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                RPC_FANS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/FANS"),
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
                RPC_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/VERSION/0/Miner"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                RPC_CONFIG,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/CONFIG/0/Hostname"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![
                (
                    MinerCommand::RPC {
                        command: "healthchipget",
                        parameters: Some(Value::String("0".to_string())),
                    },
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/CHIPS"),
                        tag: Some("CHIPS_0"),
                    },
                ),
                (
                    MinerCommand::RPC {
                        command: "healthchipget",
                        parameters: Some(Value::String("1".to_string())),
                    },
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/CHIPS"),
                        tag: Some("CHIPS_1"),
                    },
                ),
                (
                    MinerCommand::RPC {
                        command: "healthchipget",
                        parameters: Some(Value::String("2".to_string())),
                    },
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/CHIPS"),
                        tag: Some("CHIPS_2"),
                    },
                ),
                (
                    RPC_STATS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/STATS/1"),
                        tag: Some("STATS"),
                    },
                ),
                (
                    RPC_TEMPS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: None,
                    },
                ),
                (
                    MinerCommand::RPC {
                        command: "voltageget",
                        parameters: Some(Value::String("0".to_string())),
                    },
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/VOLTAGE"),
                        tag: Some("VOLTAGE_0"),
                    },
                ),
                (
                    MinerCommand::RPC {
                        command: "voltageget",
                        parameters: Some(Value::String("1".to_string())),
                    },
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/VOLTAGE"),
                        tag: Some("VOLTAGE_1"),
                    },
                ),
                (
                    MinerCommand::RPC {
                        command: "voltageget",
                        parameters: Some(Value::String("2".to_string())),
                    },
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/VOLTAGE"),
                        tag: Some("VOLTAGE_2"),
                    },
                ),
                (
                    MinerCommand::RPC {
                        command: "voltageget",
                        parameters: Some(Value::String("0".to_string())),
                    },
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/VOLTAGE"),
                        tag: Some("VOLTAGE_PSU"),
                    },
                ),
                (
                    RPC_TEMPS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("TEMPS"),
                    },
                ),
                (
                    RPC_DEVS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/DEVS"),
                        tag: Some("DEVS"),
                    },
                ),
            ],
            DataField::LightFlashing => vec![(
                RPC_CONFIG,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/CONFIG/0/RedLed"),
                    tag: None,
                },
            )],
            DataField::IsMining => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/GHS 5s"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                RPC_STATS,
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
                RPC_POWER,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/POWER/0/Watts"),
                    tag: None,
                },
            )],
            DataField::TuningTarget => vec![
                (
                    RPC_CONFIG,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/CONFIG/0/Profile"),
                        tag: Some("Profile"),
                    },
                ),
                (
                    RPC_PROFILES,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/PROFILES"),
                        tag: Some("Profiles"),
                    },
                ),
            ],
            DataField::SerialNumber => vec![(
                RPC_CONFIG,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/CONFIG/0/SerialNumber"),
                    tag: None,
                },
            )],
            DataField::Messages => vec![(
                RPC_EVENTS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/EVENTS"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                RPC_CONFIG,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/CONFIG/0/ControlBoardType"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/1/GHS 5s"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                RPC_DEVS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/DEVS"),
                    tag: None,
                },
            )],
            DataField::FluidTemperature => vec![(
                RPC_TEMPS,
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

impl GetIP for LuxMinerV1 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for LuxMinerV1 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for LuxMinerV1 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for LuxMinerV1 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s.to_uppercase()).ok())
    }
}

impl GetHostname for LuxMinerV1 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for LuxMinerV1 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFluidTemperature for LuxMinerV1 {
    fn parse_fluid_temperature(&self, data: &HashMap<DataField, Value>) -> Option<Temperature> {
        let temps_response = data.get(&DataField::FluidTemperature)?;

        let metadata = temps_response.get("METADATA")?.as_array()?;

        let mut inlet_field = None;
        let mut outlet_field = None;

        for item in metadata {
            if let Some(label) = item.get("Label").and_then(|v| v.as_str()) {
                for (key, _) in item.as_object()? {
                    if key != "Label" {
                        match label {
                            "Water Inlet" => inlet_field = Some(key.clone()),
                            "Water Outlet" => outlet_field = Some(key.clone()),
                            _ => {}
                        }
                        break;
                    }
                }
            }
        }

        let temps = temps_response.get("TEMPS")?.as_array()?;

        let mut inlet_temps = Vec::new();
        let mut outlet_temps = Vec::new();

        for temp_data in temps {
            if let Some(field) = &inlet_field
                && let Some(temp) = temp_data.get(field).and_then(|v| v.as_f64())
                && temp > 0.0
            {
                inlet_temps.push(temp);
            }

            if let Some(field) = &outlet_field
                && let Some(temp) = temp_data.get(field).and_then(|v| v.as_f64())
                && temp > 0.0
            {
                outlet_temps.push(temp);
            }
        }

        let avg_inlet = if !inlet_temps.is_empty() {
            Some(inlet_temps.iter().sum::<f64>() / inlet_temps.len() as f64)
        } else {
            None
        };

        let avg_outlet = if !outlet_temps.is_empty() {
            Some(outlet_temps.iter().sum::<f64>() / outlet_temps.len() as f64)
        } else {
            None
        };

        match (avg_inlet, avg_outlet) {
            (Some(inlet), Some(outlet)) => Some(Temperature::from_celsius((inlet + outlet) / 2.0)),
            (Some(inlet), None) => Some(Temperature::from_celsius(inlet)),
            (None, Some(outlet)) => Some(Temperature::from_celsius(outlet)),
            (None, None) => None,
        }
    }
}

impl GetFirmwareVersion for LuxMinerV1 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetHashboards for LuxMinerV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut boards: Vec<BoardData> = Vec::new();
        let board_count = self.device_info.hardware.boards.unwrap_or(3);
        for idx in 0..board_count {
            boards.push(BoardData {
                hashrate: None,
                position: idx,
                expected_hashrate: None,
                board_temperature: None,
                intake_temperature: None,
                outlet_temperature: None,
                expected_chips: self.device_info.hardware.chips,
                working_chips: None,
                serial_number: None,
                chips: vec![],
                voltage: None,
                frequency: None,
                tuned: Some(false),
                active: Some(false),
            });
        }

        if let Some(devs_data) = data
            .get(&DataField::Hashboards)
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("DEVS"))
            .and_then(|v| v.as_array())
        {
            for (idx, dev) in devs_data.iter().enumerate() {
                if let Some(dev_object) = dev.as_object() {
                    if let Some(serial_number) =
                        dev_object.get("SerialNumber").and_then(|v| v.as_str())
                    {
                        boards[idx].serial_number = Some(serial_number.to_string());
                    }

                    if let Some(expected_hashrate) =
                        dev_object.get("Nominal MHS").and_then(|v| v.as_f64())
                    {
                        boards[idx].expected_hashrate = Some(
                            HashRate {
                                value: expected_hashrate,
                                unit: HashRateUnit::MegaHash,
                                algo: String::from("SHA256"),
                            }
                            .as_unit(HashRateUnit::TeraHash),
                        );
                    }
                }
            }
        }

        if let Some(stats_data) = data
            .get(&DataField::Hashboards)
            .and_then(|v| v.get("STATS"))
        {
            for idx in 1..=board_count {
                let board_idx = (idx - 1) as usize;
                if let Some(hashrate) = stats_data
                    .get(format!("chain_rate{}", idx))
                    .and_then(|v| v.as_f64())
                    .map(|f| {
                        HashRate {
                            value: f,
                            unit: HashRateUnit::GigaHash,
                            algo: String::from("SHA256"),
                        }
                        .as_unit(HashRateUnit::TeraHash)
                    })
                {
                    boards[board_idx].hashrate = Some(hashrate);
                }

                if let Some(board_temp) = stats_data
                    .get(format!("temp_pcb{}", idx))
                    .and_then(|v| v.as_str())
                    .and_then(Self::parse_temp_string)
                {
                    boards[board_idx].board_temperature = Some(board_temp);
                }

                if let Some(chip_temp) = stats_data
                    .get(format!("temp_chip{}", idx))
                    .and_then(|v| v.as_str())
                    .and_then(Self::parse_temp_string)
                {
                    boards[board_idx].intake_temperature = Some(chip_temp);
                }

                if let Some(frequency) = stats_data
                    .get(format!("freq{}", idx))
                    .and_then(|v| v.as_u64())
                    .map(|f| Frequency::from_megahertz(f as f64))
                {
                    boards[board_idx].frequency = Some(frequency);
                }

                if let Some(working_chips) = stats_data
                    .get(format!("chain_acn{}", idx))
                    .and_then(|v| v.as_u64())
                {
                    boards[board_idx].working_chips = Some(working_chips as u16);
                }
            }
        }

        if let Some(temps_object) = data
            .get(&DataField::Hashboards)
            .and_then(|v| v.pointer("/TEMPS"))
            && let Some(temps_array) = temps_object.get("TEMPS").and_then(|v| v.as_array())
        {
            for temp_entry in temps_array {
                if let Some(board_id) = temp_entry.get("ID").and_then(|v| v.as_u64()) {
                    let board_idx = board_id as usize;
                    if board_idx < boards.len() {
                        let exhaust_temps: Vec<f64> = vec![
                            temp_entry.get("TopLeft").and_then(|v| v.as_f64()),
                            temp_entry.get("BottomLeft").and_then(|v| v.as_f64()),
                        ]
                        .into_iter()
                        .flatten()
                        .filter(|&t| t > 0.0)
                        .collect();

                        if !exhaust_temps.is_empty() {
                            let avg_exhaust =
                                exhaust_temps.iter().sum::<f64>() / exhaust_temps.len() as f64;
                            boards[board_idx].outlet_temperature =
                                Some(Temperature::from_celsius(avg_exhaust));
                        }

                        let intake_temps: Vec<f64> = vec![
                            temp_entry.get("TopRight").and_then(|v| v.as_f64()),
                            temp_entry.get("BottomRight").and_then(|v| v.as_f64()),
                        ]
                        .into_iter()
                        .flatten()
                        .filter(|&t| t > 0.0)
                        .collect();

                        if !intake_temps.is_empty() {
                            let avg_intake =
                                intake_temps.iter().sum::<f64>() / intake_temps.len() as f64;
                            boards[board_idx].intake_temperature =
                                Some(Temperature::from_celsius(avg_intake));
                        }
                    }
                }
            }
        }

        if let Some(voltage_data) = data.get(&DataField::Hashboards) {
            for (idx, tag) in (0..3).map(|i| (i, format!("/VOLTAGE_{}/0", i))) {
                if let Some(voltage_object) = voltage_data.pointer(&tag).and_then(|v| v.as_object())
                    && let Some(voltage) = voltage_object.get("Voltage").and_then(|v| v.as_f64())
                {
                    boards[idx].voltage = match voltage {
                        0.0 => voltage_data
                            .pointer("/VOLTAGE_PSU/0/Voltage")
                            .and_then(|v| v.as_f64())
                            .map(Voltage::from_volts), // If we cant read from each board, try the PSU
                        _ => Some(Voltage::from_volts(voltage)),
                    }
                }
            }
        }

        if let Some(chips_data) = data.get(&DataField::Hashboards) {
            for (idx, tag) in (0..3).map(|i| (i, format!("CHIPS_{}", i))) {
                if let Some(arr) = chips_data.get(&tag).and_then(|v| v.as_array()) {
                    boards[idx].chips = arr
                        .iter()
                        .filter_map(|v| v.as_object())
                        .map(|o| ChipData {
                            position: o.get("Chip").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
                            temperature: None,
                            hashrate: o.get("GHS 1m").and_then(|v| v.as_f64()).map(|hr| HashRate {
                                value: hr,
                                unit: HashRateUnit::GigaHash,
                                algo: "SHA256".into(),
                            }),
                            frequency: o
                                .get("Frequency")
                                .and_then(|v| v.as_f64())
                                .map(Frequency::from_megahertz),
                            tuned: o.get("Healthy").and_then(|v| v.as_str()).map(|s| s == "Y"),
                            working: o
                                .get("Healthy")
                                .and_then(|v| v.as_str())
                                .map(|s| s == "Y" || s == "Unknown"),
                            voltage: None,
                        })
                        .collect();
                }
            }
        }

        for b in &mut boards {
            if !b.chips.is_empty() {
                let total_hr: f64 = b
                    .chips
                    .iter()
                    .filter_map(|c| c.hashrate.as_ref())
                    .map(|h| h.value)
                    .sum();
                if total_hr > 0.0 {
                    b.hashrate = Some(
                        HashRate {
                            value: total_hr,
                            unit: HashRateUnit::GigaHash,
                            algo: "SHA256".into(),
                        }
                        .as_unit(HashRateUnit::TeraHash),
                    );
                }
                let freqs: Vec<f64> = b
                    .chips
                    .iter()
                    .filter_map(|c| c.frequency.as_ref())
                    .map(|f| f.as_megahertz())
                    .collect();
                if !freqs.is_empty() {
                    b.frequency = Some(Frequency::from_megahertz(
                        freqs.iter().sum::<f64>() / freqs.len() as f64,
                    ));
                }
                let active = b.working_chips.unwrap_or(0) > 0
                    || b.hashrate.as_ref().map(|h| h.value > 0.0).unwrap_or(false);
                b.active = Some(active);
                b.tuned = Some(active);
            }
        }

        boards
    }
}

impl GetHashrate for LuxMinerV1 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| {
            HashRate {
                value: f,
                unit: HashRateUnit::GigaHash,
                algo: String::from("SHA256"),
            }
            .as_unit(HashRateUnit::TeraHash)
        })
    }
}

impl GetExpectedHashrate for LuxMinerV1 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        let data = data
            .get(&DataField::ExpectedHashrate)
            .and_then(|v| v.as_array())?;
        let expected_boards = self.device_info.hardware.boards.unwrap_or(3);

        let mut expected_hashrate = 0.0;

        for idx in 0..expected_boards {
            if let Some(hashrate) = data
                .get(idx as usize)
                .and_then(|value| value.get("Nominal MHS"))
                .and_then(|v| v.as_f64())
            {
                expected_hashrate += hashrate;
            }
        }

        Some(
            HashRate {
                value: expected_hashrate,
                unit: HashRateUnit::MegaHash,
                algo: String::from("SHA256"),
            }
            .as_unit(HashRateUnit::TeraHash),
        )
    }
}

impl GetFans for LuxMinerV1 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        data.get(&DataField::Fans)
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .enumerate()
            .filter_map(|(idx, fan_info)| {
                let rpm = fan_info.get("RPM")?.as_f64()?;
                Some(FanData {
                    position: idx as i16,
                    rpm: Some(AngularVelocity::from_rpm(rpm)),
                })
            })
            .collect()
    }
}

impl GetLightFlashing for LuxMinerV1 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<String>(DataField::LightFlashing)
            .map(|s| s.to_lowercase() != "auto")
    }
}

impl GetUptime for LuxMinerV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetIsMining for LuxMinerV1 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        data.extract::<f64>(DataField::IsMining)
            .map(|hr| hr > 0.0)
            .unwrap_or(false)
    }
}

impl GetPools for LuxMinerV1 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let mut groups: Vec<(u64, PoolGroupData)> = Vec::new();

        if let Some(pools_data) = data.get(&DataField::Pools).and_then(Value::as_array) {
            for pool in pools_data {
                let group_id = pool
                    .get("GROUP")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                let pool_data = PoolData {
                    position: pool.get("POOL").and_then(Value::as_u64).map(|id| id as u16),
                    url: pool
                        .get("URL")
                        .and_then(|v| v.as_str())
                        .map(|s| PoolURL::from(s.to_string())),
                    user: pool.get("User").and_then(|v| v.as_str()).map(String::from),
                    alive: pool
                        .get("Status")
                        .and_then(|v| v.as_str())
                        .map(|s| s == "Alive"),
                    active: pool.get("Stratum Active").and_then(|v| v.as_bool()),
                    accepted_shares: pool.get("Accepted").and_then(|v| v.as_u64()),
                    rejected_shares: pool.get("Rejected").and_then(|v| v.as_u64()),
                };

                if let Some((_, group)) = groups.iter_mut().find(|(id, _)| *id == group_id) {
                    group.pools.push(pool_data);
                } else {
                    groups.push((
                        group_id,
                        PoolGroupData {
                            name: String::new(),
                            quota: Self::parse_quota(pool.get("Quota")),
                            pools: vec![pool_data],
                        },
                    ));
                }
            }
        }

        groups.sort_by_key(|(group_id, _)| *group_id);
        groups.into_iter().map(|(_, group)| group).collect()
    }
}

impl GetSerialNumber for LuxMinerV1 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        match data.extract::<String>(DataField::SerialNumber) {
            Some(s) if !s.is_empty() => Some(s),
            _ => None,
        }
    }
}

impl GetControlBoardVersion for LuxMinerV1 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| AntMinerControlBoard::parse(&s).map(|cb| cb.into()))
    }
}

impl GetWattage for LuxMinerV1 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}

impl GetTuningTarget for LuxMinerV1 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        let wattage_limit_data = data.get(&DataField::TuningTarget)?;
        let profile_name = wattage_limit_data.get("Profile")?.as_str()?;
        let profiles = wattage_limit_data.get("Profiles")?.as_array()?;

        let profile = profiles
            .iter()
            .find(|item| item.get("Profile Name").and_then(|v| v.as_str()) == Some(profile_name))?;

        let watts = profile.get("Watts")?.as_f64()?;

        Some(TuningTarget::Power(Power::from_watts(watts)))
    }
}

impl GetPsuFans for LuxMinerV1 {}

impl GetMessages for LuxMinerV1 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        data.get(&DataField::Messages)
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|event| {
                let code = event.get("Code")?.as_str()?;
                let description = event
                    .get("Description")
                    .and_then(|v| v.as_str())
                    .unwrap_or(code);

                let severity = if code.contains("FAILURE")
                    || code.contains("PANIC")
                    || (code.contains("SHUTDOWN") && !code.ends_with("USER"))
                {
                    MessageSeverity::Error
                } else if matches!(
                    code,
                    "MINER_TUNED"
                        | "MINER_TUNING"
                        | "IDLE"
                        | "SLEEP"
                        | "REBOOT_USER"
                        | "SHUTDOWN_USER"
                        | "REBOOT_SYSINIT"
                        | "HASH_ON_DISCONNECT"
                ) {
                    MessageSeverity::Info
                } else {
                    MessageSeverity::Warning
                };

                let message = match (
                    event.get("Target").and_then(|v| v.as_str()),
                    event.get("ID").and_then(|v| v.as_u64()),
                ) {
                    (Some(target), Some(id)) => {
                        format!("[{target} {id}] {description}")
                    }
                    (Some(target), None) => format!("[{target}] {description}"),
                    _ => description.to_string(),
                };

                Some(MinerMessage::new(0, 0, message, severity))
            })
            .collect()
    }
}

#[async_trait]
impl SetFaultLight for LuxMinerV1 {
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        let mode = match fault {
            true => "blink",
            false => "auto",
        };
        Ok(self.rpc.ledset("red", mode).await.is_ok())
    }
    fn supports_set_fault_light(&self) -> bool {
        true
    }
}

#[async_trait]
impl SetPowerLimit for LuxMinerV1 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsPoolsConfig for LuxMinerV1 {
    fn parse_pools_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<Vec<PoolGroupConfig>> {
        let Some(pools_data) = data.get(&ConfigField::Pools) else {
            return Ok(vec![]);
        };

        Ok(Self::parse_pool_config(pools_data))
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> anyhow::Result<bool> {
        let mut collector = self.get_config_collector();
        let current_pool_config_data = collector.collect(&[ConfigField::Pools]).await;
        let current_groups = current_pool_config_data
            .get(&ConfigField::Pools)
            .and_then(|config| {
                config
                    .pointer("/groups/0/GROUPS")
                    .or_else(|| config.pointer("/groups"))
                    .and_then(Value::as_array)
                    .cloned()
            })
            .unwrap_or_default();
        let mut group_ids: Vec<u32> = current_groups
            .iter()
            .filter_map(|group| group.get("GROUP").and_then(Value::as_u64))
            .map(|id| id as u32)
            .collect();
        group_ids.sort_unstable_by(|a, b| b.cmp(a));

        for group_id in group_ids {
            self.rpc.removegroup(group_id).await?;
        }

        for group in &config {
            self.rpc.addgroup(&group.name, group.quota).await?;
        }

        for (group_id, group) in config.iter().enumerate() {
            for pool in &group.pools {
                self.rpc
                    .addpool(
                        &pool.url.to_string(),
                        &pool.username,
                        &pool.password,
                        Some(&group_id.to_string()),
                    )
                    .await?;
            }
        }

        Ok(true)
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for LuxMinerV1 {
    async fn restart(&self) -> anyhow::Result<bool> {
        // Miners often reboot before responding — any error (timeout,
        // connection reset, broken pipe) likely means the miner is rebooting.
        let _ = self.rpc.reboot_device().await;
        Ok(true)
    }
    fn supports_restart(&self) -> bool {
        true
    }
}

#[async_trait]
impl Pause for LuxMinerV1 {
    async fn pause(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        Ok(self.rpc.sleep().await.is_ok())
    }
    fn supports_pause(&self) -> bool {
        true
    }
}

#[async_trait]
impl Resume for LuxMinerV1 {
    async fn resume(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        Ok(self.rpc.wakeup().await.is_ok())
    }
    fn supports_resume(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsScalingConfig for LuxMinerV1 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for LuxMinerV1 {
    fn supports_upgrade_firmware(&self) -> bool {
        false
    }
}

impl HasAuth for LuxMinerV1 {}
impl HasDefaultAuth for LuxMinerV1 {}

#[async_trait]
impl SupportsTuningConfig for LuxMinerV1 {
    fn supports_tuning_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsFanConfig for LuxMinerV1 {
    fn supports_fan_config(&self) -> bool {
        false
    }
}
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use serde_json::json;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
    };

    use asic_rs_core::test::api::MockAPIClient;
    use asic_rs_makes_antminer::models::AntMinerModel;

    use super::*;
    use crate::test::json::v1::{
        CONFIG, DEVS, EVENTS, FANS, HEALTHCHIPGET_0, HEALTHCHIPGET_1, HEALTHCHIPGET_2, POOLS,
        POWER, PROFILES, STATS, SUMMARY, TEMPS, VERSION, VOLTAGEGET_0, VOLTAGEGET_1, VOLTAGEGET_2,
    };

    #[tokio::test]

    async fn test_luxminer_v1() -> anyhow::Result<()> {
        let miner = LuxMinerV1::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19KPro);

        let mut results = HashMap::new();
        let version_cmd = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };

        let stats_cmd = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };

        let summary_cmd = MinerCommand::RPC {
            command: "summary",
            parameters: None,
        };

        let pools_cmd = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };

        let config_cmd = MinerCommand::RPC {
            command: "config",
            parameters: None,
        };

        let fans_cmd = MinerCommand::RPC {
            command: "fans",
            parameters: None,
        };

        let power_cmd = MinerCommand::RPC {
            command: "power",
            parameters: None,
        };

        let profiles_cmd = MinerCommand::RPC {
            command: "profiles",
            parameters: None,
        };

        let temps_cmd = MinerCommand::RPC {
            command: "temps",
            parameters: None,
        };

        let devs_cmd = MinerCommand::RPC {
            command: "devs",
            parameters: None,
        };

        results.insert(version_cmd, Value::from_str(VERSION)?);
        results.insert(stats_cmd, Value::from_str(STATS)?);
        results.insert(summary_cmd, Value::from_str(SUMMARY)?);
        results.insert(pools_cmd, Value::from_str(POOLS)?);
        results.insert(config_cmd, Value::from_str(CONFIG)?);
        results.insert(fans_cmd, Value::from_str(FANS)?);
        results.insert(power_cmd, Value::from_str(POWER)?);
        results.insert(profiles_cmd, Value::from_str(PROFILES)?);
        results.insert(temps_cmd, Value::from_str(TEMPS)?);
        results.insert(devs_cmd, Value::from_str(DEVS)?);
        results.insert(
            MinerCommand::RPC {
                command: "events",
                parameters: None,
            },
            Value::from_str(EVENTS)?,
        );

        results.insert(
            MinerCommand::RPC {
                command: "voltageget",
                parameters: Some(Value::String("0".to_string())),
            },
            Value::from_str(VOLTAGEGET_0)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "voltageget",
                parameters: Some(Value::String("1".to_string())),
            },
            Value::from_str(VOLTAGEGET_1)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "voltageget",
                parameters: Some(Value::String("2".to_string())),
            },
            Value::from_str(VOLTAGEGET_2)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "healthchipget",
                parameters: Some(Value::String("0".to_string())),
            },
            Value::from_str(HEALTHCHIPGET_0)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "healthchipget",
                parameters: Some(Value::String("1".to_string())),
            },
            Value::from_str(HEALTHCHIPGET_1)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "healthchipget",
                parameters: Some(Value::String("2".to_string())),
            },
            Value::from_str(HEALTHCHIPGET_2)?,
        );

        let mock_api = MockAPIClient::new(results);

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;

        let miner_data = miner.parse_data(data);

        assert_eq!(
            miner_data.mac,
            Some(MacAddr::from_str("62:f7:5e:b7:10:46")?)
        );
        assert_eq!(
            miner_data.serial_number,
            Some("JYZZB0UBDJABF06RB".to_string())
        );
        assert_eq!(miner_data.hostname, Some("UrlacherS19k".to_string()));
        assert_eq!(miner_data.api_version, Some("3.7".to_string()));
        assert_eq!(
            miner_data.firmware_version,
            Some("2025.4.8.220305".to_string())
        );
        assert_eq!(
            miner_data.control_board_version,
            Some(AntMinerControlBoard::CVITek.into())
        );
        assert_eq!(miner_data.wattage, Some(Power::from_watts(1051f64)));
        assert_eq!(
            miner_data.tuning_target,
            Some(TuningTarget::Power(Power::from_watts(1188f64)))
        );
        assert_eq!(miner_data.fans.len(), 4);
        assert_eq!(miner_data.hashboards[0].chips.len(), 77);
        assert_eq!(miner_data.pools.len(), 2);
        assert_eq!(miner_data.pools[0].len(), 2);
        assert_eq!(miner_data.pools[1].len(), 2);

        assert_eq!(miner_data.messages.len(), 2);
        assert_eq!(miner_data.messages[0].severity, MessageSeverity::Warning);
        assert_eq!(
            miner_data.messages[0].message,
            "[MINER] Miner shutdown due to NOPIC protection."
        );
        assert_eq!(miner_data.messages[1].severity, MessageSeverity::Warning);
        assert_eq!(
            miner_data.messages[1].message,
            "[BOARD 0] Board reboot due to it not hashing."
        );

        Ok(())
    }

    #[test]
    fn test_parse_lux_pools_config_with_empty_group() {
        let config = json!({
            "groups": [
                { "GROUP": 0, "Name": "default", "Quota": 80.0 },
                { "GROUP": 1, "Name": "testTWO", "Quota": 1.0 },
                { "GROUP": 2, "Name": "empty-group", "Quota": 5.0 }
            ],
            "pools": [
                {
                    "POOL": 0,
                    "GROUP": 0,
                    "Quota": 80.0,
                    "URL": "stratum+tcp://fuzz-7a3.invalid:17000",
                    "User": "rand-worker-a9"
                },
                {
                    "POOL": 1,
                    "GROUP": 1,
                    "Quota": 1.0,
                    "URL": "stratum+tcp://fuzz-c42.invalid:27001",
                    "User": "rand-worker-b4"
                }
            ]
        });

        let groups = LuxMinerV1::parse_pool_config(&config);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].name, "default");
        assert_eq!(groups[0].quota, 80);
        assert_eq!(groups[0].pools.len(), 1);
        assert_eq!(groups[1].name, "testTWO");
        assert_eq!(groups[1].quota, 1);
        assert_eq!(groups[1].pools.len(), 1);
        assert_eq!(groups[2].name, "empty-group");
        assert_eq!(groups[2].quota, 5);
        assert!(groups[2].pools.is_empty());
    }

    #[test]
    fn test_parse_lux_pools_config_nested_response_with_password() {
        let config = json!({
            "groups": [
                {
                    "GROUPS": [
                        { "GROUP": 0, "Name": "default", "Quota": 80.0 },
                        { "GROUP": 1, "Name": "testTWO", "Quota": 20.0 }
                    ]
                }
            ],
            "pools": [
                {
                    "POOLS": [
                        {
                            "POOL": 0,
                            "GROUP": 0,
                            "Quota": 80.0,
                            "URL": "stratum+tcp://mock-alpha.invalid:31000",
                            "User": "synthetic-user-0",
                            "Password": "secret0"
                        },
                        {
                            "POOL": 1,
                            "GROUP": 1,
                            "Quota": 20.0,
                            "URL": "stratum+tcp://mock-beta.invalid:31001",
                            "User": "synthetic-user-1",
                            "Pass": "secret1"
                        }
                    ]
                }
            ],
            "poolopts": []
        });

        let groups = LuxMinerV1::parse_pool_config(&config);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].pools[0].password, "secret0");
        assert_eq!(groups[1].pools[0].password, "secret1");
    }

    #[tokio::test]
    async fn test_set_lux_pools_config_allows_empty_groups() -> anyhow::Result<()> {
        let requests = Arc::new(Mutex::new(Vec::<(String, Option<String>)>::new()));
        let server_requests = Arc::clone(&requests);
        let listener = TcpListener::bind("127.0.0.1:4028").await?;

        let server = tokio::spawn(async move {
            for request_idx in 0..4 {
                let (socket, _) =
                    tokio::time::timeout(std::time::Duration::from_secs(2), listener.accept())
                        .await
                        .map_err(|_| {
                            anyhow::anyhow!("timed out waiting for request {}", request_idx + 1)
                        })??;
                let (reader, mut writer) = socket.into_split();
                let mut reader = BufReader::new(reader);
                let mut line = String::new();
                reader.read_line(&mut line).await?;

                let request: Value = serde_json::from_str(line.trim_end())?;
                let command = request
                    .get("command")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let parameter = request
                    .get("parameter")
                    .and_then(Value::as_str)
                    .map(str::to_string);

                server_requests
                    .lock()
                    .unwrap()
                    .push((command.clone(), parameter));

                let response = match command.as_str() {
                    "groups" => json!({
                        "GROUPS": [
                            { "GROUP": 0, "Name": "default", "Quota": 1.0 },
                            { "GROUP": 1, "Name": "backup", "Quota": 1.0 }
                        ],
                        "STATUS": [{ "STATUS": "S", "Msg": "ok" }]
                    }),
                    "pools" => json!({
                        "POOLS": [],
                        "STATUS": [{ "STATUS": "S", "Msg": "ok" }]
                    }),
                    "removegroup" => json!({
                        "STATUS": [{ "STATUS": "S", "Msg": "ok" }]
                    }),
                    other => anyhow::bail!("unexpected command: {other}"),
                };

                writer
                    .write_all(format!("{}\n", response).as_bytes())
                    .await?;
            }

            anyhow::Ok(())
        });

        let miner = LuxMinerV1::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19KPro);
        assert!(miner.set_pools_config(vec![]).await?);

        let server_result = tokio::time::timeout(std::time::Duration::from_secs(5), server)
            .await
            .map_err(|_| anyhow::anyhow!("mock rpc server did not finish in time"))?;
        server_result??;

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 4);

        let mut initial_requests = requests[..2].to_vec();
        initial_requests.sort();
        assert_eq!(
            initial_requests,
            vec![("groups".to_string(), None), ("pools".to_string(), None),]
        );
        assert_eq!(
            requests[2],
            ("removegroup".to_string(), Some("1".to_string()))
        );
        assert_eq!(
            requests[3],
            ("removegroup".to_string(), Some("0".to_string()))
        );

        Ok(())
    }
}

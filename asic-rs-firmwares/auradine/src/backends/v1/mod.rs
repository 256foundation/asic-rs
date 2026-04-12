use std::{
    collections::{BTreeMap, HashMap},
    net::IpAddr,
    str::FromStr,
    time::Duration,
};

use anyhow;
use asic_rs_core::{
    config::{
        collector::{
            ConfigCollector, ConfigExtractor, ConfigField, ConfigLocation,
            get_by_pointer as get_config_by_pointer,
        },
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
        miner::{MiningMode, TuningTarget},
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_auradine::hardware::AuradineControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use reqwest::Method;
use serde_json::{Value, json};

use crate::firmware::AuradineFirmware;

mod rpc;
mod status;
pub(crate) mod web;

use rpc::AuradineRPCAPI;
use web::AuradineWebAPI;

#[derive(Debug)]
pub struct AuradineV1 {
    ip: IpAddr,
    rpc: AuradineRPCAPI,
    web: AuradineWebAPI,
    device_info: DeviceInfo,
}

impl AuradineV1 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        let auth = Self::default_auth();
        Self {
            ip,
            rpc: AuradineRPCAPI::new(ip),
            web: AuradineWebAPI::new(ip, auth),
            device_info: DeviceInfo::new(model, AuradineFirmware::default(), HashAlgorithm::SHA256),
        }
    }

    fn parse_number_from_units(input: &str) -> Option<f64> {
        let cleaned: String = input
            .trim()
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
            .collect();
        if cleaned.is_empty() {
            return None;
        }
        cleaned.parse::<f64>().ok()
    }

    fn led_status_severity(code: u64) -> MessageSeverity {
        match code {
            2 | 3 => MessageSeverity::Info,
            _ => MessageSeverity::Warning,
        }
    }

    fn build_update_pools_payload(config: &[PoolGroupConfig]) -> anyhow::Result<Vec<Value>> {
        let pools: Vec<Value> = config
            .iter()
            .flat_map(|group| group.pools.iter())
            .take(3)
            .map(|pool| {
                let password = if pool.password.is_empty() {
                    "x"
                } else {
                    pool.password.as_str()
                };

                Ok(json!({
                    "url": pool.url.to_string(),
                    "user": pool.username,
                    "pass": password,
                }))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        anyhow::ensure!(!pools.is_empty(), "updatepools requires at least one pool");

        Ok(pools)
    }
}

#[async_trait]
impl APIClient for AuradineV1 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!("Unsupported command type for Auradine API")),
        }
    }
}

impl GetConfigsLocations for AuradineV1 {
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        const RPC_POOLS: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };

        match data_field {
            ConfigField::Pools => vec![(
                RPC_POOLS,
                ConfigExtractor {
                    func: get_config_by_pointer,
                    key: Some("/POOLS"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl CollectConfigs for AuradineV1 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for AuradineV1 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const RPC_VERSION: MinerCommand = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };
        const RPC_SUMMARY: MinerCommand = MinerCommand::RPC {
            command: "summary",
            parameters: None,
        };
        const RPC_DEVS: MinerCommand = MinerCommand::RPC {
            command: "devs",
            parameters: None,
        };
        const RPC_DEVDETAILS: MinerCommand = MinerCommand::RPC {
            command: "devdetails",
            parameters: None,
        };
        const RPC_ASCCOUNT: MinerCommand = MinerCommand::RPC {
            command: "asccount",
            parameters: None,
        };
        const RPC_POOLS: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };
        const WEB_IPREPORT: MinerCommand = MinerCommand::WebAPI {
            command: "ipreport",
            parameters: None,
        };
        const WEB_FAN: MinerCommand = MinerCommand::WebAPI {
            command: "fan",
            parameters: None,
        };
        const WEB_LED: MinerCommand = MinerCommand::WebAPI {
            command: "led",
            parameters: None,
        };
        const WEB_MODE: MinerCommand = MinerCommand::WebAPI {
            command: "mode",
            parameters: None,
        };
        const WEB_PSU: MinerCommand = MinerCommand::WebAPI {
            command: "psu",
            parameters: None,
        };
        const WEB_TEMPERATURE: MinerCommand = MinerCommand::WebAPI {
            command: "temperature",
            parameters: None,
        };
        const WEB_VOLTAGE: MinerCommand = MinerCommand::WebAPI {
            command: "voltage",
            parameters: None,
        };
        const WEB_FREQUENCY: MinerCommand = MinerCommand::WebAPI {
            command: "frequency",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                WEB_IPREPORT,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/IPReport/0/mac"),
                    tag: None,
                },
            )],
            DataField::SerialNumber => vec![(
                WEB_IPREPORT,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/IPReport/0/SerialNo"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                WEB_IPREPORT,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/IPReport/0/hostname"),
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
            DataField::FirmwareVersion => vec![
                (
                    RPC_VERSION,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/VERSION/0/GCMiner"),
                        tag: None,
                    },
                ),
                (
                    WEB_IPREPORT,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/IPReport/0/version"),
                        tag: None,
                    },
                ),
            ],
            DataField::ControlBoardVersion => vec![
                (
                    WEB_IPREPORT,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/IPReport/0/InternalType"),
                        tag: Some("internal_type"),
                    },
                ),
                (
                    RPC_DEVDETAILS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/DEVDETAILS/0/Driver"),
                        tag: Some("driver"),
                    },
                ),
            ],
            DataField::Hashboards => vec![
                (
                    RPC_DEVS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/DEVS"),
                        tag: Some("devs"),
                    },
                ),
                (
                    RPC_ASCCOUNT,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/ASCCount/0"),
                        tag: Some("asccount"),
                    },
                ),
                (
                    WEB_IPREPORT,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/IPReport/0/HBSerialNo"),
                        tag: Some("serials"),
                    },
                ),
                (
                    WEB_TEMPERATURE,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/Temperature"),
                        tag: Some("temperature"),
                    },
                ),
                (
                    WEB_VOLTAGE,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/Voltage"),
                        tag: Some("voltage"),
                    },
                ),
                (
                    WEB_FREQUENCY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/Frequency"),
                        tag: Some("frequency"),
                    },
                ),
            ],
            DataField::Hashrate => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/MHS 5s"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![
                (
                    RPC_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/SUMMARY/0/ThsThrottle"),
                        tag: None,
                    },
                ),
                (
                    WEB_MODE,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/Mode/0/Ths"),
                        tag: None,
                    },
                ),
            ],
            DataField::Fans => vec![(
                WEB_FAN,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Fan"),
                    tag: None,
                },
            )],
            DataField::PsuFans => vec![(
                WEB_PSU,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/PSU/0"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![
                (
                    RPC_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/SUMMARY/0/Wattage"),
                        tag: None,
                    },
                ),
                (
                    WEB_PSU,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/PSU/0/PowerIn"),
                        tag: None,
                    },
                ),
            ],
            DataField::TuningTarget => vec![
                (
                    RPC_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/SUMMARY/0/ThsThrottle"),
                        tag: None,
                    },
                ),
                (
                    WEB_MODE,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/Mode/0"),
                        tag: None,
                    },
                ),
            ],
            DataField::LightFlashing => vec![(
                WEB_LED,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/LED/0/Code"),
                    tag: None,
                },
            )],
            DataField::Messages => vec![(
                WEB_LED,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/LED/0"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/Elapsed"),
                    tag: None,
                },
            )],
            DataField::IsMining => vec![
                (
                    RPC_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/SUMMARY/0/MHS 5s"),
                        tag: Some("hashrate"),
                    },
                ),
                (
                    WEB_MODE,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/Mode/0/Sleep"),
                        tag: Some("sleep"),
                    },
                ),
            ],
            DataField::Pools => vec![(
                RPC_POOLS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/POOLS"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for AuradineV1 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for AuradineV1 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for AuradineV1 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for AuradineV1 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|mac| MacAddr::from_str(&mac).ok())
    }
}

impl GetSerialNumber for AuradineV1 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::SerialNumber)
    }
}

impl GetHostname for AuradineV1 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for AuradineV1 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for AuradineV1 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetControlBoardVersion for AuradineV1 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        if let Some(control_board_data) = data.get(&DataField::ControlBoardVersion) {
            if let Some(internal_type) = control_board_data
                .get("internal_type")
                .and_then(Value::as_str)
            {
                if let Some(parsed) = AuradineControlBoard::parse(internal_type) {
                    return Some(parsed.into());
                }
                return Some(MinerControlBoard::unknown(internal_type.to_string()));
            }

            if let Some(driver) = control_board_data.get("driver").and_then(Value::as_str) {
                return Some(MinerControlBoard::unknown(driver.to_string()));
            }
        }

        let cb = data.extract::<String>(DataField::ControlBoardVersion)?;
        AuradineControlBoard::parse(&cb)
            .map(MinerControlBoard::from)
            .or_else(|| Some(MinerControlBoard::unknown(cb)))
    }
}

impl GetHashboards for AuradineV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let Some(hashboards_data) = data.get(&DataField::Hashboards) else {
            return vec![];
        };

        let devs_data = hashboards_data
            .get("devs")
            .and_then(Value::as_array)
            .or_else(|| hashboards_data.as_array());
        let serials_data = hashboards_data.get("serials").and_then(Value::as_array);

        let chips_per_board = hashboards_data.get("asccount").and_then(|count| {
            let total_boards = count.get("BoardCount").and_then(Value::as_u64)?;
            let total_chips = count.get("AsicCount").and_then(Value::as_u64)?;
            if total_boards == 0 {
                return None;
            }
            u16::try_from(total_chips / total_boards).ok()
        });

        let expected_chips = chips_per_board.or(self.device_info.hardware.chips);
        let expected_boards = hashboards_data
            .get("asccount")
            .and_then(|count| count.get("BoardCount"))
            .and_then(Value::as_u64)
            .and_then(|count| u8::try_from(count).ok())
            .or(self.device_info.hardware.boards);

        let mut board_count = expected_boards.unwrap_or(0);

        if let Some(devs) = devs_data {
            for (idx, board) in devs.iter().enumerate() {
                let board_id = board
                    .get("ID")
                    .and_then(Value::as_u64)
                    .or_else(|| board.get("DEV").and_then(Value::as_u64))
                    .or_else(|| u64::try_from(idx + 1).ok());

                if let Some(board_id) = board_id
                    && board_id > 0
                    && let Ok(count) = u8::try_from(board_id)
                {
                    board_count = board_count.max(count);
                }
            }
        }

        if let Some(serials_data) = serials_data {
            let detected = u8::try_from(serials_data.len()).unwrap_or(u8::MAX);
            board_count = board_count.max(detected);
        }

        for key in ["temperature", "voltage", "frequency"] {
            if let Some(source_boards) = hashboards_data.get(key).and_then(Value::as_array) {
                for board in source_boards {
                    if let Some(board_id) = board.get("ID").and_then(Value::as_u64)
                        && board_id > 0
                        && let Ok(count) = u8::try_from(board_id)
                    {
                        board_count = board_count.max(count);
                    }
                }
            }
        }

        let mut hashboards: Vec<BoardData> = Vec::new();
        for idx in 0..board_count {
            hashboards.push(BoardData {
                hashrate: None,
                position: idx,
                expected_hashrate: None,
                board_temperature: None,
                intake_temperature: None,
                outlet_temperature: None,
                expected_chips,
                working_chips: None,
                serial_number: None,
                chips: vec![],
                voltage: None,
                frequency: None,
                tuned: None,
                active: None,
            });
        }

        if let Some(devs) = devs_data {
            for (idx, board) in devs.iter().enumerate() {
                let board_id = board
                    .get("ID")
                    .and_then(Value::as_u64)
                    .or_else(|| board.get("DEV").and_then(Value::as_u64))
                    .or_else(|| u64::try_from(idx + 1).ok())
                    .unwrap_or_default();

                let Some(position) = board_id
                    .checked_sub(1)
                    .and_then(|position| u8::try_from(position).ok())
                else {
                    continue;
                };
                let Some(hashboard) = hashboards.get_mut(position as usize) else {
                    continue;
                };

                hashboard.board_temperature = board
                    .get("Temperature")
                    .and_then(Value::as_f64)
                    .map(Temperature::from_celsius);
                hashboard.intake_temperature = hashboard.board_temperature;
                hashboard.outlet_temperature = hashboard.board_temperature;
                hashboard.hashrate =
                    board
                        .get("MHS 5s")
                        .and_then(Value::as_f64)
                        .map(|value| HashRate {
                            value,
                            unit: HashRateUnit::MegaHash,
                            algo: String::from("SHA256"),
                        });
                hashboard.serial_number = serials_data
                    .and_then(|sns| sns.get(position as usize))
                    .and_then(Value::as_str)
                    .map(String::from);
                hashboard.active = board.get("Enabled").and_then(Value::as_bool);
                hashboard.tuned = hashboard.active;
            }
        }

        if let Some(serials_data) = serials_data {
            for (idx, serial) in serials_data.iter().enumerate() {
                let Some(serial_str) = serial.as_str() else {
                    continue;
                };
                let Some(hashboard) = hashboards.get_mut(idx) else {
                    continue;
                };
                if hashboard.serial_number.is_none() {
                    hashboard.serial_number = Some(serial_str.to_string());
                }
            }
        }

        let mut chip_maps: Vec<BTreeMap<u16, ChipData>> = (0..usize::from(board_count))
            .map(|_| BTreeMap::new())
            .collect();

        if let Some(temperature_boards) =
            hashboards_data.get("temperature").and_then(Value::as_array)
        {
            for temp_board in temperature_boards {
                let Some(id) = temp_board.get("ID").and_then(Value::as_u64) else {
                    continue;
                };
                let Some(position) = id
                    .checked_sub(1)
                    .and_then(|position| u8::try_from(position).ok())
                else {
                    continue;
                };
                let Some(hashboard) = hashboards.get_mut(position as usize) else {
                    continue;
                };
                hashboard.active.get_or_insert(true);

                if let Some(sensor_values) = temp_board
                    .get("BoardTemp")
                    .and_then(Value::as_array)
                    .map(|sensors| {
                        sensors
                            .iter()
                            .filter_map(|sensor| sensor.get("Temperature").and_then(Value::as_f64))
                            .collect::<Vec<f64>>()
                    })
                    && !sensor_values.is_empty()
                {
                    let min_temp = sensor_values.iter().copied().min_by(|a, b| a.total_cmp(b));
                    let max_temp = sensor_values.iter().copied().max_by(|a, b| a.total_cmp(b));
                    if let Some(temp) = min_temp {
                        hashboard.intake_temperature = Some(Temperature::from_celsius(temp));
                    }
                    if let Some(temp) = max_temp {
                        hashboard.outlet_temperature = Some(Temperature::from_celsius(temp));
                    }
                    if hashboard.board_temperature.is_none() {
                        let avg = sensor_values.iter().sum::<f64>() / sensor_values.len() as f64;
                        hashboard.board_temperature = Some(Temperature::from_celsius(avg));
                    }
                }

                let Some(chip_temps) = temp_board.get("ChipTemp").and_then(Value::as_array) else {
                    continue;
                };

                let Some(chip_map) = chip_maps.get_mut(position as usize) else {
                    continue;
                };

                for chip in chip_temps {
                    let Some(chip_id) = chip
                        .get("ID")
                        .and_then(Value::as_u64)
                        .and_then(|chip_id| u16::try_from(chip_id).ok())
                    else {
                        continue;
                    };
                    let Some(temp) = chip.get("Temperature").and_then(Value::as_f64) else {
                        continue;
                    };

                    let chip_data = chip_map.entry(chip_id).or_insert_with(|| ChipData {
                        position: chip_id,
                        ..Default::default()
                    });
                    chip_data.temperature = Some(Temperature::from_celsius(temp));
                    chip_data.working = Some(temp > 0.0);
                }
            }
        }

        if let Some(voltage_boards) = hashboards_data.get("voltage").and_then(Value::as_array) {
            for voltage_board in voltage_boards {
                let Some(id) = voltage_board.get("ID").and_then(Value::as_u64) else {
                    continue;
                };
                let Some(position) = id
                    .checked_sub(1)
                    .and_then(|position| u8::try_from(position).ok())
                else {
                    continue;
                };
                let Some(chip_voltages) =
                    voltage_board.get("ChipVoltage").and_then(Value::as_array)
                else {
                    continue;
                };

                let Some(chip_map) = chip_maps.get_mut(position as usize) else {
                    continue;
                };

                for chip in chip_voltages {
                    let Some(chip_id) = chip
                        .get("ID")
                        .and_then(Value::as_u64)
                        .and_then(|chip_id| u16::try_from(chip_id).ok())
                    else {
                        continue;
                    };
                    let Some(voltage) = chip.get("Voltage").and_then(Value::as_f64) else {
                        continue;
                    };

                    let chip_data = chip_map.entry(chip_id).or_insert_with(|| ChipData {
                        position: chip_id,
                        ..Default::default()
                    });
                    chip_data.voltage = Some(Voltage::from_volts(voltage));
                    if voltage > 0.0 {
                        chip_data.working = Some(true);
                    }
                }
            }
        }

        if let Some(frequency_boards) = hashboards_data.get("frequency").and_then(Value::as_array) {
            for frequency_board in frequency_boards {
                let Some(id) = frequency_board.get("ID").and_then(Value::as_u64) else {
                    continue;
                };
                let Some(position) = id
                    .checked_sub(1)
                    .and_then(|position| u8::try_from(position).ok())
                else {
                    continue;
                };
                let Some(chip_frequencies) = frequency_board
                    .get("ChipFrequency")
                    .and_then(Value::as_array)
                else {
                    continue;
                };

                let Some(chip_map) = chip_maps.get_mut(position as usize) else {
                    continue;
                };

                for chip in chip_frequencies {
                    let Some(chip_id) = chip
                        .get("ID")
                        .and_then(Value::as_u64)
                        .and_then(|chip_id| u16::try_from(chip_id).ok())
                    else {
                        continue;
                    };
                    let Some(frequency) = chip.get("Frequency").and_then(Value::as_f64) else {
                        continue;
                    };

                    let chip_data = chip_map.entry(chip_id).or_insert_with(|| ChipData {
                        position: chip_id,
                        ..Default::default()
                    });
                    chip_data.frequency = Some(Frequency::from_megahertz(frequency));
                    if frequency > 0.0 {
                        chip_data.working = Some(true);
                    }
                }
            }
        }

        for (idx, hashboard) in hashboards.iter_mut().enumerate() {
            let Some(chips_map) = chip_maps.get_mut(idx) else {
                continue;
            };
            if chips_map.is_empty() {
                continue;
            }

            hashboard.chips = std::mem::take(chips_map).into_values().collect();

            if hashboard.expected_chips.is_none() {
                hashboard.expected_chips = u16::try_from(hashboard.chips.len()).ok();
            }

            let working_chip_count = hashboard
                .chips
                .iter()
                .filter(|chip| {
                    chip.working.unwrap_or_else(|| {
                        chip.temperature
                            .map(|t| t.as_celsius() > 0.0)
                            .unwrap_or(false)
                            || chip.voltage.map(|v| v.as_volts() > 0.0).unwrap_or(false)
                            || chip
                                .frequency
                                .map(|f| f.as_megahertz() > 0.0)
                                .unwrap_or(false)
                    })
                })
                .count() as u16;

            hashboard.working_chips = if let Some(expected_chips) = hashboard.expected_chips {
                if !hashboard.chips.is_empty()
                    && hashboard.chips.len() < usize::from(expected_chips)
                {
                    Some(match hashboard.active {
                        Some(false) => 0,
                        _ => expected_chips,
                    })
                } else {
                    Some(working_chip_count)
                }
            } else {
                Some(working_chip_count)
            };

            let voltages: Vec<f64> = hashboard
                .chips
                .iter()
                .filter_map(|chip| chip.voltage.map(|v| v.as_volts()))
                .collect();
            if !voltages.is_empty() {
                let avg_voltage = voltages.iter().sum::<f64>() / voltages.len() as f64;
                hashboard.voltage = Some(Voltage::from_volts(avg_voltage));
            }

            let frequencies: Vec<f64> = hashboard
                .chips
                .iter()
                .filter_map(|chip| chip.frequency.map(|f| f.as_megahertz()))
                .collect();
            if !frequencies.is_empty() {
                let avg_frequency = frequencies.iter().sum::<f64>() / frequencies.len() as f64;
                hashboard.frequency = Some(Frequency::from_megahertz(avg_frequency));
            }
        }

        hashboards
    }
}

impl GetHashrate for AuradineV1 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |value| HashRate {
            value,
            unit: HashRateUnit::MegaHash,
            algo: String::from("SHA256"),
        })
    }
}

impl GetExpectedHashrate for AuradineV1 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |value| HashRate {
            value,
            unit: HashRateUnit::TeraHash,
            algo: String::from("SHA256"),
        })
    }
}

impl GetFans for AuradineV1 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let Some(fans) = data.get(&DataField::Fans).and_then(Value::as_array) else {
            return vec![];
        };

        fans.iter()
            .enumerate()
            .map(|(idx, fan)| {
                let position = fan
                    .get("ID")
                    .and_then(Value::as_i64)
                    .map(|v| v as i16 - 1)
                    .unwrap_or(idx as i16);
                let rpm = fan
                    .get("Speed")
                    .and_then(Value::as_f64)
                    .map(AngularVelocity::from_rpm);
                FanData { position, rpm }
            })
            .collect()
    }
}

impl GetPsuFans for AuradineV1 {
    fn parse_psu_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let Some(psu) = data.get(&DataField::PsuFans).and_then(Value::as_object) else {
            return vec![];
        };

        let mut fans: Vec<FanData> = psu
            .iter()
            .filter_map(|(key, value)| {
                let position = key.strip_prefix("FanSpeed")?.parse::<i16>().ok()? - 1;
                let rpm = value
                    .as_str()
                    .and_then(Self::parse_number_from_units)
                    .map(AngularVelocity::from_rpm)?;
                Some(FanData {
                    position,
                    rpm: Some(rpm),
                })
            })
            .collect();

        fans.sort_by_key(|fan| fan.position);
        fans
    }
}

impl GetFluidTemperature for AuradineV1 {}

impl GetWattage for AuradineV1 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        let raw = data.get(&DataField::Wattage)?;
        if let Some(power) = raw.as_f64() {
            return Some(Power::from_watts(power));
        }

        raw.as_str()
            .and_then(Self::parse_number_from_units)
            .map(Power::from_watts)
    }
}

impl GetTuningTarget for AuradineV1 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        if let Some(ths) = data.extract::<f64>(DataField::TuningTarget) {
            return Some(TuningTarget::HashRate(HashRate {
                value: ths,
                unit: HashRateUnit::TeraHash,
                algo: String::from("SHA256"),
            }));
        }

        let mode = data.get(&DataField::TuningTarget)?.as_object()?;

        if let Some(power) = mode.get("Power").and_then(Value::as_f64) {
            return Some(TuningTarget::Power(Power::from_watts(power)));
        }

        if let Some(ths) = mode.get("Ths").and_then(Value::as_f64) {
            return Some(TuningTarget::HashRate(HashRate {
                value: ths,
                unit: HashRateUnit::TeraHash,
                algo: String::from("SHA256"),
            }));
        }

        match mode.get("Mode").and_then(Value::as_str) {
            Some(s) if s.eq_ignore_ascii_case("eco") => {
                Some(TuningTarget::MiningMode(MiningMode::Low))
            }
            Some(s) if s.eq_ignore_ascii_case("normal") => {
                Some(TuningTarget::MiningMode(MiningMode::Normal))
            }
            Some(s) if s.eq_ignore_ascii_case("turbo") => {
                Some(TuningTarget::MiningMode(MiningMode::High))
            }
            _ => None,
        }
    }
}

impl GetLightFlashing for AuradineV1 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract_map::<u64, _>(DataField::LightFlashing, |code| code == 3)
    }
}

impl GetMessages for AuradineV1 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let Some(led) = data.get(&DataField::Messages).and_then(Value::as_object) else {
            return vec![];
        };

        let code = led.get("Code").and_then(Value::as_u64).unwrap_or_default();
        let message = led
            .get("Msg")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| format!("LED code {code}"));

        let severity = Self::led_status_severity(code);

        vec![MinerMessage::new(0, code, message, severity)]
    }
}

impl GetUptime for AuradineV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetIsMining for AuradineV1 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        if let Some(is_mining_data) = data.get(&DataField::IsMining) {
            if let Some(state) = is_mining_data.get("sleep").and_then(Value::as_str) {
                if state.eq_ignore_ascii_case("off") {
                    return true;
                }
                if state.eq_ignore_ascii_case("on") {
                    return false;
                }
            }

            if let Some(is_sleeping) = is_mining_data.get("sleep").and_then(Value::as_bool) {
                return !is_sleeping;
            }

            if let Some(hashrate) = is_mining_data.get("hashrate").and_then(Value::as_f64) {
                return hashrate > 0.0;
            }

            if let Some(hashrate) = is_mining_data.get("hashrate").and_then(Value::as_i64) {
                return hashrate > 0;
            }
        }

        data.extract::<f64>(DataField::Hashrate)
            .map(|hr| hr > 0.0)
            .unwrap_or(false)
    }
}

impl GetPools for AuradineV1 {
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

        let mut pools = Vec::with_capacity(pools_array.len());
        for (idx, pool_info) in pools_array.iter().enumerate() {
            let url = pool_info
                .get("URL")
                .and_then(Value::as_str)
                .map(|url| PoolURL::from(url.to_string()));

            let accepted_shares = pool_info.get("Accepted").and_then(Value::as_u64);
            let rejected_shares = pool_info.get("Rejected").and_then(Value::as_u64);
            let active = pool_info.get("Stratum Active").and_then(Value::as_bool);
            let alive = pool_info
                .get("Status")
                .and_then(Value::as_str)
                .map(|status| !status.eq_ignore_ascii_case("dead"));
            let user = pool_info
                .get("User")
                .and_then(Value::as_str)
                .map(String::from);

            pools.push(PoolData {
                position: pool_info
                    .get("POOL")
                    .and_then(Value::as_u64)
                    .map(|position| position as u16)
                    .or(Some(idx as u16)),
                url,
                accepted_shares,
                rejected_shares,
                active,
                alive,
                user,
            });
        }

        vec![PoolGroupData {
            name: String::new(),
            quota: 1,
            pools,
        }]
    }
}

#[async_trait]
impl SetFaultLight for AuradineV1 {
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        let code = if fault { 3 } else { 2 };
        self.web
            .send_command("led", true, Some(json!({ "code": code })), Method::POST)
            .await?;
        Ok(true)
    }

    fn supports_set_fault_light(&self) -> bool {
        true
    }
}

#[async_trait]
impl SetPowerLimit for AuradineV1 {
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        self.web
            .send_command(
                "mode",
                true,
                Some(json!({
                    "mode": "custom",
                    "tune": "power",
                    "power": limit.as_watts().round() as u64,
                })),
                Method::POST,
            )
            .await?;
        Ok(true)
    }

    fn supports_set_power_limit(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsPoolsConfig for AuradineV1 {
    fn parse_pools_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<Vec<PoolGroupConfig>> {
        let Some(pools) = data.get(&ConfigField::Pools).and_then(Value::as_array) else {
            return Ok(vec![]);
        };

        let mut parsed = vec![];
        for pool in pools {
            let Some(url) = pool.get("URL").and_then(Value::as_str) else {
                continue;
            };
            if url.is_empty() {
                continue;
            }

            parsed.push(PoolConfig {
                url: PoolURL::from(url.to_string()),
                username: pool
                    .get("User")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                password: String::from("x"),
            });
        }

        Ok(vec![PoolGroupConfig {
            name: String::new(),
            quota: 1,
            pools: parsed,
        }])
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> anyhow::Result<bool> {
        let pools = Self::build_update_pools_payload(&config)?;

        self.web
            .send_command(
                "updatepools",
                true,
                Some(json!({ "pools": pools })),
                Method::POST,
            )
            .await?;

        Ok(true)
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for AuradineV1 {
    async fn restart(&self) -> anyhow::Result<bool> {
        self.web
            .send_command("restart", true, None, Method::POST)
            .await?;
        Ok(true)
    }

    fn supports_restart(&self) -> bool {
        true
    }
}

#[async_trait]
impl Pause for AuradineV1 {
    async fn pause(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        self.web
            .send_command("mode", true, Some(json!({ "sleep": "on" })), Method::POST)
            .await?;
        Ok(true)
    }

    fn supports_pause(&self) -> bool {
        true
    }
}

#[async_trait]
impl Resume for AuradineV1 {
    async fn resume(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        self.web
            .send_command("mode", true, Some(json!({ "sleep": "off" })), Method::POST)
            .await?;
        Ok(true)
    }

    fn supports_resume(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsScalingConfig for AuradineV1 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for AuradineV1 {
    fn supports_upgrade_firmware(&self) -> bool {
        false
    }
}

impl HasDefaultAuth for AuradineV1 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("admin", "admin")
    }
}

impl HasAuth for AuradineV1 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

#[async_trait]
impl SupportsTuningConfig for AuradineV1 {
    fn supports_tuning_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsFanConfig for AuradineV1 {
    fn supports_fan_config(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::{Context, anyhow};
    use asic_rs_core::{
        config::collector::ConfigCollector,
        config::pools::{PoolConfig, PoolGroupConfig},
        test::{api::MockAPIClient, util::get_miner},
    };
    use asic_rs_makes_auradine::models::AuradineModel;

    use super::*;
    use crate::{
        firmware::AuradineFirmware,
        test::json::v1::{
            ASCCOUNT, DEVS, FAN, FREQUENCY, IPREPORT, LED, MODE, POOLS, PSU, SUMMARY, TEMPERATURE,
            VERSION, VOLTAGE,
        },
    };

    #[tokio::test]
    async fn parse_data_test_auradine_at1500() -> anyhow::Result<()> {
        let miner = AuradineV1::new(IpAddr::from([127, 0, 0, 1]), AuradineModel::AT1500);

        let mut results = HashMap::new();
        results.insert(
            MinerCommand::RPC {
                command: "version",
                parameters: None,
            },
            Value::from_str(VERSION)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "summary",
                parameters: None,
            },
            Value::from_str(SUMMARY)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "devs",
                parameters: None,
            },
            Value::from_str(DEVS)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "asccount",
                parameters: None,
            },
            Value::from_str(ASCCOUNT)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "pools",
                parameters: None,
            },
            Value::from_str(POOLS)?,
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "ipreport",
                parameters: None,
            },
            Value::from_str(IPREPORT)?,
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "fan",
                parameters: None,
            },
            Value::from_str(FAN)?,
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "led",
                parameters: None,
            },
            Value::from_str(LED)?,
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "mode",
                parameters: None,
            },
            Value::from_str(MODE)?,
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "psu",
                parameters: None,
            },
            Value::from_str(PSU)?,
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "temperature",
                parameters: None,
            },
            Value::from_str(TEMPERATURE)?,
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "voltage",
                parameters: None,
            },
            Value::from_str(VOLTAGE)?,
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "frequency",
                parameters: None,
            },
            Value::from_str(FREQUENCY)?,
        );

        let mock_api = MockAPIClient::new(results);
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;

        let miner_data = miner.parse_data(data);

        assert_eq!(miner_data.ip, IpAddr::from([127, 0, 0, 1]));
        assert_eq!(
            miner_data
                .mac
                .map(|mac| mac.to_string().to_ascii_lowercase()),
            Some("1c:00:00:0e:3b:2b".to_string())
        );
        assert_eq!(miner_data.serial_number.as_deref(), Some("AU23200006"));
        assert_eq!(miner_data.hostname.as_deref(), Some("teraflux-01"));
        assert_eq!(miner_data.api_version.as_deref(), Some("1.0"));
        assert_eq!(miner_data.firmware_version.as_deref(), Some("2023-12.27"));
        assert_eq!(miner_data.uptime, Some(Duration::from_secs(56696)));
        assert_eq!(miner_data.expected_hashboards, Some(3));
        assert_eq!(miner_data.expected_chips, Some(396));
        assert_eq!(miner_data.total_chips, Some(396));
        assert_eq!(miner_data.expected_fans, Some(4));
        assert_eq!(miner_data.wattage, Some(Power::from_watts(58.94)));
        assert_eq!(miner_data.fans.len(), 4);
        assert_eq!(miner_data.psu_fans.len(), 2);
        assert_eq!(miner_data.hashboards.len(), 3);
        assert_eq!(
            miner_data
                .hashboards
                .first()
                .and_then(|b| b.serial_number.clone()),
            Some("QAHB01230908001X".to_string())
        );
        assert_eq!(
            miner_data.hashboards.first().and_then(|b| b.expected_chips),
            Some(132)
        );
        assert_eq!(miner_data.hashboards[0].working_chips, Some(132));
        assert_eq!(miner_data.hashboards[1].working_chips, Some(132));
        assert_eq!(miner_data.hashboards[2].working_chips, Some(132));
        assert_eq!(miner_data.hashboards[0].chips.len(), 3);
        assert_eq!(
            miner_data.hashboards[0].intake_temperature,
            Some(Temperature::from_celsius(20.5))
        );
        assert_eq!(
            miner_data.hashboards[0].outlet_temperature,
            Some(Temperature::from_celsius(41.5))
        );
        assert!(miner_data.hashboards[0].voltage.is_some());
        assert!(miner_data.hashboards[0].frequency.is_some());
        assert!(
            miner_data.hashboards[0]
                .chips
                .iter()
                .all(|chip| chip.temperature.is_some()
                    && chip.voltage.is_some()
                    && chip.frequency.is_some())
        );
        assert_eq!(
            miner_data.control_board_version,
            Some(MinerControlBoard::known("T0".to_string()))
        );
        assert_eq!(miner_data.light_flashing, Some(false));
        assert_eq!(miner_data.messages.len(), 1);
        assert_eq!(miner_data.messages[0].code, 2);
        assert_eq!(miner_data.messages[0].severity, MessageSeverity::Info);
        assert_eq!(miner_data.messages[0].message, "Normal");
        assert!(miner_data.is_mining);
        assert_eq!(miner_data.pools.len(), 1);
        assert_eq!(miner_data.pools[0].pools.len(), 3);
        assert_eq!(
            miner_data.tuning_target,
            Some(TuningTarget::Power(Power::from_watts(2000.0)))
        );
        assert_eq!(
            miner_data.expected_hashrate,
            Some(HashRate {
                value: 160.0,
                unit: HashRateUnit::TeraHash,
                algo: String::from("SHA256"),
            })
        );

        Ok(())
    }

    #[test]
    fn parse_control_board_version_keeps_unknown_internal_type() {
        let miner = AuradineV1::new(IpAddr::from([127, 0, 0, 1]), AuradineModel::AT1500);
        let mut data = HashMap::new();
        data.insert(
            DataField::ControlBoardVersion,
            json!({
                "internal_type": "T9"
            }),
        );

        assert_eq!(
            miner.parse_control_board_version(&data),
            Some(MinerControlBoard::unknown("T9".to_string()))
        );
    }

    #[tokio::test]
    async fn parse_pools_config_test() -> anyhow::Result<()> {
        let miner = AuradineV1::new(IpAddr::from([127, 0, 0, 1]), AuradineModel::AT1500);
        let mut results = HashMap::new();
        results.insert(
            MinerCommand::RPC {
                command: "pools",
                parameters: None,
            },
            Value::from_str(POOLS)?,
        );

        let mock_api = MockAPIClient::new(results);
        let mut collector = ConfigCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect(&[ConfigField::Pools]).await;

        let pools_config = miner.parse_pools_config(&data)?;
        let group = pools_config
            .first()
            .ok_or_else(|| anyhow!("missing pools group"))?;

        assert_eq!(group.quota, 1);
        assert_eq!(group.pools.len(), 3);
        assert_eq!(group.pools[0].username, "username.suffix");

        Ok(())
    }

    #[tokio::test]
    async fn parse_is_mining_prefers_sleep_state() -> anyhow::Result<()> {
        let miner = AuradineV1::new(IpAddr::from([127, 0, 0, 1]), AuradineModel::AT1500);
        let mut results = HashMap::new();
        results.insert(
            MinerCommand::RPC {
                command: "summary",
                parameters: None,
            },
            Value::from_str(SUMMARY)?,
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "mode",
                parameters: None,
            },
            json!({
                "STATUS": [{"STATUS": "S"}],
                "Mode": [{"Sleep": "on"}],
            }),
        );

        let mock_api = MockAPIClient::new(results);
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect(&[DataField::IsMining]).await;

        assert!(!miner.parse_is_mining(&data));

        Ok(())
    }

    #[test]
    fn build_update_pools_payload_defaults_empty_passwords_to_x() -> anyhow::Result<()> {
        let config = vec![PoolGroupConfig {
            name: String::new(),
            quota: 1,
            pools: vec![PoolConfig {
                url: PoolURL::from("stratum+tcp://example.com:3333".to_string()),
                username: "worker".to_string(),
                password: String::new(),
            }],
        }];

        let payload = AuradineV1::build_update_pools_payload(&config)?;
        assert_eq!(payload[0].get("pass").and_then(Value::as_str), Some("x"));

        Ok(())
    }

    #[test]
    fn build_update_pools_payload_truncates_to_three_pools() -> anyhow::Result<()> {
        let config = vec![PoolGroupConfig {
            name: String::new(),
            quota: 1,
            pools: (0..4)
                .map(|idx| PoolConfig {
                    url: PoolURL::from(format!("stratum+tcp://pool{idx}.example.com:3333")),
                    username: format!("worker{idx}"),
                    password: format!("pass{idx}"),
                })
                .collect(),
        }];

        let payload = AuradineV1::build_update_pools_payload(&config)?;
        assert_eq!(payload.len(), 3);
        assert_eq!(
            payload[2].get("user").and_then(Value::as_str),
            Some("worker2")
        );

        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires live miner; set MINER_IP"]
    async fn parse_data_live_test_auto_detect() -> anyhow::Result<()> {
        let ip_str = std::env::var("MINER_IP").context("MINER_IP is not set")?;
        let ip =
            IpAddr::from_str(&ip_str).with_context(|| format!("invalid MINER_IP: {ip_str}"))?;

        let miner = get_miner(ip, Arc::new(AuradineFirmware::default()))
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

        assert_eq!(miner_data.ip, ip);
        assert!(miner_data.timestamp > 0);
        assert!(!miner_data.schema_version.is_empty());

        Ok(())
    }
}

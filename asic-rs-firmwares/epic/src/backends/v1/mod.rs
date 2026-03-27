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
        hashrate::{HashRate, HashRateUnit},
        miner::TuningTarget,
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_antminer::hardware::AntMinerControlBoard;
use asic_rs_makes_epic::hardware::EPicControlBoard;
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
        PowerPlayV1 {
            ip,
            web: PowerPlayWebAPI::new(ip, 4028, auth),
            device_info: DeviceInfo::new(model, EPicFirmware::default(), HashAlgorithm::SHA256),
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
                        tag: Some("hashratesplit"),
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
                (
                    WEB_CAPABILITIES,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: Some("Capabilities"),
                    },
                ),
            ],
            DataField::Pools => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
                    tag: None,
                },
            )],
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
            DataField::ControlBoardVersion => vec![(
                WEB_CAPABILITIES,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Control Board Version/cpuHardware"),
                    tag: None,
                },
            )],
            DataField::SerialNumber => vec![(
                WEB_CAPABILITIES,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Control Board Version/cpuSerial"),
                    tag: None,
                },
            )],
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
        data.extract::<String>(DataField::SerialNumber)
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
        let cb_type = data.extract::<String>(DataField::ControlBoardVersion)?;
        match cb_type.as_str() {
            s if s.to_uppercase().contains("AMLOGIC") => {
                Some(AntMinerControlBoard::AMLogic).map(|cb| cb.into())
            }
            s if s.to_uppercase().contains("GENERIC AM33XX") => {
                Some(AntMinerControlBoard::BeagleBoneBlack).map(|cb| cb.into())
            }
            _ => Some(EPicControlBoard::EPicUMC).map(|cb| cb.into()),
        }
    }
}

impl GetHashboards for PowerPlayV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> = Vec::new();
        for _ in 0..self.device_info.hardware.boards.unwrap_or_default() {
            hashboards.push(BoardData {
                position: 0,
                hashrate: None,
                expected_hashrate: None,
                board_temperature: None,
                intake_temperature: None,
                outlet_temperature: None,
                expected_chips: None,
                working_chips: None,
                serial_number: None,
                chips: vec![],
                voltage: None,
                frequency: None,
                tuned: None,
                active: None,
            });
        }

        data.get(&DataField::Hashboards)
            .and_then(|v| v.pointer("/Summary/HBStatus"))
            .and_then(|v| {
                v.as_array().map(|boards| {
                    boards.iter().for_each(|board| {
                        if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                            && let Some(hashboard) = hashboards.get_mut(idx as usize)
                        {
                            hashboard.position = idx as u8;
                            if let Some(v) = board.get("Enabled").and_then(|v| v.as_bool()) {
                                hashboard.active = Some(v);
                            }
                        }
                    })
                })
            });

        // Create ChipData for each active board
        for board in &mut hashboards {
            board.expected_chips = self.device_info.hardware.chips;
            // No need to add ChipData if we know the board is not active
            if board.active.unwrap_or(false) {
                board.chips = vec![
                    ChipData {
                        position: 0,
                        hashrate: None,
                        temperature: None,
                        voltage: None,
                        frequency: None,
                        tuned: None,
                        working: None,
                    };
                    self.device_info.hardware.chips.unwrap_or_default() as usize
                ];
            }
        }

        //Capabilities Board Serial Numbers
        if let Some(serial_numbers) = data
            .get(&DataField::Hashboards)
            .and_then(|v| v.pointer("/Capabilities/Board Serial Numbers"))
            .and_then(|v| v.as_array())
        {
            for serial in serial_numbers {
                // Since we only have an array with no index, it will only correspond to working boards, so search for first working board
                // without serial and insert there
                for hb in hashboards.iter_mut() {
                    if hb.serial_number.is_none() && hb.active.unwrap_or(false) {
                        if let Some(serial_str) = serial.as_str() {
                            hb.serial_number = Some(serial_str.to_string());
                        }
                        break; // Only assign to the first board without a serial number
                    }
                }
            }
        };

        // Summary Data
        data.get(&DataField::Hashboards)
            .and_then(|v| v.pointer("/Summary/HBs"))
            .and_then(|v| {
                v.as_array().map(|boards| {
                    boards.iter().for_each(|board| {
                        if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                            && let Some(hashboard) = hashboards.get_mut(idx as usize)
                        {
                            // Hashrate
                            if let Some(h) = board
                                .get("Hashrate")
                                .and_then(|v| v.as_array())
                                .and_then(|v| v.first().and_then(|f| f.as_f64()))
                            {
                                hashboard.hashrate = Some(HashRate {
                                    value: h,
                                    unit: HashRateUnit::MegaHash,
                                    algo: String::from("SHA256"),
                                })
                            };

                            // ExpectedHashrate
                            if let Some(h) = board
                                .get("Hashrate")
                                .and_then(|v| v.as_array())
                                .and_then(|v| {
                                    Some((
                                        v.first().and_then(|f| f.as_f64())?,
                                        v.get(1).and_then(|f| f.as_f64())?,
                                    ))
                                })
                            {
                                hashboard.expected_hashrate = Some(HashRate {
                                    value: h.0 / h.1,
                                    unit: HashRateUnit::MegaHash,
                                    algo: String::from("SHA256"),
                                })
                            };

                            //Frequency
                            if let Some(f) = board.get("Core Clock Avg").and_then(|v| v.as_f64()) {
                                hashboard.frequency = Some(Frequency::from_megahertz(f))
                            };

                            //Voltage
                            if let Some(v) = board.get("Input Voltage").and_then(|v| v.as_f64()) {
                                hashboard.voltage = Some(Voltage::from_volts(v));
                            };
                            //Board Temp
                            if let Some(v) = board.get("Temperature").and_then(|v| v.as_f64()) {
                                hashboard.board_temperature = Some(Temperature::from_celsius(v));
                            };
                        };
                    })
                })
            });

        if let Some(tuned) = data
            .get(&DataField::Hashboards)
            .and_then(|v| v.pointer("/Summary/PerpetualTune/Algorithm"))
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

        //Temp Data
        data.get(&DataField::Hashboards)
            .and_then(|v| v.pointer("/Board Temps"))
            .and_then(|v| {
                v.as_array().map(|boards| {
                    boards.iter().for_each(|board| {
                        if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                            && let Some(hashboard) = hashboards.get_mut(idx as usize)
                        {
                            // Outlet Temperature
                            if let Some(h) = board.get("Data").and_then(|v| {
                                v.as_array().and_then(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_f64())
                                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                                })
                            }) {
                                hashboard.outlet_temperature = Some(Temperature::from_celsius(h));
                            };

                            if let Some(h) = board.get("Data").and_then(|v| {
                                v.as_array().and_then(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_f64())
                                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                                })
                            }) {
                                hashboard.intake_temperature = Some(Temperature::from_celsius(h));
                            };
                        };
                    })
                })
            });

        //Chip Temps
        data.get(&DataField::Hashboards)
            .and_then(|v| v.pointer("/Chip Temps"))
            .and_then(|v| {
                v.as_array().map(|boards| {
                    boards.iter().for_each(|board| {
                        if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                            && let Some(hashboard) = hashboards.get_mut(idx as usize)
                            && let Some(t) =
                                board.get("Data").and_then(|v| v.as_array()).map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_f64())
                                        .map(Temperature::from_celsius)
                                        .collect::<Vec<Temperature>>()
                                })
                        {
                            for (chip_no, temp) in t.iter().enumerate() {
                                if let Some(chip_data) = hashboard.chips.get_mut(chip_no) {
                                    chip_data.position = chip_no as u16;
                                    chip_data.temperature = Some(*temp);
                                }
                            }
                        };
                    })
                })
            });

        //Chip Voltages
        data.get(&DataField::Hashboards)
            .and_then(|v| v.pointer("/Chip Voltages"))
            .and_then(|v| {
                v.as_array().map(|boards| {
                    boards.iter().for_each(|board| {
                        if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                            && let Some(hashboard) = hashboards.get_mut(idx as usize)
                            && let Some(t) =
                                board.get("Data").and_then(|v| v.as_array()).map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_f64())
                                        .map(Voltage::from_millivolts)
                                        .collect::<Vec<Voltage>>()
                                })
                        {
                            for (chip_no, voltage) in t.iter().enumerate() {
                                if let Some(chip_data) = hashboard.chips.get_mut(chip_no) {
                                    chip_data.position = chip_no as u16;
                                    chip_data.voltage = Some(*voltage);
                                }
                            }
                        };
                    })
                })
            });

        //Chip Frequencies
        data.get(&DataField::Hashboards)
            .and_then(|v| v.pointer("/Chip Clocks"))
            .and_then(|v| {
                v.as_array().map(|boards| {
                    boards.iter().for_each(|board| {
                        if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                            && let Some(hashboard) = hashboards.get_mut(idx as usize)
                            && let Some(t) =
                                board.get("Data").and_then(|v| v.as_array()).map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_f64())
                                        .map(Frequency::from_megahertz)
                                        .collect::<Vec<Frequency>>()
                                })
                        {
                            for (chip_no, freq) in t.iter().enumerate() {
                                if let Some(chip_data) = hashboard.chips.get_mut(chip_no) {
                                    chip_data.position = chip_no as u16;
                                    chip_data.frequency = Some(*freq);
                                }
                            }
                        };
                    })
                })
            });

        //Chip Hashrate
        //There should always be a hashrate, and if there is a hashrate its also working
        data.get(&DataField::Hashboards)
            .and_then(|v| v.pointer("/Chip Hashrates"))
            .and_then(|v| {
                v.as_array().map(|boards| {
                    boards.iter().for_each(|board| {
                        if let Some(idx) = board.get("Index").and_then(|v| v.as_u64())
                            && let Some(hashboard) = hashboards.get_mut(idx as usize)
                            && let Some(t) =
                                board.get("Data").and_then(|v| v.as_array()).map(|arr| {
                                    arr.iter()
                                        .filter_map(|inner| inner.as_array())
                                        .filter_map(|inner| inner.first().and_then(|v| v.as_f64()))
                                        .map(|hr| HashRate {
                                            value: hr,
                                            unit: HashRateUnit::MegaHash,
                                            algo: String::from("SHA256"),
                                        })
                                        .collect::<Vec<HashRate>>()
                                })
                        {
                            for (chip_no, hashrate) in t.iter().enumerate() {
                                if let Some(chip_data) = hashboard.chips.get_mut(chip_no) {
                                    chip_data.position = chip_no as u16;
                                    chip_data.working = Some(true);
                                    chip_data.hashrate = Some(hashrate.clone());
                                }
                            }
                        };
                    })
                })
            });

        for board in &mut hashboards {
            board.working_chips = Some(
                board
                    .chips
                    .iter()
                    .filter(|c| c.working.unwrap_or(false))
                    .count() as u16,
            );
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

        Some(HashRate {
            value: total_hashrate,
            unit: HashRateUnit::MegaHash,
            algo: String::from("SHA256"),
        })
    }
}

impl GetExpectedHashrate for PowerPlayV1 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::TeraHash,
            algo: String::from("SHA256"),
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
    summary: &Value,
    algorithm: &str,
    stats: &Value,
    algorithm_drives_power: bool,
) -> Option<TuningTarget> {
    let target = tuning_value_as_f64(stats.get("Target")?)?;
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
    let algo = summary
        .pointer("/Mining/Algorithm")
        .and_then(Value::as_str)
        .unwrap_or("SHA256")
        .to_string();

    Some(TuningTarget::HashRate(HashRate {
        value: target,
        unit: hr_unit,
        algo,
    }))
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
            parse_tuning_target_from_stats(summary, algorithm, stats, true)
        })
    }
}

impl GetLightFlashing for PowerPlayV1 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing)
    }
}

impl GetMessages for PowerPlayV1 {}

impl GetUptime for PowerPlayV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract::<u64>(DataField::Uptime)
            .map(Duration::from_secs)
    }
}

impl GetIsMining for PowerPlayV1 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        data.extract::<String>(DataField::IsMining)
            .map(|state| state != "Idling")
            .unwrap_or(false)
    }
}

impl GetPools for PowerPlayV1 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let mut pools_vec: Vec<PoolData> = Vec::new();

        if let Some(configs) = data
            .get(&DataField::Pools)
            .and_then(|v| v.pointer("/StratumConfigs"))
            .and_then(|v| v.as_array())
        {
            for (idx, config) in configs.iter().enumerate() {
                let url = config.get("pool").and_then(|v| v.as_str()).and_then(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(PoolURL::from(s.to_string()))
                    }
                });
                let user = config
                    .get("login")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                pools_vec.push(PoolData {
                    position: Some(idx as u16),
                    url,
                    accepted_shares: None,
                    rejected_shares: None,
                    active: Some(false),
                    alive: None,
                    user,
                });
            }
        }

        if let Some(stratum) = data
            .get(&DataField::Pools)
            .and_then(|v| v.pointer("/Stratum"))
            .and_then(|v| v.as_object())
        {
            for pool in pools_vec.iter_mut() {
                if pool.position
                    == stratum
                        .get("Config Id")
                        .and_then(|v| v.as_u64().map(|v| v as u16))
                {
                    pool.active = Some(true);
                    pool.alive = stratum.get("IsPoolConnected").and_then(|v| v.as_bool());
                    pool.user = stratum
                        .get("Current User")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    pool.url = stratum
                        .get("Current Pool")
                        .and_then(|v| v.as_str())
                        .and_then(|s| {
                            if s.is_empty() {
                                None
                            } else {
                                Some(PoolURL::from(s.to_string()))
                            }
                        });

                    // Get Stats
                    if let Some(session) = data
                        .get(&DataField::Pools)
                        .and_then(|v| v.pointer("/Session"))
                        .and_then(|v| v.as_object())
                    {
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
            .get("hashratesplit")
            .and_then(|v| v.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if split_enabled {
            let mut groups: Vec<PoolGroupConfig> = Vec::new();

            if let Some(splits) = pools_object
                .get("hashratesplit")
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

        let coin = "BTC";
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

impl SupportsScalingConfig for PowerPlayV1 {
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
    fn parse_tuning_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<TuningConfig> {
        data.get(&ConfigField::Tuning)
            .and_then(|summary| {
                let (algorithm, stats) = first_perpetual_tune_algorithm(summary)?;
                let tuning_target =
                    parse_tuning_target_from_stats(summary, algorithm, stats, false)?;
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
impl UpgradeFirmware for PowerPlayV1 {
    fn supports_upgrade_firmware(&self) -> bool {
        false
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::{self, Context};
    use asic_rs_core::test::{api::MockAPIClient, util::get_miner};
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
                algo: String::from("SHA256"),
            })
        );
        assert_eq!(
            miner_data.hashboards[2].chips[72].hashrate,
            Some(HashRate {
                value: 487695.28,
                unit: HashRateUnit::MegaHash,
                algo: String::from("SHA256"),
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

        println!(
            "scalingconfig {}",
            serde_json::to_string_pretty(&miner.get_scaling_config().await?)?
        );

        println!(
            "tuningconfig {}",
            serde_json::to_string_pretty(&miner.get_tuning_config().await?)?
        );

        println!(
            "fanconfig {}",
            serde_json::to_string_pretty(&miner.get_fan_config().await?)?
        );

        assert_eq!(miner_data.ip, ip);
        assert!(miner_data.timestamp > 0);
        assert!(!miner_data.schema_version.is_empty());

        Ok(())
    }
}

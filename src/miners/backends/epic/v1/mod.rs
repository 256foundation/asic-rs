use anyhow;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use reqwest::Method;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

use crate::data::board::{BoardData, ChipData};
use crate::data::device::{DeviceInfo, HashAlgorithm, MinerFirmware, MinerModel};
use crate::data::device::{MinerControlBoard, MinerMake};
use crate::data::fan::FanData;
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::pool::{PoolData, PoolURL};
use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;
use crate::miners::data::{
    DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
};

use web::PowerPlayWebAPI;

mod web;

#[derive(Debug)]
pub struct PowerPlayV1 {
    ip: IpAddr,
    web: PowerPlayWebAPI,
    device_info: DeviceInfo,
}

impl PowerPlayV1 {
    pub fn new(ip: IpAddr, model: MinerModel) -> Self {
        PowerPlayV1 {
            ip,
            web: PowerPlayWebAPI::new(ip, 4028),
            device_info: DeviceInfo::new(
                MinerMake::from(model),
                model,
                MinerFirmware::EPic,
                HashAlgorithm::SHA256,
            ),
        }
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
        self.device_info
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
            s if s.to_uppercase().contains("AMLOGIC") => Some(MinerControlBoard::AMLogic),
            _ => Some(MinerControlBoard::EPicUMC),
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

impl GetWattageLimit for PowerPlayV1 {}

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
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolData> {
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
        pools_vec
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
impl SetPools for PowerPlayV1 {
    fn supports_set_pools(&self) -> bool {
        false
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::device::models::antminer::AntMinerModel::S19XP;
    use crate::test::api::MockAPIClient;
    use crate::test::json::epic::v1::*;
    use anyhow;

    #[tokio::test]
    async fn parse_data_test_antminer_s19xp() -> anyhow::Result<()> {
        let miner = PowerPlayV1::new(IpAddr::from([127, 0, 0, 1]), MinerModel::AntMiner(S19XP));

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
}

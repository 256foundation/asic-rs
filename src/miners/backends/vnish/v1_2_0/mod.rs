use anyhow;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use serde_json::Value;
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

use web::VnishWebAPI;

mod web;

#[derive(Debug)]
pub struct VnishV120 {
    ip: IpAddr,
    web: VnishWebAPI,
    device_info: DeviceInfo,
}

impl VnishV120 {
    pub fn new(ip: IpAddr, model: MinerModel) -> Self {
        VnishV120 {
            ip,
            web: VnishWebAPI::new(ip, 80),
            device_info: DeviceInfo::new(
                MinerMake::from(model),
                model,
                MinerFirmware::VNish,
                HashAlgorithm::SHA256,
            ),
        }
    }
}

#[async_trait]
impl APIClient for VnishV120 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!("Unsupported command type for Vnish API")),
        }
    }
}

impl GetDataLocations for VnishV120 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const WEB_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "info",
            parameters: None,
        };
        const WEB_STATUS: MinerCommand = MinerCommand::WebAPI {
            command: "status",
            parameters: None,
        };
        const WEB_SUMMARY: MinerCommand = MinerCommand::WebAPI {
            command: "summary",
            parameters: None,
        };
        const WEB_CHAINS: MinerCommand = MinerCommand::WebAPI {
            command: "chains",
            parameters: None,
        };
        const WEB_FACTORY_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "chains/factory-info",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system/network_status/mac"),
                    tag: None,
                },
            )],
            DataField::SerialNumber => vec![
                (
                    WEB_FACTORY_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/psu_serial"),
                        tag: None,
                    },
                ),
                (
                    WEB_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/serial"),
                        tag: None,
                    },
                ),
            ],
            DataField::Hostname => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system/network_status/hostname"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/fw_version"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/fw_version"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/platform"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                WEB_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system/uptime"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner/hr_realtime"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![
                (
                    WEB_FACTORY_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/hr_stock"),
                        tag: None,
                    },
                ),
                (
                    WEB_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/miner/hr_stock"),
                        tag: None,
                    },
                ),
            ],
            DataField::Wattage => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner/power_consumption"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner/cooling/fans"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![
                (
                    WEB_SUMMARY,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/miner/chains"),
                        tag: None,
                    },
                ),
                (
                    WEB_CHAINS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some(""),
                        tag: None,
                    },
                ),
            ],
            DataField::IsMining => vec![],
            DataField::Pools => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner/pools"),
                    tag: None,
                },
            )],
            DataField::MiningMode => vec![(
                WEB_STATUS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/miner_state"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                WEB_STATUS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/find_miner"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for VnishV120 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for VnishV120 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info
    }
}

impl CollectData for VnishV120 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for VnishV120 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetSerialNumber for VnishV120 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::SerialNumber)
    }
}

impl GetHostname for VnishV120 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for VnishV120 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for VnishV120 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetControlBoardVersion for VnishV120 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| MinerControlBoard::from_str(&s).ok())
    }
}

impl GetHashboards for VnishV120 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> = Vec::new();

        let chains_data = data.get(&DataField::Hashboards).and_then(|v| v.as_array());

        if let Some(chains_array) = chains_data {
            for (idx, chain) in chains_array.iter().enumerate() {
                let hashrate = Self::extract_hashrate(chain, &["/hashrate_rt", "/hr_realtime"]);
                let expected_hashrate =
                    Self::extract_hashrate(chain, &["/hashrate_ideal", "/hr_nominal"]);

                let frequency = Self::extract_frequency(chain);
                let voltage = Self::extract_voltage(chain);
                let (board_temperature, chip_temperature) = Self::extract_temperatures(chain);

                let working_chips = Self::extract_working_chips(chain);
                let active = Self::extract_chain_active_status(chain, &hashrate);
                let serial_number = Self::extract_chain_serial(chain, data);
                let tuned = Self::extract_tuned_status(chain, data);
                let chips = Self::extract_chips(chain);

                hashboards.push(BoardData {
                    position: chain
                        .pointer("/id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(idx as u64) as u8,
                    hashrate,
                    expected_hashrate,
                    board_temperature,
                    intake_temperature: chip_temperature,
                    outlet_temperature: chip_temperature,
                    expected_chips: self.device_info.hardware.chips,
                    working_chips,
                    serial_number,
                    chips,
                    voltage,
                    frequency,
                    tuned,
                    active,
                });
            }
        }

        hashboards
    }
}

impl GetHashrate for VnishV120 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: String::from("SHA256"),
        })
    }
}

impl GetIsMining for VnishV120 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        self.parse_hashrate(data)
            .map(|hr| hr.value > 0.0)
            .unwrap_or(false)
    }
}

impl GetExpectedHashrate for VnishV120 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: String::from("SHA256"),
        })
    }
}

impl GetFans for VnishV120 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans: Vec<FanData> = Vec::new();

        if let Some(fans_data) = data.get(&DataField::Fans)
            && let Some(fans_array) = fans_data.as_array()
        {
            for (idx, fan) in fans_array.iter().enumerate() {
                if let Some(rpm) = fan.pointer("/rpm").and_then(|v| v.as_i64()) {
                    fans.push(FanData {
                        position: idx as i16,
                        rpm: Some(AngularVelocity::from_rpm(rpm as f64)),
                    });
                }
            }
        }

        fans
    }
}

impl GetPsuFans for VnishV120 {}

impl GetFluidTemperature for VnishV120 {}

impl GetWattage for VnishV120 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<i64, _>(DataField::Wattage, |w| Power::from_watts(w as f64))
    }
}

impl GetWattageLimit for VnishV120 {}

impl GetLightFlashing for VnishV120 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing)
    }
}

impl GetMessages for VnishV120 {}

impl GetUptime for VnishV120 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract::<String>(DataField::Uptime)
            .and_then(|uptime_str| {
                // Parse uptime strings like "10 days, 18:00"
                let trimmed = uptime_str.trim();

                // Try to parse format like "X days, HH:MM" or "X days"
                if trimmed.contains("days") {
                    let mut total_seconds = 0u64;

                    // Extract days
                    if let Some(days_part) = trimmed.split("days").next()
                        && let Ok(days) = days_part.trim().parse::<u64>()
                    {
                        total_seconds += days * 24 * 60 * 60;
                    }

                    // Extract hours and minutes if present (after comma)
                    if let Some(time_part) = trimmed.split(',').nth(1) {
                        let time_part = time_part.trim();
                        if let Some((hours_str, minutes_str)) = time_part.split_once(':')
                            && let (Ok(hours), Ok(minutes)) = (
                                hours_str.trim().parse::<u64>(),
                                minutes_str.trim().parse::<u64>(),
                            )
                        {
                            total_seconds += hours * 60 * 60 + minutes * 60;
                        }
                    }

                    return Some(Duration::from_secs(total_seconds));
                }

                None
            })
    }
}

impl GetMiningMode for VnishV120 {
    fn parse_mining_mode(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> crate::data::miner::MiningMode {
        let enabled = data
            .extract::<String>(DataField::MiningMode)
            .map(|state| state == "mining")
            .unwrap_or(false);
        enabled.into()
    }
}

impl GetPools for VnishV120 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolData> {
        let mut pools: Vec<PoolData> = Vec::new();

        if let Some(pools_data) = data.get(&DataField::Pools)
            && let Some(pools_array) = pools_data.as_array()
        {
            for (idx, pool) in pools_array.iter().enumerate() {
                let url = pool
                    .pointer("/url")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .map(PoolURL::from);

                let user = pool
                    .pointer("/user")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let accepted_shares = pool.pointer("/accepted").and_then(|v| v.as_u64());
                let rejected_shares = pool.pointer("/rejected").and_then(|v| v.as_u64());
                let pool_status = pool.pointer("/status").and_then(|v| v.as_str());
                let (active, alive) = Self::parse_pool_status(pool_status);

                pools.push(PoolData {
                    position: Some(idx as u16),
                    url,
                    accepted_shares,
                    rejected_shares,
                    active,
                    alive,
                    user,
                });
            }
        }

        pools
    }
}

// Helper methods for data extraction
impl VnishV120 {
    fn extract_hashrate(chain: &Value, paths: &[&str]) -> Option<HashRate> {
        paths
            .iter()
            .find_map(|&path| chain.pointer(path).and_then(|v| v.as_f64()))
            .map(|f| HashRate {
                value: f,
                unit: HashRateUnit::GigaHash,
                algo: String::from("SHA256"),
            })
    }

    fn extract_frequency(chain: &Value) -> Option<Frequency> {
        chain
            .pointer("/frequency")
            .or_else(|| chain.pointer("/freq"))
            .and_then(|v| v.as_f64())
            .map(Frequency::from_megahertz)
    }

    fn extract_voltage(chain: &Value) -> Option<Voltage> {
        chain
            .pointer("/voltage")
            .and_then(|v| v.as_i64())
            .map(|v| Voltage::from_millivolts(v as f64))
    }

    fn extract_temperatures(chain: &Value) -> (Option<Temperature>, Option<Temperature>) {
        let board_temp = chain
            .pointer("/pcb_temp/max")
            .and_then(|v| v.as_i64())
            .map(|t| Temperature::from_celsius(t as f64));

        let chip_temp = chain
            .pointer("/chip_temp/max")
            .and_then(|v| v.as_i64())
            .map(|t| Temperature::from_celsius(t as f64));

        (board_temp, chip_temp)
    }

    fn extract_working_chips(chain: &Value) -> Option<u16> {
        chain
            .pointer("/chip_statuses")
            .map(|statuses| {
                let red = statuses
                    .pointer("/red")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let orange = statuses
                    .pointer("/orange")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                (red + orange) as u16
            })
            .or_else(|| {
                chain
                    .pointer("/chips")
                    .and_then(|v| v.as_array())
                    .map(|chips| chips.len() as u16)
            })
    }

    fn extract_chain_active_status(chain: &Value, hashrate: &Option<HashRate>) -> Option<bool> {
        chain
            .pointer("/status/state")
            .and_then(|v| v.as_str())
            .map(|s| s == "mining")
            .or_else(|| hashrate.as_ref().map(|h| h.value > 0.0))
    }

    fn parse_pool_status(status: Option<&str>) -> (Option<bool>, Option<bool>) {
        match status {
            Some("active" | "working") => (Some(true), Some(true)),
            Some("offline" | "disabled") => (Some(false), Some(false)),
            Some("rejecting") => (Some(false), Some(true)),
            _ => (None, None),
        }
    }

    fn extract_chain_serial(chain: &Value, data: &HashMap<DataField, Value>) -> Option<String> {
        // Try to get serial from chain-specific data first (factory-info)
        chain
            .pointer("/serial")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                // Fallback to miner-wide serial number
                data.extract::<String>(DataField::SerialNumber)
            })
    }

    fn extract_tuned_status(_chain: &Value, data: &HashMap<DataField, Value>) -> Option<bool> {
        // Check miner state to determine tuning status
        if let Some(miner_state) = data.extract::<String>(DataField::MiningMode) {
            match miner_state.as_str() {
                "auto-tuning" => Some(false), // Currently tuning, not yet tuned
                "mining" => Some(true),       // Tuned and mining
                _ => None,
            }
        } else {
            None
        }
    }

    fn extract_chips(chain: &Value) -> Vec<ChipData> {
        let mut chips: Vec<ChipData> = Vec::new();

        if let Some(chips_array) = chain.pointer("/chips").and_then(|v| v.as_array()) {
            for (idx, chip) in chips_array.iter().enumerate() {
                let hashrate = chip
                    .pointer("/hr")
                    .and_then(|v| v.as_f64())
                    .map(|f| HashRate {
                        value: f,
                        unit: HashRateUnit::GigaHash,
                        algo: String::from("SHA256"),
                    });

                let temperature = chip
                    .pointer("/temp")
                    .and_then(|v| v.as_f64())
                    .map(Temperature::from_celsius);

                let voltage = chip
                    .pointer("/volt")
                    .and_then(|v| v.as_i64())
                    .map(|v| Voltage::from_millivolts(v as f64));

                let frequency = chip
                    .pointer("/freq")
                    .and_then(|v| v.as_i64())
                    .map(|f| Frequency::from_megahertz(f as f64));

                let working = hashrate.as_ref().map(|hr| hr.value > 0.0);

                chips.push(ChipData {
                    position: chip
                        .pointer("/id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(idx as u64) as u16,
                    hashrate,
                    temperature,
                    voltage,
                    frequency,
                    tuned: None,
                    working,
                });
            }
        }

        chips
    }
}

#[async_trait]
impl SetFaultLight for VnishV120 {
    #[allow(unused_variables)]
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

#[async_trait]
impl SetPowerLimit for VnishV120 {
    #[allow(unused_variables)]
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

#[async_trait]
impl Restart for VnishV120 {
    async fn restart(&self) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

#[async_trait]
impl Pause for VnishV120 {
    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

#[async_trait]
impl Resume for VnishV120 {
    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

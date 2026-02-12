use crate::data::deserialize::deserialize_macaddr;
use crate::data::serialize::serialize_macaddr;
use crate::data::serialize::serialize_power;
use crate::data::serialize::serialize_temperature;
use std::{net::IpAddr, time::Duration};

use super::{
    board::BoardData, device::DeviceInfo, fan::FanData, hashrate::HashRate, message::MinerMessage,
    pool::PoolData,
};
use crate::data::device::MinerControlBoard;
use macaddr::MacAddr;
use measurements::{Power, Temperature};
use serde::{Deserialize, Serialize};

pub use super::mode::MiningMode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinerData {
    /// The schema version of this MinerData object, for use in external APIs
    pub schema_version: String,
    /// The time this data was gathered and constructed
    pub timestamp: u64,
    /// The IP address of the miner this data is for
    pub ip: IpAddr,
    /// The MAC address of the miner this data is for
    #[serde(
        serialize_with = "serialize_macaddr",
        deserialize_with = "deserialize_macaddr"
    )]
    pub mac: Option<MacAddr>,
    /// Hardware information about this miner
    pub device_info: DeviceInfo,
    /// The serial number of the miner, also known as the control board serial
    pub serial_number: Option<String>,
    /// The network hostname of the miner
    pub hostname: Option<String>,
    /// The API version of the miner
    pub api_version: Option<String>,
    /// The firmware version of the miner
    pub firmware_version: Option<String>,
    /// The type of control board on the miner
    pub control_board_version: Option<MinerControlBoard>,
    /// The expected number of boards in the miner.
    pub expected_hashboards: Option<u8>,
    /// Per-hashboard data for this miner
    pub hashboards: Vec<BoardData>,
    /// The current hashrate of the miner
    pub hashrate: Option<HashRate>,
    /// The expected hashrate of the miner
    pub expected_hashrate: Option<HashRate>,
    /// The total expected number of chips across all boards on this miner
    pub expected_chips: Option<u16>,
    /// The total number of working chips across all boards on this miner
    pub total_chips: Option<u16>,
    /// The expected number of fans on the miner
    pub expected_fans: Option<u8>,
    /// The current fan information for the miner
    pub fans: Vec<FanData>,
    /// The current PDU fan information for the miner
    pub psu_fans: Vec<FanData>,
    /// The average temperature across all chips in the miner
    #[serde(serialize_with = "serialize_temperature")]
    pub average_temperature: Option<Temperature>,
    /// The environment temperature of the miner, such as air temperature or immersion fluid temperature
    #[serde(serialize_with = "serialize_temperature")]
    pub fluid_temperature: Option<Temperature>,
    /// The current power consumption of the miner
    #[serde(serialize_with = "serialize_power")]
    pub wattage: Option<Power>,
    /// The current power limit or power target of the miner
    #[serde(serialize_with = "serialize_power")]
    pub wattage_limit: Option<Power>,
    /// The current efficiency in W/TH/s (J/TH) of the miner
    pub efficiency: Option<f64>,
    /// The state of the fault/alert light on the miner
    pub light_flashing: Option<bool>,
    /// Any message on the miner, including errors
    pub messages: Vec<MinerMessage>,
    /// The total uptime of the miner's system
    pub uptime: Option<Duration>,
    /// Whether the hashing process is currently running
    pub mining_mode: MiningMode,
    /// Whether the miner is currently hashing.
    pub is_mining: bool,
    /// The current pools configured on the miner
    pub pools: Vec<PoolData>,
}

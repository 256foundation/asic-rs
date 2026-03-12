use std::fmt::Display;

use measurements::{Frequency, Temperature, Voltage};
#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    hashrate::HashRate,
    serialize::{serialize_frequency, serialize_temperature, serialize_voltage},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChipData {
    /// The position of the chip on the board, indexed from 0
    pub position: u16,
    /// The current hashrate of the chip
    pub hashrate: Option<HashRate>,
    /// The current chip temperature
    #[serde(serialize_with = "serialize_temperature")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<Temperature>,
    /// The voltage set point for this chip
    #[serde(serialize_with = "serialize_voltage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voltage: Option<Voltage>,
    /// The frequency set point for this chip
    #[serde(serialize_with = "serialize_frequency")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<Frequency>,
    /// Whether this chip is tuned and optimizations have completed
    pub tuned: Option<bool>,
    /// Whether this chip is working and actively mining
    pub working: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BoardData {
    /// The board position in the miner, indexed from 0
    pub position: u8,
    /// The current hashrate of the board
    pub hashrate: Option<HashRate>,
    /// The expected or factory hashrate of the board
    pub expected_hashrate: Option<HashRate>,
    /// The board temperature, also sometimes called PCB temperature
    #[serde(serialize_with = "serialize_temperature")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board_temperature: Option<Temperature>,
    /// The temperature of the chips at the intake, usually from the first sensor on the board
    #[serde(serialize_with = "serialize_temperature")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intake_temperature: Option<Temperature>,
    /// The temperature of the chips at the outlet, usually from the last sensor on the board
    #[serde(serialize_with = "serialize_temperature")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outlet_temperature: Option<Temperature>,
    /// The expected number of chips on this board
    pub expected_chips: Option<u16>,
    /// The number of working chips on this board
    pub working_chips: Option<u16>,
    /// The serial number of this board
    pub serial_number: Option<String>,
    /// Chip level information for this board
    /// May be empty, most machines do not provide this level of in depth information
    pub chips: Vec<ChipData>,
    /// The average voltage or voltage set point of this board
    #[serde(serialize_with = "serialize_voltage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voltage: Option<Voltage>,
    /// The average frequency or frequency set point of this board
    #[serde(serialize_with = "serialize_frequency")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<Frequency>,
    /// Whether this board has been tuned and optimizations have completed
    pub tuned: Option<bool>,
    /// Whether this board is enabled and actively mining
    pub active: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
pub struct MinerControlBoard {
    pub known: bool,
    pub name: String,
}
impl MinerControlBoard {
    pub fn unknown(name: String) -> Self {
        Self { known: false, name }
    }
    pub fn known(name: String) -> Self {
        Self { known: true, name }
    }
}

impl Display for MinerControlBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.known {
            write!(f, "{}", self.name)
        } else {
            write!(f, "Unknown: {}", self.name)
        }
    }
}

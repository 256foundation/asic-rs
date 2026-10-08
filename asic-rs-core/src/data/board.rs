use std::fmt::Display;

use measurements::{Frequency, Temperature, Voltage};
#[cfg(feature = "python")]
use pyo3::pyclass;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{
    deserialize::deserialize_temperature,
    hashrate::HashRate,
    serialize::{serialize_frequency, serialize_temperature, serialize_voltage},
};

#[cfg_attr(feature = "python", pyclass(from_py_object, module = "asic_rs"))]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model(getters))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, TS)]
/// Per-chip telemetry for a hashboard.
pub struct ChipData {
    /// The position of the chip on the board, indexed from 0
    pub position: u16,
    /// The current hashrate of the chip
    pub hashrate: Option<HashRate>,
    /// The current chip temperature
    #[serde(serialize_with = "serialize_temperature")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub temperature: Option<Temperature>,
    /// The voltage set point for this chip
    #[serde(serialize_with = "serialize_voltage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub voltage: Option<Voltage>,
    /// The frequency set point for this chip
    #[serde(serialize_with = "serialize_frequency")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub frequency: Option<Frequency>,
    /// Whether this chip is tuned and optimizations have completed
    pub tuned: Option<bool>,
    /// Whether this chip is working and actively mining
    pub working: Option<bool>,
}

#[cfg_attr(feature = "python", pyclass(from_py_object, module = "asic_rs"))]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model(getters))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, TS)]
/// Per-hashboard telemetry for a miner.
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
    #[ts(optional, type = "number")]
    pub board_temperature: Option<Temperature>,
    /// The temperature of the coolest / first-in-chain chip on the board
    #[serde(serialize_with = "serialize_temperature")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub inlet_chip_temperature: Option<Temperature>,
    /// The temperature of the hottest / last-in-chain chip on the board
    #[serde(serialize_with = "serialize_temperature")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub outlet_chip_temperature: Option<Temperature>,
    /// Coolant entering this hashboard, when a dedicated hydro sensor is reported.
    /// This is not a PCB or chip temperature.
    #[serde(
        default,
        serialize_with = "serialize_temperature",
        deserialize_with = "deserialize_temperature"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub inlet_fluid_temperature: Option<Temperature>,
    /// Coolant leaving this hashboard, when a dedicated hydro sensor is reported.
    /// This is not a PCB or chip temperature.
    #[serde(
        default,
        serialize_with = "serialize_temperature",
        deserialize_with = "deserialize_temperature"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub outlet_fluid_temperature: Option<Temperature>,
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
    #[ts(optional, type = "number")]
    pub voltage: Option<Voltage>,
    /// The average frequency or frequency set point of this board
    #[serde(serialize_with = "serialize_frequency")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub frequency: Option<Frequency>,
    /// Whether this board has been tuned and optimizations have completed
    pub tuned: Option<bool>,
    /// Whether this board is active, as reported or inferred from firmware telemetry
    pub active: Option<bool>,
}

impl BoardData {
    /// Create a new board at the given position with the specified expected chip count.
    /// All other fields are left at their defaults.
    pub fn new(position: u8, expected_chips: Option<u16>) -> Self {
        Self {
            position,
            expected_chips,
            ..Default::default()
        }
    }

    /// Create a new board with tuned and active state pre-set.
    pub fn with_state(
        position: u8,
        expected_chips: Option<u16>,
        tuned: Option<bool>,
        active: Option<bool>,
    ) -> Self {
        Self {
            position,
            expected_chips,
            tuned,
            active,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
/// Control board identity reported by a miner.
pub struct MinerControlBoard {
    /// Whether the control board name is a known board identifier.
    pub known: bool,
    /// Control board name or raw unknown identifier.
    pub name: String,
}
impl MinerControlBoard {
    /// Create an unknown control board identity from a raw name.
    pub fn unknown(name: String) -> Self {
        Self { known: false, name }
    }
    /// Create a known control board identity.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coolant_fields_round_trip_as_celsius_without_changing_chip_or_board_values() {
        let board = BoardData {
            inlet_fluid_temperature: Some(Temperature::from_celsius(34.0)),
            outlet_fluid_temperature: Some(Temperature::from_celsius(42.0)),
            ..Default::default()
        };
        let json = serde_json::to_value(&board).unwrap();
        assert_eq!(json["inlet_fluid_temperature"], 34.0);
        assert_eq!(json["outlet_fluid_temperature"], 42.0);
        let restored: BoardData = serde_json::from_value(json).unwrap();
        assert_eq!(restored, board);
        assert!(restored.board_temperature.is_none());
        assert!(restored.inlet_chip_temperature.is_none());
        assert!(restored.outlet_chip_temperature.is_none());
    }

    #[test]
    fn older_board_snapshots_do_not_require_coolant_fields() {
        let json = serde_json::to_value(BoardData::default()).unwrap();
        assert!(json.get("inlet_fluid_temperature").is_none());
        assert!(json.get("outlet_fluid_temperature").is_none());
        let restored: BoardData = serde_json::from_value(json).unwrap();
        assert!(restored.inlet_fluid_temperature.is_none());
        assert!(restored.outlet_fluid_temperature.is_none());
    }
}

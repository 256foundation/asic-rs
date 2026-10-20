use measurements::{Current, Power, Temperature, Voltage};
#[cfg(feature = "python")]
use pyo3::pyclass;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::serialize::{
    serialize_current, serialize_power, serialize_temperature, serialize_voltage,
};

#[cfg_attr(feature = "python", pyclass(from_py_object, module = "asic_rs"))]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model(getters))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, TS)]
/// Power supply telemetry. PSU fan speed stays in `MinerData.psu_fans` and PSU
/// faults in `MinerData.messages`.
pub struct PsuData {
    /// The position of the PSU in the miner, indexed from 0
    pub position: u16,
    /// The PSU model reported by the device
    pub model: Option<String>,
    /// The PSU serial number
    pub serial_number: Option<String>,
    /// The PSU firmware version
    pub firmware_version: Option<String>,
    /// The AC (or DC) supply voltage into the PSU
    #[serde(serialize_with = "serialize_voltage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub input_voltage: Option<Voltage>,
    /// The DC voltage the PSU delivers to the hashboards
    #[serde(serialize_with = "serialize_voltage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub output_voltage: Option<Voltage>,
    /// The DC current the PSU delivers to the hashboards
    #[serde(serialize_with = "serialize_current")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub output_current: Option<Current>,
    /// The power drawn by the PSU
    #[serde(serialize_with = "serialize_power")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub input_power: Option<Power>,
    /// The power the PSU delivers to the hashboards
    #[serde(serialize_with = "serialize_power")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub output_power: Option<Power>,
    /// Output power divided by input power, as a ratio from 0 to 1
    pub efficiency: Option<f64>,
    /// The PSU temperature
    #[serde(serialize_with = "serialize_temperature")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub temperature: Option<Temperature>,
}

use pyo3::prelude::*;

use crate::data::board::BoardData as BoardData_Base;
use crate::data::board::ChipData as ChipData_Base;
pub(crate) use crate::data::device::{HashAlgorithm, MinerFirmware, MinerMake, MinerModel};
use crate::data::fan::FanData as FanData_Base;
use crate::data::miner::MinerData as MinerData_Base;
use crate::data::pool::{PoolGroupData, PoolURL};
use crate::data::{device::DeviceInfo, hashrate::HashRate, message::MinerMessage};
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, time::Duration};

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChipData {
    pub position: u16,
    pub hashrate: Option<HashRate>,
    pub temperature: Option<f64>,
    pub voltage: Option<f64>,
    pub frequency: Option<f64>,
    pub tuned: Option<bool>,
    pub working: Option<bool>,
}

impl From<&ChipData_Base> for ChipData {
    fn from(base: &ChipData_Base) -> Self {
        Self {
            position: base.position,
            hashrate: base.hashrate.clone(),
            temperature: base.temperature.map(|t| t.as_celsius()),
            voltage: base.voltage.map(|v| v.as_volts()),
            frequency: base.frequency.map(|f| f.as_megahertz()),
            tuned: base.tuned,
            working: base.working,
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BoardData {
    pub position: u8,
    pub hashrate: Option<HashRate>,
    pub expected_hashrate: Option<HashRate>,
    pub board_temperature: Option<f64>,
    pub intake_temperature: Option<f64>,
    pub outlet_temperature: Option<f64>,
    pub expected_chips: Option<u16>,
    pub working_chips: Option<u16>,
    pub serial_number: Option<String>,
    pub chips: Vec<ChipData>,
    pub voltage: Option<f64>,
    pub frequency: Option<f64>,
    pub tuned: Option<bool>,
    pub active: Option<bool>,
}

impl From<&BoardData_Base> for BoardData {
    fn from(base: &BoardData_Base) -> Self {
        Self {
            position: base.position,
            hashrate: base.hashrate.clone(),
            expected_hashrate: base.expected_hashrate.clone(),
            board_temperature: base.board_temperature.map(|t| t.as_celsius()),
            intake_temperature: base.intake_temperature.map(|t| t.as_celsius()),
            outlet_temperature: base.outlet_temperature.map(|t| t.as_celsius()),
            expected_chips: base.expected_chips,
            working_chips: base.working_chips,
            serial_number: base.serial_number.clone(),
            chips: base.chips.iter().map(ChipData::from).collect(),
            voltage: base.voltage.map(|v| v.as_volts()),
            frequency: base.frequency.map(|f| f.as_megahertz()),
            tuned: base.tuned,
            active: base.active,
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FanData {
    pub position: i16,
    pub rpm: Option<f64>,
}

impl From<&FanData_Base> for FanData {
    fn from(base: &FanData_Base) -> Self {
        Self {
            position: base.position,
            rpm: base.rpm.map(|r| r.as_rpm()),
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinerData {
    pub schema_version: String,
    pub timestamp: u64,
    pub ip: IpAddr,
    pub mac: Option<String>,
    pub device_info: DeviceInfo,
    pub serial_number: Option<String>,
    pub hostname: Option<String>,
    pub api_version: Option<String>,
    pub firmware_version: Option<String>,
    pub control_board_version: Option<String>,
    pub expected_hashboards: Option<u8>,
    pub hashboards: Vec<BoardData>,
    pub hashrate: Option<HashRate>,
    pub expected_hashrate: Option<HashRate>,
    pub expected_chips: Option<u16>,
    pub total_chips: Option<u16>,
    pub expected_fans: Option<u8>,
    pub fans: Vec<FanData>,
    pub psu_fans: Vec<FanData>,
    pub average_temperature: Option<f64>,
    pub fluid_temperature: Option<f64>,
    pub wattage: Option<f64>,
    pub wattage_limit: Option<f64>,
    pub efficiency: Option<f64>,
    pub light_flashing: Option<bool>,
    pub messages: Vec<MinerMessage>,
    pub uptime: Option<Duration>,
    pub is_mining: bool,
    pub pools: Vec<PoolGroupData>,
}

impl From<&MinerData_Base> for MinerData {
    fn from(base: &MinerData_Base) -> Self {
        Self {
            schema_version: base.schema_version.clone(),
            timestamp: base.timestamp,
            ip: base.ip,
            mac: base.mac.map(|m| m.to_string()),
            device_info: base.device_info,
            serial_number: base.serial_number.clone(),
            hostname: base.hostname.clone(),
            api_version: base.api_version.clone(),
            firmware_version: base.firmware_version.clone(),
            control_board_version: base.control_board_version.clone().map(|cb| cb.to_string()),
            expected_hashboards: base.expected_hashboards,
            hashboards: base.hashboards.iter().map(BoardData::from).collect(),
            hashrate: base.hashrate.clone(),
            expected_hashrate: base.expected_hashrate.clone(),
            expected_chips: base.expected_chips,
            total_chips: base.total_chips,
            expected_fans: base.expected_fans,
            fans: base.fans.iter().map(FanData::from).collect(),
            psu_fans: base.psu_fans.iter().map(FanData::from).collect(),
            average_temperature: base.average_temperature.map(|t| t.as_celsius()),
            fluid_temperature: base.fluid_temperature.map(|t| t.as_celsius()),
            wattage: base.wattage.map(|w| w.as_watts()),
            wattage_limit: base.wattage_limit.map(|w| w.as_watts()),
            efficiency: base.efficiency,
            light_flashing: base.light_flashing,
            messages: base.messages.clone(),
            uptime: base.uptime,
            is_mining: base.is_mining,
            pools: base.pools.clone(),
        }
    }
}

#[pymethods]
impl MinerData {
    pub fn __repr__<'a>(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[pymethods]
impl MinerModel {
    pub fn __repr__<'a>(&self) -> String {
        self.to_string()
    }
}

#[pymethods]
impl MinerMake {
    pub fn __repr__<'a>(&self) -> String {
        self.to_string()
    }
}

#[pymethods]
impl MinerFirmware {
    pub fn __repr__<'a>(&self) -> String {
        self.to_string()
    }
}

#[pymethods]
impl HashAlgorithm {
    pub fn __repr__<'a>(&self) -> String {
        self.to_string()
    }
}

#[pymethods]
impl PoolURL {
    pub fn __repr__<'a>(&self) -> String {
        self.to_string()
    }
}

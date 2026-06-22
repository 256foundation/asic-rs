use std::{net::IpAddr, time::Duration};

use macaddr::MacAddr;
use measurements::{Power, Temperature};
#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    board::{BoardData, MinerControlBoard},
    capabilities::TuningCapabilities,
    device::DeviceInfo,
    fan::FanData,
    hashrate::HashRate,
    message::MinerMessage,
    pool::PoolGroupData,
};
use crate::data::{
    deserialize::deserialize_macaddr,
    serialize::{serialize_macaddr, serialize_power, serialize_temperature},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Firmware tuning target reported by a miner or requested by configuration.
pub enum TuningTarget {
    /// Target a power limit.
    Power(Power),
    /// Target a hashrate.
    HashRate(HashRate),
    /// Target a named mining mode.
    MiningMode(MiningMode),
    /// Target a named firmware preset (e.g. VNish autotune presets).
    Preset(String),
}

impl TuningTarget {
    /// Create a power tuning target from watts.
    pub fn from_watts(watts: f64) -> Self {
        TuningTarget::Power(Power::from_watts(watts))
    }
}

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[cfg_attr(feature = "python", derive(asic_rs_pydantic::PyPydanticEnum))]
#[derive(
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
/// Firmware-defined mining performance mode.
pub enum MiningMode {
    /// Lower-power or quiet mining mode.
    #[cfg_attr(feature = "python", pydantic(value = "Low"))]
    Low,
    /// Normal mining mode.
    #[cfg_attr(feature = "python", pydantic(value = "Normal"))]
    Normal,
    /// High-performance mining mode.
    #[cfg_attr(feature = "python", pydantic(value = "High"))]
    High,
}

#[cfg_attr(feature = "python", pyclass(from_py_object, module = "asic_rs"))]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model(getters))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Standardized telemetry snapshot for one miner.
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
    /// The coolant exhaust temperature, only for water-cooled miners with dedicated sensors
    #[serde(serialize_with = "serialize_temperature")]
    pub outlet_fluid_temperature: Option<Temperature>,
    /// The current power consumption of the miner
    #[serde(serialize_with = "serialize_power")]
    pub wattage: Option<Power>,
    /// The current manual tuning percent of full power (100 = unthrottled), where supported
    pub tuning_percent: Option<u8>,
    /// The current tuning target of the miner, such as power target or hashrate target
    pub tuning_target: Option<TuningTarget>,
    /// The current tuning target adjusted by scaling settings, when available.
    pub scaled_tuning_target: Option<TuningTarget>,
    /// The factory tuning envelope the firmware exposes (default / min / max
    /// power and hashrate, or selectable presets), when reported.
    pub tuning_capabilities: Option<TuningCapabilities>,
    /// The current efficiency in W/TH/s (J/TH) of the miner
    pub efficiency: Option<f64>,
    /// The state of the fault/alert light on the miner
    pub light_flashing: Option<bool>,
    /// Any message on the miner, including errors
    pub messages: Vec<MinerMessage>,
    /// The total uptime of the miner's system
    pub uptime: Option<Duration>,
    /// Whether the hashing process is currently running
    pub is_mining: bool,
    /// The current pools configured on the miner
    pub pools: Vec<PoolGroupData>,
}

#[cfg(feature = "python")]
pub use python_tuning_target::PyTuningTarget;

#[cfg(feature = "python")]
mod python_tuning_target {
    use asic_rs_pydantic::{
        PyPydanticType, PydanticSchemaMode, get_required_field, literal_schema,
        pydantic_typed_dict_schema, tagged_union_schema,
    };
    use measurements::Power;
    use pyo3::{exceptions::PyValueError, prelude::*, types::PyAnyMethods};

    use super::{HashRate, MiningMode, TuningTarget};

    #[pyclass(name = "TuningTarget", skip_from_py_object, module = "asic_rs")]
    #[derive(Debug, Clone, PartialEq)]
    pub enum PyTuningTarget {
        Power { watts: f64 },
        HashRate { target_hashrate: HashRate },
        Mode { target_mode: MiningMode },
        Preset { name: String },
    }

    #[pymethods]
    impl PyTuningTarget {
        #[staticmethod]
        fn power(watts: f64) -> Self {
            Self::Power { watts }
        }

        #[staticmethod]
        fn hashrate(hashrate: HashRate) -> Self {
            Self::HashRate {
                target_hashrate: hashrate,
            }
        }

        #[staticmethod]
        fn mode(mode: MiningMode) -> Self {
            Self::Mode { target_mode: mode }
        }

        #[staticmethod]
        fn preset(name: String) -> Self {
            Self::Preset { name }
        }

        #[getter]
        fn variant(&self) -> &'static str {
            match self {
                Self::Power { .. } => "power",
                Self::HashRate { .. } => "hashrate",
                Self::Mode { .. } => "mode",
                Self::Preset { .. } => "preset",
            }
        }

        #[getter]
        fn watts(&self) -> Option<f64> {
            match self {
                Self::Power { watts } => Some(*watts),
                _ => None,
            }
        }

        #[getter]
        #[pyo3(name = "target_hashrate")]
        fn py_target_hashrate(&self) -> Option<HashRate> {
            match self {
                Self::HashRate { target_hashrate } => Some(target_hashrate.clone()),
                _ => None,
            }
        }

        #[getter]
        #[pyo3(name = "target_mode")]
        fn py_target_mode(&self) -> Option<MiningMode> {
            match self {
                Self::Mode { target_mode } => Some(*target_mode),
                _ => None,
            }
        }

        #[getter]
        fn preset_name(&self) -> Option<String> {
            match self {
                Self::Preset { name } => Some(name.clone()),
                _ => None,
            }
        }

        fn __repr__(&self) -> String {
            match self {
                Self::Power { watts } => format!("TuningTarget.power(watts={watts:?})"),
                Self::HashRate { target_hashrate } => {
                    format!("TuningTarget.hashrate(hashrate={target_hashrate})")
                }
                Self::Mode { target_mode } => format!("TuningTarget.mode(mode={target_mode})"),
                Self::Preset { name } => format!("TuningTarget.preset(name={name:?})"),
            }
        }

        fn __str__(&self) -> String {
            self.__repr__()
        }
    }

    impl From<TuningTarget> for PyTuningTarget {
        fn from(value: TuningTarget) -> Self {
            match value {
                TuningTarget::Power(power) => Self::Power {
                    watts: power.as_watts(),
                },
                TuningTarget::HashRate(hashrate) => Self::HashRate {
                    target_hashrate: hashrate,
                },
                TuningTarget::MiningMode(mode) => Self::Mode { target_mode: mode },
                TuningTarget::Preset(name) => Self::Preset { name },
            }
        }
    }

    impl From<PyTuningTarget> for TuningTarget {
        fn from(value: PyTuningTarget) -> Self {
            match value {
                PyTuningTarget::Power { watts } => TuningTarget::Power(Power::from_watts(watts)),
                PyTuningTarget::HashRate { target_hashrate } => {
                    TuningTarget::HashRate(target_hashrate)
                }
                PyTuningTarget::Mode { target_mode } => TuningTarget::MiningMode(target_mode),
                PyTuningTarget::Preset { name } => TuningTarget::Preset(name),
            }
        }
    }

    impl<'py> pyo3::IntoPyObject<'py> for TuningTarget {
        type Target = pyo3::PyAny;
        type Output = pyo3::Bound<'py, pyo3::PyAny>;
        type Error = pyo3::PyErr;

        const OUTPUT_TYPE: pyo3::inspect::PyStaticExpr =
            { <PyTuningTarget as pyo3::PyTypeInfo>::TYPE_HINT };

        fn into_pyobject(self, py: pyo3::Python<'py>) -> Result<Self::Output, Self::Error> {
            PyTuningTarget::from(self)
                .into_pyobject(py)
                .map(pyo3::Bound::into_any)
        }
    }

    impl PyPydanticType for TuningTarget {
        fn pydantic_schema<'py>(
            core_schema: &Bound<'py, PyAny>,
            mode: PydanticSchemaMode,
        ) -> PyResult<Bound<'py, PyAny>> {
            let power_schema = pydantic_typed_dict_schema!(core_schema, "asic_rs.TuningTargetPower", {
                "type" => required(literal_schema(core_schema, &["power"])?),
                "value" => required(<Power as PyPydanticType>::pydantic_schema(core_schema, mode)?),
            })?;
            let hashrate_schema = pydantic_typed_dict_schema!(core_schema, "asic_rs.TuningTargetHashRate", {
                "type" => required(literal_schema(core_schema, &["hashrate"])?),
                "value" => required(<HashRate as PyPydanticType>::pydantic_schema(core_schema, mode)?),
            })?;
            let mode_schema = pydantic_typed_dict_schema!(core_schema, "asic_rs.TuningTargetMode", {
                "type" => required(literal_schema(core_schema, &["mode"])?),
                "value" => required(<MiningMode as PyPydanticType>::pydantic_schema(core_schema, mode)?),
            })?;
            let preset_schema = pydantic_typed_dict_schema!(core_schema, "asic_rs.TuningTargetPreset", {
                "type" => required(literal_schema(core_schema, &["preset"])?),
                "value" => required(<String as PyPydanticType>::pydantic_schema(core_schema, mode)?),
            })?;
            let tagged_union = tagged_union_schema(
                core_schema,
                [
                    ("power", power_schema),
                    ("hashrate", hashrate_schema),
                    ("mode", mode_schema),
                    ("preset", preset_schema),
                ],
                "type",
                Some("asic_rs.TuningTarget"),
            )?;
            if mode == PydanticSchemaMode::Serialization {
                return Ok(tagged_union);
            }
            let target_instance = core_schema.call_method1(
                "is_instance_schema",
                (core_schema.py().get_type::<PyTuningTarget>(),),
            )?;
            asic_rs_pydantic::union_schema(core_schema, [target_instance, tagged_union])
        }

        fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
            if let Ok(target) = value.extract::<PyRef<'_, PyTuningTarget>>() {
                return Ok(target.clone().into());
            }
            let type_str: String = get_required_field(value, "type")?.extract()?;
            let v = get_required_field(value, "value")?;
            match type_str.as_str() {
                "power" => Ok(TuningTarget::Power(
                    <Power as PyPydanticType>::from_pydantic(&v)?,
                )),
                "hashrate" => Ok(TuningTarget::HashRate(
                    <HashRate as PyPydanticType>::from_pydantic(&v)?,
                )),
                "mode" => Ok(TuningTarget::MiningMode(
                    <MiningMode as PyPydanticType>::from_pydantic(&v)?,
                )),
                "preset" => Ok(TuningTarget::Preset(
                    <String as PyPydanticType>::from_pydantic(&v)?,
                )),
                _ => Err(PyValueError::new_err(format!(
                    "Unknown TuningTarget type '{type_str}', expected 'power', 'hashrate', 'mode', or 'preset'"
                ))),
            }
        }

        fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
            use pyo3::types::{PyDict, PyDictMethods};
            let dict = PyDict::new(py);
            match self {
                TuningTarget::Power(p) => {
                    dict.set_item("type", "power")?;
                    dict.set_item("value", <Power as PyPydanticType>::to_pydantic_data(p, py)?)?;
                }
                TuningTarget::HashRate(hr) => {
                    dict.set_item("type", "hashrate")?;
                    dict.set_item(
                        "value",
                        <HashRate as PyPydanticType>::to_pydantic_data(hr, py)?,
                    )?;
                }
                TuningTarget::MiningMode(m) => {
                    dict.set_item("type", "mode")?;
                    dict.set_item(
                        "value",
                        <MiningMode as PyPydanticType>::to_pydantic_data(m, py)?,
                    )?;
                }
                TuningTarget::Preset(name) => {
                    dict.set_item("type", "preset")?;
                    dict.set_item(
                        "value",
                        <String as PyPydanticType>::to_pydantic_data(name, py)?,
                    )?;
                }
            }
            Ok(dict.into_any().unbind())
        }

        fn to_pydantic_repr_value(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
            use pyo3::IntoPyObject as _;
            PyTuningTarget::from(self.clone())
                .into_pyobject(py)
                .map(|b| b.into_any().unbind())
        }
    }
}

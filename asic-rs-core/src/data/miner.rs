use std::{collections::HashMap, net::IpAddr, time::Duration};

use macaddr::MacAddr;
use measurements::{Frequency, Power, Temperature, Voltage};
#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{
    board::{BoardData, MinerControlBoard},
    capabilities::TuningCapabilities,
    device::DeviceInfo,
    fan::FanData,
    hashrate::HashRate,
    message::MinerMessage,
    operating_state::OperatingState,
    pool::PoolGroupData,
};
use crate::data::{
    deserialize::deserialize_macaddr,
    serialize::{serialize_macaddr, serialize_power, serialize_temperature},
};

/// Manual set points by board ID: (frequency, voltage). Missing measurements
/// remain `None`; an empty map means no board set points were reported.
pub type ManualTuningTargets = HashMap<u8, (Option<Frequency>, Option<Voltage>)>;

/// Scalar manual set points by board ID: (megahertz, volts).
pub type ManualTuningValues = HashMap<u8, (Option<f64>, Option<f64>)>;

pub(crate) fn manual_targets_to_values(targets: &ManualTuningTargets) -> ManualTuningValues {
    targets
        .iter()
        .map(|(&id, &(frequency, voltage))| {
            (
                id,
                (
                    frequency.map(|f| f.as_megahertz()),
                    voltage.map(|v| v.as_volts()),
                ),
            )
        })
        .collect()
}

pub(crate) fn manual_targets_from_values(values: ManualTuningValues) -> ManualTuningTargets {
    values
        .into_iter()
        .map(|(id, (frequency, voltage))| {
            (
                id,
                (
                    frequency.map(Frequency::from_megahertz),
                    voltage.map(Voltage::from_volts),
                ),
            )
        })
        .collect()
}

mod manual_targets_serde {
    use super::*;

    pub fn serialize<S: serde::Serializer>(
        targets: &ManualTuningTargets,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        manual_targets_to_values(targets).serialize(serializer)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<ManualTuningTargets, D::Error> {
        ManualTuningValues::deserialize(deserializer).map(manual_targets_from_values)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
/// Firmware tuning target reported by a miner or requested by configuration.
///
/// [`TuningTarget::Manual`] means perpetual/autotuning is disabled. Every other
/// variant represents a firmware-managed tuning target and implies that tuning
/// is enabled.
pub enum TuningTarget {
    /// Use manual tuning with perpetual tuning disabled.
    ///
    /// Set points are optional because some firmware responses report that
    /// tuning is disabled without reporting one or both configured values.
    Manual {
        /// Board IDs mapped to (frequency, voltage), serialized as (MHz, volts).
        #[serde(default, with = "manual_targets_serde")]
        #[ts(type = "Record<number, [number | null, number | null]>")]
        boards: ManualTuningTargets,
    },
    /// Target a power limit.
    Power(#[ts(type = "{ watts: number }")] Power),
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
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, strum::Display, strum::EnumString, TS,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
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
    #[ts(type = "string | null")]
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
    #[ts(type = "number | null")]
    pub average_temperature: Option<Temperature>,
    /// The environment temperature of the miner, such as air temperature or immersion fluid temperature
    #[serde(serialize_with = "serialize_temperature")]
    #[ts(type = "number | null")]
    pub fluid_temperature: Option<Temperature>,
    /// The coolant exhaust temperature, only for water-cooled miners with dedicated sensors
    #[serde(serialize_with = "serialize_temperature")]
    #[ts(type = "number | null")]
    pub outlet_fluid_temperature: Option<Temperature>,
    /// The current power consumption of the miner
    #[serde(serialize_with = "serialize_power")]
    #[ts(type = "number | null")]
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
    #[ts(type = "{ secs: number, nanos: number } | null")]
    pub uptime: Option<Duration>,
    /// Whether the hashing process is currently running,
    /// false if paused, true if running, even if the hashrate is 0
    /// or the firmware is tuning. This legacy flag may default when data is
    /// missing; use `operating_state` for explicit detailed state telemetry.
    pub is_mining: bool,
    /// Detailed state explicitly reported by firmware, independent of `is_mining`.
    /// Unavailable, invalid, and boolean-only status responses leave this `None`.
    #[serde(default)]
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub operating_state: Option<OperatingState>,
    /// The current pools configured on the miner
    pub pools: Vec<PoolGroupData>,
    /// Difficulty of the best share found over the miner's lifetime, when reported.
    ///
    /// This is a dimensionless share difficulty (not hashrate). Firmware that
    /// only expose a single best-share value should populate this field and
    /// leave [`Self::session_best_share`] as `None`.
    pub best_share: Option<f64>,
    /// Difficulty of the best share found since the last boot or hashing start,
    /// when the firmware distinguishes session from all-time.
    pub session_best_share: Option<f64>,
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

    use super::{
        HashRate, ManualTuningValues, MiningMode, TuningTarget, manual_targets_from_values,
        manual_targets_to_values,
    };

    #[pyclass(name = "TuningTarget", skip_from_py_object, module = "asic_rs")]
    #[derive(Debug, Clone, PartialEq)]
    pub enum PyTuningTarget {
        Manual { boards: ManualTuningValues },
        Power { watts: f64 },
        HashRate { target_hashrate: HashRate },
        Mode { target_mode: MiningMode },
        Preset { name: String },
    }

    #[pymethods]
    impl PyTuningTarget {
        #[staticmethod]
        #[pyo3(signature = (boards = None))]
        fn manual(boards: Option<ManualTuningValues>) -> Self {
            Self::Manual {
                boards: boards.unwrap_or_default(),
            }
        }

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
                Self::Manual { .. } => "manual",
                Self::Power { .. } => "power",
                Self::HashRate { .. } => "hashrate",
                Self::Mode { .. } => "mode",
                Self::Preset { .. } => "preset",
            }
        }

        /// Board IDs mapped to (frequency in MHz, voltage in volts).
        #[getter]
        fn boards(&self) -> Option<ManualTuningValues> {
            match self {
                Self::Manual { boards } => Some(boards.clone()),
                _ => None,
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
                Self::Manual { boards } => {
                    let mut entries: Vec<_> = boards.iter().collect();
                    entries.sort_by_key(|(id, _)| **id);
                    let entries = entries
                        .into_iter()
                        .map(|(id, (frequency, voltage))| {
                            let frequency = frequency
                                .map(|v| format!("{v:?}"))
                                .unwrap_or_else(|| "None".to_owned());
                            let voltage = voltage
                                .map(|v| format!("{v:?}"))
                                .unwrap_or_else(|| "None".to_owned());
                            format!("{id}: ({frequency}, {voltage})")
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("TuningTarget.manual(boards={{{entries}}})")
                }
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
                TuningTarget::Manual { boards } => Self::Manual {
                    boards: manual_targets_to_values(&boards),
                },
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
                PyTuningTarget::Manual { boards } => TuningTarget::Manual {
                    boards: manual_targets_from_values(boards),
                },
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
            let number = <Option<f64> as PyPydanticType>::pydantic_schema(core_schema, mode)?;
            let pair = core_schema.call_method1("tuple_schema", (vec![number.clone(), number],))?;
            let key_options = pyo3::types::PyDict::new(core_schema.py());
            use pyo3::types::PyDictMethods;
            key_options.set_item("ge", 0)?;
            key_options.set_item("le", u8::MAX)?;
            let key = core_schema.call_method("int_schema", (), Some(&key_options))?;
            let manual_value_schema = core_schema.call_method1("dict_schema", (key, pair))?;
            let manual_schema = pydantic_typed_dict_schema!(core_schema, "asic_rs.TuningTargetManual", {
                "type" => required(literal_schema(core_schema, &["manual"])?),
                "value" => required(manual_value_schema),
            })?;
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
                    ("manual", manual_schema),
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
                "manual" => Ok(TuningTarget::Manual {
                    boards: manual_targets_from_values(v.extract()?),
                }),
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
                    "Unknown TuningTarget type '{type_str}', expected 'manual', 'power', 'hashrate', 'mode', or 'preset'"
                ))),
            }
        }

        fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
            use pyo3::types::{PyDict, PyDictMethods};
            let dict = PyDict::new(py);
            match self {
                TuningTarget::Manual { boards } => {
                    dict.set_item("type", "manual")?;
                    dict.set_item("value", manual_targets_to_values(boards))?;
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_tuning_target_serialization_round_trips() -> anyhow::Result<()> {
        let target = TuningTarget::Manual {
            boards: HashMap::from([
                (
                    0,
                    (
                        Some(Frequency::from_megahertz(480.0)),
                        Some(Voltage::from_volts(12.6)),
                    ),
                ),
                (
                    3,
                    (
                        Some(Frequency::from_megahertz(490.0)),
                        Some(Voltage::from_volts(12.7)),
                    ),
                ),
                (7, (None, Some(Voltage::from_volts(12.5)))),
                (8, (Some(Frequency::from_megahertz(500.0)), None)),
                (9, (None, None)),
            ]),
        };
        let encoded = serde_json::to_value(&target)?;
        assert_eq!(
            encoded,
            serde_json::json!({"Manual": {"boards": {
                "0": [480.0, 12.6], "3": [490.0, 12.7], "7": [null, 12.5],
                "8": [500.0, null], "9": [null, null],
            }}})
        );
        assert_eq!(target, serde_json::from_value(encoded)?);
        let empty = TuningTarget::Manual {
            boards: HashMap::new(),
        };
        assert_eq!(
            empty,
            serde_json::from_value(serde_json::to_value(&empty)?)?
        );
        Ok(())
    }
}

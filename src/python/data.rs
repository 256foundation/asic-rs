use std::{net::IpAddr, time::Duration};

pub use asic_rs_core::data::{
    board::MinerControlBoard,
    device::{DeviceInfo, MinerHardware},
};
use asic_rs_core::data::{
    board::{BoardData as BoardData_Base, ChipData as ChipData_Base},
    fan::FanData as FanData_Base,
    hashrate::HashRate,
    message::MinerMessage as MinerMessage_Base,
    miner::{MinerData as MinerData_Base, MiningMode, TuningTarget as TuningTargetBase},
    pool::{PoolData as PoolData_Base, PoolGroupData as PoolGroupData_Base},
};
use asic_rs_pydantic::{
    PyPydanticType, PydanticSchemaMode, literal_schema as pydantic_literal_schema,
    model_core_schema as pydantic_model_core_schema,
    model_json_schema as pydantic_model_json_schema, reject_model_kwargs, required_dict_item,
    tagged_union_schema,
};
#[cfg(feature = "python")]
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyAnyMethods, PyDict, PyType},
};
use serde::{Deserialize, Serialize};

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolData {
    pub position: Option<u16>,
    pub url: Option<String>,
    pub accepted_shares: Option<u64>,
    pub rejected_shares: Option<u64>,
    pub active: Option<bool>,
    pub alive: Option<bool>,
    pub user: Option<String>,
}

impl From<&PoolData_Base> for PoolData {
    fn from(base: &PoolData_Base) -> Self {
        Self {
            position: base.position,
            url: base.url.as_ref().map(ToString::to_string),
            accepted_shares: base.accepted_shares,
            rejected_shares: base.rejected_shares,
            active: base.active,
            alive: base.alive,
            user: base.user.clone(),
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolGroupData {
    pub name: String,
    pub quota: u32,
    pub pools: Vec<PoolData>,
}

impl From<&PoolGroupData_Base> for PoolGroupData {
    fn from(base: &PoolGroupData_Base) -> Self {
        Self {
            name: base.name.clone(),
            quota: base.quota,
            pools: base.pools.iter().map(PoolData::from).collect(),
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinerMessage {
    pub timestamp: u32,
    pub code: u64,
    pub message: String,
    pub severity: String,
}

impl From<&MinerMessage_Base> for MinerMessage {
    fn from(base: &MinerMessage_Base) -> Self {
        Self {
            timestamp: base.timestamp,
            code: base.code,
            message: base.message.clone(),
            severity: base.severity.to_string(),
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
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
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
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
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FanData {
    pub position: i16,
    pub rpm: Option<f64>,
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TuningTarget {
    Power { watts: f64 },
    HashRate { hashrate: HashRate },
    MiningMode { mode: MiningMode },
}

#[cfg(feature = "python")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuningTargetKind {
    Any,
    Power,
    HashRate,
    MiningMode,
}

#[cfg(feature = "python")]
impl TuningTargetKind {
    const ANY_TYPE_NAME: &'static str = "TuningTarget";
    const POWER_TYPE_NAME: &'static str = "Power";
    const HASHRATE_TYPE_NAME: &'static str = "HashRate";
    const MINING_MODE_TYPE_NAME: &'static str = "MiningMode";

    fn from_type(cls: &Bound<'_, PyType>) -> PyResult<Self> {
        match cls.getattr("__name__")?.extract::<String>()?.as_str() {
            Self::ANY_TYPE_NAME => Ok(Self::Any),
            Self::POWER_TYPE_NAME => Ok(Self::Power),
            Self::HASHRATE_TYPE_NAME => Ok(Self::HashRate),
            Self::MINING_MODE_TYPE_NAME => Ok(Self::MiningMode),
            name => Err(PyValueError::new_err(format!(
                "Unsupported TuningTarget type: {name}"
            ))),
        }
    }

    fn ensure_accepts(self, actual: Self) -> PyResult<()> {
        if self == Self::Any || self == actual {
            Ok(())
        } else {
            Err(PyValueError::new_err(format!(
                "Expected {self}, got {actual}"
            )))
        }
    }
}

#[cfg(feature = "python")]
impl std::fmt::Display for TuningTargetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Any => write!(f, "TuningTarget"),
            Self::Power => write!(f, "TuningTarget.Power"),
            Self::HashRate => write!(f, "TuningTarget.HashRate"),
            Self::MiningMode => write!(f, "TuningTarget.MiningMode"),
        }
    }
}

impl From<&TuningTargetBase> for TuningTarget {
    fn from(base: &TuningTargetBase) -> Self {
        match base {
            TuningTargetBase::Power(power) => TuningTarget::Power {
                watts: power.as_watts(),
            },
            TuningTargetBase::HashRate(hashrate) => TuningTarget::HashRate {
                hashrate: hashrate.clone(),
            },
            TuningTargetBase::MiningMode(mode) => TuningTarget::MiningMode { mode: *mode },
        }
    }
}

#[cfg(feature = "python")]
impl TuningTarget {
    fn from_py_for_pydantic(
        cls: &Bound<'_, PyType>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let target = Self::parse_py_for_pydantic(cls, value)?;
        Ok(target.into_pyobject(value.py())?.into_any().unbind())
    }

    fn parse_py_for_pydantic(cls: &Bound<'_, PyType>, value: &Bound<'_, PyAny>) -> PyResult<Self> {
        if value.is_instance(cls)? {
            return value
                .extract::<Self>()
                .map_err(|error| -> PyErr { error.into() });
        }

        let expected = TuningTargetKind::from_type(cls)?;
        let dict = value
            .cast::<PyDict>()
            .map_err(|_| PyValueError::new_err("Expected TuningTarget tagged dict"))?;
        let target_type = required_dict_item(dict, "type")?.extract::<String>()?;
        let value = required_dict_item(dict, "value")?;

        match target_type.as_str() {
            "power" => {
                expected.ensure_accepts(TuningTargetKind::Power)?;
                Ok(Self::Power {
                    watts: value.extract()?,
                })
            }
            "hashrate" => {
                expected.ensure_accepts(TuningTargetKind::HashRate)?;
                Ok(Self::HashRate {
                    hashrate: HashRate::from_pydantic(&value)?,
                })
            }
            "mode" => {
                expected.ensure_accepts(TuningTargetKind::MiningMode)?;
                Ok(Self::MiningMode {
                    mode: parse_mining_mode_for_pydantic(&value)?,
                })
            }
            target_type => Err(PyValueError::new_err(format!(
                "Unknown tuning target type: {target_type}"
            ))),
        }
    }

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        match self {
            TuningTarget::Power { watts } => {
                dict.set_item("type", "power")?;
                dict.set_item("value", watts)?;
            }
            TuningTarget::HashRate { hashrate } => {
                dict.set_item("type", "hashrate")?;
                dict.set_item("value", hashrate.to_pydantic_data(py)?)?;
            }
            TuningTarget::MiningMode { mode } => {
                dict.set_item("type", "mode")?;
                dict.set_item("value", mode.to_string())?;
            }
        }
        Ok(dict.into_any().unbind())
    }

    fn repr(&self, py: Python<'_>) -> PyResult<String> {
        match self {
            TuningTarget::Power { watts } => Ok(format!("TuningTarget.Power(watts={watts:?})")),
            TuningTarget::HashRate { hashrate } => {
                let hashrate_repr: String = hashrate
                    .clone()
                    .into_pyobject(py)?
                    .into_any()
                    .repr()?
                    .extract()?;
                Ok(format!("TuningTarget.HashRate(hashrate={hashrate_repr})"))
            }
            TuningTarget::MiningMode { mode } => {
                let mode_repr: String = mode.to_pydantic_data(py)?.bind(py).repr()?.extract()?;
                Ok(format!("TuningTarget.MiningMode(mode={mode_repr})"))
            }
        }
    }
}

#[cfg(feature = "python")]
impl PyPydanticType for TuningTarget {
    fn pydantic_schema<'py>(
        core_schema: &Bound<'py, PyAny>,
        mode: PydanticSchemaMode,
    ) -> PyResult<Bound<'py, PyAny>> {
        pydantic_tuning_target_schema(
            core_schema,
            &core_schema.py().get_type::<TuningTarget>(),
            mode,
        )
    }

    fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(target) = value.extract::<Self>() {
            return Ok(target);
        }
        Self::parse_py_for_pydantic(&value.py().get_type::<TuningTarget>(), value)
    }

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.to_pydantic_data(py)
    }

    fn to_pydantic_repr_value(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self.clone().into_pyobject(py)?.into_any().unbind())
    }
}

#[cfg(feature = "python")]
fn parse_mining_mode_for_pydantic(value: &Bound<'_, PyAny>) -> PyResult<MiningMode> {
    if let Ok(mode) = value.extract::<MiningMode>() {
        return Ok(mode);
    }
    if let Ok(mode) = value.extract::<String>() {
        return match mode.as_str() {
            "Low" => Ok(MiningMode::Low),
            "Normal" => Ok(MiningMode::Normal),
            "High" => Ok(MiningMode::High),
            mode => Err(PyValueError::new_err(format!(
                "Unknown mining mode: {mode}"
            ))),
        };
    }

    Err(PyValueError::new_err("Expected MiningMode data"))
}

#[cfg(feature = "python")]
fn pydantic_power_schema<'py>(core_schema: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let type_literal = pydantic_literal_schema(core_schema, &["power"])?;
    let value_schema = core_schema.call_method0("float_schema")?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.TuningTargetPower", {
        "type" => required(type_literal),
        "value" => required(value_schema),
    })
}

#[cfg(feature = "python")]
fn pydantic_hashrate_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    mode: PydanticSchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let type_literal = pydantic_literal_schema(core_schema, &["hashrate"])?;
    let value_schema = HashRate::pydantic_schema(core_schema, mode)?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.TuningTargetHashRate", {
        "type" => required(type_literal),
        "value" => required(value_schema),
    })
}

#[cfg(feature = "python")]
fn pydantic_mining_mode_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let type_literal = pydantic_literal_schema(core_schema, &["mode"])?;
    let value_schema = pydantic_literal_schema(core_schema, &["Low", "Normal", "High"])?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.TuningTargetMiningMode", {
        "type" => required(type_literal),
        "value" => required(value_schema),
    })
}

#[cfg(feature = "python")]
fn pydantic_tuning_target_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    cls: &Bound<'_, PyType>,
    mode: PydanticSchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    match TuningTargetKind::from_type(cls)? {
        TuningTargetKind::Any => tagged_union_schema(
            core_schema,
            [
                ("power", pydantic_power_schema(core_schema)?),
                ("hashrate", pydantic_hashrate_schema(core_schema, mode)?),
                ("mode", pydantic_mining_mode_schema(core_schema)?),
            ],
            "type",
            Some("asic_rs.TuningTarget"),
        ),
        TuningTargetKind::Power => pydantic_power_schema(core_schema),
        TuningTargetKind::HashRate => pydantic_hashrate_schema(core_schema, mode),
        TuningTargetKind::MiningMode => pydantic_mining_mode_schema(core_schema),
    }
}

#[pymethods]
impl TuningTarget {
    #[classmethod]
    #[pyo3(signature = (_source_type: "object", _handler: "object") -> "object")]
    pub fn __get_pydantic_core_schema__(
        cls: &Bound<'_, PyType>,
        _source_type: &Bound<'_, PyAny>,
        _handler: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let core_schema = cls.py().import("pydantic_core")?.getattr("core_schema")?;
        let validation_schema =
            pydantic_tuning_target_schema(&core_schema, cls, PydanticSchemaMode::Validation)?;
        let serialization_schema =
            pydantic_tuning_target_schema(&core_schema, cls, PydanticSchemaMode::Serialization)?;
        pydantic_model_core_schema(cls, &validation_schema, &serialization_schema)
    }

    #[classmethod]
    #[pyo3(signature = (value: "object") -> "TuningTarget.Power | TuningTarget.HashRate | TuningTarget.MiningMode")]
    fn _pydantic_validate(
        cls: &Bound<'_, PyType>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        Self::from_py_for_pydantic(cls, value)
    }

    #[staticmethod]
    #[pyo3(signature = (value: "TuningTarget.Power | TuningTarget.HashRate | TuningTarget.MiningMode") -> "dict[str, object]")]
    fn _pydantic_serialize(value: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let tuning_target = value.extract::<Self>()?;
        tuning_target.to_pydantic_data(value.py())
    }

    #[classmethod]
    #[pyo3(signature = (obj: "object", **_kwargs: "object") -> "TuningTarget.Power | TuningTarget.HashRate | TuningTarget.MiningMode")]
    pub fn model_validate(
        cls: &Bound<'_, PyType>,
        obj: &Bound<'_, PyAny>,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        reject_model_kwargs(_kwargs, "model_validate")?;
        Self::from_py_for_pydantic(cls, obj)
    }

    #[classmethod]
    #[pyo3(signature = (**kwargs: "object") -> "dict[str, object]")]
    pub fn model_json_schema(
        cls: &Bound<'_, PyType>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        pydantic_model_json_schema(cls, kwargs)
    }

    #[pyo3(signature = (**_kwargs: "object") -> "dict[str, object]")]
    pub fn model_dump(
        &self,
        py: Python<'_>,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        reject_model_kwargs(_kwargs, "model_dump")?;
        self.to_pydantic_data(py)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        self.repr(py)
    }
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
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
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
    pub control_board_version: Option<MinerControlBoard>,
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
    pub tuning_target: Option<TuningTarget>,
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
            device_info: base.device_info.clone(),
            serial_number: base.serial_number.clone(),
            hostname: base.hostname.clone(),
            api_version: base.api_version.clone(),
            firmware_version: base.firmware_version.clone(),
            control_board_version: base.control_board_version.clone(),
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
            tuning_target: base.tuning_target.as_ref().map(TuningTarget::from),
            efficiency: base.efficiency,
            light_flashing: base.light_flashing,
            messages: base.messages.iter().map(MinerMessage::from).collect(),
            uptime: base.uptime,
            is_mining: base.is_mining,
            pools: base.pools.iter().map(PoolGroupData::from).collect(),
        }
    }
}

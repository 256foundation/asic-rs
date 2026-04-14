use std::{net::IpAddr, time::Duration};

use asic_rs_core::data::{
    board::{
        BoardData as BoardData_Base, ChipData as ChipData_Base,
        MinerControlBoard as MinerControlBoard_Base,
    },
    device::{DeviceInfo as DeviceInfo_Base, MinerHardware as MinerHardware_Base},
    fan::FanData as FanData_Base,
    hashrate::HashRate,
    message::MinerMessage as MinerMessage_Base,
    miner::{MinerData as MinerData_Base, MiningMode, TuningTarget as TuningTargetBase},
    pool::{PoolData as PoolData_Base, PoolGroupData as PoolGroupData_Base},
};
use asic_rs_pydantic::{
    PyPydanticType, PydanticSchemaMode, get_required_field, list_schema as pydantic_list_schema,
    literal_schema as pydantic_literal_schema, model_core_schema as pydantic_model_core_schema,
    model_json_schema as pydantic_model_json_schema, parse_required_list, parse_required_option,
    py_to_string, reject_model_kwargs, required_dict_item, tagged_union_schema,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MinerHardware {
    pub chips: Option<u16>,
    pub fans: Option<u8>,
    pub boards: Option<u8>,
}

impl From<&MinerHardware_Base> for MinerHardware {
    fn from(base: &MinerHardware_Base) -> Self {
        Self {
            chips: base.chips,
            fans: base.fans,
            boards: base.boards,
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(
        schema = "pydantic_device_info_schema",
        parse = "parse_device_info"
    )
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub make: String,
    pub model: String,
    pub hardware: MinerHardware,
    pub firmware: String,
    pub algo: String,
}

impl From<&DeviceInfo_Base> for DeviceInfo {
    fn from(base: &DeviceInfo_Base) -> Self {
        Self {
            make: base.make.clone(),
            model: base.model.clone(),
            hardware: MinerHardware::from(&base.hardware),
            firmware: base.firmware.clone(),
            algo: base.algo.to_string(),
        }
    }
}

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
pub struct MinerControlBoard {
    pub known: bool,
    pub name: String,
}

impl From<&MinerControlBoard_Base> for MinerControlBoard {
    fn from(base: &MinerControlBoard_Base) -> Self {
        Self {
            known: base.known,
            name: base.name.clone(),
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
fn parse_required_duration_option(
    value: &Bound<'_, PyAny>,
    key: &str,
) -> PyResult<Option<Duration>> {
    let field = get_required_field(value, key)?;
    if field.is_none() {
        return Ok(None);
    }
    if let Ok(duration) = field.extract::<Duration>() {
        return Ok(Some(duration));
    }
    if let Ok(seconds) = field.extract::<f64>()
        && seconds.is_finite()
        && seconds >= 0.0
    {
        return Ok(Some(Duration::from_secs_f64(seconds)));
    }
    if let Ok(dict) = field.cast::<PyDict>() {
        let secs = required_dict_item(dict, "secs")?.extract::<u64>()?;
        return Ok(Some(Duration::from_secs(secs)));
    }
    Err(PyValueError::new_err(
        "Expected uptime as timedelta, non-negative seconds, or {secs} dict",
    ))
}

#[cfg(feature = "python")]
fn parse_ip_addr(value: &Bound<'_, PyAny>) -> PyResult<IpAddr> {
    if let Ok(ip) = value.extract::<IpAddr>() {
        return Ok(ip);
    }
    value
        .extract::<String>()?
        .parse()
        .map_err(|error| PyValueError::new_err(format!("Invalid IP address: {error}")))
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

#[cfg(feature = "python")]
fn pydantic_device_info_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    mode: PydanticSchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let str_schema = core_schema.call_method0("str_schema")?;
    let hardware_schema = MinerHardware::pydantic_schema(core_schema, mode)?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.DeviceInfo", {
        "make" => required(str_schema),
        "model" => required(str_schema),
        "hardware" => required(hardware_schema),
        "firmware" => required(str_schema),
        "algo" => required(str_schema),
    })
}

#[cfg(feature = "python")]
fn pydantic_miner_data_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    mode: PydanticSchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let py = core_schema.py();
    let str_schema = core_schema.call_method0("str_schema")?;
    let int_schema = core_schema.call_method0("int_schema")?;
    let float_schema = core_schema.call_method0("float_schema")?;
    let bool_schema = core_schema.call_method0("bool_schema")?;
    let hashrate_schema = HashRate::pydantic_schema(core_schema, mode)?;
    let device_info_schema = pydantic_device_info_schema(core_schema, mode)?;
    let control_board_schema = MinerControlBoard::pydantic_schema(core_schema, mode)?;
    let board_schema = BoardData::pydantic_schema(core_schema, mode)?;
    let fan_schema = FanData::pydantic_schema(core_schema, mode)?;
    let target_schema =
        pydantic_tuning_target_schema(core_schema, &py.get_type::<TuningTarget>(), mode)?;
    let message_schema = MinerMessage::pydantic_schema(core_schema, mode)?;
    let pool_group_schema = PoolGroupData::pydantic_schema(core_schema, mode)?;
    let boards_schema = pydantic_list_schema(core_schema, &board_schema)?;
    let fans_schema = pydantic_list_schema(core_schema, &fan_schema)?;
    let messages_schema = pydantic_list_schema(core_schema, &message_schema)?;
    let pools_schema = pydantic_list_schema(core_schema, &pool_group_schema)?;
    let uptime_schema = match mode {
        PydanticSchemaMode::Validation => core_schema.call_method0("any_schema")?,
        PydanticSchemaMode::Serialization => float_schema.clone(),
    };
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.MinerData", {
        "schema_version" => required(str_schema),
        "timestamp" => required(int_schema),
        "ip" => required(str_schema),
        "mac" => nullable(str_schema),
        "device_info" => required(device_info_schema),
        "serial_number" => nullable(str_schema),
        "hostname" => nullable(str_schema),
        "api_version" => nullable(str_schema),
        "firmware_version" => nullable(str_schema),
        "control_board_version" => nullable(control_board_schema),
        "expected_hashboards" => nullable(int_schema),
        "hashboards" => required(boards_schema),
        "hashrate" => nullable(hashrate_schema),
        "expected_hashrate" => nullable(hashrate_schema),
        "expected_chips" => nullable(int_schema),
        "total_chips" => nullable(int_schema),
        "expected_fans" => nullable(int_schema),
        "fans" => required(fans_schema),
        "psu_fans" => required(fans_schema),
        "average_temperature" => nullable(float_schema),
        "fluid_temperature" => nullable(float_schema),
        "wattage" => nullable(float_schema),
        "tuning_target" => nullable(target_schema),
        "efficiency" => nullable(float_schema),
        "light_flashing" => nullable(bool_schema),
        "messages" => required(messages_schema),
        "uptime" => nullable(uptime_schema),
        "is_mining" => required(bool_schema),
        "pools" => required(pools_schema),
    })
}

#[cfg(feature = "python")]
fn duration_to_seconds(duration: Duration) -> f64 {
    duration.as_secs() as f64
}

#[cfg(feature = "python")]
fn optional_duration_to_seconds(
    duration: &Option<Duration>,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    if let Some(duration) = duration {
        Ok(duration_to_seconds(*duration)
            .into_pyobject(py)?
            .into_any()
            .unbind())
    } else {
        Ok(py.None())
    }
}

#[cfg(feature = "python")]
fn parse_device_info(value: &Bound<'_, PyAny>) -> PyResult<DeviceInfo> {
    if let Ok(model) = value.extract::<DeviceInfo>() {
        return Ok(model);
    }
    Ok(DeviceInfo {
        make: get_required_field(value, "make")?.extract::<String>()?,
        model: get_required_field(value, "model")?.extract::<String>()?,
        hardware: MinerHardware::from_pydantic(&get_required_field(value, "hardware")?)?,
        firmware: get_required_field(value, "firmware")?.extract::<String>()?,
        algo: py_to_string(&get_required_field(value, "algo")?)?,
    })
}

#[cfg(feature = "python")]
fn parse_miner_data(value: &Bound<'_, PyAny>) -> PyResult<MinerData> {
    if let Ok(model) = value.extract::<MinerData>() {
        return Ok(model);
    }
    let hashboards = parse_required_list(value, "hashboards", BoardData::from_pydantic)?;
    let fans = parse_required_list(value, "fans", FanData::from_pydantic)?;
    let psu_fans = parse_required_list(value, "psu_fans", FanData::from_pydantic)?;
    let messages = parse_required_list(value, "messages", MinerMessage::from_pydantic)?;
    let pools = parse_required_list(value, "pools", PoolGroupData::from_pydantic)?;

    Ok(MinerData {
        schema_version: get_required_field(value, "schema_version")?.extract()?,
        timestamp: get_required_field(value, "timestamp")?.extract()?,
        ip: parse_ip_addr(&get_required_field(value, "ip")?)?,
        mac: parse_required_option(value, "mac")?,
        device_info: parse_device_info(&get_required_field(value, "device_info")?)?,
        serial_number: parse_required_option(value, "serial_number")?,
        hostname: parse_required_option(value, "hostname")?,
        api_version: parse_required_option(value, "api_version")?,
        firmware_version: parse_required_option(value, "firmware_version")?,
        control_board_version: Option::<MinerControlBoard>::from_pydantic(&get_required_field(
            value,
            "control_board_version",
        )?)?,
        expected_hashboards: parse_required_option(value, "expected_hashboards")?,
        hashboards,
        hashrate: Option::<HashRate>::from_pydantic(&get_required_field(value, "hashrate")?)?,
        expected_hashrate: Option::<HashRate>::from_pydantic(&get_required_field(
            value,
            "expected_hashrate",
        )?)?,
        expected_chips: parse_required_option(value, "expected_chips")?,
        total_chips: parse_required_option(value, "total_chips")?,
        expected_fans: parse_required_option(value, "expected_fans")?,
        fans,
        psu_fans,
        average_temperature: parse_required_option(value, "average_temperature")?,
        fluid_temperature: parse_required_option(value, "fluid_temperature")?,
        wattage: parse_required_option(value, "wattage")?,
        tuning_target: Option::<TuningTarget>::from_pydantic(&get_required_field(
            value,
            "tuning_target",
        )?)?,
        efficiency: parse_required_option(value, "efficiency")?,
        light_flashing: parse_required_option(value, "light_flashing")?,
        messages,
        uptime: parse_required_duration_option(value, "uptime")?,
        is_mining: get_required_field(value, "is_mining")?.extract()?,
        pools,
    })
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
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(
        schema = "pydantic_miner_data_schema",
        parse = "parse_miner_data"
    )
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinerData {
    pub schema_version: String,
    pub timestamp: u64,
    #[cfg_attr(feature = "python", pydantic_data(to_string))]
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
    #[cfg_attr(
        feature = "python",
        pydantic_data(with = "optional_duration_to_seconds")
    )]
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
            device_info: DeviceInfo::from(&base.device_info),
            serial_number: base.serial_number.clone(),
            hostname: base.hostname.clone(),
            api_version: base.api_version.clone(),
            firmware_version: base.firmware_version.clone(),
            control_board_version: base
                .control_board_version
                .as_ref()
                .map(MinerControlBoard::from),
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

use asic_rs_core::{
    config::{
        fan::FanConfig as FanConfig_Base,
        pools::{PoolConfig as PoolConfig_Base, PoolGroupConfig as PoolGroupConfig_Base},
        scaling::ScalingConfig as ScalingConfig_Base,
        tuning::TuningConfig as TuningConfig_Base,
    },
    data::{
        hashrate::HashRate,
        miner::{MiningMode, TuningTarget as TuningTarget_Base},
    },
};
use asic_rs_pydantic::{
    PyPydanticType, PydanticSchemaMode as SchemaMode, model_core_schema, model_json_schema,
    py_to_string, reject_model_kwargs,
};
use pyo3::{
    prelude::*,
    types::{PyAnyMethods, PyDict, PyType},
};
use serde::{Deserialize, Serialize};

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pool {
    pub url: String,
    pub username: String,
    pub password: String,
}

impl From<PoolConfig_Base> for Pool {
    fn from(base: PoolConfig_Base) -> Self {
        Self {
            url: base.url.to_string(),
            username: base.username,
            password: base.password,
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolGroup {
    pub name: String,
    #[cfg_attr(feature = "python", pydantic(default = 1))]
    pub quota: u32,
    pub pools: Vec<Pool>,
}

impl From<PoolGroupConfig_Base> for PoolGroup {
    fn from(base: PoolGroupConfig_Base) -> Self {
        Self {
            name: base.name,
            quota: base.quota,
            pools: base.pools.into_iter().map(Pool::from).collect(),
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalingConfig {
    pub step: u32,
    pub minimum: u32,
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub shutdown: Option<bool>,
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub shutdown_duration: Option<f32>,
}

impl From<ScalingConfig_Base> for ScalingConfig {
    fn from(base: ScalingConfig_Base) -> Self {
        Self {
            step: base.step,
            minimum: base.minimum,
            shutdown: base.shutdown,
            shutdown_duration: base.shutdown_duration,
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuningConfigPower {
    #[cfg_attr(feature = "python", pydantic(literal = "power"))]
    pub variant: String,
    pub target_watts: f64,
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub algorithm: Option<String>,
}

impl TuningConfigPower {
    const VARIANT: &'static str = "power";

    fn from_parts(target_watts: f64, algorithm: Option<String>) -> Self {
        Self {
            variant: Self::VARIANT.to_owned(),
            target_watts,
            algorithm,
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuningConfigHashRate {
    #[cfg_attr(feature = "python", pydantic(literal = "hashrate"))]
    pub variant: String,
    pub target_hashrate: HashRate,
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub algorithm: Option<String>,
}

impl TuningConfigHashRate {
    const VARIANT: &'static str = "hashrate";

    fn from_parts(target_hashrate: HashRate, algorithm: Option<String>) -> Self {
        Self {
            variant: Self::VARIANT.to_owned(),
            target_hashrate,
            algorithm,
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuningConfigMode {
    #[cfg_attr(feature = "python", pydantic(literal = "mode"))]
    pub variant: String,
    pub target_mode: MiningMode,
}

impl TuningConfigMode {
    const VARIANT: &'static str = "mode";

    fn from_parts(target_mode: MiningMode) -> Self {
        Self {
            variant: Self::VARIANT.to_owned(),
            target_mode,
        }
    }
}

#[derive(Debug, Clone, PartialEq, asic_rs_pydantic::PyPydanticTaggedUnion)]
#[pydantic(discriminator = "variant", ref = "asic_rs.TuningConfig")]
pub enum TuningConfigVariant {
    #[pydantic(tag = "power")]
    Power(TuningConfigPower),
    #[pydantic(tag = "hashrate")]
    HashRate(TuningConfigHashRate),
    #[pydantic(tag = "mode")]
    Mode(TuningConfigMode),
}

impl From<TuningConfig_Base> for TuningConfigVariant {
    fn from(base: TuningConfig_Base) -> Self {
        let algorithm = base.algorithm;
        match base.target {
            TuningTarget_Base::Power(power) => {
                Self::Power(TuningConfigPower::from_parts(power.as_watts(), algorithm))
            }
            TuningTarget_Base::HashRate(target_hashrate) => {
                Self::HashRate(TuningConfigHashRate::from_parts(target_hashrate, algorithm))
            }
            TuningTarget_Base::MiningMode(target_mode) => {
                Self::Mode(TuningConfigMode::from_parts(target_mode))
            }
        }
    }
}

#[pyclass(module = "asic_rs")]
pub struct TuningConfig;

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoFanConfig {
    #[cfg_attr(feature = "python", pydantic(literal = "auto"))]
    pub mode: String,
    pub target_temp: f64,
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub idle_speed: Option<u64>,
}

impl AutoFanConfig {
    const MODE: &'static str = "auto";

    fn from_parts(target_temp: f64, idle_speed: Option<u64>) -> Self {
        Self {
            mode: Self::MODE.to_owned(),
            target_temp,
            idle_speed,
        }
    }
}

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualFanConfig {
    #[cfg_attr(feature = "python", pydantic(literal = "manual"))]
    pub mode: String,
    pub fan_speed: u64,
}

impl ManualFanConfig {
    const MODE: &'static str = "manual";

    fn from_parts(fan_speed: u64) -> Self {
        Self {
            mode: Self::MODE.to_owned(),
            fan_speed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, asic_rs_pydantic::PyPydanticTaggedUnion)]
#[pydantic(discriminator = "mode", ref = "asic_rs.FanConfig")]
pub enum FanConfig {
    #[pydantic(tag = "auto")]
    Auto(AutoFanConfig),
    #[pydantic(tag = "manual")]
    Manual(ManualFanConfig),
}

impl From<FanConfig_Base> for FanConfig {
    fn from(base: FanConfig_Base) -> Self {
        match base {
            FanConfig_Base::Auto {
                target_temp,
                idle_speed,
            } => Self::Auto(AutoFanConfig::from_parts(target_temp, idle_speed)),
            FanConfig_Base::Manual { fan_speed } => {
                Self::Manual(ManualFanConfig::from_parts(fan_speed))
            }
        }
    }
}

#[cfg(feature = "python")]
fn parse_mining_mode(value: &Bound<'_, PyAny>) -> PyResult<MiningMode> {
    MiningMode::from_pydantic(value)
}

#[pymethods]
impl TuningConfig {
    #[classmethod]
    #[pyo3(signature = (watts, *, algorithm: "HashAlgorithm | str | None" = None))]
    fn power(
        _cls: &Bound<'_, PyType>,
        watts: f64,
        algorithm: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<TuningConfigPower> {
        Ok(TuningConfigPower::from_parts(
            watts,
            algorithm.map(py_to_string).transpose()?,
        ))
    }

    #[classmethod]
    #[pyo3(signature = (hr, *, algorithm: "HashAlgorithm | str | None" = None))]
    fn hashrate(
        _cls: &Bound<'_, PyType>,
        hr: HashRate,
        algorithm: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<TuningConfigHashRate> {
        Ok(TuningConfigHashRate::from_parts(
            hr,
            algorithm.map(py_to_string).transpose()?,
        ))
    }

    #[classmethod]
    #[pyo3(signature = (mode: "MiningMode | str"))]
    fn mode(_cls: &Bound<'_, PyType>, mode: &Bound<'_, PyAny>) -> PyResult<TuningConfigMode> {
        Ok(TuningConfigMode::from_parts(parse_mining_mode(mode)?))
    }

    #[classmethod]
    #[pyo3(signature = (_source_type: "object", _handler: "object") -> "object")]
    fn __get_pydantic_core_schema__(
        cls: &Bound<'_, PyType>,
        _source_type: &Bound<'_, PyAny>,
        _handler: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let core_schema = cls.py().import("pydantic_core")?.getattr("core_schema")?;
        let validation_schema =
            TuningConfigVariant::pydantic_schema(&core_schema, SchemaMode::Validation)?;
        let serialization_schema =
            TuningConfigVariant::pydantic_schema(&core_schema, SchemaMode::Serialization)?;
        model_core_schema(cls, &validation_schema, &serialization_schema)
    }

    #[classmethod]
    #[pyo3(signature = (obj: "object", **_kwargs: "object") -> "TuningConfigPower | TuningConfigHashRate | TuningConfigMode")]
    fn model_validate(
        _cls: &Bound<'_, PyType>,
        obj: &Bound<'_, PyAny>,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        reject_model_kwargs(_kwargs, "model_validate")?;
        TuningConfigVariant::from_pydantic(obj)?
            .into_pyobject(obj.py())
            .map(Bound::unbind)
    }

    #[classmethod]
    #[pyo3(signature = (**kwargs: "object") -> "dict[str, object]")]
    fn model_json_schema(
        cls: &Bound<'_, PyType>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        model_json_schema(cls, kwargs)
    }

    #[classmethod]
    #[pyo3(signature = (obj: "TuningConfigPower | TuningConfigHashRate | TuningConfigMode", **_kwargs: "object") -> "dict[str, object]")]
    fn model_dump(
        _cls: &Bound<'_, PyType>,
        obj: &Bound<'_, PyAny>,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        reject_model_kwargs(_kwargs, "model_dump")?;
        TuningConfigVariant::from_pydantic(obj)?.to_pydantic_data(obj.py())
    }

    #[classmethod]
    #[pyo3(signature = (value: "object") -> "TuningConfigPower | TuningConfigHashRate | TuningConfigMode")]
    fn _pydantic_validate(
        cls: &Bound<'_, PyType>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        Self::model_validate(cls, value, None)
    }

    #[staticmethod]
    #[pyo3(signature = (value: "TuningConfigPower | TuningConfigHashRate | TuningConfigMode") -> "dict[str, object]")]
    fn _pydantic_serialize(value: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        TuningConfigVariant::from_pydantic(value)?.to_pydantic_data(value.py())
    }
}

#[pymethods]
impl Pool {
    #[new]
    #[pyo3(signature = (url: "str", username, password))]
    fn new(url: &Bound<'_, PyAny>, username: String, password: String) -> PyResult<Self> {
        Ok(Self {
            url: py_to_string(url)?,
            username,
            password,
        })
    }
}

#[pymethods]
impl PoolGroup {
    #[new]
    #[pyo3(signature = (name, pools, quota = 1))]
    fn new(name: String, pools: Vec<Pool>, quota: u32) -> Self {
        Self { name, quota, pools }
    }
}

#[pymethods]
impl ScalingConfig {
    #[new]
    #[pyo3(signature = (step, minimum, shutdown = None, shutdown_duration = None))]
    fn new(
        step: u32,
        minimum: u32,
        shutdown: Option<bool>,
        shutdown_duration: Option<f32>,
    ) -> Self {
        Self {
            step,
            minimum,
            shutdown,
            shutdown_duration,
        }
    }
}

#[pymethods]
impl TuningConfigPower {
    #[new]
    #[pyo3(signature = (target_watts, algorithm: "HashAlgorithm | str | None" = None))]
    fn new(target_watts: f64, algorithm: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        Ok(Self::from_parts(
            target_watts,
            algorithm.map(py_to_string).transpose()?,
        ))
    }
}

#[pymethods]
impl TuningConfigHashRate {
    #[new]
    #[pyo3(signature = (target_hashrate, algorithm: "HashAlgorithm | str | None" = None))]
    fn new(target_hashrate: HashRate, algorithm: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        Ok(Self::from_parts(
            target_hashrate,
            algorithm.map(py_to_string).transpose()?,
        ))
    }
}

#[pymethods]
impl TuningConfigMode {
    #[new]
    #[pyo3(signature = (target_mode: "MiningMode | str"))]
    fn new(target_mode: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::from_parts(parse_mining_mode(target_mode)?))
    }
}

#[pymethods]
impl AutoFanConfig {
    #[new]
    #[pyo3(signature = (target_temp, idle_speed = None))]
    fn new(target_temp: f64, idle_speed: Option<u64>) -> Self {
        Self::from_parts(target_temp, idle_speed)
    }
}

#[pymethods]
impl ManualFanConfig {
    #[new]
    #[pyo3(signature = (fan_speed))]
    fn new(fan_speed: u64) -> Self {
        Self::from_parts(fan_speed)
    }
}

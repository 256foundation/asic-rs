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
    PyPydanticType, PydanticSchemaMode as SchemaMode, get_optional_field as get_optional,
    get_required_field as get_required, list_schema as pydantic_list_schema, literal_schema,
    model_core_schema, model_json_schema, parse_optional, parse_required_list, py_to_string,
    reject_model_kwargs,
};
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyAnyMethods, PyDict, PyType},
};
use serde::{Deserialize, Serialize};

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(schema = "pool_schema", parse = "parse_pool")
)]
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
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(schema = "pool_group_schema", parse = "parse_pool_group")
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolGroup {
    pub name: String,
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
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(
        schema = "scaling_config_schema",
        parse = "parse_scaling_config"
    )
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalingConfig {
    pub step: u32,
    pub minimum: u32,
    pub shutdown: Option<bool>,
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
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(
        schema = "tuning_power_schema",
        parse = "parse_tuning_power"
    )
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuningConfigPower {
    pub variant: String,
    pub target_watts: f64,
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
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(
        schema = "tuning_hashrate_schema",
        parse = "parse_tuning_hashrate"
    )
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuningConfigHashRate {
    pub variant: String,
    pub target_hashrate: HashRate,
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
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(
        schema = "tuning_mode_schema",
        parse = "parse_tuning_mode"
    )
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuningConfigMode {
    pub variant: String,
    pub target_mode: String,
}

impl TuningConfigMode {
    const VARIANT: &'static str = "mode";

    fn from_parts(target_mode: String) -> Self {
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
                Self::Mode(TuningConfigMode::from_parts(target_mode.to_string()))
            }
        }
    }
}

#[pyclass(module = "asic_rs")]
pub struct TuningConfig;

#[pyclass(from_py_object, get_all, module = "asic_rs")]
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(schema = "auto_fan_schema", parse = "parse_auto_fan")
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoFanConfig {
    pub mode: String,
    pub target_temp: f64,
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
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(schema = "manual_fan_schema", parse = "parse_manual_fan")
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualFanConfig {
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
fn pool_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    _mode: SchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let str_schema = core_schema.call_method0("str_schema")?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.Pool", {
        "url" => required(str_schema),
        "username" => required(str_schema),
        "password" => required(str_schema),
    })
}

#[cfg(feature = "python")]
fn pool_group_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    mode: SchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let str_schema = core_schema.call_method0("str_schema")?;
    let int_schema = core_schema.call_method0("int_schema")?;
    let pool_schema = pool_schema(core_schema, mode)?;
    let pools_schema = pydantic_list_schema(core_schema, &pool_schema)?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.PoolGroup", {
        "name" => required(str_schema),
        "quota" => required_if(int_schema, mode == SchemaMode::Serialization),
        "pools" => required(pools_schema),
    })
}

#[cfg(feature = "python")]
fn scaling_config_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    mode: SchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let int_schema = core_schema.call_method0("int_schema")?;
    let bool_schema = core_schema.call_method0("bool_schema")?;
    let float_schema = core_schema.call_method0("float_schema")?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.ScalingConfig", {
        "step" => required(int_schema),
        "minimum" => required(int_schema),
        "shutdown" => nullable_if(bool_schema, mode == SchemaMode::Serialization),
        "shutdown_duration" => nullable_if(float_schema, mode == SchemaMode::Serialization),
    })
}

#[cfg(feature = "python")]
fn tuning_power_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    mode: SchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let variant_schema = literal_schema(core_schema, &[TuningConfigPower::VARIANT])?;
    let float_schema = core_schema.call_method0("float_schema")?;
    let str_schema = core_schema.call_method0("str_schema")?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.TuningConfigPower", {
        "variant" => required_if(variant_schema, mode == SchemaMode::Serialization),
        "target_watts" => required(float_schema),
        "algorithm" => nullable_if(str_schema, mode == SchemaMode::Serialization),
    })
}

#[cfg(feature = "python")]
fn tuning_hashrate_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    mode: SchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let variant_schema = literal_schema(core_schema, &[TuningConfigHashRate::VARIANT])?;
    let hashrate_schema = HashRate::pydantic_schema(core_schema, mode)?;
    let str_schema = core_schema.call_method0("str_schema")?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.TuningConfigHashRate", {
        "variant" => required_if(variant_schema, mode == SchemaMode::Serialization),
        "target_hashrate" => required(hashrate_schema),
        "algorithm" => nullable_if(str_schema, mode == SchemaMode::Serialization),
    })
}

#[cfg(feature = "python")]
fn tuning_mode_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    mode: SchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let variant_schema = literal_schema(core_schema, &[TuningConfigMode::VARIANT])?;
    let mode_schema = literal_schema(core_schema, &["Low", "Normal", "High"])?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.TuningConfigMode", {
        "variant" => required_if(variant_schema, mode == SchemaMode::Serialization),
        "target_mode" => required(mode_schema),
    })
}

#[cfg(feature = "python")]
fn auto_fan_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    mode: SchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let mode_schema = literal_schema(core_schema, &[AutoFanConfig::MODE])?;
    let float_schema = core_schema.call_method0("float_schema")?;
    let int_schema = core_schema.call_method0("int_schema")?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.AutoFanConfig", {
        "mode" => required_if(mode_schema, mode == SchemaMode::Serialization),
        "target_temp" => required(float_schema),
        "idle_speed" => nullable_if(int_schema, mode == SchemaMode::Serialization),
    })
}

#[cfg(feature = "python")]
fn manual_fan_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    mode: SchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let mode_schema = literal_schema(core_schema, &[ManualFanConfig::MODE])?;
    let int_schema = core_schema.call_method0("int_schema")?;
    asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.ManualFanConfig", {
        "mode" => required_if(mode_schema, mode == SchemaMode::Serialization),
        "fan_speed" => required(int_schema),
    })
}

#[cfg(feature = "python")]
fn parse_pool(value: &Bound<'_, PyAny>) -> PyResult<Pool> {
    if let Ok(pool) = value.extract::<Pool>() {
        return Ok(pool);
    }
    Ok(Pool {
        url: py_to_string(&get_required(value, "url")?)?,
        username: get_required(value, "username")?.extract()?,
        password: get_required(value, "password")?.extract()?,
    })
}

#[cfg(feature = "python")]
fn parse_pool_group(value: &Bound<'_, PyAny>) -> PyResult<PoolGroup> {
    if let Ok(pool_group) = value.extract::<PoolGroup>() {
        return Ok(pool_group);
    }
    let pools = parse_required_list(value, "pools", parse_pool)?;
    Ok(PoolGroup {
        name: get_required(value, "name")?.extract()?,
        quota: parse_optional(get_optional(value, "quota")?)?.unwrap_or(1),
        pools,
    })
}

#[cfg(feature = "python")]
fn parse_scaling_config(value: &Bound<'_, PyAny>) -> PyResult<ScalingConfig> {
    if let Ok(config) = value.extract::<ScalingConfig>() {
        return Ok(config);
    }
    Ok(ScalingConfig {
        step: get_required(value, "step")?.extract()?,
        minimum: get_required(value, "minimum")?.extract()?,
        shutdown: parse_optional(get_optional(value, "shutdown")?)?,
        shutdown_duration: parse_optional(get_optional(value, "shutdown_duration")?)?,
    })
}

#[cfg(feature = "python")]
fn parse_optional_string_like(value: Option<Bound<'_, PyAny>>) -> PyResult<Option<String>> {
    match value {
        Some(value) if value.is_none() => Ok(None),
        Some(value) => py_to_string(&value).map(Some),
        None => Ok(None),
    }
}

#[cfg(feature = "python")]
fn validate_optional_literal_field(
    value: &Bound<'_, PyAny>,
    key: &str,
    expected: &str,
) -> PyResult<()> {
    let Some(actual) = get_optional(value, key)? else {
        return Ok(());
    };
    let actual = py_to_string(&actual)?;
    if actual == expected {
        Ok(())
    } else {
        Err(PyValueError::new_err(format!(
            "Expected {key} to be {expected:?}, got {actual:?}"
        )))
    }
}

#[cfg(feature = "python")]
fn parse_mining_mode_string(value: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(mode) = value.extract::<MiningMode>() {
        return Ok(mode.to_string());
    }

    let mode = py_to_string(value)?;
    match mode.as_str() {
        "Low" | "Normal" | "High" => Ok(mode),
        mode => Err(PyValueError::new_err(format!(
            "Unknown mining mode: {mode}"
        ))),
    }
}

#[cfg(feature = "python")]
fn parse_tuning_power(value: &Bound<'_, PyAny>) -> PyResult<TuningConfigPower> {
    if let Ok(config) = value.extract::<TuningConfigPower>() {
        return Ok(config);
    }
    validate_optional_literal_field(value, "variant", TuningConfigPower::VARIANT)?;
    Ok(TuningConfigPower::from_parts(
        get_required(value, "target_watts")?.extract()?,
        parse_optional_string_like(get_optional(value, "algorithm")?)?,
    ))
}

#[cfg(feature = "python")]
fn parse_tuning_hashrate(value: &Bound<'_, PyAny>) -> PyResult<TuningConfigHashRate> {
    if let Ok(config) = value.extract::<TuningConfigHashRate>() {
        return Ok(config);
    }
    validate_optional_literal_field(value, "variant", TuningConfigHashRate::VARIANT)?;
    Ok(TuningConfigHashRate::from_parts(
        HashRate::from_pydantic(&get_required(value, "target_hashrate")?)?,
        parse_optional_string_like(get_optional(value, "algorithm")?)?,
    ))
}

#[cfg(feature = "python")]
fn parse_tuning_mode(value: &Bound<'_, PyAny>) -> PyResult<TuningConfigMode> {
    if let Ok(config) = value.extract::<TuningConfigMode>() {
        return Ok(config);
    }
    validate_optional_literal_field(value, "variant", TuningConfigMode::VARIANT)?;
    Ok(TuningConfigMode::from_parts(parse_mining_mode_string(
        &get_required(value, "target_mode")?,
    )?))
}

#[cfg(feature = "python")]
fn parse_auto_fan(value: &Bound<'_, PyAny>) -> PyResult<AutoFanConfig> {
    if let Ok(config) = value.extract::<AutoFanConfig>() {
        return Ok(config);
    }
    validate_optional_literal_field(value, "mode", AutoFanConfig::MODE)?;
    Ok(AutoFanConfig::from_parts(
        get_required(value, "target_temp")?.extract()?,
        parse_optional(get_optional(value, "idle_speed")?)?,
    ))
}

#[cfg(feature = "python")]
fn parse_manual_fan(value: &Bound<'_, PyAny>) -> PyResult<ManualFanConfig> {
    if let Ok(config) = value.extract::<ManualFanConfig>() {
        return Ok(config);
    }
    validate_optional_literal_field(value, "mode", ManualFanConfig::MODE)?;
    Ok(ManualFanConfig::from_parts(
        get_required(value, "fan_speed")?.extract()?,
    ))
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
        Ok(TuningConfigMode::from_parts(parse_mining_mode_string(
            mode,
        )?))
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
        Ok(Self::from_parts(parse_mining_mode_string(target_mode)?))
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

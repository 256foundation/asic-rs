#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
/// How long the fans keep running at the pause cooldown speed.
pub enum FanPauseRuntime {
    /// Fans run for a limited duration, then turn off.
    ///
    /// Firmware that reports a fixed limited duration only accepts that value.
    Limited(#[ts(type = "{ secs: number, nanos: number }")] Duration),
    /// Fans keep running at the fixed speed until the miner is resumed.
    Indefinite,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "mode", rename_all = "PascalCase")]
/// Fan behaviour while the miner is paused.
pub enum FanPause {
    /// Firmware-managed cooldown: fans start at the last mining speed and
    /// slowly ramp down to 0%.
    Auto,
    /// Fixed-speed cooldown.
    Manual {
        /// Cooldown fan speed as a ratio between 0.0 and 1.0.
        fan_speed: f64,
        /// How long the fans run at `fan_speed`.
        runtime: FanPauseRuntime,
    },
}

impl FanPause {
    /// Create a manual pause cooldown configuration.
    pub fn manual(fan_speed: f64, runtime: FanPauseRuntime) -> Self {
        Self::Manual { fan_speed, runtime }
    }

    /// Check that a manual cooldown fan speed is a ratio in `0.0..=1.0`.
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Self::Manual { fan_speed, .. } = self
            && !(0.0..=1.0).contains(fan_speed)
        {
            anyhow::bail!(
                "Pause cooldown fan speed must be a ratio between 0.0 and 1.0, got {fan_speed}"
            );
        }
        Ok(())
    }
}

#[cfg_attr(feature = "python", pyclass(skip_from_py_object, module = "asic_rs"))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
/// Fan control mode.
pub enum FanMode {
    /// Firmware-managed fan control using a target temperature.
    Auto,
    /// Fixed fan speed control.
    Manual,
}

#[cfg_attr(feature = "python", pyclass(skip_from_py_object, module = "asic_rs"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "mode", rename_all = "PascalCase")]
/// Desired fan control configuration.
///
/// Use [`Self::auto`] to target a temperature, or [`Self::manual`] to request a
/// fixed fan speed.
pub enum FanConfig {
    /// Automatic fan control with a target chip/board temperature.
    Auto {
        /// Temperature target in Celsius.
        target_temp: f64,
        /// Optional idle fan speed percentage.
        idle_speed: Option<u64>,
        /// Fan behaviour while the miner is paused, if it should be changed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pause: Option<FanPause>,
    },
    /// Manual fan control with a fixed fan speed percentage.
    Manual {
        /// Fan speed percentage.
        fan_speed: u64,
        /// Fan behaviour while the miner is paused, if it should be changed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pause: Option<FanPause>,
    },
}

impl FanConfig {
    /// Create an automatic fan configuration.
    pub fn auto(target_temp: f64, idle_speed: Option<u64>) -> Self {
        Self::Auto {
            target_temp,
            idle_speed,
            pause: None,
        }
    }

    /// Create a manual fan configuration.
    pub fn manual(fan_speed: u64) -> Self {
        Self::Manual {
            fan_speed,
            pause: None,
        }
    }

    /// Set the fan behaviour used while the miner is paused.
    pub fn with_pause(mut self, new_pause: FanPause) -> Self {
        match &mut self {
            Self::Auto { pause, .. } | Self::Manual { pause, .. } => *pause = Some(new_pause),
        }
        self
    }

    /// Return the pause fan behaviour, if one was requested.
    pub fn pause(&self) -> Option<FanPause> {
        match self {
            Self::Auto { pause, .. } | Self::Manual { pause, .. } => *pause,
        }
    }

    /// Return the selected fan mode.
    pub fn mode(&self) -> FanMode {
        match self {
            Self::Auto { .. } => FanMode::Auto,
            Self::Manual { .. } => FanMode::Manual,
        }
    }

    /// Return the automatic target temperature, if this is automatic mode.
    pub fn target_temp(&self) -> Option<f64> {
        match self {
            Self::Auto { target_temp, .. } => Some(*target_temp),
            Self::Manual { .. } => None,
        }
    }

    /// Return the automatic idle speed, if this is automatic mode.
    pub fn idle_speed(&self) -> Option<u64> {
        match self {
            Self::Auto { idle_speed, .. } => *idle_speed,
            Self::Manual { .. } => None,
        }
    }

    /// Return the manual fan speed, if this is manual mode.
    pub fn fan_speed(&self) -> Option<u64> {
        match self {
            Self::Auto { .. } => None,
            Self::Manual { fan_speed, .. } => Some(*fan_speed),
        }
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl FanConfig {
    #[staticmethod]
    #[pyo3(name = "auto")]
    #[pyo3(signature = (target_temp, idle_speed = None))]
    fn py_auto(target_temp: f64, idle_speed: Option<u64>) -> Self {
        Self::auto(target_temp, idle_speed)
    }

    #[staticmethod]
    #[pyo3(name = "manual")]
    fn py_manual(fan_speed: u64) -> Self {
        Self::manual(fan_speed)
    }

    #[pyo3(name = "with_pause")]
    fn py_with_pause(&self, pause: FanPause) -> Self {
        self.clone().with_pause(pause)
    }

    #[getter]
    #[pyo3(name = "mode")]
    fn py_mode(&self) -> &'static str {
        match self {
            Self::Auto { .. } => "auto",
            Self::Manual { .. } => "manual",
        }
    }

    #[getter]
    #[pyo3(name = "target_temp")]
    fn py_target_temp(&self) -> Option<f64> {
        self.target_temp()
    }

    #[getter]
    #[pyo3(name = "idle_speed")]
    fn py_idle_speed(&self) -> Option<u64> {
        self.idle_speed()
    }

    #[getter]
    #[pyo3(name = "fan_speed")]
    fn py_fan_speed(&self) -> Option<u64> {
        self.fan_speed()
    }

    #[getter]
    #[pyo3(name = "pause")]
    fn py_pause(&self) -> Option<FanPause> {
        self.pause()
    }

    fn __repr__(&self) -> String {
        match self {
            Self::Auto {
                target_temp,
                idle_speed,
                ..
            } => match idle_speed {
                Some(idle_speed) => {
                    format!("FanConfig.auto(target_temp={target_temp:?}, idle_speed={idle_speed})")
                }
                None => format!("FanConfig.auto(target_temp={target_temp:?}, idle_speed=None)"),
            },
            Self::Manual { fan_speed, .. } => format!("FanConfig.manual(fan_speed={fan_speed})"),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    #[classmethod]
    #[pyo3(signature = (_source_type: "object", _handler: "object") -> "object")]
    pub fn __get_pydantic_core_schema__(
        cls: &Bound<'_, pyo3::types::PyType>,
        _source_type: &Bound<'_, PyAny>,
        _handler: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let core_schema = cls.py().import("pydantic_core")?.getattr("core_schema")?;
        let validation_schema = <Self as asic_rs_pydantic::PyPydanticType>::pydantic_schema(
            &core_schema,
            asic_rs_pydantic::PydanticSchemaMode::Validation,
        )?;
        let serialization_schema = <Self as asic_rs_pydantic::PyPydanticType>::pydantic_schema(
            &core_schema,
            asic_rs_pydantic::PydanticSchemaMode::Serialization,
        )?;
        asic_rs_pydantic::model_core_schema(cls, &validation_schema, &serialization_schema)
    }

    #[classmethod]
    #[pyo3(signature = (obj: "object", **_kwargs: "object") -> "FanConfig")]
    pub fn model_validate(
        cls: &Bound<'_, pyo3::types::PyType>,
        obj: &Bound<'_, PyAny>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        use pyo3::IntoPyObject as _;
        asic_rs_pydantic::reject_model_kwargs(_kwargs, "model_validate")?;
        if obj.is_instance(cls)? {
            return Ok(obj.clone().unbind());
        }
        Ok(
            <Self as asic_rs_pydantic::PyPydanticType>::from_pydantic(obj)?
                .into_pyobject(obj.py())?
                .into_any()
                .unbind(),
        )
    }

    #[classmethod]
    #[pyo3(signature = (**kwargs: "object") -> "dict[str, object]")]
    pub fn model_json_schema(
        cls: &Bound<'_, pyo3::types::PyType>,
        kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        asic_rs_pydantic::model_json_schema(cls, kwargs)
    }

    #[pyo3(signature = (**_kwargs: "object") -> "dict[str, object]")]
    pub fn model_dump(
        &self,
        py: Python<'_>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        asic_rs_pydantic::reject_model_kwargs(_kwargs, "model_dump")?;
        <Self as asic_rs_pydantic::PyPydanticType>::to_pydantic_data(self, py)
    }

    #[classmethod]
    #[pyo3(signature = (value: "object") -> "FanConfig")]
    fn _pydantic_validate(
        cls: &Bound<'_, pyo3::types::PyType>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        Self::model_validate(cls, value, None)
    }

    #[staticmethod]
    #[pyo3(signature = (value: "FanConfig") -> "dict[str, object]")]
    fn _pydantic_serialize(value: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let model = value.extract::<Self>()?;
        <Self as asic_rs_pydantic::PyPydanticType>::to_pydantic_data(&model, value.py())
    }
}

#[cfg(feature = "python")]
mod python_impls {
    use asic_rs_pydantic::{
        PyPydanticType, PydanticSchemaMode, get_optional_field, get_required_field,
    };
    use std::time::Duration;

    use pyo3::{
        Borrowed, Bound, IntoPyObject, Py, PyAny, PyErr, PyRef, PyResult, Python,
        conversion::FromPyObject,
        types::{PyAnyMethods, PyDict, PyDictMethods},
    };

    use super::{FanConfig, FanPause, FanPauseRuntime};

    /// Read the optional `pause` field of a fan config object.
    fn extract_pause(obj: &Borrowed<'_, '_, PyAny>) -> PyResult<Option<FanPause>> {
        match get_optional_field(obj, "pause")? {
            Some(value) if !value.is_none() => Ok(Some(value.extract()?)),
            _ => Ok(None),
        }
    }

    fn value_error(message: String) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(message)
    }

    /// Python sees a pause as a dict: `{"mode": "auto"}` or
    /// `{"mode": "manual", "fan_speed": float, "runtime": {"mode": "limited",
    /// "duration_secs": float} | {"mode": "indefinite"}}`.
    impl FromPyObject<'_, '_> for FanPause {
        type Error = PyErr;

        fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
            let mode: String = get_required_field(&obj, "mode")?.extract()?;
            match mode.to_lowercase().as_str() {
                "auto" => Ok(FanPause::Auto),
                "manual" => {
                    let fan_speed: f64 = get_required_field(&obj, "fan_speed")?.extract()?;
                    let runtime = get_required_field(&obj, "runtime")?;
                    let runtime = runtime.as_borrowed();
                    let runtime_mode: String = get_required_field(&runtime, "mode")?.extract()?;
                    let runtime = match runtime_mode.to_lowercase().as_str() {
                        "indefinite" => FanPauseRuntime::Indefinite,
                        "limited" => {
                            let secs: f64 =
                                get_required_field(&runtime, "duration_secs")?.extract()?;
                            FanPauseRuntime::Limited(
                                Duration::try_from_secs_f64(secs)
                                    .map_err(|e| value_error(e.to_string()))?,
                            )
                        }
                        _ => {
                            return Err(value_error(format!(
                                "Unknown pause runtime '{runtime_mode}', expected 'limited' or 'indefinite'"
                            )));
                        }
                    };
                    Ok(FanPause::Manual { fan_speed, runtime })
                }
                _ => Err(value_error(format!(
                    "Unknown pause mode '{mode}', expected 'auto' or 'manual'"
                ))),
            }
        }
    }

    impl<'py> IntoPyObject<'py> for FanPause {
        type Target = PyDict;
        type Output = Bound<'py, PyDict>;
        type Error = PyErr;

        fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
            let dict = PyDict::new(py);
            match self {
                FanPause::Auto => dict.set_item("mode", "auto")?,
                FanPause::Manual { fan_speed, runtime } => {
                    dict.set_item("mode", "manual")?;
                    dict.set_item("fan_speed", fan_speed)?;
                    let runtime_dict = PyDict::new(py);
                    match runtime {
                        FanPauseRuntime::Limited(duration) => {
                            runtime_dict.set_item("mode", "limited")?;
                            runtime_dict.set_item("duration_secs", duration.as_secs_f64())?;
                        }
                        FanPauseRuntime::Indefinite => {
                            runtime_dict.set_item("mode", "indefinite")?
                        }
                    }
                    dict.set_item("runtime", runtime_dict)?;
                }
            }
            Ok(dict)
        }
    }

    impl FromPyObject<'_, '_> for FanConfig {
        type Error = PyErr;

        fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
            if let Ok(config) = obj.extract::<PyRef<'_, FanConfig>>() {
                return Ok(config.clone());
            }
            let mode_value = get_required_field(&obj, "mode")?;
            let mode = mode_value.extract::<String>()?;
            match mode.to_lowercase().as_str() {
                "auto" => {
                    let target_temp: f64 = get_required_field(&obj, "target_temp")?.extract()?;
                    let idle_speed: Option<u64> = get_optional_field(&obj, "idle_speed")?
                        .map(|value| value.extract())
                        .transpose()?
                        .flatten();
                    Ok(FanConfig::Auto {
                        target_temp,
                        idle_speed,
                        pause: extract_pause(&obj)?,
                    })
                }
                "manual" => {
                    let fan_speed: u64 = get_required_field(&obj, "fan_speed")?.extract()?;
                    Ok(FanConfig::Manual {
                        fan_speed,
                        pause: extract_pause(&obj)?,
                    })
                }
                _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Unknown fan mode '{mode}', expected 'auto' or 'manual'"
                ))),
            }
        }
    }

    impl PyPydanticType for FanConfig {
        fn pydantic_schema<'py>(
            core_schema: &pyo3::Bound<'py, PyAny>,
            _mode: PydanticSchemaMode,
        ) -> PyResult<pyo3::Bound<'py, PyAny>> {
            let auto_mode = asic_rs_pydantic::literal_schema(core_schema, &["auto"])?;
            let target_temp = core_schema.call_method0("float_schema")?;
            let idle_speed = core_schema.call_method0("int_schema")?;
            let pause = core_schema.call_method0("dict_schema")?;
            let auto_schema = asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.FanConfigAuto", {
                "mode" => required(auto_mode),
                "target_temp" => required(target_temp),
                "idle_speed" => nullable_if(idle_speed, false),
                "pause" => nullable_if(pause, false),
            })?;

            let manual_mode = asic_rs_pydantic::literal_schema(core_schema, &["manual"])?;
            let fan_speed = core_schema.call_method0("int_schema")?;
            let pause = core_schema.call_method0("dict_schema")?;
            let manual_schema = asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.FanConfigManual", {
                "mode" => required(manual_mode),
                "fan_speed" => required(fan_speed),
                "pause" => nullable_if(pause, false),
            })?;

            asic_rs_pydantic::tagged_union_schema(
                core_schema,
                [("auto", auto_schema), ("manual", manual_schema)],
                "mode",
                Some("asic_rs.FanConfig"),
            )
        }

        fn from_pydantic(value: &pyo3::Bound<'_, PyAny>) -> PyResult<Self> {
            value.extract()
        }

        fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let dict = PyDict::new(py);
            match self {
                FanConfig::Auto {
                    target_temp,
                    idle_speed,
                    pause,
                } => {
                    dict.set_item("mode", "auto")?;
                    dict.set_item("target_temp", target_temp)?;
                    dict.set_item("idle_speed", idle_speed)?;
                    if let Some(pause) = pause {
                        dict.set_item("pause", pause.into_pyobject(py)?)?;
                    }
                }
                FanConfig::Manual { fan_speed, pause } => {
                    dict.set_item("mode", "manual")?;
                    dict.set_item("fan_speed", fan_speed)?;
                    if let Some(pause) = pause {
                        dict.set_item("pause", pause.into_pyobject(py)?)?;
                    }
                }
            }
            Ok(dict.into_any().unbind())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{FanConfig, FanMode, FanPause, FanPauseRuntime};

    #[test]
    fn auto_mode_has_required_fields() {
        let config = FanConfig::auto(60.0, Some(35));

        assert_eq!(config.mode(), FanMode::Auto);
        assert_eq!(config.target_temp(), Some(60.0));
        assert_eq!(config.idle_speed(), Some(35));
        assert_eq!(config.fan_speed(), None);
    }

    #[test]
    fn auto_mode_allows_none_idle_speed() {
        let config = FanConfig::auto(60.0, None);

        assert_eq!(config.mode(), FanMode::Auto);
        assert_eq!(config.target_temp(), Some(60.0));
        assert_eq!(config.idle_speed(), None);
        assert_eq!(config.fan_speed(), None);
    }

    #[test]
    fn manual_mode_has_fan_speed_and_no_auto_fields() {
        let config = FanConfig::manual(75);

        assert_eq!(config.mode(), FanMode::Manual);
        assert_eq!(config.target_temp(), None);
        assert_eq!(config.idle_speed(), None);
        assert_eq!(config.fan_speed(), Some(75));
    }

    #[test]
    fn constructors_leave_pause_unset() {
        assert_eq!(FanConfig::auto(60.0, None).pause(), None);
        assert_eq!(FanConfig::manual(75).pause(), None);
    }

    #[test]
    fn legacy_json_without_pause_deserializes() {
        let auto: FanConfig =
            serde_json::from_str(r#"{"mode":"Auto","target_temp":60.0,"idle_speed":35}"#).unwrap();
        assert_eq!(auto, FanConfig::auto(60.0, Some(35)));

        let manual: FanConfig =
            serde_json::from_str(r#"{"mode":"Manual","fan_speed":75}"#).unwrap();
        assert_eq!(manual, FanConfig::manual(75));
    }

    #[test]
    fn config_without_pause_serializes_unchanged() {
        let json = serde_json::to_value(FanConfig::manual(75)).unwrap();
        assert_eq!(json, serde_json::json!({"mode":"Manual","fan_speed":75}));
    }

    #[test]
    fn pause_round_trips_through_json() {
        let pause = FanPause::manual(0.4, FanPauseRuntime::Limited(Duration::from_secs(120)));
        let config = FanConfig::manual(75).with_pause(pause);
        let json = serde_json::to_string(&config).unwrap();
        let back: FanConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, config);
        assert_eq!(back.pause(), Some(pause));

        let auto = FanConfig::auto(60.0, None).with_pause(FanPause::Auto);
        let back: FanConfig = serde_json::from_str(&serde_json::to_string(&auto).unwrap()).unwrap();
        assert_eq!(back, auto);

        let indefinite = FanPause::manual(1.0, FanPauseRuntime::Indefinite);
        let json = serde_json::to_string(&indefinite).unwrap();
        assert_eq!(serde_json::from_str::<FanPause>(&json).unwrap(), indefinite);
    }

    #[test]
    fn pause_validate_rejects_out_of_range_ratio() {
        for bad in [-0.1, 1.01, f64::NAN, f64::INFINITY] {
            assert!(
                FanPause::manual(bad, FanPauseRuntime::Indefinite)
                    .validate()
                    .is_err()
            );
        }
        for good in [0.0, 0.5, 1.0] {
            assert!(
                FanPause::manual(good, FanPauseRuntime::Indefinite)
                    .validate()
                    .is_ok()
            );
        }
        assert!(FanPause::Auto.validate().is_ok());
    }
}

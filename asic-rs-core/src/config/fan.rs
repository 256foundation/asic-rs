#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "python", pyclass(skip_from_py_object, module = "asic_rs"))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FanMode {
    Auto,
    Manual,
}

#[cfg_attr(feature = "python", pyclass(skip_from_py_object, module = "asic_rs"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "PascalCase")]
pub enum FanConfig {
    Auto {
        target_temp: f64,
        idle_speed: Option<u64>,
    },
    Manual {
        fan_speed: u64,
    },
}

impl FanConfig {
    pub fn auto(target_temp: f64, idle_speed: Option<u64>) -> Self {
        Self::Auto {
            target_temp,
            idle_speed,
        }
    }

    pub fn manual(fan_speed: u64) -> Self {
        Self::Manual { fan_speed }
    }

    pub fn mode(&self) -> FanMode {
        match self {
            Self::Auto { .. } => FanMode::Auto,
            Self::Manual { .. } => FanMode::Manual,
        }
    }

    pub fn target_temp(&self) -> Option<f64> {
        match self {
            Self::Auto { target_temp, .. } => Some(*target_temp),
            Self::Manual { .. } => None,
        }
    }

    pub fn idle_speed(&self) -> Option<u64> {
        match self {
            Self::Auto { idle_speed, .. } => *idle_speed,
            Self::Manual { .. } => None,
        }
    }

    pub fn fan_speed(&self) -> Option<u64> {
        match self {
            Self::Auto { .. } => None,
            Self::Manual { fan_speed } => Some(*fan_speed),
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
    use pyo3::{
        Borrowed, Py, PyAny, PyErr, PyRef, PyResult, Python,
        conversion::FromPyObject,
        types::{PyAnyMethods, PyDict, PyDictMethods},
    };

    use super::FanConfig;

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
                    })
                }
                "manual" => {
                    let fan_speed: u64 = get_required_field(&obj, "fan_speed")?.extract()?;
                    Ok(FanConfig::Manual { fan_speed })
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
            let auto_schema = asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.FanConfigAuto", {
                "mode" => required(auto_mode),
                "target_temp" => required(target_temp),
                "idle_speed" => nullable_if(idle_speed, false),
            })?;

            let manual_mode = asic_rs_pydantic::literal_schema(core_schema, &["manual"])?;
            let fan_speed = core_schema.call_method0("int_schema")?;
            let manual_schema = asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.FanConfigManual", {
                "mode" => required(manual_mode),
                "fan_speed" => required(fan_speed),
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
                } => {
                    dict.set_item("mode", "auto")?;
                    dict.set_item("target_temp", target_temp)?;
                    dict.set_item("idle_speed", idle_speed)?;
                }
                FanConfig::Manual { fan_speed } => {
                    dict.set_item("mode", "manual")?;
                    dict.set_item("fan_speed", fan_speed)?;
                }
            }
            Ok(dict.into_any().unbind())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FanConfig, FanMode};

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
}

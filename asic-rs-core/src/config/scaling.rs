#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(
    feature = "python",
    pyclass(skip_from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingConfig {
    pub step: u32,
    pub minimum: u32,
    pub shutdown: Option<bool>,
    pub shutdown_duration: Option<f32>,
}

impl ScalingConfig {
    pub fn new(step: u32, minimum: u32) -> Self {
        Self {
            step,
            minimum,
            shutdown: None,
            shutdown_duration: None,
        }
    }

    pub fn with_shutdown(mut self, shutdown: bool) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    pub fn with_shutdown_duration(mut self, shutdown_duration: f32) -> Self {
        self.shutdown_duration = Some(shutdown_duration);
        self
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl ScalingConfig {
    #[new]
    #[pyo3(signature = (step, minimum, shutdown = None, shutdown_duration = None))]
    fn py_new(
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

#[cfg(feature = "python")]
mod python_impls {
    use asic_rs_pydantic::{get_optional_field, get_required_field};
    use pyo3::{Borrowed, PyAny, PyErr, PyResult, conversion::FromPyObject, types::PyAnyMethods};

    use super::ScalingConfig;

    impl FromPyObject<'_, '_> for ScalingConfig {
        type Error = PyErr;

        fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
            Ok(ScalingConfig {
                step: get_required_field(&obj, "step")?.extract()?,
                minimum: get_required_field(&obj, "minimum")?.extract()?,
                shutdown: get_optional_field(&obj, "shutdown")?
                    .map(|value| value.extract())
                    .transpose()?
                    .flatten(),
                shutdown_duration: get_optional_field(&obj, "shutdown_duration")?
                    .map(|value| value.extract())
                    .transpose()?
                    .flatten(),
            })
        }
    }
}

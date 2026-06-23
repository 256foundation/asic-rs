#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Power or performance scaling configuration.
pub struct ScalingConfig {
    /// Scaling step size used by the firmware.
    pub step: u32,
    /// Minimum allowed scaling value.
    pub minimum: u32,
    /// Whether the miner may shut down when scaling cannot protect it.
    pub shutdown: Option<bool>,
    /// Optional shutdown delay or duration in firmware-defined seconds.
    pub shutdown_duration: Option<f32>,
}

impl ScalingConfig {
    /// Create a scaling configuration without shutdown options.
    pub fn new(step: u32, minimum: u32) -> Self {
        Self {
            step,
            minimum,
            shutdown: None,
            shutdown_duration: None,
        }
    }

    /// Set whether shutdown is allowed when scaling cannot protect the miner.
    pub fn with_shutdown(mut self, shutdown: bool) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    /// Set the shutdown delay or duration in firmware-defined seconds.
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

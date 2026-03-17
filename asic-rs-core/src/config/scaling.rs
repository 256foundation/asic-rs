#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(
    feature = "python",
    pyclass(skip_from_py_object, get_all, module = "asic_rs")
)]
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

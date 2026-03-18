#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::miner::TuningTarget;

#[cfg_attr(feature = "python", pyclass(skip_from_py_object, module = "asic_rs"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningConfig {
    pub target: TuningTarget,
    pub algorithm: Option<String>,
}

impl TuningConfig {
    pub fn new(target: TuningTarget) -> Self {
        Self {
            target,
            algorithm: None,
        }
    }

    pub fn with_algorithm(mut self, algorithm: impl Into<String>) -> Self {
        self.algorithm = Some(algorithm.into());
        self
    }
}

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

    pub fn variant(&self) -> &'static str {
        match &self.target {
            TuningTarget::Power(_) => "power",
            TuningTarget::HashRate(_) => "hashrate",
            TuningTarget::MiningMode(_) => "mode",
        }
    }

    /// Target power in watts, or `None` if targeting hashrate or mining mode.
    pub fn target_watts(&self) -> Option<f64> {
        match &self.target {
            TuningTarget::Power(p) => Some(p.as_watts()),
            _ => None,
        }
    }

    /// Target hashrate, or `None` if targeting power or mining mode.
    pub fn target_hashrate(&self) -> Option<&crate::data::hashrate::HashRate> {
        match &self.target {
            TuningTarget::HashRate(hr) => Some(hr),
            _ => None,
        }
    }

    /// Target mining mode, or `None` if targeting power or hashrate.
    pub fn target_mode(&self) -> Option<crate::data::miner::MiningMode> {
        match &self.target {
            TuningTarget::MiningMode(m) => Some(*m),
            _ => None,
        }
    }

    pub fn algorithm(&self) -> Option<&str> {
        self.algorithm.as_deref()
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl TuningConfig {
    #[getter]
    #[pyo3(name = "variant")]
    fn py_variant(&self) -> &'static str {
        self.variant()
    }

    /// Target power in watts, or `None` if targeting hashrate or mining mode.
    #[getter]
    #[pyo3(name = "target_watts")]
    fn py_target_watts(&self) -> Option<f64> {
        self.target_watts()
    }

    /// Target hashrate, or `None` if targeting power or mining mode.
    #[getter]
    #[pyo3(name = "target_hashrate")]
    fn py_target_hashrate(&self) -> Option<crate::data::hashrate::HashRate> {
        self.target_hashrate().cloned()
    }

    /// Target mining mode, or `None` if targeting power or hashrate.
    #[getter]
    #[pyo3(name = "target_mode")]
    fn py_target_mode(&self) -> Option<crate::data::miner::MiningMode> {
        self.target_mode()
    }

    #[getter]
    #[pyo3(name = "algorithm")]
    fn py_algorithm(&self) -> Option<&str> {
        self.algorithm()
    }
}

#[cfg(feature = "python")]
mod python_impls {
    use measurements::Power;
    use pyo3::{Borrowed, PyAny, PyErr, PyResult, conversion::FromPyObject, types::PyAnyMethods};

    use super::TuningConfig;
    use crate::data::{
        hashrate::HashRate,
        miner::{MiningMode, TuningTarget},
    };

    impl FromPyObject<'_, '_> for TuningConfig {
        type Error = PyErr;

        fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
            let variant: String = obj.getattr("variant")?.extract()?;
            let algorithm: Option<String> =
                obj.getattr("algorithm").ok().and_then(|v| v.extract().ok());

            let target = match variant.as_str() {
                "power" => {
                    let watts: f64 = obj.getattr("target_watts")?.extract()?;
                    TuningTarget::Power(Power::from_watts(watts))
                }
                "hashrate" => {
                    let hr: HashRate = obj.getattr("target_hashrate")?.extract()?;
                    TuningTarget::HashRate(hr)
                }
                "mode" => {
                    let mode_val = obj.getattr("target_mode")?;
                    let mode = mode_val.extract::<MiningMode>().or_else(|_| {
                        mode_val
                            .extract::<String>()
                            .and_then(|s| match s.to_lowercase().as_str() {
                                "low" => Ok(MiningMode::Low),
                                "normal" => Ok(MiningMode::Normal),
                                "high" => Ok(MiningMode::High),
                                _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                                    "Unknown mining mode '{s}', expected 'Low', 'Normal', or 'High'"
                                ))),
                            })
                    })?;
                    TuningTarget::MiningMode(mode)
                }
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Unknown TuningConfig variant '{variant}', expected 'power', 'hashrate', or 'mode'",
                    )));
                }
            };

            Ok(TuningConfig { target, algorithm })
        }
    }
}

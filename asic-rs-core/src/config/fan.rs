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
    #[getter]
    #[pyo3(name = "mode")]
    fn py_mode(&self) -> FanMode {
        self.mode()
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
}

#[cfg(feature = "python")]
mod python_impls {
    use pyo3::{Borrowed, PyAny, PyErr, PyResult, conversion::FromPyObject, types::PyAnyMethods};

    use super::FanConfig;

    impl FromPyObject<'_, '_> for FanConfig {
        type Error = PyErr;

        fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
            let mode: String = obj.getattr("mode")?.extract()?;
            match mode.to_lowercase().as_str() {
                "auto" => {
                    let target_temp: f64 = obj.getattr("target_temp")?.extract()?;
                    let idle_speed: Option<u64> = obj
                        .getattr("idle_speed")
                        .and_then(|v| v.extract())
                        .unwrap_or(None);
                    Ok(FanConfig::Auto {
                        target_temp,
                        idle_speed,
                    })
                }
                "manual" => {
                    let fan_speed: u64 = obj.getattr("fan_speed")?.extract()?;
                    Ok(FanConfig::Manual { fan_speed })
                }
                _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Unknown fan mode '{mode}', expected 'auto' or 'manual'"
                ))),
            }
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

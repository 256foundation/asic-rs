#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "python", pyclass(skip_from_py_object, module = "asic_rs"))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FanMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "PascalCase")]
pub enum FanConfig {
    Auto {
        target_temp: u64,
        idle_speed: Option<u64>,
    },
    Manual {
        fan_speed: u64,
    },
}

impl FanConfig {
    pub fn auto(target_temp: u64, idle_speed: Option<u64>) -> Self {
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

    pub fn target_temp(&self) -> Option<u64> {
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

#[cfg(test)]
mod tests {
    use super::{FanConfig, FanMode};

    #[test]
    fn auto_mode_has_required_fields() {
        let config = FanConfig::auto(60, Some(35));

        assert_eq!(config.mode(), FanMode::Auto);
        assert_eq!(config.target_temp(), Some(60));
        assert_eq!(config.idle_speed(), Some(35));
        assert_eq!(config.fan_speed(), None);
    }

    #[test]
    fn auto_mode_allows_none_idle_speed() {
        let config = FanConfig::auto(60, None);

        assert_eq!(config.mode(), FanMode::Auto);
        assert_eq!(config.target_temp(), Some(60));
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

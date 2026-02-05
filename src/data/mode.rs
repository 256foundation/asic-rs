use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum MiningMode {
    #[default]
    Enabled,
    Disabled,
}

impl From<bool> for MiningMode {
    fn from(value: bool) -> Self {
        if value {
            MiningMode::Enabled
        } else {
            MiningMode::Disabled
        }
    }
}

impl From<MiningMode> for bool {
    fn from(value: MiningMode) -> Self {
        matches!(value, MiningMode::Enabled)
    }
}

impl std::fmt::Display for MiningMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MiningMode::Enabled => write!(f, "enabled"),
            MiningMode::Disabled => write!(f, "disabled"),
        }
    }
}

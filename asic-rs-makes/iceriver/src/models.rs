use std::str::FromStr;

use asic_rs_core::{
    data::device::HashAlgorithm, errors::ModelSelectionError, traits::model::MinerModel,
};
use asic_rs_macros::ModelAlgorithm;
use serde::{Deserialize, Serialize};
use strum::Display;
use ts_rs::TS;

#[derive(
    Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display, ModelAlgorithm, TS,
)]
pub enum IceRiverModel {
    #[algorithm(HashAlgorithm::Blake3)]
    #[serde(alias = "ICERIVER AL3", alias = "10306")]
    AL3,
    #[strum(to_string = "{0}")]
    #[algorithm(HashAlgorithm::Unknown)]
    Unknown(String),
}

impl FromStr for IceRiverModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_uppercase();
        let normalized = normalized.strip_prefix("ICERIVER ").unwrap_or(&normalized);
        serde_json::from_value(serde_json::Value::String(normalized.to_owned()))
            .or_else(|_| Ok(Self::Unknown(s.to_owned())))
    }
}

impl MinerModel for IceRiverModel {
    fn make_name(&self) -> String {
        "IceRiver".to_owned()
    }

    fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

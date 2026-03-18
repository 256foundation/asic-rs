use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum SealMinerModel {
    A2,
    #[strum(to_string = "{0}")]
    Unknown(String),
}

impl FromStr for SealMinerModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl asic_rs_core::traits::model::MinerModel for SealMinerModel {
    fn make_name(&self) -> String {
        "Sealminer".to_string()
    }
}

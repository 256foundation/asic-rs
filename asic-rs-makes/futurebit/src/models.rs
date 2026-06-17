use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
use asic_rs_core::traits::model::MinerModel;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum FutureBitModel {
    #[serde(
        alias = "Apollo",
        alias = "Apollo BTC",
        alias = "Apollo-BTC",
        alias = "Apollo I",
        alias = "Apollo 1"
    )]
    Apollo1,
    #[serde(
        alias = "Apollo II",
        alias = "Apollo 2",
        alias = "Apollo-2",
        alias = "Apollo-BTC II",
        alias = "Apollo BTC II"
    )]
    Apollo2,
    #[strum(to_string = "{0}")]
    Unknown(String),
}

impl FromStr for FutureBitModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl MinerModel for FutureBitModel {
    fn make_name(&self) -> String {
        "FutureBit".to_string()
    }
    fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

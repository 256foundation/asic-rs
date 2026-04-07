use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum BraiinsModel {
    #[serde(alias = "BRAIINS MINI MINER BMM 100")]
    BMM100,
    #[serde(alias = "BRAIINS MINI MINER BMM 101")]
    BMM101,
    #[strum(to_string = "{0}")]
    Unknown(String),
}

impl FromStr for BraiinsModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl asic_rs_core::traits::model::MinerModel for BraiinsModel {
    fn make_name(&self) -> String {
        "Braiins".to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn known_model_parses() {
        // Act
        let result = BraiinsModel::from_str("BRAIINS MINI MINER BMM 100").unwrap();

        // Assert
        assert_eq!(result, BraiinsModel::BMM100);
    }

    #[test]
    fn unknown_model_falls_back() {
        // Act
        let result = BraiinsModel::from_str("BRAIINS MINI MINER BMM 999").unwrap();

        // Assert
        assert_eq!(
            result,
            BraiinsModel::Unknown("BRAIINS MINI MINER BMM 999".to_string())
        );
    }
}

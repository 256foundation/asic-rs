use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum EPicModel {
    #[serde(alias = "BLOCKMINER 520i")]
    BM520i,
    #[serde(alias = "ANTMINER S19J PRO DUAL")]
    S19JProDual,
    #[strum(to_string = "{0}")]
    Unknown(String),
}

impl FromStr for EPicModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl asic_rs_core::traits::model::MinerModel for EPicModel {
    fn make_name(&self) -> String {
        "ePIC".to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn known_model_parses() {
        // Act
        let result = EPicModel::from_str("BLOCKMINER 520i").unwrap();

        // Assert
        assert_eq!(result, EPicModel::BM520i);
    }

    #[test]
    fn unknown_model_falls_back() {
        // Act
        let result = EPicModel::from_str("BLOCKMINER 999").unwrap();

        // Assert
        assert_eq!(result, EPicModel::Unknown("BLOCKMINER 999".to_string()));
    }
}

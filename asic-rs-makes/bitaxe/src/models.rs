use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum BitaxeModel {
    #[serde(alias = "BM1368")]
    Supra,
    #[serde(alias = "BM1370")]
    Gamma,
    #[serde(alias = "BM1397")]
    Max,
    #[serde(alias = "BM1366")]
    Ultra,
    #[strum(to_string = "{0}")]
    Unknown(String),
}

impl FromStr for BitaxeModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl asic_rs_core::traits::model::MinerModel for BitaxeModel {
    fn make_name(&self) -> String {
        "Bitaxe".to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn known_model_parses() {
        // Act
        let result = BitaxeModel::from_str("BM1370").unwrap();

        // Assert
        assert_eq!(result, BitaxeModel::Gamma);
    }

    #[test]
    fn unknown_model_falls_back() {
        // Act
        let result = BitaxeModel::from_str("BM9999").unwrap();

        // Assert
        assert_eq!(result, BitaxeModel::Unknown("BM9999".to_string()));
    }
}

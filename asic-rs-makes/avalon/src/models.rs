use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum AvalonMinerModel {
    #[serde(alias = "721")]
    Avalon721,
    #[serde(alias = "741")]
    Avalon741,
    #[serde(alias = "761")]
    Avalon761,
    #[serde(alias = "821")]
    Avalon821,
    #[serde(alias = "841")]
    Avalon841,
    #[serde(alias = "851")]
    Avalon851,
    #[serde(alias = "921")]
    Avalon921,
    #[serde(alias = "1026")]
    Avalon1026,
    #[serde(alias = "1047")]
    Avalon1047,
    #[serde(alias = "1066")]
    Avalon1066,
    #[serde(alias = "1166PRO")]
    Avalon1166Pro,
    #[serde(alias = "1126PRO")]
    Avalon1126Pro,
    #[serde(alias = "1246")]
    Avalon1246,
    #[serde(alias = "1466")]
    Avalon1466,
    #[serde(alias = "1566")]
    Avalon1566,
    #[serde(alias = "NANO3")]
    AvalonNano3,
    #[serde(alias = "NANO3S")]
    AvalonNano3s,
    #[serde(alias = "Q")]
    AvalonHomeQ,
    #[strum(to_string = "{0}")]
    Unknown(String),
}

impl FromStr for AvalonMinerModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl asic_rs_core::traits::model::MinerModel for AvalonMinerModel {
    fn make_name(&self) -> String {
        "Avalonminer".to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn known_model_parses() {
        // Act
        let result = AvalonMinerModel::from_str("1466").unwrap();

        // Assert
        assert_eq!(result, AvalonMinerModel::Avalon1466);
    }

    #[test]
    fn unknown_model_falls_back() {
        // Act
        let result = AvalonMinerModel::from_str("9999").unwrap();

        // Assert
        assert_eq!(result, AvalonMinerModel::Unknown("9999".to_string()));
    }
}

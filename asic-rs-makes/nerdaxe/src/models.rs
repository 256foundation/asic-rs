use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum NerdAxeModel {
    #[serde(alias = "BM1368")]
    NerdAxe,
    #[serde(alias = "BM1370", alias = "nerdqaxe++", alias = "NerdQAxe++")]
    NerdQAxe,
    #[serde(alias = "BM1397")]
    NerdMiner,
    #[serde(alias = "BM1366")]
    NerdAxeUltra,
    #[strum(to_string = "{0}")]
    Unknown(String),
}

impl FromStr for NerdAxeModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl asic_rs_core::traits::model::MinerModel for NerdAxeModel {
    fn make_name(&self) -> String {
        "Nerdaxe".to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn parsing() {
        #[track_caller]
        fn case(s: &str, expected: NerdAxeModel) {
            assert_eq!(NerdAxeModel::from_str(s).unwrap(), expected);
        }

        case("NerdAxe", NerdAxeModel::NerdAxe);
        case("NerdQAxe", NerdAxeModel::NerdQAxe);
        case("NerdMiner", NerdAxeModel::NerdMiner);
        case("NerdAxeUltra", NerdAxeModel::NerdAxeUltra);
    }

    #[test]
    fn unknown_model_falls_back() {
        // Act
        let result = NerdAxeModel::from_str("NerdAxeXXX").unwrap();

        // Assert
        assert_eq!(result, NerdAxeModel::Unknown("NerdAxeXXX".to_string()));
    }
}

use std::str::FromStr;

use asic_rs_core::{errors::ModelSelectionError, traits::model::MinerModel};
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum VolcMinerModel {
    #[serde(alias = "VOLCMINER D1")]
    D1,
    #[strum(to_string = "{0}")]
    Unknown(String),
}

impl FromStr for VolcMinerModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl MinerModel for VolcMinerModel {
    fn make_name(&self) -> String {
        "VolcMiner".to_string()
    }

    fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_model_parses() {
        let result = VolcMinerModel::from_str("VOLCMINER D1").unwrap();

        assert_eq!(result, VolcMinerModel::D1);
    }

    #[test]
    fn unknown_model_falls_back() {
        let result = VolcMinerModel::from_str("VOLCMINER DX").unwrap();

        assert_eq!(result, VolcMinerModel::Unknown("VOLCMINER DX".to_string()));
    }
}

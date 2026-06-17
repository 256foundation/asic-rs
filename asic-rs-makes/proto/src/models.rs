use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
use asic_rs_core::traits::model::MinerModel;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum ProtoModel {
    #[serde(alias = "RIG")]
    Rig,
    #[strum(to_string = "{0}")]
    Unknown(String),
}

impl FromStr for ProtoModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_uppercase();
        serde_json::from_value(serde_json::Value::String(normalized.clone()))
            .or(Ok(Self::Unknown(normalized)))
    }
}

impl MinerModel for ProtoModel {
    fn make_name(&self) -> String {
        "Proto".to_string()
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
        let model = ProtoModel::from_str("rig").expect("model parses");
        assert_eq!(model, ProtoModel::Rig);
    }

    #[test]
    fn unknown_model_falls_back() {
        let model = ProtoModel::from_str("RIG-X").expect("model parses");
        assert_eq!(model, ProtoModel::Unknown("RIG-X".to_string()));
    }
}

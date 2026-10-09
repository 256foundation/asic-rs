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
pub enum GoldshellModel {
    #[serde(alias = "SC5PRO")]
    #[algorithm(HashAlgorithm::Blake2b)]
    SC5Pro,
    #[strum(to_string = "{0}")]
    #[algorithm(HashAlgorithm::Unknown)]
    Unknown(String),
}

impl FromStr for GoldshellModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let raw = s.trim();
        if raw.is_empty() {
            return Err(ModelSelectionError::UnexpectedModelResponse);
        }
        let normalized = raw.to_ascii_uppercase().replace([' ', '-', '_'], "");
        let name = normalized.strip_prefix("GOLDSHELL").unwrap_or(&normalized);
        serde_json::from_value(serde_json::Value::String(name.to_string()))
            .or_else(|_| Ok(Self::Unknown(raw.to_string())))
    }
}

impl MinerModel for GoldshellModel {
    fn make_name(&self) -> String {
        "Goldshell".to_string()
    }
    fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asic_rs_core::traits::model::MinerModelAlgorithm;

    #[test]
    fn known_model_parses_and_unknown_models_keep_their_identity() {
        for name in ["SC5Pro", "SC5 PRO", "Goldshell-SC5Pro"] {
            let model = GoldshellModel::from_str(name).unwrap();
            assert_eq!(model, GoldshellModel::SC5Pro);
            assert_eq!(model.hash_algorithm(), HashAlgorithm::Blake2b);
        }
        for name in ["SC5", "SC5ProX", "Goldshell-ARI31", "Goldshell"] {
            let model = GoldshellModel::from_str(name).unwrap();
            assert_eq!(model, GoldshellModel::Unknown(name.to_string()));
            assert_eq!(model.hash_algorithm(), HashAlgorithm::Unknown);
        }
    }
}

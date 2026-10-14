use std::str::FromStr;

use asic_rs_core::data::device::HashAlgorithm;
use asic_rs_core::errors::ModelSelectionError;
use asic_rs_core::traits::model::MinerModel;
use asic_rs_macros::ModelAlgorithm;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display};
use ts_rs::TS;

#[derive(
    Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display, AsRefStr, ModelAlgorithm, TS,
)]
pub enum SealMinerModel {
    #[algorithm(HashAlgorithm::SHA256)]
    A2,
    #[serde(alias = "A3 Pro Hydro")]
    #[algorithm(HashAlgorithm::SHA256)]
    A3ProHyd,
    #[strum(to_string = "{0}")]
    #[algorithm(HashAlgorithm::Unknown)]
    Unknown(String),
}

impl FromStr for SealMinerModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl MinerModel for SealMinerModel {
    fn make_name(&self) -> String {
        "Sealminer".to_string()
    }
    fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

#[cfg(test)]
mod tests {
    use asic_rs_core::traits::model::MinerModelAlgorithm;

    use super::*;

    #[test]
    fn hydro_model_parses_device_and_dashboard_names() {
        for name in ["A3ProHyd", "A3 Pro Hydro"] {
            let model = SealMinerModel::from_str(name).expect("model parses");
            assert_eq!(model, SealMinerModel::A3ProHyd);
            assert!(model.is_known());
            assert_eq!(model.hash_algorithm(), HashAlgorithm::SHA256);
        }
    }

    #[test]
    fn a2_model_still_parses() {
        assert_eq!(SealMinerModel::from_str("A2").unwrap(), SealMinerModel::A2);
    }

    #[test]
    fn unknown_model_preserves_input() {
        let name = "A3 Pro Hydro Unknown";
        let model = SealMinerModel::from_str(name).expect("model parses");
        assert_eq!(model, SealMinerModel::Unknown(name.to_string()));
        assert_eq!(model.to_string(), name);
        assert!(!model.is_known());
        assert_eq!(model.hash_algorithm(), HashAlgorithm::Unknown);
    }
}

use std::str::FromStr;

use asic_rs_core::data::device::HashAlgorithm;
use asic_rs_core::errors::ModelSelectionError;
use asic_rs_core::traits::model::MinerModel;
use asic_rs_macros::ModelAlgorithm;
use serde::{Deserialize, Serialize};
use strum::Display;
use ts_rs::TS;

#[derive(
    Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display, ModelAlgorithm, TS,
)]
pub enum ForgeaxeModel {
    // config-Nano.cvs uses BITFORGE_NANO; system/info identifies its ASIC as BM1370.
    #[serde(alias = "BITFORGE_NANO", alias = "BM1370")]
    #[algorithm(HashAlgorithm::SHA256)]
    BitForgeNano,
    #[strum(to_string = "{0}")]
    #[algorithm(HashAlgorithm::Unknown)]
    Unknown(String),
}

impl FromStr for ForgeaxeModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl MinerModel for ForgeaxeModel {
    fn make_name(&self) -> String {
        "Forgeaxe".to_string()
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
    fn nano_names_and_asic_model_parse() -> Result<(), ModelSelectionError> {
        for name in ["BitForgeNano", "BITFORGE_NANO", "BM1370"] {
            let model = ForgeaxeModel::from_str(name)?;
            assert_eq!(model, ForgeaxeModel::BitForgeNano);
            assert!(model.is_known());
            assert_eq!(model.make_name(), "Forgeaxe");
            assert_eq!(model.hash_algorithm(), HashAlgorithm::SHA256);
        }
        Ok(())
    }

    #[test]
    fn unknown_model_falls_back() -> Result<(), ModelSelectionError> {
        let model = ForgeaxeModel::from_str("BM9999")?;
        assert_eq!(model, ForgeaxeModel::Unknown("BM9999".to_string()));
        assert!(!model.is_known());
        assert_eq!(model.hash_algorithm(), HashAlgorithm::Unknown);
        Ok(())
    }
}

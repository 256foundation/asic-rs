//! Supported Goldshell model identities.
//! Firmware version or vendor identity alone never establishes an algorithm.
use std::str::FromStr;

use asic_rs_core::{
    data::device::HashAlgorithm,
    errors::ModelSelectionError,
    traits::model::{MinerModel, MinerModelAlgorithm},
};
use serde::{Deserialize, Serialize};
use strum::Display;
use ts_rs::TS;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display, TS)]
pub enum GoldshellModel {
    SC5Pro,
    #[strum(to_string = "{0}")]
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
        Ok(match name {
            "SC5PRO" => Self::SC5Pro,
            _ => Self::Unknown(raw.to_string()),
        })
    }
}

impl MinerModelAlgorithm for GoldshellModel {
    fn hash_algorithm(&self) -> HashAlgorithm {
        match self {
            Self::SC5Pro => HashAlgorithm::Blake2b,
            Self::Unknown(_) => HashAlgorithm::Unknown,
        }
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
    #[test]
    fn unverified_sc5_variants_and_ari31_keep_unknown_algorithms() {
        for name in ["SC5", "SC5ProX", "Goldshell-ARI31", "Goldshell"] {
            assert_eq!(
                GoldshellModel::from_str(name).unwrap().hash_algorithm(),
                HashAlgorithm::Unknown
            );
        }
    }
}

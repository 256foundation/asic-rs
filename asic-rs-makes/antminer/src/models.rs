// Modified for Support Extension (2026-10-07): exact hydro model
// identities and conservative normalization of manufacturer model strings.
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
pub enum AntMinerModel {
    #[serde(alias = "ANTMINER D3")]
    #[algorithm(HashAlgorithm::X11)]
    D3,
    #[serde(alias = "ANTMINER HS3")]
    #[algorithm(HashAlgorithm::Handshake)]
    HS3,
    #[serde(alias = "ANTMINER L3+")]
    #[algorithm(HashAlgorithm::Scrypt)]
    L3Plus,
    #[serde(alias = "ANTMINER L3++")]
    #[algorithm(HashAlgorithm::Scrypt)]
    L3PlusPlus,
    #[serde(alias = "ANTMINER KA3")]
    #[algorithm(HashAlgorithm::Kadena)]
    KA3,
    #[serde(alias = "ANTMINER KS3")]
    #[algorithm(HashAlgorithm::KHeavyHash)]
    KS3,
    #[serde(alias = "ANTMINER DR5")]
    #[algorithm(HashAlgorithm::Blake256R14)]
    DR5,
    #[serde(alias = "ANTMINER KS5")]
    #[algorithm(HashAlgorithm::KHeavyHash)]
    KS5,
    #[serde(alias = "ANTMINER KS5 PRO")]
    #[algorithm(HashAlgorithm::KHeavyHash)]
    KS5Pro,
    #[serde(alias = "ANTMINER L7")]
    #[algorithm(HashAlgorithm::Scrypt)]
    L7,
    #[serde(alias = "ANTMINER K7")]
    #[algorithm(HashAlgorithm::Eaglesong)]
    K7,
    #[serde(alias = "ANTMINER D7")]
    #[algorithm(HashAlgorithm::X11)]
    D7,
    #[serde(alias = "ANTMINER E9 PRO")]
    #[algorithm(HashAlgorithm::EtHash)]
    E9Pro,
    #[serde(alias = "ANTMINER D9")]
    #[algorithm(HashAlgorithm::X11)]
    D9,
    #[serde(alias = "ANTMINER S9")]
    #[algorithm(HashAlgorithm::SHA256)]
    S9,
    #[serde(alias = "ANTMINER S9I")]
    #[algorithm(HashAlgorithm::SHA256)]
    S9i,
    #[serde(alias = "ANTMINER S9J")]
    #[algorithm(HashAlgorithm::SHA256)]
    S9j,
    #[serde(alias = "ANTMINER T9")]
    #[algorithm(HashAlgorithm::SHA256)]
    T9,
    #[serde(alias = "ANTMINER L9")]
    #[algorithm(HashAlgorithm::Scrypt)]
    L9,
    #[serde(alias = "ANTMINER L11")]
    #[algorithm(HashAlgorithm::Scrypt)]
    L11,
    #[serde(alias = "ANTMINER Z15")]
    #[algorithm(HashAlgorithm::Equihash)]
    Z15,
    #[serde(alias = "ANTMINER Z15 PRO")]
    #[algorithm(HashAlgorithm::Equihash)]
    Z15Pro,
    #[serde(alias = "ANTMINER S17")]
    #[algorithm(HashAlgorithm::SHA256)]
    S17,
    #[serde(alias = "ANTMINER S17+")]
    #[algorithm(HashAlgorithm::SHA256)]
    S17Plus,
    #[serde(alias = "ANTMINER S17 PRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S17Pro,
    #[serde(alias = "ANTMINER S17E")]
    #[algorithm(HashAlgorithm::SHA256)]
    S17e,
    #[serde(alias = "ANTMINER T17")]
    #[algorithm(HashAlgorithm::SHA256)]
    T17,
    #[serde(alias = "ANTMINER T17+")]
    #[algorithm(HashAlgorithm::SHA256)]
    T17Plus,
    #[serde(alias = "ANTMINER T17E")]
    #[algorithm(HashAlgorithm::SHA256)]
    T17e,
    #[serde(alias = "ANTMINER S19")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19,
    #[serde(alias = "ANTMINER S19L")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19L,
    #[serde(alias = "ANTMINER S19 PRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19Pro,
    #[serde(alias = "ANTMINER S19J")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19j,
    #[serde(alias = "ANTMINER S19I")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19i,
    #[serde(alias = "ANTMINER S19+")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19Plus,
    #[serde(alias = "ANTMINER S19J88NOPIC")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19jNoPIC,
    #[serde(alias = "ANTMINER S19PRO+")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19ProPlus,
    #[serde(alias = "ANTMINER S19J PRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19jPro,
    #[serde(alias = "ANTMINER S19J PRO+")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19jProPlus,
    #[serde(alias = "ANTMINER S19 XP")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19XP,
    #[serde(alias = "ANTMINER S19A")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19a,
    #[serde(alias = "ANTMINER S19A PRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19aPro,
    #[serde(alias = "ANTMINER S19 HYDRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19Hydro,
    #[serde(alias = "ANTMINER S19 PRO HYD.")]
    #[serde(alias = "ANTMINER S19 PRO HYDRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19ProHydro,
    #[serde(alias = "ANTMINER S19 PRO+ HYD.")]
    #[serde(alias = "ANTMINER S19 PRO+ HYDRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19ProPlusHydro,
    #[serde(alias = "ANTMINER S19K PRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19KPro,
    #[serde(alias = "ANTMINER S19J XP")]
    #[algorithm(HashAlgorithm::SHA256)]
    S19jXP,
    #[serde(alias = "ANTMINER T19")]
    #[algorithm(HashAlgorithm::SHA256)]
    T19,
    #[serde(alias = "ANTMINER S21")]
    #[serde(alias = "ANTMINER BHB68601")]
    #[serde(alias = "ANTMINER BHB68606")]
    #[algorithm(HashAlgorithm::SHA256)]
    S21,
    #[serde(alias = "ANTMINER S21 PRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S21Pro,
    #[serde(alias = "ANTMINER S21 PRO+")]
    #[algorithm(HashAlgorithm::SHA256)]
    S21ProPlus,
    #[serde(alias = "ANTMINER S21 XP")]
    #[algorithm(HashAlgorithm::SHA256)]
    S21XP,
    #[serde(alias = "ANTMINER S21+")]
    #[algorithm(HashAlgorithm::SHA256)]
    S21Plus,
    #[serde(alias = "ANTMINER S21++")]
    #[algorithm(HashAlgorithm::SHA256)]
    S21PlusPlus,
    #[serde(alias = "ANTMINER S21 HYD.")]
    #[serde(alias = "ANTMINER S21 HYDRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S21Hydro,
    #[serde(alias = "ANTMINER S21+ HYD.")]
    #[serde(alias = "ANTMINER S21+ HYDRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S21PlusHydro,
    #[serde(alias = "ANTMINER S21E XP HYD.")]
    #[serde(alias = "ANTMINER S21E XP HYDRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S21eXPHydro,
    #[serde(alias = "ANTMINER S21 XP HYD.")]
    #[serde(alias = "ANTMINER S21 XP HYDRO")]
    #[serde(alias = "ANTMINER S21XPHYD.")]
    #[serde(alias = "ANTMINER S21XPHYDRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S21XPHydro,
    #[serde(alias = "ANTMINER S21J XP HYD.")]
    #[serde(alias = "ANTMINER S21J XP HYDRO")]
    #[serde(alias = "ANTMINER S21JXPHYD.")]
    #[serde(alias = "ANTMINER S21JXPHYDRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S21jXPHydro,
    #[serde(alias = "ANTMINER S23 HYD.")]
    #[serde(alias = "ANTMINER S23 HYDRO")]
    #[serde(alias = "ANTMINER S23HYD.")]
    #[serde(alias = "ANTMINER S23HYDRO")]
    #[algorithm(HashAlgorithm::SHA256)]
    S23Hydro,
    #[serde(alias = "ANTMINER T21")]
    #[algorithm(HashAlgorithm::SHA256)]
    T21,
    #[strum(to_string = "{0}")]
    #[algorithm(HashAlgorithm::Unknown)]
    Unknown(String),
}

impl FromStr for AntMinerModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Keep serialized enum names working, then normalize only formatting.
        // The serde aliases remain the exact model allowlist: a new suffix or
        // variant is never collapsed into the nearest known S21/S23 model.
        if let Ok(model) = serde_json::from_value(serde_json::Value::String(s.to_string())) {
            return Ok(model);
        }

        let normalized = s
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_uppercase();
        let without_manufacturer = normalized.strip_prefix("BITMAIN ").unwrap_or(&normalized);
        let model = without_manufacturer
            .strip_prefix("ANTMINER ")
            .unwrap_or(without_manufacturer);
        let model = if model.ends_with(" HYD") || model.ends_with("XPHYD") || model == "S23HYD" {
            format!("{model}.")
        } else {
            model.to_string()
        };
        serde_json::from_value(serde_json::Value::String(format!("ANTMINER {model}")))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl MinerModel for AntMinerModel {
    fn make_name(&self) -> String {
        "Antminer".to_string()
    }
    fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asic_rs_core::traits::model::MinerModelAlgorithm;
    use std::str::FromStr;

    #[test]
    fn known_model_parses() {
        let result = AntMinerModel::from_str("ANTMINER S21").unwrap();

        assert_eq!(result, AntMinerModel::S21);
    }

    #[test]
    fn unknown_model_falls_back() {
        let result = AntMinerModel::from_str("ANTMINER S99").unwrap();

        assert_eq!(result, AntMinerModel::Unknown("ANTMINER S99".to_string()));
    }

    #[test]
    fn l_series_is_scrypt() {
        for model in [
            AntMinerModel::L3Plus,
            AntMinerModel::L3PlusPlus,
            AntMinerModel::L7,
            AntMinerModel::L9,
            AntMinerModel::L11,
        ] {
            assert_eq!(model.hash_algorithm(), HashAlgorithm::Scrypt, "{model}");
        }
    }

    #[test]
    fn d_series_is_x11() {
        for model in [AntMinerModel::D3, AntMinerModel::D7, AntMinerModel::D9] {
            assert_eq!(model.hash_algorithm(), HashAlgorithm::X11, "{model}");
        }
    }

    #[test]
    fn sha256_models_are_unchanged() {
        for model in [
            AntMinerModel::S9,
            AntMinerModel::S19,
            AntMinerModel::S21,
            AntMinerModel::T21,
        ] {
            assert_eq!(model.hash_algorithm(), HashAlgorithm::SHA256, "{model}");
        }
    }

    #[test]
    fn non_sha256_models_use_their_declared_algorithm() {
        for (model, expected) in [
            (AntMinerModel::HS3, HashAlgorithm::Handshake),
            (AntMinerModel::DR5, HashAlgorithm::Blake256R14),
            (AntMinerModel::KA3, HashAlgorithm::Kadena),
            (AntMinerModel::KS3, HashAlgorithm::KHeavyHash),
            (AntMinerModel::KS5, HashAlgorithm::KHeavyHash),
            (AntMinerModel::KS5Pro, HashAlgorithm::KHeavyHash),
            (AntMinerModel::K7, HashAlgorithm::Eaglesong),
            (AntMinerModel::E9Pro, HashAlgorithm::EtHash),
            (AntMinerModel::Z15, HashAlgorithm::Equihash),
            (AntMinerModel::Z15Pro, HashAlgorithm::Equihash),
        ] {
            assert_eq!(model.hash_algorithm(), expected, "{model}");
        }
    }

    #[test]
    fn unknown_model_uses_unknown_algorithm() {
        let model = AntMinerModel::from_str("ANTMINER S99").unwrap();

        assert_eq!(model.hash_algorithm(), HashAlgorithm::Unknown);
    }

    #[test]
    fn distinct_hydro_models_parse_with_exact_aliases_and_formatting() {
        for (expected, aliases) in [
            (
                AntMinerModel::S21XPHydro,
                vec![
                    "ANTMINER S21 XP HYD.",
                    "Antminer S21 XP Hyd",
                    "S21 XP Hydro",
                    "S21XPHyd.",
                    "s21xphyd",
                    "S21XPHydro",
                ],
            ),
            (
                AntMinerModel::S21jXPHydro,
                vec![
                    "ANTMINER S21J XP HYD.",
                    "Antminer S21j XP Hyd",
                    "S21j XP Hydro",
                    "S21jXPHyd.",
                    "s21jxphyd",
                    "S21jXPHydro",
                ],
            ),
            (
                AntMinerModel::S23Hydro,
                vec![
                    "ANTMINER S23 HYD.",
                    "Antminer S23 Hyd",
                    "S23 Hydro",
                    "S23Hyd.",
                    "s23hyd",
                    "S23Hydro",
                ],
            ),
        ] {
            for alias in aliases {
                assert_eq!(AntMinerModel::from_str(alias).unwrap(), expected, "{alias}");
            }
            assert_eq!(expected.hash_algorithm(), HashAlgorithm::SHA256);
            assert!(expected.is_known());
            assert_eq!(
                serde_json::from_str::<AntMinerModel>(&serde_json::to_string(&expected).unwrap())
                    .unwrap(),
                expected
            );
        }
    }

    #[test]
    fn normalizes_manufacturer_prefix_case_and_whitespace() {
        for (alias, expected) in [
            (
                "  Bitmain  Antminer\tS21j   XP Hyd  ",
                AntMinerModel::S21jXPHydro,
            ),
            ("bitmain S21 XP Hyd.", AntMinerModel::S21XPHydro),
            ("s21 pro+", AntMinerModel::S21ProPlus),
            ("AntMiner S21+ Hyd", AntMinerModel::S21PlusHydro),
            ("s21e XP hydro", AntMinerModel::S21eXPHydro),
            ("s19 pro HYD", AntMinerModel::S19ProHydro),
        ] {
            assert_eq!(AntMinerModel::from_str(alias).unwrap(), expected, "{alias}");
        }
    }

    #[test]
    fn unsupported_variants_remain_unknown_without_losing_their_original_identity() {
        for alias in [
            "Antminer S21j Pro",
            "Antminer S21e",
            "S21 XP+ Hyd.",
            "S21j XP",
            "S23",
            "S23 XP Hyd.",
            "S23 Hyd. 3U",
            "S21 XP Hyd. Prototype",
            "Antminer S21++ Hyd",
        ] {
            let model = AntMinerModel::from_str(alias).unwrap();
            assert_eq!(model, AntMinerModel::Unknown(alias.to_string()), "{alias}");
            assert!(!model.is_known());
            assert_eq!(model.hash_algorithm(), HashAlgorithm::Unknown);
        }
    }
}

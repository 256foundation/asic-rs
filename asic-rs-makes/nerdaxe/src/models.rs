#[cfg(feature = "python")]
use pyo3::prelude::*;
use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
use serde::{Deserialize, Serialize};
use strum::Display;

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
pub enum NerdAxeModel {
    #[serde(alias = "BM1368")]
    NerdAxe,
    #[serde(alias = "BM1370", alias = "nerdqaxe++", alias = "NerdQAxe++")]
    NerdQAxe,
    #[serde(alias = "BM1397")]
    NerdMiner,
    #[serde(alias = "BM1366")]
    NerdAxeUltra,
}

impl FromStr for NerdAxeModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| ModelSelectionError::UnknownModel(s.to_string()))
    }
}

impl asic_rs_core::traits::model::MinerModel for NerdAxeModel {
    fn make_name(&self) -> String {
        "Nerdaxe".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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
}

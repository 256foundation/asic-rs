#[cfg(feature = "python")]
use pyo3::prelude::*;

use serde::{Deserialize, Serialize};
use strum::Display;

#[cfg_attr(feature = "python", pyclass(str, module = "asic_rs"))]
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

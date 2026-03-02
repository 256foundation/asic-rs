#[cfg(feature = "python")]
use pyo3::prelude::*;

use asic_rs_core::errors::ModelSelectionError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::Display;

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
pub enum BitaxeModel {
    #[serde(alias = "BM1368")]
    Supra,
    #[serde(alias = "BM1370")]
    Gamma,
    #[serde(alias = "BM1397")]
    Max,
    #[serde(alias = "BM1366")]
    Ultra,
}

impl FromStr for BitaxeModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| ModelSelectionError::UnknownModel(s.to_string()))
    }
}

impl asic_rs_core::traits::model::MinerModel for BitaxeModel {
    fn make_name(&self) -> String {
        "Bitaxe".to_string()
    }
}

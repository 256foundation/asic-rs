use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
pub enum EPicModel {
    #[serde(alias = "BLOCKMINER 520i")]
    BM520i,
    #[serde(alias = "ANTMINER S19J PRO DUAL")]
    S19JProDual,
}

impl FromStr for EPicModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| ModelSelectionError::UnknownModel(s.to_string()))
    }
}

impl asic_rs_core::traits::model::MinerModel for EPicModel {
    fn make_name(&self) -> String {
        "ePIC".to_string()
    }
}

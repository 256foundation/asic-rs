use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
pub enum SealMinerModel {
    A2,
}

impl FromStr for SealMinerModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| ModelSelectionError::UnknownModel(s.to_string()))
    }
}

impl asic_rs_core::traits::model::MinerModel for SealMinerModel {
    fn make_name(&self) -> String {
        "Sealminer".to_string()
    }
}

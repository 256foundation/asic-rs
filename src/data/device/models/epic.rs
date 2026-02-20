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

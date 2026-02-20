#[cfg(feature = "python")]
use pyo3::prelude::*;

use serde::{Deserialize, Serialize};
use strum::Display;

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, Hash)]
pub enum AvalonMinerModel {
    #[serde(alias = "721")]
    Avalon721,
    #[serde(alias = "741")]
    Avalon741,
    #[serde(alias = "761")]
    Avalon761,
    #[serde(alias = "821")]
    Avalon821,
    #[serde(alias = "841")]
    Avalon841,
    #[serde(alias = "851")]
    Avalon851,
    #[serde(alias = "921")]
    Avalon921,
    #[serde(alias = "1026")]
    Avalon1026,
    #[serde(alias = "1047")]
    Avalon1047,
    #[serde(alias = "1066")]
    Avalon1066,
    #[serde(alias = "1166PRO")]
    Avalon1166Pro,
    #[serde(alias = "1126PRO")]
    Avalon1126Pro,
    #[serde(alias = "1246")]
    Avalon1246,
    #[serde(alias = "1566")]
    Avalon1566,
    #[serde(alias = "NANO3")]
    AvalonNano3,
    #[serde(alias = "NANO3S")]
    AvalonNano3s,
    #[serde(alias = "Q")]
    AvalonHomeQ,
}

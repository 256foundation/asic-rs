#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display as StrumDisplay, EnumString};

use crate::traits::{firmware::MinerFirmware, model::MinerModel};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
pub struct DeviceInfo {
    pub make: String,
    pub model: String,
    pub hardware: MinerHardware,
    pub firmware: String,
    pub algo: HashAlgorithm,
}

impl DeviceInfo {
    pub fn new(model: impl MinerModel, firmware: impl MinerFirmware, algo: HashAlgorithm) -> Self {
        Self {
            hardware: model.clone().into(),
            make: model.make_name(),
            model: model.to_string(),
            firmware: firmware.to_string(),
            algo,
        }
    }
}

#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Default)]
pub struct MinerHardware {
    pub chips: Option<u16>,
    pub fans: Option<u8>,
    pub boards: Option<u8>,
}

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[derive(
    Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, StrumDisplay, EnumString,
)]
pub enum HashAlgorithm {
    #[serde(rename = "SHA256")]
    SHA256,
    #[serde(rename = "Scrypt")]
    Scrypt,
    #[serde(rename = "X11")]
    X11,
    #[serde(rename = "Blake2S256")]
    Blake2S256,
    #[serde(rename = "Kadena")]
    Kadena,
}

#[cfg_attr(feature = "python", pymethods)]
impl HashAlgorithm {
    pub fn __repr__(&self) -> String {
        self.to_string()
    }
}

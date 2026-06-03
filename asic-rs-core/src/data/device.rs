#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display as StrumDisplay, EnumString};

use crate::traits::{firmware::MinerFirmware, model::MinerModel};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyclass(from_py_object, module = "asic_rs"))]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
/// Static identity and hardware information for a miner model.
pub struct DeviceInfo {
    /// Miner manufacturer or make.
    pub make: String,
    /// Miner model name.
    pub model: String,
    /// Expected hardware shape.
    pub hardware: MinerHardware,
    /// Firmware name or family.
    pub firmware: String,
    /// Mining hash algorithm.
    pub algo: HashAlgorithm,
}

impl DeviceInfo {
    /// Build device information from a model and firmware implementation.
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

#[cfg_attr(feature = "python", pyclass(from_py_object, module = "asic_rs"))]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Default)]
/// Expected hardware counts for a miner model.
pub struct MinerHardware {
    /// Expected number of fans.
    pub fans: Option<u8>,
    /// Expected hashboards, represented as the expected number of chips per board.
    pub boards: Option<Vec<Option<u16>>>,
}

impl MinerHardware {
    /// Expected number of hashboards.
    pub fn board_count(&self) -> Option<u8> {
        self.boards
            .as_ref()
            .and_then(|boards| u8::try_from(boards.len()).ok())
    }

    /// Expected total chip count across all hashboards.
    pub fn total_chips(&self) -> Option<u16> {
        self.boards
            .as_ref()
            .map(|boards| boards.iter().copied().flatten().sum())
    }

    /// Expected chip count for a specific hashboard position.
    pub fn chips_for_board(&self, position: usize) -> Option<u16> {
        self.boards
            .as_ref()
            .and_then(|boards| boards.get(position).copied().flatten())
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl MinerHardware {
    #[getter]
    pub fn fans(&self) -> Option<u8> {
        self.fans
    }

    #[getter]
    pub fn boards(&self) -> Option<Vec<Option<u16>>> {
        self.boards.clone()
    }

    #[getter]
    pub fn chips(&self) -> Option<u16> {
        self.total_chips()
    }

    #[getter]
    #[pyo3(name = "board_count")]
    pub fn py_board_count(&self) -> Option<u8> {
        self.board_count()
    }
}

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[cfg_attr(feature = "python", derive(asic_rs_pydantic::PyPydanticEnum))]
#[derive(
    Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, StrumDisplay, EnumString,
)]
/// Mining hash algorithm.
pub enum HashAlgorithm {
    /// SHA-256 mining.
    #[cfg_attr(feature = "python", pydantic(value = "SHA256"))]
    #[serde(rename = "SHA256")]
    SHA256,
    /// Scrypt mining.
    #[cfg_attr(feature = "python", pydantic(value = "Scrypt"))]
    #[serde(rename = "Scrypt")]
    Scrypt,
    /// X11 mining.
    #[cfg_attr(feature = "python", pydantic(value = "X11"))]
    #[serde(rename = "X11")]
    X11,
    /// Blake2S256 mining.
    #[cfg_attr(feature = "python", pydantic(value = "Blake2S256"))]
    #[serde(rename = "Blake2S256")]
    Blake2S256,
    /// Kadena mining.
    #[cfg_attr(feature = "python", pydantic(value = "Kadena"))]
    #[serde(rename = "Kadena")]
    Kadena,
}

#[cfg_attr(feature = "python", pymethods)]
impl HashAlgorithm {
    pub fn __repr__(&self) -> String {
        self.to_string()
    }
}

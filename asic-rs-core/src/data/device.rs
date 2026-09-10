#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display as StrumDisplay, EnumIter, EnumString};
use ts_rs::TS;

use crate::traits::{firmware::MinerFirmware, model::MinerModel};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "python", pyclass(from_py_object, module = "asic_rs"))]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model(getters))]
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
#[cfg_attr(feature = "python", derive(asic_rs_pydantic::PyPydanticEnum))]
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Hash,
    Serialize,
    Deserialize,
    StrumDisplay,
    EnumString,
    EnumIter,
    TS,
)]
/// Miner firmware name or family.
pub enum FirmwareType {
    #[cfg_attr(feature = "python", pydantic(value = "AntMiner Stock"))]
    #[serde(rename = "AntMiner Stock")]
    #[strum(serialize = "AntMiner Stock")]
    AntMinerStock,
    #[cfg_attr(feature = "python", pydantic(value = "Auradine Stock"))]
    #[serde(rename = "Auradine Stock")]
    #[strum(serialize = "Auradine Stock")]
    AuradineStock,
    #[cfg_attr(feature = "python", pydantic(value = "AvalonMiner Stock"))]
    #[serde(rename = "AvalonMiner Stock")]
    #[strum(serialize = "AvalonMiner Stock")]
    AvalonMinerStock,
    #[cfg_attr(feature = "python", pydantic(value = "Bitaxe Stock"))]
    #[serde(rename = "Bitaxe Stock")]
    #[strum(serialize = "Bitaxe Stock")]
    BitaxeStock,
    #[cfg_attr(feature = "python", pydantic(value = "Braiins"))]
    #[serde(rename = "Braiins")]
    #[strum(serialize = "Braiins")]
    Braiins,
    #[cfg_attr(feature = "python", pydantic(value = "Elphapex Stock"))]
    #[serde(rename = "Elphapex Stock")]
    #[strum(serialize = "Elphapex Stock")]
    ElphapexStock,
    #[cfg_attr(feature = "python", pydantic(value = "FutureBit Stock"))]
    #[serde(rename = "FutureBit Stock")]
    #[strum(serialize = "FutureBit Stock")]
    FutureBitStock,
    #[cfg_attr(feature = "python", pydantic(value = "LuxOS"))]
    #[serde(rename = "LuxOS")]
    #[strum(serialize = "LuxOS")]
    LuxOS,
    #[cfg_attr(feature = "python", pydantic(value = "Marathon"))]
    #[serde(rename = "Marathon")]
    #[strum(serialize = "Marathon")]
    Marathon,
    #[cfg_attr(feature = "python", pydantic(value = "Nerdaxe Stock"))]
    #[serde(rename = "Nerdaxe Stock")]
    #[strum(serialize = "Nerdaxe Stock")]
    NerdaxeStock,
    #[cfg_attr(feature = "python", pydantic(value = "Proto Stock"))]
    #[serde(rename = "Proto Stock")]
    #[strum(serialize = "Proto Stock")]
    ProtoStock,
    #[cfg_attr(feature = "python", pydantic(value = "SealMiner Stock"))]
    #[serde(rename = "SealMiner Stock")]
    #[strum(serialize = "SealMiner Stock")]
    SealMinerStock,
    #[cfg_attr(feature = "python", pydantic(value = "UMC OS"))]
    #[serde(rename = "UMC OS")]
    #[strum(serialize = "UMC OS")]
    UmcOS,
    #[cfg_attr(feature = "python", pydantic(value = "VNish"))]
    #[serde(rename = "VNish")]
    #[strum(serialize = "VNish")]
    VNish,
    #[cfg_attr(feature = "python", pydantic(value = "VolcMiner Stock"))]
    #[serde(rename = "VolcMiner Stock")]
    #[strum(serialize = "VolcMiner Stock")]
    VolcMinerStock,
    #[cfg_attr(feature = "python", pydantic(value = "WhatsMiner Stock"))]
    #[serde(rename = "WhatsMiner Stock")]
    #[strum(serialize = "WhatsMiner Stock")]
    WhatsMinerStock,
}

#[cfg_attr(feature = "python", pyclass(from_py_object, module = "asic_rs"))]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model(getters))]
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Default, TS)]
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
    pub fn chips(&self) -> Option<u16> {
        self.total_chips()
    }

    #[getter]
    #[pyo3(name = "board_count")]
    pub fn py_board_count(&self) -> Option<u8> {
        self.board_count()
    }
}

#[cfg_attr(feature = "python", pyclass(from_py_object, module = "asic_rs"))]
#[cfg_attr(feature = "python", derive(asic_rs_pydantic::PyPydanticEnum))]
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Hash,
    Serialize,
    Deserialize,
    StrumDisplay,
    EnumString,
    EnumIter,
    TS,
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
    /// kHeavyHash mining, as used by Kaspa.
    #[cfg_attr(feature = "python", pydantic(value = "KHeavyHash"))]
    #[serde(rename = "KHeavyHash")]
    KHeavyHash,
    /// Eaglesong mining, as used by Nervos CKB.
    #[cfg_attr(feature = "python", pydantic(value = "Eaglesong"))]
    #[serde(rename = "Eaglesong")]
    Eaglesong,
    /// EtHash mining.
    #[cfg_attr(feature = "python", pydantic(value = "EtHash"))]
    #[serde(rename = "EtHash")]
    EtHash,
    /// Equihash mining, as used by Zcash.
    #[cfg_attr(feature = "python", pydantic(value = "Equihash"))]
    #[serde(rename = "Equihash")]
    Equihash,
    /// Handshake mining (Blake2b followed by SHA3).
    #[cfg_attr(feature = "python", pydantic(value = "Handshake"))]
    #[serde(rename = "Handshake")]
    Handshake,
    /// Blake256R14 mining, as used by Decred.
    #[cfg_attr(feature = "python", pydantic(value = "Blake256R14"))]
    #[serde(rename = "Blake256R14")]
    Blake256R14,
    /// An algorithm this crate cannot name.
    ///
    /// Reported when a miner names an algorithm that is not one of the above,
    /// so that an unrecognised value is not silently presented as SHA-256.
    /// Note this does not carry the original text: [`HashAlgorithm`] is a
    /// fieldless enum so that it stays `Copy`, serialises as a plain string,
    /// and works with the pydantic and TypeScript derives.
    #[cfg_attr(feature = "python", pydantic(value = "Unknown"))]
    #[serde(rename = "Unknown")]
    Unknown,
}

#[cfg_attr(feature = "python", pymethods)]
impl HashAlgorithm {
    pub fn __repr__(&self) -> String {
        self.to_string()
    }

    pub fn __str__(&self) -> String {
        self.to_string()
    }

    /// Compare against another [`HashAlgorithm`] or against its name.
    ///
    /// Without this, `hashrate.algo == "SHA256"` would evaluate to `False`
    /// rather than raising -- a silently wrong branch with no traceback. The
    /// string form is accepted so that callers who treated this as a plain
    /// string keep working.
    #[cfg(feature = "python")]
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(other) = other.extract::<HashAlgorithm>() {
            return *self == other;
        }
        other
            .extract::<String>()
            .is_ok_and(|name| name == self.to_string())
    }

    #[cfg(feature = "python")]
    fn __ne__(&self, other: &Bound<'_, PyAny>) -> bool {
        !self.__eq__(other)
    }

    /// Defining `__eq__` drops the inherited hash, so restore it explicitly --
    /// these are used as dict keys and in sets.
    #[cfg(feature = "python")]
    fn __hash__(&self) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use strum::IntoEnumIterator;

    use super::*;

    /// `Display` and `EnumString` are derived independently, so a variant whose
    /// rendered name does not parse back would be a silent one-way trip.
    /// Callers resolving an algorithm from a string rely on this holding.
    #[test]
    fn every_algorithm_round_trips_through_its_name() {
        for algo in HashAlgorithm::iter() {
            let rendered = algo.to_string();
            assert_eq!(
                HashAlgorithm::from_str(&rendered).ok(),
                Some(algo),
                "{rendered} did not round-trip"
            );
        }
    }

    /// `Unknown` is an explicit value a caller opts into, not a catch-all that
    /// `from_str` falls back to -- otherwise a typo would parse successfully
    /// and the round-trip test above would not catch it.
    #[test]
    fn unrecognised_names_do_not_parse_as_unknown() {
        assert!(HashAlgorithm::from_str("NotAnAlgorithm").is_err());
        assert_eq!(
            HashAlgorithm::from_str("Unknown").ok(),
            Some(HashAlgorithm::Unknown)
        );
    }
}

#[cfg(feature = "python")]
use pyo3::prelude::*;
use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use strum::Display;

pub mod models;
pub use models::MinerModel;

#[cfg_attr(feature = "python", pyclass(str, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
pub enum MinerFirmware {
    #[serde(rename = "Stock")]
    Stock,
    #[serde(rename = "BraiinsOS")]
    BraiinsOS,
    #[serde(rename = "VNish")]
    VNish,
    #[serde(rename = "ePIC")]
    EPic,
    #[serde(rename = "HiveOS")]
    HiveOS,
    #[serde(rename = "LuxOS")]
    LuxOS,
    #[serde(rename = "Marathon")]
    Marathon,
}

#[cfg_attr(feature = "python", pyclass(str, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
pub enum MinerMake {
    #[serde(rename = "AntMiner")]
    AntMiner,
    #[serde(rename = "WhatsMiner")]
    WhatsMiner,
    #[serde(rename = "AvalonMiner")]
    AvalonMiner,
    #[serde(rename = "ePIC")]
    EPic,
    #[serde(rename = "Braiins")]
    Braiins,
    #[serde(rename = "Bitaxe")]
    Bitaxe,
}

#[cfg_attr(feature = "python", pyclass(str, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
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

#[cfg_attr(feature = "python", pyclass(get_all, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub make: MinerMake,
    pub model: MinerModel,
    pub hardware: MinerHardware,
    pub firmware: MinerFirmware,
    pub algo: HashAlgorithm,
}

impl DeviceInfo {
    pub(crate) fn new(
        make: MinerMake,
        model: MinerModel,
        firmware: MinerFirmware,
        algo: HashAlgorithm,
    ) -> Self {
        Self {
            make,
            hardware: MinerHardware::from(&model),
            model,
            firmware,
            algo,
        }
    }
}

#[cfg_attr(feature = "python", pyclass(get_all, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct MinerHardware {
    pub chips: Option<u16>,
    pub fans: Option<u8>,
    pub boards: Option<u8>,
}

impl From<&MinerModel> for MinerHardware {
    fn from(model: &MinerModel) -> Self {
        match model {
            MinerModel::AntMiner(model_name) => Self::from(model_name),
            MinerModel::WhatsMiner(model_name) => Self::from(model_name),
            MinerModel::Braiins(model_name) => Self::from(model_name),
            MinerModel::Bitaxe(model_name) => Self::from(model_name),
            MinerModel::EPic(model_name) => Self::from(model_name),
            MinerModel::AvalonMiner(model_name) => Self::from(model_name),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum MinerControlBoard {
    // Antminer control boards
    #[serde(rename = "Xilinx")]
    Xilinx,
    #[serde(rename = "BeagleBoneBlack")]
    BeagleBoneBlack,
    #[serde(rename = "AMLogic")]
    AMLogic,
    #[serde(rename = "CVITek")]
    CVITek,
    // Whatsminer control boards
    #[serde(rename = "H3")]
    H3,
    #[serde(rename = "H6")]
    H6,
    #[serde(rename = "H6OS")]
    H6OS,
    #[serde(rename = "H616")]
    H616,
    // Avalon control boards
    #[serde(rename = "MM3v2X3")]
    MM3v2X3,
    #[serde(rename = "MM3v1X3")]
    MM3v1X3,
    #[serde(rename = "MM3v1")]
    MM3v1,
    // Bitaxe control boards
    #[serde(rename = "B102")]
    B102,
    #[serde(rename = "B201")]
    B201,
    #[serde(rename = "B202")]
    B202,
    #[serde(rename = "B203")]
    B203,
    #[serde(rename = "B204")]
    B204,
    #[serde(rename = "B205")]
    B205,
    #[serde(rename = "B207")]
    B207,
    #[serde(rename = "B401")]
    B401,
    #[serde(rename = "B402")]
    B402,
    #[serde(rename = "B403")]
    B403,
    #[serde(rename = "B601")]
    B601,
    #[serde(rename = "B602")]
    B602,
    #[serde(rename = "B800")]
    B800,
    // Custom control boards
    #[serde(rename = "BraiinsCB")]
    BraiinsCB,
    #[serde(rename = "ePIC UMC")]
    EPicUMC,
    #[serde(rename = "MaraCB")]
    MaraCB,
    // Unknown
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct ControlBoardParseError;

impl Display for ControlBoardParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse control board type")
    }
}

impl FromStr for MinerControlBoard {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cb_model = s.trim().replace(" ", "").to_uppercase();
        match cb_model.as_ref() {
            "XILINX" => Ok(Self::Xilinx),
            "BBB" => Ok(Self::BeagleBoneBlack),
            "BB" => Ok(Self::BeagleBoneBlack),
            "BEAGLEBONE" => Ok(Self::BeagleBoneBlack),
            "BEAGLEBONEBLACK" => Ok(Self::BeagleBoneBlack),
            "CVITEK" => Ok(Self::CVITek),
            "CVCTRL" => Ok(Self::CVITek),
            "AMLOGIC" => Ok(Self::AMLogic),
            "AML" => Ok(Self::AMLogic),
            "H3" => Ok(Self::H3),
            "H6" => Ok(Self::H6),
            "H6OS" => Ok(Self::H6OS),
            "H616" => Ok(Self::H616),
            "MM3V2_X3" => Ok(Self::MM3v2X3),
            "MM3V1_X3" => Ok(Self::MM3v1X3),
            "MM3V1" => Ok(Self::MM3v1),
            "102" => Ok(Self::B102),
            "201" => Ok(Self::B201),
            "202" => Ok(Self::B202),
            "203" => Ok(Self::B203),
            "204" => Ok(Self::B204),
            "205" => Ok(Self::B205),
            "207" => Ok(Self::B207),
            "401" => Ok(Self::B401),
            "402" => Ok(Self::B402),
            "403" => Ok(Self::B403),
            "601" => Ok(Self::B601),
            "602" => Ok(Self::B602),
            "800" => Ok(Self::B800),
            "BraiinsCB" => Ok(Self::BraiinsCB),
            "ePIC UMC" => Ok(Self::EPicUMC),
            _ => Ok(Self::Unknown(s.to_string())),
        }
    }
}

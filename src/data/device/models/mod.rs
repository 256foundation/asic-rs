#[cfg(feature = "python")]
use pyo3::prelude::*;

use super::{MinerFirmware, MinerMake};
use antminer::AntMinerModel;
use avalon::AvalonMinerModel;
use bitaxe::BitaxeModel;
use braiins::BraiinsModel;
use epic::EPicModel;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use whatsminer::WhatsMinerModel;

pub mod antminer;
pub mod avalon;
pub mod bitaxe;
pub mod braiins;
pub mod epic;
pub mod whatsminer;

#[derive(Debug, Clone)]
pub enum ModelSelectionError {
    UnknownModel(String),
    NoModelResponse,
    UnexpectedModelResponse,
}

impl Display for ModelSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelSelectionError::UnknownModel(model) => write!(f, "Unknown model: {}", model),
            ModelSelectionError::NoModelResponse => write!(f, "No response when querying model"),
            ModelSelectionError::UnexpectedModelResponse => {
                write!(f, "Response to model query was formatted unexpectedly")
            }
        }
    }
}

impl std::error::Error for ModelSelectionError {}

impl FromStr for WhatsMinerModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| ModelSelectionError::UnknownModel(s.to_string()))
    }
}
impl FromStr for AntMinerModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| ModelSelectionError::UnknownModel(s.to_string()))
    }
}
impl FromStr for BitaxeModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| ModelSelectionError::UnknownModel(s.to_string()))
    }
}

impl FromStr for BraiinsModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| ModelSelectionError::UnknownModel(s.to_string()))
    }
}

impl FromStr for AvalonMinerModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| ModelSelectionError::UnknownModel(s.to_string()))
    }
}

impl FromStr for EPicModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|_| ModelSelectionError::UnknownModel(s.to_string()))
    }
}

#[cfg_attr(feature = "python", pyclass(str, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MinerModel {
    AntMiner(AntMinerModel),
    WhatsMiner(WhatsMinerModel),
    Braiins(BraiinsModel),
    Bitaxe(BitaxeModel),
    AvalonMiner(AvalonMinerModel),
    EPic(EPicModel),
}

impl Display for MinerModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MinerModel::AntMiner(m) => Ok(m.fmt(f)?),
            MinerModel::WhatsMiner(m) => Ok(m.fmt(f)?),
            MinerModel::Braiins(m) => Ok(m.fmt(f)?),
            MinerModel::Bitaxe(m) => Ok(m.fmt(f)?),
            MinerModel::EPic(m) => Ok(m.fmt(f)?),
            MinerModel::AvalonMiner(m) => Ok(m.fmt(f)?),
        }
    }
}

impl From<MinerModel> for MinerMake {
    fn from(model: MinerModel) -> Self {
        match model {
            MinerModel::AntMiner(_) => MinerMake::AntMiner,
            MinerModel::WhatsMiner(_) => MinerMake::WhatsMiner,
            MinerModel::Braiins(_) => MinerMake::Braiins,
            MinerModel::Bitaxe(_) => MinerMake::Bitaxe,
            MinerModel::EPic(_) => MinerMake::EPic,
            MinerModel::AvalonMiner(_) => MinerMake::AvalonMiner,
        }
    }
}

pub(crate) struct MinerModelFactory {
    make: Option<MinerMake>,
    firmware: Option<MinerFirmware>,
}

impl MinerModelFactory {
    pub fn new() -> Self {
        MinerModelFactory {
            make: None,
            firmware: None,
        }
    }

    pub(crate) fn with_make(&mut self, make: MinerMake) -> &Self {
        self.make = Some(make);
        self
    }
    pub(crate) fn with_firmware(&mut self, firmware: MinerFirmware) -> &Self {
        self.firmware = Some(firmware);
        self
    }

    pub(crate) fn parse_model(&self, model_str: &str) -> Result<MinerModel, ModelSelectionError> {
        match self.make {
            Some(MinerMake::AntMiner) => {
                let model = AntMinerModel::from_str(model_str)?;
                Ok(MinerModel::AntMiner(model))
            }
            Some(MinerMake::WhatsMiner) => {
                let model = WhatsMinerModel::from_str(model_str)?;
                Ok(MinerModel::WhatsMiner(model))
            }
            Some(MinerMake::Bitaxe) => {
                let model = BitaxeModel::from_str(model_str)?;
                Ok(MinerModel::Bitaxe(model))
            }
            Some(MinerMake::AvalonMiner) => {
                let model = AvalonMinerModel::from_str(model_str)?;
                Ok(MinerModel::AvalonMiner(model))
            }
            None => match self.firmware {
                Some(MinerFirmware::BraiinsOS) => {
                    match (
                        AntMinerModel::from_str(model_str),
                        BraiinsModel::from_str(model_str),
                    ) {
                        (Ok(model), _) => Ok(MinerModel::AntMiner(model)),
                        (_, Ok(model)) => Ok(MinerModel::Braiins(model)),
                        // errors should all be the same with the same from_str implementation
                        (Err(am_err), _) => Err(am_err),
                    }
                }
                Some(MinerFirmware::EPic) => {
                    match (
                        AntMinerModel::from_str(model_str),
                        WhatsMinerModel::from_str(model_str),
                        EPicModel::from_str(model_str),
                    ) {
                        (Ok(model), _, _) => Ok(MinerModel::AntMiner(model)),
                        (_, Ok(model), _) => Ok(MinerModel::WhatsMiner(model)),
                        (_, _, Ok(model)) => Ok(MinerModel::EPic(model)),
                        // errors should all be the same with the same from_str implementation
                        (Err(am_err), _, _) => Err(am_err),
                    }
                }
                Some(MinerFirmware::LuxOS) => match AntMinerModel::from_str(model_str) {
                    Ok(model) => Ok(MinerModel::AntMiner(model)),
                    Err(am_err) => Err(am_err),
                },
                Some(MinerFirmware::Marathon) => match AntMinerModel::from_str(model_str) {
                    Ok(model) => Ok(MinerModel::AntMiner(model)),
                    Err(am_err) => Err(am_err),
                },
                Some(MinerFirmware::VNish) => match AntMinerModel::from_str(model_str) {
                    Ok(model) => Ok(MinerModel::AntMiner(model)),
                    Err(am_err) => Err(am_err),
                },
                Some(MinerFirmware::HiveOS) => match AntMinerModel::from_str(model_str) {
                    Ok(model) => Ok(MinerModel::AntMiner(model)),
                    Err(am_err) => Err(am_err),
                },
                // Stock is checked by make
                Some(MinerFirmware::Stock) => unreachable!(),
                // (None, None) must be checked before this point
                None => unreachable!(),
            },
            // Braiins and EPic must be found via the firmware model check
            Some(MinerMake::Braiins) => unreachable!(),
            Some(MinerMake::EPic) => unreachable!(),
        }
    }
}

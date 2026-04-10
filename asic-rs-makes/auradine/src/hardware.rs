use asic_rs_core::data::{board::MinerControlBoard, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::models::AuradineModel;

impl From<AuradineModel> for MinerHardware {
    fn from(model: AuradineModel) -> Self {
        match model {
            AuradineModel::AT2880 => Self {
                chips: Some(138),
                fans: Some(4),
                boards: Some(3),
            },
            AuradineModel::AT1500 => Self {
                chips: Some(132),
                fans: Some(4),
                boards: Some(3),
            },
            AuradineModel::Unknown(_)
            | AuradineModel::AI2500
            | AuradineModel::AI3680
            | AuradineModel::AH3880 => Self {
                chips: None,
                fans: None,
                boards: None,
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum AuradineControlBoard {
    #[serde(rename = "T0")]
    T0,
    #[serde(rename = "T1")]
    T1,
    #[serde(rename = "T2")]
    T2,
    #[serde(rename = "T3")]
    T3,
}

impl AuradineControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "T0" => Some(Self::T0),
            "T1" => Some(Self::T1),
            "T2" => Some(Self::T2),
            "T3" => Some(Self::T3),
            _ => None,
        }
    }
}

impl From<AuradineControlBoard> for MinerControlBoard {
    fn from(cb: AuradineControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

use asic_rs_core::data::{board::MinerControlBoard, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::models::EPicModel;

impl From<EPicModel> for MinerHardware {
    fn from(value: EPicModel) -> Self {
        match value {
            EPicModel::BM520i => Self {
                chips: Some(124),
                fans: Some(4),
                boards: Some(3),
            },
            EPicModel::S19JProDual => Self {
                chips: Some(126),
                fans: Some(8),
                boards: Some(6),
            },
            EPicModel::Unknown(_) => Default::default(),
        }
    }
}
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum EPicControlBoard {
    #[serde(rename = "ePIC UMC")]
    EPicUMC,
}

impl EPicControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        let cb_model = s.trim().replace(" ", "").to_uppercase();
        match cb_model.as_ref() {
            "EPICUMC" => Some(Self::EPicUMC),
            _ => None,
        }
    }
}

impl From<EPicControlBoard> for MinerControlBoard {
    fn from(cb: EPicControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

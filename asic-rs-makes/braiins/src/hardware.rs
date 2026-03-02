use crate::models::BraiinsModel;
use asic_rs_core::data::board::MinerControlBoard;
use asic_rs_core::data::device::MinerHardware;
use serde::{Deserialize, Serialize};
use strum::Display;

impl From<BraiinsModel> for MinerHardware {
    fn from(value: BraiinsModel) -> Self {
        match value {
            BraiinsModel::BMM100 => Self {
                chips: None,
                fans: Some(1),
                boards: Some(1),
            },
            BraiinsModel::BMM101 => Self {
                chips: None,
                fans: Some(1),
                boards: Some(1),
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum BraiinsControlBoard {
    #[serde(rename = "BraiinsCB")]
    BraiinsCB,
}

impl BraiinsControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        let cb_model = s.trim().replace(" ", "").to_uppercase();
        match cb_model.as_ref() {
            "BRAIINSCB" => Some(Self::BraiinsCB),
            _ => None,
        }
    }
}

impl From<BraiinsControlBoard> for MinerControlBoard {
    fn from(cb: BraiinsControlBoard) -> Self {
        MinerControlBoard::Known(cb.to_string())
    }
}

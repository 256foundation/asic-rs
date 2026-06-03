use asic_rs_core::data::collector::FromValue;
use asic_rs_core::data::{board::MinerControlBoard, device::MinerHardware};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use crate::models::EPicModel;

impl From<EPicModel> for MinerHardware {
    fn from(value: EPicModel) -> Self {
        match value {
            EPicModel::BM520i => Self {
                fans: Some(4),
                boards: Some(vec![Some(124), Some(124), Some(124)]),
            },
            EPicModel::S19JProDual => Self {
                fans: Some(8),
                boards: Some(vec![
                    Some(126),
                    Some(126),
                    Some(126),
                    Some(126),
                    Some(126),
                    Some(126),
                ]),
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
        let cb_model = s.trim().replace(' ', "").to_uppercase();
        match cb_model.as_ref() {
            "EPICUMC" | "UMC" => Some(Self::EPicUMC),
            _ => None,
        }
    }
}

impl FromValue for EPicControlBoard {
    fn from_value(value: &Value) -> Option<Self> {
        Self::parse(value.as_str()?)
    }
}

impl From<EPicControlBoard> for MinerControlBoard {
    fn from(cb: EPicControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

use asic_rs_core::data::{board::MinerControlBoard, collector::FromValue, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::models::SealMinerModel;

impl From<SealMinerModel> for MinerHardware {
    fn from(value: SealMinerModel) -> Self {
        match &value {
            SealMinerModel::A2 => MinerHardware {
                fans: Some(4),
                boards: Some(vec![Some(153), Some(153), Some(153)]),
            },
            SealMinerModel::Unknown(_) => Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum SealMinerControlBoard {
    #[serde(rename = "TaurusAir")]
    TaurusAir,
}

impl SealMinerControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().replace(['-', '_', ' ', '.'], "").as_str() {
            s if s.starts_with("taurusair") => Some(Self::TaurusAir),
            _ => None,
        }
    }
}

impl FromValue for SealMinerControlBoard {
    fn from_value(value: &serde_json::Value) -> Option<Self> {
        Self::parse(value.as_str()?)
    }
}

impl From<SealMinerControlBoard> for MinerControlBoard {
    fn from(cb: SealMinerControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

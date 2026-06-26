use asic_rs_core::data::{board::MinerControlBoard, collector::FromValue, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::models::VolcMinerModel;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum VolcMinerControlBoard {
    #[serde(rename = "TVXilinx")]
    TVXilinx,
}

impl VolcMinerControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        let normalized = s.trim().to_ascii_lowercase();
        if normalized.contains("xilinx") {
            Some(Self::TVXilinx)
        } else {
            None
        }
    }
}

impl FromValue for VolcMinerControlBoard {
    fn from_value(value: &serde_json::Value) -> Option<Self> {
        Self::parse(value.as_str()?)
    }
}

impl From<VolcMinerControlBoard> for MinerControlBoard {
    fn from(cb: VolcMinerControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

impl From<VolcMinerModel> for MinerHardware {
    fn from(value: VolcMinerModel) -> Self {
        match value {
            VolcMinerModel::D1 => Self {
                fans: Some(4),
                boards: None,
            },
            VolcMinerModel::Unknown(_) => Default::default(),
        }
    }
}

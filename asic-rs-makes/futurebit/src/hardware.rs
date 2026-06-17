use asic_rs_core::data::{board::MinerControlBoard, collector::FromValue, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::models::FutureBitModel;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum FutureBitControlBoard {
    Apollo,
}

impl FutureBitControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        let normalized = s.trim().replace([' ', '_'], "-").to_lowercase();
        if normalized.contains("apollo") {
            Some(Self::Apollo)
        } else {
            None
        }
    }
}

impl FromValue for FutureBitControlBoard {
    fn from_value(value: &serde_json::Value) -> Option<Self> {
        Self::parse(value.as_str()?)
    }
}

impl From<FutureBitControlBoard> for MinerControlBoard {
    fn from(cb: FutureBitControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

impl From<FutureBitModel> for MinerHardware {
    fn from(model: FutureBitModel) -> Self {
        match model {
            FutureBitModel::Apollo1 => Self {
                fans: Some(1),
                boards: Some(vec![Some(44)]),
            },
            FutureBitModel::Apollo2 => Self {
                fans: Some(1),
                boards: Some(vec![Some(44)]),
            },
            FutureBitModel::Unknown(_) => Default::default(),
        }
    }
}

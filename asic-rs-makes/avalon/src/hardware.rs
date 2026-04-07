use asic_rs_core::data::{board::MinerControlBoard, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::models::AvalonMinerModel;

impl From<AvalonMinerModel> for MinerHardware {
    fn from(value: AvalonMinerModel) -> Self {
        match value {
            AvalonMinerModel::Avalon721 => Self {
                boards: Some(4),
                chips: Some(18),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon741 => Self {
                boards: Some(4),
                chips: Some(22),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon761 => Self {
                boards: Some(4),
                chips: Some(18),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon821 => Self {
                boards: Some(4),
                chips: Some(26),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon841 => Self {
                boards: Some(4),
                chips: Some(26),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon851 => Self {
                boards: Some(4),
                chips: Some(26),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon921 => Self {
                boards: Some(4),
                chips: Some(26),
                fans: Some(1),
            },
            AvalonMinerModel::Avalon1026 => Self {
                boards: Some(3),
                chips: Some(80),
                fans: Some(2),
            },
            AvalonMinerModel::Avalon1047 => Self {
                boards: Some(3),
                chips: Some(80),
                fans: Some(2),
            },
            AvalonMinerModel::Avalon1066 => Self {
                boards: Some(3),
                chips: Some(114),
                fans: Some(4),
            },
            AvalonMinerModel::Avalon1126Pro => Self {
                boards: Some(3),
                chips: Some(120),
                fans: Some(4),
            },
            AvalonMinerModel::Avalon1166Pro => Self {
                boards: Some(3),
                chips: Some(120),
                fans: Some(4),
            },
            AvalonMinerModel::Avalon1246 => Self {
                boards: Some(3),
                chips: Some(120),
                fans: Some(4),
            },
            AvalonMinerModel::Avalon1466 => Self {
                boards: Some(3),
                chips: None,
                fans: Some(2),
            },
            AvalonMinerModel::Avalon1566 => Self {
                boards: Some(3),
                chips: Some(160),
                fans: Some(2),
            },
            AvalonMinerModel::AvalonNano3 => Self {
                boards: Some(1),
                chips: Some(10),
                fans: Some(1),
            },
            AvalonMinerModel::AvalonNano3s => Self {
                boards: Some(1),
                chips: Some(12),
                fans: Some(1),
            },
            AvalonMinerModel::AvalonHomeQ => Self {
                boards: Some(1),
                chips: Some(160),
                fans: Some(4),
            },
            AvalonMinerModel::Unknown(_) => Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum AvalonMinerControlBoard {
    #[serde(rename = "MM3v2X3")]
    MM3v2X3,
    #[serde(rename = "MM3v1X3")]
    MM3v1X3,
    #[serde(rename = "MM3v1")]
    MM3v1,
    #[serde(rename = "MM4v1X3")]
    MM4v1X3,
}

impl AvalonMinerControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        let cb_model = s.trim().replace(" ", "").to_uppercase();
        match cb_model.as_ref() {
            "MM3V2_X3" => Some(Self::MM3v2X3),
            "MM3V1_X3" => Some(Self::MM3v1X3),
            "MM3V1" => Some(Self::MM3v1),
            "MM4V1_X3" => Some(Self::MM4v1X3),
            _ => None,
        }
    }
}

impl From<AvalonMinerControlBoard> for MinerControlBoard {
    fn from(cb: AvalonMinerControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

use asic_rs_core::data::{board::MinerControlBoard, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::models::AntMinerModel;

impl From<AntMinerModel> for MinerHardware {
    fn from(value: AntMinerModel) -> Self {
        match &value {
            AntMinerModel::D3 => Self {
                chips: Some(60),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::HS3 => Self {
                chips: Some(92),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::L3Plus => Self {
                chips: Some(72),
                fans: Some(2),
                boards: Some(4),
            },
            AntMinerModel::KA3 => Self {
                chips: Some(92),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::KS3 => Self {
                chips: Some(92),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::DR5 => Self {
                chips: Some(72),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::KS5 => Self {
                chips: Some(92),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::KS5Pro => Self {
                chips: Some(92),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::L7 => Self {
                chips: Some(120),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::K7 => Self {
                chips: Some(92),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::D7 => Self {
                chips: Some(70),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::E9Pro => Self {
                chips: Some(8),
                fans: Some(4),
                boards: Some(2),
            },
            AntMinerModel::D9 => Self {
                chips: Some(126),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S9 => Self {
                chips: Some(63),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::S9i => Self {
                chips: Some(63),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::S9j => Self {
                chips: Some(63),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::T9 => Self {
                chips: Some(54),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::L9 => Self {
                chips: Some(110),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::Z15 => Self {
                chips: Some(3),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::Z15Pro => Self {
                chips: Some(6),
                fans: Some(2),
                boards: Some(3),
            },
            AntMinerModel::S17 => Self {
                chips: Some(48),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S17Plus => Self {
                chips: Some(65),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S17Pro => Self {
                chips: Some(48),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S17e => Self {
                chips: Some(135),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::T17 => Self {
                chips: Some(30),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::T17Plus => Self {
                chips: Some(44),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::T17e => Self {
                chips: Some(78),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19 => Self {
                chips: Some(76),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19L => Self {
                chips: Some(76),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19Pro => Self {
                chips: Some(114),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19j => Self {
                chips: Some(114),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19i => Self {
                chips: Some(80),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19Plus => Self {
                chips: Some(80),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19jNoPIC => Self {
                chips: Some(88),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19ProPlus => Self {
                chips: Some(120),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19jPro => Self {
                chips: Some(126),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19jProPlus => Self {
                chips: Some(120),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19XP => Self {
                chips: Some(110),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19a => Self {
                chips: Some(72),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19aPro => Self {
                chips: Some(100),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19Hydro => Self {
                chips: Some(104),
                fans: Some(0),
                boards: Some(4),
            },
            AntMinerModel::S19ProHydro => Self {
                chips: Some(180),
                fans: Some(0),
                boards: Some(4),
            },
            AntMinerModel::S19ProPlusHydro => Self {
                chips: Some(180),
                fans: Some(0),
                boards: Some(4),
            },
            AntMinerModel::S19KPro => Self {
                chips: Some(77),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S19jXP => Self {
                chips: Some(110),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::T19 => Self {
                chips: Some(76),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S21 => Self {
                chips: Some(108),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S21Plus => Self {
                chips: Some(55),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S21PlusHydro => Self {
                chips: Some(95),
                fans: Some(0),
                boards: Some(3),
            },
            AntMinerModel::S21Pro => Self {
                chips: Some(65),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S21XP => Self {
                chips: Some(91),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::T21 => Self {
                chips: Some(108),
                fans: Some(4),
                boards: Some(3),
            },
            AntMinerModel::S21Hydro => Self {
                chips: Some(216),
                fans: Some(0),
                boards: Some(3),
            },
            AntMinerModel::S21eXPHydro => Self {
                chips: Some(160),
                fans: Some(0),
                boards: Some(3),
            },
            AntMinerModel::Unknown(_) => Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum AntMinerControlBoard {
    #[serde(rename = "Xilinx")]
    Xilinx,
    #[serde(rename = "BeagleBoneBlack")]
    BeagleBoneBlack,
    #[serde(rename = "AMLogic")]
    AMLogic,
    #[serde(rename = "CVITek")]
    CVITek,
}

impl AntMinerControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        let cb_model = s.trim().to_uppercase();
        let compact = cb_model
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect::<String>();

        match compact.as_str() {
            "XILINX" => Some(Self::Xilinx),
            "BBB" | "BBCTRL" | "BB" | "BEAGLEBONE" | "BEAGLEBONEBLACK" => {
                Some(Self::BeagleBoneBlack)
            }
            "CVITEK" | "CVCTRL" => Some(Self::CVITek),
            "AMLOGIC" | "AML" => Some(Self::AMLogic),
            "AMCB07" => Some(Self::Xilinx), // Mara FW
            "ZYNQ7007" => Some(Self::Xilinx),
            _ => None,
        }
    }
}

impl From<AntMinerControlBoard> for MinerControlBoard {
    fn from(cb: AntMinerControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

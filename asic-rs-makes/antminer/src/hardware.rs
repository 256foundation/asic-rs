use asic_rs_core::data::collector::FromValue;
use asic_rs_core::data::{board::MinerControlBoard, device::MinerHardware};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use crate::models::AntMinerModel;

impl From<AntMinerModel> for MinerHardware {
    fn from(value: AntMinerModel) -> Self {
        match &value {
            AntMinerModel::D3 => Self {
                fans: Some(4),
                boards: Some(vec![Some(60), Some(60), Some(60)]),
            },
            AntMinerModel::HS3 => Self {
                fans: Some(4),
                boards: Some(vec![Some(92), Some(92), Some(92)]),
            },
            AntMinerModel::L3Plus => Self {
                fans: Some(2),
                boards: Some(vec![Some(72), Some(72), Some(72), Some(72)]),
            },
            AntMinerModel::KA3 => Self {
                fans: Some(4),
                boards: Some(vec![Some(92), Some(92), Some(92)]),
            },
            AntMinerModel::KS3 => Self {
                fans: Some(2),
                boards: Some(vec![Some(92), Some(92), Some(92)]),
            },
            AntMinerModel::DR5 => Self {
                fans: Some(2),
                boards: Some(vec![Some(72), Some(72), Some(72)]),
            },
            AntMinerModel::KS5 => Self {
                fans: Some(4),
                boards: Some(vec![Some(92), Some(92), Some(92)]),
            },
            AntMinerModel::KS5Pro => Self {
                fans: Some(4),
                boards: Some(vec![Some(92), Some(92), Some(92)]),
            },
            AntMinerModel::L7 => Self {
                fans: Some(4),
                boards: Some(vec![Some(120), Some(120), Some(120)]),
            },
            AntMinerModel::K7 => Self {
                fans: Some(2),
                boards: Some(vec![Some(92), Some(92), Some(92)]),
            },
            AntMinerModel::D7 => Self {
                fans: Some(4),
                boards: Some(vec![Some(70), Some(70), Some(70)]),
            },
            AntMinerModel::E9Pro => Self {
                fans: Some(4),
                boards: Some(vec![Some(8), Some(8)]),
            },
            AntMinerModel::D9 => Self {
                fans: Some(4),
                boards: Some(vec![Some(126), Some(126), Some(126)]),
            },
            AntMinerModel::S9 => Self {
                fans: Some(2),
                boards: Some(vec![Some(63), Some(63), Some(63)]),
            },
            AntMinerModel::S9i => Self {
                fans: Some(2),
                boards: Some(vec![Some(63), Some(63), Some(63)]),
            },
            AntMinerModel::S9j => Self {
                fans: Some(2),
                boards: Some(vec![Some(63), Some(63), Some(63)]),
            },
            AntMinerModel::T9 => Self {
                fans: Some(2),
                boards: Some(vec![Some(54), Some(54), Some(54)]),
            },
            AntMinerModel::L9 => Self {
                fans: Some(4),
                boards: Some(vec![Some(110), Some(110), Some(110)]),
            },
            AntMinerModel::Z15 => Self {
                fans: Some(2),
                boards: Some(vec![Some(3), Some(3), Some(3)]),
            },
            AntMinerModel::Z15Pro => Self {
                fans: Some(2),
                boards: Some(vec![Some(6), Some(6), Some(6)]),
            },
            AntMinerModel::S17 => Self {
                fans: Some(4),
                boards: Some(vec![Some(48), Some(48), Some(48)]),
            },
            AntMinerModel::S17Plus => Self {
                fans: Some(4),
                boards: Some(vec![Some(65), Some(65), Some(65)]),
            },
            AntMinerModel::S17Pro => Self {
                fans: Some(4),
                boards: Some(vec![Some(48), Some(48), Some(48)]),
            },
            AntMinerModel::S17e => Self {
                fans: Some(4),
                boards: Some(vec![Some(135), Some(135), Some(135)]),
            },
            AntMinerModel::T17 => Self {
                fans: Some(4),
                boards: Some(vec![Some(30), Some(30), Some(30)]),
            },
            AntMinerModel::T17Plus => Self {
                fans: Some(4),
                boards: Some(vec![Some(44), Some(44), Some(44)]),
            },
            AntMinerModel::T17e => Self {
                fans: Some(4),
                boards: Some(vec![Some(78), Some(78), Some(78)]),
            },
            AntMinerModel::S19 => Self {
                fans: Some(4),
                boards: Some(vec![Some(76), Some(76), Some(76)]),
            },
            AntMinerModel::S19L => Self {
                fans: Some(4),
                boards: Some(vec![Some(76), Some(76), Some(76)]),
            },
            AntMinerModel::S19Pro => Self {
                fans: Some(4),
                boards: Some(vec![Some(114), Some(114), Some(114)]),
            },
            AntMinerModel::S19j => Self {
                fans: Some(4),
                boards: Some(vec![Some(114), Some(114), Some(114)]),
            },
            AntMinerModel::S19i => Self {
                fans: Some(4),
                boards: Some(vec![Some(80), Some(80), Some(80)]),
            },
            AntMinerModel::S19Plus => Self {
                fans: Some(4),
                boards: Some(vec![Some(80), Some(80), Some(80)]),
            },
            AntMinerModel::S19jNoPIC => Self {
                fans: Some(4),
                boards: Some(vec![Some(88), Some(88), Some(88)]),
            },
            AntMinerModel::S19ProPlus => Self {
                fans: Some(4),
                boards: Some(vec![Some(120), Some(120), Some(120)]),
            },
            AntMinerModel::S19jPro => Self {
                fans: Some(4),
                boards: Some(vec![Some(126), Some(126), Some(126)]),
            },
            AntMinerModel::S19jProPlus => Self {
                fans: Some(4),
                boards: Some(vec![Some(120), Some(120), Some(120)]),
            },
            AntMinerModel::S19XP => Self {
                fans: Some(4),
                boards: Some(vec![Some(110), Some(110), Some(110)]),
            },
            AntMinerModel::S19a => Self {
                fans: Some(4),
                boards: Some(vec![Some(72), Some(72), Some(72)]),
            },
            AntMinerModel::S19aPro => Self {
                fans: Some(4),
                boards: Some(vec![Some(100), Some(100), Some(100)]),
            },
            AntMinerModel::S19Hydro => Self {
                fans: Some(0),
                boards: Some(vec![Some(104), Some(104), Some(104), Some(104)]),
            },
            AntMinerModel::S19ProHydro => Self {
                fans: Some(0),
                boards: Some(vec![Some(180), Some(180), Some(180), Some(180)]),
            },
            AntMinerModel::S19ProPlusHydro => Self {
                fans: Some(0),
                boards: Some(vec![Some(180), Some(180), Some(180), Some(180)]),
            },
            AntMinerModel::S19KPro => Self {
                fans: Some(4),
                boards: Some(vec![Some(77), Some(77), Some(77)]),
            },
            AntMinerModel::S19jXP => Self {
                fans: Some(4),
                boards: Some(vec![Some(110), Some(110), Some(110)]),
            },
            AntMinerModel::T19 => Self {
                fans: Some(4),
                boards: Some(vec![Some(76), Some(76), Some(76)]),
            },
            AntMinerModel::S21 => Self {
                fans: Some(4),
                boards: Some(vec![Some(108), Some(108), Some(108)]),
            },
            AntMinerModel::S21Plus => Self {
                fans: Some(4),
                boards: Some(vec![Some(55), Some(55), Some(55)]),
            },
            AntMinerModel::S21PlusHydro => Self {
                fans: Some(0),
                boards: Some(vec![Some(95), Some(95), Some(95)]),
            },
            AntMinerModel::S21Pro => Self {
                fans: Some(4),
                boards: Some(vec![Some(65), Some(65), Some(65)]),
            },
            AntMinerModel::S21ProPlus => Self {
                fans: Some(4),
                boards: Some(vec![Some(65), Some(65), Some(65)]),
            },
            AntMinerModel::S21XP => Self {
                fans: Some(4),
                boards: Some(vec![Some(91), Some(91), Some(91)]),
            },
            AntMinerModel::T21 => Self {
                fans: Some(4),
                boards: Some(vec![Some(108), Some(108), Some(108)]),
            },
            AntMinerModel::S21Hydro => Self {
                fans: Some(0),
                boards: Some(vec![Some(216), Some(216), Some(216)]),
            },
            AntMinerModel::S21eXPHydro => Self {
                fans: Some(0),
                boards: Some(vec![Some(160), Some(160), Some(160)]),
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

impl FromValue for AntMinerControlBoard {
    fn from_value(value: &Value) -> Option<Self> {
        Self::parse(value.as_str()?)
    }
}

impl From<AntMinerControlBoard> for MinerControlBoard {
    fn from(cb: AntMinerControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

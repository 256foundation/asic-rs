use asic_rs_core::data::{board::MinerControlBoard, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::models::NerdAxeModel;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum NerdAxeControlBoard {
    #[serde(rename = "B102")]
    B102,
    #[serde(rename = "B201")]
    B201,
    #[serde(rename = "B202")]
    B202,
    #[serde(rename = "B203")]
    B203,
    #[serde(rename = "B204")]
    B204,
    #[serde(rename = "B205")]
    B205,
    #[serde(rename = "B207")]
    B207,
    #[serde(rename = "B401")]
    B401,
    #[serde(rename = "B402")]
    B402,
    #[serde(rename = "B403")]
    B403,
    #[serde(rename = "B601")]
    B601,
    #[serde(rename = "B602")]
    B602,
    #[serde(rename = "B800")]
    B800,
}

impl NerdAxeControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        let cb_model = s.trim().replace(" ", "").to_uppercase();
        match cb_model.as_ref() {
            "102" => Some(Self::B102),
            "201" => Some(Self::B201),
            "202" => Some(Self::B202),
            "203" => Some(Self::B203),
            "204" => Some(Self::B204),
            "205" => Some(Self::B205),
            "207" => Some(Self::B207),
            "401" => Some(Self::B401),
            "402" => Some(Self::B402),
            "403" => Some(Self::B403),
            "601" => Some(Self::B601),
            "602" => Some(Self::B602),
            "800" => Some(Self::B800),
            _ => None,
        }
    }
}

impl From<NerdAxeControlBoard> for MinerControlBoard {
    fn from(cb: NerdAxeControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

impl From<NerdAxeModel> for MinerHardware {
    fn from(model: NerdAxeModel) -> Self {
        match model {
            NerdAxeModel::NerdAxe => Self {
                chips: Some(1),
                fans: Some(1),
                boards: Some(1),
            },
            NerdAxeModel::NerdQAxe => Self {
                chips: Some(4),
                fans: Some(1),
                boards: Some(1),
            },
            NerdAxeModel::NerdMiner => Self {
                chips: Some(1),
                fans: Some(1),
                boards: Some(1),
            },
            NerdAxeModel::NerdAxeUltra => Self {
                chips: Some(1),
                fans: Some(1),
                boards: Some(1),
            },
            NerdAxeModel::Unknown(_) => Default::default(),
        }
    }
}

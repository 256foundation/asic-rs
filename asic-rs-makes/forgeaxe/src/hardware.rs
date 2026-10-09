use asic_rs_core::data::{board::MinerControlBoard, collector::FromValue, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;
use ts_rs::TS;

use crate::models::ForgeaxeModel;

impl From<ForgeaxeModel> for MinerHardware {
    fn from(model: ForgeaxeModel) -> Self {
        match model {
            // ForgeOS defines BITFORGE_NANO_ASIC_COUNT as 2, including in v1.1:
            // https://github.com/WantClue/forge-os/blob/v1.1/components/asic/include/asic.h
            ForgeaxeModel::BitForgeNano => Self {
                fans: Some(1),
                boards: Some(vec![Some(2)]),
            },
            ForgeaxeModel::Unknown(_) => Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display, TS)]
pub enum ForgeaxeControlBoard {
    #[serde(rename = "B800")]
    B800,
}

impl ForgeaxeControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().replace(' ', "").to_uppercase().as_str() {
            "800" => Some(Self::B800),
            _ => None,
        }
    }
}

impl FromValue for ForgeaxeControlBoard {
    fn from_value(value: &serde_json::Value) -> Option<Self> {
        Self::parse(value.as_str()?)
    }
}

impl From<ForgeaxeControlBoard> for MinerControlBoard {
    fn from(cb: ForgeaxeControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nano_hardware_matches_firmware() {
        let hardware = MinerHardware::from(ForgeaxeModel::BitForgeNano);
        assert_eq!(hardware.fans, Some(1));
        assert_eq!(hardware.board_count(), Some(1));
        assert_eq!(hardware.chips_for_board(0), Some(2));
        assert_eq!(hardware.total_chips(), Some(2));
        assert_eq!(
            MinerHardware::from(ForgeaxeModel::Unknown("unknown".into())),
            MinerHardware::default()
        );
    }

    #[test]
    fn nano_control_board_parses() {
        assert_eq!(
            ForgeaxeControlBoard::parse("800"),
            Some(ForgeaxeControlBoard::B800)
        );
        assert_eq!(ForgeaxeControlBoard::parse("601"), None);
        let board = ForgeaxeControlBoard::from_value(&serde_json::json!("800"));
        assert_eq!(
            board.map(MinerControlBoard::from).map(|cb| cb.to_string()),
            Some("B800".into())
        );
    }
}

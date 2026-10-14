use asic_rs_core::data::{board::MinerControlBoard, collector::FromValue, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;
use ts_rs::TS;

use crate::models::SealMinerModel;

impl From<SealMinerModel> for MinerHardware {
    fn from(value: SealMinerModel) -> Self {
        match &value {
            SealMinerModel::A2 => MinerHardware {
                fans: Some(4),
                boards: Some(vec![Some(153), Some(153), Some(153)]),
            },
            SealMinerModel::A3ProHyd => MinerHardware {
                fans: Some(0),
                boards: Some(vec![Some(396), Some(396)]),
            },
            SealMinerModel::Unknown(_) => Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display, TS)]
pub enum SealMinerControlBoard {
    #[serde(rename = "TaurusAir")]
    TaurusAir,
    #[serde(rename = "TaurusHydro")]
    TaurusHydro,
}

impl SealMinerControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().replace(['-', '_', ' ', '.'], "").as_str() {
            s if s.starts_with("taurusair") => Some(Self::TaurusAir),
            s if s.starts_with("taurushydro") => Some(Self::TaurusHydro),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydro_hardware_metadata() {
        let hardware = MinerHardware::from(SealMinerModel::A3ProHyd);
        assert_eq!(hardware.fans, Some(0));
        assert_eq!(hardware.boards, Some(vec![Some(396), Some(396)]));
        assert_eq!(hardware.board_count(), Some(2));
        assert_eq!(hardware.total_chips(), Some(792));
    }

    #[test]
    fn a2_hardware_is_unchanged() {
        let hardware = MinerHardware::from(SealMinerModel::A2);
        assert_eq!(hardware.fans, Some(4));
        assert_eq!(hardware.boards, Some(vec![Some(153), Some(153), Some(153)]));
    }

    #[test]
    fn unknown_hardware_stays_unknown() {
        let hardware = MinerHardware::from(SealMinerModel::Unknown("Unrecognized".to_string()));
        assert_eq!(hardware.fans, None);
        assert_eq!(hardware.boards, None);
        assert_eq!(hardware.board_count(), None);
        assert_eq!(hardware.total_chips(), None);
    }

    #[test]
    fn hydro_control_board_parses_name_and_version() {
        for name in ["TaurusHydro", "taurus_hydro_v1.2", "Taurus-Hydro v1.2"] {
            assert_eq!(
                SealMinerControlBoard::parse(name),
                Some(SealMinerControlBoard::TaurusHydro)
            );
        }
        assert_eq!(
            SealMinerControlBoard::from_value(&serde_json::json!("taurus_hydro_v1.2")),
            Some(SealMinerControlBoard::TaurusHydro)
        );
        assert_eq!(
            MinerControlBoard::from(SealMinerControlBoard::TaurusHydro),
            MinerControlBoard::known("TaurusHydro".to_string())
        );
    }

    #[test]
    fn existing_control_board_handling_is_unchanged() {
        assert_eq!(
            SealMinerControlBoard::parse("taurus_air_v1.2"),
            Some(SealMinerControlBoard::TaurusAir)
        );
        assert_eq!(SealMinerControlBoard::parse("unrecognized"), None);
        assert_eq!(
            SealMinerControlBoard::from_value(&serde_json::Value::Null),
            None
        );
    }
}

use asic_rs_core::data::{board::MinerControlBoard, collector::FromValue, device::MinerHardware};
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::models::ProtoModel;

impl From<ProtoModel> for MinerHardware {
    fn from(model: ProtoModel) -> Self {
        match model {
            // A Proto rig is a heterogeneous, hot-swappable chassis: board
            // count, chips per board, and fan count all vary by configuration.
            // The shape can't be hardcoded per model, so it is discovered from
            // the device at build time (see `ProtoV1::refresh_hardware`).
            ProtoModel::Rig | ProtoModel::Unknown(_) => Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rig_hardware_is_discovered_not_hardcoded() {
        // Counts are unknown until the rig is queried, so the static mapping
        // intentionally leaves them empty.
        let hardware = MinerHardware::from(ProtoModel::Rig);
        assert_eq!(hardware.chips, None);
        assert_eq!(hardware.fans, None);
        assert_eq!(hardware.boards, None);
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum ProtoControlBoard {
    #[serde(rename = "C1")]
    C1,
    #[serde(rename = "C2")]
    C2,
    #[serde(rename = "C3")]
    C3,
}

impl ProtoControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "C1" => Some(Self::C1),
            "C2" => Some(Self::C2),
            "C3" => Some(Self::C3),
            _ => None,
        }
    }
}

impl FromValue for ProtoControlBoard {
    fn from_value(value: &serde_json::Value) -> Option<Self> {
        Self::parse(value.as_str()?)
    }
}

impl From<ProtoControlBoard> for MinerControlBoard {
    fn from(cb: ProtoControlBoard) -> Self {
        MinerControlBoard::known(cb.to_string())
    }
}

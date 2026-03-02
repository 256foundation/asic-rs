use asic_rs_core::data::board::MinerControlBoard;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum MarathonControlBoard {
    #[serde(rename = "MaraCB")]
    MaraCB,
}

impl MarathonControlBoard {
    pub fn parse(s: &str) -> Option<Self> {
        let cb_model = s.trim().replace(" ", "").to_uppercase();
        match &cb_model {
            s if s.starts_with("MARACB") => Some(Self::MaraCB),
            _ => None,
        }
    }
}

impl From<MarathonControlBoard> for MinerControlBoard {
    fn from(cb: MarathonControlBoard) -> Self {
        MinerControlBoard::Known(cb.to_string())
    }
}

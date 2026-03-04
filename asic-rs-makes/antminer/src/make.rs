use crate::hardware::AntMinerControlBoard;
use crate::models::AntMinerModel;
use asic_rs_core::data::board::MinerControlBoard;
use asic_rs_core::errors::ModelSelectionError;
use asic_rs_core::traits::make::MinerMake;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Default)]
pub struct AntMinerMake {}

impl Display for AntMinerMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Antminer")
    }
}

impl MinerMake for AntMinerMake {
    type Model = AntMinerModel;
    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        AntMinerModel::from_str(&model)
    }

    fn parse_control_board(&self, cb_type: &str) -> Option<MinerControlBoard> {
        Some(AntMinerControlBoard::parse(cb_type)?.into())
    }
}

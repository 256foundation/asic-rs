use std::{fmt::Display, str::FromStr};

use asic_rs_core::{
    data::board::MinerControlBoard, errors::ModelSelectionError, traits::make::MinerMake,
};

use crate::{hardware::AntMinerControlBoard, models::AntMinerModel};

#[derive(Default)]
pub struct AntMinerMake {}

impl Display for AntMinerMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AntMiner")
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

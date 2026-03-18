use std::{fmt::Display, str::FromStr};

use asic_rs_core::{
    data::board::MinerControlBoard, errors::ModelSelectionError, traits::make::MinerMake,
};

use crate::{hardware::SealMinerControlBoard, models::SealMinerModel};

#[derive(Default)]
pub struct SealMinerMake {}

impl Display for SealMinerMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SealMiner")
    }
}

impl MinerMake for SealMinerMake {
    type Model = SealMinerModel;
    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        SealMinerModel::from_str(&model)
    }

    fn parse_control_board(&self, cb_type: &str) -> Option<MinerControlBoard> {
        Some(SealMinerControlBoard::parse(cb_type)?.into())
    }
}

use std::{fmt::Display, str::FromStr};

use asic_rs_core::{
    data::board::MinerControlBoard, errors::ModelSelectionError, traits::make::MinerMake,
};

use crate::models::IceRiverModel;

#[derive(Default)]
pub struct IceRiverMake;

impl Display for IceRiverMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IceRiver")
    }
}

impl MinerMake for IceRiverMake {
    type Model = IceRiverModel;

    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        IceRiverModel::from_str(&model)
    }

    fn parse_control_board(&self, _cb_type: &str) -> Option<MinerControlBoard> {
        None
    }
}

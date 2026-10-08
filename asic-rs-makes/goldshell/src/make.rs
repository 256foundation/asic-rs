use std::{fmt::Display, str::FromStr};

use asic_rs_core::{
    data::board::MinerControlBoard, errors::ModelSelectionError, traits::make::MinerMake,
};

use crate::models::GoldshellModel;

#[derive(Default, Debug)]
pub struct GoldshellMake;

impl Display for GoldshellMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Goldshell")
    }
}

impl MinerMake for GoldshellMake {
    type Model = GoldshellModel;

    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        GoldshellModel::from_str(&model)
    }

    fn parse_control_board(&self, _cb_type: &str) -> Option<MinerControlBoard> {
        None
    }
}

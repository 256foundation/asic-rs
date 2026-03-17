use std::{fmt::Display, str::FromStr};

use asic_rs_core::{errors::ModelSelectionError, traits::make::MinerMake};

use crate::models::BraiinsModel;

#[derive(Default)]
pub struct BraiinsMake {}

impl Display for BraiinsMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Braiins")
    }
}

impl MinerMake for BraiinsMake {
    type Model = BraiinsModel;
    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        BraiinsModel::from_str(&model)
    }
}

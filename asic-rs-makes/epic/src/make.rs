use std::{fmt::Display, str::FromStr};

use asic_rs_core::{errors::ModelSelectionError, traits::make::MinerMake};

use crate::models::EPicModel;

#[derive(Default)]
pub struct EPicMake {}

impl Display for EPicMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ePIC")
    }
}

impl MinerMake for EPicMake {
    type Model = EPicModel;
    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        EPicModel::from_str(&model)
    }
}

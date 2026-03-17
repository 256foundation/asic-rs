use std::{fmt::Display, str::FromStr};

use asic_rs_core::{errors::ModelSelectionError, traits::make::MinerMake};

use crate::models::NerdAxeModel;

#[derive(Default)]
pub struct NerdAxeMake {}

impl Display for NerdAxeMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Nerdaxe")
    }
}

impl MinerMake for NerdAxeMake {
    type Model = NerdAxeModel;
    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        NerdAxeModel::from_str(&model)
    }
}

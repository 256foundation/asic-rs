use crate::models::NerdAxeModel;
use asic_rs_core::errors::ModelSelectionError;
use asic_rs_core::traits::make::MinerMake;
use std::fmt::Display;
use std::str::FromStr;

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

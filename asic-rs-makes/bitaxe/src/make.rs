use crate::models::BitaxeModel;
use asic_rs_core::errors::ModelSelectionError;
use asic_rs_core::traits::make::MinerMake;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Default)]
pub struct BitaxeMake {}

impl Display for BitaxeMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bitaxe")
    }
}

impl MinerMake for BitaxeMake {
    type Model = BitaxeModel;
    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        BitaxeModel::from_str(&model)
    }
}

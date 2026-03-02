use crate::models::AvalonMinerModel;
use asic_rs_core::errors::ModelSelectionError;
use asic_rs_core::traits::make::MinerMake;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Default)]
pub struct AvalonMinerMake {}

impl Display for AvalonMinerMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AvalonMiner")
    }
}

impl MinerMake for AvalonMinerMake {
    type Model = AvalonMinerModel;
    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        AvalonMinerModel::from_str(&model)
    }
}

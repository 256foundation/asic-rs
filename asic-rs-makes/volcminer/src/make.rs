use std::{fmt::Display, str::FromStr};

use asic_rs_core::{errors::ModelSelectionError, traits::make::MinerMake};

use crate::models::VolcMinerModel;

#[derive(Default)]
pub struct VolcMinerMake {}

impl Display for VolcMinerMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VolcMiner")
    }
}

impl MinerMake for VolcMinerMake {
    type Model = VolcMinerModel;

    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        VolcMinerModel::from_str(&model.trim().to_uppercase())
    }
}

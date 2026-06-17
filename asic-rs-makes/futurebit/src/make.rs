use std::{fmt::Display, str::FromStr};

use asic_rs_core::{errors::ModelSelectionError, traits::make::MinerMake};

use crate::models::FutureBitModel;

#[derive(Default)]
pub struct FutureBitMake {}

impl Display for FutureBitMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FutureBit")
    }
}

impl MinerMake for FutureBitMake {
    type Model = FutureBitModel;

    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        FutureBitModel::from_str(&model)
    }
}

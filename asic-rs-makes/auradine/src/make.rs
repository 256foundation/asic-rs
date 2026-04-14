use std::{fmt::Display, str::FromStr};

use asic_rs_core::{errors::ModelSelectionError, traits::make::MinerMake};

use crate::models::AuradineModel;

#[derive(Default)]
pub struct AuradineMake {}

impl Display for AuradineMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Auradine")
    }
}

impl MinerMake for AuradineMake {
    type Model = AuradineModel;

    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        AuradineModel::from_str(&model)
    }
}

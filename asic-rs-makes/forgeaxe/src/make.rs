use std::{fmt::Display, str::FromStr};

use asic_rs_core::{errors::ModelSelectionError, traits::make::MinerMake};

use crate::models::ForgeaxeModel;

#[derive(Default)]
pub struct ForgeaxeMake {}

impl Display for ForgeaxeMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Forgeaxe")
    }
}

impl MinerMake for ForgeaxeMake {
    type Model = ForgeaxeModel;

    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        ForgeaxeModel::from_str(&model)
    }
}

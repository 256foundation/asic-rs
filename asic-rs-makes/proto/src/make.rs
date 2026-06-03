use std::{fmt::Display, str::FromStr};

use asic_rs_core::{errors::ModelSelectionError, traits::make::MinerMake};

use crate::models::ProtoModel;

#[derive(Default)]
pub struct ProtoMake {}

impl Display for ProtoMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Proto")
    }
}

impl MinerMake for ProtoMake {
    type Model = ProtoModel;

    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        ProtoModel::from_str(&model)
    }
}

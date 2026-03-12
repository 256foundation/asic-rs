use std::{fmt::Display, str::FromStr};

use asic_rs_core::{errors::ModelSelectionError, traits::make::MinerMake};

use crate::models::WhatsMinerModel;

#[derive(Default)]
pub struct WhatsMinerMake {}

impl Display for WhatsMinerMake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WhatsMiner")
    }
}

impl MinerMake for WhatsMinerMake {
    type Model = WhatsMinerModel;
    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError> {
        WhatsMinerModel::from_str(&model)
    }
}

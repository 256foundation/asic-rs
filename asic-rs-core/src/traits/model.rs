use std::{
    any::Any,
    fmt::{Display, Formatter},
    str::FromStr,
};

use crate::{data::device::MinerHardware, errors::ModelSelectionError};

pub trait MinerModel: Display + Into<MinerHardware> + Clone + Any {
    fn make_name(&self) -> String;
}

#[derive(Debug, Clone)]
pub struct UnknownMinerModel {
    pub name: String,
}

impl Display for UnknownMinerModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown: {}", self.name)
    }
}

impl From<UnknownMinerModel> for MinerHardware {
    fn from(_: UnknownMinerModel) -> Self {
        Default::default()
    }
}

impl MinerModel for UnknownMinerModel {
    fn make_name(&self) -> String {
        "Unknown".to_string()
    }
}

impl FromStr for UnknownMinerModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            name: s.to_string(),
        })
    }
}

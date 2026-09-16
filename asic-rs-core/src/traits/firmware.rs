use std::net::IpAddr;

use async_trait::async_trait;

use crate::{
    errors::ModelSelectionError,
    traits::{discovery::DiscoveryCommands, model::MinerModel},
};

#[async_trait]
pub trait MinerFirmware: ToString + DiscoveryCommands {
    type Model: MinerModel + Send;

    async fn get_model(ip: IpAddr) -> Result<Self::Model, ModelSelectionError>;
    async fn get_version(ip: IpAddr) -> Option<semver::Version>;
}

use std::net::IpAddr;

use async_trait::async_trait;

use crate::{
    errors::ModelSelectionError,
    traits::{discovery::DiscoveryCommands, model::MinerModel},
};

#[expect(
    clippy::double_must_use,
    reason = "async_trait adds must_use to methods returning must-use futures"
)]
#[async_trait]
pub trait MinerFirmware: ToString + DiscoveryCommands {
    type Model: MinerModel + Send;

    async fn get_model(ip: IpAddr) -> Result<Self::Model, ModelSelectionError>;
    async fn get_version(ip: IpAddr) -> Option<semver::Version>;
}

use std::net::IpAddr;

use async_trait::async_trait;

use crate::{
    errors::ModelSelectionError,
    traits::{discovery::DiscoveryCommands, model::MinerModel},
};

#[async_trait]
pub trait MinerFirmware: ToString + DiscoveryCommands {
    type Model: MinerModel + Send;

    #[allow(clippy::double_must_use)] // async_trait adds #[must_use] to a must-use future.
    async fn get_model(ip: IpAddr) -> Result<Self::Model, ModelSelectionError>;
    #[allow(clippy::double_must_use)] // async_trait adds #[must_use] to a must-use future.
    async fn get_version(ip: IpAddr) -> Option<semver::Version>;
}

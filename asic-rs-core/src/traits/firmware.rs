use crate::errors::ModelSelectionError;
use crate::traits::discovery::DiscoveryCommands;
use crate::traits::model::MinerModel;
use async_trait::async_trait;
use std::net::IpAddr;

#[async_trait]
pub trait MinerFirmware: ToString + DiscoveryCommands {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError>;
    async fn get_version(ip: IpAddr) -> Option<semver::Version>;
}

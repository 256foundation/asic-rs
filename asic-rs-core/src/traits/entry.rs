use std::net::IpAddr;

use async_trait::async_trait;

use crate::{
    errors::ModelSelectionError,
    traits::{discovery::DiscoveryCommands, identification::FirmwareIdentification, miner::Miner},
};

/// Combined trait for firmware registry entries.
///
/// Provides identification logic, discovery commands, and the ability to
/// construct a fully-typed miner instance after identification succeeds.
#[async_trait]
pub trait FirmwareEntry: FirmwareIdentification + DiscoveryCommands + Send + Sync {
    async fn build_miner(&self, ip: IpAddr) -> Result<Box<dyn Miner>, ModelSelectionError>;
}

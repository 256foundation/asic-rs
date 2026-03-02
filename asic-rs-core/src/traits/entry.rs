use crate::errors::ModelSelectionError;
use crate::traits::discovery::DiscoveryCommands;
use crate::traits::identification::FirmwareIdentification;
use crate::traits::miner::Miner;
use async_trait::async_trait;
use std::net::IpAddr;

/// Combined trait for firmware registry entries.
///
/// Provides identification logic, discovery commands, and the ability to
/// construct a fully-typed miner instance after identification succeeds.
#[async_trait]
pub trait FirmwareEntry: FirmwareIdentification + DiscoveryCommands + Send + Sync {
    async fn build_miner(&self, ip: IpAddr) -> Result<Box<dyn Miner>, ModelSelectionError>;
}

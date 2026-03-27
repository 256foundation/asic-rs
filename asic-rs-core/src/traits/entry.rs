use std::fmt::Debug;
use std::net::IpAddr;

use async_trait::async_trait;

use crate::{
    errors::ModelSelectionError,
    traits::{
        auth::MinerAuth, discovery::DiscoveryCommands, identification::FirmwareIdentification,
        miner::Miner,
    },
};

/// Combined trait for firmware registry entries.
///
/// Provides identification logic, discovery commands, and the ability to
/// construct a fully-typed miner instance after identification succeeds.
#[async_trait]
pub trait FirmwareEntry: FirmwareIdentification + DiscoveryCommands + Send + Sync + Debug {
    /// Construct a fully-typed miner instance for the given IP.
    ///
    /// When `auth` is provided, it is used for both discovery (e.g.
    /// AntMiner digest auth) and applied to the miner for runtime
    /// operations. When `None`, backends use their default credentials.
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError>;
}

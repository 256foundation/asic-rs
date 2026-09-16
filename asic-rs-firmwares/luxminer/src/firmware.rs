use std::{fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::command::DiscoveryCommand,
    discovery::{HTTP_WEB_ROOT, RPC_VERSION},
    errors::ModelSelectionError,
    traits::{
        discovery::DiscoveryCommands,
        entry::FirmwareEntry,
        firmware::MinerFirmware,
        identification::{FirmwareIdentification, WebResponse},
        make::MinerMake,
        miner::{Miner, MinerAuth, MinerConstructor},
    },
    util,
};
use asic_rs_makes_antminer::make::AntMinerMake;
use async_trait::async_trait;

#[derive(Default, Debug)]
pub struct LuxMinerFirmware {}

impl Display for LuxMinerFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LuxOS")
    }
}

impl DiscoveryCommands for LuxMinerFirmware {
    fn get_discovery_commands(&self) -> Vec<DiscoveryCommand> {
        vec![HTTP_WEB_ROOT, RPC_VERSION]
    }
}

#[async_trait]
impl MinerFirmware for LuxMinerFirmware {
    type Model = asic_rs_makes_antminer::models::AntMinerModel;

    async fn get_model(ip: IpAddr) -> Result<Self::Model, ModelSelectionError> {
        let data = util::send_rpc_command(&ip, "version")
            .await
            .ok_or(ModelSelectionError::NoModelResponse)?;

        let model = data["VERSION"][0]["Type"]
            .as_str()
            .ok_or(ModelSelectionError::UnexpectedModelResponse)?
            .to_uppercase();

        AntMinerMake::parse_model(model)
    }

    async fn get_version(_ip: IpAddr) -> Option<semver::Version> {
        None
    }
}

impl FirmwareIdentification for LuxMinerFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        response.contains("LUXMINER")
    }

    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("Luxor Firmware")
    }
}

#[async_trait]
impl FirmwareEntry for LuxMinerFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = LuxMinerFirmware::get_model(ip).await?;
        let version = LuxMinerFirmware::get_version(ip).await;
        let mut miner = crate::backends::LuxMiner::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, sync::Arc};

    use anyhow::Context;
    use asic_rs_core::test::util::get_miner;

    use super::*;

    #[tokio::test]
    #[ignore = "DESTRUCTIVE: restores stock OS; set MINER_IP"]
    async fn restore_stock_os_live_test_auto_detect() -> anyhow::Result<()> {
        let ip_str = std::env::var("MINER_IP").context("MINER_IP is not set")?;
        let ip =
            IpAddr::from_str(&ip_str).with_context(|| format!("invalid MINER_IP: {ip_str}"))?;

        let miner = get_miner(ip, Arc::new(LuxMinerFirmware::default()))
            .await?
            .context("no miner detected at MINER_IP")?;

        anyhow::ensure!(
            miner.supports_restore_stock_os(),
            "miner does not advertise restore stock OS support"
        );
        let result = miner.restore_stock_os().await?;

        println!("restore stock OS result: {result:?}");
        assert!(result.accepted);

        Ok(())
    }
}

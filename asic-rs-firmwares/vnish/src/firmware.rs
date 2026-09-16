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
        model::MinerModel,
    },
    util,
};
use asic_rs_makes_antminer::make::AntMinerMake;
use async_trait::async_trait;

#[derive(Default, Debug)]
pub struct VnishFirmware {}

impl Display for VnishFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VNish")
    }
}

impl DiscoveryCommands for VnishFirmware {
    fn get_discovery_commands(&self) -> Vec<DiscoveryCommand> {
        vec![HTTP_WEB_ROOT, RPC_VERSION]
    }
}

#[async_trait]
impl MinerFirmware for VnishFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel + Send, ModelSelectionError> {
        let (text, _, _) = util::send_web_command(&ip, "/api/v1/info")
            .await
            .ok_or(ModelSelectionError::NoModelResponse)?;

        let json_data: serde_json::Value = serde_json::from_str(&text)
            .map_err(|_| ModelSelectionError::UnexpectedModelResponse)?;

        let model = json_data["miner"]
            .as_str()
            .ok_or(ModelSelectionError::UnexpectedModelResponse)?
            .to_uppercase();

        AntMinerMake::parse_model(model)
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let (text, _, _) = util::send_web_command(&ip, "/api/v1/info").await?;
        let json_data: serde_json::Value = serde_json::from_str(&text).ok()?;
        let version_str = json_data["fw_version"].as_str()?;

        semver::Version::parse(version_str)
            .ok()
            .or_else(|| semver::Version::parse(&format!("{}.0", version_str)).ok())
    }
}

impl FirmwareIdentification for VnishFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        response.contains("VNISH")
    }

    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("AnthillOS")
    }
}

#[async_trait]
impl FirmwareEntry for VnishFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = VnishFirmware::get_model(ip).await?;
        let version = VnishFirmware::get_version(ip).await;
        let mut miner = crate::backends::Vnish::new(ip, model, version);
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

        let miner = get_miner(ip, Arc::new(VnishFirmware::default()))
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

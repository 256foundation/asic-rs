use asic_rs_core::data::command::MinerCommand;
use asic_rs_core::discovery::{HTTP_WEB_ROOT, RPC_VERSION};
use asic_rs_core::errors::ModelSelectionError;
use asic_rs_core::traits::discovery::DiscoveryCommands;
use asic_rs_core::traits::entry::FirmwareEntry;
use asic_rs_core::traits::firmware::MinerFirmware;
use asic_rs_core::traits::identification::{FirmwareIdentification, WebResponse};
use asic_rs_core::traits::make::MinerMake;
use asic_rs_core::traits::miner::{Miner, MinerConstructor};
use asic_rs_core::traits::model::MinerModel;
use asic_rs_core::util;
use asic_rs_makes_antminer::make::AntMinerMake;
use async_trait::async_trait;
use std::fmt::Display;
use std::net::IpAddr;

#[derive(Default)]
pub struct VnishFirmware {}

impl Display for VnishFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VNish")
    }
}

impl DiscoveryCommands for VnishFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![HTTP_WEB_ROOT, RPC_VERSION]
    }
}

#[async_trait]
impl MinerFirmware for VnishFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
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
    async fn build_miner(&self, ip: IpAddr) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = VnishFirmware::get_model(ip).await?;
        let version = VnishFirmware::get_version(ip).await;
        Ok(crate::backends::Vnish::new(ip, model, version))
    }
}

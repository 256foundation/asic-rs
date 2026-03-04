use asic_rs_core::data::command::MinerCommand;
use asic_rs_core::discovery::HTTP_WEB_ROOT;
use asic_rs_core::errors::ModelSelectionError;
use asic_rs_core::traits::discovery::DiscoveryCommands;
use asic_rs_core::traits::entry::FirmwareEntry;
use asic_rs_core::traits::firmware::MinerFirmware;
use asic_rs_core::traits::identification::{FirmwareIdentification, WebResponse};
use asic_rs_core::traits::make::MinerMake;
use asic_rs_core::traits::miner::{Miner, MinerConstructor};
use asic_rs_core::traits::model::MinerModel;
use asic_rs_core::util;
use asic_rs_makes_nerdaxe::make::NerdAxeMake;
use async_trait::async_trait;
use std::fmt::Display;
use std::net::IpAddr;

#[derive(Default)]
pub struct NerdAxeFirmware {}

impl Display for NerdAxeFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Nerdaxe Stock")
    }
}

impl DiscoveryCommands for NerdAxeFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![HTTP_WEB_ROOT]
    }
}

#[async_trait]
impl MinerFirmware for NerdAxeFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let (text, _, _) = util::send_web_command(&ip, "/api/system/info")
            .await
            .ok_or(ModelSelectionError::NoModelResponse)?;

        let json_data: serde_json::Value = serde_json::from_str(&text)
            .map_err(|_| ModelSelectionError::UnexpectedModelResponse)?;

        let model = json_data["deviceModel"]
            .as_str()
            .ok_or(ModelSelectionError::UnexpectedModelResponse)?
            .to_string();

        NerdAxeMake::parse_model(model)
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let (text, _, _) = util::send_web_command(&ip, "/api/system/info").await?;
        let json_data: serde_json::Value = serde_json::from_str(&text).ok()?;
        let version_str = json_data["version"].as_str()?.trim_start_matches('v');
        semver::Version::parse(version_str).ok()
    }
}

impl FirmwareIdentification for NerdAxeFirmware {
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("Nerd")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for NerdAxeFirmware {
    async fn build_miner(&self, ip: IpAddr) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = NerdAxeFirmware::get_model(ip).await?;
        let version = NerdAxeFirmware::get_version(ip).await;
        Ok(crate::backends::NerdAxe::new(ip, model, version))
    }
}

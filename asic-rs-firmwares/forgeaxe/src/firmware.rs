use std::{fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::command::DiscoveryCommand,
    discovery::HTTP_WEB_ROOT,
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
use asic_rs_makes_forgeaxe::make::ForgeaxeMake;
use async_trait::async_trait;
use semver::Version;

#[derive(Default, Debug)]
pub struct ForgeaxeFirmware {}

impl Display for ForgeaxeFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ForgeOS Stock")
    }
}

impl DiscoveryCommands for ForgeaxeFirmware {
    fn get_discovery_commands(&self) -> Vec<DiscoveryCommand> {
        vec![HTTP_WEB_ROOT]
    }
}

fn parse_version(version: &str) -> Option<Version> {
    let version = version.trim().trim_start_matches('v');
    // ForgeOS release tags omit the patch component (e.g. v1.1).
    Version::parse(version).ok().or_else(|| {
        let (major, minor) = version.split_once('.')?;
        Some(Version::new(major.parse().ok()?, minor.parse().ok()?, 0))
    })
}

#[async_trait]
impl MinerFirmware for ForgeaxeFirmware {
    type Model = asic_rs_makes_forgeaxe::models::ForgeaxeModel;

    async fn get_model(ip: IpAddr) -> Result<Self::Model, ModelSelectionError> {
        let response = util::send_web_command(&ip, "/api/system/info").await;

        match response {
            Some((raw_json, _, _)) => {
                let Ok(json_data) = serde_json::from_str::<serde_json::Value>(&raw_json) else {
                    return Err(ModelSelectionError::UnexpectedModelResponse);
                };

                let Some(model) = json_data["ASICModel"].as_str() else {
                    return Err(ModelSelectionError::UnexpectedModelResponse);
                };
                let model = model.to_uppercase();

                ForgeaxeMake::parse_model(model)
            }
            None => Err(ModelSelectionError::NoModelResponse),
        }
    }

    async fn get_version(ip: IpAddr) -> Option<Version> {
        let (text, _, _) = util::send_web_command(&ip, "/api/system/info").await?;
        let data: serde_json::Value = serde_json::from_str(&text).ok()?;
        parse_version(data["version"].as_str()?)
    }
}

impl FirmwareIdentification for ForgeaxeFirmware {
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("ForgeOS")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for ForgeaxeFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = Self::get_model(ip).await?;
        let version = Self::get_version(ip).await;
        let mut miner = crate::backends::Forgeaxe::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_versions() {
        assert_eq!(parse_version("v1.1"), Some(Version::new(1, 1, 0)));
        assert_eq!(parse_version("1.7"), Some(Version::new(1, 7, 0)));
        assert_eq!(parse_version("v1.1.2"), Some(Version::new(1, 1, 2)));
        assert_eq!(
            parse_version("v1.1.0-rc.1"),
            Version::parse("1.1.0-rc.1").ok()
        );
        assert_eq!(parse_version("unknown"), None);
        assert_eq!(parse_version("1.invalid"), None);
    }

    #[test]
    fn identifies_forgeos_only() {
        for (body, expected) in [
            ("<html><title>ForgeOS</title></html>", true),
            ("<html><title>AxeOS</title></html>", false),
            ("<html><title>NerdAxe</title></html>", false),
            ("", false),
        ] {
            let response = WebResponse {
                body,
                auth_header: "",
                algo_header: "",
                redirect_header: "",
                status: 200,
            };
            assert_eq!(
                ForgeaxeFirmware::default().identify_web(&response),
                expected
            );
        }
        assert!(ForgeaxeFirmware::default().is_stock());
    }
}

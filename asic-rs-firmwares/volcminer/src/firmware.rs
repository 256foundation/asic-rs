use std::{fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::command::MinerCommand,
    discovery::HTTP_WEB_ROOT,
    errors::ModelSelectionError,
    traits::{
        discovery::DiscoveryCommands,
        entry::FirmwareEntry,
        firmware::MinerFirmware,
        identification::{FirmwareIdentification, WebResponse},
        make::MinerMake,
        miner::{HasDefaultAuth, Miner, MinerAuth, MinerConstructor},
        model::MinerModel,
    },
};
use asic_rs_makes_volcminer::{make::VolcMinerMake, models::VolcMinerModel};
use async_trait::async_trait;
use serde_json::Value;

use crate::backends::v1::{VolcMinerV1, web::VolcMinerWebAPI};

#[derive(Default, Debug)]
pub struct VolcMinerFirmware {}

impl Display for VolcMinerFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VolcMiner Stock")
    }
}

impl DiscoveryCommands for VolcMinerFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![HTTP_WEB_ROOT]
    }
}

async fn get_system_info_with_auth(ip: IpAddr, auth: &MinerAuth) -> Option<Value> {
    VolcMinerWebAPI::new(ip, auth.clone())
        .get_system_info()
        .await
        .ok()
}

async fn get_model_with_auth(
    ip: IpAddr,
    auth: &MinerAuth,
) -> Result<VolcMinerModel, ModelSelectionError> {
    let json_data = get_system_info_with_auth(ip, auth)
        .await
        .ok_or(ModelSelectionError::NoModelResponse)?;

    let model = json_data["minertype"]
        .as_str()
        .ok_or(ModelSelectionError::UnexpectedModelResponse)?
        .to_uppercase();

    VolcMinerMake::parse_model(model)
}

async fn get_version_with_auth(ip: IpAddr, auth: &MinerAuth) -> Option<semver::Version> {
    let json_data = get_system_info_with_auth(ip, auth).await?;
    let version = json_data["system_filesystem_version"].as_str()?;
    let date = version.split_whitespace().next()?;
    let mut parts = date.split('-').filter_map(|part| part.parse::<u64>().ok());
    Some(semver::Version::new(
        parts.next()?,
        parts.next()?,
        parts.next()?,
    ))
}

#[async_trait]
impl MinerFirmware for VolcMinerFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let default = VolcMinerV1::default_auth();
        get_model_with_auth(ip, &default).await
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let default = VolcMinerV1::default_auth();
        get_version_with_auth(ip, &default).await
    }
}

impl FirmwareIdentification for VolcMinerFirmware {
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("VolcMiner")
            || response.auth_header.contains("blackMiner Configuration")
    }
}

#[async_trait]
impl FirmwareEntry for VolcMinerFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let default = VolcMinerV1::default_auth();
        let resolved = auth.unwrap_or(&default);
        let model = get_model_with_auth(ip, resolved).await?;
        let version = get_version_with_auth(ip, resolved).await;
        let mut miner = crate::backends::VolcMiner::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

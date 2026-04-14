use std::{fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::command::MinerCommand,
    discovery::{RPC_DEVDETAILS, RPC_VERSION},
    errors::ModelSelectionError,
    traits::{
        discovery::DiscoveryCommands,
        entry::FirmwareEntry,
        firmware::MinerFirmware,
        identification::FirmwareIdentification,
        make::MinerMake,
        miner::{Miner, MinerAuth, MinerConstructor},
        model::MinerModel,
    },
    util,
};
use asic_rs_makes_auradine::make::AuradineMake;
use async_trait::async_trait;

#[derive(Default, Debug)]
pub struct AuradineFirmware {}

impl Display for AuradineFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Auradine Stock")
    }
}

impl DiscoveryCommands for AuradineFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![RPC_VERSION, RPC_DEVDETAILS]
    }
}

fn parse_semver_like(version_str: &str) -> Option<semver::Version> {
    let trimmed = version_str.trim().trim_start_matches('v');
    if let Ok(version) = semver::Version::parse(trimmed) {
        return Some(version);
    }

    let normalized = trimmed.replace('-', ".");
    if let Ok(version) = semver::Version::parse(&normalized) {
        return Some(version);
    }

    let parts: Vec<&str> = normalized
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect();
    match parts.len() {
        1 => semver::Version::parse(&format!("{}.0.0", parts[0])).ok(),
        2 => semver::Version::parse(&format!("{}.{}.0", parts[0], parts[1])).ok(),
        _ => None,
    }
}

#[async_trait]
impl MinerFirmware for AuradineFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let data = util::send_rpc_command(&ip, "devdetails")
            .await
            .ok_or(ModelSelectionError::NoModelResponse)?;

        let model = data
            .pointer("/DEVDETAILS/0/Model")
            .and_then(|v| v.as_str())
            .ok_or(ModelSelectionError::UnexpectedModelResponse)?
            .to_ascii_uppercase();

        AuradineMake::parse_model(model)
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let data = util::send_rpc_command(&ip, "version").await?;
        let gcminer_version = data
            .pointer("/VERSION/0/GCMiner")
            .and_then(|v| v.as_str())?;
        parse_semver_like(gcminer_version)
    }
}

impl FirmwareIdentification for AuradineFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        response.contains("GCMINER") || response.contains("FLUXOS")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for AuradineFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = AuradineFirmware::get_model(ip).await?;
        let version = AuradineFirmware::get_version(ip).await;
        let mut miner = crate::backends::Auradine::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

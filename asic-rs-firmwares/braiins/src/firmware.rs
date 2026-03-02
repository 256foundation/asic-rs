use asic_rs_core::data::command::MinerCommand;
use asic_rs_core::data::device::MinerHardware;
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
use asic_rs_makes_antminer::models::AntMinerModel;
use asic_rs_makes_braiins::make::BraiinsMake;
use asic_rs_makes_braiins::models::BraiinsModel;
use async_trait::async_trait;
use std::fmt;
use std::fmt::Display;
use std::net::IpAddr;

#[derive(Clone)]
pub enum BraiinsCompatibleModel {
    AntMiner(AntMinerModel),
    Braiins(BraiinsModel),
}

impl Display for BraiinsCompatibleModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AntMiner(m) => m.fmt(f),
            Self::Braiins(m) => m.fmt(f),
        }
    }
}

impl From<BraiinsCompatibleModel> for MinerHardware {
    fn from(model: BraiinsCompatibleModel) -> Self {
        match model {
            BraiinsCompatibleModel::AntMiner(m) => m.into(),
            BraiinsCompatibleModel::Braiins(m) => m.into(),
        }
    }
}

impl MinerModel for BraiinsCompatibleModel {
    fn make_name(&self) -> String {
        match self {
            Self::AntMiner(m) => m.make_name(),
            Self::Braiins(m) => m.make_name(),
        }
    }
}

#[derive(Default)]
pub struct BraiinsFirmware {}

impl Display for BraiinsFirmware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Braiins")
    }
}

impl DiscoveryCommands for BraiinsFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![RPC_VERSION, HTTP_WEB_ROOT]
    }
}

#[async_trait]
impl MinerFirmware for BraiinsFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        if let Some(json_data) =
            util::send_graphql_command(&ip, "{ bosminer { info { modelName } } }").await
            && let Some(model_str) = json_data["data"]["bosminer"]["info"]["modelName"].as_str()
        {
            let model = model_str
                .to_uppercase()
                .replace("BITMAIN ", "")
                .replace("S19XP", "S19 XP");

            return AntMinerMake::parse_model(model.clone())
                .map(BraiinsCompatibleModel::AntMiner)
                .or_else(|_| BraiinsMake::parse_model(model).map(BraiinsCompatibleModel::Braiins));
        }

        Err(ModelSelectionError::NoModelResponse)
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let response =
            util::send_graphql_command(&ip, "{ bos { info { version { full } } } }").await?;

        let full = response["data"]["bos"]["info"]["version"]["full"].as_str()?;

        let version_str = full.split('-').rev().find(|s| s.contains('.'))?;

        let normalized = version_str
            .split('.')
            .map(|part| part.trim_start_matches('0').to_string())
            .map(|part| {
                if part.is_empty() {
                    "0".to_string()
                } else {
                    part
                }
            })
            .collect::<Vec<_>>()
            .join(".");

        // pad if needed, semver requires major.minor.patch
        let padded = match version_str.split('.').count() {
            2 => format!("{}.0", normalized),
            _ => normalized.to_string(),
        };
        semver::Version::parse(&padded).ok()
    }
}

impl FirmwareIdentification for BraiinsFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        response.contains("BOSMINER") || response.contains("BOSER")
    }

    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("Braiins OS")
    }
}

#[async_trait]
impl FirmwareEntry for BraiinsFirmware {
    async fn build_miner(&self, ip: IpAddr) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = BraiinsFirmware::get_model(ip).await?;
        let version = BraiinsFirmware::get_version(ip).await;
        Ok(crate::backends::Braiins::new(ip, model, version))
    }
}

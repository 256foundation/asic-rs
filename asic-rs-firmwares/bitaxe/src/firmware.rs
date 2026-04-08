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
        miner::{Miner, MinerAuth, MinerConstructor},
        model::MinerModel,
    },
    util,
};
use asic_rs_makes_bitaxe::make::BitaxeMake;
use async_trait::async_trait;

#[derive(Default, Debug)]
pub struct BitaxeFirmware {}

impl Display for BitaxeFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bitaxe Stock")
    }
}

impl DiscoveryCommands for BitaxeFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![HTTP_WEB_ROOT]
    }
}

#[async_trait]
impl MinerFirmware for BitaxeFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let response = util::send_web_command(&ip, "/api/system/info").await;

        match response {
            Some((raw_json, _, _)) => {
                let json_data: Option<serde_json::Value> = serde_json::from_str(&raw_json).ok();
                if json_data.is_none() {
                    return Err(ModelSelectionError::UnexpectedModelResponse);
                }
                let json_data = json_data.unwrap();

                let model = json_data["ASICModel"].as_str();
                if model.is_none() {
                    return Err(ModelSelectionError::UnexpectedModelResponse);
                }
                let model = model.unwrap().to_uppercase();

                BitaxeMake::parse_model(model)
            }
            None => Err(ModelSelectionError::NoModelResponse),
        }
    }

    async fn get_version(_ip: IpAddr) -> Option<semver::Version> {
        None
    }
}

impl FirmwareIdentification for BitaxeFirmware {
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("AxeOS")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for BitaxeFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = BitaxeFirmware::get_model(ip).await?;
        let version = BitaxeFirmware::get_version(ip).await;
        let mut miner = crate::backends::Bitaxe::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

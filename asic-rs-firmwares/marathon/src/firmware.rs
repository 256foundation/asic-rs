use std::{fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::command::MinerCommand,
    discovery::RPC_VERSION,
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
pub struct MarathonFirmware {}

impl Display for MarathonFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Marathon")
    }
}

impl DiscoveryCommands for MarathonFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![RPC_VERSION]
    }
}

#[async_trait]
impl MinerFirmware for MarathonFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let data = util::send_rpc_command(&ip, "version")
            .await
            .ok_or(ModelSelectionError::NoModelResponse)?;

        let model = data["VERSION"][0]["Model"]
            .as_str()
            .ok_or(ModelSelectionError::UnexpectedModelResponse)?
            .to_uppercase();

        AntMinerMake::parse_model(model)
    }

    async fn get_version(_ip: IpAddr) -> Option<semver::Version> {
        None
    }
}

impl FirmwareIdentification for MarathonFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        response.contains("MARAFW") || response.contains("KAONSU")
    }

    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.status == 401 && response.algo_header.contains("MD5")
    }
}

#[async_trait]
impl FirmwareEntry for MarathonFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = MarathonFirmware::get_model(ip).await?;
        let version = MarathonFirmware::get_version(ip).await;
        let mut miner = crate::backends::Marathon::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

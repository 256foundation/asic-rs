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
pub struct LuxMinerFirmware {}

impl Display for LuxMinerFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LuxOS")
    }
}

impl DiscoveryCommands for LuxMinerFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![HTTP_WEB_ROOT, RPC_VERSION]
    }
}

#[async_trait]
impl MinerFirmware for LuxMinerFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
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
    async fn build_miner(&self, ip: IpAddr) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = LuxMinerFirmware::get_model(ip).await?;
        let version = LuxMinerFirmware::get_version(ip).await;
        Ok(crate::backends::LuxMiner::new(ip, model, version))
    }
}

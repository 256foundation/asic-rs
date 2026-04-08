use std::{fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::command::MinerCommand,
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
use asic_rs_makes_avalon::make::AvalonMinerMake;
use async_trait::async_trait;

#[derive(Default, Debug)]
pub struct AvalonStockFirmware {}

impl Display for AvalonStockFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AvalonMiner Stock")
    }
}
impl DiscoveryCommands for AvalonStockFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![RPC_VERSION, HTTP_WEB_ROOT]
    }
}

#[async_trait]
impl MinerFirmware for AvalonStockFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let response = util::send_rpc_command(&ip, "version").await;

        match response {
            Some(json_data) => {
                let model = json_data["VERSION"][0]["MODEL"].as_str();
                if model.is_none() {
                    return Err(ModelSelectionError::UnexpectedModelResponse);
                }
                let model = model.unwrap().split("-").collect::<Vec<&str>>()[0].to_uppercase();

                AvalonMinerMake::parse_model(model)
            }
            None => Err(ModelSelectionError::NoModelResponse),
        }
    }

    async fn get_version(_ip: IpAddr) -> Option<semver::Version> {
        None
    }
}

impl FirmwareIdentification for AvalonStockFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        response.contains("AVALON")
    }

    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("Avalon")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for AvalonStockFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = AvalonStockFirmware::get_model(ip).await?;
        let version = AvalonStockFirmware::get_version(ip).await;
        let mut miner = crate::backends::AvalonMiner::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

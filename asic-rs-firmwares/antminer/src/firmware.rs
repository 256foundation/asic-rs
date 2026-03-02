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
use asic_rs_makes_antminer::make::AntMinerMake;
use async_trait::async_trait;
use chrono::{Datelike, NaiveDateTime};
use diqwest::WithDigestAuth;
use reqwest::{Client, Response};
use std::fmt::Display;
use std::net::IpAddr;

#[derive(Default)]
pub struct AntMinerStockFirmware {}

impl Display for AntMinerStockFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AntMiner Stock")
    }
}

impl DiscoveryCommands for AntMinerStockFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![RPC_VERSION, HTTP_WEB_ROOT]
    }
}

#[async_trait]
impl MinerFirmware for AntMinerStockFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let response: Option<Response> = Client::new()
            .get(format!("http://{ip}/cgi-bin/get_system_info.cgi"))
            .send_digest_auth(("root", "root"))
            .await
            .ok();
        match response {
            Some(data) => {
                let json_data = data.json::<serde_json::Value>().await.ok();
                if json_data.is_none() {
                    return Err(ModelSelectionError::UnexpectedModelResponse);
                }
                let json_data = json_data.unwrap();

                let model = json_data["minertype"].as_str().unwrap_or("").to_uppercase();

                AntMinerMake::parse_model(model)
            }
            None => Err(ModelSelectionError::NoModelResponse),
        }
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let response: Option<Response> = Client::new()
            .get(format!("http://{ip}/cgi-bin/summary.cgi"))
            .send_digest_auth(("root", "root"))
            .await
            .ok();
        match response {
            Some(data) => {
                let json_data = data.json::<serde_json::Value>().await.ok()?;
                let fw_version = json_data["INFO"]["CompileTime"].as_str().unwrap_or("");

                let cleaned: String = {
                    let mut parts: Vec<&str> = fw_version.split_whitespace().collect();
                    parts.remove(4); // remove time zone
                    parts.join(" ")
                };

                let dt = NaiveDateTime::parse_from_str(&cleaned, "%a %b %e %H:%M:%S %Y").ok()?;

                let version =
                    semver::Version::new(dt.year() as u64, dt.month() as u64, dt.day() as u64);

                Some(version)
            }
            None => None,
        }
    }
}

impl FirmwareIdentification for AntMinerStockFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        response.contains("ANTMINER")
    }

    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.status == 401 && response.auth_header.contains("realm=\"antMiner")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for AntMinerStockFirmware {
    async fn build_miner(&self, ip: IpAddr) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = AntMinerStockFirmware::get_model(ip).await?;
        let version = AntMinerStockFirmware::get_version(ip).await;
        Ok(crate::backends::AntMiner::new(ip, model, version))
    }
}

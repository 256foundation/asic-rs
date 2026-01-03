use crate::data::device::models::{MinerModelFactory, ModelSelectionError};
use crate::data::device::{MinerMake, MinerModel};
use reqwest::{Client, Response};
use std::net::IpAddr;

pub(crate) async fn get_model_vnish(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
    let response: Option<Response> = Client::new()
        .get(format!("http://{ip}/api/v1/info"))
        .send()
        .await
        .ok();

    match response {
        Some(data) => {
            let json_data = data.json::<serde_json::Value>().await.ok();

            if json_data.is_none() {
                return Err(ModelSelectionError::UnexpectedModelResponse);
            }
            let json_data = json_data.unwrap();

            let model = json_data["miner"].as_str().unwrap_or("").to_uppercase();

            // VnishOS typically runs on AntMiner hardware
            let mut factory = MinerModelFactory::new();
            factory.with_make(MinerMake::AntMiner).parse_model(&model)
        }
        None => Err(ModelSelectionError::NoModelResponse),
    }
}

pub(crate) async fn get_version_vnish(ip: IpAddr) -> Option<semver::Version> {
    let response: Option<Response> = Client::new()
        .get(format!("http://{ip}/api/v1/info"))
        .send()
        .await
        .ok();

    match response {
        Some(data) => {
            let json_data = data.json::<serde_json::Value>().await.ok()?;
            let fw_version = json_data["fw_version"].as_str().unwrap_or("");

            // Try parsing directly first
            if let Ok(version) = semver::Version::parse(fw_version) {
                return Some(version);
            }

            // If direct parsing fails, try adding .0 for patch version
            let normalized_version = format!("{fw_version}.0");
            semver::Version::parse(&normalized_version).ok()
        }
        None => None,
    }
}

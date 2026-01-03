use crate::data::device::models::{MinerModelFactory, ModelSelectionError};
use crate::data::device::{MinerFirmware, MinerModel};
use reqwest::{Client, Response};
use std::net::IpAddr;

pub(crate) async fn get_model_epic(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
    let response: Option<Response> = Client::new()
        .get(format!("http://{ip}:4028/capabilities"))
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

            let model = json_data["Model"].as_str().unwrap_or("").to_uppercase();

            MinerModelFactory::new()
                .with_firmware(MinerFirmware::EPic)
                .parse_model(&model)
        }
        None => Err(ModelSelectionError::NoModelResponse),
    }
}
pub(crate) async fn get_version_epic(ip: IpAddr) -> Option<semver::Version> {
    let response: Option<Response> = Client::new()
        .get(format!("http://{ip}:4028/summary"))
        .send()
        .await
        .ok();

    match response {
        Some(data) => {
            let json_data = data.json::<serde_json::Value>().await.ok()?;
            let fw_version = json_data["Software"]
                .as_str()
                .unwrap_or("")
                .split(" ")
                .last()?
                .strip_prefix("v")?;
            semver::Version::parse(fw_version).ok()
        }
        None => None,
    }
}

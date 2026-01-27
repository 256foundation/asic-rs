use crate::data::device::models::{MinerModelFactory, ModelSelectionError};
use crate::data::device::{MinerFirmware, MinerMake, MinerModel};
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

            if model.starts_with("WHATSMINER") {
                // Need to append the subtype to the base type
                let submodel = json_data["Model Subtype"]
                    .as_str()
                    .unwrap_or("")
                    .to_uppercase();
                let split_model = model.split(" ").collect::<Vec<&str>>();
                let base_model = split_model.get(1);
                match base_model {
                    None => Err(ModelSelectionError::UnknownModel(model.to_string())),
                    Some(base) => {
                        let full_model = format!("{}{}", base, submodel);
                        MinerModelFactory::new()
                            .with_firmware(MinerFirmware::EPic)
                            .with_make(MinerMake::WhatsMiner)
                            .parse_model(&full_model)
                    }
                }
            } else if model.starts_with("ANTMINER") {
                MinerModelFactory::new()
                    .with_firmware(MinerFirmware::EPic)
                    .with_make(MinerMake::AntMiner)
                    .parse_model(&model)
            } else {
                MinerModelFactory::new()
                    .with_firmware(MinerFirmware::EPic)
                    .parse_model(&model)
            }
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

use crate::data::device::models::{MinerModelFactory, ModelSelectionError};
use crate::data::device::{MinerMake, MinerModel};
use crate::miners::backends::traits::APIClient;
use crate::miners::backends::whatsminer::v3;
use crate::miners::commands::MinerCommand;
use crate::miners::util;
use serde_json::json;
use std::net::IpAddr;

pub(crate) async fn get_model_whatsminer(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
    let response = util::send_rpc_command(&ip, "get_version").await;

    match response {
        Some(json_data) => {
            let fw_version: Option<&str> = json_data["Msg"]["fw_ver"].as_str();
            if fw_version.is_none() {
                return Err(ModelSelectionError::UnexpectedModelResponse);
            }

            let fw_version = fw_version.unwrap();

            // Parse the firmware version format: YYYYMMDD.XX.REL
            // Extract the date components
            if fw_version.len() < 8 {
                return Err(ModelSelectionError::UnexpectedModelResponse);
            }

            let date_part = &fw_version[..8];
            if let (Ok(year), Ok(month), Ok(day)) = (
                date_part[..4].parse::<u32>(),
                date_part[4..6].parse::<u32>(),
                date_part[6..8].parse::<u32>(),
            ) {
                let version = semver::Version::new(year as u64, month as u64, day as u64);
                // Determine which API version to use based on the firmware date
                if semver::VersionReq::parse(">=2024.11.0")
                    .unwrap()
                    .matches(&version)
                {
                    get_model_whatsminer_v3(ip).await
                } else {
                    get_model_whatsminer_v2(ip).await
                }
            } else {
                Err(ModelSelectionError::UnexpectedModelResponse)
            }
        }
        None => Err(ModelSelectionError::NoModelResponse),
    }
}

pub(crate) async fn get_version_whatsminer(ip: IpAddr) -> Option<semver::Version> {
    let response = util::send_rpc_command(&ip, "get_version").await;

    match response {
        Some(json_data) => {
            let fw_version: Option<&str> = json_data["Msg"]["fw_ver"].as_str();
            fw_version?;

            let fw_version = fw_version.unwrap();

            // Parse the firmware version format: YYYYMMDD.XX.REL
            // Extract the date components
            if fw_version.len() < 8 {
                return None;
            }

            let date_part = &fw_version[..8];
            if let (Ok(year), Ok(month), Ok(day)) = (
                date_part[..4].parse::<u32>(),
                date_part[4..6].parse::<u32>(),
                date_part[6..8].parse::<u32>(),
            ) {
                let version = semver::Version::new(year as u64, month as u64, day as u64);
                Some(version)
            } else {
                None
            }
        }
        None => None,
    }
}

pub(crate) async fn get_model_whatsminer_v2(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
    let mut response = None;
    for _ in 0..3 {
        response = util::send_rpc_command(&ip, "devdetails").await;
        if response.is_some() {
            break;
        }
    }

    match response {
        Some(json_data) => {
            let model = json_data["DEVDETAILS"][0]["Model"].as_str();

            if model.is_none() {
                return Err(ModelSelectionError::UnexpectedModelResponse);
            }

            let mut model = model.unwrap().to_uppercase().replace("_", "");
            model.pop();
            model.push('0');

            MinerModelFactory::new()
                .with_make(MinerMake::WhatsMiner)
                .parse_model(&model)
        }
        None => Err(ModelSelectionError::NoModelResponse),
    }
}

pub(crate) async fn get_model_whatsminer_v3(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
    let rpc = v3::WhatsMinerRPCAPI::new(ip, None);
    let response = rpc
        .get_api_result(&MinerCommand::RPC {
            command: "get.device.info",
            parameters: Some(json!("miner")),
        })
        .await;

    match response {
        Ok(json_data) => {
            let model = json_data["msg"]["miner"]["type"].as_str();

            if model.is_none() {
                return Err(ModelSelectionError::UnexpectedModelResponse);
            }

            let mut model = model.unwrap().to_uppercase().replace("_", "");
            model.pop();
            model.push('0');

            MinerModelFactory::new()
                .with_make(MinerMake::WhatsMiner)
                .parse_model(&model)
        }
        Err(_) => Err(ModelSelectionError::NoModelResponse),
    }
}

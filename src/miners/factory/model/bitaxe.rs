use crate::data::device::models::{MinerModelFactory, ModelSelectionError};
use crate::data::device::{MinerMake, MinerModel};
use crate::miners::util;
use std::net::IpAddr;

pub(crate) async fn get_model_bitaxe(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
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

            MinerModelFactory::new()
                .with_make(MinerMake::Bitaxe)
                .parse_model(&model)
        }
        None => Err(ModelSelectionError::NoModelResponse),
    }
}
pub(crate) async fn get_version_bitaxe(ip: IpAddr) -> Option<semver::Version> {
    let raw_json = util::send_web_command(&ip, "/api/system/info")
        .await
        .unwrap()
        .0;
    let response: serde_json::Value = serde_json::from_str(&raw_json).ok()?;

    match response["version"].as_str() {
        Some(v) => {
            let mut version = semver::Version::parse(v.strip_prefix("v")?).ok()?;
            version.pre = semver::Prerelease::EMPTY;
            version.build = semver::BuildMetadata::EMPTY;
            Some(version)
        }
        _ => None,
    }
}

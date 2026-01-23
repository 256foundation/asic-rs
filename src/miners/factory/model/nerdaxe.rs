use crate::data::device::models::{MinerModelFactory, ModelSelectionError};
use crate::data::device::{MinerMake, MinerModel};
use crate::miners::util;
use std::net::IpAddr;

pub(crate) async fn get_model_nerdaxe(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
    let (raw_json, _, _) = util::send_web_command(&ip, "/api/system/info")
        .await
        .ok_or(ModelSelectionError::NoModelResponse)?;

    let json_data: serde_json::Value = serde_json::from_str(&raw_json)
        .map_err(|_| ModelSelectionError::UnexpectedModelResponse)?;

    let model = json_data["deviceModel"]
        .as_str()
        .ok_or(ModelSelectionError::UnexpectedModelResponse)?;

    MinerModelFactory::new()
        .with_make(MinerMake::NerdAxe)
        .parse_model(model)
}

pub(crate) async fn get_version_nerdaxe(ip: IpAddr) -> Option<semver::Version> {
    let raw_json = util::send_web_command(&ip, "/api/system/info").await?.0;
    let response: serde_json::Value = serde_json::from_str(&raw_json).ok()?;

    match response["version"].as_str() {
        Some(v) => {
            let v = v.strip_prefix("v").unwrap_or(v);
            let mut version = semver::Version::parse(v).ok()?;
            version.pre = semver::Prerelease::EMPTY;
            version.build = semver::BuildMetadata::EMPTY;
            Some(version)
        }
        _ => None,
    }
}

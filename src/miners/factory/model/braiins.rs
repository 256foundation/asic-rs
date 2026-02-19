use crate::data::device::models::{MinerModelFactory, ModelSelectionError};
use crate::data::device::{MinerFirmware, MinerModel};
use crate::miners::util;
use std::net::IpAddr;

pub(crate) async fn get_model_braiins_os(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
    if let Some(json_data) =
        util::send_graphql_command(&ip, "{ bosminer { info { modelName } } }").await
        && let Some(model_str) = json_data["data"]["bosminer"]["info"]["modelName"].as_str()
    {
        let model = model_str
            .to_uppercase()
            .replace("BITMAIN ", "")
            .replace("S19XP", "S19 XP");

        return MinerModelFactory::new()
            .with_firmware(MinerFirmware::BraiinsOS)
            .parse_model(&model);
    }

    Err(ModelSelectionError::NoModelResponse)
}

pub(crate) async fn get_version_braiins_os(ip: IpAddr) -> Option<semver::Version> {
    let response = util::send_graphql_command(&ip, "{ bos { info { version { full } } } }").await?;

    let full = response["data"]["bos"]["info"]["version"]["full"].as_str()?;

    let version_str = full.split('-').rev().find(|s| s.contains('.'))?;

    let normalized = version_str
        .split('.')
        .map(|part| part.trim_start_matches('0').to_string())
        .map(|part| {
            if part.is_empty() {
                "0".to_string()
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(".");

    // pad if needed, semver requires major.minor.patch
    let padded = match version_str.split('.').count() {
        2 => format!("{}.0", normalized),
        _ => normalized.to_string(),
    };
    semver::Version::parse(&padded).ok()
}

use std::{fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::command::MinerCommand,
    discovery::HTTP_WEB_ROOT,
    errors::ModelSelectionError,
    traits::{
        discovery::DiscoveryCommands,
        entry::FirmwareEntry,
        firmware::MinerFirmware,
        identification::{FirmwareIdentification, WebResponse},
        make::MinerMake,
        miner::{HasDefaultAuth, Miner, MinerAuth, MinerConstructor},
        model::MinerModel,
    },
};
use asic_rs_makes_volcminer::{make::VolcMinerMake, models::VolcMinerModel};
use async_trait::async_trait;
use serde_json::Value;

use crate::backends::v1::{VolcMinerV1, web::VolcMinerWebAPI};

#[derive(Default, Debug)]
pub struct VolcMinerStockFirmware {}

impl Display for VolcMinerStockFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VolcMiner Stock")
    }
}

impl DiscoveryCommands for VolcMinerStockFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![HTTP_WEB_ROOT]
    }
}

async fn get_system_info_with_auth(ip: IpAddr, auth: &MinerAuth) -> Option<Value> {
    VolcMinerWebAPI::new(ip, auth.clone())
        .get_system_info()
        .await
        .ok()
}

fn model_from_system_info(json_data: &Value) -> Result<VolcMinerModel, ModelSelectionError> {
    let model = json_data["minertype"]
        .as_str()
        .ok_or(ModelSelectionError::UnexpectedModelResponse)?
        .to_uppercase();

    VolcMinerMake::parse_model(model)
}

fn version_from_system_info(json_data: &Value) -> Option<semver::Version> {
    let version = json_data["system_filesystem_version"].as_str()?;
    let date = version.split_whitespace().next()?;
    let mut parts = date.split('-').filter_map(|part| part.parse::<u64>().ok());
    Some(semver::Version::new(
        parts.next()?,
        parts.next()?,
        parts.next()?,
    ))
}

async fn get_model_with_auth(
    ip: IpAddr,
    auth: &MinerAuth,
) -> Result<VolcMinerModel, ModelSelectionError> {
    let json_data = get_system_info_with_auth(ip, auth)
        .await
        .ok_or(ModelSelectionError::NoModelResponse)?;

    model_from_system_info(&json_data)
}

async fn get_version_with_auth(ip: IpAddr, auth: &MinerAuth) -> Option<semver::Version> {
    let json_data = get_system_info_with_auth(ip, auth).await?;
    version_from_system_info(&json_data)
}

#[async_trait]
impl MinerFirmware for VolcMinerStockFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let default = VolcMinerV1::default_auth();
        get_model_with_auth(ip, &default).await
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let default = VolcMinerV1::default_auth();
        get_version_with_auth(ip, &default).await
    }
}

impl FirmwareIdentification for VolcMinerStockFirmware {
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("VolcMiner")
            || response.auth_header.contains("blackMiner Configuration")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for VolcMinerStockFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let default = VolcMinerV1::default_auth();
        let resolved = auth.unwrap_or(&default);
        let model = get_model_with_auth(ip, resolved).await?;
        let version = get_version_with_auth(ip, resolved).await;
        let mut miner = crate::backends::VolcMiner::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

#[cfg(test)]
mod tests {
    use asic_rs_core::traits::discovery::DiscoveryCommands;
    use serde_json::json;

    use super::*;

    fn web_response<'a>(body: &'a str, auth_header: &'a str) -> WebResponse<'a> {
        WebResponse {
            body,
            auth_header,
            algo_header: "",
            redirect_header: "",
            status: 200,
        }
    }

    #[test]
    fn display_and_discovery_match_stock_volcminer() {
        let firmware = VolcMinerStockFirmware::default();

        assert_eq!(firmware.to_string(), "VolcMiner Stock");
        assert_eq!(firmware.get_discovery_commands(), vec![HTTP_WEB_ROOT]);
        assert!(firmware.is_stock());
    }

    #[test]
    fn default_auth_matches_vendor_default() {
        let auth = VolcMinerV1::default_auth();

        assert_eq!(auth.username(), "root");
        assert_eq!(auth.password(), "ltc@dog");
    }

    #[test]
    fn identifies_stock_web_responses() {
        let firmware = VolcMinerStockFirmware::default();

        assert!(firmware.identify_web(&web_response("VolcMiner", "")));
        assert!(firmware.identify_web(&web_response("", "blackMiner Configuration")));
        assert!(!firmware.identify_web(&web_response("other miner", "other realm")));
    }

    #[test]
    fn parses_model_from_system_info_case_insensitively() {
        let info = json!({ "minertype": "VolcMiner D1" });

        assert_eq!(model_from_system_info(&info).unwrap(), VolcMinerModel::D1);
    }

    #[test]
    fn rejects_system_info_without_model() {
        let error = model_from_system_info(&json!({})).unwrap_err();

        assert!(matches!(
            error,
            ModelSelectionError::UnexpectedModelResponse
        ));
    }

    #[test]
    fn parses_version_date_from_system_info() {
        let info = json!({ "system_filesystem_version": "2025-10-08 04-26-50 CST" });

        assert_eq!(
            version_from_system_info(&info),
            Some(semver::Version::new(2025, 10, 8))
        );
    }
}

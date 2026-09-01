use std::{fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::{command::MinerCommand, device::FirmwareType},
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
use asic_rs_makes_elphapex::{make::ElphapexMake, models::ElphapexModel};
use async_trait::async_trait;

use crate::backends::v1::{ElphapexV1, web::ElphapexWebAPI};

#[derive(Default, Debug)]
pub struct ElphapexStockFirmware {}

impl Display for ElphapexStockFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", FirmwareType::ElphapexStock)
    }
}

impl DiscoveryCommands for ElphapexStockFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![HTTP_WEB_ROOT]
    }
}

async fn get_model_with_auth(
    ip: IpAddr,
    auth: &MinerAuth,
) -> Result<ElphapexModel, ModelSelectionError> {
    let json_data = ElphapexWebAPI::new(ip, auth.clone())
        .get_system_info()
        .await
        .map_err(|_| ModelSelectionError::NoModelResponse)?;
    let model = json_data["minertype"]
        .as_str()
        .ok_or(ModelSelectionError::UnexpectedModelResponse)?;
    ElphapexMake::parse_model(model.to_string())
}

async fn get_version_with_auth(ip: IpAddr, auth: &MinerAuth) -> Option<semver::Version> {
    let web = ElphapexWebAPI::new(ip, auth.clone());

    if let Ok(json_data) = web.get_system_info().await
        && let Some(version) = json_data
            .get("system_filesystem_version")
            .and_then(|value| value.as_str())
    {
        let version = version
            .split('_')
            .next_back()
            .unwrap_or(version)
            .trim()
            .trim_start_matches(['v', 'V']);
        if let Ok(version) = version.parse() {
            return Some(version);
        }
    }

    let json_data = web.stats().await.ok()?;
    let version = json_data
        .pointer("/INFO/miner_version")
        .and_then(|value| value.as_str())?
        .split('_')
        .next_back()
        .unwrap_or("")
        .trim()
        .trim_start_matches(['v', 'V']);
    version.parse().ok()
}

#[async_trait]
impl MinerFirmware for ElphapexStockFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let default = ElphapexV1::default_auth();
        get_model_with_auth(ip, &default).await
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let default = ElphapexV1::default_auth();
        get_version_with_auth(ip, &default).await
    }
}

impl FirmwareIdentification for ElphapexStockFirmware {
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.status == 401 && response.auth_header.contains("realm=\"Daoge")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for ElphapexStockFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let default = ElphapexV1::default_auth();
        let resolved = auth.unwrap_or(&default);
        let model = get_model_with_auth(ip, resolved).await?;
        let version = get_version_with_auth(ip, resolved).await;
        let mut miner = crate::backends::Elphapex::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

#[cfg(test)]
mod tests {
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
    fn identifies_stock_web_responses() {
        let firmware = ElphapexStockFirmware::default();

        assert!(firmware.identify_web(&WebResponse {
            body: "",
            auth_header: "Digest realm=\"Daoge Miner\"",
            algo_header: "",
            redirect_header: "",
            status: 401,
        }));
        assert!(!firmware.identify_web(&web_response("DG-Home1", "")));
        assert!(!firmware.identify_web(&WebResponse {
            body: "",
            auth_header: "Digest realm=\"Daoge Miner\"",
            algo_header: "",
            redirect_header: "",
            status: 200,
        }));
        assert!(!firmware.identify_web(&WebResponse {
            body: "",
            auth_header: "Digest realm=\"Elphapex\"",
            algo_header: "",
            redirect_header: "",
            status: 401,
        }));
    }
}

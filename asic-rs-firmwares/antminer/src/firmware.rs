use std::{fmt, fmt::Display, net::IpAddr};

use asic_rs_core::data::device::{FirmwareType, HashAlgorithm, MinerHardware};
use asic_rs_core::traits::model::{MinerModelAlgorithm, UnknownMinerModel};
use asic_rs_core::{
    data::command::MinerCommand,
    discovery::{HTTP_WEB_ROOT, RPC_VERSION},
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
    util::build_discovery_client,
};
use asic_rs_makes_antminer::make::AntMinerMake;
use asic_rs_makes_antminer::models::AntMinerModel;
use async_trait::async_trait;
use chrono::{Datelike, NaiveDateTime};
use diqwest::WithDigestAuth;
use reqwest::{Response, StatusCode};
use serde_json::Value;

#[derive(Clone)]
pub enum AntMinerCompatibleModel {
    AntMiner(AntMinerModel),
    Unknown(UnknownMinerModel),
}

impl Display for AntMinerCompatibleModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AntMiner(m) => m.fmt(f),
            Self::Unknown(m) => m.fmt(f),
        }
    }
}

impl From<AntMinerCompatibleModel> for MinerHardware {
    fn from(model: AntMinerCompatibleModel) -> Self {
        match model {
            AntMinerCompatibleModel::AntMiner(m) => m.into(),
            AntMinerCompatibleModel::Unknown(m) => m.into(),
        }
    }
}

impl MinerModel for AntMinerCompatibleModel {
    fn make_name(&self) -> String {
        match self {
            Self::AntMiner(m) => m.make_name(),
            Self::Unknown(m) => m.make_name(),
        }
    }
    fn is_known(&self) -> bool {
        match self {
            Self::AntMiner(m) => m.is_known(),
            Self::Unknown(m) => m.is_known(),
        }
    }
}

impl MinerModelAlgorithm for AntMinerCompatibleModel {
    fn hash_algorithm(&self) -> HashAlgorithm {
        match self {
            Self::AntMiner(m) => m.hash_algorithm(),
            Self::Unknown(m) => m.hash_algorithm(),
        }
    }
}

#[derive(Default, Debug)]
pub struct AntMinerStockFirmware {}

impl Display for AntMinerStockFirmware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", FirmwareType::AntMinerStock)
    }
}

impl DiscoveryCommands for AntMinerStockFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![RPC_VERSION, HTTP_WEB_ROOT]
    }
}

fn parse_model_response(
    json_data: Value,
    model_key: &str,
) -> Result<AntMinerCompatibleModel, ModelSelectionError> {
    let model = json_data[model_key].as_str().unwrap_or("").to_uppercase();

    if model == "ANTMINER BHB42XXX" {
        Ok(AntMinerCompatibleModel::Unknown(UnknownMinerModel {
            name: model,
        }))
    } else {
        AntMinerMake::parse_model(model).map(AntMinerCompatibleModel::AntMiner)
    }
}

/// Fetch the model from a miner using digest auth.
async fn get_model_with_auth(
    ip: IpAddr,
    auth: &MinerAuth,
) -> Result<AntMinerCompatibleModel, ModelSelectionError> {
    let client = build_discovery_client()?;
    let response: Option<Response> = client
        .get(format!("http://{ip}/cgi-bin/miner_type.cgi"))
        .send_digest_auth((auth.username(), auth.password()))
        .await
        .ok();

    let Some(response) = response else {
        return Err(ModelSelectionError::NoModelResponse);
    };

    let (response, model_key) = if response.status() == StatusCode::NOT_FOUND {
        let fallback_response: Option<Response> = client
            .get(format!("http://{ip}/cgi-bin/get_system_info.cgi"))
            .send_digest_auth((auth.username(), auth.password()))
            .await
            .ok();

        let Some(response) = fallback_response else {
            return Err(ModelSelectionError::NoModelResponse);
        };

        (response, "minertype")
    } else {
        (response, "miner_type")
    };

    if let Ok(json_data) = response.json::<Value>().await {
        parse_model_response(json_data, model_key)
    } else {
        Err(ModelSelectionError::UnexpectedModelResponse)
    }
}

/// Fetch the firmware version from a miner using digest auth.
async fn get_version_with_auth(ip: IpAddr, auth: &MinerAuth) -> Option<semver::Version> {
    let client = build_discovery_client().ok()?;
    let data: Response = client
        .get(format!("http://{ip}/cgi-bin/summary.cgi"))
        .send_digest_auth((auth.username(), auth.password()))
        .await
        .ok()?;

    let json_data = data.json::<serde_json::Value>().await.ok()?;
    let fw_version = json_data["INFO"]["CompileTime"].as_str().unwrap_or("");

    let cleaned: String = {
        let mut parts: Vec<&str> = fw_version.split_whitespace().collect();
        if parts.len() > 4 {
            parts.remove(4); // remove time zone
        }
        parts.join(" ")
    };

    let dt = NaiveDateTime::parse_from_str(&cleaned, "%a %b %e %H:%M:%S %Y").ok()?;

    Some(semver::Version::new(
        dt.year() as u64,
        dt.month() as u64,
        dt.day() as u64,
    ))
}

#[async_trait]
impl MinerFirmware for AntMinerStockFirmware {
    /// Uses default credentials. For custom credentials, use `build_miner`
    /// which passes auth through to the underlying digest-auth requests.
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel + Send, ModelSelectionError> {
        let default = crate::backends::v2020::AntMinerV2020::default_auth();
        get_model_with_auth(ip, &default).await
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let default = crate::backends::v2020::AntMinerV2020::default_auth();
        get_version_with_auth(ip, &default).await
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
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let default = crate::backends::v2020::AntMinerV2020::default_auth();
        let resolved = auth.unwrap_or(&default);
        let model = get_model_with_auth(ip, resolved).await?;
        let version = get_version_with_auth(ip, resolved).await;
        let mut miner = crate::backends::AntMiner::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

#[cfg(test)]
mod tests {
    use asic_rs_makes_antminer::models::AntMinerModel;

    use super::*;

    /// Live model detection always yields an [`AntMinerCompatibleModel`], never
    /// a bare [`AntMinerModel`]. When this wrapper failed to forward
    /// `hash_algorithm` every miner reported SHA-256 on real hardware while the
    /// make-level tests passed, so the forwarding is asserted here directly.
    #[test]
    fn wrapper_forwards_hash_algorithm() {
        assert_eq!(
            AntMinerCompatibleModel::AntMiner(AntMinerModel::L9).hash_algorithm(),
            HashAlgorithm::Scrypt
        );
        assert_eq!(
            AntMinerCompatibleModel::AntMiner(AntMinerModel::L11).hash_algorithm(),
            HashAlgorithm::Scrypt
        );
        assert_eq!(
            AntMinerCompatibleModel::AntMiner(AntMinerModel::S21).hash_algorithm(),
            HashAlgorithm::SHA256
        );
        assert_eq!(
            AntMinerCompatibleModel::Unknown(UnknownMinerModel {
                name: "ANTMINER S99".to_string(),
            })
            .hash_algorithm(),
            HashAlgorithm::Unknown
        );
    }
}

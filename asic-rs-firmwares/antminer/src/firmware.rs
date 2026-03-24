use std::{fmt, fmt::Display, net::IpAddr};

use asic_rs_core::data::device::MinerHardware;
use asic_rs_core::traits::model::UnknownMinerModel;
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
        miner::{ExposeSecret, HasDefaultAuth, Miner, MinerAuth, MinerConstructor},
        model::MinerModel,
    },
};
use asic_rs_makes_antminer::make::AntMinerMake;
use asic_rs_makes_antminer::models::AntMinerModel;
use async_trait::async_trait;
use chrono::{Datelike, NaiveDateTime};
use diqwest::WithDigestAuth;
use reqwest::{Client, Response};
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
}

#[derive(Default, Debug)]
pub struct AntMinerStockFirmware {}

impl Display for AntMinerStockFirmware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AntMiner Stock")
    }
}

impl DiscoveryCommands for AntMinerStockFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![RPC_VERSION, HTTP_WEB_ROOT]
    }
}

/// Fetch the model from a miner using digest auth.
async fn get_model_with_auth(
    ip: IpAddr,
    auth: &MinerAuth,
) -> Result<AntMinerCompatibleModel, ModelSelectionError> {
    let response: Option<Response> = Client::new()
        .get(format!("http://{ip}/cgi-bin/miner_type.cgi"))
        .send_digest_auth((auth.username.as_str(), auth.password.expose_secret()))
        .await
        .ok();
    match response {
        Some(data) => {
            let json_data = data.json::<Value>().await.ok();
            if json_data.is_none() {
                return Err(ModelSelectionError::UnexpectedModelResponse);
            }
            let json_data = json_data.unwrap();

            let model = json_data["miner_type"]
                .as_str()
                .unwrap_or("")
                .to_uppercase();

            if model == "ANTMINER BHB42XXX" {
                Ok(AntMinerCompatibleModel::Unknown(UnknownMinerModel {
                    name: model,
                }))
            } else {
                AntMinerMake::parse_model(model).map(AntMinerCompatibleModel::AntMiner)
            }
        }
        None => Err(ModelSelectionError::NoModelResponse),
    }
}

/// Fetch the firmware version from a miner using digest auth.
async fn get_version_with_auth(ip: IpAddr, auth: &MinerAuth) -> Option<semver::Version> {
    let response: Option<Response> = Client::new()
        .get(format!("http://{ip}/cgi-bin/summary.cgi"))
        .send_digest_auth((auth.username.as_str(), auth.password.expose_secret()))
        .await
        .ok();
    match response {
        Some(data) => {
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

            let version =
                semver::Version::new(dt.year() as u64, dt.month() as u64, dt.day() as u64);

            Some(version)
        }
        None => None,
    }
}

#[async_trait]
impl MinerFirmware for AntMinerStockFirmware {
    /// Uses default credentials. For custom credentials, use `build_miner`
    /// which passes auth through to the underlying digest-auth requests.
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
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

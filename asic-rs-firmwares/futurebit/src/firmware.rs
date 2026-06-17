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
use asic_rs_makes_futurebit::{make::FutureBitMake, models::FutureBitModel};
use async_trait::async_trait;

#[derive(Default, Debug)]
pub struct ApolloFirmware {}

impl Display for ApolloFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FutureBit Stock")
    }
}

impl DiscoveryCommands for ApolloFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![HTTP_WEB_ROOT]
    }
}

async fn get_model_with_auth(
    ip: IpAddr,
    auth: &MinerAuth,
) -> Result<FutureBitModel, ModelSelectionError> {
    let stats = crate::backends::v2::ApolloGraphQLAPI::new(ip, auth.clone())
        .get_miner_stats()
        .await
        .map_err(|_| ModelSelectionError::NoModelResponse)?;

    let version = stats
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    if version.eq_ignore_ascii_case("v1") {
        FutureBitMake::parse_model("Apollo BTC".to_string())
    } else if version.eq_ignore_ascii_case("v2") {
        FutureBitMake::parse_model("Apollo II".to_string())
    } else {
        FutureBitMake::parse_model("Apollo".to_string())
    }
}

async fn get_version_with_auth(ip: IpAddr, auth: &MinerAuth) -> Option<semver::Version> {
    let stats = crate::backends::v2::ApolloGraphQLAPI::new(ip, auth.clone())
        .get_miner_stats()
        .await
        .ok()?;
    let version = stats
        .pointer("/versions/miner")
        .or_else(|| stats.get("statVersion"))?
        .as_str()?;
    semver::Version::parse(version.trim_start_matches('v')).ok()
}

#[async_trait]
impl MinerFirmware for ApolloFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let auth = crate::backends::v2::ApolloV2::default_auth();
        get_model_with_auth(ip, &auth).await
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let auth = crate::backends::v2::ApolloV2::default_auth();
        get_version_with_auth(ip, &auth).await
    }
}

impl FirmwareIdentification for ApolloFirmware {
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.status == 308 && response.redirect_header.contains("/overview")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for ApolloFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let default = crate::backends::v2::ApolloV2::default_auth();
        let resolved = auth.unwrap_or(&default);
        let model = get_model_with_auth(ip, resolved).await?;
        let version = get_version_with_auth(ip, resolved).await;
        let mut miner = crate::backends::Apollo::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

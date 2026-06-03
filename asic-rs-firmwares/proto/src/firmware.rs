use std::{fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::command::MinerCommand,
    discovery::HTTP_WEB_ROOT,
    errors::ModelSelectionError,
    traits::{
        auth::{HasAuth, HasDefaultAuth},
        discovery::DiscoveryCommands,
        entry::FirmwareEntry,
        firmware::MinerFirmware,
        identification::{FirmwareIdentification, WebResponse},
        make::MinerMake,
        miner::{Miner, MinerAuth},
    },
    util::send_web_command,
};
use asic_rs_makes_proto::make::ProtoMake;
use async_trait::async_trait;
use serde_json::Value;

// Discovery identifies Proto from its web dashboard on port 80 (the shared
// `HTTP_WEB_ROOT` path), but the JSON API — including the system endpoint that
// backs model/version lookups — is served on 8080, so that port is part of the
// path used after identification.
const WEB_SYSTEM: &str = ":8080/api/v1/system";

#[derive(Default, Debug)]
pub struct ProtoFirmware {}

impl Display for ProtoFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Proto Stock")
    }
}

impl DiscoveryCommands for ProtoFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![HTTP_WEB_ROOT]
    }
}

#[async_trait]
impl MinerFirmware for ProtoFirmware {
    async fn get_model(
        ip: IpAddr,
    ) -> Result<impl asic_rs_core::traits::model::MinerModel, ModelSelectionError> {
        let (body, _, _) = send_web_command(&ip, WEB_SYSTEM)
            .await
            .ok_or(ModelSelectionError::NoModelResponse)?;
        let data: Value = serde_json::from_str(&body)
            .map_err(|_| ModelSelectionError::UnexpectedModelResponse)?;
        let model = data
            .pointer("/system-info/model")
            .and_then(Value::as_str)
            .ok_or(ModelSelectionError::UnexpectedModelResponse)?;
        // `ProtoModel::from_str` normalizes (trims/uppercases) the value, so
        // hand it the raw model string as-is.
        ProtoMake::parse_model(model.to_string())
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let (body, _, _) = send_web_command(&ip, WEB_SYSTEM).await?;
        let data: Value = serde_json::from_str(&body).ok()?;
        let version = data
            .pointer("/system-info/os/version")
            .and_then(Value::as_str)?;
        semver::Version::parse(version.trim_start_matches('v')).ok()
    }
}

impl FirmwareIdentification for ProtoFirmware {
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        // The Proto web dashboard served at `/` carries this in its <title>.
        response.body.contains("Proto OS")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for ProtoFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = ProtoFirmware::get_model(ip).await?;
        let version = ProtoFirmware::get_version(ip).await;
        // Proto rigs are heterogeneous and hot-swappable, so the board/chip/fan
        // layout can't be derived from the model. Discover it from the device
        // here, as part of discovery, and pass it into the miner rather than
        // mutating the constructed miner afterward. The hardware endpoint is
        // authenticated, so use the resolved credentials.
        let default = crate::backends::ProtoV1::default_auth();
        let resolved = auth.unwrap_or(&default);
        let hardware = crate::backends::ProtoV1::discover_hardware(ip, resolved).await;
        let mut miner = crate::backends::ProtoV1::new(ip, model, version, hardware);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(Box::new(miner))
    }
}

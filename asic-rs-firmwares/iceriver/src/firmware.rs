use std::{fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::command::DiscoveryCommand,
    discovery::HTTP_WEB_ROOT,
    errors::ModelSelectionError,
    traits::{
        discovery::DiscoveryCommands,
        entry::FirmwareEntry,
        firmware::MinerFirmware,
        identification::{FirmwareIdentification, WebResponse},
        miner::{HasDefaultAuth, Miner, MinerAuth},
    },
};
use asic_rs_makes_iceriver::models::IceRiverModel;
use async_trait::async_trait;
use serde_json::Value;

use crate::backends::v1::{IceRiverV1, panel_data, web::IceRiverWebAPI};

#[derive(Debug, Default)]
pub struct IceRiverStockFirmware;

impl Display for IceRiverStockFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IceRiver Stock")
    }
}

impl DiscoveryCommands for IceRiverStockFirmware {
    fn get_discovery_commands(&self) -> Vec<DiscoveryCommand> {
        vec![HTTP_WEB_ROOT]
    }
}

pub(crate) fn model_from_userpanel(response: &Value) -> Result<IceRiverModel, ModelSelectionError> {
    let panel = panel_data(response, None).ok_or(ModelSelectionError::UnexpectedModelResponse)?;
    let version = panel
        .get("softver1")
        .and_then(Value::as_str)
        .filter(|version| !version.trim().is_empty())
        .ok_or(ModelSelectionError::UnexpectedModelResponse)?;
    let parts = version.split('_').collect::<Vec<_>>();
    let last = *parts
        .last()
        .ok_or(ModelSelectionError::UnexpectedModelResponse)?;
    let model = if last.eq_ignore_ascii_case("miner") {
        parts
            .get(parts.len().saturating_sub(2))
            .copied()
            .ok_or(ModelSelectionError::UnexpectedModelResponse)?
    } else {
        last.strip_suffix("miner")
            .or_else(|| last.strip_suffix("MINER"))
            .unwrap_or(last)
    };
    if model.is_empty() || model.eq_ignore_ascii_case("miner") {
        return Err(ModelSelectionError::UnexpectedModelResponse);
    }
    model.parse()
}

#[async_trait]
impl MinerFirmware for IceRiverStockFirmware {
    type Model = IceRiverModel;

    async fn get_model(ip: IpAddr) -> Result<Self::Model, ModelSelectionError> {
        let response = IceRiverWebAPI::new(ip, IceRiverV1::default_auth())
            .userpanel()
            .await
            .map_err(|_| ModelSelectionError::NoModelResponse)?;
        model_from_userpanel(&response)
    }

    async fn get_version(_ip: IpAddr) -> Option<semver::Version> {
        // IceRiver exposes its opaque software label in get_firmware_version;
        // that label is not reliably a semantic version.
        None
    }
}

impl FirmwareIdentification for IceRiverStockFirmware {
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        // Same stock signature used by the established pyasic discovery code.
        response.body.contains("<TITLE>用户界面</TITLE>")
            || response.body.to_ascii_lowercase().contains("iceriver")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for IceRiverStockFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let resolved = auth.cloned().unwrap_or_else(IceRiverV1::default_auth);
        let response = IceRiverWebAPI::new(ip, resolved.clone())
            .userpanel()
            .await
            .map_err(|_| ModelSelectionError::NoModelResponse)?;
        let model = model_from_userpanel(&response)?;
        Ok(Box::new(IceRiverV1::with_auth(ip, model, resolved)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn identifies_iceriver_signature_without_claiming_bitmain_al3() {
        let firmware = IceRiverStockFirmware;
        let web = |body| WebResponse {
            body,
            auth_header: "",
            algo_header: "",
            redirect_header: "",
            status: 200,
        };
        assert!(firmware.identify_web(&web("<TITLE>用户界面</TITLE>")));
        assert!(firmware.identify_web(&web("IceRiver")));
        assert!(!firmware.identify_web(&web("Antminer AL3")));
    }

    #[test]
    fn parses_source_backed_model_suffixes_and_rejects_missing_identity() {
        assert_eq!(
            model_from_userpanel(&json!({"data":{"softver1":"release_10306_miner"}})).unwrap(),
            IceRiverModel::AL3
        );
        assert!(model_from_userpanel(&json!({"data":{}})).is_err());
        assert!(model_from_userpanel(&json!({"data":{"softver1":"miner"}})).is_err());
    }
}

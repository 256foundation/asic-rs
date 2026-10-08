use crate::backends::v1::{GoldshellV1, web::GoldshellWebAPI};
use asic_rs_core::{
    data::command::DiscoveryCommand,
    discovery::{HTTP_WEB_ROOT, RPC_VERSION},
    errors::ModelSelectionError,
    traits::{
        discovery::DiscoveryCommands,
        entry::FirmwareEntry,
        firmware::MinerFirmware,
        identification::{FirmwareIdentification, WebResponse},
        miner::{HasAuth, HasDefaultAuth, Miner, MinerAuth},
    },
};
use asic_rs_makes_goldshell::models::GoldshellModel;
use async_trait::async_trait;
use serde_json::Value;
use std::{fmt::Display, net::IpAddr, str::FromStr};

#[derive(Default, Debug)]
pub struct GoldshellFirmware;
impl Display for GoldshellFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Goldshell Stock")
    }
}
impl DiscoveryCommands for GoldshellFirmware {
    fn get_discovery_commands(&self) -> Vec<DiscoveryCommand> {
        vec![
            HTTP_WEB_ROOT,
            RPC_VERSION,
            DiscoveryCommand::Web {
                command: "/mcb/status",
                port: 80,
            },
        ]
    }
}
pub(crate) fn model_from_status(status: &Value) -> Result<GoldshellModel, ModelSelectionError> {
    let raw = status
        .get("model")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or(ModelSelectionError::UnexpectedModelResponse)?;
    GoldshellModel::from_str(raw)
}
#[async_trait]
impl MinerFirmware for GoldshellFirmware {
    type Model = GoldshellModel;
    async fn get_model(ip: IpAddr) -> Result<Self::Model, ModelSelectionError> {
        let value = GoldshellWebAPI::new(ip, GoldshellV1::default_auth())
            .read("status")
            .await
            .map_err(|_| ModelSelectionError::NoModelResponse)?;
        model_from_status(&value)
    }
    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let value = GoldshellWebAPI::new(ip, GoldshellV1::default_auth())
            .read("status")
            .await
            .ok()?;
        semver::Version::parse(value.get("firmware")?.as_str()?.trim_start_matches('v')).ok()
    }
}
impl FirmwareIdentification for GoldshellFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        let response = response.to_ascii_lowercase();
        response.contains("goldshell")
            || response.contains("kdaminer")
            || response.contains("intchains_qomo")
    }
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        let body = response.body.to_ascii_lowercase();
        matches!(response.status, 200 | 401 | 403)
            && (body.contains("goldshell")
                || body.contains("cloud-box")
                || response
                    .auth_header
                    .to_ascii_lowercase()
                    .contains("goldshell"))
    }
    fn is_stock(&self) -> bool {
        true
    }
}
#[async_trait]
impl FirmwareEntry for GoldshellFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let resolved = auth.cloned().unwrap_or_else(GoldshellV1::default_auth);
        let web = GoldshellWebAPI::new(ip, resolved.clone());
        let model = match web.read("status").await {
            Ok(status) => model_from_status(&status)
                .unwrap_or_else(|_| GoldshellModel::Unknown("Goldshell".into())),
            Err(_) => GoldshellModel::Unknown("Goldshell".into()),
        };
        let mut miner = GoldshellV1::new(ip, model);
        miner.set_auth(resolved);
        Ok(Box::new(miner))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use asic_rs_core::data::device::HashAlgorithm;
    use asic_rs_core::traits::model::MinerModelAlgorithm;
    #[test]
    fn generic_inventory_identity_and_firmware_do_not_imply_an_algorithm() {
        let value = serde_json::json!({"model": "Goldshell", "firmware": "2.2.0"});
        assert_eq!(
            model_from_status(&value).unwrap().hash_algorithm(),
            HashAlgorithm::Unknown
        );
        assert!(model_from_status(&serde_json::json!({"firmware":"2.2.0"})).is_err());
    }
}

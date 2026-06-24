use std::{fmt::Display, net::IpAddr};

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
    util::{build_discovery_client, send_rpc_command},
};
use asic_rs_makes_sealminer::make::SealMinerMake;
use asic_rs_makes_sealminer::models::SealMinerModel;
use async_trait::async_trait;

#[derive(Default, Debug)]
pub struct SealMinerStockFirmware {}

impl Display for SealMinerStockFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SealMiner Stock")
    }
}

impl DiscoveryCommands for SealMinerStockFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![RPC_VERSION, HTTP_WEB_ROOT]
    }
}

async fn get_system_info(ip: IpAddr, auth: &MinerAuth) -> Option<serde_json::Value> {
    let client = build_discovery_client().ok()?;

    let login_response = client
        .post(format!("http://{ip}/cgi-bin/login.php"))
        .header(
            "Content-Type",
            "application/x-www-form-urlencoded; charset=UTF-8",
        )
        .header("X-Requested-With", "XMLHttpRequest")
        .body(format!(
            "username={}&origin_pwd={}",
            auth.username(),
            auth.password()
        ))
        .send()
        .await
        .ok()?;

    let session_cookie = login_response
        .headers()
        .get("set-cookie")?
        .to_str()
        .ok()?
        .split(';')
        .next()?
        .to_string();

    client
        .get(format!("http://{ip}/cgi-bin/get_system_info.php"))
        .header("Cookie", session_cookie)
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()
}

async fn get_model_with_auth(
    ip: IpAddr,
    auth: &MinerAuth,
) -> Result<SealMinerModel, ModelSelectionError> {
    // Try RPC first (no auth required); fall back to web if miner hasn't started yet.
    if let Some(data) = send_rpc_command(&ip, "devdetails").await
        && let Some(model) = data["DEVDETAILS"][0]["Model"].as_str()
    {
        return SealMinerMake::parse_model(model.to_string());
    }

    let info = get_system_info(ip, auth)
        .await
        .ok_or(ModelSelectionError::NoModelResponse)?;
    let model = info["miner_type"]
        .as_str()
        .ok_or(ModelSelectionError::UnexpectedModelResponse)?
        .to_string();
    SealMinerMake::parse_model(model)
}

async fn get_version_with_auth(ip: IpAddr, auth: &MinerAuth) -> Option<semver::Version> {
    // Try RPC first; fall back to web.
    let fw_version_str = if let Some(data) = send_rpc_command(&ip, "stats").await {
        data["STATS"][0]["Firmware"].as_str().map(|s| s.to_string())
    } else {
        None
    };

    let fw_version_str = match fw_version_str {
        Some(s) => s,
        None => get_system_info(ip, auth).await?["firmware_version"]
            .as_str()?
            .to_string(),
    };

    if fw_version_str.len() < 8 {
        return None;
    }
    let year: u64 = fw_version_str[0..4].parse().ok()?;
    let month: u64 = fw_version_str[4..6].parse().ok()?;
    let day: u64 = fw_version_str[6..8].parse().ok()?;
    Some(semver::Version::new(year, month, day))
}

#[async_trait]
impl MinerFirmware for SealMinerStockFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let default = crate::backends::v2025::SealMinerV2025::default_auth();
        get_model_with_auth(ip, &default).await
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let default = crate::backends::v2025::SealMinerV2025::default_auth();
        get_version_with_auth(ip, &default).await
    }
}

impl FirmwareIdentification for SealMinerStockFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        response.contains("BDMINER")
    }

    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("amazeui")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for SealMinerStockFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let default = crate::backends::v2025::SealMinerV2025::default_auth();
        let resolved = auth.unwrap_or(&default);
        let model = get_model_with_auth(ip, resolved).await?;
        let version = get_version_with_auth(ip, resolved).await;
        let mut miner = crate::backends::SealMiner::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

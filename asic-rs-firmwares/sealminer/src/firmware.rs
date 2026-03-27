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
        miner::{Miner, MinerConstructor},
        model::MinerModel,
    },
    util::send_rpc_command,
};
use asic_rs_makes_sealminer::make::SealMinerMake;
use async_trait::async_trait;
use reqwest::Client;

#[derive(Default)]
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

async fn get_system_info(ip: IpAddr) -> Option<serde_json::Value> {
    let client = Client::new();

    let login_response = client
        .post(format!("http://{ip}/cgi-bin/login.php"))
        .header(
            "Content-Type",
            "application/x-www-form-urlencoded; charset=UTF-8",
        )
        .header("X-Requested-With", "XMLHttpRequest")
        .body("username=seal&origin_pwd=seal")
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

#[async_trait]
impl MinerFirmware for SealMinerStockFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        // Try RPC first (no auth required); fall back to web if miner hasn't started yet.
        if let Some(data) = send_rpc_command(&ip, "devdetails").await
            && let Some(model) = data["DEVDETAILS"][0]["Model"].as_str()
        {
            return SealMinerMake::parse_model(model.to_string());
        }

        let info = get_system_info(ip)
            .await
            .ok_or(ModelSelectionError::NoModelResponse)?;
        let model = info["miner_type"]
            .as_str()
            .ok_or(ModelSelectionError::UnexpectedModelResponse)?
            .to_string();
        SealMinerMake::parse_model(model)
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        // Try RPC first; fall back to web.
        let fw_version_str = if let Some(data) = send_rpc_command(&ip, "stats").await {
            data["STATS"][0]["Firmware"].as_str().map(|s| s.to_string())
        } else {
            None
        };

        let fw_version_str = match fw_version_str {
            Some(s) => s,
            None => get_system_info(ip).await?["firmware_version"]
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
    async fn build_miner(&self, ip: IpAddr) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = SealMinerStockFirmware::get_model(ip).await?;
        let version = SealMinerStockFirmware::get_version(ip).await;
        Ok(crate::backends::SealMiner::new(ip, model, version))
    }
}

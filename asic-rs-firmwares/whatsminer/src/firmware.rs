use std::{fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::command::MinerCommand,
    discovery::{HTTP_WEB_ROOT, RPC_DEVDETAILS},
    errors::ModelSelectionError,
    traits::{
        discovery::DiscoveryCommands,
        entry::FirmwareEntry,
        firmware::MinerFirmware,
        identification::{FirmwareIdentification, WebResponse},
        make::MinerMake,
        miner::{APIClient, HasDefaultAuth, Miner, MinerAuth, MinerConstructor},
        model::MinerModel,
    },
    util,
};
use asic_rs_makes_whatsminer::make::WhatsMinerMake;
use async_trait::async_trait;
use serde_json::json;

use crate::backends::v3;

#[derive(Default, Debug)]
pub struct WhatsMinerFirmware {}

impl Display for WhatsMinerFirmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WhatsMiner Stock")
    }
}

fn normalize_whatsminer_model(model: &str) -> String {
    let mut s = model.to_uppercase().replace("_", "");
    if !s.is_empty() {
        s.pop();
        s.push('0');
    }
    s
}

async fn send_v3_get_device_info(ip: &IpAddr) -> Option<serde_json::Value> {
    let auth = crate::backends::WhatsMinerV3::default_auth();
    let rpc = v3::WhatsMinerRPCAPI::new(*ip, None, auth);
    let response = rpc
        .get_api_result(&MinerCommand::RPC {
            command: "get.device.info",
            parameters: Some(json!("miner")),
        })
        .await;
    response.ok()
}

impl DiscoveryCommands for WhatsMinerFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![RPC_DEVDETAILS, HTTP_WEB_ROOT]
    }
}

#[async_trait]
impl MinerFirmware for WhatsMinerFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let use_v3 = if let Some(data) = util::send_rpc_command(&ip, "get_version").await {
            data["Msg"]["fw_ver"]
                .as_str()
                .and_then(|v| v.split('.').next())
                .and_then(|date| date.parse::<u64>().ok())
                .map(|date| date >= 20241101)
                .unwrap_or(false)
        } else {
            false
        };

        if use_v3 {
            let data = send_v3_get_device_info(&ip)
                .await
                .ok_or(ModelSelectionError::NoModelResponse)?;
            let model_str = data["msg"]["miner"]["type"]
                .as_str()
                .ok_or(ModelSelectionError::UnexpectedModelResponse)?;
            WhatsMinerMake::parse_model(normalize_whatsminer_model(model_str))
        } else {
            let data = util::send_rpc_command(&ip, "devdetails")
                .await
                .ok_or(ModelSelectionError::NoModelResponse)?;
            let model_str = data["DEVDETAILS"][0]["Model"]
                .as_str()
                .ok_or(ModelSelectionError::UnexpectedModelResponse)?;
            WhatsMinerMake::parse_model(normalize_whatsminer_model(model_str))
        }
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let data = util::send_rpc_command(&ip, "get_version").await?;
        let fw_ver = data["Msg"]["fw_ver"].as_str()?;
        let date_part = fw_ver.split('.').next()?;
        if date_part.len() != 8 {
            return None;
        }
        let year: u64 = date_part[0..4].parse().ok()?;
        let month: u64 = date_part[4..6].parse().ok()?;
        let day: u64 = date_part[6..8].parse().ok()?;
        Some(semver::Version::new(year, month, day))
    }
}

impl FirmwareIdentification for WhatsMinerFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        response.contains("BITMICRO") || response.contains("BTMINER")
    }

    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        (response.redirect_header.contains("https://") && response.status == 307)
            || response.body.contains("/cgi-bin/luci")
    }

    fn is_stock(&self) -> bool {
        true
    }
}

#[async_trait]
impl FirmwareEntry for WhatsMinerFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = WhatsMinerFirmware::get_model(ip).await?;
        let version = WhatsMinerFirmware::get_version(ip).await;
        let mut miner = crate::backends::WhatsMiner::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

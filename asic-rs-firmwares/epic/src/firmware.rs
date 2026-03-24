use std::{fmt, fmt::Display, net::IpAddr};

use asic_rs_core::{
    data::{command::MinerCommand, device::MinerHardware},
    discovery::HTTP_WEB_ROOT,
    errors::ModelSelectionError,
    traits::{
        discovery::DiscoveryCommands,
        entry::FirmwareEntry,
        firmware::MinerFirmware,
        identification::{FirmwareIdentification, WebResponse},
        make::MinerMake,
        miner::{Miner, MinerAuth, MinerConstructor},
        model::{MinerModel, UnknownMinerModel},
    },
};
use asic_rs_makes_antminer::{make::AntMinerMake, models::AntMinerModel};
use asic_rs_makes_epic::{make::EPicMake, models::EPicModel};
use asic_rs_makes_whatsminer::{make::WhatsMinerMake, models::WhatsMinerModel};
use async_trait::async_trait;

#[derive(Clone)]
pub enum EPicCompatibleModel {
    AntMiner(AntMinerModel),
    WhatsMiner(WhatsMinerModel),
    EPic(EPicModel),
    Unknown(UnknownMinerModel),
}

impl Display for EPicCompatibleModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AntMiner(m) => m.fmt(f),
            Self::WhatsMiner(m) => m.fmt(f),
            Self::EPic(m) => m.fmt(f),
            Self::Unknown(m) => m.fmt(f),
        }
    }
}

impl From<EPicCompatibleModel> for MinerHardware {
    fn from(model: EPicCompatibleModel) -> Self {
        match model {
            EPicCompatibleModel::AntMiner(m) => m.into(),
            EPicCompatibleModel::WhatsMiner(m) => m.into(),
            EPicCompatibleModel::EPic(m) => m.into(),
            EPicCompatibleModel::Unknown(m) => m.into(),
        }
    }
}

impl MinerModel for EPicCompatibleModel {
    fn make_name(&self) -> String {
        match self {
            Self::AntMiner(m) => m.make_name(),
            Self::WhatsMiner(m) => m.make_name(),
            Self::EPic(m) => m.make_name(),
            Self::Unknown(m) => m.make_name(),
        }
    }
}

#[derive(Default, Debug)]
pub struct EPicFirmware {}

impl Display for EPicFirmware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ePIC")
    }
}

impl DiscoveryCommands for EPicFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![HTTP_WEB_ROOT]
    }
}

#[async_trait]
impl MinerFirmware for EPicFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel, ModelSelectionError> {
        let url = format!("http://{}:4028/capabilities", ip);
        let response = reqwest::Client::new()
            .get(&url)
            .send()
            .await
            .map_err(|_| ModelSelectionError::NoModelResponse)?;

        let json_data = response
            .json::<serde_json::Value>()
            .await
            .map_err(|_| ModelSelectionError::UnexpectedModelResponse)?;

        let model_base = json_data["Model"]
            .as_str()
            .ok_or(ModelSelectionError::UnexpectedModelResponse)?;

        let model_upper = model_base.to_uppercase();
        let subtype = json_data["Model Subtype"]
            .as_str()
            .unwrap_or("")
            .to_uppercase();

        if model_upper == "UNDEFINED" {
            return Ok(EPicCompatibleModel::Unknown(UnknownMinerModel {
                name: model_base.to_string(),
            }));
        }
        if model_upper.starts_with("ANTMINER") {
            AntMinerMake::parse_model(model_upper.clone())
                .map(EPicCompatibleModel::AntMiner)
                .or(Ok(EPicCompatibleModel::Unknown(UnknownMinerModel {
                    name: model_upper,
                })))
        } else if model_upper.starts_with("WHATSMINER") {
            let base = model_upper.replace("WHATSMINER ", "");
            let mut model_str = format!("{}{}", base, subtype).replace("_", "");
            if !model_str.is_empty() {
                model_str.pop();
                model_str.push('0');
            }
            WhatsMinerMake::parse_model(model_str.clone())
                .map(EPicCompatibleModel::WhatsMiner)
                .or(Ok(EPicCompatibleModel::Unknown(UnknownMinerModel {
                    name: model_str,
                })))
        } else {
            EPicMake::parse_model(model_upper)
                .map(EPicCompatibleModel::EPic)
                .or(Ok(EPicCompatibleModel::Unknown(UnknownMinerModel {
                    name: model_base.to_string(),
                })))
        }
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let url = format!("http://{}:4028/summary", ip);
        let response = reqwest::Client::new().get(&url).send().await.ok()?;
        let json_data = response.json::<serde_json::Value>().await.ok()?;

        let fw_str = json_data["Software"].as_str()?;
        let version_str = fw_str.split_whitespace().last()?.trim_start_matches('v');
        semver::Version::parse(version_str).ok()
    }
}

impl FirmwareIdentification for EPicFirmware {
    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("Miner Web Dashboard")
    }
}

#[async_trait]
impl FirmwareEntry for EPicFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = EPicFirmware::get_model(ip).await?;
        let version = EPicFirmware::get_version(ip).await;
        let mut miner = crate::backends::PowerPlay::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

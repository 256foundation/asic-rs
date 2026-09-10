use std::{fmt, fmt::Display, net::IpAddr};

use asic_rs_core::traits::model::MinerModelAlgorithm;
use asic_rs_core::{
    data::{
        command::MinerCommand,
        device::{FirmwareType, HashAlgorithm, MinerHardware},
    },
    discovery::{HTTP_WEB_ROOT, RPC_VERSION},
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
    util,
};
use asic_rs_makes_antminer::{make::AntMinerMake, models::AntMinerModel};
use asic_rs_makes_braiins::{make::BraiinsMake, models::BraiinsModel};
use async_trait::async_trait;

#[derive(Clone)]
pub enum BraiinsCompatibleModel {
    AntMiner(AntMinerModel),
    Braiins(BraiinsModel),
    Unknown(UnknownMinerModel),
}

impl Display for BraiinsCompatibleModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AntMiner(m) => m.fmt(f),
            Self::Braiins(m) => m.fmt(f),
            Self::Unknown(m) => m.fmt(f),
        }
    }
}

impl From<BraiinsCompatibleModel> for MinerHardware {
    fn from(model: BraiinsCompatibleModel) -> Self {
        match model {
            BraiinsCompatibleModel::AntMiner(m) => m.into(),
            BraiinsCompatibleModel::Braiins(m) => m.into(),
            BraiinsCompatibleModel::Unknown(m) => m.into(),
        }
    }
}

impl MinerModel for BraiinsCompatibleModel {
    fn make_name(&self) -> String {
        match self {
            Self::AntMiner(m) => m.make_name(),
            Self::Braiins(m) => m.make_name(),
            Self::Unknown(m) => m.make_name(),
        }
    }
    fn is_known(&self) -> bool {
        match self {
            Self::AntMiner(m) => m.is_known(),
            Self::Braiins(m) => m.is_known(),
            Self::Unknown(m) => m.is_known(),
        }
    }
}

impl MinerModelAlgorithm for BraiinsCompatibleModel {
    fn hash_algorithm(&self) -> HashAlgorithm {
        match self {
            Self::AntMiner(m) => m.hash_algorithm(),
            Self::Braiins(m) => m.hash_algorithm(),
            Self::Unknown(m) => m.hash_algorithm(),
        }
    }
}

#[derive(Default, Debug)]
pub struct BraiinsFirmware {}

impl Display for BraiinsFirmware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", FirmwareType::Braiins)
    }
}

impl DiscoveryCommands for BraiinsFirmware {
    fn get_discovery_commands(&self) -> Vec<MinerCommand> {
        vec![RPC_VERSION, HTTP_WEB_ROOT]
    }
}

#[async_trait]
impl MinerFirmware for BraiinsFirmware {
    async fn get_model(ip: IpAddr) -> Result<impl MinerModel + Send, ModelSelectionError> {
        if let Some(json_data) =
            util::send_graphql_command(&ip, "{ bosminer { info { modelName } } }").await
            && let Some(model_str) = json_data["data"]["bosminer"]["info"]["modelName"].as_str()
        {
            let model = model_str
                .to_uppercase()
                .replace("BITMAIN ", "")
                .replace("S19XP", "S19 XP");

            let antminer = AntMinerMake::parse_model(model.clone())
                .ok()
                .filter(MinerModel::is_known)
                .map(BraiinsCompatibleModel::AntMiner);

            let braiins = BraiinsMake::parse_model(model.clone())
                .ok()
                .filter(MinerModel::is_known)
                .map(BraiinsCompatibleModel::Braiins);

            return Ok(antminer
                .or(braiins)
                .unwrap_or(BraiinsCompatibleModel::Unknown(UnknownMinerModel {
                    name: model,
                })));
        }

        Err(ModelSelectionError::NoModelResponse)
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        let response =
            util::send_graphql_command(&ip, "{ bos { info { version { full } } } }").await?;

        let full = response["data"]["bos"]["info"]["version"]["full"].as_str()?;

        crate::backends::util::parse_bos_version(full)
    }
}

impl FirmwareIdentification for BraiinsFirmware {
    fn identify_rpc(&self, response: &str) -> bool {
        response.contains("BOSMINER") || response.contains("BOSER")
    }

    fn identify_web(&self, response: &WebResponse<'_>) -> bool {
        response.body.contains("Braiins OS")
    }
}

#[async_trait]
impl FirmwareEntry for BraiinsFirmware {
    async fn build_miner(
        &self,
        ip: IpAddr,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let model = BraiinsFirmware::get_model(ip).await?;
        let version = BraiinsFirmware::get_version(ip).await;
        let mut miner = crate::backends::Braiins::new(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        Ok(miner)
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, sync::Arc};

    use anyhow::Context;
    use asic_rs_core::test::util::get_miner;

    use super::*;

    #[tokio::test]
    #[ignore = "requires live miner; set MINER_IP"]
    async fn parse_data_live_test_auto_detect() -> anyhow::Result<()> {
        let ip_str = std::env::var("MINER_IP").context("MINER_IP is not set")?;
        let ip =
            IpAddr::from_str(&ip_str).with_context(|| format!("invalid MINER_IP: {ip_str}"))?;

        let miner = get_miner(ip, Arc::new(BraiinsFirmware::default()))
            .await?
            .context("no miner detected at MINER_IP")?;
        let miner_data = miner.get_data().await;
        let mut miner_data_print = miner_data.clone();
        for hashboard in &mut miner_data_print.hashboards {
            hashboard.chips.clear();
        }
        println!("data {}", serde_json::to_string_pretty(&miner_data_print)?);

        println!(
            "pools {}",
            serde_json::to_string_pretty(&miner.get_pools_config().await?)?
        );

        assert_eq!(miner_data.ip, ip);
        assert!(miner_data.timestamp > 0);
        assert!(!miner_data.schema_version.is_empty());

        Ok(())
    }
}

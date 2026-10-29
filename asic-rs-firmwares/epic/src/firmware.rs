use std::{fmt, fmt::Display, net::IpAddr};

use asic_rs_core::traits::model::MinerModelAlgorithm;
use asic_rs_core::{
    data::{
        command::DiscoveryCommand,
        device::{HashAlgorithm, MinerHardware},
    },
    discovery::HTTP_WEB_ROOT,
    errors::ModelSelectionError,
    traits::{
        auth::HasAuth,
        discovery::DiscoveryCommands,
        entry::FirmwareEntry,
        firmware::MinerFirmware,
        identification::{FirmwareIdentification, WebResponse},
        make::MinerMake,
        miner::{Miner, MinerAuth},
        model::{MinerModel, UnknownMinerModel},
    },
    util::build_discovery_client,
};
use asic_rs_makes_antminer::{make::AntMinerMake, models::AntMinerModel};
use asic_rs_makes_epic::{make::EPicMake, models::EPicModel};
use asic_rs_makes_volcminer::{make::VolcMinerMake, models::VolcMinerModel};
use asic_rs_makes_whatsminer::{make::WhatsMinerMake, models::WhatsMinerModel};
use async_trait::async_trait;

#[derive(Clone)]
pub enum EPicCompatibleModel {
    AntMiner(AntMinerModel),
    VolcMiner(VolcMinerModel),
    WhatsMiner(WhatsMinerModel),
    EPic(EPicModel),
    Unknown(UnknownMinerModel),
}

impl Display for EPicCompatibleModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AntMiner(m) => m.fmt(f),
            Self::VolcMiner(m) => m.fmt(f),
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
            EPicCompatibleModel::VolcMiner(m) => m.into(),
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
            Self::VolcMiner(m) => m.make_name(),
            Self::WhatsMiner(m) => m.make_name(),
            Self::EPic(m) => m.make_name(),
            Self::Unknown(m) => m.make_name(),
        }
    }
    fn is_known(&self) -> bool {
        match self {
            Self::AntMiner(m) => m.is_known(),
            Self::VolcMiner(m) => m.is_known(),
            Self::WhatsMiner(m) => m.is_known(),
            Self::EPic(m) => m.is_known(),
            Self::Unknown(m) => m.is_known(),
        }
    }
}

impl MinerModelAlgorithm for EPicCompatibleModel {
    fn hash_algorithm(&self) -> HashAlgorithm {
        match self {
            Self::AntMiner(m) => m.hash_algorithm(),
            Self::VolcMiner(m) => m.hash_algorithm(),
            Self::WhatsMiner(m) => m.hash_algorithm(),
            Self::EPic(m) => m.hash_algorithm(),
            Self::Unknown(m) => m.hash_algorithm(),
        }
    }
}

#[derive(Default, Debug)]
pub struct EPicFirmware {}

impl Display for EPicFirmware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UMC OS")
    }
}

impl DiscoveryCommands for EPicFirmware {
    fn get_discovery_commands(&self) -> Vec<DiscoveryCommand> {
        vec![HTTP_WEB_ROOT]
    }
}

impl EPicFirmware {
    async fn get_capabilities(
        ip: IpAddr,
        port: u16,
    ) -> Result<serde_json::Value, ModelSelectionError> {
        build_discovery_client()?
            .get(format!("http://{ip}:{port}/capabilities"))
            .send()
            .await
            .map_err(|_| ModelSelectionError::NoModelResponse)?
            .json()
            .await
            .map_err(|_| ModelSelectionError::UnexpectedModelResponse)
    }

    fn model_from_capabilities(
        json_data: &serde_json::Value,
    ) -> Result<EPicCompatibleModel, ModelSelectionError> {
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
        } else if model_upper.starts_with("VOLCMINER") {
            VolcMinerMake::parse_model(model_upper.clone())
                .map(EPicCompatibleModel::VolcMiner)
                .or(Ok(EPicCompatibleModel::Unknown(UnknownMinerModel {
                    name: model_upper,
                })))
        } else if model_upper.starts_with("WHATSMINER") {
            let base = model_upper.replace("WHATSMINER ", "");
            let mut model_str = format!("{}{}", base, subtype).replace('_', "");
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

    async fn get_version_on_port(ip: IpAddr, port: u16) -> Option<semver::Version> {
        let url = format!("http://{ip}:{port}/summary");
        let response = build_discovery_client().ok()?.get(&url).send().await.ok()?;
        let json_data = response.json::<serde_json::Value>().await.ok()?;

        let fw_str = json_data["Software"].as_str()?;
        let version_str = fw_str.split_whitespace().last()?.trim_start_matches('v');
        semver::Version::parse(version_str).ok()
    }

    async fn build_miner_on_port(
        ip: IpAddr,
        port: u16,
        auth: Option<&MinerAuth>,
    ) -> Result<Box<dyn Miner>, ModelSelectionError> {
        let capabilities = Self::get_capabilities(ip, port).await?;
        let model = Self::model_from_capabilities(&capabilities)?;
        let uninstall_supported = capabilities["UninstallSupported"]
            .as_bool()
            .unwrap_or(false);
        let version = Self::get_version_on_port(ip, port).await;
        let mut miner = crate::backends::PowerPlay::build(ip, model, version);
        if let Some(auth) = auth {
            miner.set_auth(auth.clone());
        }
        // Reuse the discovery response so constructing the miner does not
        // initialize its persistent HTTP client or retain an idle connection.
        miner.set_restore_stock_os_support(uninstall_supported);
        Ok(Box::new(miner))
    }
}

#[async_trait]
impl MinerFirmware for EPicFirmware {
    type Model = EPicCompatibleModel;

    async fn get_model(ip: IpAddr) -> Result<Self::Model, ModelSelectionError> {
        Self::model_from_capabilities(&Self::get_capabilities(ip, 4028).await?)
    }

    async fn get_version(ip: IpAddr) -> Option<semver::Version> {
        Self::get_version_on_port(ip, 4028).await
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
        Self::build_miner_on_port(ip, 4028, auth).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::{Value, json};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        time::timeout,
    };

    use super::EPicFirmware;

    #[tokio::test]
    async fn discovery_reuses_capabilities_and_closes_connections() -> anyhow::Result<()> {
        for (support, expected) in [
            (Some(json!(true)), true),
            (Some(json!(false)), false),
            (None, false),
            (Some(Value::Null), false),
            (Some(json!("true")), false),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let mut capabilities = json!({ "Model": "Antminer S19 XP" });
            if let Some(support) = support {
                capabilities["UninstallSupported"] = support;
            }
            let server = tokio::spawn(async move {
                for (path, body) in [
                    ("/capabilities", capabilities),
                    ("/summary", json!({ "Software": "UMC OS v1.2.3" })),
                ] {
                    let (mut socket, _) =
                        timeout(Duration::from_secs(2), listener.accept()).await??;
                    let mut request = Vec::new();
                    let mut chunk = [0_u8; 1024];
                    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                        let count =
                            timeout(Duration::from_secs(2), socket.read(&mut chunk)).await??;
                        anyhow::ensure!(count > 0, "connection closed before request headers");
                        request.extend_from_slice(&chunk[..count]);
                    }
                    let request = String::from_utf8(request)?;
                    assert!(request.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
                    assert!(request.lines().any(|line| {
                        line.split_once(':').is_some_and(|(name, value)| {
                            name.eq_ignore_ascii_case("connection")
                                && value.trim().eq_ignore_ascii_case("close")
                        })
                    }));

                    // Offer keep-alive to verify discovery closes its side even
                    // when the server would allow the connection to be reused.
                    let body = body.to_string();
                    socket
                        .write_all(
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n{body}",
                                body.len()
                            )
                            .as_bytes(),
                        )
                        .await?;
                    let count = timeout(Duration::from_secs(2), socket.read(&mut chunk)).await??;
                    assert_eq!(count, 0, "discovery retained the {path} connection");
                }
                anyhow::Ok(())
            });

            let miner =
                EPicFirmware::build_miner_on_port(address.ip(), address.port(), None).await?;
            assert_eq!(miner.get_device_info().model, "S19XP");
            assert_eq!(miner.supports_restore_stock_os(), expected);
            server.await??;
        }
        Ok(())
    }
}

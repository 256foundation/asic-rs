use std::{net::IpAddr, time::Duration};

use anyhow::{Result, anyhow, bail};
use asic_rs_core::{
    config::pools::PoolConfig,
    data::command::MinerCommand,
    traits::miner::{APIClient, MinerAuth, WebAPIClient},
};
use async_trait::async_trait;
use diqwest::WithDigestAuth;
use once_cell::sync::OnceCell;
use reqwest::{Client, Method};
use serde_json::Value;
use tokio::time::sleep;

use super::{config_form, status_parser};

#[derive(Debug)]
pub struct VolcMinerWebAPI {
    ip: IpAddr,
    port: u16,
    client: OnceCell<Client>,
    timeout: Duration,
    auth: MinerAuth,
}

impl VolcMinerWebAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            ip,
            port: 80,
            client: OnceCell::new(),
            timeout: Duration::from_secs(10),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    #[cfg(test)]
    pub(super) fn auth(&self) -> &MinerAuth {
        &self.auth
    }

    fn build_client() -> Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| anyhow!("failed to create HTTP client: {e}"))
    }

    fn client(&self) -> Result<&Client> {
        self.client.get_or_try_init(Self::build_client)
    }

    fn url(&self, command: &str) -> String {
        format!("http://{}:{}/cgi-bin/{}.cgi", self.ip, self.port, command)
    }

    async fn request_text(
        &self,
        command: &str,
        method: Method,
        body: Option<String>,
    ) -> Result<String> {
        let client = self.client()?;
        let url = self.url(command);
        let mut builder = match method {
            Method::GET => client.get(url),
            Method::POST => client.post(url),
            _ => bail!("Unsupported HTTP method: {method}"),
        }
        .timeout(self.timeout);

        if let Some(body) = body {
            builder = builder
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body);
        }

        let response = builder
            .send_digest_auth((self.auth.username(), self.auth.password()))
            .await
            .map_err(|e| anyhow!("HTTP request failed: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            bail!("HTTP request failed with status: {status}");
        }

        response.text().await.map_err(|e| anyhow!(e.to_string()))
    }

    async fn request_json(&self, command: &str) -> Result<Value> {
        let text = self.request_text(command, Method::GET, None).await?;
        match serde_json::from_str(&text) {
            Ok(value) => Ok(value),
            Err(error) if command == "get_miner_status" => status_parser::parse_miner_status_text(&text)
                .map_err(|fallback_error| {
                    anyhow!(
                        "failed to parse {command} JSON: {error}; fallback parser failed: {fallback_error}"
                    )
                }),
            Err(error) => Err(anyhow!("failed to parse {command} JSON: {error}")),
        }
    }

    pub async fn miner_conf(&self) -> Result<Value> {
        self.request_json("get_miner_conf").await
    }

    pub async fn system_info(&self) -> Result<Value> {
        self.request_json("get_system_info").await
    }

    async fn conf_metadata(&self) -> config_form::MinerConfMetadata {
        let Ok(text) = self
            .request_text("get_miner_confV1", Method::GET, None)
            .await
        else {
            return config_form::MinerConfMetadata::default();
        };

        config_form::MinerConfMetadata {
            runmode: status_parser::extract_text_field(&text, "runmode")
                .unwrap_or_else(|| "0".to_string()),
            voltage: status_parser::extract_text_field(&text, "voltage")
                .unwrap_or_else(|| "1260".to_string()),
            debug_enabled: status_parser::extract_text_field(&text, "bb_debug_enable")
                .map(|v| v == "true")
                .unwrap_or(false),
        }
    }

    async fn confirm_pools_config(&self, pools: &[PoolConfig]) -> bool {
        for _ in 0..5 {
            if self
                .miner_conf()
                .await
                .map(|config| config_form::pools_match_config(&config, pools))
                .unwrap_or(false)
            {
                return true;
            }
            sleep(Duration::from_secs(1)).await;
        }

        false
    }

    pub async fn set_pools_config(&self, pools: &[PoolConfig]) -> Result<bool> {
        let current = self.miner_conf().await?;
        let metadata = self.conf_metadata().await;
        let body = config_form::build_miner_conf_body(&current, &metadata, pools);

        if let Err(error) = self
            .request_text("set_miner_conf", Method::POST, Some(body))
            .await
            && !self.confirm_pools_config(pools).await
        {
            return Err(error);
        }

        Ok(true)
    }
}

#[async_trait]
impl APIClient for VolcMinerWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command,
                parameters: None,
            } => self.send_command(command, false, None, Method::GET).await,
            MinerCommand::WebAPI {
                parameters: Some(_),
                ..
            } => bail!("VolcMiner WebAPI commands do not support parameters"),
            _ => Err(anyhow!("Unsupported command type for VolcMiner API")),
        }
    }
}

#[async_trait]
impl WebAPIClient for VolcMinerWebAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        if parameters.is_some() {
            bail!("VolcMiner WebAPI commands do not support parameters");
        }

        match method {
            Method::GET => self.request_json(command).await,
            _ => bail!("Unsupported VolcMiner command method: {method}"),
        }
    }
}

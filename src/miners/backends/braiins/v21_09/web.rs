use async_trait::async_trait;
use reqwest::{Client, Method};
use serde_json::Value;
use std::net::IpAddr;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

#[derive(Debug)]
pub struct BraiinsWebAPI {
    client: Client,
    ip: IpAddr,
    port: u16,
    timeout: Duration,
    session_id: RwLock<Option<String>>,
    username: String,
    password: String,
}

impl BraiinsWebAPI {
    pub fn new(ip: IpAddr) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            ip,
            port: 80,
            timeout: Duration::from_secs(5),
            session_id: RwLock::new(None),
            username: "root".to_string(),
            password: String::new(),
        }
    }

    async fn authenticate(&self) -> anyhow::Result<String> {
        let url = format!("http://{}:{}/cgi-bin/luci", self.ip, self.port);
        let body = format!(
            "luci_username={}&luci_password={}",
            self.username, self.password
        );

        let response = self
            .client
            .post(&url)
            .header("User-Agent", "BTC Tools v0.1")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Luci auth failed: {}", e))?;

        for cookie in response.headers().get_all("set-cookie") {
            if let Ok(cookie_str) = cookie.to_str() {
                for part in cookie_str.split(';') {
                    let part = part.trim();
                    if let Some(value) = part.strip_prefix("session_id=")
                        && !value.is_empty()
                    {
                        return Ok(value.to_string());
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Failed to obtain Luci session cookie"))
    }

    async fn ensure_authenticated(&self) -> anyhow::Result<()> {
        if self.session_id.read().await.is_some() {
            return Ok(());
        }

        let session = self.authenticate().await?;
        *self.session_id.write().await = Some(session);
        Ok(())
    }

    pub async fn send_luci_command(&self, command: &str) -> anyhow::Result<Value> {
        self.ensure_authenticated().await?;

        let url = format!("http://{}:{}/cgi-bin/luci/{}", self.ip, self.port, command);

        let mut request = self
            .client
            .get(&url)
            .header("User-Agent", "BTC Tools v0.1")
            .timeout(self.timeout);

        if let Some(ref session) = *self.session_id.read().await {
            request = request.header("Cookie", format!("session_id={}", session));
        }

        let response = request
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Luci command failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Luci HTTP error: {}", response.status()));
        }

        response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Luci parse error: {}", e))
    }
}

#[async_trait]
impl WebAPIClient for BraiinsWebAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        _parameters: Option<Value>,
        _method: Method,
    ) -> anyhow::Result<Value> {
        self.send_luci_command(command).await
    }
}

#[async_trait]
impl APIClient for BraiinsWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI { command, .. } => self.send_luci_command(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for Luci web client"
            )),
        }
    }
}

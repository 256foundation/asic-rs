use std::{net::IpAddr, time::Duration};

use once_cell::sync::OnceCell;

use asic_rs_core::{data::command::MinerCommand, traits::miner::*};
use async_trait::async_trait;
use reqwest::{Client, Method};
use serde_json::Value;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct BraiinsWebAPI {
    client: OnceCell<Client>,
    ip: IpAddr,
    port: u16,
    timeout: Duration,
    session_id: RwLock<Option<String>>,
    auth: MinerAuth,
}

impl BraiinsWebAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            client: OnceCell::new(),
            ip,
            port: 80,
            timeout: Duration::from_secs(5),
            session_id: RwLock::new(None),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
        *self.session_id.get_mut() = None;
    }

    fn build_client() -> anyhow::Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| anyhow::anyhow!("failed to create HTTP client: {e}"))
    }

    fn client(&self) -> anyhow::Result<&Client> {
        self.client.get_or_try_init(Self::build_client)
    }

    async fn authenticate(&self) -> anyhow::Result<String> {
        let url = format!("http://{}:{}/cgi-bin/luci", self.ip, self.port);
        let client = self.client()?;
        let body = format!(
            "luci_username={}&luci_password={}",
            self.auth.username,
            self.auth.password.expose_secret()
        );

        let response = client
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
        let client = self.client()?;

        let mut request = client
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

use std::{net::IpAddr, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use asic_rs_core::{
    data::command::MinerCommand,
    traits::{
        auth::{ExposeSecret, MinerAuth},
        miner::{APIClient, WebAPIClient},
    },
};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use reqwest::{Client, Method, Response, StatusCode};
use serde_json::{Value, json};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ProtoWebAPI {
    ip: IpAddr,
    port: u16,
    client: OnceCell<Client>,
    timeout: Duration,
    token: RwLock<Option<String>>,
    auth: MinerAuth,
}

impl ProtoWebAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            ip,
            port: 8080,
            client: OnceCell::new(),
            timeout: Duration::from_secs(5),
            token: RwLock::new(None),
            auth,
        }
    }

    fn build_client() -> Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| anyhow!("failed to create HTTP client: {e}"))
    }

    fn client(&self) -> Result<&Client> {
        self.client.get_or_try_init(Self::build_client)
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
        *self.token.get_mut() = None;
    }

    fn endpoint_url(&self, command: &str) -> String {
        let endpoint = command.trim_start_matches('/');
        format!("http://{}:{}/{}", self.ip, self.port, endpoint)
    }

    async fn clear_token(&self) {
        *self.token.write().await = None;
    }

    async fn ensure_authenticated(&self) -> Result<()> {
        if self.token.read().await.is_some() {
            return Ok(());
        }

        let token = self.authenticate().await?;
        *self.token.write().await = Some(token);
        Ok(())
    }

    async fn authenticate(&self) -> Result<String> {
        let payload = json!({
            "password": self.auth.password.expose_secret(),
        });
        let response = self
            .client()?
            .post(self.endpoint_url("/api/v1/auth/login"))
            .json(&payload)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        if !response.status().is_success() {
            bail!("Login failed with status {}", response.status());
        }

        let body = response
            .json::<Value>()
            .await
            .map_err(|e| anyhow!("Failed to parse login JSON: {e}"))?;

        body.get("access_token")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .context("missing access_token in login response")
    }

    async fn execute_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
        token: Option<String>,
    ) -> Result<Response> {
        let client = self.client()?;

        // Read-only GET requests carry their parameters as URL query params;
        // writable requests send them as a JSON body.
        let mut request_builder = match *method {
            Method::GET => {
                let mut parsed = reqwest::Url::parse(url).map_err(|e| anyhow!(e.to_string()))?;
                if let Some(params) = parameters.as_ref().and_then(Value::as_object) {
                    let mut pairs = parsed.query_pairs_mut();
                    for (key, value) in params {
                        let rendered = match value {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        pairs.append_pair(key, &rendered);
                    }
                }
                client.get(parsed)
            }
            Method::POST | Method::PUT => {
                let mut request_builder = if *method == Method::POST {
                    client.post(url)
                } else {
                    client.put(url)
                };
                if let Some(payload) = parameters {
                    request_builder = request_builder.json(&payload);
                }
                request_builder
            }
            _ => bail!("Unsupported method: {method}"),
        };

        if let Some(token) = token {
            request_builder = request_builder.bearer_auth(token);
        }

        request_builder
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Reboot the rig.
    pub async fn reboot(&self) -> Result<Value> {
        self.send_command("/api/v1/system/reboot", false, None, Method::POST)
            .await
    }

    /// Start mining.
    pub async fn mining_start(&self) -> Result<Value> {
        self.send_command("/api/v1/mining/start", false, None, Method::POST)
            .await
    }

    /// Stop mining.
    pub async fn mining_stop(&self) -> Result<Value> {
        self.send_command("/api/v1/mining/stop", false, None, Method::POST)
            .await
    }

    /// Set the mining power target, in watts.
    pub async fn set_miner_target(&self, power_target_watts: u64) -> Result<Value> {
        self.send_command(
            "/api/v1/mining/target",
            false,
            Some(json!({ "power_target_watts": power_target_watts })),
            Method::PUT,
        )
        .await
    }

    /// Update the cooling configuration.
    pub async fn set_cooling(&self, payload: Value) -> Result<Value> {
        self.send_command("/api/v1/cooling", false, Some(payload), Method::PUT)
            .await
    }

    /// Replace the configured pools. Accepts an array of pool configs.
    pub async fn set_pools(&self, payload: Value) -> Result<Value> {
        self.send_command("/api/v1/pools", false, Some(payload), Method::POST)
            .await
    }

    /// Flash the control-board locate LED to help find the miner. The LED
    /// turns itself off after the API's default duration; there is no way to
    /// turn it off early, and requests made while it is already lit are ignored.
    pub async fn locate(&self) -> Result<Value> {
        self.send_command("/api/v1/system/locate", false, None, Method::POST)
            .await
    }

    /// Set the device password.
    pub async fn set_password(&self, password: &str) -> Result<bool> {
        self.send_command(
            "/api/v1/auth/password",
            false,
            Some(json!({ "password": password })),
            Method::PUT,
        )
        .await?;
        Ok(true)
    }

    /// Read system logs, returning the log lines joined by newlines.
    pub async fn read_logs(&self) -> Result<String> {
        let response = self
            .send_command("/api/v1/system/logs", false, None, Method::GET)
            .await?;
        let logs = response
            .pointer("/logs/content")
            .and_then(Value::as_array)
            .map(|lines| {
                lines
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        Ok(logs)
    }
}

#[async_trait]
impl WebAPIClient for ProtoWebAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        self.ensure_authenticated().await?;
        let url = self.endpoint_url(command);
        let mut attempt = 0;

        loop {
            attempt += 1;
            let token = self.token.read().await.clone();
            let response = self
                .execute_request(&url, &method, parameters.clone(), token)
                .await?;

            if response.status() == StatusCode::UNAUTHORIZED && attempt == 1 {
                self.clear_token().await;
                self.ensure_authenticated().await?;
                continue;
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                bail!("HTTP request failed with status code {status}: {body}");
            }

            return response
                .json::<Value>()
                .await
                .map_err(|e| anyhow!("Failed to parse JSON response: {e}"));
        }
    }
}

#[async_trait]
impl APIClient for ProtoWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command,
                parameters,
            } => {
                self.send_command(command, false, parameters.clone(), Method::GET)
                    .await
            }
            _ => Err(anyhow!("Unsupported command type for Proto WebAPI")),
        }
    }
}

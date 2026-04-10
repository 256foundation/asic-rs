use std::{net::IpAddr, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use asic_rs_core::{data::command::MinerCommand, traits::miner::*};
use async_trait::async_trait;
use reqwest::{Client, Method, Response, StatusCode};
use serde_json::{Map, Value, json};
use tokio::sync::RwLock;

use super::status::StatusFromAuradineV1;

#[derive(Debug)]
pub struct AuradineWebAPI {
    ip: IpAddr,
    port: u16,
    client: Client,
    timeout: Duration,
    token: RwLock<Option<String>>,
    auth: MinerAuth,
}

impl AuradineWebAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            ip,
            port: 8080,
            client,
            timeout: Duration::from_secs(5),
            token: RwLock::new(None),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
        *self.token.get_mut() = None;
    }

    fn endpoint_url(&self, command: &str) -> String {
        let endpoint = command.trim_start_matches('/');
        format!("http://{}:{}/{}", self.ip, self.port, endpoint)
    }

    fn build_post_payload(command: &str, parameters: Option<Value>) -> Value {
        match parameters {
            Some(Value::Object(mut map)) => {
                map.entry("command".to_string())
                    .or_insert(Value::String(command.to_string()));
                Value::Object(map)
            }
            Some(value) => json!({
                "command": command,
                "parameter": value,
            }),
            None => {
                let mut map = Map::new();
                map.insert("command".to_string(), Value::String(command.to_string()));
                Value::Object(map)
            }
        }
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
        let url = self.endpoint_url("token");
        let payload = json!({
            "command": "token",
            "user": self.auth.username,
            "password": self.auth.password.expose_secret(),
        });

        let response = self
            .client
            .post(url)
            .json(&payload)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        if !response.status().is_success() {
            bail!(
                "Token request failed with status code {}",
                response.status()
            );
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse token response JSON: {e}"))?;

        let status = data.status_from_auradine_v1()?;
        status.into_result()?;

        data.pointer("/Token/0/Token")
            .and_then(|t| t.as_str())
            .map(String::from)
            .context("missing /Token/0/Token in token response")
    }

    async fn execute_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
        token: Option<String>,
    ) -> Result<Response> {
        let mut request_builder = match *method {
            Method::GET => self.client.get(url),
            Method::POST => {
                let payload = parameters.unwrap_or_else(|| json!({}));
                self.client.post(url).json(&payload)
            }
            _ => bail!("Unsupported method: {}", method),
        };

        if let Some(token) = token {
            request_builder = request_builder.header("Token", token);
        }

        let response = request_builder
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        Ok(response)
    }
}

#[async_trait]
impl APIClient for AuradineWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command,
                parameters,
            } => {
                self.send_command(command, false, parameters.clone(), Method::GET)
                    .await
            }
            _ => Err(anyhow!("Unsupported command type for Auradine WebAPI")),
        }
    }
}

#[async_trait]
impl WebAPIClient for AuradineWebAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        self.ensure_authenticated().await?;

        let mut attempt = 0;
        let url = self.endpoint_url(command);

        loop {
            attempt += 1;
            let token = self.token.read().await.clone();
            let payload = if method == Method::POST {
                Some(Self::build_post_payload(command, parameters.clone()))
            } else {
                None
            };

            let response = self.execute_request(&url, &method, payload, token).await?;

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

            let data: Value = response
                .json()
                .await
                .map_err(|e| anyhow!("Failed to parse JSON response: {e}"))?;

            let status = data.status_from_auradine_v1()?;
            if let Err(err) = status.into_result() {
                let msg = err.to_string().to_ascii_uppercase();
                if attempt == 1
                    && (msg.contains("TOKEN")
                        || msg.contains("SESSION")
                        || msg.contains("EXPIRED")
                        || msg.contains("UNAUTHORIZED"))
                {
                    self.clear_token().await;
                    self.ensure_authenticated().await?;
                    continue;
                }
                return Err(anyhow!(err.to_string()));
            }

            return Ok(data);
        }
    }
}

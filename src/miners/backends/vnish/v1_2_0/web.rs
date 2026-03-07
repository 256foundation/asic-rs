use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;
use reqwest::{Client, Method, Response, StatusCode};
use serde_json::Value;
use std::{net::IpAddr, time::Duration};
use tokio::sync::RwLock;

use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

pub struct VnishWebAPI {
    client: Client,
    pub ip: IpAddr,
    port: u16,
    timeout: Duration,
    bearer_token: RwLock<Option<String>>,
    password: Option<String>,
}

impl std::fmt::Debug for VnishWebAPI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VnishWebAPI")
            .field("ip", &self.ip)
            .field("port", &self.port)
            .field("timeout", &self.timeout)
            .field("bearer_token", &"<redacted>")
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl VnishWebAPI {
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
            bearer_token: RwLock::new(None),
            password: Some("admin".to_string()),
        }
    }

    pub fn with_auth(ip: IpAddr, password: String) -> Self {
        let mut client = Self::new(ip);
        client.password = Some(password);
        client
    }

    fn web_url(&self, command: &str) -> String {
        format!("http://{}:{}/api/v1/{}", self.ip, self.port, command)
    }

    async fn send_web_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        let response = self
            .execute_web_request(&self.web_url(command), &method, parameters)
            .await?;

        if response.status().is_success() {
            return response.json().await.map_err(|e| anyhow!(e.to_string()));
        }

        bail!("HTTP request failed with status code {}", response.status());
    }

    async fn authenticate(&self, password: &str) -> Result<String> {
        let unlock_payload = serde_json::json!({ "pw": password });
        let url = self.web_url("unlock");

        let response = self
            .client
            .post(&url)
            .json(&unlock_payload)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        if !response.status().is_success() {
            bail!("VNish authentication failed");
        }

        let unlock_response: Value = response.json().await.map_err(|e| anyhow!(e.to_string()))?;

        unlock_response
            .pointer("/token")
            .and_then(|t| t.as_str())
            .map(String::from)
            .ok_or_else(|| anyhow!("VNish token missing from unlock response"))
    }

    async fn get_auth_token(&self, force_refresh: bool) -> Result<String> {
        if !force_refresh
            && let Some(token) = self.bearer_token.read().await.clone()
        {
            return Ok(token);
        }

        let Some(password) = self.password.clone() else {
            bail!("VNish unlock password is not configured");
        };

        let token = self.authenticate(&password).await?;
        *self.bearer_token.write().await = Some(token.clone());
        Ok(token)
    }

    async fn execute_web_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> Result<Response> {
        let first_token = if self.password.is_some() {
            Some(self.get_auth_token(false).await?)
        } else {
            None
        };

        let mut response = self
            .execute_web_request_once(url, method, parameters.clone(), first_token.as_deref())
            .await?;

        if matches!(response.status(), StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN)
            && self.password.is_some()
        {
            let refreshed = self.get_auth_token(true).await?;
            response = self
                .execute_web_request_once(url, method, parameters, Some(refreshed.as_str()))
                .await?;
        }

        Ok(response)
    }

    async fn execute_web_request_once(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
        bearer_token: Option<&str>,
    ) -> Result<Response> {
        let mut request_builder = match *method {
            Method::GET => self.client.get(url),
            Method::POST => {
                let mut builder = self.client.post(url);
                if let Some(params) = parameters {
                    builder = builder.json(&params);
                }
                builder
            }
            Method::PATCH => {
                let mut builder = self.client.patch(url);
                if let Some(params) = parameters {
                    builder = builder.json(&params);
                }
                builder
            }
            Method::PUT => {
                let mut builder = self.client.put(url);
                if let Some(params) = parameters {
                    builder = builder.json(&params);
                }
                builder
            }
            _ => bail!("Unsupported method: {}", method),
        }
        .timeout(self.timeout);

        if let Some(token) = bearer_token {
            request_builder = request_builder.header("Authorization", format!("Bearer {token}"));
        }

        request_builder
            .send()
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    async fn response_body(response: Response) -> String {
        response.text().await.unwrap_or_default()
    }

    async fn send_action_command(
        &self,
        command: &str,
        parameters: Option<Value>,
        method: Method,
        action: &str,
    ) -> Result<()> {
        let response = self
            .execute_web_request(&self.web_url(command), &method, parameters)
            .await?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = Self::response_body(response).await;
        bail!("VNish {action} failed: HTTP {status}: {body}");
    }

    async fn get_find_miner_state(&self) -> Result<Option<bool>> {
        let json = self
            .send_web_command("status", false, None, Method::GET)
            .await?;
        let v = json.pointer("/find_miner");

        if let Some(b) = v.and_then(|vv| vv.as_bool()) {
            return Ok(Some(b));
        }

        Ok(v.and_then(|vv| vv.as_i64()).map(|n| n != 0))
    }

    pub async fn blink(&self, blink: bool) -> Result<()> {
        // Some builds treat this command like a toggle, so check state first.
        let current = self
            .get_find_miner_state()
            .await
            .map_err(|e| anyhow!("VNish blink failed to read current state: {e}"))?;
        let Some(current) = current else {
            bail!("VNish blink failed to read current state");
        };

        if current == blink {
            return Ok(());
        }

        self.send_action_command(
            "find-miner",
            Some(serde_json::json!({ "blink": blink })),
            Method::POST,
            "blink",
        )
        .await
    }

    pub async fn set_fault_light(&self, fault: bool) -> Result<bool> {
        self.blink(fault).await?;
        Ok(true)
    }

    pub async fn restart_mining(&self) -> Result<()> {
        self.send_action_command("mining/restart", None, Method::POST, "restart")
            .await
    }

    pub async fn reboot(&self) -> Result<()> {
        self.send_action_command("system/reboot", None, Method::POST, "reboot")
            .await
    }

    pub async fn restart(&self) -> Result<bool> {
        self.restart_mining().await?;
        Ok(true)
    }

    pub async fn set_pools(&self, pools: Vec<Value>) -> Result<bool> {
        let payload = serde_json::json!({ "miner": { "pools": pools } });
        let url = self.web_url("settings");

        let mut response = self
            .execute_web_request(&url, &Method::POST, Some(payload.clone()))
            .await?;

        if response.status() == StatusCode::METHOD_NOT_ALLOWED {
            response = self
                .execute_web_request(&url, &Method::PUT, Some(payload))
                .await?;
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = Self::response_body(response).await;
            bail!("VNish set pools failed: HTTP {status}: {body}");
        }

        let json: Value = match response.json().await {
            Ok(v) => v,
            Err(_) => return Ok(true),
        };

        let reboot_required = json
            .pointer("/reboot_required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let restart_required = json
            .pointer("/restart_required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if reboot_required {
            self.reboot().await?;
            return Ok(true);
        }

        if restart_required {
            self.restart_mining().await?;
        }

        Ok(true)
    }

    pub async fn stop_mining(&self) -> Result<()> {
        self.send_action_command("mining/stop", None, Method::POST, "stop")
            .await
    }

    pub async fn pause(&self) -> Result<bool> {
        self.stop_mining().await?;
        Ok(true)
    }

    pub async fn start_mining(&self) -> Result<()> {
        self.send_action_command("mining/start", None, Method::POST, "start")
            .await
    }

    pub async fn resume(&self) -> Result<bool> {
        self.start_mining().await?;
        Ok(true)
    }
}

#[async_trait]
impl APIClient for VnishWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command,
                parameters,
            } => self
                .send_web_command(command, false, parameters.clone(), Method::GET)
                .await
                .map_err(|e| anyhow!(e.to_string())),
            _ => Err(anyhow!("Unsupported command type for Web client")),
        }
    }
}

#[async_trait]
impl WebAPIClient for VnishWebAPI {
    async fn send_command(
        &self,
        command: &str,
        privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        self.send_web_command(command, privileged, parameters, method)
            .await
    }
}

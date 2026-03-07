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

    async fn send_web_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        let response = self
            .execute_with_auth_fallback(
                &self.web_url(command),
                &method,
                parameters,
                self.password.is_some(),
            )
            .await?;

        let status = response.status();
        if status.is_success() {
            let json_data = response
                .json()
                .await
                .map_err(|e| VnishError::ParseError(e.to_string()))?;
            Ok(json_data)
        } else {
            Err(VnishError::HttpError(status.as_u16()).into())
        }
    }

    fn is_timeout_error(err: &anyhow::Error) -> bool {
        if err
            .downcast_ref::<VnishError>()
            .is_some_and(|e| matches!(e, VnishError::Timeout))
        {
            return true;
        }

        err.downcast_ref::<reqwest::Error>()
            .is_some_and(|e| e.is_timeout())
    }

    fn web_url(&self, command: &str) -> String {
        format!("http://{}:{}/api/v1/{}", self.ip, self.port, command)
    }

    async fn execute_with_auth_fallback(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
        should_auth: bool,
    ) -> Result<Response> {
        let use_auth = if should_auth {
            match self.ensure_authenticated().await {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!(
                        "VNish auth setup failed before request, trying without auth: {e}"
                    );
                    false
                }
            }
        } else {
            false
        };

        match self
            .execute_web_request(url, method, parameters.clone(), use_auth)
            .await
        {
            Ok(response)
                if should_auth
                    && matches!(
                        response.status(),
                        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
                    ) =>
            {
                self.retry_after_unauthorized(url, method, parameters).await
            }
            Ok(response) => Ok(response),
            Err(_) if should_auth => {
                self.retry_after_error_with_auth(url, method, parameters)
                    .await
            }
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    async fn retry_after_unauthorized(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> Result<Response> {
        *self.bearer_token.write().await = None;

        if self.ensure_authenticated().await.is_ok() {
            match self
                .execute_web_request(url, method, parameters.clone(), true)
                .await
            {
                Ok(r)
                    if !matches!(r.status(), StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) =>
                {
                    Ok(r)
                }
                _ => self
                    .execute_web_request(url, method, parameters, false)
                    .await
                    .map_err(anyhow::Error::from),
            }
        } else {
            self.execute_web_request(url, method, parameters, false)
                .await
                .map_err(anyhow::Error::from)
        }
    }

    async fn retry_after_error_with_auth(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> Result<Response> {
        *self.bearer_token.write().await = None;

        if let Err(e) = self.ensure_authenticated().await {
            tracing::warn!("VNish re-auth failed, trying without auth: {e}");
            return self
                .execute_web_request(url, method, parameters, false)
                .await
                .map_err(anyhow::Error::from);
        }

        let response = match self
            .execute_web_request(url, method, parameters.clone(), true)
            .await
        {
            Ok(response)
                if !matches!(
                    response.status(),
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
                ) =>
            {
                response
            }
            Ok(_) | Err(_) => {
                self.execute_web_request(url, method, parameters, false)
                    .await?
            }
        };

        Ok(response)
    }

    pub async fn blink(&self, blink: bool) -> Result<()> {
        // Some builds treat this like a toggle, so check first.
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

        self.send_control_command(
            "find-miner",
            Some(serde_json::json!({ "blink": blink })),
            "blink",
        )
        .await
    }

    pub async fn set_fault_light(&self, fault: bool) -> Result<bool> {
        self.blink(fault).await?;
        Ok(true)
    }

    async fn send_control_command(
        &self,
        command: &str,
        parameters: Option<Value>,
        action: &str,
    ) -> Result<()> {
        let Some(password) = self.password.clone() else {
            bail!("VNish unlock password is not configured");
        };

        let url = self.web_url(command);

        let response = match self
            .authenticate(&password)
            .await
            .map_err(|e| anyhow!(e.to_string()))
        {
            Ok(token) => {
                *self.bearer_token.write().await = Some(token);
                match self
                    .try_post_then_get_with_auth(&url, parameters.clone(), None)
                    .await
                {
                    Ok(response)
                        if !matches!(
                            response.status(),
                            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
                        ) =>
                    {
                        response
                    }
                    Ok(_) | Err(_) => {
                        *self.bearer_token.write().await = None;
                        let token = self
                            .authenticate(&password)
                            .await
                            .map_err(|e| anyhow!(e.to_string()))?;
                        *self.bearer_token.write().await = Some(token);
                        self.try_post_then_get_with_auth(&url, parameters, None)
                            .await?
                    }
                }
            }
            Err(error) => return Err(error),
        };

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = Self::response_body_lower(response).await;
        bail!("VNish {action} failed: HTTP {status}: {body}");
    }

    async fn get_find_miner_state(&self) -> Result<Option<bool>> {
        let url = self.web_url("status");

        let unauth_resp = self
            .execute_web_request(&url, &Method::GET, None, false)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        let resp = match unauth_resp {
            r if r.status() != StatusCode::UNAUTHORIZED && r.status() != StatusCode::FORBIDDEN => r,
            _ => {
                let Some(password) = self.password.clone() else {
                    return Ok(None);
                };

                let token = self
                    .authenticate(&password)
                    .await
                    .map_err(|e| anyhow!(e.to_string()))?;
                *self.bearer_token.write().await = Some(token);

                self.execute_web_request(&url, &Method::GET, None, true)
                    .await
                    .map_err(|e| anyhow!(e.to_string()))?
            }
        };

        if !resp.status().is_success() {
            return Ok(None);
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| anyhow!("VNish status parse failed: {e}"))?;
        let v = json.pointer("/find_miner");

        if let Some(b) = v.and_then(|vv| vv.as_bool()) {
            return Ok(Some(b));
        }

        Ok(v.and_then(|vv| vv.as_i64()).map(|n| n != 0))
    }

    pub async fn restart_mining(&self) -> Result<()> {
        self.send_control_command("mining/restart", None, "restart")
            .await
    }

    pub async fn reboot(&self) -> Result<()> {
        self.send_control_command("system/reboot", None, "reboot")
            .await
    }

    pub async fn restart(&self) -> Result<bool> {
        self.restart_mining().await?;
        Ok(true)
    }

    pub async fn set_pools(&self, pools: Vec<Value>) -> Result<bool> {
        let payload = serde_json::json!({ "miner": { "pools": pools } });

        // Some builds take longer to apply settings.
        let long_timeout = self.timeout.checked_mul(12).unwrap_or(self.timeout);

        let mut response = match self
            .api_settings_call_with_auth_retry(
                "settings",
                payload.clone(),
                Method::POST,
                long_timeout,
            )
            .await
        {
            Ok(r) => r,
            Err(e) if Self::is_timeout_error(&e) => {
                bail!("VNish set pools timed out; apply result is unknown: {e}");
            }
            Err(e) => return Err(e),
        };

        if response.status() == StatusCode::METHOD_NOT_ALLOWED {
            response = match self
                .api_settings_call_with_auth_retry("settings", payload, Method::PUT, long_timeout)
                .await
            {
                Ok(r) => r,
                Err(e) if Self::is_timeout_error(&e) => {
                    bail!("VNish set pools timed out (PUT fallback); apply result is unknown: {e}");
                }
                Err(e) => return Err(e),
            };
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = Self::response_body_lower(response).await;
            bail!("VNish set pools failed: HTTP {status}: {body}");
        }

        let json: Value = match response.json().await {
            Ok(v) => v,
            Err(_) => {
                // Some builds return an empty body here.
                return Ok(true);
            }
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
        self.send_control_command("mining/stop", None, "stop").await
    }

    pub async fn pause(&self) -> Result<bool> {
        self.stop_mining().await?;
        Ok(true)
    }

    pub async fn start_mining(&self) -> Result<()> {
        self.send_control_command("mining/start", None, "start")
            .await
    }

    pub async fn resume(&self) -> Result<bool> {
        self.start_mining().await?;
        Ok(true)
    }

    async fn try_post_then_get_with_auth(
        &self,
        url: &str,
        post_parameters: Option<Value>,
        get_parameters: Option<Value>,
    ) -> Result<Response> {
        let post_resp = self
            .execute_web_request(url, &Method::POST, post_parameters, true)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        if post_resp.status().is_success() {
            return Ok(post_resp);
        }

        if !matches!(
            post_resp.status(),
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_FOUND
        ) {
            return Ok(post_resp);
        }

        let get_resp = self
            .execute_web_request(url, &Method::GET, get_parameters, true)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        Ok(get_resp)
    }

    async fn response_body_lower(response: Response) -> String {
        response.text().await.unwrap_or_default().to_lowercase()
    }

    /// Make sure an auth token is ready when auth is enabled.
    async fn ensure_authenticated(&self) -> Result<(), VnishError> {
        if self.bearer_token.read().await.is_none() && self.password.is_some() {
            if let Some(ref password) = self.password {
                match self.authenticate(password).await {
                    Ok(token) => {
                        *self.bearer_token.write().await = Some(token);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                Err(VnishError::AuthenticationFailed)
            }
        } else {
            Ok(())
        }
    }

    async fn authenticate(&self, password: &str) -> Result<String, VnishError> {
        let unlock_payload = serde_json::json!({ "pw": password });
        let url = self.web_url("unlock");

        let response = self
            .client
            .post(&url)
            .json(&unlock_payload)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| VnishError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(VnishError::AuthenticationFailed);
        }

        let unlock_response: Value = response
            .json()
            .await
            .map_err(|e| VnishError::ParseError(e.to_string()))?;

        unlock_response
            .pointer("/token")
            .and_then(|t| t.as_str())
            .map(String::from)
            .ok_or(VnishError::AuthenticationFailed)
    }

    async fn execute_web_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
        include_auth: bool,
    ) -> Result<Response, VnishError> {
        self.execute_web_request_with_timeout(url, method, parameters, include_auth, self.timeout)
            .await
    }

    async fn execute_web_request_with_timeout(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
        include_auth: bool,
        timeout: Duration,
    ) -> Result<Response, VnishError> {
        let request_builder = match *method {
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
            _ => return Err(VnishError::UnsupportedMethod(method.to_string())),
        };

        let mut request_builder = request_builder.timeout(timeout);

        if include_auth {
            let token = self.bearer_token.read().await.clone();
            let Some(token) = token else {
                return Err(VnishError::MissingAuthToken);
            };

            request_builder = request_builder.header("Authorization", format!("Bearer {token}"));
        }

        let request = request_builder
            .build()
            .map_err(|e| VnishError::RequestError(e.to_string()))?;

        let response = self.client.execute(request).await.map_err(|e| {
            if e.is_timeout() {
                VnishError::Timeout
            } else {
                VnishError::NetworkError(e.to_string())
            }
        })?;

        Ok(response)
    }

    async fn api_settings_call_with_auth_retry(
        &self,
        command: &str,
        parameters: Value,
        method: Method,
        timeout: Duration,
    ) -> Result<Response> {
        let Some(password) = self.password.clone() else {
            bail!("VNish unlock password is not configured");
        };

        let url = self.web_url(command);

        let token = self
            .authenticate(&password)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;
        *self.bearer_token.write().await = Some(token);

        match self
            .execute_web_request_with_timeout(
                &url,
                &method,
                Some(parameters.clone()),
                true,
                timeout,
            )
            .await
        {
            Ok(response)
                if !matches!(
                    response.status(),
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
                ) =>
            {
                Ok(response)
            }
            Ok(_) | Err(_) => {
                *self.bearer_token.write().await = None;
                let token = self
                    .authenticate(&password)
                    .await
                    .map_err(|e| anyhow!(e.to_string()))?;
                *self.bearer_token.write().await = Some(token);
                self.execute_web_request_with_timeout(
                    &url,
                    &method,
                    Some(parameters),
                    true,
                    timeout,
                )
                .await
                .map_err(anyhow::Error::from)
            }
        }
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

#[derive(Debug, Clone)]
pub enum VnishError {
    NetworkError(String),
    HttpError(u16),
    ParseError(String),
    RequestError(String),
    Timeout,
    UnsupportedMethod(String),
    AuthenticationFailed,
    MissingAuthToken,
}

impl std::fmt::Display for VnishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VnishError::NetworkError(msg) => write!(f, "Network error: {msg}"),
            VnishError::HttpError(code) => write!(f, "HTTP error: {code}"),
            VnishError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            VnishError::RequestError(msg) => write!(f, "Request error: {msg}"),
            VnishError::Timeout => write!(f, "Request timeout"),
            VnishError::UnsupportedMethod(method) => write!(f, "Unsupported method: {method}"),
            VnishError::AuthenticationFailed => write!(f, "Authentication failed"),
            VnishError::MissingAuthToken => write!(f, "Missing auth token"),
        }
    }
}

impl std::error::Error for VnishError {}

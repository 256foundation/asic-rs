use async_trait::async_trait;
use reqwest::{Client, Method, Response, StatusCode};
use serde_json::Value;
use std::{net::IpAddr, time::Duration};
use tokio::sync::RwLock;

use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

/// VNish WebAPI client
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

#[async_trait]
impl APIClient for VnishWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command,
                parameters,
            } => self
                .send_command(command, false, parameters.clone(), Method::GET)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string())),
            _ => Err(anyhow::anyhow!("Cannot send non web command to web API")),
        }
    }
}

#[async_trait]
impl WebAPIClient for VnishWebAPI {
    /// Send a command to the Vnish miner API
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> anyhow::Result<Value> {
        let should_auth = self.password.is_some();

        let url = self.build_api_url(command);

        let response = self
            .execute_with_auth_fallback(&url, &method, parameters, should_auth)
            .await?;

        let status = response.status();
        if status.is_success() {
            let json_data = response
                .json()
                .await
                .map_err(|e| VnishError::ParseError(e.to_string()))?;
            Ok(json_data)
        } else {
            Err(VnishError::HttpError(status.as_u16()))?
        }
    }
}

impl VnishWebAPI {
    fn is_timeout_error(err: &anyhow::Error) -> bool {
        // Prefer our own typed timeout signal.
        if err
            .downcast_ref::<VnishError>()
            .is_some_and(|e| matches!(e, VnishError::Timeout))
        {
            return true;
        }

        // Fallback: if the raw client error makes it through, treat an actual timeout as
        // "applied but no response".
        err.downcast_ref::<reqwest::Error>()
            .is_some_and(|e| e.is_timeout())
    }

    fn build_api_url(&self, command: &str) -> String {
        format!("http://{}:{}/api/v1/{}", self.ip, self.port, command)
    }

    async fn execute_with_auth_fallback(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
        should_auth: bool,
    ) -> anyhow::Result<Response> {
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
            .execute_request(url, method, parameters.clone(), use_auth, self.timeout)
            .await
        {
            Ok(resp)
                if should_auth
                    && matches!(
                        resp.status(),
                        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
                    ) =>
            {
                self.retry_after_unauthorized(url, method, parameters).await
            }
            Ok(resp) => Ok(resp),
            Err(_) if should_auth => {
                self.retry_after_error_with_auth(url, method, parameters)
                    .await
            }
            Err(e) => Err(anyhow::anyhow!(e.to_string())),
        }
    }

    async fn retry_after_unauthorized(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> anyhow::Result<Response> {
        // Token might be stale; try a fresh auth once before falling back to unauth.
        *self.bearer_token.write().await = None;

        if self.ensure_authenticated().await.is_ok() {
            match self
                .execute_request(url, method, parameters.clone(), true, self.timeout)
                .await
            {
                Ok(r)
                    if !matches!(r.status(), StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) =>
                {
                    Ok(r)
                }
                _ => self
                    .execute_request(url, method, parameters, false, self.timeout)
                    .await
                    .map_err(anyhow::Error::from),
            }
        } else {
            self.execute_request(url, method, parameters, false, self.timeout)
                .await
                .map_err(anyhow::Error::from)
        }
    }

    async fn retry_after_error_with_auth(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> anyhow::Result<Response> {
        // If the request failed while auth is enabled, try a fresh login and retry with auth
        // before falling back to a no-auth retry.
        // Force a fresh login attempt.
        *self.bearer_token.write().await = None;

        if let Err(e) = self.ensure_authenticated().await {
            tracing::warn!("VNish re-auth failed, trying without auth: {e}");
            return self
                .execute_request(url, method, parameters, false, self.timeout)
                .await
                .map_err(anyhow::Error::from);
        }

        let resp = match self
            .execute_request(url, method, parameters.clone(), true, self.timeout)
            .await
        {
            Ok(resp)
                if !matches!(
                    resp.status(),
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
                ) =>
            {
                resp
            }
            Ok(_) | Err(_) => {
                self.execute_request(url, method, parameters, false, self.timeout)
                    .await?
            }
        };

        Ok(resp)
    }

    /// Create a new Vnish WebAPI client
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

    /// Create a new Vnish WebAPI client with a custom unlock password.
    ///
    /// VNish's Web UI/API typically uses only a password (no username).
    pub fn with_auth(ip: IpAddr, password: String) -> Self {
        let mut client = Self::new(ip);
        client.password = Some(password);
        client
    }

    pub async fn blink(&self, blink: bool) -> anyhow::Result<()> {
        // VNish's find-miner endpoint behaves like a toggle on some builds.
        // To keep on/off buttons from acting like "toggle", check current state first.

        let current = self
            .get_find_miner_state()
            .await
            .map_err(|e| anyhow::anyhow!("VNish blink failed to read current state: {e}"))?;
        let Some(current) = current else {
            anyhow::bail!("VNish blink failed to read current state");
        };

        if current == blink {
            return Ok(());
        }

        let payload = serde_json::json!({ "blink": blink });
        let api_url = format!("http://{}:{}/api/v1/find-miner", self.ip, self.port);

        let resp = self
            .api_action_call_with_auth_retry(&api_url, Some(payload), None)
            .await?;

        if resp.status().is_success() {
            return Ok(());
        }

        anyhow::bail!("VNish blink failed: HTTP {}", resp.status());
    }

    async fn get_find_miner_state(&self) -> anyhow::Result<Option<bool>> {
        let status_url = format!("http://{}:{}/api/v1/status", self.ip, self.port);

        let unauth_resp = self
            .execute_request(&status_url, &Method::GET, None, false, self.timeout)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let resp = match unauth_resp {
            r if r.status() != StatusCode::UNAUTHORIZED && r.status() != StatusCode::FORBIDDEN => r,
            _ => {
                let Some(password) = self.password.clone() else {
                    return Ok(None);
                };

                let token = self
                    .authenticate(&password)
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                *self.bearer_token.write().await = Some(token);

                self.execute_request(&status_url, &Method::GET, None, true, self.timeout)
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
            }
        };

        if !resp.status().is_success() {
            return Ok(None);
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("VNish status parse failed: {e}"))?;
        let v = json.pointer("/find_miner");

        if let Some(b) = v.and_then(|vv| vv.as_bool()) {
            return Ok(Some(b));
        }

        Ok(v.and_then(|vv| vv.as_i64()).map(|n| n != 0))
    }

    pub async fn restart_mining(&self) -> anyhow::Result<()> {
        // Use the same auth pattern as `blink()`:
        // try unauthenticated first; if that fails due to auth, unlock and retry.

        let restart_url = format!("http://{}:{}/api/v1/mining/restart", self.ip, self.port);
        let restart_resp = self.api_action_call_with_auth(&restart_url).await?;

        if restart_resp.status().is_success() {
            return Ok(());
        }

        let status = restart_resp.status();
        let body = Self::response_body_lower(restart_resp).await;
        anyhow::bail!("VNish restart failed: HTTP {status}: {body}");
    }

    pub async fn reboot(&self) -> anyhow::Result<()> {
        let reboot_url = format!("http://{}:{}/api/v1/system/reboot", self.ip, self.port);
        let reboot_resp = self.api_action_call_with_auth(&reboot_url).await?;

        if reboot_resp.status().is_success() {
            return Ok(());
        }

        let status = reboot_resp.status();
        let body = Self::response_body_lower(reboot_resp).await;
        anyhow::bail!("VNish reboot failed: HTTP {status}: {body}");
    }

    pub async fn set_pools(&self, pools: Vec<Value>) -> anyhow::Result<bool> {
        let url = format!("http://{}:{}/api/v1/settings", self.ip, self.port);

        let payload = serde_json::json!({ "miner": { "pools": pools } });

        // Some builds take a while to apply settings.
        let long_timeout = self.timeout.checked_mul(12).unwrap_or(self.timeout);

        let mut resp = match self
            .api_settings_call_with_auth_retry(&url, payload.clone(), Method::POST, long_timeout)
            .await
        {
            Ok(r) => r,
            Err(e) if Self::is_timeout_error(&e) => {
                anyhow::bail!("VNish set pools timed out; apply result is unknown: {e}");
            }
            Err(e) => return Err(e),
        };

        if resp.status() == StatusCode::METHOD_NOT_ALLOWED {
            resp = match self
                .api_settings_call_with_auth_retry(&url, payload, Method::PUT, long_timeout)
                .await
            {
                Ok(r) => r,
                Err(e) if Self::is_timeout_error(&e) => {
                    anyhow::bail!(
                        "VNish set pools timed out (PUT fallback); apply result is unknown: {e}"
                    );
                }
                Err(e) => return Err(e),
            };
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = Self::response_body_lower(resp).await;
            anyhow::bail!("VNish set pools failed: HTTP {status}: {body}");
        }

        let json: Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => {
                // Some firmware returns an empty body.
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

    pub async fn stop_mining(&self) -> anyhow::Result<()> {
        // VNish calls this "stop". Same flow as `blink()` / `restart_mining()`:
        // try without auth first; if that doesn't work, unlock and retry.
        // Also: POST first, then GET fallback.

        let stop_url = format!("http://{}:{}/api/v1/mining/stop", self.ip, self.port);
        let stop_resp = self.api_action_call_with_auth(&stop_url).await?;

        if stop_resp.status().is_success() {
            return Ok(());
        }

        let status = stop_resp.status();
        let body = Self::response_body_lower(stop_resp).await;
        anyhow::bail!("VNish stop failed: HTTP {status}: {body}");
    }

    pub async fn start_mining(&self) -> anyhow::Result<()> {
        // VNish calls this "start". Same flow as `blink()` / `restart_mining()`:
        // try without auth first; if that doesn't work, unlock and retry.
        // Also: POST first, then GET fallback.

        let start_url = format!("http://{}:{}/api/v1/mining/start", self.ip, self.port);
        let start_resp = self.api_action_call_with_auth(&start_url).await?;

        if start_resp.status().is_success() {
            return Ok(());
        }

        let status = start_resp.status();
        let body = Self::response_body_lower(start_resp).await;
        anyhow::bail!("VNish start failed: HTTP {status}: {body}");
    }
    async fn try_unauth_requests(
        &self,
        url: &str,
        post_parameters: Option<Value>,
        get_parameters: Option<Value>,
    ) -> anyhow::Result<Option<Response>> {
        // Try unauth first.
        let unauth_resp = self
            .execute_request(
                url,
                &Method::POST,
                post_parameters.clone(),
                false,
                self.timeout,
            )
            .await;

        match unauth_resp {
            Ok(resp)
                if resp.status() != StatusCode::UNAUTHORIZED
                    && resp.status() != StatusCode::FORBIDDEN =>
            {
                if resp.status().is_success() {
                    return Ok(Some(resp));
                }

                // Only fall back to GET for endpoint/method mismatch.
                if !matches!(
                    resp.status(),
                    StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_FOUND
                ) {
                    return Ok(Some(resp));
                }

                // POST failed, try GET (still unauth).
                let get_resp = self
                    .execute_request(
                        url,
                        &Method::GET,
                        get_parameters.clone(),
                        false,
                        self.timeout,
                    )
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;

                if get_resp.status().is_success() {
                    return Ok(Some(get_resp));
                }

                if get_resp.status() != StatusCode::UNAUTHORIZED
                    && get_resp.status() != StatusCode::FORBIDDEN
                {
                    return Ok(Some(get_resp));
                }
                // 401/403 on GET should continue to unlock+retry path.
                Ok(None)
            }
            Ok(_) => Ok(None),
            Err(e) => Err(anyhow::anyhow!(e.to_string())),
        }
    }

    async fn try_with_token(
        &self,
        url: &str,
        post_parameters: Option<Value>,
        get_parameters: Option<Value>,
    ) -> anyhow::Result<Option<Response>> {
        // Try the current token first, and only unlock again after a 401/403.
        if self.bearer_token.read().await.is_none() {
            return Ok(None);
        }

        let post_resp = self
            .execute_request(
                url,
                &Method::POST,
                post_parameters.clone(),
                true,
                self.timeout,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if post_resp.status().is_success() {
            return Ok(Some(post_resp));
        }

        if !matches!(
            post_resp.status(),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ) {
            // Only fall back to GET for endpoint/method mismatch.
            if !matches!(
                post_resp.status(),
                StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_FOUND
            ) {
                return Ok(Some(post_resp));
            }

            let get_resp = self
                .execute_request(
                    url,
                    &Method::GET,
                    get_parameters.clone(),
                    true,
                    self.timeout,
                )
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            if get_resp.status().is_success() {
                return Ok(Some(get_resp));
            }

            if !matches!(
                get_resp.status(),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
            ) {
                return Ok(Some(get_resp));
            }
            // 401/403 on GET should continue to unlock+retry path.
        }

        Ok(None)
    }

    async fn authenticate_and_set_token(&self, password: &str) -> anyhow::Result<()> {
        let token = self
            .authenticate(password)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        *self.bearer_token.write().await = Some(token);
        Ok(())
    }

    async fn try_post_then_get_with_auth(
        &self,
        url: &str,
        post_parameters: Option<Value>,
        get_parameters: Option<Value>,
    ) -> anyhow::Result<Response> {
        let post_resp = self
            .execute_request(url, &Method::POST, post_parameters, true, self.timeout)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if post_resp.status().is_success() {
            return Ok(post_resp);
        }

        // Only fall back to GET for endpoint/method mismatch.
        if !matches!(
            post_resp.status(),
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_FOUND
        ) {
            return Ok(post_resp);
        }

        // POST failed, try GET with auth.
        let get_resp = self
            .execute_request(url, &Method::GET, get_parameters, true, self.timeout)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        Ok(get_resp)
    }

    async fn api_action_call_with_auth_retry(
        &self,
        url: &str,
        post_parameters: Option<Value>,
        get_parameters: Option<Value>,
    ) -> anyhow::Result<Response> {
        if let Some(resp) = self
            .try_unauth_requests(url, post_parameters.clone(), get_parameters.clone())
            .await?
        {
            return Ok(resp);
        }

        if let Some(resp) = self
            .try_with_token(url, post_parameters.clone(), get_parameters.clone())
            .await?
        {
            return Ok(resp);
        }

        let Some(password) = self.password.clone() else {
            anyhow::bail!("VNish unlock password is not configured");
        };

        self.authenticate_and_set_token(&password).await?;

        self.try_post_then_get_with_auth(url, post_parameters, get_parameters)
            .await
    }

    async fn api_action_call_with_auth(&self, url: &str) -> anyhow::Result<Response> {
        self.api_action_call_with_auth_retry(url, None, None).await
    }

    async fn response_body_lower(response: Response) -> String {
        response.text().await.unwrap_or_default().to_lowercase()
    }

    /// Ensure authentication token is present, authenticate if needed
    async fn ensure_authenticated(&self) -> anyhow::Result<(), VnishError> {
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

    async fn authenticate(&self, password: &str) -> anyhow::Result<String, VnishError> {
        let unlock_payload = serde_json::json!({ "pw": password });
        let url = format!("http://{}:{}/api/v1/unlock", self.ip, self.port);

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

    /// Execute the actual HTTP request
    async fn execute_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
        include_auth: bool,
        timeout: Duration,
    ) -> anyhow::Result<Response, VnishError> {
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

        // Add authentication headers if requested
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
        url: &str,
        parameters: Value,
        method: Method,
        timeout: Duration,
    ) -> anyhow::Result<Response> {
        let should_auth = self.password.is_some();

        // Try without auth first.
        let unauth_resp = self
            .execute_request(url, &method, Some(parameters.clone()), false, timeout)
            .await
            .map_err(anyhow::Error::from)?;

        if unauth_resp.status().is_success() {
            return Ok(unauth_resp);
        }

        if should_auth
            && matches!(
                unauth_resp.status(),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
            )
        {
            let Some(password) = self.password.clone() else {
                anyhow::bail!("VNish unlock password is not configured");
            };

            let token = self
                .authenticate(&password)
                .await
                .map_err(anyhow::Error::from)?;
            *self.bearer_token.write().await = Some(token);

            return self
                .execute_request(url, &method, Some(parameters), true, timeout)
                .await
                .map_err(anyhow::Error::from);
        }

        Ok(unauth_resp)
    }
}

/// Error types for Vnish WebAPI operations
#[derive(Debug, Clone)]
pub enum VnishError {
    /// Network error (connection issues, DNS resolution, etc.)
    NetworkError(String),
    /// HTTP error with status code
    HttpError(u16),
    /// JSON parsing error
    ParseError(String),
    /// Request building error
    RequestError(String),
    /// Timeout error
    Timeout,
    /// Unsupported HTTP method
    UnsupportedMethod(String),
    /// Authentication failed
    AuthenticationFailed,
    /// Auth was requested but no bearer token is set
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

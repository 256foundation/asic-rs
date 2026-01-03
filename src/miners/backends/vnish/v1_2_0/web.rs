use anyhow;
use async_trait::async_trait;
use reqwest::{Client, Method, Response};
use serde_json::Value;
use std::{net::IpAddr, time::Duration};
use tokio::sync::RwLock;

use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

/// VNish WebAPI client
#[derive(Debug)]
pub struct VnishWebAPI {
    client: Client,
    pub ip: IpAddr,
    port: u16,
    timeout: Duration,
    bearer_token: RwLock<Option<String>>,
    password: Option<String>,
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
        // Ensure we're authenticated before making the request
        if let Err(e) = self.ensure_authenticated().await {
            return Err(anyhow::anyhow!("Failed to authenticate: {}", e));
        }

        let url = format!("http://{}:{}/api/v1/{}", self.ip, self.port, command);

        let response = self.execute_request(&url, &method, parameters).await?;

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
    /// Create a new Vnish WebAPI client
    pub fn new(ip: IpAddr, port: u16) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            ip,
            port,
            timeout: Duration::from_secs(5),
            bearer_token: RwLock::new(None),
            password: Some("admin".to_string()), // Default password
        }
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
            _ => return Err(VnishError::UnsupportedMethod(method.to_string())),
        };

        let mut request_builder = request_builder.timeout(self.timeout);

        // Add authentication headers if provided
        if let Some(ref token) = *self.bearer_token.read().await {
            request_builder = request_builder.header("Authorization", format!("Bearer {token}"));
        }

        let request = request_builder
            .build()
            .map_err(|e| VnishError::RequestError(e.to_string()))?;

        let response = self
            .client
            .execute(request)
            .await
            .map_err(|e| VnishError::NetworkError(e.to_string()))?;

        Ok(response)
    }
}

/// Error types for Vnish WebAPI operations
#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    /// Maximum retries exceeded
    MaxRetriesExceeded,
    /// Authentication failed
    AuthenticationFailed,
    /// Unauthorized (401)
    Unauthorized,
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
            VnishError::MaxRetriesExceeded => write!(f, "Maximum retries exceeded"),
            VnishError::AuthenticationFailed => write!(f, "Authentication failed"),
            VnishError::Unauthorized => write!(f, "Unauthorized access"),
        }
    }
}

impl std::error::Error for VnishError {}

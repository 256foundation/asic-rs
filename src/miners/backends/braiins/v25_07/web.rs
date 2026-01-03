use anyhow;
use async_trait::async_trait;
use reqwest::{Client, Method, Response};
use serde_json::Value;
use std::{net::IpAddr, time::Duration};
use tokio::sync::RwLock;

use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

/// Braiins WebAPI client
#[derive(Debug)]
pub struct BraiinsWebAPI {
    client: Client,
    pub ip: IpAddr,
    port: u16,
    timeout: Duration,
    bearer_token: RwLock<Option<String>>,
    username: Option<String>,
    password: Option<String>,
}

#[async_trait]
impl APIClient for BraiinsWebAPI {
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
impl WebAPIClient for BraiinsWebAPI {
    /// Send a command to the Braiins miner API
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
                .map_err(|e| BraiinsError::ParseError(e.to_string()))?;
            Ok(json_data)
        } else {
            Err(BraiinsError::HttpError(status.as_u16()))?
        }
    }
}

impl BraiinsWebAPI {
    /// Create a new Braiins WebAPI client
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
            username: Some("root".to_string()), // Default user
            password: Some("root".to_string()), // Default password
        }
    }

    /// Ensure authentication token is present, authenticate if needed
    async fn ensure_authenticated(&self) -> anyhow::Result<(), BraiinsError> {
        if self.bearer_token.read().await.is_some() {
            return Ok(());
        }

        let password = self
            .password
            .as_ref()
            .ok_or(BraiinsError::AuthenticationFailed)?;

        let token = self.authenticate(password).await?;
        *self.bearer_token.write().await = Some(token);

        Ok(())
    }
    async fn authenticate(&self, password: &str) -> anyhow::Result<String, BraiinsError> {
        let unlock_payload = serde_json::json!({ "password": password, "username": "root" });
        let url = format!("http://{}:{}/api/v1/auth/login", self.ip, self.port);

        let response = self
            .client
            .post(&url)
            .json(&unlock_payload)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| BraiinsError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(BraiinsError::AuthenticationFailed);
        }

        let unlock_response: Value = response
            .json()
            .await
            .map_err(|e| BraiinsError::ParseError(e.to_string()))?;

        unlock_response
            .pointer("/token")
            .and_then(|t| t.as_str())
            .map(String::from)
            .ok_or(BraiinsError::AuthenticationFailed)
    }

    /// Execute the actual HTTP request
    async fn execute_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> anyhow::Result<Response, BraiinsError> {
        let request_builder = match *method {
            Method::GET => self.client.get(url),
            Method::POST => {
                let mut builder = self.client.post(url);
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
            Method::PATCH => {
                let mut builder = self.client.patch(url);
                if let Some(params) = parameters {
                    builder = builder.json(&params);
                }
                builder
            }
            _ => return Err(BraiinsError::UnsupportedMethod(method.to_string())),
        };

        let mut request_builder = request_builder.timeout(self.timeout);

        // Add authentication headers if provided
        if let Some(ref token) = *self.bearer_token.read().await {
            request_builder = request_builder.header("Authorization", token.to_string());
        }

        let request = request_builder
            .build()
            .map_err(|e| BraiinsError::RequestError(e.to_string()))?;

        let response = self
            .client
            .execute(request)
            .await
            .map_err(|e| BraiinsError::NetworkError(e.to_string()))?;

        Ok(response)
    }
}

/// Error types for Braiins WebAPI operations
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum BraiinsError {
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

impl std::fmt::Display for BraiinsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BraiinsError::NetworkError(msg) => write!(f, "Network error: {msg}"),
            BraiinsError::HttpError(code) => write!(f, "HTTP error: {code}"),
            BraiinsError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            BraiinsError::RequestError(msg) => write!(f, "Request error: {msg}"),
            BraiinsError::Timeout => write!(f, "Request timeout"),
            BraiinsError::UnsupportedMethod(method) => write!(f, "Unsupported method: {method}"),
            BraiinsError::MaxRetriesExceeded => write!(f, "Maximum retries exceeded"),
            BraiinsError::AuthenticationFailed => write!(f, "Authentication failed"),
            BraiinsError::Unauthorized => write!(f, "Unauthorized access"),
        }
    }
}

impl std::error::Error for BraiinsError {}

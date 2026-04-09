use std::{net::IpAddr, time::Duration};

use anyhow::{self, Context, bail};
use asic_rs_core::{
    data::{command::MinerCommand, firmware::FirmwareImage},
    traits::miner::*,
};
use async_trait::async_trait;
use reqwest::{Client, Method, Response, header, multipart};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tracing::warn;

/// ePIC PowerPlay WebAPI client
#[derive(Debug)]
pub struct PowerPlayWebAPI {
    client: Client,
    pub ip: IpAddr,
    port: u16,
    timeout: Duration,
    auth: MinerAuth,
}

#[async_trait]
impl APIClient for PowerPlayWebAPI {
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
impl WebAPIClient for PowerPlayWebAPI {
    /// Send a command to the EPic miner API
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> anyhow::Result<Value> {
        let url = format!("http://{}:{}/{}", self.ip, self.port, command);

        let response = self
            .execute_request(&url, &method, parameters.clone())
            .await?;

        let status = response.status();
        if status.is_success() {
            let json_data = response
                .json()
                .await
                .map_err(|e| PowerPlayError::ParseError(e.to_string()))?;
            Ok(json_data)
        } else {
            Err(PowerPlayError::HttpError(status.as_u16()))?
        }
    }
}

impl PowerPlayWebAPI {
    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }

    /// Create a new EPic WebAPI client
    pub fn new(ip: IpAddr, port: u16, auth: MinerAuth) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            ip,
            port,
            timeout: Duration::from_secs(5),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    pub async fn upgrade_firmware(&self, image: FirmwareImage) -> anyhow::Result<bool> {
        let url = format!("http://{}:{}{}", self.ip, self.port, "/systemupdate");
        let FirmwareImage { filename, bytes } = image;
        let checksum = Self::sha256_hex(&bytes);

        let form = multipart::Form::new()
            .text("password", self.auth.password.expose_secret().to_string())
            .text("checksum", checksum)
            .text("keepsettings", "true")
            .part(
                "update.zip",
                multipart::Part::bytes(bytes)
                    .file_name(filename)
                    .mime_str("application/zip")
                    .context("failed to set firmware part mime type")?,
            );

        let response = self
            .client
            .post(url)
            .header(header::ACCEPT, "application/json")
            .timeout(self.timeout.max(Duration::from_secs(300)))
            .multipart(form)
            .send()
            .await
            .context("firmware upload HTTP request failed")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("failed to read firmware upload response body")?;

        if !status.is_success() {
            bail!(
                "Firmware upload failed with status code {}: {}",
                status,
                body
            );
        }

        let payload: Value = serde_json::from_str(&body).with_context(|| {
            format!(
                "Invalid {} response body from {}: {}",
                "/systemupdate", self.ip, body
            )
        })?;
        let result = payload
            .get("result")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if !result && let Some(error) = payload.get("error").and_then(Value::as_str) {
            warn!(
                miner_ip = %self.ip,
                endpoint = "/systemupdate",
                error = error,
                "ePIC firmware update API returned result=false"
            );
        }

        Ok(result)
    }

    /// Execute the actual HTTP request
    async fn execute_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> anyhow::Result<Response, PowerPlayError> {
        let request_builder = match *method {
            Method::GET => self.client.get(url),
            Method::POST => self.client.post(url).json(&{
                let mut p = parameters.unwrap_or_else(|| json!({}));
                p.as_object_mut().map(|m| {
                    m.insert(
                        "password".into(),
                        Value::String(self.auth.password.expose_secret().to_string()),
                    )
                });
                p
            }),
            _ => return Err(PowerPlayError::UnsupportedMethod(method.to_string())),
        };

        let request_builder = request_builder.timeout(self.timeout);

        let request = request_builder
            .build()
            .map_err(|e| PowerPlayError::RequestError(e.to_string()))?;

        let response = self
            .client
            .execute(request)
            .await
            .map_err(|e| PowerPlayError::NetworkError(e.to_string()))?;

        Ok(response)
    }
}

/// Error types for EPic WebAPI operations
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PowerPlayError {
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

impl std::fmt::Display for PowerPlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerPlayError::NetworkError(msg) => write!(f, "Network error: {msg}"),
            PowerPlayError::HttpError(code) => write!(f, "HTTP error: {code}"),
            PowerPlayError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            PowerPlayError::RequestError(msg) => write!(f, "Request error: {msg}"),
            PowerPlayError::Timeout => write!(f, "Request timeout"),
            PowerPlayError::UnsupportedMethod(method) => write!(f, "Unsupported method: {method}"),
            PowerPlayError::MaxRetriesExceeded => write!(f, "Maximum retries exceeded"),
            PowerPlayError::AuthenticationFailed => write!(f, "Authentication failed"),
            PowerPlayError::Unauthorized => write!(f, "Unauthorized access"),
        }
    }
}

impl std::error::Error for PowerPlayError {}

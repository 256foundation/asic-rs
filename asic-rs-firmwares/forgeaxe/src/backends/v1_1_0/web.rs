use std::{net::IpAddr, time::Duration};

use once_cell::sync::OnceCell;

use anyhow;
use asic_rs_core::{
    data::command::MinerCommand,
    traits::miner::{APIClient, WebAPIClient},
};
use async_trait::async_trait;
use reqwest::{Client, Method, Response};
use serde_json::Value;
use tokio::time::timeout;

/// Forgeaxe WebAPI client for communicating with ForgeOS miners
#[derive(Debug)]
pub struct ForgeaxeWebAPI {
    client: OnceCell<Client>,
    pub ip: IpAddr,
    port: u16,
    timeout: Duration,
    retries: u32,
}

#[async_trait]
#[allow(dead_code)]
trait Forgeaxe110WebAPI: WebAPIClient {
    /// Get system information
    async fn system_info(&self) -> anyhow::Result<Value> {
        self.send_command("system/info", false, None, Method::GET)
            .await
    }

    /// Get swarm information
    async fn swarm_info(&self) -> anyhow::Result<Value> {
        self.send_command("swarm/info", false, None, Method::GET)
            .await
    }

    /// Restart the system
    async fn restart(&self) -> anyhow::Result<Value> {
        self.send_command("system/restart", false, None, Method::POST)
            .await
    }

    /// Update system settings
    async fn update_settings(&self, config: Value) -> anyhow::Result<Value> {
        self.send_command("system", false, Some(config), Method::PATCH)
            .await
    }
}

#[async_trait]
impl APIClient for ForgeaxeWebAPI {
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
impl WebAPIClient for ForgeaxeWebAPI {
    /// Send a command to the miner
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> anyhow::Result<Value> {
        let url = format!("http://{}:{}/api/{}", self.ip, self.port, command);

        for attempt in 0..=self.retries {
            let result = self
                .execute_request(&url, &method, parameters.clone())
                .await;

            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json().await {
                            Ok(json_data) => return Ok(json_data),
                            Err(e) => {
                                if attempt == self.retries {
                                    Err(ForgeaxeError::ParseError(e.to_string()))?;
                                }
                            }
                        }
                    } else if attempt == self.retries {
                        Err(ForgeaxeError::HttpError(response.status().as_u16()))?;
                    }
                }
                Err(e) => {
                    if attempt == self.retries {
                        Err(e)?;
                    }
                }
            }
        }

        Err(ForgeaxeError::MaxRetriesExceeded)?
    }
}

impl Forgeaxe110WebAPI for ForgeaxeWebAPI {}

impl ForgeaxeWebAPI {
    /// Create a new Forgeaxe WebAPI client
    pub fn new(ip: IpAddr, port: u16) -> Self {
        Self {
            client: OnceCell::new(),
            ip,
            port,
            timeout: Duration::from_secs(5),
            retries: 1,
        }
    }

    fn build_client() -> Result<Client, ForgeaxeError> {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| ForgeaxeError::RequestError(format!("failed to create HTTP client: {e}")))
    }

    fn client(&self) -> Result<&Client, ForgeaxeError> {
        self.client.get_or_try_init(Self::build_client)
    }

    /// Execute the actual HTTP request
    async fn execute_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> anyhow::Result<Response, ForgeaxeError> {
        let client = self.client()?;

        let request_builder = match *method {
            Method::GET => client.get(url),
            Method::POST => {
                let mut builder = client.post(url);
                if let Some(params) = parameters {
                    builder = builder.json(&params);
                }
                builder
            }
            Method::PATCH => {
                let mut builder = client.patch(url);
                if let Some(params) = parameters {
                    builder = builder.json(&params);
                }
                builder
            }
            _ => return Err(ForgeaxeError::UnsupportedMethod(method.to_string())),
        };

        let request = request_builder
            .timeout(self.timeout)
            .build()
            .map_err(|e| ForgeaxeError::RequestError(e.to_string()))?;

        let response = timeout(self.timeout, client.execute(request))
            .await
            .map_err(|_| ForgeaxeError::Timeout)?
            .map_err(|e| ForgeaxeError::NetworkError(e.to_string()))?;
        Ok(response)
    }
}

/// Error types for Forgeaxe WebAPI operations
#[derive(Debug, Clone)]
pub enum ForgeaxeError {
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
}

impl std::fmt::Display for ForgeaxeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForgeaxeError::NetworkError(msg) => write!(f, "Network error: {msg}"),
            ForgeaxeError::HttpError(code) => write!(f, "HTTP error: {code}"),
            ForgeaxeError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            ForgeaxeError::RequestError(msg) => write!(f, "Request error: {msg}"),
            ForgeaxeError::Timeout => write!(f, "Request timeout"),
            ForgeaxeError::UnsupportedMethod(method) => write!(f, "Unsupported method: {method}"),
            ForgeaxeError::MaxRetriesExceeded => write!(f, "Maximum retries exceeded"),
        }
    }
}

impl std::error::Error for ForgeaxeError {}

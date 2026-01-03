use anyhow;
use async_trait::async_trait;
use reqwest::{Client, Method, Response};
use serde_json::Value;
use std::{net::IpAddr, time::Duration};
use tokio::time::timeout;

use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

/// Bitaxe WebAPI client for communicating with Bitaxe and similar miners
#[derive(Debug)]
pub struct BitaxeWebAPI {
    client: Client,
    pub ip: IpAddr,
    port: u16,
    timeout: Duration,
    retries: u32,
}

#[async_trait]
#[allow(dead_code)]
trait Bitaxe200WebAPI: WebAPIClient {
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
impl APIClient for BitaxeWebAPI {
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
impl WebAPIClient for BitaxeWebAPI {
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
                                    return Err(BitaxeError::ParseError(e.to_string()))?;
                                }
                            }
                        }
                    } else if attempt == self.retries {
                        return Err(BitaxeError::HttpError(response.status().as_u16()))?;
                    }
                }
                Err(e) => {
                    if attempt == self.retries {
                        return Err(e)?;
                    }
                }
            }
        }

        Err(BitaxeError::MaxRetriesExceeded)?
    }
}

impl Bitaxe200WebAPI for BitaxeWebAPI {}

impl BitaxeWebAPI {
    /// Create a new Bitaxe WebAPI client
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
            retries: 1,
        }
    }

    /// Execute the actual HTTP request
    async fn execute_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> anyhow::Result<Response, BitaxeError> {
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
            _ => return Err(BitaxeError::UnsupportedMethod(method.to_string())),
        };

        let request = request_builder
            .timeout(self.timeout)
            .build()
            .map_err(|e| BitaxeError::RequestError(e.to_string()))?;

        let response = timeout(self.timeout, self.client.execute(request))
            .await
            .map_err(|_| BitaxeError::Timeout)?
            .map_err(|e| BitaxeError::NetworkError(e.to_string()))?;
        Ok(response)
    }
}

/// Error types for Bitaxe WebAPI operations
#[derive(Debug, Clone)]
pub enum BitaxeError {
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

impl std::fmt::Display for BitaxeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BitaxeError::NetworkError(msg) => write!(f, "Network error: {msg}"),
            BitaxeError::HttpError(code) => write!(f, "HTTP error: {code}"),
            BitaxeError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            BitaxeError::RequestError(msg) => write!(f, "Request error: {msg}"),
            BitaxeError::Timeout => write!(f, "Request timeout"),
            BitaxeError::UnsupportedMethod(method) => write!(f, "Unsupported method: {method}"),
            BitaxeError::MaxRetriesExceeded => write!(f, "Maximum retries exceeded"),
        }
    }
}

impl std::error::Error for BitaxeError {}

// Usage example
#[cfg(test)]
mod tests {
    /*
    #[tokio::test]
    async fn test_espminer_api() {
        let api = EspWebApi::new("192.168.1.100".into(), 80)
            .with_timeout(Duration::from_secs(5))
            .with_retries(3);

        // Test system info
        match api.system_info().await {
            Ok(info) => println!("System info: {:?}", info),
            Err(e) => println!("Error getting system info: {}", e),
        }
    }
     */
}

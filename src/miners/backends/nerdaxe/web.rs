use async_trait::async_trait;
use reqwest::{Client, Method, Response};
use serde_json::Value;
use std::{net::IpAddr, time::Duration};
use tokio::time::timeout;

use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

#[derive(Debug)]
pub struct NerdAxeWebAPI {
    client: Client,
    pub ip: IpAddr,
    port: u16,
    timeout: Duration,
    retries: u32,
}

#[async_trait]
impl APIClient for NerdAxeWebAPI {
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
impl WebAPIClient for NerdAxeWebAPI {
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
                                    return Err(NerdAxeError::ParseError(e.to_string()))?;
                                }
                            }
                        }
                    } else if attempt == self.retries {
                        return Err(NerdAxeError::HttpError(response.status().as_u16()))?;
                    }
                }
                Err(e) => {
                    if attempt == self.retries {
                        return Err(e)?;
                    }
                }
            }
        }

        Err(NerdAxeError::MaxRetriesExceeded)?
    }
}

impl NerdAxeWebAPI {
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

    async fn execute_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> anyhow::Result<Response, NerdAxeError> {
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
            _ => return Err(NerdAxeError::UnsupportedMethod(method.to_string())),
        };

        let request = request_builder
            .timeout(self.timeout)
            .build()
            .map_err(|e| NerdAxeError::RequestError(e.to_string()))?;

        let response = timeout(self.timeout, self.client.execute(request))
            .await
            .map_err(|_| NerdAxeError::Timeout)?
            .map_err(|e| NerdAxeError::NetworkError(e.to_string()))?;
        Ok(response)
    }
}

#[derive(Debug, Clone)]
pub enum NerdAxeError {
    NetworkError(String),
    HttpError(u16),
    ParseError(String),
    RequestError(String),
    Timeout,
    UnsupportedMethod(String),
    MaxRetriesExceeded,
}

impl std::fmt::Display for NerdAxeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NerdAxeError::NetworkError(msg) => write!(f, "Network error: {msg}"),
            NerdAxeError::HttpError(code) => write!(f, "HTTP error: {code}"),
            NerdAxeError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            NerdAxeError::RequestError(msg) => write!(f, "Request error: {msg}"),
            NerdAxeError::Timeout => write!(f, "Request timeout"),
            NerdAxeError::UnsupportedMethod(method) => write!(f, "Unsupported method: {method}"),
            NerdAxeError::MaxRetriesExceeded => write!(f, "Maximum retries exceeded"),
        }
    }
}

impl std::error::Error for NerdAxeError {}

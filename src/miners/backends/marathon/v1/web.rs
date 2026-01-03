use crate::miners::backends::traits::{APIClient, WebAPIClient};
use crate::miners::commands::MinerCommand;
use anyhow;
use async_trait::async_trait;
use diqwest::WithDigestAuth;
use reqwest::{Client, Method};
use serde_json::Value;
use std::net::IpAddr;
use std::time::Duration;

#[derive(Debug)]
pub struct MaraWebAPI {
    ip: IpAddr,
    port: u16,
    client: Client,
    username: String,
    password: String,
}

impl MaraWebAPI {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        Self {
            ip,
            port,
            client,
            username: "root".to_string(),
            password: "root".to_string(),
        }
    }

    async fn make_request(
        &self,
        endpoint: &str,
        method: Method,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        let url = format!("http://{}:{}/kaonsu/v1/{}", self.ip, self.port, endpoint);

        let mut request_builder = match method {
            Method::GET => self.client.get(&url),
            Method::POST => self.client.post(&url),
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method")),
        };

        if let Some(params) = parameters
            && method == Method::POST
        {
            request_builder = request_builder.json(&params);
        }

        let response = request_builder
            .send_with_digest_auth(&self.username, &self.password)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if response.status().is_success() {
            let json_response = response
                .json::<Value>()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;
            Ok(json_response)
        } else {
            Err(anyhow::anyhow!(
                "HTTP request failed with status: {}",
                response.status()
            ))
        }
    }
}

#[async_trait]
impl WebAPIClient for MaraWebAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> anyhow::Result<Value> {
        self.make_request(command, method, parameters).await
    }
}

#[async_trait]
impl APIClient for MaraWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command,
                parameters,
            } => {
                self.send_command(command, false, parameters.clone(), Method::GET)
                    .await
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for Marathon WebAPI"
            )),
        }
    }
}

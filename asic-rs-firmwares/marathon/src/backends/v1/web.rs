use std::{net::IpAddr, time::Duration};

use once_cell::sync::OnceCell;

use anyhow;
use asic_rs_core::{
    data::command::MinerCommand,
    traits::miner::{APIClient, ExposeSecret, MinerAuth, WebAPIClient},
};
use async_trait::async_trait;
use diqwest::WithDigestAuth;
use reqwest::{Client, Method};
use serde_json::Value;

#[derive(Debug)]
pub struct MaraWebAPI {
    ip: IpAddr,
    port: u16,
    client: OnceCell<Client>,
    auth: MinerAuth,
}

impl MaraWebAPI {
    pub fn new(ip: IpAddr, port: u16, auth: MinerAuth) -> Self {
        Self {
            ip,
            port,
            client: OnceCell::new(),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    fn build_client() -> anyhow::Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| anyhow::anyhow!("failed to create HTTP client: {e}"))
    }

    fn client(&self) -> anyhow::Result<&Client> {
        self.client.get_or_try_init(Self::build_client)
    }

    async fn make_request(
        &self,
        endpoint: &str,
        method: Method,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        let url = format!("http://{}:{}/kaonsu/v1/{}", self.ip, self.port, endpoint);
        let client = self.client()?;

        let mut request_builder = match method {
            Method::GET => client.get(&url),
            Method::POST => client.post(&url),
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method")),
        };

        if let Some(params) = parameters
            && method == Method::POST
        {
            request_builder = request_builder.json(&params);
        }

        let response = request_builder
            .send_digest_auth((
                self.auth.username.as_str(),
                self.auth.password.expose_secret(),
            ))
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

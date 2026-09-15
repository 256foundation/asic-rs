use crate::firmware::ElphapexStockFirmware;

use std::{net::IpAddr, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use asic_rs_core::{
    config::pools::PoolConfig,
    data::command::MinerCommand,
    traits::miner::{APIClient, MinerAuth, WebAPIClient},
};
use async_trait::async_trait;
use diqwest::WithDigestAuth;
use once_cell::sync::OnceCell;
use reqwest::{Client, Method, Response};
use serde_json::{Value, json};

#[derive(Debug)]
pub struct ElphapexWebAPI {
    ip: IpAddr,
    port: u16,
    client: OnceCell<Client>,
    timeout: Duration,
    retries: u32,
    auth: MinerAuth,
}

impl ElphapexWebAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            ip,
            port: 80,
            client: OnceCell::new(),
            timeout: Duration::from_secs(10),
            retries: 3,
            auth,
        }
    }

    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    pub fn auth(&self) -> MinerAuth {
        self.auth.clone()
    }

    fn build_client() -> Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed to create HTTP client")
    }

    fn client(&self) -> Result<&Client> {
        self.client.get_or_try_init(Self::build_client)
    }

    async fn execute_web_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> Result<Response> {
        let client = self.client()?;

        let response = match *method {
            Method::GET => {
                if parameters.is_some() {
                    bail!(
                        "{} GET commands do not support parameters",
                        ElphapexStockFirmware::default()
                    );
                }
                client
                    .get(url)
                    .timeout(self.timeout)
                    .send_digest_auth((self.auth.username(), self.auth.password()))
                    .await
                    .map_err(|e| anyhow!(e.to_string()))?
            }
            Method::POST => {
                let mut builder = client.post(url).timeout(self.timeout);
                if let Some(body) = parameters {
                    builder = builder.json(&body);
                }
                builder
                    .send_digest_auth((self.auth.username(), self.auth.password()))
                    .await
                    .map_err(|e| anyhow!(e.to_string()))?
            }
            _ => bail!("Unsupported method: {}", method),
        };

        Ok(response)
    }

    async fn send_web_command(
        &self,
        command: &str,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        let url = format!("http://{}:{}/cgi-bin/{}.cgi", self.ip, self.port, command);

        let mut last_error = None;
        for _ in 0..=self.retries {
            match self.request_json(&url, &method, parameters.clone()).await {
                Ok(json) => return Ok(json),
                Err(error) => {
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow!(
                "{} web API retries exceeded",
                ElphapexStockFirmware::default()
            )
        }))
    }

    async fn request_json(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> Result<Value> {
        let response = self.execute_web_request(url, method, parameters).await?;
        let status = response.status();
        if !status.is_success() {
            bail!("HTTP request failed with status code {}", status);
        }

        response
            .json::<Value>()
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn get_miner_conf(&self) -> Result<Value> {
        self.send_web_command("get_miner_conf", None, Method::GET)
            .await
    }

    pub async fn set_miner_conf(&self, conf: Value) -> Result<Value> {
        self.send_web_command("set_miner_conf", Some(conf), Method::POST)
            .await
    }

    pub async fn reboot(&self) -> Result<bool> {
        self.send_web_command("reboot", None, Method::GET)
            .await
            .map(|_| true)
    }

    pub async fn blink(&self, blink: bool) -> Result<Value> {
        self.send_web_command(
            "blink",
            Some(json!({ "blink": blink.to_string() })),
            Method::POST,
        )
        .await
    }

    pub async fn get_system_info(&self) -> Result<Value> {
        self.send_web_command("get_system_info", None, Method::GET)
            .await
    }

    pub async fn stats(&self) -> Result<Value> {
        self.send_web_command("stats", None, Method::GET).await
    }

    pub async fn set_pools_config(&self, pools: &[PoolConfig]) -> Result<bool> {
        let mut current = self.get_miner_conf().await?;
        let Some(object) = current.as_object_mut() else {
            bail!(
                "{} miner config response is not an object",
                ElphapexStockFirmware::default()
            );
        };

        for (idx, pool) in pools.iter().take(3).enumerate() {
            object.insert(format!("pool{idx}url"), json!(pool.url.to_string()));
            object.insert(format!("pool{idx}user"), json!(pool.username));
            object.insert(format!("pool{idx}pw"), json!(pool.password));
        }

        self.set_miner_conf(current).await.map(|_| true)
    }
}

#[async_trait]
impl APIClient for ElphapexWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command,
                parameters,
            } => {
                self.send_web_command(command, parameters.clone(), Method::GET)
                    .await
            }
            _ => Err(anyhow!(
                "Unsupported command type for {} API",
                ElphapexStockFirmware::default()
            )),
        }
    }
}

#[async_trait]
impl WebAPIClient for ElphapexWebAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        self.send_web_command(command, parameters, method).await
    }
}

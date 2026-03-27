use std::{net::IpAddr, time::Duration};

use asic_rs_core::{
    config::pools::{PoolConfig, PoolGroupConfig},
    data::{command::MinerCommand, pool::PoolURL},
    traits::miner::{APIClient, WebAPIClient},
};
use async_trait::async_trait;
use reqwest::{Client, Method};
use serde_json::{Value, json};

#[derive(Debug)]
pub struct SealMinerWebAPI {
    client: Client,
    pub ip: IpAddr,
}

impl SealMinerWebAPI {
    pub fn new(ip: IpAddr) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");
        Self { client, ip }
    }

    async fn login(&self) -> anyhow::Result<String> {
        let response = self
            .client
            .post(format!("http://{}/cgi-bin/login.php", self.ip))
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .header("X-Requested-With", "XMLHttpRequest")
            .body("username=seal&origin_pwd=seal")
            .send()
            .await?;

        let cookie = response
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .ok_or_else(|| anyhow::anyhow!("No session cookie in login response"))?
            .to_string();

        Ok(cookie)
    }
}

#[async_trait]
impl APIClient for SealMinerWebAPI {
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
                "Unsupported command type for SealMiner API"
            )),
        }
    }
}

#[async_trait]
impl WebAPIClient for SealMinerWebAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> anyhow::Result<Value> {
        let cookie = self.login().await?;
        let url = format!("http://{}/cgi-bin/{}.php", self.ip, command);

        let mut builder = match method {
            Method::POST => {
                let b = self.client.post(&url);
                if let Some(body) = parameters {
                    b.header("Content-Type", "application/json").json(&body)
                } else {
                    b
                }
            }
            _ => self.client.get(&url),
        };
        builder = builder.header("Cookie", cookie);

        builder
            .send()
            .await?
            .json::<Value>()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
}

impl SealMinerWebAPI {
    pub async fn reboot(&self) -> anyhow::Result<Value> {
        self.send_command("reboot", false, None, Method::GET).await
    }

    pub async fn set_led(&self, on: bool) -> anyhow::Result<Value> {
        let value = if on { "on" } else { "off" };

        self.send_command(
            "led_conf",
            false,
            Some(json!({"key": "led", "value": value})),
            Method::POST,
        )
        .await
    }

    pub async fn get_pool_conf(&self) -> anyhow::Result<PoolGroupConfig> {
        let data = self
            .send_command("get_miner_poolconf", false, None, Method::GET)
            .await?;

        let pools = data["pools"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No pools array in response"))?
            .iter()
            .filter_map(|p| {
                let url = PoolURL::from(p["url"].as_str()?.to_string());
                let username = p["user"].as_str().unwrap_or("").to_string();
                let password = p["pass"].as_str().unwrap_or("x").to_string();
                Some(PoolConfig {
                    url,
                    username,
                    password,
                })
            })
            .collect();

        Ok(PoolGroupConfig {
            name: String::new(),
            quota: 1,
            pools,
        })
    }

    pub async fn set_mining_mode(&self, mode: u32) -> anyhow::Result<Value> {
        self.send_command(
            "set_mining_mode",
            false,
            Some(json!({"miningMode": mode})),
            Method::POST,
        )
        .await
    }
}

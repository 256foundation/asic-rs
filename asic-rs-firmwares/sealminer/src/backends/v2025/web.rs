use std::{net::IpAddr, time::Duration};

use asic_rs_core::{
    config::pools::{PoolConfig, PoolGroupConfig},
    data::{command::MinerCommand, pool::PoolURL},
    traits::miner::{APIClient, MinerAuth, WebAPIClient},
};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use reqwest::{Client, Method};
use serde_json::{Value, json};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct SealMinerWebAPI {
    client: OnceCell<Client>,
    pub ip: IpAddr,
    auth: MinerAuth,
    session_cookie: Mutex<Option<String>>,
}

impl SealMinerWebAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            client: OnceCell::new(),
            ip,
            auth,
            session_cookie: Mutex::new(None),
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
        self.session_cookie = Mutex::new(None);
    }

    fn build_client() -> anyhow::Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| anyhow::anyhow!("failed to create HTTP client: {e}"))
    }

    fn client(&self) -> anyhow::Result<&Client> {
        self.client.get_or_try_init(Self::build_client)
    }

    async fn session_cookie(&self) -> anyhow::Result<String> {
        if let Some(cookie) = self.session_cookie.lock().await.clone() {
            return Ok(cookie);
        }
        let client = self.client()?;

        let body = format!(
            "username={}&origin_pwd={}",
            self.auth.username(),
            self.auth.password()
        );
        let response = client
            .post(format!("http://{}/cgi-bin/login.php", self.ip))
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .header("X-Requested-With", "XMLHttpRequest")
            .body(body)
            .send()
            .await?;

        let cookie = response
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .ok_or_else(|| anyhow::anyhow!("No session cookie in login response"))?
            .to_string();

        *self.session_cookie.lock().await = Some(cookie.clone());
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
        let cookie = self.session_cookie().await?;
        let url = format!("http://{}/cgi-bin/{}.php", self.ip, command);
        let client = self.client()?;

        let mut builder = match method {
            Method::POST => {
                let b = client.post(&url);
                if let Some(body) = parameters {
                    b.header("Content-Type", "application/json").json(&body)
                } else {
                    b
                }
            }
            _ => client.get(&url),
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

#[allow(dead_code)]
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

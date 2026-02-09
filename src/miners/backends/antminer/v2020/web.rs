use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;
use diqwest::WithDigestAuth;
use reqwest::{Client, Method, Response};
use serde_json::{Value, json};
use std::{net::IpAddr, time::Duration};

use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

#[derive(Debug)]
pub struct AntMinerWebAPI {
    ip: IpAddr,
    port: u16,
    client: Client,
    timeout: Duration,
    username: String,
    password: String,
}

impl AntMinerWebAPI {
    pub fn new(ip: IpAddr) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            ip,
            port: 80,
            client,
            timeout: Duration::from_secs(5),
            username: "root".to_string(),
            password: "root".to_string(),
        }
    }

    pub fn with_auth(ip: IpAddr, username: String, password: String) -> Self {
        let mut client = Self::new(ip);
        client.port = 80;
        client.username = username;
        client.password = password;
        client
    }

    pub fn with_timeout(ip: IpAddr, timeout: Duration) -> Self {
        let mut client = Self::new(ip);
        client.port = 80;
        client.timeout = timeout;
        client
    }

    async fn send_web_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        let url = format!("http://{}:{}/cgi-bin/{}.cgi", self.ip, self.port, command);

        let response = self
            .execute_web_request(&url, &method, parameters.clone())
            .await?;

        let status = response.status();
        if status.is_success() {
            let json_data = response.json().await.map_err(|e| anyhow!(e.to_string()))?;
            Ok(json_data)
        } else {
            bail!("HTTP request failed with status code {}", status);
        }
    }

    async fn execute_web_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> Result<Response> {
        let response = match *method {
            Method::GET => self
                .client
                .get(url)
                .timeout(self.timeout)
                .send_digest_auth((self.username.as_str(), self.password.as_str()))
                .await
                .map_err(|e| anyhow!(e.to_string()))?,
            Method::POST => {
                let data = parameters.unwrap_or_else(|| json!({}));
                self.client
                    .post(url)
                    .json(&data)
                    .timeout(self.timeout)
                    .send_digest_auth((self.username.as_str(), self.password.as_str()))
                    .await
                    .map_err(|e| anyhow!(e.to_string()))?
            }
            _ => bail!("Unsupported method: {}", method),
        };

        Ok(response)
    }

    pub async fn get_miner_conf(&self) -> Result<Value> {
        self.send_web_command("get_miner_conf", false, None, Method::GET)
            .await
    }

    pub async fn set_miner_conf(&self, conf: Value) -> Result<Value> {
        self.send_web_command("set_miner_conf", false, Some(conf), Method::POST)
            .await
    }

    pub async fn blink(&self, blink: bool) -> Result<Value> {
        let param = if blink {
            json!({"blink": "true"})
        } else {
            json!({"blink": "false"})
        };
        self.send_web_command("blink", false, Some(param), Method::POST)
            .await
    }

    pub async fn reboot(&self) -> Result<Value> {
        self.send_web_command("reboot", false, None, Method::POST)
            .await
    }

    pub async fn get_system_info(&self) -> Result<Value> {
        self.send_web_command("get_system_info", false, None, Method::GET)
            .await
    }

    pub async fn get_network_info(&self) -> Result<Value> {
        self.send_web_command("get_network_info", false, None, Method::GET)
            .await
    }

    pub async fn summary(&self) -> Result<Value> {
        self.send_web_command("summary", false, None, Method::GET)
            .await
    }

    pub async fn get_blink_status(&self) -> Result<Value> {
        self.send_web_command("get_blink_status", false, None, Method::GET)
            .await
    }

    pub async fn set_network_conf(
        &self,
        ip: String,
        dns: String,
        gateway: String,
        subnet_mask: String,
        hostname: String,
        protocol: u8,
    ) -> Result<Value> {
        let config = json!({
            "ipAddress": ip,
            "ipDns": dns,
            "ipGateway": gateway,
            "ipHost": hostname,
            "ipPro": protocol,
            "ipSub": subnet_mask
        });
        self.send_web_command("set_network_conf", false, Some(config), Method::POST)
            .await
    }
}

#[async_trait]
impl APIClient for AntMinerWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command,
                parameters,
            } => self
                .send_web_command(command, false, parameters.clone(), Method::GET)
                .await
                .map_err(|e| anyhow!(e.to_string())),
            _ => Err(anyhow!("Unsupported command type for Web client")),
        }
    }
}

#[async_trait]
impl WebAPIClient for AntMinerWebAPI {
    async fn send_command(
        &self,
        command: &str,
        privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        self.send_web_command(command, privileged, parameters, method)
            .await
    }
}

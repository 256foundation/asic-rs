use std::{net::IpAddr, time::Duration};

use anyhow;
use asic_rs_core::{
    data::command::MinerCommand,
    traits::miner::{APIClient, WebAPIClient},
};
use async_trait::async_trait;
use reqwest::{Client, Method};
use serde_json::{Value, json};

#[derive(Debug)]
pub struct LuxMinerWebAPI {
    ip: IpAddr,
    port: u16,
    client: Client,
}

impl LuxMinerWebAPI {
    pub fn new(ip: IpAddr) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create LuxMiner web client");

        Self {
            ip,
            port: 8080,
            client,
        }
    }

    fn build_request_body(command: &str, parameters: Option<Value>) -> Value {
        let mut body = json!({ "command": command });

        if let Some(params) = parameters {
            match params {
                Value::Object(params_object) => {
                    if let Some(body_object) = body.as_object_mut() {
                        for (key, value) in params_object {
                            body_object.insert(key, value);
                        }
                    }
                }
                value => {
                    body["parameter"] = value;
                }
            }
        }

        body
    }

    async fn make_request(
        &self,
        command: &str,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        let url = format!("http://{}:{}/api?command={}", self.ip, self.port, command);
        let body = Self::build_request_body(command, parameters);

        let response = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {e}"))?;

        if response.status().is_success() {
            response
                .json::<Value>()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {e}"))
        } else {
            Err(anyhow::anyhow!(
                "HTTP request failed with status: {}",
                response.status()
            ))
        }
    }

    pub async fn add_group(&self, name: &str, quota: u32) -> anyhow::Result<Value> {
        self.send_command(
            "addgroup",
            false,
            Some(Value::String(format!("{name},{quota}"))),
            Method::POST,
        )
        .await
    }

    pub async fn remove_group(&self, group_id: u32) -> anyhow::Result<Value> {
        self.send_command(
            "removegroup",
            false,
            Some(Value::String(group_id.to_string())),
            Method::POST,
        )
        .await
    }

    pub async fn remove_pool(&self, pool_id: i32) -> anyhow::Result<Value> {
        self.send_command(
            "removepool",
            false,
            Some(Value::String(pool_id.to_string())),
            Method::POST,
        )
        .await
    }

    pub async fn set_group_quota(&self, group_id: u32, quota: u32) -> anyhow::Result<Value> {
        self.send_command(
            "groupquota",
            false,
            Some(Value::String(format!("{group_id},{quota}"))),
            Method::POST,
        )
        .await
    }

    pub async fn add_pool(
        &self,
        url: &str,
        user: &str,
        password: &str,
        group_id: u32,
    ) -> anyhow::Result<Value> {
        self.send_command(
            "addpool",
            false,
            Some(Value::String(format!("{url},{user},{password},{group_id}"))),
            Method::POST,
        )
        .await
    }
}

#[async_trait]
impl WebAPIClient for LuxMinerWebAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> anyhow::Result<Value> {
        if method != Method::POST {
            return Err(anyhow::anyhow!("LuxMiner web API only supports POST here"));
        }

        self.make_request(command, parameters).await
    }
}

#[async_trait]
impl APIClient for LuxMinerWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command,
                parameters,
            } => self.make_request(command, parameters.clone()).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for LuxMiner web API"
            )),
        }
    }
}

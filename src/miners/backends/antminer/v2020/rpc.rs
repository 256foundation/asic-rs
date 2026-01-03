use anyhow;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::net::IpAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::miners::api::rpc::errors::RPCError;
use crate::miners::api::rpc::status::RPCCommandStatus;
use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

#[derive(Debug)]
pub struct AntMinerRPCAPI {
    ip: IpAddr,
    port: u16,
}

impl AntMinerRPCAPI {
    pub fn new(ip: IpAddr) -> Self {
        Self { ip, port: 4028 }
    }

    async fn send_rpc_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        let mut stream = tokio::net::TcpStream::connect((self.ip, self.port))
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;

        let request = if let Some(params) = parameters {
            json!({
                "command": command,
                "parameter": params
            })
        } else {
            json!({
                "command": command
            })
        };

        let json_str = request.to_string();
        let message = format!("{}\n", json_str);

        stream.write_all(message.as_bytes()).await?;

        let mut response = String::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = stream.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);
            response.push_str(&chunk);

            if response.contains('\0') || response.ends_with('\n') {
                break;
            }
        }

        let clean_response = response.trim_end_matches('\0').trim_end_matches('\n');
        self.parse_rpc_result(clean_response)
    }

    fn parse_rpc_result(&self, response: &str) -> anyhow::Result<Value> {
        let status = RPCCommandStatus::from_antminer(response)?;
        match status.into_result() {
            Ok(_) => Ok(serde_json::from_str(response)?),
            Err(e) => Err(e)?,
        }
    }

    pub async fn stats(&self, new_api: bool) -> anyhow::Result<Value> {
        if new_api {
            self.send_rpc_command("stats", false, Some(json!({"new_api": true})))
                .await
        } else {
            self.send_rpc_command("stats", false, None).await
        }
    }

    pub async fn summary(&self, new_api: bool) -> anyhow::Result<Value> {
        if new_api {
            self.send_rpc_command("summary", false, Some(json!({"new_api": true})))
                .await
        } else {
            self.send_rpc_command("summary", false, None).await
        }
    }

    pub async fn pools(&self, new_api: bool) -> anyhow::Result<Value> {
        if new_api {
            self.send_rpc_command("pools", false, Some(json!({"new_api": true})))
                .await
        } else {
            self.send_rpc_command("pools", false, None).await
        }
    }

    pub async fn version(&self) -> anyhow::Result<Value> {
        self.send_rpc_command("version", false, None).await
    }

    pub async fn rate(&self) -> anyhow::Result<Value> {
        self.send_rpc_command("rate", false, Some(json!({"new_api": true})))
            .await
    }

    pub async fn warning(&self) -> anyhow::Result<Value> {
        self.send_rpc_command("warning", false, Some(json!({"new_api": true})))
            .await
    }

    pub async fn reload(&self) -> anyhow::Result<Value> {
        self.send_rpc_command("reload", false, Some(json!({"new_api": true})))
            .await
    }
}

#[async_trait]
impl APIClient for AntMinerRPCAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC {
                command,
                parameters,
            } => self
                .send_rpc_command(command, false, parameters.clone())
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string())),
            _ => Err(anyhow::anyhow!("Unsupported command type for RPC client")),
        }
    }
}

#[async_trait]
impl RPCAPIClient for AntMinerRPCAPI {
    async fn send_command(
        &self,
        command: &str,
        privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.send_rpc_command(command, privileged, parameters).await
    }
}

impl RPCCommandStatus {
    pub fn from_antminer(response: &str) -> Result<Self, RPCError> {
        let value: serde_json::Value = serde_json::from_str(response)?;

        if let Some(status_array) = value.get("STATUS")
            && let Some(status_obj) = status_array.get(0)
            && let Some(status) = status_obj.get("STATUS").and_then(|v| v.as_str())
        {
            let message = status_obj.get("Msg").and_then(|v| v.as_str());

            return Ok(Self::from_str(status, message));
        }

        Ok(Self::Success)
    }
}

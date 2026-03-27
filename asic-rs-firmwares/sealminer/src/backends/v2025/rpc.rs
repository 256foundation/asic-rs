use std::net::IpAddr;

use asic_rs_core::{
    data::command::{MinerCommand, RPCCommandStatus},
    errors::RPCError,
    traits::miner::*,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub struct SealMinerRPCAPI {
    ip: IpAddr,
    port: u16,
}

#[allow(dead_code)]
impl SealMinerRPCAPI {
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
        let status = RPCCommandStatus::from_sealminer(response)?;
        match status.into_result() {
            Ok(_) => Ok(serde_json::from_str(response)?),
            Err(e) => Err(e)?,
        }
    }

    pub async fn summary(&self) -> anyhow::Result<Value> {
        self.send_rpc_command("summary", false, None).await
    }

    pub async fn stats(&self) -> anyhow::Result<Value> {
        self.send_rpc_command("stats", false, None).await
    }

    pub async fn pools(&self) -> anyhow::Result<Value> {
        self.send_rpc_command("pools", false, None).await
    }

    pub async fn version(&self) -> anyhow::Result<Value> {
        self.send_rpc_command("version", false, None).await
    }

    pub async fn updatepools(&self, pools: &[(&str, &str, &str)]) -> anyhow::Result<Value> {
        let pools_json = serde_json::json!({
            "pools": pools.iter().map(|(url, user, pass)| serde_json::json!({
                "url": url,
                "user": user,
                "pass": pass,
            })).collect::<Vec<_>>()
        });
        let parameter = pools_json.to_string();
        self.send_rpc_command(
            "updatepools",
            false,
            Some(serde_json::Value::String(parameter)),
        )
        .await
    }

    pub async fn ascset(&self, parameter: &str) -> anyhow::Result<Value> {
        self.send_rpc_command(
            "ascset",
            false,
            Some(serde_json::Value::String(parameter.to_string())),
        )
        .await
    }

    pub async fn restart(&self) -> anyhow::Result<Value> {
        self.send_rpc_command("restart", false, None).await
    }
}

#[async_trait]
impl APIClient for SealMinerRPCAPI {
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
impl RPCAPIClient for SealMinerRPCAPI {
    async fn send_command(
        &self,
        command: &str,
        privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.send_rpc_command(command, privileged, parameters).await
    }
}

trait StatusFromSealMiner {
    fn from_sealminer(response: &str) -> Result<Self, RPCError>
    where
        Self: Sized;
}

impl StatusFromSealMiner for RPCCommandStatus {
    fn from_sealminer(response: &str) -> Result<Self, RPCError> {
        let value: Value = serde_json::from_str(response)?;

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

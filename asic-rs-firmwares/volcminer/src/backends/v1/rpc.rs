use std::net::IpAddr;

use anyhow;
use asic_rs_core::{
    data::command::{MinerCommand, RPCCommandStatus},
    errors::RPCError,
    traits::miner::*,
    util::{DEFAULT_RPC_TIMEOUT, connect_tcp_stream, read_stream_response, write_all_with_timeout},
};
use async_trait::async_trait;
use serde_json::{Value, json};

#[derive(Debug)]
pub struct VolcMinerRPCAPI {
    ip: IpAddr,
    port: u16,
}

#[allow(dead_code)]
impl VolcMinerRPCAPI {
    pub fn new(ip: IpAddr) -> Self {
        Self { ip, port: 8359 }
    }

    async fn send_rpc_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
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

        let response = {
            let mut stream = connect_tcp_stream((self.ip, self.port), DEFAULT_RPC_TIMEOUT)
                .await
                .map_err(|_| RPCError::ConnectionFailed)?;

            write_all_with_timeout(&mut stream, message.as_bytes(), DEFAULT_RPC_TIMEOUT).await?;
            read_stream_response(&mut stream, DEFAULT_RPC_TIMEOUT).await
        };
        let response = response?;

        self.parse_rpc_result(&response)
    }

    pub(super) fn parse_rpc_result(&self, response: &str) -> anyhow::Result<Value> {
        let status = RPCCommandStatus::from_volcminer(response)?;
        match status.into_result() {
            Ok(_) => Ok(Self::parse_rpc_json(response)?),
            Err(e) => Err(e)?,
        }
    }

    fn parse_rpc_json(response: &str) -> Result<Value, serde_json::Error> {
        serde_json::from_str(response).or_else(|error| {
            let repaired = response.replace("}{\"STATS\"", "},{\"STATS\"");
            if repaired == response {
                return Err(error);
            }

            serde_json::from_str(&repaired)
        })
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
impl APIClient for VolcMinerRPCAPI {
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
impl RPCAPIClient for VolcMinerRPCAPI {
    async fn send_command(
        &self,
        command: &str,
        privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.send_rpc_command(command, privileged, parameters).await
    }
}

trait StatusFromVolcMiner {
    fn from_volcminer(response: &str) -> Result<Self, RPCError>
    where
        Self: Sized;
}

impl StatusFromVolcMiner for RPCCommandStatus {
    fn from_volcminer(response: &str) -> Result<Self, RPCError> {
        let value: Value = VolcMinerRPCAPI::parse_rpc_json(response)?;

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

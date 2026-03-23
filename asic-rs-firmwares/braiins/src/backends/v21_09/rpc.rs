use std::net::IpAddr;

use anyhow;
use asic_rs_core::{
    data::command::{MinerCommand, RPCCommandStatus},
    errors::RPCError,
    traits::miner::*,
    util::{DEFAULT_RPC_TIMEOUT, read_stream_response},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub struct BraiinsRPCAPI {
    ip: IpAddr,
    port: u16,
}

impl BraiinsRPCAPI {
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

        let response = read_stream_response(&mut stream, DEFAULT_RPC_TIMEOUT).await;
        let _ = stream.shutdown().await;
        let response = response?;
        self.parse_rpc_result(&response)
    }

    fn parse_rpc_result(&self, response: &str) -> anyhow::Result<Value> {
        let status = RPCCommandStatus::from_braiins(response)?;
        match status.into_result() {
            Ok(_) => Ok(serde_json::from_str(response)?),
            Err(e) => Err(e)?,
        }
    }
}

#[async_trait]
impl APIClient for BraiinsRPCAPI {
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
impl RPCAPIClient for BraiinsRPCAPI {
    async fn send_command(
        &self,
        command: &str,
        privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.send_rpc_command(command, privileged, parameters).await
    }
}

trait StatusFromBraiins {
    fn from_braiins(response: &str) -> Result<Self, RPCError>
    where
        Self: Sized;
}

impl StatusFromBraiins for RPCCommandStatus {
    fn from_braiins(response: &str) -> Result<Self, RPCError> {
        let json: Value = serde_json::from_str(response)
            .map_err(|_| RPCError::StatusCheckFailed("Invalid JSON response".to_string()))?;

        let status = json
            .pointer("/STATUS/0/STATUS")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                RPCError::StatusCheckFailed(
                    "Failed to parse status from Braiins response".to_string(),
                )
            })?;

        let message = json.pointer("/STATUS/0/Msg").and_then(|v| v.as_str());

        Ok(Self::from_str(status, message))
    }
}

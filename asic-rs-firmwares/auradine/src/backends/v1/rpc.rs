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
pub struct AuradineRPCAPI {
    ip: IpAddr,
    port: u16,
}

impl AuradineRPCAPI {
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

        let message = format!("{}\n", request);
        stream.write_all(message.as_bytes()).await?;

        let response = read_stream_response(&mut stream, DEFAULT_RPC_TIMEOUT).await;
        let _ = stream.shutdown().await;
        let response = response?;

        self.parse_rpc_result(&response)
    }

    fn parse_rpc_result(&self, response: &str) -> anyhow::Result<Value> {
        let status = RPCCommandStatus::from_auradine_v1(response)?;
        match status.into_result() {
            Ok(_) => Ok(serde_json::from_str(response)?),
            Err(e) => Err(e)?,
        }
    }
}

#[async_trait]
impl APIClient for AuradineRPCAPI {
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
impl RPCAPIClient for AuradineRPCAPI {
    async fn send_command(
        &self,
        command: &str,
        privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.send_rpc_command(command, privileged, parameters).await
    }
}

pub(crate) trait StatusFromAuradineV1 {
    fn from_auradine_v1(response: &str) -> Result<Self, RPCError>
    where
        Self: Sized;
}

impl StatusFromAuradineV1 for RPCCommandStatus {
    fn from_auradine_v1(response: &str) -> Result<Self, RPCError> {
        let value: Value = serde_json::from_str(response)?;

        if let Some(status_array) = value.get("STATUS").and_then(|v| v.as_array())
            && let Some(status_obj) = status_array.first()
            && let Some(status) = status_obj.get("STATUS").and_then(|v| v.as_str())
        {
            let message = status_obj.get("Msg").and_then(|v| v.as_str());
            return Ok(Self::from_str(status, message));
        }

        if let Some(status) = value.get("STATUS").and_then(|v| v.as_str()) {
            return Ok(Self::from_str(status, None));
        }

        Ok(Self::Success)
    }
}

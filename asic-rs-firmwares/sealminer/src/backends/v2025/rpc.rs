use std::net::IpAddr;

use asic_rs_core::{
    data::{
        command::{MinerCommand, RPCCommandStatus},
        miner::MiningMode,
    },
    errors::RPCError,
    traits::miner::*,
    util::{DEFAULT_RPC_TIMEOUT, connect_tcp_stream, read_stream_response, write_all_with_timeout},
};
use async_trait::async_trait;
use measurements::Power;
use serde_json::{Value, json};

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

    #[cfg(test)]
    pub(super) fn new_with_port(ip: IpAddr, port: u16) -> Self {
        Self { ip, port }
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
            read_stream_response(&mut stream, DEFAULT_RPC_TIMEOUT).await?
        };

        self.parse_rpc_result(&response)
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

    async fn set_tuning_option(&self, option: &str, settings: Value) -> anyhow::Result<Value> {
        let parameter = format!("0,{option},{settings}");
        let response = self.ascset(&parameter).await?;
        anyhow::ensure!(
            response
                .get("STATUS")
                .and_then(Value::as_array)
                .and_then(|statuses| statuses.first())
                .is_some_and(|status| {
                    status.get("STATUS").and_then(Value::as_str) == Some("S")
                        && status.get("Code").and_then(Value::as_u64) == Some(119)
                }),
            "Sealminer {option} did not return a successful ASC set (STATUS S, Code 119): {response}"
        );
        Ok(response)
    }

    pub async fn set_power_limit(&self, watts: Power) -> anyhow::Result<Value> {
        let watts = watts.as_watts();
        anyhow::ensure!(
            watts.is_finite()
                && (0.0..=f64::from(u32::MAX)).contains(&watts)
                && watts.fract() == 0.0,
            "Sealminer power limit must be finite, whole, non-negative watts representable as u32"
        );
        self.set_tuning_option("powerlimit", json!({"value": (watts as u32).to_string()}))
            .await
    }

    pub async fn set_mining_mode(&self, mode: MiningMode) -> anyhow::Result<Value> {
        let mode = match mode {
            MiningMode::Low => "low_power",
            MiningMode::Normal => "normal",
            MiningMode::High => "high_performance",
        };
        self.set_tuning_option("workmode", json!({"mode": mode}))
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

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

/// Bitmain's one API extension to the cgminer protocol.
const NEW_API_FLAG: &str = "new_api";

/// Build the JSON request for a cgminer-style RPC call.
///
/// `new_api` is a top-level flag alongside `command`, not a value of cgminer's
/// `parameter` argument. Nested under `parameter` the firmware ignores it and
/// answers with the legacy payload — a well-formed success a caller cannot
/// tell apart from the real thing. Everything else keeps the `parameter`
/// wrapper cgminer expects, e.g. `switchpool`'s pool index.
fn build_rpc_request(command: &str, parameters: Option<Value>) -> Value {
    let mut request = json!({ "command": command });

    match parameters {
        Some(Value::Object(mut params)) => {
            if let Some(new_api) = params.remove(NEW_API_FLAG) {
                request[NEW_API_FLAG] = new_api;
            }
            if !params.is_empty() {
                request["parameter"] = Value::Object(params);
            }
        }
        Some(params) => request["parameter"] = params,
        None => {}
    }

    request
}

#[derive(Debug)]
pub struct AntMinerRPCAPI {
    ip: IpAddr,
    port: u16,
}

#[allow(dead_code)]
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
        let request = build_rpc_request(command, parameters);

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

trait StatusFromAntMiner {
    fn from_antminer(response: &str) -> Result<Self, RPCError>
    where
        Self: Sized;
}

impl StatusFromAntMiner for RPCCommandStatus {
    fn from_antminer(response: &str) -> Result<Self, RPCError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_api_becomes_a_top_level_flag() {
        // The firmware only honours `new_api` at the top level; nested under
        // `parameter` it is ignored and the legacy payload comes back.
        assert_eq!(
            build_rpc_request("stats", Some(json!({"new_api": true}))),
            json!({"command": "stats", "new_api": true})
        );
    }

    #[test]
    fn scalar_parameters_keep_the_cgminer_wrapper() {
        // cgminer's own convention, e.g. `switchpool` with a pool index.
        assert_eq!(
            build_rpc_request("switchpool", Some(json!("1"))),
            json!({"command": "switchpool", "parameter": "1"})
        );
    }

    #[test]
    fn other_object_parameters_keep_the_cgminer_wrapper() {
        // Only `new_api` is lifted; anything else stays where cgminer wants it.
        assert_eq!(
            build_rpc_request("ascset", Some(json!({"idx": 0}))),
            json!({"command": "ascset", "parameter": {"idx": 0}})
        );
    }

    #[test]
    fn new_api_is_lifted_out_of_a_larger_parameter_object() {
        assert_eq!(
            build_rpc_request("stats", Some(json!({"new_api": true, "idx": 0}))),
            json!({"command": "stats", "new_api": true, "parameter": {"idx": 0}})
        );
    }

    #[test]
    fn absent_parameters_send_the_bare_command() {
        assert_eq!(
            build_rpc_request("version", None),
            json!({"command": "version"})
        );
    }
}

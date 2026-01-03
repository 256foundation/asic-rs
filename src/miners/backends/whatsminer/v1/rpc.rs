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
pub struct WhatsMinerRPCAPI {
    ip: IpAddr,
    port: u16,
}

#[async_trait]
impl APIClient for WhatsMinerRPCAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC {
                command,
                parameters,
            } => self
                .send_command(command, false, parameters.clone())
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string())),
            _ => Err(anyhow::anyhow!("Cannot send non RPC command to RPC API")),
        }
    }
}

impl RPCCommandStatus {
    fn from_btminer_v1(response: &str) -> anyhow::Result<Self, RPCError> {
        let parsed: anyhow::Result<serde_json::Value, _> = serde_json::from_str(response);

        if let Ok(data) = &parsed {
            let command_status = data["STATUS"][0]["STATUS"]
                .as_str()
                .or(data["STATUS"].as_str());
            let message = data["STATUS"][0]["Msg"].as_str().or(data["Msg"].as_str());

            match command_status {
                Some(status) => match status {
                    "S" | "I" => Ok(RPCCommandStatus::Success),
                    _ => Err(RPCError::StatusCheckFailed(
                        message
                            .unwrap_or("Unknown error when looking for status code")
                            .to_owned(),
                    )),
                },
                None => Err(RPCError::StatusCheckFailed(
                    message
                        .unwrap_or("Unknown error when parsing status")
                        .to_owned(),
                )),
            }
        } else {
            Err(RPCError::DeserializationFailed(parsed.err().unwrap()))
        }
    }
}

#[async_trait]
impl RPCAPIClient for WhatsMinerRPCAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        let mut stream = tokio::net::TcpStream::connect((self.ip, self.port))
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;

        let request = match parameters {
            Some(Value::Object(mut obj)) => {
                // Use the existing object as the base
                obj.insert("command".to_string(), json!(command));
                Value::Object(obj)
            }
            Some(other) => {
                // Wrap non-objects into the "param" key
                json!({ "command": command, "parameter": other })
            }
            None => {
                // No parameters at all
                json!({ "command": command })
            }
        };
        let json_str = request.to_string();
        let json_bytes = json_str.as_bytes();

        stream.write_all(json_bytes).await.unwrap();

        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).await.unwrap();

        let response = String::from_utf8_lossy(&buffer)
            .into_owned()
            .replace('\0', "")
            .replace("\n", "") // Fix for WM V1, can have newlines in version which breaks the json parser
            .replace(",}", "}"); // Fix for WM V1, can have trailing commas which breaks the json parser

        self.parse_rpc_result(&response)
    }
}

impl WhatsMinerRPCAPI {
    pub fn new(ip: IpAddr, port: Option<u16>) -> Self {
        Self {
            ip,
            port: port.unwrap_or(4028),
        }
    }

    fn parse_rpc_result(&self, response: &str) -> anyhow::Result<Value> {
        let status = RPCCommandStatus::from_btminer_v1(response)?;
        match status.into_result() {
            Ok(_) => Ok(serde_json::from_str(response)?),
            Err(e) => Err(e)?,
        }
    }
}

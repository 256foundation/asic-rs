use anyhow;
use async_trait::async_trait;
use base64::prelude::*;
use chrono::Utc;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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
    user: String,
    password: String,
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
    fn from_btminer_v3(response: &str) -> anyhow::Result<Self, RPCError> {
        let value: serde_json::Value = serde_json::from_str(response)?;

        match value["code"].as_i64() {
            None => {
                let message = value["msg"].as_str();

                Err(RPCError::StatusCheckFailed(
                    message
                        .unwrap_or("Unknown error when looking for status code")
                        .to_owned(),
                ))
            }
            Some(code) => match code {
                0 => Ok(Self::Success),
                _ => {
                    let message = value["msg"].as_str();
                    Err(RPCError::StatusCheckFailed(
                        message
                            .unwrap_or("Unknown error when parsing status")
                            .to_owned(),
                    ))
                }
            },
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
        if _privileged || command.starts_with("set.") {
            return self.send_privileged_command(command, parameters).await;
        }

        let mut stream = tokio::net::TcpStream::connect((self.ip, self.port))
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;

        let request = match parameters {
            Some(Value::Object(mut obj)) => {
                // Use the existing object as the base
                obj.insert("cmd".to_string(), json!(command));
                Value::Object(obj)
            }
            Some(other) => {
                // Wrap non-objects into the "param" key
                json!({ "cmd": command, "param": other })
            }
            None => {
                // No parameters at all
                json!({ "cmd": command })
            }
        };
        let json_str = request.to_string();
        let json_bytes = json_str.as_bytes();
        let length = json_bytes.len() as u32;

        stream.write_all(&length.to_le_bytes()).await?;
        stream.write_all(json_bytes).await?;

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let response_len = u32::from_le_bytes(len_buf) as usize;

        let mut resp_buf = vec![0u8; response_len];
        stream.read_exact(&mut resp_buf).await?;

        let response_str = String::from_utf8_lossy(&resp_buf).into_owned();

        self.parse_rpc_result(&response_str)
    }
}

impl WhatsMinerRPCAPI {
    pub fn new(ip: IpAddr, port: Option<u16>) -> Self {
        Self {
            ip,
            port: port.unwrap_or(4433),
            user: "super".to_string(),
            password: "super".to_string(),
        }
    }

    fn parse_rpc_result(&self, response: &str) -> anyhow::Result<Value> {
        let status = RPCCommandStatus::from_btminer_v3(response)?;
        match status.into_result() {
            Ok(_) => Ok(serde_json::from_str(response)?),
            Err(e) => Err(e)?,
        }
    }

    async fn send_privileged_command(
        &self,
        command: &str,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        let salt = self.get_salt().await;
        if salt.is_none() {
            anyhow::bail!("Could not get salt for privileged command.");
        };

        let mut stream = tokio::net::TcpStream::connect((self.ip, self.port))
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;

        let timestamp = Utc::now().timestamp();

        let tokenized_command =
            format!("{}{}{}{}", command, self.password, salt.unwrap(), timestamp);

        let hashed_command = Sha256::digest(tokenized_command.as_bytes());
        let encoded_command = BASE64_STANDARD.encode(hashed_command);
        let mut command_bytes = encoded_command.into_bytes();

        if command_bytes.len() > 8 {
            command_bytes[8] = 0;
            command_bytes = command_bytes[..8].to_vec();
        }

        let token = String::from_utf8_lossy(command_bytes.as_slice());

        let request = match parameters {
            Some(Value::Object(mut obj)) => {
                // Use the existing object as the base
                obj.insert("cmd".to_string(), json!(command));
                obj.insert("token".to_string(), json!(token));
                obj.insert("account".to_string(), json!(self.user.clone()));
                obj.insert("ts".to_string(), json!(timestamp));
                Value::Object(obj)
            }
            Some(other) => {
                // Wrap non-objects into the "param" key
                json!({
                    "cmd": command,
                    "param": other,
                    "token": token,
                    "account": self.user.clone(),
                    "ts": timestamp,
                })
            }
            None => {
                // No parameters at all
                json!({
                    "cmd": command,
                    "token": token,
                    "account": self.user.clone(),
                    "ts": timestamp,
                })
            }
        };
        let json_str = request.to_string();
        let json_bytes = json_str.as_bytes();
        let length = json_bytes.len() as u32;

        stream.write_all(&length.to_le_bytes()).await?;
        stream.write_all(json_bytes).await?;

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let response_len = u32::from_le_bytes(len_buf) as usize;

        let mut resp_buf = vec![0u8; response_len];
        stream.read_exact(&mut resp_buf).await?;

        let response_str = String::from_utf8_lossy(&resp_buf).into_owned();

        self.parse_rpc_result(&response_str)
    }

    async fn get_salt(&self) -> Option<String> {
        self.send_command("get.device.info", false, Some(json!("salt")))
            .await
            .ok()
            .and_then(|s| s["msg"]["salt"].as_str().map(|s| s.to_string()))
    }
}

use std::net::IpAddr;

use aes::{
    Aes256,
    cipher::{BlockEncryptMut, KeyInit},
};
use anyhow;
use asic_rs_core::errors::RPCError::StatusCheckFailed;
use asic_rs_core::{
    data::command::{MinerCommand, RPCCommandStatus},
    errors::RPCError,
    traits::miner::*,
    util::{DEFAULT_RPC_TIMEOUT, read_exact_with_timeout},
};
use async_trait::async_trait;
use base64::prelude::*;
use chrono::Utc;
use ecb::cipher::block_padding::ZeroPadding;
use md5crypt::md5crypt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const UNLOCK_CLIENT: &str = "heatcore";
const UNLOCK_MAGIC: &str = "3804fe31981418ce711a31d94bc69651";

type Aes256EcbEnc = ecb::Encryptor<Aes256>;

fn encrypt_param(aes_key: &[u8], data: &str) -> anyhow::Result<String> {
    let original_len = data.len();
    let padded_len = (original_len + 15) & !15;
    let mut buffer = data.as_bytes().to_vec();
    buffer.resize(padded_len, 0);

    let enc = Aes256EcbEnc::new_from_slice(aes_key)
        .map_err(|e| anyhow::anyhow!("Invalid AES key length: {e:?}"))?
        .encrypt_padded_mut::<ZeroPadding>(&mut buffer, original_len)
        .map_err(|e| anyhow::anyhow!("AES encryption failed: {e:?}"))?;

    Ok(BASE64_STANDARD.encode(enc).replace('\n', ""))
}

#[derive(Debug)]
pub struct WhatsMinerRPCAPI {
    ip: IpAddr,
    port: u16,
    auth: MinerAuth,
}

#[async_trait]
impl APIClient for WhatsMinerRPCAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC {
                command,
                parameters,
            } => self.send_command(command, false, parameters.clone()).await,
            _ => Err(anyhow::anyhow!("Cannot send non RPC command to RPC API")),
        }
    }
}

trait StatusFromBTMinerV3 {
    fn from_btminer_v3(response: &str) -> Result<Self, RPCError>
    where
        Self: Sized;
}

impl StatusFromBTMinerV3 for RPCCommandStatus {
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
            let result = self
                .send_privileged_command(command, parameters.clone())
                .await;
            return match &result {
                Err(e)
                    if e.downcast_ref::<RPCError>()
                        .is_some_and(|rpc| matches!(rpc, StatusCheckFailed(_))) =>
                {
                    self.unlock_write_commands().await?;
                    self.send_privileged_command(command, parameters.clone())
                        .await
                }
                _ => result,
            };
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

        stream
            .write_all(&length.to_le_bytes())
            .await
            .map_err(RPCError::from)?;
        stream.write_all(json_bytes).await.map_err(RPCError::from)?;

        let mut len_buf = [0u8; 4];
        read_exact_with_timeout(&mut stream, &mut len_buf, DEFAULT_RPC_TIMEOUT).await?;
        let response_len = u32::from_le_bytes(len_buf) as usize;

        let mut resp_buf = vec![0u8; response_len];
        read_exact_with_timeout(&mut stream, &mut resp_buf, DEFAULT_RPC_TIMEOUT).await?;

        let response_str = String::from_utf8_lossy(&resp_buf).into_owned();

        self.parse_rpc_result(&response_str)
    }
}

impl WhatsMinerRPCAPI {
    pub fn new(ip: IpAddr, port: Option<u16>, auth: MinerAuth) -> Self {
        Self {
            ip,
            port: port.unwrap_or(4433),
            auth,
        }
    }

    async fn unlock_write_commands(&self) -> anyhow::Result<()> {
        let mut stream = tokio::net::TcpStream::connect((self.ip, 4028))
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;

        let open_cmd = json!({
            "command": "open_write_api",
            "client": UNLOCK_CLIENT,
            "enable": true,
        });
        stream
            .write_all(open_cmd.to_string().as_bytes())
            .await
            .map_err(RPCError::from)?;

        let mut buf = vec![0u8; 4096];
        let n = tokio::time::timeout(DEFAULT_RPC_TIMEOUT, stream.read(&mut buf))
            .await
            .map_err(|_| RPCError::ReadTimeout)?
            .map_err(RPCError::from)?;
        let response: Value = serde_json::from_str(String::from_utf8_lossy(&buf[..n]).trim())?;

        let msg = response
            .get("Msg")
            .ok_or_else(|| anyhow::anyhow!("Missing Msg in open_write_api response"))?;
        let salt = msg["salt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing salt"))?;
        let newsalt = msg["newsalt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing newsalt"))?;
        let timestamp = msg["time"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing time"))?;

        let crypted = md5crypt("admin".as_bytes(), salt.as_bytes());
        let full_password = String::from_utf8_lossy(&crypted);
        let pwd_md5 = full_password
            .split('$')
            .nth(3)
            .ok_or_else(|| anyhow::anyhow!("Failed to extract md5crypt hash"))?;

        let token_data = format!("{}{}{}{}", timestamp, newsalt, UNLOCK_MAGIC, pwd_md5);
        let token_md5 = format!("{:x}", md5::compute(token_data.as_bytes()));

        let token_json = json!({ "token": token_md5 });
        stream
            .write_all(token_json.to_string().as_bytes())
            .await
            .map_err(RPCError::from)?;

        let mut final_buf = vec![0u8; 4096];
        let _ = tokio::time::timeout(DEFAULT_RPC_TIMEOUT, stream.read(&mut final_buf))
            .await
            .map_err(|_| RPCError::ReadTimeout)?
            .map_err(RPCError::from)?;

        let _ = stream.shutdown().await;
        Ok(())
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
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
        let salt = self
            .get_salt()
            .await
            .ok_or_else(|| anyhow::anyhow!("Could not get salt for privileged command"))?;

        let mut stream = tokio::net::TcpStream::connect((self.ip, self.port))
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;

        let timestamp = Utc::now().timestamp();

        let tokenized_command = format!(
            "{}{}{}{}",
            command,
            self.auth.password.expose_secret(),
            salt,
            timestamp
        );

        let hashed_command = Sha256::digest(tokenized_command.as_bytes());
        let encoded_command = BASE64_STANDARD.encode(hashed_command);
        let mut command_bytes = encoded_command.into_bytes();

        if command_bytes.len() > 8 {
            command_bytes[8] = 0;
            command_bytes = command_bytes[..8].to_vec();
        }

        let token = String::from_utf8_lossy(command_bytes.as_slice());

        let request = match parameters {
            Some(other) => {
                let param = if command == "set.miner.pools" {
                    json!(encrypt_param(&hashed_command, &other.to_string())?)
                } else {
                    other
                };
                json!({
                    "cmd": command,
                    "param": param,
                    "token": token,
                    "account": self.auth.username.clone(),
                    "ts": timestamp,
                })
            }
            None => {
                // No parameters at all
                json!({
                    "cmd": command,
                    "token": token,
                    "account": self.auth.username.clone(),
                    "ts": timestamp,
                })
            }
        };
        let json_str = request.to_string();
        let json_bytes = json_str.as_bytes();
        let length = json_bytes.len() as u32;

        stream
            .write_all(&length.to_le_bytes())
            .await
            .map_err(RPCError::from)?;
        stream.write_all(json_bytes).await.map_err(RPCError::from)?;

        let mut len_buf = [0u8; 4];
        read_exact_with_timeout(&mut stream, &mut len_buf, DEFAULT_RPC_TIMEOUT).await?;
        let response_len = u32::from_le_bytes(len_buf) as usize;

        let mut resp_buf = vec![0u8; response_len];
        read_exact_with_timeout(&mut stream, &mut resp_buf, DEFAULT_RPC_TIMEOUT).await?;

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

use std::{net::IpAddr, string::ToString};

use aes::{
    Aes256,
    cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit},
};
use anyhow;
use asic_rs_core::{
    data::command::{MinerCommand, RPCCommandStatus},
    errors::{RPCError, RPCError::StatusCheckFailed},
    traits::miner::*,
    util::{DEFAULT_RPC_TIMEOUT, read_stream_response},
};
use async_trait::async_trait;
use base64::prelude::*;
use ecb::cipher::block_padding::ZeroPadding;
use md5crypt::md5crypt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const UNLOCK_CLIENT: &str = "heatcore";
const UNLOCK_MAGIC: &str = "3804fe31981418ce711a31d94bc69651";

type Aes256EcbDec = ecb::Decryptor<Aes256>;
type Aes256EcbEnc = ecb::Encryptor<Aes256>;

struct TokenData {
    host_password_md5: String,
    host_sign: String,
}

impl TokenData {
    pub fn new(host_password_md5: String, host_sign: String) -> Self {
        Self {
            host_password_md5,
            host_sign,
        }
    }
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
fn add_to_16(input: &str) -> Vec<u8> {
    let mut bytes = input.as_bytes().to_vec();
    while !bytes.len().is_multiple_of(16) {
        bytes.push(0);
    }
    bytes
}

fn aes_ecb_enc(key: &str, data: &str) -> anyhow::Result<String> {
    let original_message = data.as_bytes(); // no manual padding
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let hashed_key = format!("{:x}", hasher.finalize());
    let aes_key = hex::decode(hashed_key).expect("SHA256 hex output is always valid hex");

    let mut buffer = add_to_16(data).to_vec();

    let enc = Aes256EcbEnc::new_from_slice(&aes_key)
        .expect("SHA256 always produces 32 bytes, valid for AES-256")
        .encrypt_padded_mut::<ZeroPadding>(&mut buffer, original_message.len())
        .map_err(|e| anyhow::anyhow!("AES encryption failed: {e:?}"))?;

    Ok(BASE64_STANDARD.encode(enc).replace('\n', ""))
}

fn aes_ecb_dec(key: &str, data: &str) -> anyhow::Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let hashed_key = format!("{:x}", hasher.finalize());
    let aes_key = hex::decode(hashed_key).expect("SHA256 hex output is always valid hex");

    let b64_dec = &mut BASE64_STANDARD.decode(data)?[..];

    let dec = Aes256EcbDec::new_from_slice(aes_key.as_slice())
        .expect("SHA256 always produces 32 bytes, valid for AES-256")
        .decrypt_padded_mut::<ZeroPadding>(b64_dec)
        .map_err(|e| anyhow::anyhow!("AES decryption failed: {e:?}"))?;

    Ok(String::from_utf8_lossy(dec).into_owned())
}

trait StatusFromBTMinerV2 {
    fn from_btminer_v2(response: &str) -> Result<Self, RPCError>
    where
        Self: Sized;
}

impl StatusFromBTMinerV2 for RPCCommandStatus {
    fn from_btminer_v2(response: &str) -> anyhow::Result<Self, RPCError> {
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
        let result = self
            .send_command_once(command, _privileged, parameters.clone())
            .await;
        match &result {
            Err(e)
                if e.downcast_ref::<RPCError>()
                    .is_some_and(|rpc| matches!(rpc, StatusCheckFailed(_))) =>
            {
                self.unlock_write_commands().await?;
                self.send_command_once(command, _privileged, parameters)
                    .await
            }
            _ => result,
        }
    }
}

impl WhatsMinerRPCAPI {
    async fn send_command_once(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        if _privileged || command.starts_with("set_") {
            return self.send_privileged_command(command, parameters).await;
        }

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

        stream.write_all(json_bytes).await.map_err(RPCError::from)?;

        let response = read_stream_response(&mut stream, DEFAULT_RPC_TIMEOUT).await;
        let _ = stream.shutdown().await;
        let response = response?;

        self.parse_rpc_result(&response)
    }

    async fn unlock_write_commands(&self) -> anyhow::Result<()> {
        let mut stream = tokio::net::TcpStream::connect((self.ip, self.port))
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

        let crypted = md5crypt(
            self.auth.password.expose_secret().as_bytes(),
            salt.as_bytes(),
        );
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

    pub fn new(ip: IpAddr, port: Option<u16>, auth: MinerAuth) -> Self {
        Self {
            ip,
            port: port.unwrap_or(4028),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    fn parse_rpc_result(&self, response: &str) -> anyhow::Result<Value> {
        let status = RPCCommandStatus::from_btminer_v2(response)?;
        match status.into_result() {
            Ok(_) => Ok(serde_json::from_str(response)?),
            Err(e) => Err(e)?,
        }
    }

    fn parse_privileged_rpc_result(&self, key: &str, response: &str) -> anyhow::Result<Value> {
        let enc_result = serde_json::from_str::<Value>(response)?;
        match enc_result.get("enc").and_then(|v| v.as_str()) {
            Some(enc_data) => {
                let result = aes_ecb_dec(key, enc_data)?;
                self.parse_rpc_result(&result)
            }
            None => self.parse_rpc_result(response),
        }
    }

    async fn get_token_data(&self) -> anyhow::Result<TokenData> {
        let api_token = self.send_command("get_token", false, None).await?;
        let salt = api_token
            .get("Msg")
            .and_then(|json| json.get("salt"))
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Could not get salt"))?;
        let new_salt = api_token
            .get("Msg")
            .and_then(|json| json.get("newsalt"))
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Could not get newsalt"))?;
        let api_time = api_token
            .get("Msg")
            .and_then(|json| json.get("time"))
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Could not get time"))?;

        let crypted = md5crypt(
            self.auth.password.expose_secret().as_bytes(),
            salt.as_bytes(),
        );
        let full_password = String::from_utf8_lossy(&crypted);
        let host_password_md5 = full_password
            .split("$")
            .nth(3)
            .ok_or(anyhow::anyhow!("Failed to extract md5crypt hash"))?;

        let new_crypted = md5crypt(
            format!("{}{}", host_password_md5, api_time).as_bytes(),
            new_salt.as_bytes(),
        );
        let full_host_sign = String::from_utf8_lossy(&new_crypted);
        let host_sign = full_host_sign
            .split("$")
            .nth(3)
            .ok_or(anyhow::anyhow!("Failed to extract host sign"))?;

        Ok(TokenData::new(
            host_password_md5.to_owned(),
            host_sign.to_owned(),
        ))
    }

    async fn send_privileged_command(
        &self,
        command: &str,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        let token_data = self.get_token_data().await?;

        let mut stream = tokio::net::TcpStream::connect((self.ip, self.port))
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;

        let request = match parameters {
            Some(Value::Object(mut obj)) => {
                // Use the existing object as the base
                obj.insert("command".to_string(), json!(command));
                obj.insert("token".to_string(), json!(token_data.host_sign));
                Value::Object(obj)
            }
            Some(other) => {
                // Wrap non-objects into the "param" key
                json!({ "command": command, "parameter": other, "token": token_data.host_sign })
            }
            None => {
                // No parameters at all
                json!({ "command": command, "token": token_data.host_sign })
            }
        };
        let enc = aes_ecb_enc(&token_data.host_password_md5, &request.to_string())?;
        let command = json!({"enc": 1, "data": enc});
        let json_str = command.to_string();
        let json_bytes = json_str.as_bytes();

        stream.write_all(json_bytes).await.map_err(RPCError::from)?;

        let response = read_stream_response(&mut stream, DEFAULT_RPC_TIMEOUT).await;
        let _ = stream.shutdown().await;
        let response = response?;

        self.parse_privileged_rpc_result(&token_data.host_password_md5, &response)
    }
}

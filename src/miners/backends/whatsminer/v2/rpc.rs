use aes::Aes256;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit};
use anyhow;
use async_trait::async_trait;
use base64::prelude::*;
use ecb::cipher::block_padding::ZeroPadding;
use md5crypt::md5crypt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::miners::api::rpc::errors::RPCError;
use crate::miners::api::rpc::status::RPCCommandStatus;
use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

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
#[allow(dead_code)]
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
fn add_to_16(input: &str) -> Vec<u8> {
    let mut bytes = input.as_bytes().to_vec();
    while !bytes.len().is_multiple_of(16) {
        bytes.push(0);
    }
    bytes
}

fn aes_ecb_enc(key: &str, data: &str) -> String {
    let original_message = data.as_bytes(); // no manual padding
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let hashed_key = format!("{:x}", hasher.finalize());
    let aes_key = hex::decode(hashed_key).unwrap();

    let mut buffer = add_to_16(data).to_vec();

    let enc = Aes256EcbEnc::new_from_slice(&aes_key)
        .unwrap()
        .encrypt_padded_mut::<ZeroPadding>(&mut buffer, original_message.len())
        .unwrap();

    BASE64_STANDARD.encode(enc).replace('\n', "")
}

fn aes_ecb_dec(key: &str, data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let hashed_key = format!("{:x}", hasher.finalize());
    let aes_key = hex::decode(hashed_key).unwrap();

    let b64_dec = &mut BASE64_STANDARD.decode(data).unwrap()[..];

    let dec = Aes256EcbDec::new_from_slice(aes_key.as_slice())
        .unwrap()
        .decrypt_padded_mut::<ZeroPadding>(b64_dec)
        .unwrap();

    String::from_utf8_lossy(dec).into_owned()
}

impl RPCCommandStatus {
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

        stream.write_all(json_bytes).await?;

        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).await?;

        let response = String::from_utf8_lossy(&buffer)
            .into_owned()
            .replace('\0', "");

        self.parse_rpc_result(&response)
    }
}

impl WhatsMinerRPCAPI {
    pub fn new(ip: IpAddr, port: Option<u16>) -> Self {
        Self {
            ip,
            port: port.unwrap_or(4028),
            user: "admin".to_string(),
            password: "admin".to_string(),
        }
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
        let result = aes_ecb_dec(key, enc_result.get("enc").unwrap().as_str().unwrap());

        self.parse_rpc_result(&result)
    }

    async fn get_token_data(&self) -> anyhow::Result<TokenData> {
        let api_token = self.send_command("get_token", false, None).await?;
        let salt = api_token
            .get("Msg")
            .and_then(|json| json.get("salt"))
            .ok_or(anyhow::anyhow!("Could not get salt"))?
            .as_str()
            .unwrap();
        let new_salt = api_token
            .get("Msg")
            .and_then(|json| json.get("newsalt"))
            .ok_or(anyhow::anyhow!("Could not get newsalt"))?
            .as_str()
            .unwrap();
        let api_time = api_token
            .get("Msg")
            .and_then(|json| json.get("time"))
            .ok_or(anyhow::anyhow!("Could not get time"))?
            .as_str()
            .unwrap();

        let crypted = md5crypt(self.password.as_bytes(), salt.as_bytes());
        let full_password = String::from_utf8_lossy(&crypted);
        let host_password_md5 = full_password.split("$").nth(3).unwrap();

        let new_crypted = md5crypt(
            format!("{}{}", host_password_md5, api_time).as_bytes(),
            new_salt.as_bytes(),
        );
        let full_host_sign = String::from_utf8_lossy(&new_crypted);
        let host_sign = full_host_sign.split("$").nth(3).unwrap();

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
        let enc = aes_ecb_enc(&token_data.host_password_md5, &request.to_string());
        let command = json!({"enc": 1, "data": enc});
        let json_str = command.to_string();
        let json_bytes = json_str.as_bytes();

        stream.write_all(json_bytes).await?;

        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).await?;

        let response = String::from_utf8_lossy(&buffer)
            .into_owned()
            .replace('\0', "");

        self.parse_privileged_rpc_result(&token_data.host_password_md5, &response)
    }
}

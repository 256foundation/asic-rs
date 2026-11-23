use std::{net::IpAddr, string::ToString, time::Duration};

use aes::{
    Aes256,
    cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit},
};
use anyhow::{Context, Result, ensure};
use asic_rs_core::{
    data::{
        command::{MinerCommand, RPCCommandStatus},
        firmware::FirmwareImage,
    },
    errors::{RPCError, RPCError::StatusCheckFailed},
    traits::miner::*,
    util::{DEFAULT_RPC_TIMEOUT, connect_tcp_stream, write_all_with_timeout},
};
use async_trait::async_trait;
use base64::prelude::*;
use ecb::cipher::block_padding::ZeroPadding;
use md5crypt::md5crypt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::{io::AsyncReadExt, net::TcpStream, time::timeout};

use crate::backends::shared::update::{
    self, COMMAND_DEADLINE, Device, Package, UploadOutcomeUnconfirmed,
};

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
    let aes_key =
        hex::decode(hashed_key).map_err(|e| anyhow::anyhow!("invalid SHA256 hex output: {e}"))?;

    let mut buffer = add_to_16(data).to_vec();

    let enc = Aes256EcbEnc::new_from_slice(&aes_key)
        .map_err(|e| anyhow::anyhow!("invalid AES-256 key length: {e:?}"))?
        .encrypt_padded_mut::<ZeroPadding>(&mut buffer, original_message.len())
        .map_err(|e| anyhow::anyhow!("AES encryption failed: {e:?}"))?;

    Ok(BASE64_STANDARD.encode(enc).replace('\n', ""))
}

fn aes_ecb_dec(key: &str, data: &str) -> anyhow::Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let hashed_key = format!("{:x}", hasher.finalize());
    let aes_key =
        hex::decode(hashed_key).map_err(|e| anyhow::anyhow!("invalid SHA256 hex output: {e}"))?;

    let b64_dec = &mut BASE64_STANDARD.decode(data)?[..];

    let dec = Aes256EcbDec::new_from_slice(aes_key.as_slice())
        .map_err(|e| anyhow::anyhow!("invalid AES-256 key length: {e:?}"))?
        .decrypt_padded_mut::<ZeroPadding>(b64_dec)
        .map_err(|e| anyhow::anyhow!("AES decryption failed: {e:?}"))?;

    Ok(String::from_utf8_lossy(dec).into_owned())
}

trait StatusFromBTMinerV2 {
    fn from_btminer_v2(data: &Value) -> Result<Self, RPCError>
    where
        Self: Sized;
}

impl StatusFromBTMinerV2 for RPCCommandStatus {
    fn from_btminer_v2(data: &Value) -> anyhow::Result<Self, RPCError> {
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
        let mut stream = connect_tcp_stream((self.ip, self.port), DEFAULT_RPC_TIMEOUT)
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;
        Self::send_message(&mut stream, &request, DEFAULT_RPC_TIMEOUT).await?;
        let response =
            Self::receive_message(&mut stream, &mut Vec::new(), DEFAULT_RPC_TIMEOUT).await?;
        self.validate_rpc_response(response)
    }

    async fn unlock_write_commands(&self) -> anyhow::Result<()> {
        let mut stream = connect_tcp_stream((self.ip, self.port), DEFAULT_RPC_TIMEOUT)
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;

        let open_cmd = json!({
            "command": "open_write_api",
            "client": UNLOCK_CLIENT,
            "enable": true,
        });
        write_all_with_timeout(
            &mut stream,
            open_cmd.to_string().as_bytes(),
            DEFAULT_RPC_TIMEOUT,
        )
        .await?;

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

        let crypted = md5crypt(self.auth.password().as_bytes(), salt.as_bytes());
        let full_password = String::from_utf8_lossy(&crypted);
        let pwd_md5 = full_password
            .split('$')
            .nth(3)
            .ok_or_else(|| anyhow::anyhow!("Failed to extract md5crypt hash"))?;

        let token_data = format!("{}{}{}{}", timestamp, newsalt, UNLOCK_MAGIC, pwd_md5);
        let token_md5 = format!("{:x}", md5::compute(token_data.as_bytes()));

        let token_json = json!({ "token": token_md5 });
        write_all_with_timeout(
            &mut stream,
            token_json.to_string().as_bytes(),
            DEFAULT_RPC_TIMEOUT,
        )
        .await?;

        let mut final_buf = vec![0u8; 4096];
        let _ = tokio::time::timeout(DEFAULT_RPC_TIMEOUT, stream.read(&mut final_buf))
            .await
            .map_err(|_| RPCError::ReadTimeout)?
            .map_err(RPCError::from)?;

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

    fn validate_rpc_response(&self, response: Value) -> anyhow::Result<Value> {
        RPCCommandStatus::from_btminer_v2(&response)?.into_result()?;
        Ok(response)
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

        let crypted = md5crypt(self.auth.password().as_bytes(), salt.as_bytes());
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
        let mut stream = connect_tcp_stream((self.ip, self.port), DEFAULT_RPC_TIMEOUT)
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;
        Self::send_message(&mut stream, &command, DEFAULT_RPC_TIMEOUT).await?;
        let response =
            Self::receive_message(&mut stream, &mut Vec::new(), DEFAULT_RPC_TIMEOUT).await?;
        self.validate_rpc_response(Self::unwrap_reply(response, &token_data.host_password_md5)?)
    }

    /// Selects an upload payload from a combined firmware bundle.
    ///
    /// Although both use `FirmwareImage`, the input is a bundle and the output is
    /// an extracted member. The output cannot be passed back to `upgrade_firmware`,
    /// which expects a bundle, so this helper is kept internal rather than exposed
    /// through `UpgradeFirmware::prepare_firmware`.
    async fn prepare_firmware(&self, image: FirmwareImage) -> Result<FirmwareImage> {
        let package = Package::parse(image.bytes)?;
        let response = self.send_command("get_version", false, None).await?;
        ensure!(
            response["STATUS"] == "S",
            "WhatsMiner firmware version lookup requires a successful status"
        );
        let msg = &response["Msg"];
        ensure!(
            update::field(msg, "api_ver")?.starts_with("2."),
            "unexpected API version for this firmware endpoint"
        );
        let platform = update::field(msg, "platform")?.to_ascii_lowercase();
        let is_btrom = update::parse_btrom(msg)?;
        let device = Device {
            platform_version: update::platform_version(
                &platform,
                "",
                update::field(msg, "fw_ver")?,
            ),
            platform,
            chip: update::chip_selector(&msg["chip"], &msg["miner_type"]),
            is_btrom,
            clock: response["When"].as_u64().context("missing miner clock")?,
        };
        package.select(image.filename, device)
    }

    pub(super) async fn upgrade_firmware(&self, image: FirmwareImage) -> Result<bool> {
        let image = self.prepare_firmware(image).await?;
        self.upload(&image.bytes).await?;
        Ok(true)
    }

    /// Writes one unframed V2 JSON message within the given timeout.
    async fn send_message(
        socket: &mut TcpStream,
        message: &Value,
        deadline: Duration,
    ) -> Result<()> {
        let encoded = serde_json::to_vec(message)?;
        write_all_with_timeout(socket, &encoded, deadline).await?;
        Ok(())
    }

    /// Reads one JSON object, retaining any following replies for the next call.
    async fn receive_message(
        socket: &mut TcpStream,
        buffered: &mut Vec<u8>,
        deadline: Duration,
    ) -> Result<Value> {
        timeout(deadline, async {
            loop {
                let padding = buffered
                    .iter()
                    .take_while(|byte| **byte == 0 || byte.is_ascii_whitespace())
                    .count();
                buffered.drain(..padding);
                let mut values =
                    serde_json::Deserializer::from_slice(buffered).into_iter::<Value>();
                if let Some(result) = values.next() {
                    match result {
                        Ok(value) => {
                            let used = values.byte_offset();
                            buffered.drain(..used);
                            ensure!(value.is_object(), "expected a WhatsMiner response object");
                            return Ok(value);
                        }
                        Err(error) if error.is_eof() => {}
                        Err(error) => return Err(RPCError::DeserializationFailed(error).into()),
                    }
                }
                let mut chunk = [0u8; 2048];
                let count = socket.read(&mut chunk).await.map_err(RPCError::from)?;
                ensure!(count != 0, "WhatsMiner closed an incomplete response");
                buffered.extend_from_slice(&chunk[..count]);
            }
        })
        .await
        .map_err(|_| RPCError::ReadTimeout)?
    }

    fn require_ack(response: &Value, expected: &str) -> Result<()> {
        ensure!(
            response["STATUS"] == "S" && response["Code"] == 131 && response["Msg"] == expected,
            "WhatsMiner update did not acknowledge {expected} (code {}, {})",
            response["Code"],
            match response["Code"].as_u64() {
                Some(45) => "access denied",
                Some(135) => "authentication failed",
                Some(132) => "command rejected",
                Some(14) => "unsupported command",
                _ => "unexpected reply",
            }
        );
        Ok(())
    }

    fn unwrap_reply(response: Value, password_hash: &str) -> Result<Value> {
        if let Some(encrypted) = response.get("enc") {
            let encrypted = encrypted
                .as_str()
                .context("invalid encrypted response envelope")?;
            let plaintext = aes_ecb_dec(password_hash, encrypted)?;
            Ok(serde_json::from_str(&plaintext).map_err(RPCError::from)?)
        } else {
            Ok(response)
        }
    }

    async fn upload(&self, member: &[u8]) -> Result<()> {
        ensure!(
            !self.auth.password().is_empty(),
            "stock API password is required"
        );
        let token_data = self.get_token_data().await?;
        let password_hash = token_data.host_password_md5;
        let encrypted = aes_ecb_enc(
            &password_hash,
            &json!({"cmd":"update_firmware", "token":token_data.host_sign}).to_string(),
        )?;
        let mut socket = connect_tcp_stream((self.ip, self.port), COMMAND_DEADLINE).await?;
        let mut buffered = Vec::new();
        Self::send_message(
            &mut socket,
            &json!({"enc":1, "data":encrypted}),
            COMMAND_DEADLINE,
        )
        .await?;
        let ready = Self::unwrap_reply(
            Self::receive_message(&mut socket, &mut buffered, COMMAND_DEADLINE).await?,
            &password_hash,
        )?;
        Self::require_ack(&ready, "ready").context("firmware rejected before transfer")?;

        update::transfer(&mut socket, member).await?;
        let acknowledgement =
            match Self::receive_message(&mut socket, &mut buffered, COMMAND_DEADLINE)
                .await
                .and_then(|value| Self::unwrap_reply(value, &password_hash))
            {
                Ok(value) => value,
                Err(_) => {
                    return Err(UploadOutcomeUnconfirmed {
                        stage: "final acknowledgement unavailable",
                    }
                    .into());
                }
            };
        if acknowledgement["STATUS"] == "E" {
            return Self::require_ack(&acknowledgement, "API command OK")
                .context("firmware handler rejected submission");
        }
        if Self::require_ack(&acknowledgement, "API command OK").is_err() {
            return Err(UploadOutcomeUnconfirmed {
                stage: "unexpected final acknowledgement",
            }
            .into());
        }
        Ok(())
    }
}

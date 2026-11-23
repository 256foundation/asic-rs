use std::net::IpAddr;
use std::time::Duration;

use aes::{
    Aes256,
    cipher::{BlockEncryptMut, KeyInit},
};
use anyhow::{Context, Result, bail, ensure};
use asic_rs_core::errors::RPCError::StatusCheckFailed;
use asic_rs_core::{
    data::{
        command::{MinerCommand, RPCCommandStatus},
        firmware::FirmwareImage,
    },
    errors::RPCError,
    traits::miner::*,
    util::{DEFAULT_RPC_TIMEOUT, connect_tcp_stream, write_all_with_timeout},
};
use async_trait::async_trait;
use base64::prelude::*;
use ecb::cipher::block_padding::ZeroPadding;
use jiff::Timestamp;
use md5crypt::md5crypt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::{io::AsyncReadExt, net::TcpStream, time::timeout};

use crate::backends::shared::update::{
    self, COMMAND_DEADLINE, Device, Package, UploadOutcomeUnconfirmed,
};

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

fn signed_token(command: &str, password: &str, salt: &str) -> (i64, [u8; 32], String) {
    let timestamp = Timestamp::now().as_second();
    let digest = Sha256::digest(format!("{command}{password}{salt}{timestamp}")).into();
    let token = BASE64_STANDARD.encode(digest);
    (timestamp, digest, token[..8].to_owned())
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
    fn from_btminer_v3(value: &Value) -> Result<Self, RPCError>
    where
        Self: Sized;
}

impl StatusFromBTMinerV3 for RPCCommandStatus {
    fn from_btminer_v3(value: &Value) -> anyhow::Result<Self, RPCError> {
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

        let mut stream = connect_tcp_stream((self.ip, self.port), DEFAULT_RPC_TIMEOUT)
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
        Self::send_message(&mut stream, &request, DEFAULT_RPC_TIMEOUT).await?;
        let response = Self::receive_message(&mut stream, DEFAULT_RPC_TIMEOUT).await?;
        self.validate_rpc_response(response)
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
        let mut stream = connect_tcp_stream((self.ip, 4028), DEFAULT_RPC_TIMEOUT)
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

        let crypted = md5crypt("admin".as_bytes(), salt.as_bytes());
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

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    fn validate_rpc_response(&self, response: Value) -> anyhow::Result<Value> {
        RPCCommandStatus::from_btminer_v3(&response)?.into_result()?;
        Ok(response)
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

        let mut stream = connect_tcp_stream((self.ip, self.port), DEFAULT_RPC_TIMEOUT)
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;

        let (timestamp, hashed_command, token) = signed_token(command, self.auth.password(), &salt);
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
                    "account": self.auth.username().to_string(),
                    "ts": timestamp,
                })
            }
            None => {
                // No parameters at all
                json!({
                    "cmd": command,
                    "token": token,
                    "account": self.auth.username().to_string(),
                    "ts": timestamp,
                })
            }
        };
        Self::send_message(&mut stream, &request, DEFAULT_RPC_TIMEOUT).await?;
        let response = Self::receive_message(&mut stream, DEFAULT_RPC_TIMEOUT).await?;
        self.validate_rpc_response(response)
    }

    async fn get_salt(&self) -> Option<String> {
        self.send_command("get.device.info", false, Some(json!("salt")))
            .await
            .ok()
            .and_then(|s| s["msg"]["salt"].as_str().map(|s| s.to_string()))
    }

    /// Selects an upload payload from a combined firmware bundle.
    ///
    /// Although both use `FirmwareImage`, the input is a bundle and the output is
    /// an extracted member. The output cannot be passed back to `upgrade_firmware`,
    /// which expects a bundle, so this helper is kept internal rather than exposed
    /// through `UpgradeFirmware::prepare_firmware`.
    async fn prepare_firmware(&self, image: FirmwareImage) -> Result<FirmwareImage> {
        let package = Package::parse(image.bytes)?;
        let response = self.send_command("get.device.info", false, None).await?;
        let msg = &response["msg"]["system"];
        let miner = &response["msg"]["miner"];
        ensure!(
            update::field(msg, "api")?.starts_with("3."),
            "unexpected API version for this firmware endpoint"
        );
        let platform = update::field(msg, "platform")?.to_ascii_lowercase();
        let is_btrom = update::parse_btrom(msg)?;
        let device = Device {
            platform_version: update::platform_version(
                &platform,
                msg["control-board-version"].as_str().unwrap_or_default(),
                update::field(msg, "fwversion")?,
            ),
            platform,
            chip: update::chip_selector(&miner["chipdata0"], &miner["type"]),
            is_btrom,
            clock: response["when"].as_u64().context("missing miner clock")?,
        };
        package.select(image.filename, device)
    }

    pub(super) async fn upgrade_firmware(&self, image: FirmwareImage) -> Result<bool> {
        let image = self.prepare_firmware(image).await?;
        self.upload(&image.bytes).await?;
        Ok(true)
    }

    /// Writes a length-prefixed JSON frame, allowing each write the given timeout.
    async fn send_message(
        socket: &mut TcpStream,
        message: &Value,
        deadline: Duration,
    ) -> Result<()> {
        let encoded = serde_json::to_vec(message)?;
        let length = u32::try_from(encoded.len())?;
        write_all_with_timeout(socket, &length.to_le_bytes(), deadline).await?;
        write_all_with_timeout(socket, &encoded, deadline).await?;
        Ok(())
    }

    /// Reads and decodes one JSON frame within a single response timeout.
    async fn receive_message(socket: &mut TcpStream, deadline: Duration) -> Result<Value> {
        let mut bytes = Vec::new();
        timeout(deadline, async {
            let length = socket.read_u32_le().await.map_err(RPCError::from)?;
            let count = (&mut *socket)
                .take(u64::from(length))
                .read_to_end(&mut bytes)
                .await
                .map_err(RPCError::from)?;
            ensure!(count == length as usize, "incomplete V3 response");
            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|_| RPCError::ReadTimeout)??;
        Ok(serde_json::from_slice(&bytes).map_err(RPCError::from)?)
    }

    async fn upload(&self, member: &[u8]) -> Result<()> {
        const COMMAND: &str = "set.system.update_firmware";
        let salt = self
            .get_salt()
            .await
            .filter(|salt| !salt.is_empty())
            .context("missing V3 salt")?;
        let (ts, _, token) = signed_token(COMMAND, self.auth.password(), &salt);
        let mut socket = connect_tcp_stream((self.ip, self.port), COMMAND_DEADLINE).await?;
        Self::send_message(
            &mut socket,
            &json!({"cmd": COMMAND, "ts": ts, "token": token, "account": self.auth.username()}),
            COMMAND_DEADLINE,
        )
        .await?;
        let ready = Self::receive_message(&mut socket, COMMAND_DEADLINE).await?;
        ensure!(
            ready["code"] == 0 && ready["msg"] == "ready",
            "V3 firmware rejected before transfer (code {})",
            ready["code"]
        );
        update::transfer(&mut socket, member).await?;
        let response = Self::receive_message(&mut socket, Duration::from_secs(100))
            .await
            .map_err(|_| UploadOutcomeUnconfirmed {
                stage: "final V3 confirmation unavailable",
            })?;
        match (
            response["desc"].as_str(),
            response["code"].as_i64(),
            response["msg"].as_str(),
        ) {
            (Some(COMMAND), Some(0), Some("ok")) => Ok(()),
            (Some(COMMAND), Some(code), Some(message)) if code != 0 => {
                bail!("V3 firmware handler rejected submission (code {code}): {message}")
            }
            _ => Err(UploadOutcomeUnconfirmed {
                stage: "unexpected final V3 confirmation",
            }
            .into()),
        }
    }
}

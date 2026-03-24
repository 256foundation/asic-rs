use std::{net::IpAddr, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use asic_rs_core::data::firmware::FirmwareImage;
use asic_rs_core::{data::command::MinerCommand, traits::miner::*};
use async_trait::async_trait;
use diqwest::WithDigestAuth;
use reqwest::{Client, Method, Response, header::CONTENT_TYPE};
use serde_json::{Value, json};

use super::firmware::AntMinerFirmwareUpgradeResponseExt;

#[derive(Debug)]
pub struct AntMinerWebAPI {
    ip: IpAddr,
    port: u16,
    client: Client,
    timeout: Duration,
    auth: MinerAuth,
}

#[allow(dead_code)]
impl AntMinerWebAPI {
    const FIRMWARE_UPLOAD_BOUNDARY: &str = "asic-rs-antminer-firmware-boundary";

    fn truncated_firmware_upgrade_response_body(body: &str) -> String {
        const MAX_BODY_CHARS: usize = 512;

        let truncated: String = body.chars().take(MAX_BODY_CHARS).collect();
        if body.chars().count() > MAX_BODY_CHARS {
            format!("{truncated}...")
        } else {
            truncated
        }
    }

    fn build_firmware_upload_request_body(image: FirmwareImage) -> (Vec<u8>, String) {
        let FirmwareImage { filename, bytes } = image;
        let payload_len = bytes.len();
        let mut body = Vec::with_capacity(payload_len + 256);
        body.extend_from_slice(
            format!(
                "--{}\r\nContent-Disposition: form-data; name=\"firmware\"; filename=\"{}\"\r\nContent-Type: application/octet-stream\r\n\r\n",
                Self::FIRMWARE_UPLOAD_BOUNDARY,
                filename
            )
            .as_bytes(),
        );
        body.extend_from_slice(&bytes);
        body.extend_from_slice(
            format!("\r\n--{}--\r\n", Self::FIRMWARE_UPLOAD_BOUNDARY).as_bytes(),
        );
        (
            body,
            format!(
                "multipart/form-data; boundary={}",
                Self::FIRMWARE_UPLOAD_BOUNDARY
            ),
        )
    }

    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            ip,
            port: 80,
            client,
            timeout: Duration::from_secs(5),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    pub fn with_timeout(ip: IpAddr, timeout: Duration, auth: MinerAuth) -> Self {
        let mut client = Self::new(ip, auth);
        client.port = 80;
        client.timeout = timeout;
        client
    }

    async fn send_web_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        let url = format!("http://{}:{}/cgi-bin/{}.cgi", self.ip, self.port, command);

        let response = self
            .execute_web_request(&url, &method, parameters.clone())
            .await?;

        let status = response.status();
        if status.is_success() {
            let json_data = response.json().await.map_err(|e| anyhow!(e.to_string()))?;
            Ok(json_data)
        } else {
            bail!("HTTP request failed with status code {}", status);
        }
    }

    async fn execute_web_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
    ) -> Result<Response> {
        let response = match *method {
            Method::GET => self
                .client
                .get(url)
                .timeout(self.timeout)
                .send_digest_auth((
                    self.auth.username.as_str(),
                    self.auth.password.expose_secret(),
                ))
                .await
                .map_err(|e| anyhow!(e.to_string()))?,
            Method::POST => {
                let data = parameters.unwrap_or_else(|| json!({}));
                self.client
                    .post(url)
                    .json(&data)
                    .timeout(self.timeout)
                    .send_digest_auth((
                        self.auth.username.as_str(),
                        self.auth.password.expose_secret(),
                    ))
                    .await
                    .map_err(|e| anyhow!(e.to_string()))?
            }
            _ => bail!("Unsupported method: {}", method),
        };

        Ok(response)
    }

    pub async fn get_miner_conf(&self) -> Result<Value> {
        self.send_web_command("get_miner_conf", false, None, Method::GET)
            .await
    }

    pub async fn set_miner_conf(&self, conf: Value) -> Result<Value> {
        self.send_web_command("set_miner_conf", false, Some(conf), Method::POST)
            .await
    }

    pub async fn blink(&self, blink: bool) -> Result<Value> {
        let param = if blink {
            json!({"blink": "true"})
        } else {
            json!({"blink": "false"})
        };
        self.send_web_command("blink", false, Some(param), Method::POST)
            .await
    }

    pub async fn reboot(&self) -> Result<Value> {
        self.send_web_command("reboot", false, None, Method::POST)
            .await
    }

    pub async fn upgrade_firmware(&self, image: FirmwareImage) -> Result<()> {
        let url = format!("http://{}:{}/cgi-bin/upgrade.cgi", self.ip, self.port);
        let (body, content_type) = Self::build_firmware_upload_request_body(image);

        let response = self
            .client
            .post(url)
            .header(CONTENT_TYPE, content_type)
            .body(body)
            .timeout(self.timeout.max(Duration::from_secs(60)))
            .send_digest_auth((
                self.auth.username.as_str(),
                self.auth.password.expose_secret(),
            ))
            .await
            .with_context(|| "firmware upload HTTP request failed".to_string())?;

        let status = response.status();
        let body = response
            .text()
            .await
            .with_context(|| "failed to read firmware upload response body".to_string())?;
        if !status.is_success() {
            bail!(
                "Firmware upload failed with status code {}: {}",
                status,
                Self::truncated_firmware_upgrade_response_body(&body)
            );
        }

        body.validate_firmware_upgrade_response()
    }

    pub async fn get_system_info(&self) -> Result<Value> {
        self.send_web_command("get_system_info", false, None, Method::GET)
            .await
    }

    pub async fn miner_type(&self) -> Result<Value> {
        self.send_web_command("miner_type", false, None, Method::GET)
            .await
    }

    pub async fn get_network_info(&self) -> Result<Value> {
        self.send_web_command("get_network_info", false, None, Method::GET)
            .await
    }

    pub async fn summary(&self) -> Result<Value> {
        self.send_web_command("summary", false, None, Method::GET)
            .await
    }

    pub async fn get_blink_status(&self) -> Result<Value> {
        self.send_web_command("get_blink_status", false, None, Method::GET)
            .await
    }

    pub async fn set_network_conf(
        &self,
        ip: String,
        dns: String,
        gateway: String,
        subnet_mask: String,
        hostname: String,
        protocol: u8,
    ) -> Result<Value> {
        let config = json!({
            "ipAddress": ip,
            "ipDns": dns,
            "ipGateway": gateway,
            "ipHost": hostname,
            "ipPro": protocol,
            "ipSub": subnet_mask
        });
        self.send_web_command("set_network_conf", false, Some(config), Method::POST)
            .await
    }
}

#[async_trait]
impl APIClient for AntMinerWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command,
                parameters,
            } => self
                .send_web_command(command, false, parameters.clone(), Method::GET)
                .await
                .map_err(|e| anyhow!(e.to_string())),
            _ => Err(anyhow!("Unsupported command type for Web client")),
        }
    }
}

#[async_trait]
impl WebAPIClient for AntMinerWebAPI {
    async fn send_command(
        &self,
        command: &str,
        privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        self.send_web_command(command, privileged, parameters, method)
            .await
    }
}

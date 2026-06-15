use std::{net::IpAddr, time::Duration};

use once_cell::sync::OnceCell;

use anyhow::{Context, Result, anyhow, bail};
use asic_rs_core::data::firmware::FirmwareImage;
use asic_rs_core::{data::command::MinerCommand, traits::miner::*};
use async_trait::async_trait;
use diqwest::WithDigestAuth;
use reqwest::{Client, Method, Response, multipart};
use serde_json::{Value, json};

use super::firmware::AntMinerFirmwareUpgradeResponseExt;

#[derive(Debug)]
pub struct AntMinerWebAPI {
    ip: IpAddr,
    port: u16,
    client: OnceCell<Client>,
    timeout: Duration,
    auth: MinerAuth,
}

#[allow(dead_code)]
impl AntMinerWebAPI {
    fn truncated_firmware_upgrade_response_body(body: &str) -> String {
        const MAX_BODY_CHARS: usize = 512;

        let truncated: String = body.chars().take(MAX_BODY_CHARS).collect();
        if body.chars().count() > MAX_BODY_CHARS {
            format!("{truncated}...")
        } else {
            truncated
        }
    }

    fn build_firmware_upload_form(image: FirmwareImage) -> Result<multipart::Form> {
        let FirmwareImage { filename, bytes } = image;
        let part = multipart::Part::bytes(bytes)
            .file_name(filename)
            .mime_str("application/octet-stream")
            .context("failed to set firmware part mime type")?;
        Ok(multipart::Form::new().part("firmware", part))
    }

    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            ip,
            port: 80,
            client: OnceCell::new(),
            timeout: Duration::from_secs(5),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    pub fn username(&self) -> &str {
        self.auth.username.as_str()
    }

    pub fn with_timeout(ip: IpAddr, timeout: Duration, auth: MinerAuth) -> Self {
        let mut client = Self::new(ip, auth);
        client.port = 80;
        client.timeout = timeout;
        client
    }

    fn build_client() -> Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed to create HTTP client")
    }

    fn client(&self) -> Result<&Client> {
        self.client.get_or_try_init(Self::build_client)
    }

    async fn send_web_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        self.send_web_command_with_timeout(command, _privileged, parameters, method, self.timeout)
            .await
    }

    async fn send_web_command_with_timeout(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
        timeout: Duration,
    ) -> Result<Value> {
        let url = format!("http://{}:{}/cgi-bin/{}.cgi", self.ip, self.port, command);

        let response = self
            .execute_web_request(&url, &method, parameters.clone(), timeout)
            .await?;

        let status = response.status();
        if status.is_success() {
            let json_data = response.json().await.map_err(|e| anyhow!(e.to_string()))?;
            Ok(json_data)
        } else {
            bail!("HTTP request failed with status code {}", status);
        }
    }

    async fn send_web_text_command(
        &self,
        command: &str,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<String> {
        let url = format!("http://{}:{}/cgi-bin/{}.cgi", self.ip, self.port, command);
        let response = self
            .execute_web_request(&url, &method, parameters, self.timeout)
            .await?;

        let status = response.status();
        if !status.is_success() {
            bail!("HTTP request failed with status code {}", status);
        }

        response.text().await.map_err(|e| anyhow!(e.to_string()))
    }

    async fn send_web_status_command(
        &self,
        command: &str,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<bool> {
        let url = format!("http://{}:{}/cgi-bin/{}.cgi", self.ip, self.port, command);
        let response = self
            .execute_web_request(&url, &method, parameters, self.timeout)
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(true)
        } else {
            bail!("HTTP request failed with status code {}", status);
        }
    }

    async fn execute_web_request(
        &self,
        url: &str,
        method: &Method,
        parameters: Option<Value>,
        timeout: Duration,
    ) -> Result<Response> {
        let client = self.client()?;

        let response = match *method {
            Method::GET => client
                .get(url)
                .timeout(timeout)
                .send_digest_auth((
                    self.auth.username.as_str(),
                    self.auth.password.expose_secret(),
                ))
                .await
                .map_err(|e| anyhow!(e.to_string()))?,
            Method::POST => {
                let data = parameters.unwrap_or_else(|| json!({}));
                client
                    .post(url)
                    .json(&data)
                    .timeout(timeout)
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
        self.send_web_command_with_timeout(
            "set_miner_conf",
            false,
            Some(conf),
            Method::POST,
            self.timeout.max(Duration::from_secs(30)),
        )
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

    pub async fn read_logs(&self) -> Result<String> {
        self.send_web_text_command("log", None, Method::GET).await
    }

    pub async fn factory_reset(&self) -> Result<bool> {
        self.send_web_status_command("reset_conf", None, Method::POST)
            .await
    }

    pub async fn change_password(&self, password: &str) -> Result<bool> {
        let payload = json!({
            "curPwd": self.auth.password.expose_secret(),
            "newPwd": password,
            "confirmPwd": password,
        });
        let response = self
            .send_web_command("passwd", true, Some(payload), Method::POST)
            .await?;
        Ok(response.get("stats").and_then(Value::as_str) == Some("success"))
    }

    pub async fn upgrade_firmware(&self, image: FirmwareImage) -> Result<()> {
        let url = format!("http://{}:{}/cgi-bin/upgrade.cgi", self.ip, self.port);
        let form = Self::build_firmware_upload_form(image)?;

        let response = self
            .client()?
            .post(url)
            .multipart(form)
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

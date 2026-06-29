use std::{net::IpAddr, time::Duration};

use once_cell::sync::OnceCell;

use anyhow::{Context, Result, anyhow, bail};
use asic_rs_core::{config::pools::PoolConfig, data::command::MinerCommand, traits::miner::*};
use async_trait::async_trait;
use diqwest::WithDigestAuth;
use reqwest::{Client, Method, Response};
use serde_json::{Value, json};
use tokio::time::sleep;
use url::form_urlencoded;

use super::{config_form, status_parser};

#[derive(Debug)]
pub struct VolcMinerWebAPI {
    ip: IpAddr,
    port: u16,
    client: OnceCell<Client>,
    timeout: Duration,
    auth: MinerAuth,
}

#[allow(dead_code)]
impl VolcMinerWebAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            ip,
            port: 80,
            client: OnceCell::new(),
            timeout: Duration::from_secs(10),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    pub fn auth(&self) -> MinerAuth {
        self.auth.clone()
    }

    pub fn username(&self) -> &str {
        self.auth.username()
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

    fn form_value(value: &Value) -> String {
        match value {
            Value::Null => String::new(),
            Value::String(value) => value.clone(),
            Value::Bool(value) => value.to_string(),
            Value::Number(value) => value.to_string(),
            Value::Array(_) | Value::Object(_) => value.to_string(),
        }
    }

    fn post_body(parameters: Option<Value>) -> Result<Option<String>> {
        match parameters {
            None => Ok(None),
            Some(Value::String(body)) => Ok(Some(body)),
            Some(Value::Object(fields)) => {
                let mut serializer = form_urlencoded::Serializer::new(String::new());
                for (key, value) in fields {
                    if let Value::Array(values) = value {
                        for value in values {
                            serializer.append_pair(&key, &Self::form_value(&value));
                        }
                    } else {
                        serializer.append_pair(&key, &Self::form_value(&value));
                    }
                }
                Ok(Some(serializer.finish()))
            }
            Some(_) => bail!("VolcMiner POST parameters must be an object or raw form body string"),
        }
    }

    async fn send_web_command(
        &self,
        command: &str,
        privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        self.send_web_command_with_timeout(command, privileged, parameters, method, self.timeout)
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
            let text = response.text().await.map_err(|e| anyhow!(e.to_string()))?;
            Self::parse_response_json(command, &text)
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
            Method::GET => {
                if parameters.is_some() {
                    bail!("VolcMiner GET commands do not support parameters");
                }
                client
                    .get(url)
                    .timeout(timeout)
                    .send_digest_auth((self.auth.username(), self.auth.password()))
                    .await
                    .map_err(|e| anyhow!(e.to_string()))?
            }
            Method::POST => {
                let body = Self::post_body(parameters)?;
                let mut builder = client.post(url).timeout(timeout);
                if let Some(body) = body {
                    builder = builder
                        .header("Content-Type", "application/x-www-form-urlencoded")
                        .body(body);
                }
                builder
                    .send_digest_auth((self.auth.username(), self.auth.password()))
                    .await
                    .map_err(|e| anyhow!(e.to_string()))?
            }
            _ => bail!("Unsupported method: {}", method),
        };

        Ok(response)
    }

    fn parse_response_json(command: &str, text: &str) -> Result<Value> {
        match serde_json::from_str(text) {
            Ok(value) => Ok(value),
            Err(error) if command == "get_miner_status" => {
                status_parser::parse_miner_status_text(text).map_err(|fallback_error| {
                    anyhow!(
                        "failed to parse {command} JSON: {error}; fallback parser failed: {fallback_error}"
                    )
                })
            }
            Err(error) => Err(anyhow!("failed to parse {command} JSON: {error}")),
        }
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

    pub async fn reboot(&self) -> Result<bool> {
        self.send_web_status_command("reboot", None, Method::GET)
            .await
    }

    pub async fn change_password(&self, password: &str) -> Result<bool> {
        let payload = json!({
            "cur_pwd": self.auth.password(),
            "new_pwd": password,
        });
        let response = self
            .send_web_text_command("passwdV1", Some(payload), Method::POST)
            .await?;
        let response = response
            .rsplit_once("\n\n")
            .map(|(_, body)| body)
            .unwrap_or(&response);
        let response = serde_json::from_str::<Value>(response.trim())
            .context("failed to parse VolcMiner password response")?;

        Ok(response.get("code").and_then(Value::as_str) == Some("200"))
    }

    pub async fn get_system_info(&self) -> Result<Value> {
        self.send_web_command("get_system_info", false, None, Method::GET)
            .await
    }

    async fn conf_metadata(&self) -> config_form::MinerConfMetadata {
        let Ok(text) = self
            .send_web_text_command("get_miner_confV1", None, Method::GET)
            .await
        else {
            return config_form::MinerConfMetadata::default();
        };

        config_form::MinerConfMetadata {
            runmode: status_parser::extract_text_field(&text, "runmode")
                .unwrap_or_else(|| "0".to_string()),
            voltage: status_parser::extract_text_field(&text, "voltage")
                .unwrap_or_else(|| "1260".to_string()),
            debug_enabled: status_parser::extract_text_field(&text, "bb_debug_enable")
                .map(|v| v == "true")
                .unwrap_or(false),
        }
    }

    async fn confirm_pools_config(&self, pools: &[PoolConfig]) -> bool {
        for _ in 0..5 {
            if self
                .get_miner_conf()
                .await
                .map(|config| config_form::pools_match_config(&config, pools))
                .unwrap_or(false)
            {
                return true;
            }
            sleep(Duration::from_secs(1)).await;
        }

        false
    }

    pub async fn set_pools_config(&self, pools: &[PoolConfig]) -> Result<bool> {
        let current = self.get_miner_conf().await?;
        let metadata = self.conf_metadata().await;
        let body = config_form::build_miner_conf_body(&current, &metadata, pools);

        if let Err(error) = self
            .send_web_text_command("set_miner_conf", Some(Value::String(body)), Method::POST)
            .await
            && !self.confirm_pools_config(pools).await
        {
            return Err(error);
        }

        Ok(true)
    }
}

#[async_trait]
impl APIClient for VolcMinerWebAPI {
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
impl WebAPIClient for VolcMinerWebAPI {
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

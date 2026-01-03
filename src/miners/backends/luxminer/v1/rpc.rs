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
pub struct LUXMinerRPCAPI {
    ip: IpAddr,
    port: u16,
    session_token: Option<String>,
}

impl LUXMinerRPCAPI {
    pub fn new(ip: IpAddr) -> Self {
        Self {
            ip,
            port: 4028,
            session_token: None,
        }
    }

    fn parse_rpc_result(&self, response: &str) -> anyhow::Result<Value> {
        let status = RPCCommandStatus::from_luxminer(response)?;
        match status.into_result() {
            Ok(_) => Ok(serde_json::from_str(response)?),
            Err(e) => Err(e)?,
        }
    }

    async fn auth(&self) -> anyhow::Result<String> {
        if let Ok(data) = self.session().await
            && let Some(session_id) = data
                .get("SESSION")
                .and_then(|s| s.get(0))
                .and_then(|s| s.get("SessionID"))
                .and_then(|s| s.as_str())
            && !session_id.is_empty()
        {
            return Ok(session_id.to_string());
        }

        let data = self.logon().await?;
        if let Some(session_id) = data
            .get("SESSION")
            .and_then(|s| s.get(0))
            .and_then(|s| s.get("SessionID"))
            .and_then(|s| s.as_str())
        {
            Ok(session_id.to_string())
        } else {
            Err(anyhow::anyhow!(
                "Failed to get session ID from logon response"
            ))
        }
    }

    // Basic commands
    pub async fn summary(&self) -> anyhow::Result<Value> {
        self.send_command("summary", false, None).await
    }

    pub async fn stats(&self) -> anyhow::Result<Value> {
        self.send_command("stats", false, None).await
    }

    pub async fn version(&self) -> anyhow::Result<Value> {
        self.send_command("version", false, None).await
    }

    pub async fn config(&self) -> anyhow::Result<Value> {
        self.send_command("config", false, None).await
    }

    pub async fn pools(&self) -> anyhow::Result<Value> {
        self.send_command("pools", false, None).await
    }

    pub async fn devs(&self) -> anyhow::Result<Value> {
        self.send_command("devs", false, None).await
    }

    pub async fn fans(&self) -> anyhow::Result<Value> {
        self.send_command("fans", false, None).await
    }

    pub async fn temps(&self) -> anyhow::Result<Value> {
        self.send_command("temps", false, None).await
    }

    pub async fn power(&self) -> anyhow::Result<Value> {
        self.send_command("power", false, None).await
    }

    pub async fn coin(&self) -> anyhow::Result<Value> {
        self.send_command("coin", false, None).await
    }

    pub async fn profiles(&self) -> anyhow::Result<Value> {
        self.send_command("profiles", false, None).await
    }

    pub async fn tempctrl(&self) -> anyhow::Result<Value> {
        self.send_command("tempctrl", false, None).await
    }

    pub async fn groups(&self) -> anyhow::Result<Value> {
        self.send_command("groups", false, None).await
    }

    pub async fn limits(&self) -> anyhow::Result<Value> {
        self.send_command("limits", false, None).await
    }

    // Session management
    pub async fn session(&self) -> anyhow::Result<Value> {
        self.send_command("session", false, None).await
    }

    pub async fn logon(&self) -> anyhow::Result<Value> {
        self.send_command("logon", false, None).await
    }

    pub async fn logoff(&mut self) -> anyhow::Result<Value> {
        let result = self.send_command("logoff", true, None).await;
        self.session_token = None;
        result
    }

    // Privileged commands
    pub async fn reboot_device(&self) -> anyhow::Result<Value> {
        self.send_command("rebootdevice", true, None).await
    }

    pub async fn reset_miner(&self) -> anyhow::Result<Value> {
        self.send_command("resetminer", true, None).await
    }

    pub async fn sleep(&self) -> anyhow::Result<Value> {
        self.send_command("curtail", true, Some(Value::String("sleep".to_string())))
            .await
    }

    pub async fn wakeup(&self) -> anyhow::Result<Value> {
        self.send_command("curtail", true, Some(Value::String("wakeup".to_string())))
            .await
    }

    pub async fn ledset(&self, color: &str, state: &str) -> anyhow::Result<Value> {
        self.send_command(
            "ledset",
            true,
            Some(Value::String(format!("{},{}", color, state))),
        )
        .await
    }

    pub async fn profileset(&self, profile: &str) -> anyhow::Result<Value> {
        self.send_command("profileset", true, Some(Value::String(profile.to_string())))
            .await
    }

    pub async fn fanset(&self, speed: Option<i32>, min_fans: Option<i32>) -> anyhow::Result<Value> {
        let mut params = Vec::new();
        if let Some(speed) = speed {
            params.push(format!("speed={}", speed));
        }
        if let Some(min_fans) = min_fans {
            params.push(format!("min_fans={}", min_fans));
        }

        if params.is_empty() {
            return Err(anyhow::anyhow!(
                "At least one parameter required for fanset"
            ));
        }

        self.send_command("fanset", true, Some(Value::String(params.join(","))))
            .await
    }

    // ATM (Advanced Thermal Management) commands
    pub async fn atm(&self) -> anyhow::Result<Value> {
        self.send_command("atm", false, None).await
    }

    pub async fn atmset(
        &self,
        enabled: Option<bool>,
        startup_minutes: Option<i32>,
        post_ramp_minutes: Option<i32>,
        temp_window: Option<i32>,
        min_profile: Option<&str>,
        max_profile: Option<&str>,
    ) -> anyhow::Result<Value> {
        let mut params = Vec::new();

        if let Some(enabled) = enabled {
            params.push(format!("enabled={}", enabled.to_string().to_lowercase()));
        }
        if let Some(startup_minutes) = startup_minutes {
            params.push(format!("startup_minutes={}", startup_minutes));
        }
        if let Some(post_ramp_minutes) = post_ramp_minutes {
            params.push(format!("post_ramp_minutes={}", post_ramp_minutes));
        }
        if let Some(temp_window) = temp_window {
            params.push(format!("temp_window={}", temp_window));
        }
        if let Some(min_profile) = min_profile {
            params.push(format!("min_profile={}", min_profile));
        }
        if let Some(max_profile) = max_profile {
            params.push(format!("max_profile={}", max_profile));
        }

        if params.is_empty() {
            return Err(anyhow::anyhow!(
                "At least one parameter required for atmset"
            ));
        }

        self.send_command("atmset", true, Some(Value::String(params.join(","))))
            .await
    }

    // Pool management
    pub async fn addpool(
        &self,
        url: &str,
        user: &str,
        pass: &str,
        group_id: Option<&str>,
    ) -> anyhow::Result<Value> {
        let mut params = vec![url, user, pass];
        if let Some(group_id) = group_id {
            params.push(group_id);
        }

        self.send_command("addpool", false, Some(Value::String(params.join(","))))
            .await
    }

    pub async fn removepool(&self, pool_id: i32) -> anyhow::Result<Value> {
        self.send_command(
            "removepool",
            false,
            Some(Value::String(pool_id.to_string())),
        )
        .await
    }

    pub async fn switchpool(&self, pool_id: i32) -> anyhow::Result<Value> {
        self.send_command(
            "switchpool",
            false,
            Some(Value::String(pool_id.to_string())),
        )
        .await
    }

    pub async fn enablepool(&self, pool_id: i32) -> anyhow::Result<Value> {
        self.send_command(
            "enablepool",
            false,
            Some(Value::String(pool_id.to_string())),
        )
        .await
    }

    pub async fn disablepool(&self, pool_id: i32) -> anyhow::Result<Value> {
        self.send_command(
            "disablepool",
            false,
            Some(Value::String(pool_id.to_string())),
        )
        .await
    }

    // Multi-command functionality
    pub async fn multicommand(&self, commands: &[&str]) -> anyhow::Result<Value> {
        let mut results = json!({});

        for &command in commands {
            match command {
                "summary" => {
                    if let Ok(result) = self.summary().await {
                        results[command] = result;
                    }
                }
                "stats" => {
                    if let Ok(result) = self.stats().await {
                        results[command] = result;
                    }
                }
                "version" => {
                    if let Ok(result) = self.version().await {
                        results[command] = result;
                    }
                }
                "config" => {
                    if let Ok(result) = self.config().await {
                        results[command] = result;
                    }
                }
                "pools" => {
                    if let Ok(result) = self.pools().await {
                        results[command] = result;
                    }
                }
                "fans" => {
                    if let Ok(result) = self.fans().await {
                        results[command] = result;
                    }
                }
                "temps" => {
                    if let Ok(result) = self.temps().await {
                        results[command] = result;
                    }
                }
                "tempctrl" => {
                    if let Ok(result) = self.tempctrl().await {
                        results[command] = result;
                    }
                }
                "groups" => {
                    if let Ok(result) = self.groups().await {
                        results[command] = result;
                    }
                }
                "profiles" => {
                    if let Ok(result) = self.profiles().await {
                        results[command] = result;
                    }
                }
                _ => {
                    // For unknown commands, try to send directly
                    if let Ok(result) = self.send_command(command, false, None).await {
                        results[command] = result;
                    }
                }
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl APIClient for LUXMinerRPCAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC {
                command,
                parameters,
            } => self
                .send_command(command, false, parameters.clone())
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string())),
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for LuxMiner RPC API"
            )),
        }
    }
}

#[async_trait]
impl RPCAPIClient for LUXMinerRPCAPI {
    async fn send_command(
        &self,
        command: &str,
        privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        let mut stream = tokio::net::TcpStream::connect((self.ip, self.port))
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;

        let mut request = json!({
            "command": command
        });

        // Add session token for privileged commands
        if privileged {
            if let Ok(token) = &self.auth().await {
                if let Some(params) = parameters {
                    request["parameter"] =
                        json!(format!("{},{}", token, params.as_str().unwrap_or_default()));
                } else {
                    request["parameter"] = Value::String(token.clone());
                }
            } else {
                return Err(anyhow::anyhow!(
                    "No session token available for privileged command"
                ));
            }
        } else if let Some(params) = parameters {
            request["parameter"] = params;
        }

        let json_str = request.to_string();
        let message = format!("{}\n", json_str);

        stream.write_all(message.as_bytes()).await?;

        let mut response = String::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = stream.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);
            response.push_str(&chunk);

            if response.contains('\0') || response.ends_with('\n') {
                break;
            }
        }

        let clean_response = response.trim_end_matches('\0').trim_end_matches('\n');
        self.parse_rpc_result(clean_response)
    }
}

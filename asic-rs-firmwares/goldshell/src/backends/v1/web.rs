// SPDX-License-Identifier: Apache-2.0
// Adapted from pyasic web/goldshell.py (Copyright 2022 Upstream Data Inc); modified in Rust.
use asic_rs_core::traits::miner::{ExposeSecret, MinerAuth, SecretString};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::{net::IpAddr, time::Duration};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct GoldshellWebAPI {
    ip: IpAddr,
    auth: MinerAuth,
    client: Result<Client, reqwest::Error>,
    token: Mutex<Option<SecretString>>,
}

impl GoldshellWebAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            ip,
            auth,
            client: Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .build(),
            token: Mutex::new(None),
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
        self.token = Mutex::new(None);
    }

    pub fn allows_read(command: &str) -> bool {
        matches!(command, "status" | "setting")
    }

    fn client(&self) -> anyhow::Result<&Client> {
        self.client
            .as_ref()
            .map_err(|_| anyhow::anyhow!("Cannot initialize Goldshell read client"))
    }

    async fn login(&self) -> anyhow::Result<SecretString> {
        // Do not invalidate another user's session via /user/logout.
        let response = self
            .client()?
            .get(format!("http://{}/user/login", self.ip))
            .query(&[
                ("username", self.auth.username()),
                ("password", self.auth.password()),
                ("cipher", "false"),
            ])
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Goldshell read-session login failed"))?
            .error_for_status()
            .map_err(|_| anyhow::anyhow!("Goldshell read-session login denied"))?;
        let value: Value = response
            .json()
            .await
            .map_err(|_| anyhow::anyhow!("Invalid Goldshell read-session response"))?;
        let token = value
            .get("JWT Token")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Goldshell read-session token missing"))?;
        Ok(SecretString::from(token.to_string()))
    }

    pub async fn read(&self, command: &str) -> anyhow::Result<Value> {
        anyhow::ensure!(
            Self::allows_read(command),
            "Unsupported Goldshell read command"
        );
        let mut token = self.token.lock().await;
        if let MinerAuth::TokenAuth(provided) = &self.auth {
            *token = Some(provided.clone());
        }
        for attempt in 0..2 {
            let mut request = self
                .client()?
                .get(format!("http://{}/mcb/{command}", self.ip));
            if let Some(token) = token.as_ref() {
                request = request.bearer_auth(token.expose_secret());
            }
            let response = request
                .send()
                .await
                .map_err(|_| anyhow::anyhow!("Goldshell read request failed"))?;
            if matches!(
                response.status(),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
            ) && attempt == 0
                && !matches!(self.auth, MinerAuth::TokenAuth(_))
            {
                *token = Some(self.login().await?);
                continue;
            }
            return response
                .error_for_status()
                .map_err(|_| anyhow::anyhow!("Goldshell read request denied"))?
                .json()
                .await
                .map_err(|_| anyhow::anyhow!("Invalid Goldshell read response"));
        }
        anyhow::bail!("Goldshell read authentication failed")
    }
}

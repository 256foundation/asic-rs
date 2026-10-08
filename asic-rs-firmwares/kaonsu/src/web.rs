// SPDX-License-Identifier: Apache-2.0
use asic_rs_core::traits::miner::MinerAuth;
use diqwest::WithDigestAuth;
use once_cell::sync::OnceCell;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use tokio::sync::Mutex;

const API_PREFIX: &str = "/kaonsu/v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Transport {
    HttpDigest,
    HttpBasic,
    HttpsDigest,
    HttpsBasic,
}
impl Transport {
    fn scheme(self) -> &'static str {
        match self {
            Self::HttpDigest | Self::HttpBasic => "http",
            _ => "https",
        }
    }
    fn digest(self) -> bool {
        matches!(self, Self::HttpDigest | Self::HttpsDigest)
    }
}

#[derive(Debug)]
pub(crate) struct KaonsuWebAPI {
    ip: IpAddr,
    auth: MinerAuth,
    client: OnceCell<Client>,
    preferred: Mutex<Option<Transport>>,
}

impl KaonsuWebAPI {
    pub(crate) fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            ip,
            auth,
            client: OnceCell::new(),
            preferred: Mutex::new(None),
        }
    }
    pub(crate) fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    pub(crate) async fn get(&self, command: &str) -> anyhow::Result<Value> {
        if !matches!(command, "brief" | "overview" | "hashboards" | "fans") {
            anyhow::bail!("KaonSu telemetry supports only brief, overview, hashboards and fans");
        }
        // Local firmware may use self-signed HTTPS certificates.
        let client = self.client.get_or_try_init(|| {
            Client::builder()
                .timeout(Duration::from_secs(5))
                .no_proxy()
                .danger_accept_invalid_certs(true)
                .redirect(reqwest::redirect::Policy::none())
                .build()
        })?;
        let preferred = *self.preferred.lock().await;
        let mut transports = vec![
            Transport::HttpDigest,
            Transport::HttpBasic,
            Transport::HttpsDigest,
            Transport::HttpsBasic,
        ];
        if let Some(preferred) = preferred {
            transports.retain(|transport| *transport != preferred);
            transports.insert(0, preferred);
        }
        let mut unreachable = Vec::new();
        for transport in transports {
            if unreachable.contains(&transport.scheme()) {
                continue;
            }
            let port = if transport.scheme() == "https" {
                443
            } else {
                80
            };
            let url = format!(
                "{}://{}{API_PREFIX}/{command}",
                transport.scheme(),
                SocketAddr::new(self.ip, port)
            );
            let request = client
                .get(url)
                .header(reqwest::header::ACCEPT, "application/json");
            let response = if transport.digest() {
                request
                    .send_digest_auth((self.auth.username(), self.auth.password()))
                    .await
            } else {
                request
                    .basic_auth(self.auth.username(), Some(self.auth.password()))
                    .send()
                    .await
                    .map_err(diqwest::error::Error::from)
            };
            let response = match response {
                Ok(response) => response,
                Err(diqwest::error::Error::Reqwest(_)) => {
                    unreachable.push(transport.scheme());
                    continue;
                }
                Err(_) => continue, // Some reachable releases omit digest challenges.
            };
            if matches!(
                response.status(),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
            ) {
                continue;
            }
            let response = response.error_for_status()?;
            let value: Value = response.json().await?;
            if !value.is_object() {
                anyhow::bail!("KaonSu {command} returned non-object JSON");
            }
            *self.preferred.lock().await = Some(transport);
            return Ok(value);
        }
        anyhow::bail!("KaonSu {command} read request failed")
    }
}

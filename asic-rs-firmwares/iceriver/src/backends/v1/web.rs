use std::{net::IpAddr, time::Duration};

use anyhow::{Result, bail};
use asic_rs_core::{
    data::command::MinerCommand,
    traits::miner::{APIClient, MinerAuth},
};
use async_trait::async_trait;
use reqwest::{
    Client,
    header::{COOKIE, SET_COOKIE},
    redirect::Policy,
};
use serde_json::Value;

/// IceRiver's authenticated read-only userpanel operation.
///
/// The vendor uses POST with query parameters for both session login and reads.
/// No arbitrary endpoint, userpanel post code, or parameter is accepted here.
#[derive(Debug)]
pub struct IceRiverWebAPI {
    ip: IpAddr,
    port: u16,
    auth: MinerAuth,
}

impl IceRiverWebAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self { ip, port: 80, auth }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    pub async fn userpanel(&self) -> Result<Value> {
        if self.auth.token().is_some() {
            bail!("IceRiver userpanel requires username/password authentication");
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(Policy::none())
            .build()?;
        let base = format!("http://{}:{}", self.ip, self.port);
        let login = client
            .post(format!("{base}/user/loginpost"))
            .query(&[
                ("post", "6"),
                ("user", self.auth.username()),
                ("pwd", self.auth.password()),
            ])
            .send()
            .await
            // Reqwest errors may include the credential-bearing query URL.
            .map_err(|_| anyhow::anyhow!("IceRiver login request failed"))?;
        if !login.status().is_success() {
            bail!("IceRiver login returned HTTP {}", login.status());
        }
        let cookies = login
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .filter_map(|header| header.to_str().ok())
            .filter_map(|cookie| cookie.split(';').next())
            .collect::<Vec<_>>()
            .join("; ");
        let mut request = client
            .post(format!("{base}/user/userpanel"))
            .query(&[("post", "4")]);
        if !cookies.is_empty() {
            request = request.header(COOKIE, cookies);
        }
        let response = request
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("IceRiver userpanel request failed"))?;
        if !response.status().is_success() {
            bail!("IceRiver userpanel returned HTTP {}", response.status());
        }
        let response: Value = response
            .json()
            .await
            .map_err(|_| anyhow::anyhow!("IceRiver userpanel returned invalid JSON"))?;
        if super::panel_data(&response, None).is_none() {
            bail!("IceRiver userpanel has no telemetry data object");
        }
        Ok(response)
    }
}

#[async_trait]
impl APIClient for IceRiverWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::WebAPI {
                command: "userpanel",
                parameters: None,
            } => self.userpanel().await,
            _ => bail!("Only the read-only IceRiver userpanel command is supported"),
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;

    #[tokio::test]
    async fn read_uses_vendor_post_codes_and_preserves_login_session_cookie() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            for (expected_path, body, cookie) in [
                (
                    "/user/loginpost?post=6&user=&pwd=",
                    "{}",
                    "Set-Cookie: session=fixture; Path=/\r\n",
                ),
                (
                    "/user/userpanel?post=4",
                    "{\"data\":{\"powstate\":true}}",
                    "",
                ),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                loop {
                    let mut chunk = [0u8; 1024];
                    let size = socket.read(&mut chunk).await.unwrap();
                    assert!(size > 0);
                    request.extend_from_slice(&chunk[..size]);
                    if request.windows(4).any(|chunk| chunk == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(request).unwrap();
                assert!(request.starts_with(&format!("POST {expected_path} HTTP/1.1\r\n")));
                if expected_path.contains("userpanel") {
                    assert!(
                        request
                            .to_ascii_lowercase()
                            .contains("cookie: session=fixture\r\n")
                    );
                }
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{cookie}Connection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let mut web = IceRiverWebAPI::new("127.0.0.1".parse().unwrap(), MinerAuth::new("", ""));
        web.port = port;
        let result = web.userpanel().await.unwrap();
        assert_eq!(result["data"]["powstate"], true);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn rejects_control_commands_and_parameters_before_network_io() {
        let web = IceRiverWebAPI::new("127.0.0.1".parse().unwrap(), MinerAuth::new("", ""));
        for command in [
            MinerCommand::WebAPI {
                command: "userpanel",
                parameters: Some(serde_json::json!({"post": "5"})),
            },
            MinerCommand::WebAPI {
                command: "reboot",
                parameters: None,
            },
            MinerCommand::RPC {
                command: "stats",
                parameters: None,
            },
        ] {
            assert!(
                web.get_api_result(&command)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("read-only")
            );
        }
    }
}

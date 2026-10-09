use std::{net::IpAddr, time::Duration};

use once_cell::sync::OnceCell;

use asic_rs_core::{data::command::MinerCommand, traits::miner::*};
use async_trait::async_trait;
use reqwest::{Client, Method};
use serde_json::Value;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct BraiinsWebAPI {
    client: OnceCell<Client>,
    ip: IpAddr,
    port: u16,
    timeout: Duration,
    session_id: RwLock<Option<String>>,
    auth: MinerAuth,
}

impl BraiinsWebAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            client: OnceCell::new(),
            ip,
            port: 80,
            timeout: Duration::from_secs(5),
            session_id: RwLock::new(None),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
        *self.session_id.get_mut() = None;
    }

    fn build_client() -> anyhow::Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| anyhow::anyhow!("failed to create HTTP client: {e}"))
    }

    fn client(&self) -> anyhow::Result<&Client> {
        self.client.get_or_try_init(Self::build_client)
    }

    async fn authenticate(&self) -> anyhow::Result<String> {
        let url = format!("http://{}:{}/cgi-bin/luci", self.ip, self.port);
        let client = self.client()?;
        let response = client
            .post(&url)
            .header("User-Agent", "BTC Tools v0.1")
            .form(&[
                ("luci_username", self.auth.username()),
                ("luci_password", self.auth.password()),
            ])
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Luci auth failed: {}", e))?;

        for cookie in response.headers().get_all("set-cookie") {
            if let Ok(cookie_str) = cookie.to_str() {
                for part in cookie_str.split(';') {
                    let part = part.trim();
                    if let Some(value) = part.strip_prefix("session_id=")
                        && !value.is_empty()
                    {
                        return Ok(value.to_string());
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Failed to obtain Luci session cookie"))
    }

    async fn ensure_authenticated(&self) -> anyhow::Result<()> {
        if self.session_id.read().await.is_some() {
            return Ok(());
        }

        let session = self.authenticate().await?;
        *self.session_id.write().await = Some(session);
        Ok(())
    }

    pub async fn send_luci_command(&self, command: &str) -> anyhow::Result<Value> {
        self.ensure_authenticated().await?;

        let url = format!("http://{}:{}/cgi-bin/luci/{}", self.ip, self.port, command);
        let client = self.client()?;

        let mut request = client
            .get(&url)
            .header("User-Agent", "BTC Tools v0.1")
            .timeout(self.timeout);

        if let Some(ref session) = *self.session_id.read().await {
            request = request.header("Cookie", format!("session_id={}", session));
        }

        let response = request
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Luci command failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Luci HTTP error: {}", response.status()));
        }

        response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Luci parse error: {}", e))
    }
}

#[async_trait]
impl WebAPIClient for BraiinsWebAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        _parameters: Option<Value>,
        _method: Method,
    ) -> anyhow::Result<Value> {
        self.send_luci_command(command).await
    }
}

#[async_trait]
impl APIClient for BraiinsWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::WebAPI { command, .. } => self.send_luci_command(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for Luci web client"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use serde_json::json;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    use super::*;

    async fn read_http_request(socket: &mut TcpStream) -> anyhow::Result<String> {
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];

        loop {
            if let Some(header_end) = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|position| position + 4)
            {
                let headers = std::str::from_utf8(&request[..header_end])?;
                let content_length = headers
                    .lines()
                    .filter_map(|line| line.split_once(':'))
                    .find_map(|(name, value)| {
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or_default();
                if request.len() >= header_end + content_length {
                    break;
                }
            }

            let bytes_read = socket.read(&mut chunk).await?;
            if bytes_read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..bytes_read]);
        }

        Ok(String::from_utf8(request)?)
    }

    #[tokio::test]
    async fn custom_credentials_authenticate_luci_telemetry() -> anyhow::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (headers, body) in [
                ("Set-Cookie: session_id=custom-session; Path=/\r\n", ""),
                ("", r#"[{"macaddr":"00:11:22:33:44:55"}]"#),
            ] {
                let (mut socket, _) =
                    tokio::time::timeout(Duration::from_secs(2), listener.accept())
                        .await
                        .context("timed out waiting for Luci request")??;
                requests.push(read_http_request(&mut socket).await?);
                let response = format!(
                    "HTTP/1.1 200 OK\r\n{headers}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await?;
            }
            Ok::<_, anyhow::Error>(requests)
        });

        let mut api = BraiinsWebAPI::new(IpAddr::from([127, 0, 0, 1]), MinerAuth::new("root", ""));
        api.port = port;
        api.set_auth(MinerAuth::new("root+operator", "custom&password=+ é"));
        let network = api
            .send_luci_command("admin/network/iface_status/lan")
            .await?;
        let requests = task.await??;

        assert!(requests[0].starts_with("POST /cgi-bin/luci HTTP/1.1\r\n"));
        assert!(requests[0].lines().any(|line| {
            line.eq_ignore_ascii_case("content-type: application/x-www-form-urlencoded")
        }));
        assert_eq!(
            requests[0]
                .split_once("\r\n\r\n")
                .context("missing login body")?
                .1,
            "luci_username=root%2Boperator&luci_password=custom%26password%3D%2B+%C3%A9"
        );
        assert!(
            requests[1]
                .starts_with("GET /cgi-bin/luci/admin/network/iface_status/lan HTTP/1.1\r\n")
        );
        assert!(
            requests[1]
                .lines()
                .any(|line| line.eq_ignore_ascii_case("cookie: session_id=custom-session"))
        );
        assert_eq!(network, json!([{ "macaddr": "00:11:22:33:44:55" }]));
        Ok(())
    }
}

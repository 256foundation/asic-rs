use std::{net::IpAddr, time::Duration};

use once_cell::sync::OnceCell;

use asic_rs_core::{
    data::{command::MinerCommand, firmware::FirmwareImage},
    traits::miner::*,
};
use async_trait::async_trait;
use reqwest::{Client, multipart};
use serde_json::{Value, json};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct BraiinsGraphQLAPI {
    client: OnceCell<Client>,
    ip: IpAddr,
    port: u16,
    timeout: Duration,
    session_id: RwLock<Option<String>>,
    auth: MinerAuth,
}

impl BraiinsGraphQLAPI {
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

    #[cfg(test)]
    fn with_session(ip: IpAddr, port: u16, session: &str) -> Self {
        Self {
            port,
            session_id: RwLock::new(Some(session.to_string())),
            ..Self::new(ip, MinerAuth::new("root", "root"))
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
        *self.session_id.get_mut() = None;
    }

    pub fn username(&self) -> &str {
        self.auth.username()
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
        let url = format!("http://{}:{}/graphql", self.ip, self.port);
        let client = self.client()?;
        let body = json!({
            "query": r#"mutation (
                $username: String!,
                $password: String!
            ) {
                auth {
                    login(
                        username: $username,
                        password: $password
                    ) {
                        ... on VoidResult {
                            void
                        }
                    }
                }
            }"#,
            "variables": {
                "username": self.auth.username(),
                "password": self.auth.password(),
            }
        });

        let response = client
            .post(&url)
            .json(&body)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Auth request failed: {}", e))?;

        // Extract session_id from Set-Cookie header
        for cookie in response.headers().get_all("set-cookie") {
            if let Ok(cookie_str) = cookie.to_str() {
                // Look for session_id or similar cookie
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

        Err(anyhow::anyhow!("Failed to obtain session cookie"))
    }

    async fn ensure_authenticated(&self) -> anyhow::Result<()> {
        if self.session_id.read().await.is_some() {
            return Ok(());
        }

        let session = self.authenticate().await?;
        *self.session_id.write().await = Some(session);
        Ok(())
    }

    pub async fn send_graphql_command(
        &self,
        command: &str,
        privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        if privileged {
            self.ensure_authenticated().await?;
        }

        let url = format!("http://{}:{}/graphql", self.ip, self.port);
        let client = self.client()?;
        let mut body = json!({ "query": command });
        if let Some(vars) = parameters {
            body["variables"] = vars;
        }

        let mut request = client.post(&url).json(&body).timeout(self.timeout);

        if let Some(ref session) = *self.session_id.read().await {
            request = request.header("Cookie", format!("session_id={}", session));
        }

        let response = request
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("GraphQL request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GraphQL HTTP error: {}", response.status()));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("GraphQL parse error: {}", e))?;

        // Check for errors first — GraphQL can return both "data": null and "errors" together
        if let Some(errors) = json.get("errors")
            && let Some(arr) = errors.as_array()
            && !arr.is_empty()
        {
            return Err(anyhow::anyhow!("GraphQL errors: {}", errors));
        }

        if let Some(data) = json.get("data")
            && !data.is_null()
        {
            return Ok(data.clone());
        }

        Err(anyhow::anyhow!("GraphQL returned null data"))
    }

    pub async fn set_password(&self, password: &str) -> anyhow::Result<bool> {
        let mutation = r#"mutation ($password: String) {
            bos {
                setPassword(newPassword: $password) {
                    ... on VoidResult {
                        void
                    }
                    ... on BosError {
                        message
                    }
                }
            }
        }"#;

        let variables = json!({ "password": password });
        let result = self
            .send_graphql_command(mutation, true, Some(variables))
            .await?;

        Ok(result.pointer("/bos/setPassword/message").is_none()
            && result.pointer("/bos/setPassword").is_some())
    }

    pub async fn factory_reset(&self) -> anyhow::Result<bool> {
        let mutation = r#"mutation {
            bos {
                factoryReset {
                    ... on VoidResult {
                        void
                    }
                    ... on BosError {
                        message
                    }
                }
            }
        }"#;

        let result = self.send_graphql_command(mutation, true, None).await?;

        Ok(result.pointer("/bos/factoryReset/message").is_none()
            && result.pointer("/bos/factoryReset").is_some())
    }

    pub async fn read_logs(&self) -> anyhow::Result<String> {
        let log_query = r#"query ($target: LogTarget!) {
            bos {
                log(target: $target)
            }
        }"#;

        let mut logs = String::new();
        let variables = json!({ "target": "BOSMINER" });
        let lines = self
            .send_graphql_command(log_query, true, Some(variables))
            .await?
            .pointer("/bos/log")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        for line in lines.iter().filter_map(Value::as_str) {
            logs.push_str(line);
            logs.push('\n');
        }

        Ok(logs)
    }

    /// Enable or disable hashboards by their zero-based position in BOSminer.
    pub async fn set_hashboards_enabled(
        &self,
        board_indices: &[u8],
        enabled: bool,
    ) -> anyhow::Result<bool> {
        if board_indices.is_empty() {
            return Ok(true);
        }

        let solvers = self
            .send_graphql_command(
                r#"query {
                    bosminer {
                        info {
                            workSolver {
                                childSolvers { name }
                            }
                        }
                    }
                }"#,
                true,
                None,
            )
            .await?
            .pointer("/bosminer/info/workSolver/childSolvers")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Braiins API did not return hashboard identifiers"))?;

        let mut hash_chains = Vec::with_capacity(board_indices.len());
        for index in board_indices {
            let name = solvers
                .get(*index as usize)
                .and_then(|solver| solver.get("name"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    anyhow::anyhow!("Hashboard index {index} is not available on this miner")
                })?;
            hash_chains.push(json!({ "name": name, "enabled": enabled }));
        }

        let mutation = r#"mutation ($input: PerformanceIn!) {
            bosminer {
                config {
                    updatePerformance(input: $input, apply: true) {
                        ... on PerformanceOut { hashChains { name enabled } }
                        ... on PerformanceError { message }
                        ... on AttributeError { message }
                    }
                }
            }
        }"#;
        let result = self
            .send_graphql_command(
                mutation,
                true,
                Some(json!({ "input": { "hashChains": hash_chains } })),
            )
            .await?;
        let update = result
            .pointer("/bosminer/config/updatePerformance")
            .ok_or_else(|| anyhow::anyhow!("Invalid Braiins hashboard update response"))?;

        if let Some(message) = update.get("message").and_then(Value::as_str) {
            anyhow::bail!("Braiins rejected hashboard state change: {message}");
        }
        if update.get("hashChains").is_none() {
            anyhow::bail!("Invalid Braiins hashboard update response: missing hashChains");
        }

        Ok(true)
    }

    /// Upload a firmware image the way the BOS web UI does: `POST
    /// /system/upgrade/upload` with the image in the multipart field `image`,
    /// authenticated by the GraphQL session cookie. Returns the SHA-256 the
    /// miner computed for the stored image, which [`Self::start_upgrade`] takes.
    pub async fn upload_firmware(&self, image: FirmwareImage) -> anyhow::Result<String> {
        // Always log in afresh: a cached session may have expired, and a
        // multipart body cannot be replayed after a 401. Upgrades are rare.
        let session = self.authenticate().await?;
        *self.session_id.write().await = Some(session.clone());

        let url = format!("http://{}:{}/system/upgrade/upload", self.ip, self.port);
        let FirmwareImage { filename, bytes } = image;
        let part = multipart::Part::bytes(bytes)
            .file_name(filename)
            .mime_str("application/octet-stream")
            .map_err(|e| anyhow::anyhow!("failed to set firmware part mime type: {e}"))?;
        let form = multipart::Form::new().part("image", part);

        // Firmware images are ~100 MB; the client default (10 s) is far too short.
        let response = self
            .client()?
            .post(&url)
            .multipart(form)
            .header("Cookie", format!("session_id={session}"))
            .timeout(Duration::from_secs(600))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("firmware upload request failed: {e}"))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("firmware upload response unreadable: {e}"))?;
        if !status.is_success() {
            anyhow::bail!("firmware upload HTTP error: {status}: {text}");
        }
        parse_upload_response(&text)
    }

    /// Install a previously uploaded image (`bos.upgrade(hash)`). On success the
    /// miner reboots into the new firmware and is unreachable for a while.
    pub async fn start_upgrade(&self, hash: &str) -> anyhow::Result<()> {
        let mutation = r#"mutation ($hash: String!) {
            bos {
                upgrade(hash: $hash) {
                    __typename
                    ... on BosError { message }
                }
            }
        }"#;
        let data = self
            .send_graphql_command(mutation, true, Some(json!({ "hash": hash })))
            .await?;
        match data
            .pointer("/bos/upgrade/__typename")
            .and_then(Value::as_str)
        {
            Some("VoidResult") => Ok(()),
            Some("BosError") => anyhow::bail!(
                "Braiins rejected the firmware upgrade: {}",
                data.pointer("/bos/upgrade/message")
                    .and_then(Value::as_str)
                    .unwrap_or("no message")
            ),
            _ => anyhow::bail!("Invalid Braiins upgrade response: {data}"),
        }
    }
}

/// `{"sha256": "<hex>"}` on success, `{"error": "<reason>"}` when BOS rejects the image.
fn parse_upload_response(text: &str) -> anyhow::Result<String> {
    let body: Value = serde_json::from_str(text)
        .map_err(|e| anyhow::anyhow!("invalid firmware upload response ({e}): {text}"))?;
    if let Some(error) = body.get("error") {
        let reason = error
            .as_str()
            .map_or_else(|| error.to_string(), String::from);
        anyhow::bail!("Braiins rejected the firmware image: {reason}");
    }
    body.get("sha256")
        .and_then(Value::as_str)
        .filter(|hash| !hash.is_empty())
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("firmware upload response has no sha256: {text}"))
}

#[async_trait]
impl GraphQLClient for BraiinsGraphQLAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.send_graphql_command(command, _privileged, parameters)
            .await
    }
}

#[async_trait]
impl APIClient for BraiinsGraphQLAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::GraphQL { command } => {
                self.send_graphql_command(command, true, None).await
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for GraphQL client"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;

    /// Serve one HTTP request per entry of `responses` (extra headers, body),
    /// in order, and return the raw requests received.
    async fn serve(
        responses: Vec<(&'static str, &'static str)>,
    ) -> anyhow::Result<(u16, tokio::task::JoinHandle<anyhow::Result<Vec<String>>>)> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (headers, body) in responses {
                let (mut socket, _) =
                    tokio::time::timeout(Duration::from_secs(2), listener.accept())
                        .await
                        .context("timed out waiting for HTTP request")??;
                let mut request = Vec::new();
                let mut chunk = [0_u8; 4096];
                loop {
                    if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&request[..end]).to_lowercase();
                        let length = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:"))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .context("request without content-length")?;
                        if request.len() >= end + 4 + length {
                            break;
                        }
                    }
                    let read = socket.read(&mut chunk).await?;
                    anyhow::ensure!(read > 0, "connection closed mid-request");
                    request.extend_from_slice(&chunk[..read]);
                }
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await?;
                requests.push(String::from_utf8_lossy(&request).into_owned());
            }
            Ok(requests)
        });
        Ok((port, task))
    }

    async fn within<T>(fut: impl std::future::Future<Output = T>) -> anyhow::Result<T> {
        tokio::time::timeout(Duration::from_secs(10), fut)
            .await
            .context("test timed out")
    }

    fn api(port: u16) -> BraiinsGraphQLAPI {
        BraiinsGraphQLAPI::with_session(IpAddr::from([127, 0, 0, 1]), port, "sess-1")
    }

    #[tokio::test]
    async fn upload_firmware_logs_in_afresh_and_returns_hash() -> anyhow::Result<()> {
        let (port, task) = serve(vec![
            (
                "Set-Cookie: session_id=fresh-1; Path=/; HttpOnly\r\n",
                r#"{"data":{"auth":{"login":{"void":null}}}}"#,
            ),
            ("", r#"{"sha256":"abc123"}"#),
        ])
        .await?;
        let image = FirmwareImage::new("fw.tar".to_string(), b"FIRMWARE".to_vec());

        // The cached session "sess-1" must not be used for the upload.
        let api = api(port);
        assert_eq!(within(api.upload_firmware(image)).await??, "abc123");

        let requests = within(task).await???;
        assert!(requests[0].starts_with("POST /graphql HTTP/1.1\r\n"));
        assert!(requests[0].contains("login("));
        let upload = &requests[1];
        assert!(upload.starts_with("POST /system/upgrade/upload HTTP/1.1\r\n"));
        assert!(upload.contains("cookie: session_id=fresh-1"));
        assert!(upload.contains("name=\"image\"; filename=\"fw.tar\""));
        assert!(upload.contains("FIRMWARE"));
        Ok(())
    }

    #[tokio::test]
    async fn start_upgrade_sends_hash_and_accepts_void_result() -> anyhow::Result<()> {
        let (port, task) =
            serve(vec![("", r#"{"data":{"bos":{"upgrade":{"__typename":"VoidResult"}}}}"#)])
                .await?;

        let api = api(port);
        within(api.start_upgrade("abc123")).await??;

        let request = within(task).await???.remove(0);
        assert!(request.starts_with("POST /graphql HTTP/1.1\r\n"));
        assert!(request.contains("upgrade(hash: $hash)"));
        assert!(request.contains(r#""hash":"abc123""#));
        Ok(())
    }

    #[tokio::test]
    async fn start_upgrade_surfaces_bos_error() -> anyhow::Result<()> {
        let (port, task) = serve(vec![(
            "",
            r#"{"data":{"bos":{"upgrade":{"__typename":"BosError","message":"image not found"}}}}"#,
        )])
        .await?;

        let api = api(port);
        let err = within(api.start_upgrade("abc123")).await?.unwrap_err();
        assert!(err.to_string().contains("image not found"));
        within(task).await???;
        Ok(())
    }

    #[test]
    fn parse_upload_response_variants() {
        assert_eq!(
            parse_upload_response(r#"{"sha256":"ff00"}"#).unwrap(),
            "ff00"
        );
        let err = parse_upload_response(r#"{"error":"invalid image"}"#).unwrap_err();
        assert!(err.to_string().contains("invalid image"));
        assert!(parse_upload_response(r#"{"sha256":""}"#).is_err());
        assert!(parse_upload_response("not json").is_err());
    }
}

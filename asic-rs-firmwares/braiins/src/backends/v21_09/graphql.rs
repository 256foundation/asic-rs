use std::{net::IpAddr, time::Duration};

use once_cell::sync::OnceCell;

use asic_rs_core::{data::command::MinerCommand, traits::miner::*};
use async_trait::async_trait;
use reqwest::Client;
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
    pub(crate) fn new_with_port(ip: IpAddr, port: u16, auth: MinerAuth) -> Self {
        let mut api = Self::new(ip, auth);
        api.port = port;
        api
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

    /// Restart BOSminer without rebooting the operating system.
    pub async fn restart_bosminer(&self) -> anyhow::Result<bool> {
        let mutation = r#"mutation {
            bosminer {
                restart {
                    ... on BosminerError {
                        message
                    }
                }
            }
        }"#;

        let result = self.send_graphql_command(mutation, true, None).await?;

        // There is only a message field if there is an error
        if let Some(message) = result.pointer("/bosminer/restart/message") {
            let message = message
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| message.to_string());
            anyhow::bail!("BOSminer restart request failed: {message}");
        }

        Ok(true)
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
    use asic_rs_makes_antminer::models::AntMinerModel;

    use super::*;
    use crate::{
        backends::{v21_09::BraiinsV2109, v25_03::BraiinsV2503, v25_05::BraiinsV2505},
        test::http::{MockResponse, MockSequenceServer, mock_sequence_server, request_body},
    };

    const LOGIN_OK: MockResponse = MockResponse {
        status: "200 OK",
        headers: &["Set-Cookie: session_id=test-session; Path=/"],
        body: r#"{"data":{"auth":{"login":{"void":null}}}}"#,
    };

    fn localhost() -> IpAddr {
        IpAddr::from([127, 0, 0, 1])
    }

    fn mock_graphql(port: u16) -> BraiinsGraphQLAPI {
        BraiinsGraphQLAPI::new_with_port(localhost(), port, MinerAuth::new("root", "root"))
    }

    #[tokio::test]
    async fn restart_mining_sends_authenticated_bosminer_restart() -> anyhow::Result<()> {
        let MockSequenceServer { port, task } = mock_sequence_server(vec![
            LOGIN_OK,
            MockResponse::ok(r#"{"data":{"bosminer":{"restart":{}}}}"#),
        ])
        .await?;
        let mut miner = BraiinsV2109::new(localhost(), AntMinerModel::S9);
        miner.graphql = mock_graphql(port);

        assert!(miner.supports_restart_mining());
        assert!(miner.restart_mining().await?);

        let requests = task.await??;
        assert!(requests[1].starts_with("POST /graphql HTTP/1.1\r\n"));
        assert!(
            requests[1]
                .lines()
                .any(|line| line.eq_ignore_ascii_case("cookie: session_id=test-session"))
        );
        let query = request_body(&requests[1])?["query"]
            .as_str()
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(query.starts_with("mutation { bosminer { restart {"));
        assert!(!query.contains("reboot"));

        Ok(())
    }

    #[tokio::test]
    async fn restart_mining_surfaces_bosminer_error_message() -> anyhow::Result<()> {
        let MockSequenceServer { port, task } = mock_sequence_server(vec![
            LOGIN_OK,
            MockResponse::ok(
                r#"{"data":{"bosminer":{"restart":{"message":"bosminer is not running"}}}}"#,
            ),
        ])
        .await?;

        let error = mock_graphql(port).restart_bosminer().await.unwrap_err();

        task.await??;
        assert!(error.to_string().contains("bosminer is not running"));

        Ok(())
    }

    #[tokio::test]
    async fn restart_mining_surfaces_graphql_errors() -> anyhow::Result<()> {
        let MockSequenceServer { port, task } = mock_sequence_server(vec![
            LOGIN_OK,
            MockResponse::ok(r#"{"data":null,"errors":[{"message":"Unauthorized"}]}"#),
        ])
        .await?;

        let error = mock_graphql(port).restart_bosminer().await.unwrap_err();

        task.await??;
        assert!(error.to_string().contains("Unauthorized"));

        Ok(())
    }

    #[test]
    fn graphql_backends_support_restart_mining() {
        assert!(BraiinsV2109::new(localhost(), AntMinerModel::S9).supports_restart_mining());
        assert!(BraiinsV2503::new(localhost(), AntMinerModel::S19XP).supports_restart_mining());
        assert!(BraiinsV2505::new(localhost(), AntMinerModel::S19XP).supports_restart_mining());
    }
}

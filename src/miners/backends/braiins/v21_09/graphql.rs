use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::net::IpAddr;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

#[derive(Debug)]
pub struct BraiinsGraphQLAPI {
    client: Client,
    ip: IpAddr,
    port: u16,
    timeout: Duration,
    session_id: RwLock<Option<String>>,
    username: String,
    password: String,
}

impl BraiinsGraphQLAPI {
    pub fn new(ip: IpAddr) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            ip,
            port: 80,
            timeout: Duration::from_secs(5),
            session_id: RwLock::new(None),
            username: "root".to_string(),
            password: String::new(),
        }
    }

    async fn authenticate(&self) -> anyhow::Result<String> {
        let url = format!("http://{}:{}/graphql", self.ip, self.port);
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
                "username": self.username,
                "password": self.password,
            }
        });

        let response = self
            .client
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
        let mut body = json!({ "query": command });
        if let Some(vars) = parameters {
            body["variables"] = vars;
        }

        let mut request = self.client.post(&url).json(&body).timeout(self.timeout);

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

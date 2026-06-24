use std::{net::IpAddr, time::Duration};

use anyhow::Context;
use asic_rs_core::{data::command::MinerCommand, traits::miner::*};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use reqwest::Client;
use serde_json::{Value, json};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ApolloGraphQLAPI {
    client: OnceCell<Client>,
    ip: IpAddr,
    port: u16,
    timeout: Duration,
    token: RwLock<Option<String>>,
    auth: MinerAuth,
}

impl ApolloGraphQLAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            client: OnceCell::new(),
            ip,
            port: 5000,
            timeout: Duration::from_secs(5),
            token: RwLock::new(None),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
        *self.token.get_mut() = None;
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
        let url = format!("http://{}:{}/api/graphql", self.ip, self.port);
        let client = self.client()?;
        let body = json!({
            "query": r#"query Login($input: AuthLoginInput!) {
                Auth {
                    login(input: $input) {
                        result {
                            accessToken
                        }
                        error {
                            message
                        }
                    }
                }
            }"#,
            "variables": {
                "input": {
                    "password": self.auth.password(),
                }
            }
        });

        let response = client
            .post(&url)
            .json(&body)
            .timeout(self.timeout)
            .send()
            .await
            .context("Apollo GraphQL auth request failed")?;

        if !response.status().is_success() {
            anyhow::bail!("Apollo GraphQL auth HTTP error: {}", response.status());
        }

        let data = response
            .json::<Value>()
            .await
            .context("Apollo GraphQL auth parse error")?;

        if let Some(errors) = data.get("errors") {
            anyhow::bail!("Apollo GraphQL auth errors: {errors}");
        }

        if let Some(message) = data
            .pointer("/data/Auth/login/error/message")
            .and_then(Value::as_str)
        {
            anyhow::bail!("Apollo GraphQL auth failed: {message}");
        }

        data.pointer("/data/Auth/login/result/accessToken")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow::anyhow!("Apollo GraphQL auth returned no access token"))
    }

    async fn ensure_authenticated(&self) -> anyhow::Result<()> {
        if self.token.read().await.is_some() {
            return Ok(());
        }

        let mut token = self.token.write().await;
        if token.is_some() {
            return Ok(());
        }

        *token = Some(self.authenticate().await?);
        Ok(())
    }

    pub async fn send_graphql_command(
        &self,
        command: &str,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.ensure_authenticated().await?;

        let url = format!("http://{}:{}/api/graphql", self.ip, self.port);
        let client = self.client()?;
        let mut body = json!({ "query": command });
        if let Some(vars) = parameters {
            body["variables"] = vars;
        }

        let mut request = client.post(&url).json(&body).timeout(self.timeout);

        if let Some(token) = self.token.read().await.as_ref() {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .context("Apollo GraphQL request failed")?;

        if !response.status().is_success() {
            anyhow::bail!("Apollo GraphQL HTTP error: {}", response.status());
        }

        let json_response = response
            .json::<Value>()
            .await
            .context("Apollo GraphQL parse error")?;

        if let Some(errors) = json_response.get("errors") {
            anyhow::bail!("Apollo GraphQL errors: {errors}");
        }

        json_response
            .get("data")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Apollo GraphQL returned no data"))
    }

    pub async fn get_miner_stats(&self) -> anyhow::Result<Value> {
        let data = self
            .send_graphql_command(
                r#"{
                    Miner {
                        stats {
                            result {
                                stats {
                                    version
                                    statVersion
                                    versions {
                                        miner
                                    }
                                }
                            }
                            error { message }
                        }
                    }
                }"#,
                None,
            )
            .await?;
        let stats = data
            .pointer("/Miner/stats/result/stats")
            .and_then(|v| v.as_array())
            .and_then(|items| items.first())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Apollo GraphQL returned no miner stats"))?;

        Ok(stats)
    }
}

#[async_trait]
impl GraphQLClient for ApolloGraphQLAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.send_graphql_command(command, parameters).await
    }
}

#[async_trait]
impl APIClient for ApolloGraphQLAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::GraphQL { command } => self.send_command(command, false, None).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for Apollo GraphQL client"
            )),
        }
    }
}

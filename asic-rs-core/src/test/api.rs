use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::{data::command::MinerCommand, traits::miner::APIClient};

pub struct MockAPIClient {
    results: HashMap<MinerCommand, Value>,
}

#[async_trait]
impl APIClient for MockAPIClient {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        if let Some(result) = self.results.get(command) {
            Ok(result.clone())
        } else {
            Err(anyhow::anyhow!("Command not found"))
        }
    }
}

impl MockAPIClient {
    pub fn new(results: HashMap<MinerCommand, Value>) -> Self {
        Self { results }
    }
}

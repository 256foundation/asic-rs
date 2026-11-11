use std::{collections::HashSet, net::IpAddr, sync::Mutex};

use asic_rs_core::{
    data::{collector::DataField, command::MinerCommand},
    read_context::ReadContext,
    traits::miner::{APIClient, MinerConstructor},
};
use asic_rs_makes_antminer::models::AntMinerModel;
use async_trait::async_trait;
use semver::Version;
use serde_json::{Value, json};

use super::PowerPlay;

struct RecordingClient {
    summary: Value,
    commands: Mutex<Vec<MinerCommand>>,
}

impl RecordingClient {
    fn new() -> anyhow::Result<Self> {
        Ok(Self {
            summary: serde_json::from_str(include_str!("../test/json/v1/summary.json"))?,
            commands: Mutex::new(Vec::new()),
        })
    }

    fn commands(&self) -> anyhow::Result<Vec<MinerCommand>> {
        self.commands
            .lock()
            .map(|commands| commands.clone())
            .map_err(|_| anyhow::anyhow!("poisoned request log"))
    }
}

#[async_trait]
impl APIClient for RecordingClient {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        self.commands
            .lock()
            .map_err(|_| anyhow::anyhow!("poisoned request log"))?
            .push(command.clone());
        tokio::task::yield_now().await;
        match command {
            MinerCommand::WebAPI {
                command: "summary", ..
            } => Ok(self.summary.clone()),
            _ => Ok(json!({})),
        }
    }
}

fn assert_one_summary(commands: &[MinerCommand]) {
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(
                command,
                MinerCommand::WebAPI {
                    command: "summary",
                    ..
                }
            ))
            .count(),
        1
    );
}

#[tokio::test]
async fn tuning_alone_fetches_summary_and_a_new_context_refreshes() -> anyhow::Result<()> {
    for version in [None, Some(Version::new(1, 28, 4))] {
        let miner = PowerPlay::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP, version);
        let client = RecordingClient::new()?;
        {
            let reads = ReadContext::new_with_client(miner.as_ref(), &client);
            let config = reads.get_tuning_config().await?;
            assert_eq!(config.algorithm.as_deref(), Some("VoltageOptimizer"));
            reads.get_tuning_config().await?;
            assert_eq!(client.commands()?.len(), 1);
            assert_one_summary(&client.commands()?);
        }
        let reads = ReadContext::new_with_client(miner.as_ref(), &client);
        reads.get_tuning_config().await?;
        assert_eq!(client.commands()?.len(), 2);
    }
    Ok(())
}

#[tokio::test]
async fn telemetry_and_tuning_share_responses_in_either_order_or_concurrently() -> anyhow::Result<()>
{
    for (version, request_count) in [(None, 10), (Some(Version::new(1, 28, 4)), 11)] {
        for order in 0..3 {
            let miner = PowerPlay::new(
                IpAddr::from([127, 0, 0, 1]),
                AntMinerModel::S19XP,
                version.clone(),
            );
            let client = RecordingClient::new()?;
            let reads = ReadContext::new_with_client(miner.as_ref(), &client);
            let (data, tuning) = match order {
                0 => (reads.get_data().await, reads.get_tuning_config().await?),
                1 => {
                    let tuning = reads.get_tuning_config().await?;
                    (reads.get_data().await, tuning)
                }
                _ => {
                    let (data, tuning) = tokio::join!(reads.get_data(), reads.get_tuning_config());
                    (data, tuning?)
                }
            };
            assert_eq!(tuning.algorithm.as_deref(), Some("VoltageOptimizer"));
            assert!(data.hashrate.is_some());
            let commands = client.commands()?;
            assert_eq!(commands.len(), request_count);
            assert_eq!(commands.iter().collect::<HashSet<_>>().len(), request_count);
            assert_one_summary(&commands);
            let serialized = serde_json::to_value(data)?;
            assert!(serialized.get("tuning_algorithm").is_none());
            assert!(serialized.get("fan_config").is_none());
        }
    }
    Ok(())
}

#[tokio::test]
async fn cached_responses_do_not_bypass_excluded_telemetry_fields() -> anyhow::Result<()> {
    let miner = PowerPlay::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP, None);
    let client = RecordingClient::new()?;
    let reads = ReadContext::new_with_client(miner.as_ref(), &client);
    reads.get_tuning_config().await?;
    let data = reads.get_data_filtered(vec![DataField::Wattage]).await;
    assert!(data.wattage.is_none());
    assert!(data.hashrate.is_some());
    assert_one_summary(&client.commands()?);
    Ok(())
}

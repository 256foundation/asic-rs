use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    sync::Mutex,
};

use asic_rs_core::{
    config::collector::ConfigField,
    config::fan::FanConfig,
    data::{
        collector::{DataCollector, DataField},
        command::MinerCommand,
        miner::MinerData,
    },
    traits::miner::{APIClient, MinerConstructor},
};
use asic_rs_makes_antminer::models::AntMinerModel;
use async_trait::async_trait;
use semver::Version;
use serde_json::{Value, json};

use super::PowerPlay;

struct RecordingClient {
    summary: Value,
    calls: Mutex<Vec<MinerCommand>>,
}

#[async_trait]
impl APIClient for RecordingClient {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        self.calls
            .lock()
            .map_err(|_| anyhow::anyhow!("poisoned request log"))?
            .push(command.clone());
        match command {
            MinerCommand::WebAPI {
                command: "summary", ..
            } => Ok(self.summary.clone()),
            _ => Ok(json!({})),
        }
    }
}

#[tokio::test]
async fn snapshot_reuses_summary_for_fan_config() -> anyhow::Result<()> {
    let cases = [
        (
            json!({"Fans": {"Fan Mode": {"Auto": {
                "Target Temperature": 60, "Idle Speed": 20
            }}}}),
            Some(FanConfig::auto(60.0, Some(20))),
        ),
        (
            json!({"Fans": {"Fan Mode": {"Auto": {
                "Target Temperature": 65
            }}}}),
            Some(FanConfig::auto(65.0, None)),
        ),
        (
            json!({"Fans": {"Fan Mode": {"Manual": 75}}}),
            Some(FanConfig::manual(75)),
        ),
        (json!({"Fans": {"Fan Mode": {"Auto": {}}}}), None),
        (json!({}), None),
    ];
    for (version, request_count) in [(None, 10), (Some(Version::new(1, 28, 4)), 11)] {
        let miner = PowerPlay::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP, version);
        for (summary, expected) in &cases {
            let client = RecordingClient {
                summary: summary.clone(),
                calls: Mutex::new(Vec::new()),
            };
            let mut collector = DataCollector::new_with_client(miner.as_ref(), &client);
            let values = collector.collect_all().await;
            let snapshot = miner.parse_data(values);
            assert_eq!(&snapshot.fan_config, expected);

            let calls = client
                .calls
                .lock()
                .map_err(|_| anyhow::anyhow!("poisoned request log"))?;
            assert_eq!(calls.len(), request_count);
            assert_eq!(calls.iter().collect::<HashSet<_>>().len(), request_count);
            assert_eq!(
                calls
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

            let config_value = summary
                .pointer("/Fans/Fan Mode")
                .cloned()
                .unwrap_or(Value::Null);
            let config = HashMap::from([(ConfigField::Fan, config_value)]);
            assert_eq!(snapshot.fan_config, miner.parse_fan_config(&config).ok());
            let mut encoded = serde_json::to_value(&snapshot)?;
            let decoded: MinerData = serde_json::from_value(encoded.clone())?;
            assert_eq!(decoded.fan_config, snapshot.fan_config);
            encoded
                .as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("snapshot must be an object"))?
                .remove("fan_config");
            assert!(
                serde_json::from_value::<MinerData>(encoded)?
                    .fan_config
                    .is_none()
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn fan_config_can_be_excluded_without_losing_other_summary_fields() -> anyhow::Result<()> {
    let miner = PowerPlay::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP, None);
    let client = RecordingClient {
        summary: json!({"Session": {"Uptime": 123}}),
        calls: Mutex::new(Vec::new()),
    };
    let mut collector = DataCollector::new_with_client(miner.as_ref(), &client);
    let values = collector.collect(&[DataField::Uptime]).await;
    let snapshot = miner.parse_data(values);
    assert!(snapshot.fan_config.is_none());
    assert_eq!(snapshot.uptime.map(|uptime| uptime.as_secs()), Some(123));
    assert_eq!(
        client
            .calls
            .lock()
            .map_err(|_| anyhow::anyhow!("poisoned request log"))?
            .len(),
        1
    );
    assert!(miner.parse_collected_fan_config(&HashMap::new()).is_none());
    Ok(())
}

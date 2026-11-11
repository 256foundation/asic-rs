use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    sync::Mutex,
};

use asic_rs_core::{
    config::collector::ConfigField,
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
async fn snapshot_reuses_summary_for_tuning_algorithm() -> anyhow::Result<()> {
    let cases = [
        (
            json!({"PerpetualTune": {"Running": true, "Algorithm": {
                "VoltageOptimizer": {"Target": 143, "Unit": "TH/s"}
            }}}),
            Some("VoltageOptimizer".to_string()),
        ),
        (
            json!({"PerpetualTune": {"Running": true, "Algorithm": {
                "PowerTune": {"Target": 3200, "Unit": "W"}
            }}}),
            Some("PowerTune".to_string()),
        ),
        (
            json!({"PerpetualTune": {"Running": false, "Algorithm": {
                "VoltageOptimizer": {"Target": 143, "Unit": "TH/s"}
            }}}),
            None,
        ),
        (
            json!({"PerpetualTune": {"Running": true, "Algorithm": {}}}),
            None,
        ),
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
            assert_eq!(&snapshot.tuning_algorithm, expected);

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

            let config_value = summary.clone();
            let config = HashMap::from([(ConfigField::Tuning, config_value)]);
            assert_eq!(
                snapshot.tuning_algorithm,
                miner
                    .parse_tuning_config(&config)
                    .ok()
                    .and_then(|value| value.algorithm)
            );
            let mut encoded = serde_json::to_value(&snapshot)?;
            let decoded: MinerData = serde_json::from_value(encoded.clone())?;
            assert_eq!(decoded.tuning_algorithm, snapshot.tuning_algorithm);
            encoded
                .as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("snapshot must be an object"))?
                .remove("tuning_algorithm");
            assert!(
                serde_json::from_value::<MinerData>(encoded)?
                    .tuning_algorithm
                    .is_none()
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn tuning_algorithm_can_be_excluded_without_losing_other_summary_fields() -> anyhow::Result<()>
{
    let miner = PowerPlay::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP, None);
    let client = RecordingClient {
        summary: json!({"Session": {"Uptime": 123}}),
        calls: Mutex::new(Vec::new()),
    };
    let mut collector = DataCollector::new_with_client(miner.as_ref(), &client);
    let values = collector.collect(&[DataField::Uptime]).await;
    let snapshot = miner.parse_data(values);
    assert!(snapshot.tuning_algorithm.is_none());
    assert_eq!(snapshot.uptime.map(|uptime| uptime.as_secs()), Some(123));
    assert_eq!(
        client
            .calls
            .lock()
            .map_err(|_| anyhow::anyhow!("poisoned request log"))?
            .len(),
        1
    );
    assert!(miner.parse_tuning_algorithm(&HashMap::new()).is_none());
    Ok(())
}

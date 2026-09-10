use asic_rs_core::data::operating_state::OperatingState;
use serde_json::Value;

pub(super) fn parse(value: &Value) -> Option<OperatingState> {
    let label = value.as_str()?;
    match label {
        "auto-tuning" | "auto_tuning" => Some(OperatingState::Tuning {}),
        "failure" | "failed" | "broken" => Some(OperatingState::Error {}),
        "idle" => Some(OperatingState::Idling {}),
        _ => OperatingState::from_label(label),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, net::IpAddr};

    use asic_rs_core::{
        data::{
            collector::{DataCollector, DataField},
            command::MinerCommand,
        },
        test::api::MockAPIClient,
        traits::miner::Miner,
    };
    use asic_rs_makes_antminer::models::AntMinerModel;
    use serde_json::json;

    use super::*;
    use crate::backends::{VnishV120, VnishV130};

    #[tokio::test]
    async fn both_backends_normalize_states_without_changing_is_mining() {
        let ip = IpAddr::from([127, 0, 0, 1]);
        let miners: [Box<dyn Miner>; 2] = [
            Box::new(VnishV120::new(ip, AntMinerModel::S19XP)),
            Box::new(VnishV130::new(ip, AntMinerModel::S19XP)),
        ];
        let cases = [
            ("mining", OperatingState::Mining {}, true),
            ("auto-tuning", OperatingState::Tuning {}, true),
            ("paused", OperatingState::Paused {}, false),
            ("failure", OperatingState::Error {}, false),
            (
                "new-vnish-state",
                OperatingState::Unknown {
                    raw: "new-vnish-state".into(),
                },
                false,
            ),
        ];
        for miner in &miners {
            for (label, expected, is_mining) in &cases {
                let client = MockAPIClient::new(HashMap::from([(
                    MinerCommand::WebAPI {
                        command: "status",
                        parameters: None,
                    },
                    json!({ "miner_state": label }),
                )]));
                let mut collector = DataCollector::new_with_client(miner.as_ref(), &client);
                let data = collector
                    .collect(&[DataField::OperatingState, DataField::IsMining])
                    .await;
                let snapshot = miner.parse_data(data);
                assert_eq!(snapshot.operating_state.as_ref(), Some(expected));
                assert_eq!(snapshot.is_mining, *is_mining);
            }
        }
    }
}

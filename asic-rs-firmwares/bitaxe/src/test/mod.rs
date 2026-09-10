pub(crate) mod json;

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, net::IpAddr};

    use asic_rs_core::{
        data::{
            collector::{DataCollector, DataField},
            command::MinerCommand,
            operating_state::OperatingState,
        },
        test::api::MockAPIClient,
        traits::miner::Miner,
    };
    use asic_rs_makes_bitaxe::models::BitaxeModel;
    use serde_json::json;

    use crate::backends::{Bitaxe200, Bitaxe290, Bitaxe2140};

    #[tokio::test]
    async fn operating_state_uses_explicit_pause_telemetry() {
        let ip = IpAddr::from([127, 0, 0, 1]);
        let miners: Vec<Box<dyn Miner>> = vec![
            Box::new(Bitaxe200::new(ip, BitaxeModel::Supra)),
            Box::new(Bitaxe290::new(ip, BitaxeModel::Supra)),
            Box::new(Bitaxe2140::new(ip, BitaxeModel::Supra)),
        ];
        for miner in miners {
            for (response, expected) in [
                (
                    json!({"miningPaused": true}),
                    Some(OperatingState::Paused {}),
                ),
                (
                    json!({"miningPaused": false}),
                    Some(OperatingState::Mining {}),
                ),
                (json!({"hashRate": 500}), None),
                (json!({"miningPaused": null}), None),
                (json!({"miningPaused": "false"}), None),
            ] {
                let client = MockAPIClient::new(HashMap::from([(
                    MinerCommand::WebAPI {
                        command: "system/info",
                        parameters: None,
                    },
                    response,
                )]));
                let mut collector = DataCollector::new_with_client(miner.as_ref(), &client);
                let data = collector.collect(&[DataField::OperatingState]).await;
                assert_eq!(miner.parse_operating_state(&data), expected);
                assert_eq!(miner.parse_data(data).operating_state, expected);
            }
        }
    }

    #[tokio::test]
    async fn working_chips_uses_asic_info_without_chip_details() {
        let ip = IpAddr::from([127, 0, 0, 1]);
        let miners: Vec<Box<dyn Miner>> = vec![
            Box::new(Bitaxe290::new(ip, BitaxeModel::Supra)),
            Box::new(Bitaxe2140::new(ip, BitaxeModel::Supra)),
        ];
        for miner in miners {
            for (system_info, asic_info, expected) in [
                (json!({}), Some(json!({"asicCount": 1})), Some(1)),
                (json!({}), Some(json!({"asicCount": 0})), Some(0)),
                (json!({"asicCount": 1}), None, Some(1)),
                (json!({}), None, None),
            ] {
                let mut responses = HashMap::from([(
                    MinerCommand::WebAPI {
                        command: "system/info",
                        parameters: None,
                    },
                    system_info,
                )]);
                if let Some(info) = asic_info {
                    responses.insert(
                        MinerCommand::WebAPI {
                            command: "system/asic",
                            parameters: None,
                        },
                        info,
                    );
                }
                let client = MockAPIClient::new(responses);
                let mut collector = DataCollector::new_with_client(miner.as_ref(), &client);
                let data = collector.collect(&[DataField::Hashboards]).await;
                let boards = miner.parse_hashboards(&data);
                assert!(boards[0].chips.is_empty());
                assert_eq!(boards[0].working_chips, expected);
                assert_eq!(miner.parse_data(data).total_chips, expected);
            }
        }
    }
}

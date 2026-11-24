use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor, Validate},
    model::MinerModel,
};
pub use v1::PowerPlayV1;
pub use v1_28_4::PowerPlayV1284;

mod shared;
pub mod v1;
pub mod v1_28_4;

pub struct PowerPlay;

impl PowerPlay {
    pub(crate) fn build(
        ip: IpAddr,
        model: impl MinerModel,
        version: Option<semver::Version>,
        uninstall_supported: Option<bool>,
    ) -> Box<dyn Miner> {
        if PowerPlayV1284::validate(version.as_ref()) {
            let mut miner = PowerPlayV1284::new(ip, model);
            if let Some(supported) = uninstall_supported {
                miner.set_restore_stock_os_support(supported);
            }
            Box::new(miner)
        } else {
            let mut miner = PowerPlayV1::new(ip, model);
            if let Some(supported) = uninstall_supported {
                miner.set_restore_stock_os_support(supported);
            }
            Box::new(miner)
        }
    }
}

impl MinerConstructor for PowerPlay {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        Self::build(ip, model, version, None)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, net::IpAddr};

    use asic_rs_core::{
        data::{
            collector::{DataCollector, DataField, FieldCollectionStatus},
            command::MinerCommand,
            message::MessageSeverity,
        },
        test::api::MockAPIClient,
    };
    use asic_rs_makes_antminer::models::AntMinerModel;
    use semver::Version;
    use serde_json::json;

    use super::{MinerConstructor, PowerPlay, PowerPlayV1, PowerPlayV1284, Validate};

    const SUMMARY: MinerCommand = MinerCommand::WebAPI {
        command: "summary",
        parameters: None,
    };
    const HISTORY: MinerCommand = MinerCommand::WebAPI {
        command: "errors/history",
        parameters: None,
    };

    const HISTORY_V2: MinerCommand = MinerCommand::WebAPI {
        command: "errors/history/v2",
        parameters: None,
    };

    #[test]
    fn selects_message_sources_by_firmware_version() {
        for (version, uses_history) in [
            (None, false),
            (Some(Version::new(1, 22, 5)), false),
            (Some(Version::new(1, 28, 3)), false),
            (Some(Version::new(1, 28, 4)), true),
            (Some(Version::new(1, 28, 5)), true),
            (Some(Version::new(1, 38, 3)), true),
            (Some(Version::new(1, 39, 20)), true),
            (Some(Version::new(1, 40, 0)), true),
            (Some(Version::new(2, 0, 0)), true),
        ] {
            assert_eq!(PowerPlayV1284::validate(version.as_ref()), uses_history);
            assert_eq!(PowerPlayV1::validate(version.as_ref()), !uses_history);
            let miner = PowerPlay::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP, version);
            let commands: Vec<_> = miner
                .get_locations(DataField::Messages)
                .into_iter()
                .map(|(command, _)| command)
                .collect();
            let expected = if uses_history {
                vec![HISTORY_V2, HISTORY, SUMMARY]
            } else {
                vec![SUMMARY]
            };
            assert_eq!(commands, expected);
        }
    }

    #[test]
    fn backends_parse_legacy_error_values() {
        use asic_rs_core::traits::miner::GetMessages;

        let legacy = PowerPlayV1::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP);
        let history = PowerPlayV1284::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP);
        let summary_data = HashMap::from([(
            DataField::Messages,
            json!({ "Status": { "Last Error": "legacy error" } }),
        )]);
        let history_data = HashMap::from([(
            DataField::Messages,
            json!([{ "NoSolutions": "history error" }]),
        )]);

        assert_eq!(
            legacy.parse_messages(&summary_data)[0].message,
            "legacy error"
        );
        assert!(legacy.parse_messages(&history_data).is_empty());
        assert_eq!(
            history.parse_messages(&history_data)[0].message,
            "history error"
        );
        let messages = history.parse_messages(&summary_data);
        assert_eq!(messages[0].message, "legacy error");
        assert_eq!(messages[0].timestamp, 0);
    }

    #[tokio::test]
    async fn older_and_unknown_firmware_only_collects_summary_errors() {
        for version in [None, Some(Version::new(1, 28, 3))] {
            let miner = PowerPlay::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP, version);
            let client = MockAPIClient::new(HashMap::from([(
                SUMMARY,
                json!({ "Status": { "Last Error": "legacy error" } }),
            )]));
            let mut collector = DataCollector::new_with_client(miner.as_ref(), &client);
            let report = collector.collect_report(&[DataField::Messages]).await;
            let messages = miner.parse_messages(&report.values);

            assert_eq!(report.diagnostics.successful_commands, 1);
            assert_eq!(report.diagnostics.failed_commands, 0);
            assert_eq!(
                report.diagnostics.field_status[&DataField::Messages],
                FieldCollectionStatus::Extracted
            );
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].message, "legacy error");
            assert_eq!(messages[0].timestamp, 0);
        }
    }

    #[tokio::test]
    async fn newer_firmware_collects_history_in_order_with_repeats() {
        let miner = PowerPlay::new(
            IpAddr::from([127, 0, 0, 1]),
            AntMinerModel::S19XP,
            Some(Version::new(1, 28, 4)),
        );
        let client = MockAPIClient::new(HashMap::from([
            (
                HISTORY_V2,
                json!([
                    { "ShutdownTempReached": "shutdown", "occurred_at": 1_793_363_696_u64 },
                    { "ShutdownTempReached": "shutdown", "occurred_at": 1_793_363_695_u64 },
                    { "NoSolutions": "missing occurrence time" },
                    { "NoSolutions": "null occurrence time", "occurred_at": null },
                    { "NoSolutions": "out-of-range occurrence time", "occurred_at": 4_294_967_296_u64 },
                    { "occurred_at": 1_793_363_695_u64 }
                ]),
            ),
            (HISTORY, json!([{ "NoSolutions": "legacy history error" }])),
            (
                SUMMARY,
                json!({ "Status": { "Last Error": "legacy summary error" } }),
            ),
        ]));
        let mut collector = DataCollector::new_with_client(miner.as_ref(), &client);
        let report = collector.collect_report(&[DataField::Messages]).await;
        let messages = miner.parse_messages(&report.values);

        assert_eq!(report.diagnostics.successful_commands, 3);
        assert_eq!(report.diagnostics.failed_commands, 0);
        assert_eq!(
            report.diagnostics.field_status[&DataField::Messages],
            FieldCollectionStatus::Extracted
        );
        assert_eq!(
            messages
                .iter()
                .map(|message| (message.timestamp, message.message.as_str()))
                .collect::<Vec<_>>(),
            [
                (1_793_363_696, "shutdown"),
                (1_793_363_695, "shutdown"),
                (0, "missing occurrence time"),
                (0, "null occurrence time"),
                (0, "out-of-range occurrence time"),
            ]
        );
        assert!(
            messages
                .iter()
                .all(|message| message.severity == MessageSeverity::Error)
        );
    }

    #[tokio::test]
    async fn newer_firmware_falls_back_when_history_is_unavailable() {
        let miner = PowerPlay::new(
            IpAddr::from([127, 0, 0, 1]),
            AntMinerModel::S19XP,
            Some(Version::new(1, 28, 4)),
        );
        let missing_endpoint = json!({ "result": false, "error": "invalid url" });
        let legacy_history = json!([{ "NoSolutions": "legacy history error" }]);
        let summary = json!({ "Status": { "Last Error": "legacy summary error" } });

        for (v2, history, summary, expected, status) in [
            (
                Some(missing_endpoint.clone()),
                Some(legacy_history.clone()),
                Some(summary.clone()),
                Some("legacy history error"),
                FieldCollectionStatus::Extracted,
            ),
            (
                None,
                Some(legacy_history.clone()),
                Some(summary.clone()),
                Some("legacy history error"),
                FieldCollectionStatus::Extracted,
            ),
            (
                None,
                None,
                Some(summary.clone()),
                Some("legacy summary error"),
                FieldCollectionStatus::Extracted,
            ),
            (
                Some(missing_endpoint.clone()),
                Some(missing_endpoint.clone()),
                Some(summary.clone()),
                Some("legacy summary error"),
                FieldCollectionStatus::Extracted,
            ),
            (
                Some(json!([])),
                Some(legacy_history),
                Some(summary.clone()),
                None,
                FieldCollectionStatus::Extracted,
            ),
            (
                None,
                Some(json!([])),
                Some(summary),
                None,
                FieldCollectionStatus::Extracted,
            ),
            (
                None,
                None,
                Some(json!({ "Status": { "Last Error": null } })),
                None,
                FieldCollectionStatus::Extracted,
            ),
            (None, None, None, None, FieldCollectionStatus::Failed),
        ] {
            let responses: HashMap<_, _> =
                [(HISTORY_V2, v2), (HISTORY, history), (SUMMARY, summary)]
                    .into_iter()
                    .filter_map(|(command, value)| value.map(|value| (command, value)))
                    .collect();
            let failed_commands = 3 - responses.len();
            let client = MockAPIClient::new(responses);
            let mut collector = DataCollector::new_with_client(miner.as_ref(), &client);
            let report = collector
                .collect_report(&[DataField::Messages, DataField::OperatingState])
                .await;

            assert_eq!(report.diagnostics.failed_commands, failed_commands);
            assert_eq!(
                report.diagnostics.field_status[&DataField::Messages],
                status
            );
            let messages = miner.parse_messages(&report.values);
            assert_eq!(messages.len(), usize::from(expected.is_some()));
            assert_eq!(
                messages.first().map(|message| message.message.as_str()),
                expected
            );
            assert!(messages.iter().all(|message| message.timestamp == 0));
        }
    }

    #[test]
    fn preserves_discovered_uninstall_support_for_both_backends() {
        for version in [None, Some(Version::new(1, 28, 4))] {
            for supported in [true, false] {
                let miner = PowerPlay::build(
                    IpAddr::from([127, 0, 0, 1]),
                    AntMinerModel::S19XP,
                    version.clone(),
                    Some(supported),
                );
                assert_eq!(miner.supports_restore_stock_os(), supported);
            }
        }
    }
}

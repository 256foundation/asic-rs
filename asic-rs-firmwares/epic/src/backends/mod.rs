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
    use std::collections::HashMap;

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

    use super::*;

    const SUMMARY: MinerCommand = MinerCommand::WebAPI {
        command: "summary",
        parameters: None,
    };
    const HISTORY: MinerCommand = MinerCommand::WebAPI {
        command: "errors/history",
        parameters: None,
    };

    #[test]
    fn selects_message_endpoint_by_firmware_version() {
        for (version, command) in [
            (None, SUMMARY),
            (Some(Version::new(1, 22, 5)), SUMMARY),
            (Some(Version::new(1, 28, 3)), SUMMARY),
            (Some(Version::new(1, 28, 4)), HISTORY),
            (Some(Version::new(1, 28, 5)), HISTORY),
            (Some(Version::new(1, 38, 3)), HISTORY),
            (Some(Version::new(1, 39, 20)), HISTORY),
            (Some(Version::new(1, 40, 0)), HISTORY),
            (Some(Version::new(2, 0, 0)), HISTORY),
        ] {
            assert_eq!(
                PowerPlayV1284::validate(version.as_ref()),
                command == HISTORY
            );
            assert_eq!(PowerPlayV1::validate(version.as_ref()), command == SUMMARY);
            let miner = PowerPlay::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP, version);
            let locations = miner.get_locations(DataField::Messages);
            assert_eq!(locations.len(), 1);
            assert_eq!(locations[0].0, command);
        }
    }

    #[test]
    fn backends_only_parse_their_own_error_source() {
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
        assert!(history.parse_messages(&summary_data).is_empty());
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
        }
    }

    #[tokio::test]
    async fn newer_firmware_collects_history_in_order_with_repeats() {
        let miner = PowerPlay::new(
            IpAddr::from([127, 0, 0, 1]),
            AntMinerModel::S19XP,
            Some(Version::new(1, 28, 4)),
        );
        let history: Vec<_> = (0..10)
            .map(|index| json!({ "NoSolutions": format!("report {}", index / 2) }))
            .collect();
        let expected: Vec<_> = (0..10)
            .map(|index| format!("report {}", index / 2))
            .collect();

        for (history, expected) in [(json!(history), expected), (json!([]), Vec::new())] {
            let client = MockAPIClient::new(HashMap::from([(HISTORY, history)]));
            let mut collector = DataCollector::new_with_client(miner.as_ref(), &client);
            let report = collector.collect_report(&[DataField::Messages]).await;
            let messages = miner.parse_messages(&report.values);

            assert_eq!(report.diagnostics.successful_commands, 1);
            assert_eq!(report.diagnostics.failed_commands, 0);
            assert_eq!(
                report.diagnostics.field_status[&DataField::Messages],
                FieldCollectionStatus::Extracted
            );
            assert_eq!(
                messages
                    .iter()
                    .map(|message| &message.message)
                    .collect::<Vec<_>>(),
                expected.iter().collect::<Vec<_>>()
            );
            assert!(
                messages
                    .iter()
                    .all(|message| message.severity == MessageSeverity::Error)
            );
        }
    }

    #[tokio::test]
    async fn newer_firmware_reports_history_failures_even_when_summary_succeeds() {
        let miner = PowerPlay::new(
            IpAddr::from([127, 0, 0, 1]),
            AntMinerModel::S19XP,
            Some(Version::new(1, 28, 4)),
        );
        for (summary_available, history_available) in [(true, false), (false, true), (false, false)]
        {
            let mut responses = HashMap::new();
            if summary_available {
                responses.insert(
                    SUMMARY,
                    json!({ "Status": { "Last Error": "legacy error" } }),
                );
            }
            if history_available {
                responses.insert(HISTORY, json!([{ "NoSolutions": "no shares received" }]));
            }
            let client = MockAPIClient::new(responses);
            let mut collector = DataCollector::new_with_client(miner.as_ref(), &client);
            let report = collector
                .collect_report(&[DataField::Messages, DataField::OperatingState])
                .await;

            assert_eq!(
                report.diagnostics.failed_commands,
                usize::from(!summary_available) + usize::from(!history_available)
            );
            assert_eq!(
                report.diagnostics.field_status[&DataField::Messages],
                if history_available {
                    FieldCollectionStatus::Extracted
                } else {
                    FieldCollectionStatus::Failed
                }
            );
            assert_eq!(
                miner.parse_messages(&report.values).len(),
                usize::from(history_available)
            );
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

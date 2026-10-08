// SPDX-License-Identifier: Apache-2.0
use crate::{
    backends::v1::KaonsuMiner,
    firmware::{KaonsuFirmware, model_from_overview},
    telemetry,
};
use asic_rs_core::{
    data::{
        collector::{DataCollector, DataField},
        command::MinerCommand,
        device::HashAlgorithm,
        hashrate::HashRateUnit,
        message::MessageSeverity,
        operating_state::OperatingState,
    },
    test::api::MockAPIClient,
    traits::{
        identification::{FirmwareIdentification, WebResponse},
        miner::*,
    },
};
use asic_rs_makes_antminer::models::AntMinerModel;
use serde_json::{Value, json};
use std::collections::HashSet;

const KS5_CONTRACT: &str = include_str!("test/ks5_contract.json");
const MARA_REL314_GH: &str = include_str!("test/mara_rel314_gh.json");
const MARA_REL314_STOPPED: &str = include_str!("test/mara_rel314_stopped.json");
fn fixture() -> Value {
    serde_json::from_str(KS5_CONTRACT).unwrap()
}
async fn collect(fixture: &Value, model: AntMinerModel) -> asic_rs_core::data::miner::MinerData {
    let miner = KaonsuMiner::new("127.0.0.1".parse().unwrap(), model);
    let responses = ["brief", "overview", "hashboards", "fans"]
        .into_iter()
        .map(|command| {
            (
                MinerCommand::WebAPI {
                    command,
                    parameters: None,
                },
                fixture[command].clone(),
            )
        })
        .collect();
    let client = MockAPIClient::new(responses);
    let mut collector = DataCollector::new_with_client(&miner, &client);
    miner.parse_data(collector.collect_all().await)
}

#[tokio::test]
async fn live_mara_gh_units_metadata_and_missing_board_remain_distinct() {
    let fixture: Value = serde_json::from_str(MARA_REL314_GH).unwrap();
    let model = model_from_overview(&fixture["overview"]).unwrap();
    assert_eq!(model, AntMinerModel::KS5Pro);
    let data = collect(&fixture, model).await;
    assert_eq!(data.device_info.algo, HashAlgorithm::KHeavyHash);
    assert!(data.is_mining);
    assert_eq!(data.operating_state, Some(OperatingState::Mining {}));
    assert_eq!(data.expected_hashboards, Some(3));
    assert!(data.device_info.hardware.boards.is_none());
    let control_board = data.control_board_version.unwrap();
    assert_eq!(control_board.name, "CVCtrl");
    assert!(!control_board.known);
    assert_eq!(data.uptime.unwrap().as_secs(), 661101);
    assert!((data.hashrate.as_ref().unwrap().value - 12.724099).abs() < 1e-9);
    assert_eq!(data.total_chips, Some(184));
    assert_eq!(data.hashboards[0].working_chips, Some(0));
    assert_eq!(data.hashboards[0].expected_chips, Some(92));
    assert_eq!(data.hashboards[0].active, Some(false));
    assert!(data.hashboards[0].board_temperature.is_none());
    assert!((data.hashboards[1].hashrate.as_ref().unwrap().value - 6.429394).abs() < 1e-8);
    assert!((data.hashboards[2].hashrate.as_ref().unwrap().value - 6.294704).abs() < 1e-8);
    assert!((data.hashboards[2].board_temperature.unwrap().as_celsius() - 70.511185).abs() < 1e-9);
    assert!(
        (data.hashboards[2]
            .outlet_chip_temperature
            .unwrap()
            .as_celsius()
            - 75.511185)
            .abs()
            < 1e-9
    );
    assert_eq!(data.wattage.unwrap().as_watts(), 1946.0);
    assert_eq!(data.wattage_is_estimated, Some(true));
    assert_eq!(data.wattage_indicator, Some(0));
    assert!((data.reported_max_temperature.unwrap().as_celsius() - 60.511185).abs() < 1e-9);
}

#[tokio::test]
async fn live_stopped_mara_keeps_zero_rates_fans_and_reported_chip_counts() {
    let fixture: Value = serde_json::from_str(MARA_REL314_STOPPED).unwrap();
    let model = model_from_overview(&fixture["overview"]).unwrap();
    let data = collect(&fixture, model).await;
    assert!(!data.is_mining);
    assert_eq!(data.hashrate.as_ref().unwrap().value, 0.0);
    assert_eq!(data.hashrate.as_ref().unwrap().unit, HashRateUnit::TeraHash);
    assert_eq!(data.expected_hashboards, Some(3));
    assert_eq!(data.total_chips, Some(276));
    assert!(data.expected_chips.is_none());
    assert!(data.hashboards.iter().all(|board| {
        board.working_chips == Some(92)
            && board.expected_chips == Some(92)
            && board.active == Some(false)
    }));
    assert_eq!(data.fans.len(), 4);
    assert!(data.fans.iter().all(|fan| fan.rpm.unwrap().as_rpm() == 0.0));
    assert_eq!(data.wattage.unwrap().as_watts(), 25.0);
    assert_eq!(data.wattage_is_estimated, Some(true));
    assert_eq!(data.wattage_firmware_source.as_deref(), Some("EST"));
    assert_eq!(data.uptime.unwrap().as_secs(), 264);
    assert_eq!(
        data.operating_state,
        Some(OperatingState::Unknown {
            raw: "Sleep".to_owned()
        })
    );
}

#[tokio::test]
async fn unknown_product_retains_the_declared_rate_unit() {
    let mut fixture = fixture();
    fixture["overview"]["model_extended"] = json!("Mara unknown model");
    fixture["brief"]["hashrate_unit"] = json!("GH/s");
    fixture["brief"]["hashrate_realtime_10m"] = json!(12570);
    let model = model_from_overview(&fixture["overview"]).unwrap();
    let data = collect(&fixture, model).await;
    let rate = data.hashrate.unwrap();
    assert_eq!(rate.algo, HashAlgorithm::Unknown);
    assert_eq!(rate.unit, HashRateUnit::GigaHash);
    assert_eq!(rate.value, 12570.0);
    assert!(data.device_info.hardware.boards.is_none());
}

#[tokio::test]
async fn board_averages_keep_documented_gh_unit_when_aggregate_is_th() {
    let mut fixture = fixture();
    fixture["hashboards"]["hashboards"][0]["hashrate_average"] = json!(6700);
    fixture["hashboards"]["hashboards"][0]
        .as_object_mut()
        .unwrap()
        .remove("hashrate_average_unit");
    let data = collect(&fixture, AntMinerModel::KS5Pro).await;
    assert_eq!(data.hashboards[0].hashrate.as_ref().unwrap().value, 6.7);
}

#[test]
fn invalid_elapsed_and_board_count_do_not_fabricate_metadata() {
    for invalid in [
        json!(-1),
        json!("nan"),
        json!("inf"),
        json!(null),
        json!({}),
    ] {
        assert!(telemetry::uptime(&json!({"elapsed": invalid})).is_none());
    }
    assert_eq!(
        telemetry::uptime(&json!({"elapsed": 0})).unwrap().as_secs(),
        0
    );
    for invalid in [
        json!(0),
        json!(-1),
        json!(2.5),
        json!(256),
        json!(null),
        json!("nan"),
    ] {
        assert!(telemetry::expected_boards(&json!({"hashboard_num_ideal": invalid})).is_none());
    }
    assert_eq!(
        telemetry::expected_boards(&json!({"hashboard_num_ideal": 3.0})),
        Some(3)
    );
}

#[tokio::test]
async fn source_contract_covers_unit_defaults_and_missing_sensor_fields() {
    let fixture = fixture();
    let data = collect(&fixture, model_from_overview(&fixture["overview"]).unwrap()).await;
    assert_eq!(data.firmware_version.as_deref(), Some("1.0.3"));
    assert!((data.hashrate.unwrap().value - 20.1).abs() < 1e-9);
    assert_eq!(data.expected_hashrate.unwrap().value, 21.0);
    assert_eq!(data.wattage.unwrap().as_watts(), 3400.0);
    assert_eq!(
        data.wattage_source.as_deref(),
        Some("kaonsu.brief:power_consumption_estimated")
    );
    assert_eq!(data.wattage_firmware_source.as_deref(), Some("PSU"));
    assert!(data.average_temperature.is_none());
    assert_eq!(data.fans.len(), 2);
    assert_eq!(data.fans[0].rpm.unwrap().as_rpm(), 5200.0);
    assert!(data.uptime.is_none());
    for board in &data.hashboards {
        assert!(board.expected_chips.is_none());
        assert_eq!(board.inlet_chip_temperature.unwrap().as_celsius(), 65.0);
        assert!(board.board_temperature.is_none());
    }
}

#[tokio::test]
async fn same_protocol_has_algorithm_appropriate_units_for_ks5_l9_and_s19k() {
    for (model, raw, raw_unit, expected, unit, algorithm) in [
        (
            AntMinerModel::KS5Pro,
            0.021,
            "PH/s",
            21.0,
            HashRateUnit::TeraHash,
            HashAlgorithm::KHeavyHash,
        ),
        (
            AntMinerModel::L9,
            16000.0,
            "MH/s",
            16.0,
            HashRateUnit::GigaHash,
            HashAlgorithm::Scrypt,
        ),
        (
            AntMinerModel::S19KPro,
            120000.0,
            "GH/s",
            120.0,
            HashRateUnit::TeraHash,
            HashAlgorithm::SHA256,
        ),
    ] {
        let mut fixture = fixture();
        fixture["brief"]["hashrate_realtime_10m"] = json!(raw);
        fixture["brief"]["hashrate_realtime_10m_unit"] = json!(raw_unit);
        fixture["brief"]["hashrate_stock"] = json!(raw);
        fixture["brief"]["hashrate_stock_unit"] = json!(raw_unit);
        fixture["hashboards"]["hashboards"][0]["hashrate_average"] = json!(raw);
        fixture["hashboards"]["hashboards"][0]["hashrate_average_unit"] = json!(raw_unit);
        let data = collect(&fixture, model).await;
        let rate = data.hashrate.unwrap();
        assert!((rate.value - expected).abs() < 1e-8);
        assert_eq!(rate.unit, unit);
        assert_eq!(rate.algo, algorithm);
        let expected_rate = data.expected_hashrate.unwrap();
        assert!((expected_rate.value - expected).abs() < 1e-8);
        assert_eq!(expected_rate.algo, algorithm);
        let board_rate = data.hashboards[0].hashrate.as_ref().unwrap();
        assert!((board_rate.value - expected).abs() < 1e-8);
        assert_eq!(board_rate.unit, unit);
        assert_eq!(board_rate.algo, algorithm);
    }
}

#[tokio::test]
async fn explicit_zero_chips_rate_power_and_stopped_fan_are_preserved() {
    let mut fixture = fixture();
    fixture["brief"]["hashrate_realtime_10m"] = json!(0);
    fixture["brief"]["hashrate_average"] = json!(20.1);
    fixture["brief"]["power_consumption_estimated"] = json!(0);
    fixture["hashboards"]["hashboards"][0]["asic_num"] = json!(0);
    fixture["hashboards"]["hashboards"][0]["asic_num_ideal"] = json!(120);
    fixture["hashboards"]["hashboards"][0]["hashrate_realtime_10m"] = json!(0);
    fixture["fans"]["fans"][0] = json!({"current_speed": 0, "rpm": 5200});
    let data = collect(&fixture, AntMinerModel::KS5Pro).await;
    assert_eq!(data.hashrate.unwrap().value, 0.0);
    assert!(!data.is_mining);
    assert_eq!(data.wattage.unwrap().as_watts(), 0.0);
    assert_eq!(data.wattage_is_estimated, Some(true));
    assert_eq!(data.hashboards[0].working_chips, Some(0));
    assert_eq!(data.hashboards[0].expected_chips, Some(120));
    assert_eq!(data.hashboards[0].hashrate.as_ref().unwrap().value, 0.0);
    assert_eq!(data.hashboards[0].active, Some(false));
    assert_eq!(data.fans[0].rpm.unwrap().as_rpm(), 0.0);
}

#[tokio::test]
async fn explicit_pause_and_failure_override_lingering_rate_and_keep_status() {
    for status in ["Paused", "Failure"] {
        let mut fixture = fixture();
        fixture["brief"]["status"] = json!(status);
        fixture["brief"]["status_long"] = json!(format!("{status}: reported reason"));
        let data = collect(&fixture, AntMinerModel::KS5Pro).await;
        assert!(!data.is_mining);
        assert_eq!(data.operating_state, OperatingState::from_label(status));
        assert!(
            data.messages
                .iter()
                .any(|message| message.message.contains("reported reason"))
        );
        if status == "Failure" {
            assert_eq!(data.messages[0].severity, MessageSeverity::Error);
        }
    }
}

#[tokio::test]
async fn board_failure_keeps_raw_status_description_and_identity() {
    let mut fixture = fixture();
    fixture["hashboards"]["hashboards"][1]["status"] = json!("Failure");
    fixture["hashboards"]["hashboards"][1]["description"] = json!("board contract reason");
    let data = collect(&fixture, AntMinerModel::KS5Pro).await;
    assert_eq!(data.hashboards[1].active, Some(false));
    assert_eq!(data.hashboards[1].working_chips, Some(120));
    assert!(
        data.messages
            .iter()
            .any(|message| message.message == "Hashboard 1: Failure"
                && message.severity == MessageSeverity::Error)
    );
    assert!(data.messages.iter().any(|message| message.message
        == "Hashboard 1: board contract reason"
        && message.severity == MessageSeverity::Error));
}

#[tokio::test]
async fn malformed_readings_do_not_invent_provenance_or_reuse_stale_values() {
    let mut fixture = fixture();
    fixture["brief"]["hashrate_realtime_10m"] = json!("NaN");
    fixture["brief"]["hashrate_average"] = json!(20.1);
    fixture["brief"]["power_consumption_estimated"] = json!(-1);
    fixture["brief"]["power_indicator"] = json!({"unexpected": 0});
    let data = collect(&fixture, AntMinerModel::KS5Pro).await;
    assert!(data.hashrate.is_none());
    assert!(!data.is_mining);
    assert!(data.wattage.is_none());
    assert!(data.wattage_source.is_none());
    assert!(data.wattage_is_estimated.is_none());
    assert!(data.wattage_firmware_source.is_none());
    assert!(data.wattage_indicator.is_none());
    let value =
        json!({"hashrate_realtime": 20.0, "hashrate_unit": "not a unit", "hashrate_average": 21.0});
    assert!(telemetry::hashrate(&value, HashAlgorithm::KHeavyHash).is_none());
}

#[tokio::test]
async fn wrapped_objects_board_sensor_domains_and_missing_metadata_stay_distinct() {
    let mut fixture = fixture();
    fixture["hashboards"]["hashboards"][0]["temperature_pcb"] = json!({"inlet": 45, "outlet": 52});
    fixture["hashboards"]["hashboards"][0]["temperature_raw"] = json!([0, -20, 200]);
    fixture["hashboards"]["hashboards"][0]["present"] = json!(false);
    fixture["brief"]
        .as_object_mut()
        .unwrap()
        .remove("power_source");
    fixture["brief"]
        .as_object_mut()
        .unwrap()
        .remove("power_indicator");
    for key in ["brief", "overview", "hashboards", "fans"] {
        fixture[key] = json!({"data": fixture[key].clone()});
    }
    let data = collect(&fixture, AntMinerModel::KS5Pro).await;
    assert_eq!(
        data.hashboards[0].board_temperature.unwrap().as_celsius(),
        52.0
    );
    assert_eq!(
        data.hashboards[0]
            .outlet_chip_temperature
            .unwrap()
            .as_celsius(),
        68.0
    );
    assert_eq!(data.hashboards[0].active, Some(false));
    assert!(data.wattage_firmware_source.is_none());
    assert!(data.wattage_indicator.is_none());
    assert!(data.fluid_temperature.is_none());
    assert_eq!(data.reported_max_temperature.unwrap().as_celsius(), 70.0);
}

#[test]
fn reported_max_temperature_uses_only_the_exact_max_field() {
    assert_eq!(
        telemetry::reported_max_temperature(&json!({"temperature_max": 170}))
            .unwrap()
            .as_celsius(),
        170.0
    );
    for value in [
        json!({"temperature_average": 70}),
        json!({"temperature_max": 0}),
        json!({"temperature_max": "NaN"}),
        json!({"temperature_max": 200}),
    ] {
        assert!(telemetry::reported_max_temperature(&value).is_none());
    }
}

#[test]
fn board_ids_and_documented_gh_fields_do_not_require_magnitude_guesses() {
    let boards = json!({"hashboard_num_ideal": 3, "hashboards": [
        {"id": 3, "hashrate_average": 6700, "asic_num": 120},
        {"id": 1, "hashrate_average": 0, "asic_num": 0},
        {"id": 2, "hashrate_average": 6700, "asic_num": 120}
    ]});
    let parsed = telemetry::boards(&boards, HashAlgorithm::KHeavyHash);
    assert_eq!(
        parsed
            .iter()
            .map(|board| board.position)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(parsed[0].working_chips, Some(0));
    assert_eq!(parsed[1].hashrate.as_ref().unwrap().value, 6.7);
    assert_eq!(
        telemetry::expected_hashrate(&json!({"hashrate_ideal": 21000}), HashAlgorithm::KHeavyHash)
            .unwrap()
            .value,
        21.0
    );
    assert!(
        telemetry::boards(
            &json!({"hashboards": [{"id": 0}, {"id": 0}]}),
            HashAlgorithm::KHeavyHash
        )
        .is_empty()
    );
    assert!(
        telemetry::boards(&json!({"hashboards": [null]}), HashAlgorithm::KHeavyHash).is_empty()
    );
}

#[test]
fn partial_or_invalid_board_identity_does_not_relocate_observed_boards() {
    for rows in [
        json!([{"index": 2}, {"index": "bad"}]),
        json!([{"index": 2}, {}]),
        json!([{"index": null}, {}]),
        json!([{"index": -1}, {"index": 2}]),
        json!([{"index": 1.5}, {"index": 2}]),
        json!([{"index": 0}, {"index": 0}]),
        json!([{"index": "bad"}, {"index": "bad"}]),
    ] {
        assert!(
            telemetry::boards(&json!({"hashboards": rows}), HashAlgorithm::KHeavyHash).is_empty()
        );
    }
    let parsed = telemetry::boards(
        &json!({"hashboards": [{"asic_num": 91}, {"asic_num": 92}]}),
        HashAlgorithm::KHeavyHash,
    );
    assert_eq!(
        parsed
            .iter()
            .map(|board| (board.position, board.working_chips))
            .collect::<Vec<_>>(),
        vec![(0, Some(91)), (1, Some(92))]
    );
}

#[test]
fn explicit_indices_keep_gaps_and_alternate_ids_require_known_numbering() {
    for rows in [
        json!([{"index": 2, "asic_num": 92}]),
        json!([{"index": 2, "id": 3, "asic_num": 92}]),
    ] {
        let parsed = telemetry::boards(&json!({"hashboards": rows}), HashAlgorithm::KHeavyHash);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].position, 2);
        assert_eq!(parsed[0].working_chips, Some(92));
    }
    let parsed = telemetry::boards(
        &json!({"hashboard_num_ideal": 3, "hashboards": [{"index": 1}, {"index": 2}]}),
        HashAlgorithm::KHeavyHash,
    );
    assert_eq!(
        parsed
            .iter()
            .map(|board| board.position)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    for value in [
        json!({"hashboards": [{"id": 1}, {"id": 2}, {"id": 3}]}),
        json!({"hashboard_num_ideal": 3, "hashboards": [{"id": 1}, {"id": 3}]}),
        json!({"hashboard_num_ideal": 3, "hashboards": [{"id": 2}, {"id": 3}, {"id": 4}]}),
        json!({"hashboards": [{"slot": 2}]}),
        json!({"hashboards": [{"index": 0}, {"id": 1}]}),
    ] {
        assert!(telemetry::boards(&value, HashAlgorithm::KHeavyHash).is_empty());
    }
}

#[test]
fn firmware_identity_requires_mara_signature_and_exact_overview_product() {
    let firmware = KaonsuFirmware;
    let web = |body, auth_header| WebResponse {
        body,
        auth_header,
        algo_header: "MD5",
        redirect_header: "",
        status: 401,
    };
    assert!(firmware.identify_web(&web("", "Digest realm=\"MaraFW\"")));
    assert!(!firmware.identify_web(&web("Antminer KS5 Pro", "Digest realm=\"antMiner\"")));
    assert!(!firmware.identify_rpc("ANTMINER KS5 PRO"));
    assert!(firmware.identify_rpc("KAONSU"));
    assert_eq!(
        model_from_overview(&json!({"model_extended": "Antminer S19k Pro"})).unwrap(),
        AntMinerModel::S19KPro
    );
    assert_eq!(
        model_from_overview(&json!({"model_extended": "Antminer L9"})).unwrap(),
        AntMinerModel::L9
    );
    assert_eq!(
        model_from_overview(&json!({"model": "Mara unknown model"})).unwrap(),
        AntMinerModel::Unknown("Mara unknown model".to_owned())
    );
    assert!(model_from_overview(&json!({})).is_err());
    assert!(
        model_from_overview(&json!({"model": "Antminer L9", "model_extended": "Antminer KS5 Pro"}))
            .is_err()
    );
}

#[tokio::test]
async fn only_four_read_paths_are_available_and_no_controls_are_advertised() {
    let miner = KaonsuMiner::new("127.0.0.1".parse().unwrap(), AntMinerModel::KS5Pro);
    let commands = [
        DataField::Hashrate,
        DataField::ExpectedHashrate,
        DataField::Wattage,
        DataField::Messages,
        DataField::OperatingState,
        DataField::Hashboards,
        DataField::Fans,
        DataField::FirmwareVersion,
    ]
    .into_iter()
    .flat_map(|field| miner.get_locations(field))
    .map(|(command, _)| command)
    .collect::<HashSet<_>>();
    assert_eq!(commands.len(), 4);
    for command in commands {
        assert!(matches!(
            command,
            MinerCommand::WebAPI {
                command: "brief" | "overview" | "hashboards" | "fans",
                parameters: None
            }
        ));
    }
    assert!(!miner.supports_pause());
    assert!(!miner.supports_resume());
    assert!(!miner.supports_restart());
    assert!(!miner.supports_set_fault_light());
    assert!(!miner.supports_set_power_limit());
    assert!(!miner.supports_pools_config());
    assert!(!miner.supports_scaling_config());
    assert!(!miner.supports_tuning_config());
    assert!(!miner.supports_fan_config());
    assert!(!miner.supports_upgrade_firmware());
    assert!(!miner.supports_factory_reset());
    assert!(!miner.supports_change_password());
    assert!(!miner.supports_set_hashboards_enabled());
    assert!(!miner.supports_set_tuning_percent());
    for command in ["miner_cfg", "miner_config", "pools", "curtailment"] {
        assert!(
            miner
                .get_api_result(&MinerCommand::WebAPI {
                    command,
                    parameters: None
                })
                .await
                .is_err()
        );
    }
    assert!(
        miner
            .get_api_result(&MinerCommand::WebAPI {
                command: "brief",
                parameters: Some(json!({}))
            })
            .await
            .is_err()
    );
    assert!(
        miner
            .get_api_result(&MinerCommand::RPC {
                command: "restart",
                parameters: None
            })
            .await
            .is_err()
    );
    assert!(miner.pause(None).await.is_err());
    assert!(miner.resume(None).await.is_err());
    assert!(miner.restart().await.is_err());
    assert!(miner.get_config_collector().collect(&[]).await.is_empty());
}

// Support Extension additions: offline stock telemetry regression contracts.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use asic_rs_core::{
    data::{
        collector::DataCollector,
        command::MinerCommand,
        device::{HashAlgorithm, MinerHardware},
        hashrate::HashRateUnit,
    },
    test::api::MockAPIClient,
    traits::miner::*,
};
use asic_rs_makes_antminer::models::AntMinerModel;
use serde_json::{Value, json};

use super::{telemetry::*, v2020::AntMinerV2020, v2023_07::AntMinerV202307};
use crate::test::json::v2023_07::{
    S21_HYDRO_COOLANT_SYNTHETIC, S21_PLUS_HYDRO_SYNTHETIC, S23_HYDRO_STANDARD_SYNTHETIC,
};

fn row(fixture: &str) -> Value {
    extract_stats(&serde_json::from_str(fixture).unwrap(), None)
        .unwrap()
        .clone()
}

fn no_assumed_hardware() -> MinerHardware {
    MinerHardware {
        fans: None,
        boards: None,
    }
}

#[test]
fn aggregate_location_accepts_both_api_shapes_and_rejects_ambiguity() {
    assert_eq!(row(S21_PLUS_HYDRO_SYNTHETIC)["rate_5s"], json!(600000));
    assert_eq!(row(S21_HYDRO_COOLANT_SYNTHETIC)["rate_5s"], json!(0));
    assert!(extract_stats(&json!({"STATS": [{"rate_5s": 100}, {"GHS 5s": 200}]}), None).is_none());
}

#[test]
fn modern_s21_plus_hydro_keeps_sensor_domains_separate() {
    let boards = hashboards(
        &row(S21_PLUS_HYDRO_SYNTHETIC),
        "S21+ Hyd",
        HashAlgorithm::SHA256,
        &no_assumed_hardware(),
    );
    let board = &boards[0];
    assert_eq!(board.position, 2);
    assert_eq!(board.hashrate.as_ref().unwrap().value, 600.0);
    assert_eq!(board.board_temperature.unwrap().as_celsius(), 55.0);
    assert_eq!(board.inlet_chip_temperature.unwrap().as_celsius(), 80.0);
    assert_eq!(board.outlet_chip_temperature.unwrap().as_celsius(), 80.0);
    assert_eq!(board.inlet_fluid_temperature.unwrap().as_celsius(), 30.0);
    assert_eq!(board.outlet_fluid_temperature.unwrap().as_celsius(), 39.0);
    assert_eq!(board.expected_chips, None);
    assert_eq!(board.working_chips, Some(120));
    assert_eq!(board.active, Some(true));
}

#[test]
fn s21_hydro_coolant_pair_never_becomes_chip_or_protective_pcb_temperature() {
    for model in ["S21 XP Hyd", "S21XPHydro", "S21eXPHydro", "S21jXPHydro"] {
        let boards = hashboards(
            &row(S21_HYDRO_COOLANT_SYNTHETIC),
            model,
            HashAlgorithm::SHA256,
            &no_assumed_hardware(),
        );
        let board = &boards[0];
        assert_eq!(board.board_temperature, None, "{model}");
        assert_eq!(board.outlet_chip_temperature, None, "{model}");
        assert_eq!(
            board.inlet_fluid_temperature.unwrap().as_celsius(),
            30.0,
            "{model}"
        );
        assert_eq!(
            board.outlet_fluid_temperature.unwrap().as_celsius(),
            41.0,
            "{model}"
        );
        assert_eq!(board.active, Some(false));
    }
}

#[test]
fn ordinary_hydro_chip_arrays_remain_chip_arrays() {
    for model in ["S23 Hyd", "S19 XP Hyd", "S21 XP Hyd"] {
        let boards = hashboards(
            &row(S23_HYDRO_STANDARD_SYNTHETIC),
            model,
            HashAlgorithm::SHA256,
            &no_assumed_hardware(),
        );
        assert_eq!(boards[0].board_temperature.unwrap().as_celsius(), 54.0);
        assert_eq!(
            boards[0].outlet_chip_temperature.unwrap().as_celsius(),
            82.0
        );
        assert_eq!(boards[0].inlet_fluid_temperature, None);
    }
    let boards = hashboards(
        &row(S21_HYDRO_COOLANT_SYNTHETIC),
        "S19 XP Hyd",
        HashAlgorithm::SHA256,
        &no_assumed_hardware(),
    );
    assert_eq!(
        boards[0].outlet_chip_temperature.unwrap().as_celsius(),
        41.0
    );
    assert_eq!(boards[0].inlet_fluid_temperature, None);
}

#[test]
fn invalid_sensor_readings_are_not_reported_as_cooling_proof() {
    let value = json!({"chain": [{"index": 0, "temp_pcb": [0, -10, 170, "NaN"], "temp_chip": [0, "infinity"]}]});
    let boards = hashboards(&value, "S21", HashAlgorithm::SHA256, &no_assumed_hardware());
    assert_eq!(boards[0].board_temperature, None);
    assert_eq!(boards[0].outlet_chip_temperature, None);
    let value = json!({"chain": [{"index": 0, "temp_pcb": "-10", "temp_chip": "-20"}]});
    let boards = hashboards(&value, "S21", HashAlgorithm::SHA256, &no_assumed_hardware());
    assert_eq!(boards[0].board_temperature, None);
    assert_eq!(boards[0].outlet_chip_temperature, None);
    let value = json!({"chain": [{"index": 0, "temp_pcb": "1e-3"}]});
    let boards = hashboards(&value, "S21", HashAlgorithm::SHA256, &no_assumed_hardware());
    // Scientific notation is a scalar, never split at the exponent sign.
    assert!(boards[0].board_temperature.unwrap().as_celsius() < 0.01);
}

#[test]
fn zero_current_rate_overrides_lingering_average_and_config_intent() {
    let value = row(S21_HYDRO_COOLANT_SYNTHETIC);
    let rate = hashrate(&value, HashAlgorithm::SHA256).unwrap();
    assert_eq!(rate.value, 0.0);
    assert!(!is_mining(
        Some(&json!(0)),
        current_rate(&value, HashAlgorithm::SHA256)
    ));
    assert!(!is_mining(Some(&json!("0")), None));
    assert!(!is_mining(
        Some(&json!(1)),
        hashrate(&json!({"GHS 5s": 200000}), HashAlgorithm::SHA256)
    ));
}

#[test]
fn malformed_or_conflicting_current_rate_does_not_use_average() {
    for value in [
        json!({"rate_5s": -1, "rate_avg": 100, "rate_unit": "GH"}),
        json!({"rate_5s": "NaN", "rate_avg": 100, "rate_unit": "GH"}),
        json!({"rate_5s": 1e308, "rate_avg": 100, "rate_unit": "GH"}),
        json!({"rate_5s": 100, "GHS 5s": 200, "rate_avg": 100, "rate_unit": "GH"}),
    ] {
        assert!(hashrate(&value, HashAlgorithm::SHA256).is_none());
    }
}

#[test]
fn scrypt_modern_units_are_explicit_and_legacy_board_units_independent() {
    assert!(hashrate(&json!({"rate_5s": 16000}), HashAlgorithm::Scrypt).is_none());
    let value = json!({"MHS 5s": 16000, "rate_unit": "MH", "chain_acn1": 180, "chain_rate1": 16});
    let rate = hashrate(&value, HashAlgorithm::Scrypt).unwrap();
    assert_eq!(rate.value, 16.0);
    assert_eq!(rate.unit, HashRateUnit::GigaHash);
    let boards = hashboards(&value, "L9", HashAlgorithm::Scrypt, &no_assumed_hardware());
    assert_eq!(boards[0].hashrate.as_ref().unwrap().value, 16.0);
    assert!(expected_hashrate(&json!({"rate_ideal": 16000}), HashAlgorithm::Scrypt).is_none());
    assert!(expected_hashrate(&json!({"hashrate": 16000}), HashAlgorithm::Equihash).is_none());
    assert!(
        expected_hashrate(&json!({"total_rateideal": 16000}), HashAlgorithm::Equihash).is_none()
    );
    assert_eq!(
        expected_hashrate(&json!({"total_rateideal": 17.03}), HashAlgorithm::Scrypt)
            .unwrap()
            .value,
        17.03
    );
    assert_eq!(
        expected_hashrate(
            &json!({"rate_ideal": 16000, "rate_unit": "MH"}),
            HashAlgorithm::Scrypt
        )
        .unwrap()
        .value,
        16.0
    );
}

#[test]
fn duplicate_modern_board_slots_are_rejected_and_legacy_padding_skipped() {
    assert!(
        hashboards(
            &json!({"chain": [{"index": 1}, {"index": 1}]}),
            "S21",
            HashAlgorithm::SHA256,
            &no_assumed_hardware()
        )
        .is_empty()
    );
    assert!(
        hashboards(
            &json!({"chain": [null]}),
            "S21",
            HashAlgorithm::SHA256,
            &no_assumed_hardware()
        )
        .is_empty()
    );
    let value = json!({"chain_acn1": 0, "chain_rate1": 0, "chain_acs1": "", "chain_acn6": 114, "chain_rate6": 47000, "temp_pcb6": "40-51-49", "temp_chip6": "71-82-74", "chain_acn7": 114, "chain_rate7": 0});
    let boards = hashboards(&value, "S19", HashAlgorithm::SHA256, &no_assumed_hardware());
    assert_eq!(boards.len(), 2);
    assert_eq!(boards[0].position, 5);
    assert_eq!(boards[0].board_temperature.unwrap().as_celsius(), 51.0);
    assert_eq!(
        boards[0].outlet_chip_temperature.unwrap().as_celsius(),
        82.0
    );
    assert_eq!(boards[1].active, Some(false));
}

#[test]
fn stopped_fans_remain_visible_and_hydro_has_no_invented_fans() {
    let fans = fans(&json!({"fan": ["6000", 0, -10]}));
    assert_eq!(fans.len(), 2);
    assert_eq!(fans[1].rpm.unwrap().as_rpm(), 0.0);
    assert!(super::telemetry::fans(&row(S21_PLUS_HYDRO_SYNTHETIC)).is_empty());
}

#[test]
fn actual_power_is_optional_zero_is_valid_and_conflicts_are_rejected() {
    assert_eq!(
        wattage(&row(S23_HYDRO_STANDARD_SYNTHETIC))
            .unwrap()
            .as_watts(),
        5000.0
    );
    assert_eq!(wattage(&json!({"power": 0})).unwrap().as_watts(), 0.0);
    assert!(wattage(&json!({"power": 5000, "watt": 3500})).is_none());
    assert!(wattage(&json!({"power": "NaN"})).is_none());
    assert!(wattage(&row(S21_PLUS_HYDRO_SYNTHETIC)).is_none());
}

#[test]
fn partial_new_api_power_does_not_discard_legacy_board_and_fan_data() {
    let value = json!({"modern": {"watt": 3200}, "legacy": {"chain_acn1": 114, "chain_rate1": 100000, "fan1": 6000, "total_rateideal": 110000}});
    assert_eq!(
        hashboards(&value, "S19", HashAlgorithm::SHA256, &no_assumed_hardware()).len(),
        1
    );
    assert_eq!(fans(&value).len(), 1);
    assert_eq!(
        expected_hashrate(&value, HashAlgorithm::SHA256)
            .unwrap()
            .value,
        110.0
    );
    assert_eq!(wattage(&value).unwrap().as_watts(), 3200.0);
}

#[tokio::test]
async fn recognized_s21_plus_hydro_collects_separate_sensors_using_canonical_model_name() {
    let miner = AntMinerV202307::new("127.0.0.1".parse().unwrap(), AntMinerModel::S21PlusHydro);
    let mut responses = HashMap::new();
    responses.insert(
        MinerCommand::RPC {
            command: "stats",
            parameters: Some(json!({"new_api": true})),
        },
        serde_json::from_str(S21_PLUS_HYDRO_SYNTHETIC).unwrap(),
    );
    let mock = MockAPIClient::new(responses);
    let mut collector = DataCollector::new_with_client(&miner, &mock);
    let data = miner.parse_data(collector.collect_all().await);
    assert_eq!(data.hashrate.unwrap().value, 600.0);
    assert!(data.is_mining);
    assert_eq!(
        data.hashboards[0].board_temperature.unwrap().as_celsius(),
        55.0
    );
    assert_eq!(data.fluid_temperature.unwrap().as_celsius(), 30.0);
    assert_eq!(data.outlet_fluid_temperature.unwrap().as_celsius(), 39.0);
    assert!(data.fans.is_empty());
    assert!(data.wattage.is_none());
}

#[tokio::test]
async fn both_firmware_generations_collect_zero_current_rate_without_false_mining() {
    for miner in [
        Box::new(AntMinerV2020::new(
            "127.0.0.1".parse().unwrap(),
            AntMinerModel::S21Hydro,
        )) as Box<dyn Miner>,
        Box::new(AntMinerV202307::new(
            "127.0.0.1".parse().unwrap(),
            AntMinerModel::S21Hydro,
        )) as Box<dyn Miner>,
    ] {
        let mut responses = HashMap::new();
        responses.insert(
            MinerCommand::RPC {
                command: "stats",
                parameters: Some(json!({"new_api": true})),
            },
            serde_json::from_str(S21_HYDRO_COOLANT_SYNTHETIC).unwrap(),
        );
        responses.insert(
            MinerCommand::WebAPI {
                command: "get_miner_conf",
                parameters: None,
            },
            json!({"bitmain-work-mode": 0}),
        );
        let mock = MockAPIClient::new(responses);
        let mut collector = DataCollector::new_with_client(miner.as_ref(), &mock);
        let data = miner.parse_data(collector.collect_all().await);
        assert_eq!(data.hashrate.unwrap().value, 0.0);
        assert!(!data.is_mining);
        assert!(data.average_temperature.is_none());
        assert!(data.hashboards[0].outlet_chip_temperature.is_none());
        assert_eq!(data.fluid_temperature.unwrap().as_celsius(), 30.0);
        assert_eq!(data.outlet_fluid_temperature.unwrap().as_celsius(), 41.0);
    }
}

#[tokio::test]
async fn separate_config_mode_extractors_do_not_overwrite_pause_or_conflict() {
    for config in [
        json!({"bitmain-work-mode": 1, "miner-mode": 0}),
        json!({"bitmain-work-mode": 0, "miner-mode": 1}),
        json!({"bitmain-work-mode": 0, "miner-mode": 2}),
        json!({"miner-mode": "unrecognized"}),
        json!({"miner-mode": true}),
    ] {
        for miner in [
            Box::new(AntMinerV2020::new(
                "127.0.0.1".parse().unwrap(),
                AntMinerModel::S21PlusHydro,
            )) as Box<dyn Miner>,
            Box::new(AntMinerV202307::new(
                "127.0.0.1".parse().unwrap(),
                AntMinerModel::S21PlusHydro,
            )) as Box<dyn Miner>,
        ] {
            let mut responses = HashMap::new();
            responses.insert(
                MinerCommand::RPC {
                    command: "stats",
                    parameters: Some(json!({"new_api": true})),
                },
                serde_json::from_str(S21_PLUS_HYDRO_SYNTHETIC).unwrap(),
            );
            responses.insert(
                MinerCommand::WebAPI {
                    command: "get_miner_conf",
                    parameters: None,
                },
                config.clone(),
            );
            let mock = MockAPIClient::new(responses);
            let mut collector = DataCollector::new_with_client(miner.as_ref(), &mock);
            let data = miner.parse_data(collector.collect_all().await);
            assert_eq!(data.hashrate.unwrap().value, 600.0);
            assert!(!data.is_mining, "{config}");
        }
    }
}

#[tokio::test]
async fn malformed_modern_chain_is_not_reported_as_a_board() {
    let miner = AntMinerV202307::new("127.0.0.1".parse().unwrap(), AntMinerModel::S21PlusHydro);
    let mut stats: Value = serde_json::from_str(S21_PLUS_HYDRO_SYNTHETIC).unwrap();
    stats["STATS"][0]["chain"] = json!([null]);
    let mut responses = HashMap::new();
    responses.insert(
        MinerCommand::RPC {
            command: "stats",
            parameters: Some(json!({"new_api": true})),
        },
        stats,
    );
    let mock = MockAPIClient::new(responses);
    let mut collector = DataCollector::new_with_client(&miner, &mock);
    let data = miner.parse_data(collector.collect_all().await);
    assert!(data.hashboards.is_empty());
    assert!(data.fluid_temperature.is_none());
    assert!(data.average_temperature.is_none());
}

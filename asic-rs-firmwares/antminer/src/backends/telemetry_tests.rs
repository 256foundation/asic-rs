// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, str::FromStr};

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
    KS7_MODERN_STATS_CAPTURED, S21_HYDRO_COOLANT_SYNTHETIC, S21_PLUS_HYDRO_SYNTHETIC,
    S21_XP_HYDRO_WEB_STATS_CAPTURED, S21_XP_HYDRO_WEB_SUMMARY_CAPTURED,
    S21J_XP_HYDRO_RPC_STATS_CAPTURED, S21J_XP_HYDRO_WEB_STATS_CAPTURED,
    S21J_XP_HYDRO_WEB_SUMMARY_CAPTURED, S21PROPLUS_MODERN_STATS_CAPTURED,
    S23_HYDRO_STANDARD_SYNTHETIC, Z15_MALFORMED_STATS_CAPTURED, Z15_SUMMARY_CAPTURED,
    Z15PRO_LEGACY_STATS_CAPTURED, Z15PRO_MODERN_STATS_CAPTURED, Z15PRO_SUMMARY_CAPTURED,
};

fn row(fixture: &str) -> Value {
    extract_stats(&serde_json::from_str(fixture).unwrap(), None)
        .unwrap()
        .clone()
}

#[tokio::test]
async fn captured_product_identities_preserve_algorithm_and_observed_counts() {
    for (fixture, expected_model, algorithm, chips, expected_chips) in [
        (
            KS7_MODERN_STATS_CAPTURED,
            "KS7",
            HashAlgorithm::KHeavyHash,
            108,
            None,
        ),
        (
            S21PROPLUS_MODERN_STATS_CAPTURED,
            "S21ProPlus",
            HashAlgorithm::SHA256,
            65,
            Some(65),
        ),
    ] {
        let raw: Value = serde_json::from_str(fixture).unwrap();
        let model = AntMinerModel::from_str(
            &raw.pointer("/INFO/type")
                .unwrap()
                .as_str()
                .unwrap()
                .to_uppercase(),
        )
        .unwrap();
        let miner = AntMinerV202307::new("127.0.0.1".parse().unwrap(), model);
        let mock = MockAPIClient::new(HashMap::from([(
            MinerCommand::RPC {
                command: "stats",
                parameters: Some(json!({"new_api": true})),
            },
            raw.clone(),
        )]));
        let mut collector = DataCollector::new_with_client(&miner, &mock);
        let data = miner.parse_data(collector.collect_all().await);
        assert_eq!(data.device_info.model, expected_model);
        assert_eq!(data.device_info.algo, algorithm);
        assert_eq!(
            data.hashrate.unwrap().value,
            raw["STATS"][0]["rate_5s"].as_f64().unwrap() / 1000.0
        );
        assert!(data.is_mining);
        assert_eq!(data.hashboards.len(), 3);
        for board in data.hashboards {
            assert_eq!(board.working_chips, Some(chips));
            assert_eq!(board.expected_chips, expected_chips);
        }
        assert_eq!(data.fans.len(), 4);
    }
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
fn plausible_hot_board_and_chip_readings_remain_visible() {
    let boards = hashboards(
        &json!({"chain": [
            {"index": 1, "temp_pcb": [0, 180, 255], "temp_chip": [180, 200, 255]},
            {"index": 2, "temp_pcb": [255], "temp_chip": [255]}
        ]}),
        "S19",
        HashAlgorithm::SHA256,
        &no_assumed_hardware(),
    );
    assert_eq!(boards[0].board_temperature.unwrap().as_celsius(), 180.0);
    assert_eq!(
        boards[0].inlet_chip_temperature.unwrap().as_celsius(),
        180.0
    );
    assert_eq!(
        boards[0].outlet_chip_temperature.unwrap().as_celsius(),
        200.0
    );
    assert!(boards[0].inlet_fluid_temperature.is_none());
    assert!(boards[1].board_temperature.is_none());
    assert!(boards[1].outlet_chip_temperature.is_none());
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
    let value = json!({"chain": [{"index": 0, "temp_pcb": [0, -10, 255, "NaN"], "temp_chip": [0, "infinity"]}]});
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
    let value = json!({"MHS 5s": 16000, "rate_unit": "MH", "chain_acn1": 180, "chain_rate1": 16, "CHAIN AVG HASHRATE1": "16000 MH/s"});
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
fn legacy_board_rates_honor_declared_units_without_explicit_board_units() {
    for (unit, algorithm, raw, expected) in [
        ("MH", HashAlgorithm::Scrypt, 6700.0, Some(6.7)),
        ("TH", HashAlgorithm::SHA256, 2.0, Some(2.0)),
        ("Sol/s", HashAlgorithm::SHA256, 200.0, None),
    ] {
        let boards = hashboards(
            &json!({"rate_unit": unit, "chain_acn1": 10, "chain_rate1": raw}),
            "Unknown",
            algorithm,
            &no_assumed_hardware(),
        );
        assert_eq!(boards[0].hashrate.as_ref().map(|rate| rate.value), expected);
    }
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
fn reported_power_is_optional_zero_is_valid_and_conflicts_are_rejected() {
    assert_eq!(
        wattage(&row(S23_HYDRO_STANDARD_SYNTHETIC))
            .unwrap()
            .as_watts(),
        5000.0
    );
    assert_eq!(wattage(&json!({"power": 0})).unwrap().as_watts(), 0.0);
    assert_eq!(
        wattage_source(&json!({"power": 0})).as_deref(),
        Some("antminer.stats:power")
    );
    assert_eq!(
        wattage_source(&json!({"modern": {"watt": 0}})).as_deref(),
        Some("antminer.rpc.stats(new_api=true):watt")
    );
    assert_eq!(
        wattage_source(&json!({"legacy": {"chain_power": "5000 W"}})).as_deref(),
        Some("antminer.rpc.stats:chain_power")
    );
    assert!(wattage(&json!({"power": 5000, "watt": 3500})).is_none());
    assert!(wattage_source(&json!({"power": 5000, "watt": 3500})).is_none());
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

#[tokio::test]
async fn captured_stock_hydro_web_fallback_preserves_actual_telemetry_for_both_backends() {
    for (model, stats, summary, chips, rate, elapsed, watts, frequency, pcb, chip, fan_count) in [
        (
            AntMinerModel::S21XPHydro,
            S21_XP_HYDRO_WEB_STATS_CAPTURED,
            S21_XP_HYDRO_WEB_SUMMARY_CAPTURED,
            160,
            483.97925,
            602533,
            0.0,
            495.0,
            [52.0, 51.0, 51.0],
            [63.0, 63.0, 63.0],
            4,
        ),
        (
            AntMinerModel::S21jXPHydro,
            S21J_XP_HYDRO_WEB_STATS_CAPTURED,
            S21J_XP_HYDRO_WEB_SUMMARY_CAPTURED,
            42,
            503.57633,
            602134,
            6124.0,
            581.0,
            [50.0, 50.0, 50.0],
            [59.0, 59.0, 60.0],
            0,
        ),
    ] {
        let raw = row(stats);
        for miner in [
            Box::new(AntMinerV2020::new(
                "127.0.0.1".parse().unwrap(),
                model.clone(),
            )) as Box<dyn Miner>,
            Box::new(AntMinerV202307::new(
                "127.0.0.1".parse().unwrap(),
                model.clone(),
            )) as Box<dyn Miner>,
        ] {
            // No RPC response: the captured dashboard is the only available
            // route, as can occur with firewalled port 4028.
            let mock = MockAPIClient::new(HashMap::from([
                (
                    MinerCommand::WebAPI {
                        command: "stats",
                        parameters: None,
                    },
                    serde_json::from_str(stats).unwrap(),
                ),
                (
                    MinerCommand::WebAPI {
                        command: "summary",
                        parameters: None,
                    },
                    serde_json::from_str(summary).unwrap(),
                ),
            ]));
            let mut collector = DataCollector::new_with_client(miner.as_ref(), &mock);
            let data = miner.parse_data(collector.collect_all().await);
            assert!(
                (data.hashrate.unwrap().value - rate).abs() < 1e-8,
                "{model}"
            );
            assert!(data.is_mining, "{model}");
            assert_eq!(data.uptime.unwrap().as_secs(), elapsed, "{model}");
            assert_eq!(data.wattage.unwrap().as_watts(), watts, "{model}");
            assert_eq!(
                data.wattage_source.as_deref(),
                Some("antminer.cgi.stats:watt")
            );
            assert!(data.wattage_is_estimated.is_none());
            assert_eq!(data.hashboards.len(), 3, "{model}");
            assert_eq!(data.fans.len(), fan_count, "{model}");
            assert!(data.fans.iter().all(|fan| fan.rpm.unwrap().as_rpm() == 0.0));
            assert!(
                data.messages.is_empty(),
                "unused hydro fan status is not an error: {model}"
            );
            for (index, board) in data.hashboards.iter().enumerate() {
                assert_eq!(board.position, index as u8);
                assert_eq!(board.expected_chips, Some(chips));
                assert_eq!(board.working_chips, Some(chips));
                assert_eq!(
                    board.chips.len(),
                    chips as usize,
                    "display padding must not add physical chips"
                );
                assert!(board.chips.iter().all(|chip| chip.working == Some(true)));
                assert_eq!(board.board_temperature.unwrap().as_celsius(), pcb[index]);
                assert_eq!(
                    board.outlet_chip_temperature.unwrap().as_celsius(),
                    chip[index]
                );
                assert_eq!(board.frequency.unwrap().as_megahertz(), frequency);
                assert_eq!(board.serial_number.as_deref(), Some("REDACTED-SERIAL"));
                assert_eq!(board.active, Some(true));
                assert!(board.inlet_fluid_temperature.is_none());
                assert!(board.outlet_fluid_temperature.is_none());
                assert!(
                    (board.expected_hashrate.as_ref().unwrap().value
                        - raw["chain"][index]["rate_ideal"].as_f64().unwrap() / 1000.0)
                        .abs()
                        < 1e-8
                );
                assert!(
                    (board.hashrate.as_ref().unwrap().value
                        - raw["chain"][index]["rate_real"].as_f64().unwrap() / 1000.0)
                        .abs()
                        < 1e-8
                );
            }
        }
    }
}

#[test]
fn captured_legacy_hydro_chip_status_padding_is_not_extra_hardware() {
    let model = AntMinerModel::S21jXPHydro;
    let raw = row(S21J_XP_HYDRO_RPC_STATS_CAPTURED);
    let boards = hashboards(
        &raw,
        &model.to_string(),
        HashAlgorithm::SHA256,
        &MinerHardware::from(model),
    );
    assert_eq!(boards.len(), 3);
    assert!(fans(&raw).is_empty());
    for board in boards {
        assert_eq!(board.expected_chips, Some(42));
        assert_eq!(board.working_chips, Some(42));
        assert_eq!(board.chips.len(), 42);
    }
}

#[test]
fn captured_response_mutations_keep_real_failures_and_zero_chip_counts_visible() {
    let mut captured = row(S21J_XP_HYDRO_WEB_STATS_CAPTURED);
    captured["chain"][0]["asic_num"] = json!(0);
    captured["chain"][0]["rate_real"] = json!(0);
    captured["chain"][0]["asic"] = json!(format!("x{}--", "o".repeat(41)));
    let hardware = MinerHardware::from(AntMinerModel::S21jXPHydro);
    let boards = hashboards(&captured, "S21jXPHydro", HashAlgorithm::SHA256, &hardware);
    assert_eq!(boards[0].working_chips, Some(0));
    assert_eq!(boards[0].active, Some(false));
    assert_eq!(boards[0].chips.len(), 42);
    assert_eq!(boards[0].chips[0].working, Some(false));
    assert_eq!(boards[0].chips[1].working, Some(true));
    captured["elapsed"] = json!("NaN");
    assert!(uptime(&captured).is_none());
    let mut summary: Value = serde_json::from_str(S21J_XP_HYDRO_WEB_SUMMARY_CAPTURED).unwrap();
    summary["SUMMARY"][0]["status"][0] =
        json!({"status": "W", "code": 12, "msg": "Reduced hashrate"});
    let errors = messages(&summary);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, 12);
    assert_eq!(errors[0].timestamp, 1791401146);
    assert_eq!(
        errors[0].severity,
        asic_rs_core::data::message::MessageSeverity::Warning
    );
}

#[test]
fn solution_units_are_explicit_and_algorithm_specific() {
    for unit in ["Ksol/s", "KSol/s", "KSols/s", "KS/s"] {
        let raw = json!({"rate_5s": 891.28, "rate_unit": unit});
        let rate = hashrate(&raw, HashAlgorithm::Equihash).unwrap();
        assert_eq!(rate.unit, HashRateUnit::KiloHash);
        assert_eq!(rate.value, 891.28);
        assert!(hashrate(&raw, HashAlgorithm::SHA256).is_none());
        assert!(hashrate(&raw, HashAlgorithm::KHeavyHash).is_none());
    }
    let raw = json!({"rate_5s": 380090.19, "rate_unit": "Sol/s"});
    assert!((hashrate(&raw, HashAlgorithm::Equihash).unwrap().value - 380.09019).abs() < 1e-8);
    assert!(hashrate(&json!({"GHS 5s": 380090.19}), HashAlgorithm::Equihash).is_none());
    assert_eq!(
        hashrate(&json!({"Ksol/s 5s": 891.28}), HashAlgorithm::Equihash)
            .unwrap()
            .value,
        891.28
    );
    assert_eq!(
        hashrate(&json!({"GHS 5s": 380090.19}), HashAlgorithm::SHA256)
            .unwrap()
            .value,
        380.09019
    );
}

#[tokio::test]
async fn captured_z15_restores_proven_units_partial_chips_and_sensor_channels() {
    // Captured Z15 9.0.0.5 dashboard: KSol/S aggregate uses GHS 5s / 1000;
    // KSol/S board column displays chain_rateN directly. Its PCB column uses
    // tempN, chip column temp2_N, and fans display only nonzero channels.
    let response = super::rpc_response::parse_response(Z15_MALFORMED_STATS_CAPTURED).unwrap();
    let summary: Value = serde_json::from_str(Z15_SUMMARY_CAPTURED).unwrap();
    let context = json!({"legacy_header": response["STATS"][0], "summary": summary["SUMMARY"][0]});
    assert!(
        (hashrate_for_model(&context, HashAlgorithm::Equihash, Some("Z15"))
            .unwrap()
            .value
            - 380.09019)
            .abs()
            < 1e-8
    );
    assert!(hashrate_for_model(&context, HashAlgorithm::Equihash, Some("Z15Pro")).is_none());
    let mut changed = context.clone();
    changed["legacy_header"]["Miner"] = json!("unknown");
    assert!(hashrate_for_model(&changed, HashAlgorithm::Equihash, Some("Z15")).is_none());
    for miner in [
        Box::new(AntMinerV2020::new(
            "127.0.0.1".parse().unwrap(),
            AntMinerModel::Z15,
        )) as Box<dyn Miner>,
        Box::new(AntMinerV202307::new(
            "127.0.0.1".parse().unwrap(),
            AntMinerModel::Z15,
        )),
    ] {
        let mock = MockAPIClient::new(HashMap::from([(
            MinerCommand::RPC {
                command: "stats",
                parameters: None,
            },
            response.clone(),
        )]));
        let mut collector = DataCollector::new_with_client(miner.as_ref(), &mock);
        let data = miner.parse_data(collector.collect_all().await);
        let rate = data.hashrate.unwrap();
        assert_eq!(rate.algo, HashAlgorithm::Equihash);
        assert_eq!(rate.unit, HashRateUnit::KiloHash);
        assert!((rate.value - 394.50432).abs() < 1e-8);
        assert!(data.is_mining);
        assert_eq!(data.uptime.unwrap().as_secs(), 844088);
        assert_eq!(data.hashboards.len(), 3);
        for (index, board) in data.hashboards.iter().enumerate() {
            assert_eq!(board.position, index as u8);
            assert_eq!(board.expected_chips, Some(3));
            assert_eq!(board.working_chips, Some([3, 3, 2][index]));
            assert_eq!(
                board.frequency.unwrap().as_megahertz(),
                [800.0, 800.0, 815.0][index]
            );
            assert_eq!(board.chips.len(), [3, 3, 2][index] as usize);
            assert_eq!(
                board.hashrate.as_ref().unwrap().value,
                [135.0, 155.97, 103.54][index]
            );
            assert_eq!(board.board_temperature.unwrap().as_celsius(), 51.0);
            assert_eq!(
                board.outlet_chip_temperature.unwrap().as_celsius(),
                [80.0, 77.0, 73.0][index]
            );
        }
        assert_eq!(data.fans.len(), 2);
        assert_eq!(
            data.fans.iter().map(|fan| fan.position).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert!(
            data.fans
                .iter()
                .all(|fan| fan.rpm.unwrap().as_rpm() == 4320.0)
        );
        assert!(data.wattage.is_none());
        assert!(data.wattage_source.is_none());
    }
}

#[tokio::test]
async fn captured_z15pro_uses_explicit_solution_units_and_physical_slots() {
    for miner in [
        Box::new(AntMinerV2020::new(
            "127.0.0.1".parse().unwrap(),
            AntMinerModel::Z15Pro,
        )) as Box<dyn Miner>,
        Box::new(AntMinerV202307::new(
            "127.0.0.1".parse().unwrap(),
            AntMinerModel::Z15Pro,
        )),
    ] {
        let mock = MockAPIClient::new(HashMap::from([(
            MinerCommand::RPC {
                command: "stats",
                parameters: Some(json!({"new_api": true})),
            },
            serde_json::from_str(Z15PRO_MODERN_STATS_CAPTURED).unwrap(),
        )]));
        let mut collector = DataCollector::new_with_client(miner.as_ref(), &mock);
        let data = miner.parse_data(collector.collect_all().await);
        assert_eq!(data.hashrate.unwrap().value, 891.28);
        assert_eq!(data.expected_hashrate.unwrap().value, 874.11);
        assert!(data.is_mining);
        assert_eq!(
            data.hashboards
                .iter()
                .map(|board| board.position)
                .collect::<Vec<_>>(),
            vec![0, 1, 3]
        );
        for (index, board) in data.hashboards.iter().enumerate() {
            assert_eq!(board.working_chips, Some(6));
            assert_eq!(
                board.hashrate.as_ref().unwrap().unit,
                HashRateUnit::KiloHash
            );
            assert_eq!(
                board.hashrate.as_ref().unwrap().value,
                [285.88, 290.54, 286.31][index]
            );
            assert_eq!(board.expected_chips, if index < 2 { Some(6) } else { None });
        }
    }
    let summary: Value = serde_json::from_str(Z15PRO_SUMMARY_CAPTURED).unwrap();
    let rate = hashrate(&summary["SUMMARY"][0], HashAlgorithm::Equihash).unwrap();
    assert_eq!(rate.value, 891.29);
    let boards = hashboards(
        &row(Z15PRO_LEGACY_STATS_CAPTURED),
        "Z15Pro",
        HashAlgorithm::Equihash,
        &MinerHardware::from(AntMinerModel::Z15Pro),
    );
    assert_eq!(
        boards
            .iter()
            .map(|board| board.hashrate.as_ref().unwrap().value)
            .collect::<Vec<_>>(),
        vec![289.94, 290.84, 238.95]
    );
}

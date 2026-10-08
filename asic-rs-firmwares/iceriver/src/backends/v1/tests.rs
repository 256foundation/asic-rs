use std::sync::atomic::{AtomicUsize, Ordering};

use asic_rs_core::{data::device::HashAlgorithm, test::api::MockAPIClient};
use asic_rs_makes_iceriver::models::IceRiverModel;
use serde_json::json;

use super::*;

fn miner() -> IceRiverV1 {
    IceRiverV1::with_auth(
        "127.0.0.1".parse().unwrap(),
        IceRiverModel::AL3,
        IceRiverV1::default_auth(),
    )
}

fn response() -> Value {
    serde_json::from_str(include_str!("fixtures/al3-userpanel.json")).unwrap()
}

async fn collect(miner: &IceRiverV1, response: Value) -> HashMap<DataField, Value> {
    let mock = MockAPIClient::new(HashMap::from([(USERPANEL, response)]));
    DataCollector::new_with_client(miner, &mock)
        .collect_all()
        .await
}

#[tokio::test]
async fn source_fixture_preserves_hardware_units_sensors_and_zero_fan() {
    let miner = miner();
    let data = miner.parse_data(collect(&miner, response()).await);
    assert_eq!(data.expected_hashboards, Some(3));
    assert_eq!(data.expected_chips, None);
    assert_eq!(data.total_chips, Some(53));
    assert_eq!(data.expected_fans, Some(4));
    assert_eq!(data.hashboards.len(), 3);
    assert_eq!(data.hashboards[0].expected_chips, None);
    assert_eq!(data.hashboards[0].working_chips, Some(18));
    assert_eq!(
        data.hashboards[0].board_temperature.unwrap().as_celsius(),
        60.0
    );
    assert_eq!(
        data.hashboards[0]
            .outlet_chip_temperature
            .unwrap()
            .as_celsius(),
        80.0
    );
    assert!(data.hashboards[0].inlet_chip_temperature.is_none());
    assert!(data.hashboards[0].chips.is_empty());
    assert_eq!(
        data.hashboards[0].hashrate.as_ref().unwrap().unit,
        HashRateUnit::GigaHash
    );
    assert_eq!(data.hashboards[0].hashrate.as_ref().unwrap().value, 5000.0);
    assert_eq!(data.hashboards[0].active, Some(true));
    assert!(data.hashboards[0].tuned.is_none());
    let hashrate = data.hashrate.unwrap();
    assert_eq!(hashrate.unit, HashRateUnit::TeraHash);
    assert_eq!(hashrate.value, 15.0);
    assert_eq!(data.fans.len(), 3);
    assert_eq!(data.fans[2].position, 2);
    assert_eq!(data.fans[2].rpm.unwrap().as_rpm(), 0.0);
    assert_eq!(data.uptime, Some(Duration::from_secs(93784)));
    assert_eq!(data.light_flashing, Some(false));
    assert!(data.is_mining);
    assert!(data.wattage.is_none());
    assert!(data.efficiency.is_none());
    assert!(data.expected_hashrate.is_none());
    assert!(data.operating_state.is_none());
    assert!(data.mac.is_none());
    assert!(data.hostname.is_none());
}

#[tokio::test]
async fn accepts_pyasic_recording_wrapper_and_uses_one_read_for_all_fields() {
    struct CountingClient {
        count: AtomicUsize,
    }
    #[async_trait]
    impl APIClient for CountingClient {
        async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
            assert_eq!(command, &USERPANEL);
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(json!({"userpanel": response()}))
        }
    }
    let miner = miner();
    let client = CountingClient {
        count: AtomicUsize::new(0),
    };
    let data = DataCollector::new_with_client(&miner, &client)
        .collect_all()
        .await;
    assert_eq!(client.count.load(Ordering::SeqCst), 1);
    assert_eq!(miner.parse_hashrate(&data).unwrap().value, 15.0);
}

#[tokio::test]
async fn live_al3_integral_float_counts_preserve_observations_and_static_metadata_separately() {
    let miner = miner();
    let response =
        serde_json::from_str(include_str!("fixtures/al3-10306-live-float-counts.json")).unwrap();
    let data = miner.parse_data(collect(&miner, response).await);
    assert_eq!(data.hashboards.len(), 3);
    assert_eq!(data.expected_chips, None);
    assert_eq!(data.total_chips, Some(54));
    for (position, board) in data.hashboards.iter().enumerate() {
        assert_eq!(board.position as usize, position);
        // The observed 18 does not establish expected chip capacity.
        assert_eq!(board.expected_chips, None);
        assert_eq!(board.working_chips, Some(18));
        assert_eq!(board.active, Some(true));
    }
    assert_eq!(
        data.hashboards[0].board_temperature.unwrap().as_celsius(),
        56.0
    );
    assert_eq!(
        data.hashboards[0]
            .outlet_chip_temperature
            .unwrap()
            .as_celsius(),
        65.0
    );
    let board_rate = data.hashboards[0].hashrate.as_ref().unwrap();
    assert_eq!(board_rate.unit, HashRateUnit::GigaHash);
    assert_eq!(board_rate.value, 5153.04);
    let rate = data.hashrate.unwrap();
    assert_eq!(rate.unit, HashRateUnit::GigaHash);
    assert_eq!(rate.value, 14762.0);
    let default_rate = rate.as_default_unit();
    assert_eq!(default_rate.unit, HashRateUnit::TeraHash);
    assert_eq!(default_rate.value, 14.762);
    assert_eq!(data.fans.len(), 4);
    assert!(data.is_mining);
}

#[test]
fn float_counts_reject_fractional_negative_nonfinite_and_overflow_values() {
    assert_eq!(unsigned(&json!(18.0)), Some(18));
    assert_eq!(unsigned(&json!(0.0)), Some(0));
    for value in [
        json!(1.5),
        json!(-1.0),
        json!(18446744073709551616.0),
        json!(true),
        json!("NaN"),
    ] {
        assert_eq!(unsigned(&value), None);
    }
}

#[tokio::test]
async fn missing_boards_and_fields_stay_unknown_and_mining_flag_is_explicit() {
    let miner = miner();
    let data = miner.parse_data(
        collect(
            &miner,
            json!({"data": {
                "unit": "T", "rtpow": "15T", "powstate": false,
                "boards": [{"no": 2, "chipnum": 0, "rtpow": "0G", "intmp": 0, "outtmp": null}]
            }}),
        )
        .await,
    );
    assert_eq!(data.hashboards.len(), 3);
    let missing = &data.hashboards[0];
    assert_eq!(missing.expected_chips, None);
    assert!(missing.working_chips.is_none());
    assert!(missing.hashrate.is_none());
    assert!(missing.active.is_none());
    assert!(missing.board_temperature.is_none());
    assert_eq!(data.hashboards[1].working_chips, Some(0));
    assert_eq!(data.hashboards[1].active, Some(false));
    assert!(data.hashboards[1].board_temperature.is_none());
    assert!(data.hashboards[1].outlet_chip_temperature.is_none());
    // One reported stopped board does not establish the other boards' counts.
    assert!(data.total_chips.is_none());
    assert!(!data.is_mining);
    assert!(data.operating_state.is_none());
    assert!(data.uptime.is_none());
    assert!(data.light_flashing.is_none());
    assert!(data.fans.is_empty());
    assert!(data.wattage.is_none());
}

#[tokio::test]
async fn conflicting_duplicate_board_rows_reject_board_snapshot_without_losing_device_rate() {
    for model in [IceRiverModel::AL3, IceRiverModel::Unknown("future".into())] {
        let miner = IceRiverV1::with_auth(
            "127.0.0.1".parse().unwrap(),
            model,
            IceRiverV1::default_auth(),
        );
        for reverse in [false, true] {
            let mut response = response();
            let boards = response["data"]["boards"].as_array_mut().unwrap();
            // Integer and float identifiers name the same board. This row
            // conflicts with the fixture's hot, hashing first board.
            boards.push(json!({
                "no": 1.0, "intmp": 20, "outtmp": 25, "rtpow": "0G", "chipnum": 0.0
            }));
            if reverse {
                boards.reverse();
            }
            let data = miner.parse_data(collect(&miner, response).await);
            assert!(data.hashboards.is_empty());
            assert!(data.total_chips.is_none());
            assert!(data.average_temperature.is_none());
            // The separately reported aggregate rate/state do not depend on
            // accepting the ambiguous board rows.
            let hashrate = data.hashrate.unwrap();
            assert_eq!(hashrate.value, 15.0);
            assert_eq!(hashrate.unit, HashRateUnit::TeraHash);
            assert!(data.is_mining);
        }
    }
}

#[tokio::test]
async fn malformed_units_positions_counts_and_uptime_do_not_create_telemetry() {
    let miner = miner();
    let data = miner.parse_data(collect(&miner, json!({"data": {
        "unit": "unsupported", "rtpow": "15unsupported", "runtime": "18446744073709551615:00:00:00",
        "powstate": 7, "locate": 7, "fans": [-1, "NaN"],
        "boards": [
            {"no": 0, "chipnum": 156},
            {"no": 9999, "chipnum": 156},
            {"no": 1, "chipnum": 70000, "rtpow": "NaNG", "intmp": "NaN"}
        ]
    }})).await);
    assert_eq!(data.hashboards.len(), 3);
    assert!(
        data.hashboards
            .iter()
            .all(|board| board.working_chips.is_none() && board.hashrate.is_none())
    );
    assert!(data.hashrate.is_none());
    assert!(data.total_chips.is_none());
    assert!(data.uptime.is_none());
    assert!(data.light_flashing.is_none());
    assert!(data.fans.is_empty());
    assert!(!data.is_mining);
}

#[tokio::test]
async fn firmware_with_unknown_model_keeps_unknown_expected_hardware() {
    let miner = IceRiverV1::with_auth(
        "127.0.0.1".parse().unwrap(),
        IceRiverModel::Unknown("future".to_owned()),
        IceRiverV1::default_auth(),
    );
    let data = miner.parse_data(
        collect(
            &miner,
            json!({"data": {
                "unit": "G", "rtpow": "30G", "boards": [{"no": 2, "chipnum": 12, "rtpow": "30G"}]
            }}),
        )
        .await,
    );
    assert_eq!(data.device_info.algo, HashAlgorithm::Unknown);
    assert!(data.expected_hashboards.is_none());
    assert!(data.expected_chips.is_none());
    assert_eq!(data.hashboards.len(), 1);
    assert_eq!(data.hashboards[0].position, 1);
    assert!(data.hashboards[0].expected_chips.is_none());
    assert_eq!(data.hashboards[0].working_chips, Some(12));
}

#[tokio::test]
async fn all_mutations_are_unsupported_and_rejected_without_device_io() {
    let mut miner = miner();
    assert!(!miner.supports_restart());
    assert!(!miner.supports_pause());
    assert!(!miner.supports_resume());
    assert!(!miner.supports_set_fault_light());
    assert!(!miner.supports_set_power_limit());
    assert!(!miner.supports_set_hashboards_enabled());
    assert!(!miner.supports_change_password());
    assert!(!miner.supports_pools_config());
    assert!(!miner.supports_upgrade_firmware());
    assert!(!miner.supports_factory_reset());
    assert!(miner.restart().await.is_err());
    assert!(miner.pause(None).await.is_err());
    assert!(miner.resume(None).await.is_err());
    assert!(miner.set_fault_light(true).await.is_err());
    assert!(miner.change_password("").await.is_err());
    assert!(miner.set_hashboards_enabled(&[0], false).await.is_err());
}

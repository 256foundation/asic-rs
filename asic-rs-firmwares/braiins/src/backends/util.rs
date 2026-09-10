use asic_rs_core::data::capabilities::{
    HashRateTuningCapabilities, PowerTuningCapabilities, TuningCapabilities,
};
use asic_rs_core::data::device::HashAlgorithm;
use asic_rs_core::data::hashrate::{HashRate, HashRateUnit};
use asic_rs_core::data::miner::TuningTarget;
use measurements::Power;
use serde_json::Value;

/// Build [`TuningCapabilities`] from a BOS GraphQL `powerTarget` metadata object
/// (`{ default, min, max }`, all in watts). Older GraphQL BraiinsOS backends only
/// expose the power-target envelope this way.
pub(crate) fn power_target_capabilities(power_target: &Value) -> TuningCapabilities {
    let watts = |key: &str| {
        power_target
            .get(key)
            .and_then(|v| v.as_i64())
            .map(|w| TuningTarget::from_watts(w as f64))
    };
    TuningCapabilities {
        power: Some(PowerTuningCapabilities {
            default: watts("default"),
            minimum: watts("min"),
            maximum: watts("max"),
        }),
        ..Default::default()
    }
}

/// Parse a BOS version string (e.g. `bos.info.version.full`) into semver.
///
/// BOS versions are CalVer-ish (`26.04`, `2026-04-1`): take the last dotted
/// segment, strip leading zeros per component, and pad to `major.minor.patch`.
pub(crate) fn parse_bos_version(full: &str) -> Option<semver::Version> {
    let version_str = full.split('-').rev().find(|s| s.contains('.'))?;
    let normalized = version_str
        .split('.')
        .map(|part| part.trim_start_matches('0').to_string())
        .map(|part| {
            if part.is_empty() {
                "0".to_string()
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(".");
    let padded = match version_str.split('.').count() {
        2 => format!("{normalized}.0"),
        _ => normalized,
    };
    semver::Version::parse(&padded).ok()
}

/// Build [`TuningCapabilities`] from a BOS+ REST `tuner_constraints` object
/// (`power_target` as `PowerConstraints` in watts, `hashrate_target` as
/// `HashrateConstraints` in TH/s). Used by the REST BraiinsOS backends
/// (25.07 and newer).
pub(crate) fn tuner_constraints_capabilities(
    tuner: &Value,
    algo: HashAlgorithm,
) -> TuningCapabilities {
    let power = tuner.get("power_target").map(|target| {
        let watts = |key: &str| {
            target
                .pointer(&format!("/{key}/watt"))
                .and_then(|v| v.as_i64())
                .map(|w| TuningTarget::from_watts(w as f64))
        };
        PowerTuningCapabilities {
            default: watts("default"),
            minimum: watts("min"),
            maximum: watts("max"),
        }
    });

    let hashrate = tuner.get("hashrate_target").map(|target| {
        let ths = |key: &str| {
            target
                .pointer(&format!("/{key}/terahash_per_second"))
                .and_then(|v| v.as_f64())
                .map(|value| {
                    TuningTarget::HashRate(HashRate {
                        value,
                        unit: HashRateUnit::TeraHash,
                        algo,
                    })
                })
        };
        HashRateTuningCapabilities {
            default: ths("default"),
            minimum: ths("min"),
            maximum: ths("max"),
        }
    });

    TuningCapabilities {
        power,
        hashrate,
        presets: None,
    }
}

pub(crate) fn parse_configured_tuning_target(
    value: &Value,
    algo: HashAlgorithm,
) -> Option<TuningTarget> {
    parse_tagged_tuning_target(value, "configured", algo)
}

pub(crate) fn parse_scaled_tuning_target(
    value: &Value,
    algo: HashAlgorithm,
) -> Option<TuningTarget> {
    parse_tagged_tuning_target(value, "scaled", algo)
}

fn parse_tagged_tuning_target(
    value: &Value,
    prefix: &str,
    algo: HashAlgorithm,
) -> Option<TuningTarget> {
    let power_key = format!("{prefix}_power");
    let hashrate_key = format!("{prefix}_hashrate");

    let power = value
        .get(&power_key)
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .map(Power::from_watts)
        .map(TuningTarget::Power);

    let hashrate = value
        .get(&hashrate_key)
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .map(|value| {
            TuningTarget::HashRate(HashRate {
                value,
                unit: HashRateUnit::TeraHash,
                algo,
            })
        });

    match (
        value.get("mode").and_then(Value::as_str),
        value.get("mode").and_then(Value::as_i64),
    ) {
        (Some("HASHRATE_TARGET"), _) | (_, Some(2)) => hashrate.or(power),
        (Some("POWER_TARGET"), _) | (_, Some(1)) => power.or(hashrate),
        _ => power.or(hashrate),
    }
}

#[cfg(test)]
mod operating_state_tests {
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
    use asic_rs_makes_antminer::models::AntMinerModel;
    use serde_json::json;

    use super::*;
    use crate::backends::{v25_07::BraiinsV2507, v26_04::BraiinsV2604};

    #[tokio::test]
    async fn rest_status_codes_reach_the_snapshot_for_both_backends() -> anyhow::Result<()> {
        let ip = IpAddr::from([127, 0, 0, 1]);
        let miners: [Box<dyn Miner>; 2] = [
            Box::new(BraiinsV2507::new(ip, AntMinerModel::S21Pro)),
            Box::new(BraiinsV2604::new(ip, AntMinerModel::S21Pro)),
        ];
        let cases = [
            (1, OperatingState::Stopped {}, false),
            (2, OperatingState::Mining {}, true),
            (3, OperatingState::Paused {}, false),
            (4, OperatingState::Suspended {}, false),
            (5, OperatingState::Restricted {}, false),
            (99, OperatingState::Unknown { raw: "99".into() }, false),
        ];
        for miner in &miners {
            for (code, expected, is_mining) in &cases {
                let mut details: Value =
                    serde_json::from_str(crate::test::json::v26_04::WEB_MINER_DETAILS_COMMAND)?;
                details["status"] = json!(code);
                let client = MockAPIClient::new(HashMap::from([(
                    MinerCommand::WebAPI {
                        command: "miner/details",
                        parameters: None,
                    },
                    details,
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
        Ok(())
    }
}

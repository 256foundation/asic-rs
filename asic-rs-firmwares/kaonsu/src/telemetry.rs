// SPDX-License-Identifier: Apache-2.0
use asic_rs_core::data::{
    board::BoardData,
    device::HashAlgorithm,
    fan::FanData,
    hashrate::{HashRate, HashRateUnit},
    message::{MessageSeverity, MinerMessage},
    operating_state::OperatingState,
};
use measurements::{AngularVelocity, Power, Temperature};
use serde_json::Value;
use std::{collections::BTreeSet, str::FromStr, time::Duration};

pub(crate) fn payload<'a>(value: &'a Value, _key: Option<&str>) -> Option<&'a Value> {
    let value = value
        .get("data")
        .filter(|data| data.is_object())
        .unwrap_or(value);
    value.is_object().then_some(value)
}
fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.trim().parse().ok())
        .filter(|value| value.is_finite())
}
fn unsigned(value: &Value) -> Option<u16> {
    number(value)
        .filter(|value| *value >= 0.0 && value.fract() == 0.0 && *value <= u16::MAX as f64)
        .map(|value| value as u16)
}
fn rate(
    value: &Value,
    unit: Option<&Value>,
    default_unit: HashRateUnit,
    algo: HashAlgorithm,
) -> Option<HashRate> {
    let unit = match unit {
        Some(value) => HashRateUnit::from_str(value.as_str()?).ok()?,
        None => default_unit,
    };
    let rate = HashRate {
        value: number(value).filter(|value| *value >= 0.0)?,
        unit,
        algo,
    };
    let rate = if algo == HashAlgorithm::Unknown {
        rate
    } else {
        rate.as_default_unit()
    };
    rate.value.is_finite().then_some(rate)
}
pub(crate) fn hashrate(value: &Value, algo: HashAlgorithm) -> Option<HashRate> {
    let value = payload(value, None)?;
    for key in [
        "hashrate_realtime_10m",
        "hashrate_realtime",
        "hashrate_average",
    ] {
        if let Some(raw) = value.get(key) {
            // Do not replace a stopped or invalid current sample with an older average.
            return rate(
                raw,
                value
                    .get(format!("{key}_unit"))
                    .or_else(|| value.get("hashrate_unit")),
                HashRateUnit::TeraHash,
                algo,
            );
        }
    }
    None
}
pub(crate) fn expected_hashrate(value: &Value, algo: HashAlgorithm) -> Option<HashRate> {
    let value = payload(value, None)?;
    for key in ["hashrate_stock", "hashrate_sales", "hashrate_ideal"] {
        if let Some(raw) = value.get(key) {
            // The ideal field uses GH/s even on builds reporting aggregates in TH/s.
            let unit = value.get(format!("{key}_unit")).or_else(|| {
                if key == "hashrate_ideal" {
                    None
                } else {
                    value.get("hashrate_unit")
                }
            });
            let default = if key == "hashrate_ideal" {
                HashRateUnit::GigaHash
            } else {
                HashRateUnit::TeraHash
            };
            return rate(raw, unit, default, algo);
        }
    }
    None
}
pub(crate) fn power(value: &Value) -> Option<Power> {
    number(payload(value, None)?.get("power_consumption_estimated")?)
        .filter(|value| *value >= 0.0)
        .map(Power::from_watts)
}
pub(crate) fn uptime(value: &Value) -> Option<Duration> {
    let seconds = number(payload(value, None)?.get("elapsed")?)?;
    Duration::try_from_secs_f64(seconds).ok()
}
pub(crate) fn expected_boards(value: &Value) -> Option<u8> {
    let count = unsigned(payload(value, None)?.get("hashboard_num_ideal")?)?;
    u8::try_from(count).ok().filter(|count| *count > 0)
}
pub(crate) fn reported_max_temperature(value: &Value) -> Option<Temperature> {
    number(payload(value, None)?.get("temperature_max")?)
        .filter(|value| *value > 0.0 && *value < 200.0)
        .map(Temperature::from_celsius)
}
pub(crate) fn temperatures(value: Option<&Value>) -> Vec<f64> {
    let values = match value {
        Some(Value::Array(values)) => values.iter().collect::<Vec<_>>(),
        Some(Value::Object(values)) => values.values().collect(),
        Some(value) => vec![value],
        None => vec![],
    };
    values
        .into_iter()
        .filter_map(number)
        .filter(|value| *value > 0.0 && *value < 200.0)
        .collect()
}
fn maximum(values: &[f64]) -> Option<Temperature> {
    values
        .iter()
        .copied()
        .reduce(f64::max)
        .map(Temperature::from_celsius)
}
fn minimum(values: &[f64]) -> Option<Temperature> {
    values
        .iter()
        .copied()
        .reduce(f64::min)
        .map(Temperature::from_celsius)
}

#[cfg(test)]
pub(crate) fn boards(value: &Value, algo: HashAlgorithm) -> Vec<BoardData> {
    boards_with_unit(value, algo, None)
}
pub(crate) fn boards_with_unit(
    value: &Value,
    algo: HashAlgorithm,
    aggregate_unit: Option<&Value>,
) -> Vec<BoardData> {
    let Some(value) = payload(value, None) else {
        return vec![];
    };
    let Some(rows) = ["hashboards", "boards", "chains"]
        .into_iter()
        .find_map(|key| value.get(key))
        .and_then(Value::as_array)
    else {
        return vec![];
    };
    if rows.iter().any(|row| !row.is_object()) {
        return vec![];
    }
    let raw_ids = rows
        .iter()
        .map(|row| {
            ["index", "id", "slot", "chain_id"]
                .into_iter()
                .find_map(|key| row.get(key).map(|value| (key, value)))
        })
        .collect::<Vec<_>>();
    let id_field = raw_ids.iter().find_map(|id| id.map(|(key, _)| key));
    let explicit_ids = id_field.is_some();
    if raw_ids
        .iter()
        .flatten()
        .any(|(key, _)| Some(*key) != id_field)
    {
        return vec![];
    }
    let ids = raw_ids
        .into_iter()
        .map(|id| id.and_then(|(_, value)| unsigned(value)))
        .collect::<Vec<_>>();
    let valid_ids = ids.iter().filter_map(|id| *id).collect::<BTreeSet<_>>();
    if explicit_ids && (ids.iter().any(Option::is_none) || valid_ids.len() != rows.len()) {
        return vec![];
    }
    let indexed = id_field == Some("index");
    let one_based = explicit_ids
        && !indexed
        && expected_boards(value).is_some_and(|count| {
            rows.len() == usize::from(count) && valid_ids.iter().copied().eq(1..=u16::from(count))
        });
    if explicit_ids && !indexed && !one_based && !valid_ids.contains(&0) {
        return vec![];
    }
    let mut output = Vec::new();
    for (ordinal, row) in rows.iter().enumerate() {
        let position = if explicit_ids {
            ids[ordinal].and_then(|id| {
                if one_based {
                    id.checked_sub(1)
                } else {
                    Some(id)
                }
            })
        } else {
            u16::try_from(ordinal).ok()
        };
        let Some(position) = position.and_then(|position| u8::try_from(position).ok()) else {
            return vec![];
        };
        let mut board = BoardData::new(position, row.get("asic_num_ideal").and_then(unsigned));
        board.working_chips = row.get("asic_num").and_then(unsigned);
        for key in ["hashrate_realtime_10m", "hashrate_average"] {
            if let Some(raw) = row.get(key) {
                let default = if key == "hashrate_average" {
                    HashRateUnit::GigaHash
                } else {
                    HashRateUnit::TeraHash
                };
                board.hashrate = rate(
                    raw,
                    row.get(format!("{key}_unit"))
                        .or_else(|| row.get("hashrate_unit"))
                        .or_else(|| value.get("hashrate_unit"))
                        // The firmware contract fixes board averages in GH/s.
                        // Its live 10-minute rate follows the aggregate unit.
                        .or_else(|| {
                            (key == "hashrate_realtime_10m")
                                .then_some(aggregate_unit)
                                .flatten()
                        }),
                    default,
                    algo,
                );
                break;
            }
        }
        let mut pcb = temperatures(row.get("temperature_pcb"));
        pcb.extend(temperatures(row.get("temperature_raw")));
        board.board_temperature = maximum(&pcb);
        let chips = temperatures(row.get("temperature_chip"));
        board.inlet_chip_temperature = minimum(&chips);
        board.outlet_chip_temperature = maximum(&chips);
        board.active = match row.get("present") {
            Some(Value::Bool(false)) => Some(false),
            Some(value) if !value.is_boolean() => None,
            _ if explicitly_stopped(row) => Some(false),
            _ => board.hashrate.as_ref().map(|rate| rate.value > 0.0),
        };
        output.push(board);
    }
    output.sort_by_key(|board| board.position);
    output
}
pub(crate) fn fans(value: &Value) -> Vec<FanData> {
    let Some(value) = payload(value, None) else {
        return vec![];
    };
    let Some(rows) = value
        .get("fans")
        .or_else(|| value.get("fan"))
        .and_then(Value::as_array)
    else {
        return vec![];
    };
    rows.iter()
        .enumerate()
        .filter_map(|(ordinal, row)| {
            let raw = if row.is_object() {
                ["current_speed", "rpm", "speed"]
                    .into_iter()
                    .find_map(|key| row.get(key))?
            } else {
                row
            };
            Some(FanData {
                position: i16::try_from(ordinal).ok()?,
                rpm: Some(AngularVelocity::from_rpm(
                    number(raw).filter(|value| *value >= 0.0)?,
                )),
            })
        })
        .collect()
}
fn descriptions(value: &Value) -> impl Iterator<Item = &str> {
    [
        "indicator_long",
        "status_long",
        "indicator",
        "description",
        "status",
        "work_mode",
        "work-mode",
    ]
    .into_iter()
    .filter_map(|key| value.get(key).and_then(Value::as_str))
}
pub(crate) fn is_mining(value: &Value, algo: HashAlgorithm) -> bool {
    let Some(value) = payload(value, None) else {
        return false;
    };
    if explicitly_stopped(value) {
        return false;
    }
    hashrate(value, algo).is_some_and(|rate| rate.value > 0.0)
}
fn explicitly_stopped(value: &Value) -> bool {
    let description = descriptions(value)
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    [
        "critical", "fatal", "failure", "failed", "sleep", "pause", "curtail", "stopped", "idle",
        "suspend",
    ]
    .iter()
    .any(|word| description.contains(word))
}
pub(crate) fn operating_state(value: &Value) -> Option<OperatingState> {
    let value = payload(value, None)?;
    OperatingState::from_label(value.get("status")?.as_str()?.trim())
}
pub(crate) fn messages(value: &Value) -> Vec<MinerMessage> {
    let Some(value) = payload(value, None) else {
        return vec![];
    };
    let context = descriptions(value)
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let severity = if ["critical", "fatal", "failure", "failed"]
        .iter()
        .any(|word| context.contains(word))
    {
        MessageSeverity::Error
    } else if context.contains("warning") {
        MessageSeverity::Warning
    } else {
        MessageSeverity::Info
    };
    descriptions(value)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| MinerMessage::new(0, 0, text.to_owned(), severity.clone()))
        .collect()
}

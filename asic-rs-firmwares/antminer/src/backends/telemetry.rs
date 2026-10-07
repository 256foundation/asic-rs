// Support Extension additions: conservative stock Bitmain telemetry parsing.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeSet, str::FromStr};

use asic_rs_core::data::{
    board::BoardData,
    collector::{DataExtractor, DataLocation, get_by_pointer},
    command::MinerCommand,
    device::{HashAlgorithm, MinerHardware},
    fan::FanData,
    hashrate::{HashRate, HashRateUnit},
};
use measurements::{AngularVelocity, Frequency, Power, Temperature};
use serde_json::{Value, json};

pub(super) fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.trim().parse().ok())
        .filter(|value| value.is_finite())
}

fn temperature(value: &Value) -> Option<f64> {
    number(value).filter(|value| *value > 0.0 && *value <= 150.0)
}

fn temperatures(value: Option<&Value>) -> Vec<f64> {
    match value {
        Some(Value::Array(values)) => values.iter().filter_map(temperature).collect(),
        Some(Value::String(values))
            if values.contains('-')
                && values.split('-').all(|value| {
                    !value.is_empty()
                        && value
                            .chars()
                            .all(|character| character.is_ascii_digit() || character == '.')
                        && value.parse::<f64>().is_ok()
                }) =>
        {
            values
                .split('-')
                .filter_map(|value| temperature(&Value::String(value.to_owned())))
                .collect()
        }
        Some(value) => temperature(value).into_iter().collect(),
        None => vec![],
    }
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

/// Select the unique telemetry aggregate instead of assuming STATS/1.
/// Header records can be absent on the new API. Multiple aggregates are
/// ambiguous and must not be silently combined into one miner.
pub(super) fn extract_stats<'a>(value: &'a Value, _key: Option<&str>) -> Option<&'a Value> {
    let rows = value.get("STATS")?.as_array()?;
    let mut aggregates = rows.iter().filter(|row| {
        row.as_object().is_some_and(|row| {
            row.keys().any(|key| {
                key == "chain"
                    || key == "rate_5s"
                    || key == "rate_avg"
                    || key == "total_rateideal"
                    || key == "rate_ideal"
                    || key == "chain_power"
                    || key == "power"
                    || key == "watt"
                    || key == "Power"
                    || key.starts_with("chain_rate")
                    || key.ends_with(" 5s")
                    || key.ends_with(" av")
            })
        })
    });
    let row = aggregates.next()?;
    aggregates.next().is_none().then_some(row)
}

pub(super) fn stats_locations() -> Vec<DataLocation> {
    [(None, "legacy"), (Some(json!({"new_api": true})), "modern")]
        .into_iter()
        .map(|(parameters, tag)| {
            (
                MinerCommand::RPC {
                    command: "stats",
                    parameters,
                },
                DataExtractor {
                    func: extract_stats,
                    key: None,
                    tag: Some(tag),
                },
            )
        })
        .collect()
}

pub(super) fn rate_locations() -> Vec<DataLocation> {
    let mut locations = stats_locations();
    locations.push((
        MinerCommand::RPC {
            command: "summary",
            parameters: None,
        },
        DataExtractor {
            func: get_by_pointer,
            key: Some("/SUMMARY/0"),
            tag: Some("summary"),
        },
    ));
    locations
}

fn rows(value: &Value) -> impl Iterator<Item = &Value> {
    [value.get("modern"), value.get("legacy"), Some(value)]
        .into_iter()
        .flatten()
}

fn rate_unit(value: Option<&Value>, algo: HashAlgorithm) -> Option<HashRateUnit> {
    match value {
        Some(value) => HashRateUnit::from_str(value.as_str()?).ok(),
        None if matches!(algo, HashAlgorithm::SHA256 | HashAlgorithm::KHeavyHash) => {
            Some(HashRateUnit::GigaHash)
        }
        // Known legacy rates identify their own unit; a modern rate needs
        // one explicitly for other algorithms.
        None => None,
    }
}

fn make_rate(value: &Value, unit: HashRateUnit, algo: HashAlgorithm) -> Option<HashRate> {
    let rate = HashRate {
        value: number(value).filter(|value| *value >= 0.0)?,
        unit,
        algo,
    }
    .as_default_unit();
    rate.value.is_finite().then_some(rate)
}

fn rate_window(
    row: &Value,
    window: &str,
    modern_key: &str,
    algo: HashAlgorithm,
) -> Option<HashRate> {
    let mut values = Vec::new();
    if let Some(value) = row.get(modern_key) {
        // An invalid explicit current sample must not fall through to a
        // rolling average that might still include an earlier mining period.
        values.push(make_rate(
            value,
            rate_unit(row.get("rate_unit"), algo)?,
            algo,
        )?);
    }
    for (key, value) in row.as_object()? {
        let Some((unit, suffix)) = key.split_once(' ') else {
            continue;
        };
        if suffix == window
            && let Ok(unit) = HashRateUnit::from_str(unit)
        {
            values.push(make_rate(value, unit, algo)?);
        }
    }
    let first = values.first()?.clone();
    values
        .iter()
        .all(|rate| (rate.value - first.value).abs() <= first.value.abs() * 0.001 + 1e-9)
        .then_some(first)
}

pub(super) fn current_rate(value: &Value, algo: HashAlgorithm) -> Option<HashRate> {
    for row in [
        value.get("modern"),
        value.get("legacy"),
        value.get("summary"),
        Some(value),
    ]
    .into_iter()
    .flatten()
    {
        if row.as_object().is_some_and(|row| {
            row.keys()
                .any(|key| key == "rate_5s" || key.ends_with(" 5s"))
        }) {
            return rate_window(row, "5s", "rate_5s", algo);
        }
    }
    None
}

pub(super) fn hashrate(value: &Value, algo: HashAlgorithm) -> Option<HashRate> {
    if let Some(rate) = current_rate(value, algo) {
        return Some(rate);
    }
    // If a current field was present but invalid/conflicting, no average can
    // safely repair it. Missing fields alone may fall back to an average.
    for row in [
        value.get("modern"),
        value.get("legacy"),
        value.get("summary"),
        Some(value),
    ]
    .into_iter()
    .flatten()
    {
        if row.as_object().is_some_and(|row| {
            row.keys()
                .any(|key| key == "rate_5s" || key.ends_with(" 5s"))
        }) {
            return None;
        }
    }
    for row in [
        value.get("modern"),
        value.get("legacy"),
        value.get("summary"),
        Some(value),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(rate) = rate_window(row, "av", "rate_avg", algo) {
            return Some(rate);
        }
    }
    None
}

pub(super) fn expected_hashrate(value: &Value, algo: HashAlgorithm) -> Option<HashRate> {
    let row = rows(value).find(|row| {
        ["rate_ideal", "total_rateideal", "hashrate"]
            .into_iter()
            .any(|key| row.get(key).is_some())
    })?;
    let (key, raw) = ["rate_ideal", "total_rateideal", "hashrate"]
        .into_iter()
        .find_map(|key| Some((key, row.get(key)?)))?;
    let unit = row.get("rate_unit").or_else(|| row.get("unit"));
    // Legacy total_rateideal uses GH/s when it omits rate_unit (captured L9/L11).
    let unit = match unit {
        Some(value) => HashRateUnit::from_str(value.as_str()?).ok()?,
        None if matches!(algo, HashAlgorithm::SHA256 | HashAlgorithm::KHeavyHash)
            || key == "total_rateideal" && algo == HashAlgorithm::Scrypt =>
        {
            HashRateUnit::GigaHash
        }
        None => return None,
    };
    make_rate(raw, unit, algo)
}

fn compact_model(model: &str) -> String {
    let model = model.to_ascii_uppercase();
    model
        .strip_prefix("ANTMINER")
        .unwrap_or(&model)
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '+')
        .collect()
}

fn is_s21_hydro(model: &str) -> bool {
    matches!(
        compact_model(model).as_str(),
        "S21HYD"
            | "S21HYDRO"
            | "S21XPHYD"
            | "S21XPHYDRO"
            | "S21JXPHYD"
            | "S21JXPHYDRO"
            | "S21EXPHYD"
            | "S21EXPHYDRO"
            | "S21+HYD"
            | "S21+HYDRO"
            | "S21PLUSHYDRO"
    )
}

fn separated_temperatures(chain: &Value, model: &str, board: &mut BoardData) {
    let pcb = chain.get("temp_pcb");
    let chips = chain.get("temp_chip");
    let pic = chain.get("temp_pic");
    let plus_hydro = matches!(
        compact_model(model).as_str(),
        "S21+HYD" | "S21+HYDRO" | "S21PLUSHYDRO"
    );
    if plus_hydro
        && let (Some(pcb), Some(pic)) =
            (pcb.and_then(Value::as_array), pic.and_then(Value::as_array))
        && pcb.len() == 4
        && pic.len() == 4
    {
        board.inlet_fluid_temperature = temperature(&pcb[0]).map(Temperature::from_celsius);
        board.outlet_fluid_temperature = temperature(&pcb[2]).map(Temperature::from_celsius);
        let values = [&pcb[1], &pcb[3], &pic[1], &pic[2], &pic[3]]
            .into_iter()
            .filter_map(temperature)
            .collect::<Vec<_>>();
        board.board_temperature = maximum(&values);
        let chip = temperature(&pic[0]).map(Temperature::from_celsius);
        board.inlet_chip_temperature = chip;
        board.outlet_chip_temperature = chip;
        return;
    }
    board.board_temperature = maximum(&temperatures(pcb));
    if is_s21_hydro(model)
        && let Some(chips) = chips.and_then(Value::as_array)
        && chips.len() == 4
        && chips[2..].iter().all(|value| number(value) == Some(0.0))
    {
        // Only this exact documented S21 hydro layout contains a coolant pair.
        // Other hydro products/array shapes remain ordinary chip telemetry.
        board.inlet_fluid_temperature = temperature(&chips[0]).map(Temperature::from_celsius);
        board.outlet_fluid_temperature = temperature(&chips[1]).map(Temperature::from_celsius);
    } else {
        let values = temperatures(chips);
        board.inlet_chip_temperature = minimum(&values);
        board.outlet_chip_temperature = maximum(&values);
    }
}

pub(super) fn hashboards(
    value: &Value,
    model: &str,
    algo: HashAlgorithm,
    hardware: &MinerHardware,
) -> Vec<BoardData> {
    let row = rows(value)
        .find(|row| {
            row.get("chain").is_some()
                || row.as_object().is_some_and(|row| {
                    row.keys().any(|key| {
                        key.starts_with("chain_rate")
                            || key.starts_with("chain_acn")
                            || key.starts_with("chain_acs")
                    })
                })
        })
        .unwrap_or(value);
    let mut boards = Vec::new();
    if let Some(chains) = row.get("chain").and_then(Value::as_array) {
        let mut positions = BTreeSet::new();
        for (ordinal, chain) in chains.iter().enumerate() {
            if !chain.is_object() {
                return vec![];
            }
            let position = match chain.get("index") {
                Some(value) => number(value)
                    .filter(|value| {
                        value.fract() == 0.0 && *value >= 0.0 && *value <= u8::MAX as f64
                    })
                    .map(|value| value as u8),
                None => u8::try_from(ordinal).ok(),
            };
            let Some(position) = position else {
                return vec![];
            };
            if !positions.insert(position) {
                return vec![];
            }
            let mut board = BoardData::new(position, hardware.chips_for_board(position as usize));
            board.hashrate = chain.get("rate_real").and_then(|value| {
                make_rate(
                    value,
                    rate_unit(
                        chain.get("rate_unit").or_else(|| row.get("rate_unit")),
                        algo,
                    )?,
                    algo,
                )
            });
            board.working_chips = chain
                .get("asic_num")
                .and_then(number)
                .filter(|value| value.fract() == 0.0 && *value >= 0.0 && *value <= u16::MAX as f64)
                .map(|value| value as u16);
            board.frequency = chain
                .get("frequency_mhz")
                .and_then(number)
                .filter(|value| *value > 0.0)
                .map(Frequency::from_megahertz);
            board.active = board.hashrate.as_ref().map(|rate| rate.value > 0.0);
            separated_temperatures(chain, model, &mut board);
            boards.push(board);
        }
    } else if let Some(row) = row.as_object() {
        let slots = row
            .keys()
            .filter_map(|key| {
                ["chain_rate", "chain_acn", "chain_acs"]
                    .into_iter()
                    .find_map(|prefix| key.strip_prefix(prefix)?.parse::<u16>().ok())
            })
            .collect::<BTreeSet<_>>();
        for slot in slots {
            if slot == 0 || slot > u8::MAX as u16 + 1 {
                continue;
            }
            let rate = row
                .get(&format!("chain_rate{slot}"))
                .and_then(|value| make_rate(value, HashRateUnit::GigaHash, algo));
            let chips = row
                .get(&format!("chain_acn{slot}"))
                .and_then(number)
                .filter(|value| value.fract() == 0.0 && *value >= 0.0 && *value <= u16::MAX as f64)
                .map(|value| value as u16);
            let state = row
                .get(&format!("chain_acs{slot}"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            if rate.as_ref().is_none_or(|rate| rate.value == 0.0)
                && chips.unwrap_or(0) == 0
                && state.is_empty()
            {
                continue;
            }
            let mut board = BoardData::new(
                (slot - 1) as u8,
                hardware.chips_for_board(slot as usize - 1),
            );
            board.hashrate = rate;
            board.working_chips = chips;
            board.active = board.hashrate.as_ref().map(|rate| rate.value > 0.0);
            let mut pcb = temperatures(row.get(&format!("temp_pcb{slot}")));
            if pcb.is_empty() {
                pcb = ["temp_out_pcb_", "temp_in_pcb_"]
                    .into_iter()
                    .flat_map(|prefix| temperatures(row.get(&format!("{prefix}{slot}"))))
                    .collect();
            }
            if pcb.is_empty() {
                pcb = temperatures(row.get(&format!("temp2_{slot}")));
            }
            board.board_temperature = maximum(&pcb);
            let mut chip = temperatures(row.get(&format!("temp_chip{slot}")));
            if chip.is_empty() {
                chip = ["temp_out_chip_", "temp_in_chip_"]
                    .into_iter()
                    .flat_map(|prefix| temperatures(row.get(&format!("{prefix}{slot}"))))
                    .collect();
            }
            if chip.is_empty() {
                chip = temperatures(row.get(&format!("temp{slot}")));
            }
            board.inlet_chip_temperature = minimum(&chip);
            board.outlet_chip_temperature = maximum(&chip);
            board.frequency = row
                .get(&format!("freq{slot}"))
                .and_then(number)
                .filter(|value| *value > 0.0)
                .map(Frequency::from_megahertz);
            boards.push(board);
        }
    }
    boards.sort_by_key(|board| board.position);
    boards
}

pub(super) fn fans(value: &Value) -> Vec<FanData> {
    let row = rows(value)
        .find(|row| {
            row.get("fan").is_some()
                || row.as_object().is_some_and(|row| {
                    row.keys().any(|key| {
                        key.strip_prefix("fan")
                            .is_some_and(|key| key.parse::<usize>().is_ok())
                    })
                })
        })
        .unwrap_or(value);
    let raw = if let Some(fans) = row.get("fan").and_then(Value::as_array) {
        fans.iter().enumerate().collect::<Vec<_>>()
    } else if let Some(row) = row.as_object() {
        let mut raw = row
            .iter()
            .filter_map(|(key, value)| {
                Some((
                    key.strip_prefix("fan")?
                        .parse::<usize>()
                        .ok()?
                        .checked_sub(1)?,
                    value,
                ))
            })
            .collect::<Vec<_>>();
        raw.sort_by_key(|(index, _)| *index);
        raw
    } else {
        return vec![];
    };
    raw.into_iter()
        .filter_map(|(index, value)| {
            Some(FanData {
                position: i16::try_from(index).ok()?,
                rpm: Some(AngularVelocity::from_rpm(
                    number(value).filter(|value| *value >= 0.0)?,
                )),
            })
        })
        .collect()
}

pub(super) fn is_mining(value: Option<&Value>, rate: Option<HashRate>) -> bool {
    let modes = match value {
        Some(value) if value.is_object() => ["work_mode", "miner_mode", "mode"]
            .into_iter()
            .filter_map(|key| value.get(key))
            .collect::<Vec<_>>(),
        Some(value) => vec![value],
        None => vec![],
    };
    let mut normal_mode = None;
    for raw in modes {
        let mode = raw
            .as_str()
            .map(|value| value.trim().to_ascii_lowercase())
            .or_else(|| number(raw).map(|value| value.to_string()));
        let Some(mode) = mode else {
            return false;
        };
        if ["1", "5", "stopped", "idle", "sleep"].contains(&mode.as_str()) {
            return false;
        }
        if !["0", "2", "3"].contains(&mode.as_str()) {
            return false;
        }
        if normal_mode
            .as_ref()
            .is_some_and(|previous| previous != &mode)
        {
            return false;
        }
        normal_mode = Some(mode);
    }
    // A normal config means an intention to run; it is not evidence of hashing.
    rate.is_some_and(|rate| rate.value > 0.0)
}

pub(super) fn wattage(value: &Value) -> Option<Power> {
    let row = rows(value).find(|row| {
        ["chain_power", "power", "Power", "watt"]
            .into_iter()
            .any(|key| row.get(key).is_some())
    })?;
    let mut candidates = Vec::new();
    if let Some(raw) = row.get("chain_power").and_then(Value::as_str)
        && let Some(raw) = raw
            .trim()
            .strip_suffix('W')
            .or_else(|| raw.trim().strip_suffix('w'))
        && let Some(watts) =
            number(&Value::String(raw.trim().to_owned())).filter(|value| *value >= 0.0)
    {
        candidates.push(watts);
    }
    for key in ["power", "Power", "watt"] {
        if let Some(value) = row.get(key).and_then(number).filter(|value| *value >= 0.0) {
            candidates.push(value);
        }
    }
    let first = *candidates.first()?;
    candidates
        .iter()
        .all(|value| (*value - first).abs() <= first * 0.01 + 1.0)
        .then_some(Power::from_watts(first))
}

pub(super) fn fluid_temperature(boards: &[BoardData], outlet: bool) -> Option<Temperature> {
    let values = boards
        .iter()
        .filter_map(|board| {
            if outlet {
                board.outlet_fluid_temperature
            } else {
                board.inlet_fluid_temperature
            }
        })
        .map(|value| value.as_celsius())
        .collect::<Vec<_>>();
    (!values.is_empty())
        .then(|| Temperature::from_celsius(values.iter().sum::<f64>() / values.len() as f64))
}

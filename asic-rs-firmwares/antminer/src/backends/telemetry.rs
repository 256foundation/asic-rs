// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeSet, str::FromStr, time::Duration};

use asic_rs_core::data::{
    board::{BoardData, ChipData},
    collector::{DataExtractor, DataLocation, get_by_pointer},
    command::MinerCommand,
    device::{HashAlgorithm, MinerHardware},
    fan::FanData,
    hashrate::{HashRate, HashRateUnit},
    message::{MessageSeverity, MinerMessage},
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
    number(value).filter(|value| *value > 0.0 && *value <= 200.0)
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
                    || key == "total_power"
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
    let mut locations = [(None, "legacy"), (Some(json!({"new_api": true})), "modern")]
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
        .collect::<Vec<_>>();
    // Keep the legacy product/firmware header for model-specific unit contracts.
    // It is collected from the same cached read-only stats response.
    locations.push((
        MinerCommand::RPC {
            command: "stats",
            parameters: None,
        },
        DataExtractor {
            func: get_by_pointer,
            key: Some("/STATS/0"),
            tag: Some("legacy_header"),
        },
    ));
    // The modern stock dashboard exposes this same aggregate over HTTP, even
    // when its cgminer RPC listener is unavailable (as used by native pyasic).
    locations.push((
        MinerCommand::WebAPI {
            command: "stats",
            parameters: None,
        },
        DataExtractor {
            func: extract_stats,
            key: None,
            tag: Some("web"),
        },
    ));
    locations
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
    [
        value.get("modern"),
        value.get("web"),
        value.get("legacy"),
        Some(value),
    ]
    .into_iter()
    .flatten()
}

fn rate_unit(value: Option<&Value>, algo: HashAlgorithm) -> Option<HashRateUnit> {
    match value {
        Some(value) => {
            let raw = value.as_str()?.trim().to_ascii_uppercase();
            let solution_unit = match raw.as_str() {
                "S/S" | "SOL/S" | "SOLS/S" => Some(HashRateUnit::Hash),
                "KS/S" | "KSOL/S" | "KSOLS/S" => Some(HashRateUnit::KiloHash),
                "MS/S" | "MSOL/S" | "MSOLS/S" => Some(HashRateUnit::MegaHash),
                "GS/S" | "GSOL/S" | "GSOLS/S" => Some(HashRateUnit::GigaHash),
                _ => None,
            };
            if solution_unit.is_some() {
                return (algo == HashAlgorithm::Equihash).then_some(solution_unit?);
            }
            HashRateUnit::from_str(&raw).ok()
        }
        None if matches!(algo, HashAlgorithm::SHA256 | HashAlgorithm::KHeavyHash) => {
            Some(HashRateUnit::GigaHash)
        }
        // Known legacy rates identify their own unit; a modern rate needs
        // one explicitly for other algorithms.
        None => None,
    }
}

fn text_rate(value: &Value, algo: HashAlgorithm) -> Option<HashRate> {
    let (raw, unit) = value.as_str()?.trim().split_once(' ')?;
    make_rate(
        &Value::String(raw.to_owned()),
        rate_unit(Some(&json!(unit.trim())), algo)?,
        algo,
    )
}

fn legacy_z15_contract(value: &Value, model: Option<&str>) -> bool {
    let Some(header) = value.get("legacy_header") else {
        return false;
    };
    model.is_some_and(|model| compact_model(model) == "Z15")
        && header
            .get("Type")
            .and_then(Value::as_str)
            .is_some_and(|model| compact_model(model) == "Z15")
        && header.get("CGMiner").and_then(Value::as_str) == Some("4.9.0")
        && header.get("Miner").and_then(Value::as_str) == Some("9.0.0.5")
        && header.get("CompileTime").and_then(Value::as_str) == Some("Fri Jul  3 11:39:06 CST 2020")
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
    z15_contract: bool,
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
    let explicit_key = if window == "5s" {
        "RT HASHRATE"
    } else {
        "AV HASHRATE"
    };
    if let Some(raw) = row.get(explicit_key) {
        values.push(text_rate(raw, algo)?);
    }
    for (key, value) in row.as_object()? {
        let Some((unit, suffix)) = key.split_once(' ') else {
            continue;
        };
        if suffix == window
            && let Some(mut unit) = rate_unit(Some(&json!(unit)), algo)
        {
            if algo == HashAlgorithm::Equihash {
                // This exact legacy Z15 dashboard labels KSol/s and divides
                // GHS 5s/av by 1000. The field name is not its physical unit.
                if z15_contract && key.starts_with("GHS ") {
                    unit = HashRateUnit::Hash;
                } else if key.starts_with("GHS ") {
                    // Other legacy Equihash firmware can also mislabel GHS.
                    // Prefer its explicit RT/AV HASHRATE, never a magnitude guess.
                    continue;
                }
            }
            values.push(make_rate(value, unit, algo)?);
        }
    }
    let first = values.first()?.clone();
    values
        .iter()
        .all(|rate| (rate.value - first.value).abs() <= first.value.abs() * 0.001 + 1e-9)
        .then_some(first)
}

#[cfg(test)]
pub(super) fn current_rate(value: &Value, algo: HashAlgorithm) -> Option<HashRate> {
    current_rate_for_model(value, algo, None)
}

pub(super) fn current_rate_for_model(
    value: &Value,
    algo: HashAlgorithm,
    model: Option<&str>,
) -> Option<HashRate> {
    for row in [
        value.get("modern"),
        value.get("web"),
        value.get("legacy"),
        value.get("summary"),
        Some(value),
    ]
    .into_iter()
    .flatten()
    {
        if row.as_object().is_some_and(|row| {
            row.keys()
                .any(|key| key == "rate_5s" || key.ends_with(" 5s") || key == "RT HASHRATE")
        }) {
            if algo == HashAlgorithm::Equihash
                && row.get("rate_5s").is_none()
                && row.get("RT HASHRATE").is_none()
                && !legacy_z15_contract(value, model)
                && !row.as_object()?.keys().any(|key| {
                    key.split_once(' ').is_some_and(|(unit, suffix)| {
                        suffix == "5s"
                            && unit != "GHS"
                            && rate_unit(Some(&json!(unit)), algo).is_some()
                    })
                })
            {
                continue;
            }
            return rate_window(
                row,
                "5s",
                "rate_5s",
                algo,
                legacy_z15_contract(value, model),
            );
        }
    }
    None
}

#[cfg(test)]
pub(super) fn hashrate(value: &Value, algo: HashAlgorithm) -> Option<HashRate> {
    hashrate_for_model(value, algo, None)
}

pub(super) fn hashrate_for_model(
    value: &Value,
    algo: HashAlgorithm,
    model: Option<&str>,
) -> Option<HashRate> {
    if let Some(rate) = current_rate_for_model(value, algo, model) {
        return Some(rate);
    }
    // If a current field was present but invalid/conflicting, no average can
    // safely repair it. Missing fields alone may fall back to an average.
    for row in [
        value.get("modern"),
        value.get("web"),
        value.get("legacy"),
        value.get("summary"),
        Some(value),
    ]
    .into_iter()
    .flatten()
    {
        if row.as_object().is_some_and(|row| {
            row.keys()
                .any(|key| key == "rate_5s" || key.ends_with(" 5s") || key == "RT HASHRATE")
        }) {
            return None;
        }
    }
    for row in [
        value.get("modern"),
        value.get("web"),
        value.get("legacy"),
        value.get("summary"),
        Some(value),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(rate) = rate_window(
            row,
            "av",
            "rate_avg",
            algo,
            legacy_z15_contract(value, model),
        ) {
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
        Some(value) => rate_unit(Some(value), algo)?,
        None if matches!(algo, HashAlgorithm::SHA256 | HashAlgorithm::KHeavyHash)
            || key == "total_rateideal" && algo == HashAlgorithm::Scrypt =>
        {
            HashRateUnit::GigaHash
        }
        None => return None,
    };
    make_rate(raw, unit, algo)
}

pub(super) fn uptime(value: &Value) -> Option<Duration> {
    for row in rows(value) {
        if let Some(raw) = row.get("elapsed").or_else(|| row.get("Elapsed")) {
            let seconds = raw
                .as_u64()
                .or_else(|| raw.as_str()?.trim().parse::<u64>().ok())?;
            return Some(Duration::from_secs(seconds));
        }
    }
    None
}

pub(super) fn messages(value: &Value) -> Vec<MinerMessage> {
    let timestamp = value
        .pointer("/STATUS/when")
        .or_else(|| value.pointer("/STATUS/0/When"))
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);
    let statuses = value.pointer("/SUMMARY/0/status").unwrap_or(value);
    statuses
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let status = item.get("status")?.as_str()?.trim().to_ascii_lowercase();
            if status == "s" {
                return None;
            }
            let text = item.get("msg").and_then(Value::as_str).unwrap_or("").trim();
            let code = item.get("code").and_then(Value::as_u64).unwrap_or(0);
            // The captured hydro dashboard leaves unused fan status blank. It is
            // not a miner error and must not create an empty/"Unknown error" row.
            if status.is_empty() && text.is_empty() && code == 0 {
                return None;
            }
            let severity = match status.as_str() {
                "e" => MessageSeverity::Error,
                "w" => MessageSeverity::Warning,
                _ => MessageSeverity::Info,
            };
            Some(MinerMessage::new(
                timestamp,
                code,
                text.to_owned(),
                severity,
            ))
        })
        .collect()
}

fn chip_states(value: Option<&Value>, physical_count: Option<u16>) -> Vec<ChipData> {
    let Some(raw) = value.and_then(Value::as_str) else {
        return vec![];
    };
    let mut statuses = raw
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect::<Vec<_>>();
    // Stock cgminer exposes one character per chip; spacing groups chips.
    // Reject an unknown layout instead of assigning invented chip positions.
    if statuses
        .iter()
        .any(|character| !matches!(character, 'o' | 'O' | 'x' | 'X' | '-'))
    {
        return vec![];
    }
    if let Some(physical_count) = physical_count {
        let count = physical_count as usize;
        // Captured 42/84-chip boards pad their final display group with '-'.
        // Remove only a proven empty tail, never working/failing chip data.
        if statuses
            .get(count..)
            .is_some_and(|tail| tail.iter().all(|status| *status == '-'))
        {
            statuses.truncate(count);
        }
    }
    statuses
        .into_iter()
        .enumerate()
        .filter_map(|(position, status)| {
            Some(ChipData {
                position: u16::try_from(position).ok()?,
                working: Some(matches!(status, 'o' | 'O')),
                ..Default::default()
            })
        })
        .collect()
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
            board.expected_hashrate = chain.get("rate_ideal").and_then(|value| {
                make_rate(
                    value,
                    rate_unit(
                        chain.get("rate_unit").or_else(|| row.get("rate_unit")),
                        algo,
                    )?,
                    algo,
                )
            });
            board.serial_number = chain
                .get("sn")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned);
            board.working_chips = chain
                .get("asic_num")
                .and_then(number)
                .filter(|value| value.fract() == 0.0 && *value >= 0.0 && *value <= u16::MAX as f64)
                .map(|value| value as u16);
            board.chips = chip_states(chain.get("asic"), board.expected_chips);
            board.frequency = chain
                .get("frequency_mhz")
                .or_else(|| chain.get("freq_avg"))
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
            let rate = row.get(&format!("chain_rate{slot}")).and_then(|raw| {
                if let Some(explicit) = row.get(&format!("CHAIN AVG HASHRATE{slot}")) {
                    text_rate(explicit, algo)
                } else if algo == HashAlgorithm::Equihash {
                    if legacy_z15_contract(value, Some(model)) {
                        // The legacy Z15 dashboard displays chain_rateN
                        // directly under its KSol/s board-rate header.
                        make_rate(raw, HashRateUnit::KiloHash, algo)
                    } else {
                        None
                    }
                } else {
                    let unit = match row.get("rate_unit") {
                        Some(unit) => rate_unit(Some(unit), algo)?,
                        None => HashRateUnit::GigaHash,
                    };
                    make_rate(raw, unit, algo)
                }
            });
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
            board.chips = chip_states(row.get(&format!("chain_acs{slot}")), board.expected_chips);
            board.active = board.hashrate.as_ref().map(|rate| rate.value > 0.0);
            let mut pcb = temperatures(row.get(&format!("temp_pcb{slot}")));
            if pcb.is_empty() {
                pcb = ["temp_out_pcb_", "temp_in_pcb_"]
                    .into_iter()
                    .flat_map(|prefix| temperatures(row.get(&format!("{prefix}{slot}"))))
                    .collect();
            }
            if pcb.is_empty() {
                let key = if legacy_z15_contract(value, Some(model)) {
                    format!("temp{slot}")
                } else {
                    format!("temp2_{slot}")
                };
                pcb = temperatures(row.get(&key));
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
                let key = if legacy_z15_contract(value, Some(model)) {
                    format!("temp2_{slot}")
                } else {
                    format!("temp{slot}")
                };
                chip = temperatures(row.get(&key));
            }
            board.inlet_chip_temperature = minimum(&chip);
            board.outlet_chip_temperature = maximum(&chip);
            board.frequency = row
                .get(&format!("freq{slot}"))
                .or_else(|| row.get(&format!("frequency{slot}")))
                .and_then(number)
                .filter(|value| *value > 0.0)
                .map(Frequency::from_megahertz);
            boards.push(board);
        }
    }
    boards.sort_by_key(|board| board.position);
    boards
}

#[cfg(test)]
pub(super) fn fans(value: &Value) -> Vec<FanData> {
    fans_for_model(value, None)
}

pub(super) fn fans_for_model(value: &Value, model: Option<&str>) -> Vec<FanData> {
    let z15_contract = legacy_z15_contract(value, model);
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
    let count = row
        .get("fan_num")
        .and_then(number)
        .filter(|value| value.fract() == 0.0 && *value >= 0.0 && *value <= i16::MAX as f64)
        .map(|value| value as usize);
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
            let rpm = number(value).filter(|value| *value >= 0.0)?;
            // This captured Z15 dashboard displays only nonzero legacy fan
            // channels. fan_num=2 is paired with fan3/4; fan1/2 are padding,
            // so their zeros cannot establish physical stopped fan identities.
            if z15_contract && rpm == 0.0 {
                return None;
            }
            // Captured hydro RPC replies pad fan1..fan4 with zeros while
            // fan_num is zero. Keep stopped fans inside the reported count,
            // and never discard a contradictory positive RPM observation.
            if count.is_some_and(|count| index >= count) && rpm == 0.0 {
                return None;
            }
            Some(FanData {
                position: i16::try_from(index).ok()?,
                rpm: Some(AngularVelocity::from_rpm(rpm)),
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

fn reported_power(value: &Value) -> Option<(Power, String)> {
    let (row, route) = [
        (value.get("modern"), "antminer.rpc.stats(new_api=true)"),
        (value.get("web"), "antminer.cgi.stats"),
        (value.get("legacy"), "antminer.rpc.stats"),
        (Some(value), "antminer.stats"),
    ]
    .into_iter()
    .filter_map(|(row, route)| Some((row?, route)))
    .find(|(row, _)| {
        ["chain_power", "total_power", "power", "Power", "watt"]
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
        candidates.push((watts, "chain_power"));
    }
    for key in ["total_power", "power", "Power", "watt"] {
        if let Some(value) = row.get(key).and_then(number).filter(|value| *value >= 0.0) {
            candidates.push((value, key));
        }
    }
    let (first, field) = *candidates.first()?;
    candidates
        .iter()
        .all(|(value, _)| (*value - first).abs() <= first * 0.01 + 1.0)
        .then(|| (Power::from_watts(first), format!("{route}:{field}")))
}

pub(super) fn wattage(value: &Value) -> Option<Power> {
    reported_power(value).map(|(power, _)| power)
}

pub(super) fn wattage_source(value: &Value) -> Option<String> {
    // The API response proves where the reported number came from. It does
    // not prove a PSU measurement or an estimate, so that flag remains unknown.
    reported_power(value).map(|(_, source)| source)
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

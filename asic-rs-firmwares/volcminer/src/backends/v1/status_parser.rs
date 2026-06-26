use anyhow::{Result, anyhow};
use serde_json::Value;

fn raw_value_after_key<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let after_key = text.get(text.find(&needle)? + needle.len()..)?;
    after_key
        .get(after_key.find(':')? + 1..)
        .map(str::trim_start)
}

pub(super) fn extract_text_field(text: &str, key: &str) -> Option<String> {
    let raw_value = raw_value_after_key(text, key)?;

    if let Some(value) = raw_value.strip_prefix('"') {
        return value.split('"').next().map(str::to_string);
    }

    let value = raw_value
        .split([',', '}', '\n'])
        .next()?
        .trim()
        .trim_matches('"');
    Some(value.to_string())
}

fn matching_delimiter_end(text: &str, start: usize, opener: char, closer: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in text.get(start..)?.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
        } else if ch == opener {
            depth += 1;
        } else if ch == closer {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(start + offset + ch.len_utf8());
            }
        }
    }

    None
}

fn extract_json_after_key(text: &str, key: &str, opener: char, closer: char) -> Option<String> {
    let raw_value = raw_value_after_key(text, key)?;
    let start = raw_value.find(opener)?;
    let end = matching_delimiter_end(raw_value, start, opener, closer)?;

    raw_value.get(start..end).map(str::to_string)
}

fn extract_quoted_field(text: &str, key: &str) -> Option<String> {
    let value = raw_value_after_key(text, key)?.strip_prefix('"')?;
    value.split('"').next().map(str::to_string)
}

fn extract_temp_field(text: &str) -> Option<String> {
    let raw_value = raw_value_after_key(text, "temp")?;
    let after_bracket = raw_value.strip_prefix('[').or_else(|| {
        raw_value
            .find('[')
            .and_then(|start| raw_value.get(start + 1..))
    })?;
    after_bracket
        .split([',', ']'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_legacy_devs(text: &str) -> Vec<Value> {
    let Some(devs_text) = extract_json_after_key(text, "devs", '[', ']') else {
        return vec![];
    };

    let mut devs = Vec::new();
    let mut remaining = devs_text.as_str();
    while let Some(start) = remaining.find('{') {
        let Some(end) = matching_delimiter_end(remaining, start, '{', '}') else {
            break;
        };
        let Some(dev_text) = remaining.get(start..end) else {
            break;
        };
        let mut dev = serde_json::Map::new();
        for key in ["index", "chain_acn", "freq", "chain_acs"] {
            if let Some(value) = extract_quoted_field(dev_text, key) {
                dev.insert(key.to_string(), Value::String(value));
            }
        }
        if let Some(temp) = extract_temp_field(dev_text) {
            dev.insert("temp".to_string(), Value::String(temp));
        }
        if !dev.is_empty() {
            devs.push(Value::Object(dev));
        }
        remaining = remaining.get(end..).unwrap_or_default();
    }

    devs
}

pub(super) fn parse_miner_status_text(text: &str) -> Result<Value> {
    let mut status = serde_json::Map::new();

    let summary = extract_json_after_key(text, "summary", '{', '}')
        .ok_or_else(|| anyhow!("missing summary"))?;
    status.insert("summary".to_string(), serde_json::from_str(&summary)?);

    if let Some(pools) = extract_json_after_key(text, "pools", '[', ']')
        && let Ok(pools) = serde_json::from_str(&pools)
    {
        status.insert("pools".to_string(), pools);
    }

    for idx in 1..=4 {
        let key = format!("fan{idx}");
        if let Some(value) = extract_text_field(text, &key) {
            status.insert(key, Value::String(value));
        }
    }

    let devs = parse_legacy_devs(text);
    if !devs.is_empty() {
        status.insert("devs".to_string(), Value::Array(devs));
    }

    Ok(Value::Object(status))
}

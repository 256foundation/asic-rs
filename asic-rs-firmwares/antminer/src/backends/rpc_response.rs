// SPDX-License-Identifier: Apache-2.0
use asic_rs_core::data::command::RPCCommandStatus;
use serde_json::Value;

/// Decode stock RPC JSON, including the captured Z15 9.0.0.5 producer defect.
/// That firmware omits one comma between the header and telemetry STATS rows.
/// Only an unquoted adjacent-object boundary and that exact response schema
/// qualify; strings, other models and other malformed JSON are never repaired.
pub(super) fn parse_response(response: &str) -> anyhow::Result<Value> {
    let value = match serde_json::from_str::<Value>(response) {
        Ok(value) => value,
        Err(original) => repair_z15_stats(response).ok_or(original)?,
    };
    let status = value.get("STATUS").and_then(|status| {
        if status.is_array() {
            status.get(0)
        } else {
            Some(status)
        }
    });
    if let Some(status) = status {
        let code = status.get("STATUS").and_then(Value::as_str).unwrap_or("S");
        let message = status.get("Msg").and_then(Value::as_str);
        RPCCommandStatus::from_str(code, message).into_result()?;
    }
    Ok(value)
}

fn repair_z15_stats(response: &str) -> Option<Value> {
    let mut quoted = false;
    let mut escaped = false;
    let mut separator = None;
    let bytes = response.as_bytes();
    for (index, byte) in bytes.iter().copied().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        } else if byte == b'}'
            && bytes.get(index + 1) == Some(&b'{')
            && separator.replace(index + 1).is_some()
        {
            return None;
        }
    }
    let mut repaired = response.to_owned();
    repaired.insert(separator?, ',');
    let value: Value = serde_json::from_str(&repaired).ok()?;
    let rows = value.get("STATS")?.as_array()?;
    if rows.len() != 2 {
        return None;
    }
    let header = &rows[0];
    let telemetry = &rows[1];
    (header.get("Type")?.as_str()? == "Antminer Z15"
        && header.get("CGMiner")?.as_str()? == "4.9.0"
        && header.get("Miner")?.as_str()? == "9.0.0.5"
        && header.get("CompileTime")?.as_str()? == "Fri Jul  3 11:39:06 CST 2020"
        && telemetry.get("ID")?.as_str()? == "ZCASH0"
        && telemetry.get("STATS").and_then(Value::as_u64) == Some(0)
        && telemetry.get("chain_acn1").is_some())
    .then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::json::v2023_07::Z15_MALFORMED_STATS_CAPTURED;

    #[test]
    fn captured_missing_separator_is_repaired_only_for_exact_z15_producer() {
        assert!(serde_json::from_str::<Value>(Z15_MALFORMED_STATS_CAPTURED).is_err());
        let value = parse_response(Z15_MALFORMED_STATS_CAPTURED).unwrap();
        assert_eq!(value["STATS"][1]["chain_acn3"], 2);
        assert!(
            parse_response(&Z15_MALFORMED_STATS_CAPTURED.replace("Antminer Z15", "Antminer S19"))
                .is_err()
        );
        assert!(
            parse_response(&Z15_MALFORMED_STATS_CAPTURED.replace("9.0.0.5", "9.0.0.6")).is_err()
        );
        assert!(
            parse_response(
                &Z15_MALFORMED_STATS_CAPTURED.replace("\"STATUS\":\"S\"", "\"STATUS\":\"E\"")
            )
            .is_err()
        );
        assert!(
            parse_response(r#"{"note":"}{","STATS":[{"Type":"Antminer Z15"}{"chain_acn1":3}]}"#)
                .is_err()
        );
    }

    #[test]
    fn valid_json_and_modern_error_status_keep_their_meaning() {
        let raw = r#"{"STATUS":[{"STATUS":"S"}],"note":"}{"}"#;
        assert_eq!(parse_response(raw).unwrap()["note"], "}{");
        assert!(parse_response(r#"{"STATUS":{"STATUS":"E","Msg":"unavailable"}}"#).is_err());
    }
}

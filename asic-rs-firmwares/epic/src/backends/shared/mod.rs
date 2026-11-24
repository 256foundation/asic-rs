use asic_rs_core::data::message::{MessageSeverity, MinerMessage};
use serde_json::Value;

pub(super) mod releases;
pub(super) mod web;

pub(super) fn parse_error(error: &Value) -> Option<MinerMessage> {
    let (timestamp, message) = match error.as_object() {
        Some(fields) if fields.contains_key("occurred_at") => {
            let mut fields = fields.clone();
            let timestamp = fields
                .remove("occurred_at")
                .and_then(|value| value.as_u64())
                .and_then(|seconds| u32::try_from(seconds).ok())
                .unwrap_or(0);
            if fields.len() != 1 {
                return None;
            }
            (timestamp, format_error(&Value::Object(fields))?)
        }
        _ => (0, format_error(error)?),
    };
    Some(MinerMessage::new(
        timestamp,
        0,
        message,
        MessageSeverity::Error,
    ))
}

/// Format a miner error from the summary or an error history entry.
fn format_error(error: &Value) -> Option<String> {
    match error {
        Value::Null => None,
        Value::String(message) => {
            let message = message.trim();
            if message.is_empty() {
                return None;
            }

            // Older PowerPlay versions may serialize the externally tagged
            // MinerError object into a JSON string instead of returning the
            // object directly.
            serde_json::from_str::<Value>(message)
                .ok()
                .filter(|value| value.is_object())
                .and_then(|value| format_error(&value))
                .or_else(|| Some(message.to_string()))
        }
        Value::Object(error) if error.is_empty() => None,
        Value::Object(error) if error.len() == 1 => {
            let (kind, details) = error.iter().next()?;
            match details {
                Value::Null => Some(kind.clone()),
                Value::String(message) if message.trim().is_empty() => Some(kind.clone()),
                Value::String(message) => Some(message.trim().to_string()),
                details => Some(format!("{kind}: {details}")),
            }
        }
        value => Some(value.to_string()),
    }
}

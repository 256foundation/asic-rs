use serde_json::Value;

pub(super) mod releases;
pub(super) mod web;

/// Format a miner error from the summary or an error history entry.
pub(super) fn format_error(error: &Value) -> Option<String> {
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

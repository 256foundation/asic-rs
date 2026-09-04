//! Share difficulty helpers.
//!
//! Firmwares report best-share difficulty as a JSON number, a decimal string,
//! or an AxeOS-style suffix string (`"483k"`, `"1.2M"`).

use serde_json::Value;

/// Parse a share difficulty from a JSON value.
///
/// Accepts numbers and strings. Suffixes `k`, `M`, `G`, `T`, `P`, and `E`
/// are treated as SI powers of 1000. Returns `None` for negative, non-finite,
/// or unparsable values.
pub fn parse_share_difficulty(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64().filter(|v| v.is_finite() && *v >= 0.0),
        Value::String(s) => parse_share_difficulty_str(s),
        _ => None,
    }
}

/// Parse a share difficulty from a decimal or suffix string.
pub fn parse_share_difficulty_str(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num, mul) = match s.as_bytes().last().copied() {
        Some(
            b @ (b'k' | b'K' | b'm' | b'M' | b'g' | b'G' | b't' | b'T' | b'p' | b'P' | b'e' | b'E'),
        ) => {
            let mul = match b {
                b'k' | b'K' => 1e3,
                b'm' | b'M' => 1e6,
                b'g' | b'G' => 1e9,
                b't' | b'T' => 1e12,
                b'p' | b'P' => 1e15,
                b'e' | b'E' => 1e18,
                _ => unreachable!(),
            };
            (&s[..s.len() - 1], mul)
        }
        _ => (s, 1.0),
    };
    let v: f64 = num.trim().parse().ok()?;
    let v = v * mul;
    (v.is_finite() && v >= 0.0).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_numbers_and_suffix_strings() {
        assert_eq!(parse_share_difficulty(&json!(12)), Some(12.0));
        assert_eq!(parse_share_difficulty(&json!(0)), Some(0.0));
        assert_eq!(parse_share_difficulty(&json!("483k")), Some(483_000.0));
        assert_eq!(parse_share_difficulty(&json!("1.2M")), Some(1.2e6));
        assert_eq!(
            parse_share_difficulty(&json!("499065300")),
            Some(499_065_300.0)
        );
        assert_eq!(parse_share_difficulty(&json!("  2.0k ")), Some(2_000.0));
        assert_eq!(parse_share_difficulty(&json!(-1)), None);
        assert_eq!(parse_share_difficulty(&json!("nope")), None);
        assert_eq!(parse_share_difficulty(&json!(null)), None);
    }
}

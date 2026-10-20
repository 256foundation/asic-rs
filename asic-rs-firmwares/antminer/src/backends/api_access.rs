//! RPC API access settings on stock Antminer firmware.
//!
//! `get_miner_conf.cgi` returns the miner config file verbatim, including the
//! cgminer/bmminer keys `api-listen`, `api-network`, `api-groups` and
//! `api-allow`. The stock default is
//! `"api-groups": "A:stats:pools:devs:summary:version"` with
//! `"api-allow": "A:0/0,W:*"`, which gives every host read-only access and no
//! host privileged access (`A:0/0` matches first, and `*` is not an address
//! cgminer accepts).
//!
//! The settings cannot be written through the web API. In the stock
//! `set_miner_conf.cgi` sources Bitmain has published, the CGI rewrites the
//! whole config file from a template with the `api-*` lines hard-coded, so it
//! never reads them from the request. Later firmware, whose CGIs are compiled,
//! still answers with the same stock values and offers no setting for them.
//! Writing pools, fan or mode settings through the web API also resets an
//! allow list that was edited on the miner itself.

use anyhow::Result;
use asic_rs_core::config::api_access::{ApiAccessConfig, ApiAllowList, ApiCommandGroup};
use serde_json::Value;

use crate::firmware::AntMinerStockFirmware;

fn flag(value: &Value) -> Option<bool> {
    value.as_bool().or_else(|| match value.as_str()? {
        "1" => Some(true),
        "0" => Some(false),
        s if s.eq_ignore_ascii_case("true") => Some(true),
        s if s.eq_ignore_ascii_case("false") => Some(false),
        _ => None,
    })
}

/// Read the API access settings out of a `get_miner_conf.cgi` response.
pub(crate) fn api_access_from_miner_conf(miner_conf: &Value) -> Result<ApiAccessConfig> {
    if !miner_conf.is_object() {
        anyhow::bail!(
            "{} miner config is not a JSON object",
            AntMinerStockFirmware::default()
        );
    }
    let text = |key: &str| miner_conf.get(key).and_then(Value::as_str);

    let groups = text("api-groups")
        .map(ApiCommandGroup::parse_list)
        .transpose()?;
    let allow = text("api-allow")
        .map(|allow| allow.parse::<ApiAllowList>())
        .transpose()?;

    Ok(ApiAccessConfig {
        listen: miner_conf.get("api-listen").and_then(flag),
        network: miner_conf.get("api-network").and_then(flag),
        groups,
        allow,
    })
}

/// The error returned for a write, with the reason it cannot work.
pub(crate) fn set_api_access_unsupported() -> anyhow::Error {
    anyhow::anyhow!(
        "Setting API access is not supported on {} firmware: set_miner_conf.cgi rewrites \
         api-listen, api-network, api-groups and api-allow from a fixed template, so the \
         allow list can only be changed on the miner itself",
        AntMinerStockFirmware::default()
    )
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use serde_json::json;

    use super::*;

    #[test]
    fn reads_the_stock_default() -> Result<()> {
        let conf = json!({
            "pools": [],
            "api-listen": true,
            "api-network": true,
            "api-groups": "A:stats:pools:devs:summary:version",
            "api-allow": "A:0/0,W:*",
            "bitmain-fan-ctrl": false,
        });
        let access = api_access_from_miner_conf(&conf)?;
        assert_eq!(access.listen, Some(true));
        assert_eq!(access.network, Some(true));
        let groups = access.groups.clone().unwrap_or_default();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].letter, 'A');
        assert_eq!(
            access.allow.as_ref().map(ToString::to_string).as_deref(),
            Some("A:0/0,W:*")
        );
        let lan: IpAddr = "192.0.2.10".parse()?;
        assert!(!access.allows_write(lan));
        Ok(())
    }

    #[test]
    fn reads_an_allow_list_edited_on_the_miner() -> Result<()> {
        // String flags, as some shell-templated configs carry them.
        let conf = json!({
            "api-listen": "true",
            "api-network": "true",
            "api-allow": "W:192.0.2.10,W:127.0.0.1,A:0/0",
        });
        let access = api_access_from_miner_conf(&conf)?;
        assert_eq!(access.listen, Some(true));
        assert_eq!(access.groups, None);
        assert!(access.allows_write("192.0.2.10".parse()?));
        assert!(access.allows_write("127.0.0.1".parse()?));
        assert!(!access.allows_write("192.0.2.11".parse()?));
        Ok(())
    }

    #[test]
    fn missing_keys_are_reported_as_unknown() -> Result<()> {
        let access = api_access_from_miner_conf(&json!({"pools": []}))?;
        assert_eq!(access, ApiAccessConfig::default());
        assert!(api_access_from_miner_conf(&json!("not a config")).is_err());
        Ok(())
    }

    #[test]
    fn invalid_api_groups_are_an_error() {
        assert!(api_access_from_miner_conf(&json!({"api-groups": "AB:stats"})).is_err());
    }
}

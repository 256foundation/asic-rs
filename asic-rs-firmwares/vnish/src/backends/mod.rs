use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor, Validate},
    model::MinerModel,
};
use serde_json::Value;
pub use v1_2_0::VnishV120;
pub use v1_3_0::VnishV130;

pub mod v1_2_0;
pub mod v1_3_0;

#[cfg(test)]
mod test;

pub struct Vnish;

fn parse_devfee_connected(pools: &Value) -> Option<bool> {
    let mut disconnected = false;

    for pool in pools.as_array()? {
        let is_devfee = pool
            .get("pool_type")
            .and_then(Value::as_str)
            .is_some_and(|pool_type| pool_type.eq_ignore_ascii_case("DevFee"));
        if !is_devfee {
            continue;
        }

        let Some(status) = pool.get("status").and_then(Value::as_str) else {
            continue;
        };
        if ["active", "working", "rejecting"]
            .iter()
            .any(|candidate| status.eq_ignore_ascii_case(candidate))
        {
            return Some(true);
        }
        if ["offline", "disabled"]
            .iter()
            .any(|candidate| status.eq_ignore_ascii_case(candidate))
        {
            disconnected = true;
        }
    }

    disconnected.then_some(false)
}

impl MinerConstructor for Vnish {
    #[allow(clippy::new_ret_no_self)]
    #[allow(clippy::if_same_then_else)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        // Manual throttle (`tuning_percent`) was introduced in the 1.3.x line;
        // assumed cutoff 1.3.0 (1.3.4 verified live, 1.2.x API has no endpoint).
        if VnishV130::validate(version.as_ref()) {
            Box::new(VnishV130::new(ip, model))
        } else if VnishV120::validate(version.as_ref()) {
            Box::new(VnishV120::new(ip, model))
        } else {
            Box::new(VnishV120::new(ip, model))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_devfee_connected;

    #[test]
    fn devfee_connection_uses_typed_pool_status() {
        assert_eq!(
            parse_devfee_connected(&json!([
                { "pool_type": "UserPool", "status": "working" },
                { "pool_type": "DevFee", "status": "offline" }
            ])),
            Some(false)
        );
        assert_eq!(
            parse_devfee_connected(&json!([
                { "pool_type": "DevFee", "status": "offline" },
                { "pool_type": "DevFee", "status": "rejecting" }
            ])),
            Some(true)
        );
        assert_eq!(
            parse_devfee_connected(&json!([
                { "pool_type": "UserPool", "status": "offline" }
            ])),
            None
        );
        assert_eq!(
            parse_devfee_connected(&json!([
                { "pool_type": "DevFee", "status": "unknown" }
            ])),
            None
        );
    }
}

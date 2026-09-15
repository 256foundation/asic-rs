pub mod avalon_a;
pub mod avalon_q;
pub(crate) mod rpc;

use std::{any::Any, net::IpAddr};

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
use asic_rs_makes_avalon::models::AvalonMinerModel;
pub use avalon_a::AvalonAMiner;
pub use avalon_q::AvalonQMiner;
use serde_json::Value;

fn parse_last_share_time(pool: &Value, elapsed: Option<u64>) -> Option<u64> {
    let accepted_shares = pool.get("Accepted").and_then(Value::as_u64);
    if accepted_shares == Some(0) {
        return None;
    }

    let raw_time = pool.get("Last Share Time")?;
    if let Some(timestamp) = asic_rs_core::util::parse_last_share_time(raw_time) {
        return Some(timestamp);
    }

    let session_time = raw_time.as_u64()?;
    let seconds_ago = elapsed?.checked_sub(session_time)?;
    if accepted_shares.is_none_or(|shares| shares == 0) {
        return None;
    }

    asic_rs_core::util::unix_timestamp_secs().checked_sub(seconds_ago)
}

pub struct AvalonMiner;

impl MinerConstructor for AvalonMiner {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, _: Option<semver::Version>) -> Box<dyn Miner> {
        let avalon_model = (&model as &dyn Any)
            .downcast_ref::<AvalonMinerModel>()
            .cloned();
        match avalon_model {
            Some(AvalonMinerModel::AvalonHomeQ) => Box::new(AvalonQMiner::new(ip, model)),
            _ => Box::new(AvalonAMiner::new(ip, model)),
        }
    }
}

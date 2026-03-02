use asic_rs_core::traits::miner::{Miner, MinerConstructor};
use asic_rs_core::traits::model::MinerModel;
use std::net::IpAddr;
pub use v1::MaraV1;

pub mod v1;

pub struct Marathon;

impl MinerConstructor for Marathon {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, _: Option<semver::Version>) -> Box<dyn Miner> {
        Box::new(MaraV1::new(ip, model))
    }
}

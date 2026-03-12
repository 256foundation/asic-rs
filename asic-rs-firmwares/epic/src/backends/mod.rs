use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
pub use v1::PowerPlayV1;

pub mod v1;

pub struct PowerPlay;

impl MinerConstructor for PowerPlay {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, _: Option<semver::Version>) -> Box<dyn Miner> {
        Box::new(PowerPlayV1::new(ip, model))
    }
}

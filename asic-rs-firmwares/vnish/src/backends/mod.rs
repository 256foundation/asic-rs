use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
pub use v1_2_0::VnishV120;

pub mod v1_2_0;

pub struct Vnish;

impl MinerConstructor for Vnish {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, _: Option<semver::Version>) -> Box<dyn Miner> {
        Box::new(VnishV120::new(ip, model))
    }
}

use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
use v1::LuxMinerV1;

pub mod v1;

pub struct LuxMiner;

impl MinerConstructor for LuxMiner {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, _: Option<semver::Version>) -> Box<dyn Miner> {
        Box::new(LuxMinerV1::new(ip, model))
    }
}

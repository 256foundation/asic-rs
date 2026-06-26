use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
use v1::VolcMinerV1;

pub mod v1;

pub struct VolcMiner;

impl MinerConstructor for VolcMiner {
    fn new(ip: IpAddr, model: impl MinerModel, _: Option<semver::Version>) -> Box<dyn Miner> {
        Box::new(VolcMinerV1::new(ip, model))
    }
}

pub mod v2020;
pub mod v2023_07;

use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor, Validate},
    model::MinerModel,
};
use v2020::AntMinerV2020;
use v2023_07::AntMinerV202307;

pub struct AntMiner;

impl MinerConstructor for AntMiner {
    #[allow(clippy::new_ret_no_self)]
    #[allow(clippy::if_same_then_else)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        if AntMinerV2020::validate(version.as_ref()) {
            Box::new(AntMinerV2020::new(ip, model))
        } else if AntMinerV202307::validate(version.as_ref()) {
            Box::new(AntMinerV202307::new(ip, model))
        } else {
            Box::new(AntMinerV202307::new(ip, model))
        }
    }
}

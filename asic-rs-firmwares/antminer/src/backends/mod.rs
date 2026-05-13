pub mod v2020;
pub mod v2023_07;

use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
use semver::Version;
use v2020::AntMinerV2020;
use v2023_07::AntMinerV202307;

pub struct AntMiner;

impl MinerConstructor for AntMiner {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        match version {
            Some(ref v) if *v < Version::new(2023, 7, 0) => Box::new(AntMinerV2020::new(ip, model)),
            _ => Box::new(AntMinerV202307::new(ip, model)),
        }
    }
}

pub mod v2020;

use asic_rs_core::traits::miner::{Miner, MinerConstructor};
use asic_rs_core::traits::model::MinerModel;
use std::net::IpAddr;
use v2020::AntMinerV2020;

pub struct AntMiner;

impl MinerConstructor for AntMiner {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, _: Option<semver::Version>) -> Box<dyn Miner> {
        Box::new(AntMinerV2020::new(ip, model))
    }
}

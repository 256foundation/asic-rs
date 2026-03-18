pub(crate) mod v2025;

use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
use v2025::SealMinerV2025;

pub struct SealMiner;

impl MinerConstructor for SealMiner {
    #[allow(clippy::new_ret_no_self)]
    fn new(
        ip: IpAddr,
        model: impl MinerModel,
        _version: Option<semver::Version>,
    ) -> Box<dyn Miner> {
        Box::new(SealMinerV2025::new(ip, model))
    }
}

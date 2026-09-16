use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
pub use v1::PowerPlayV1;

pub mod v1;

pub struct PowerPlay;

impl PowerPlay {
    pub(crate) fn build(
        ip: IpAddr,
        model: impl MinerModel,
        _version: Option<semver::Version>,
    ) -> PowerPlayV1 {
        PowerPlayV1::new(ip, model)
    }
}

impl MinerConstructor for PowerPlay {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        Box::new(Self::build(ip, model, version))
    }
}

use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
pub use v1::AuradineV1;

pub mod v1;

pub struct Auradine;

impl MinerConstructor for Auradine {
    #[allow(clippy::new_ret_no_self)]
    fn new(
        ip: IpAddr,
        model: impl MinerModel,
        _version: Option<semver::Version>,
    ) -> Box<dyn Miner> {
        Box::new(AuradineV1::new(ip, model))
    }
}

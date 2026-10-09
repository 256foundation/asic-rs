use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
pub use v1_1_0::Forgeaxe110;

pub mod v1_1_0;

pub struct Forgeaxe;

impl MinerConstructor for Forgeaxe {
    #[allow(clippy::new_ret_no_self)]
    fn new(
        ip: IpAddr,
        model: impl MinerModel,
        _version: Option<semver::Version>,
    ) -> Box<dyn Miner> {
        Box::new(Forgeaxe110::new(ip, model))
    }
}

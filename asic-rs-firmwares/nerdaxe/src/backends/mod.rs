use asic_rs_core::traits::miner::{Miner, MinerConstructor};
use asic_rs_core::traits::model::MinerModel;
use std::net::IpAddr;
use v1::NerdAxeV1;

pub mod v1;

pub struct NerdAxe;

impl MinerConstructor for NerdAxe {
    #[allow(clippy::new_ret_no_self)]
    fn new(
        ip: IpAddr,
        model: impl MinerModel,
        _version: Option<semver::Version>,
    ) -> Box<dyn Miner> {
        Box::new(NerdAxeV1::new(ip, model))
    }
}

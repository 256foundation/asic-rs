use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};

pub use v2::ApolloV2;

pub mod v2;

pub struct Apollo;

impl MinerConstructor for Apollo {
    #[allow(clippy::new_ret_no_self)]
    fn new(
        ip: IpAddr,
        model: impl MinerModel,
        _version: Option<semver::Version>,
    ) -> Box<dyn Miner> {
        Box::new(ApolloV2::new(ip, model))
    }
}

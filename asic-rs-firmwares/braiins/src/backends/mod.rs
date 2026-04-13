pub mod v21_09;
pub mod v25_07;

use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
use semver::Version;
use v21_09::BraiinsV2109;
use v25_07::BraiinsV2507;

pub struct Braiins;

impl MinerConstructor for Braiins {
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        if version.is_some_and(|v| v >= Version::new(25, 7, 0)) {
            Box::new(BraiinsV2507::new(ip, model))
        } else {
            Box::new(BraiinsV2109::new(ip, model))
        }
    }
}

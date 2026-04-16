pub mod v21_09;
pub mod v25_03;
pub mod v25_05;
pub mod v25_07;
pub mod v26_04;

use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
use semver::Version;
use v21_09::BraiinsV2109;
use v25_03::BraiinsV2503;
use v25_05::BraiinsV2505;
use v25_07::BraiinsV2507;
use v26_04::BraiinsV2604;

pub struct Braiins;

impl MinerConstructor for Braiins {
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        match version {
            Some(ref v) if *v >= Version::new(26, 4, 0) => Box::new(BraiinsV2604::new(ip, model)),
            Some(ref v) if *v >= Version::new(25, 7, 0) => Box::new(BraiinsV2507::new(ip, model)),
            Some(ref v) if *v >= Version::new(25, 5, 0) => Box::new(BraiinsV2505::new(ip, model)),
            Some(ref v) if *v >= Version::new(25, 3, 0) => Box::new(BraiinsV2503::new(ip, model)),
            _ => Box::new(BraiinsV2109::new(ip, model)),
        }
    }
}

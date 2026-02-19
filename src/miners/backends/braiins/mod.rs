mod v21_09;
pub mod v25_07;

use crate::data::device::MinerModel;
use crate::miners::backends::traits::*;
use std::net::IpAddr;
use v21_09::BraiinsV2109;
use v25_07::BraiinsV2507;

pub struct Braiins;

impl MinerConstructor for Braiins {
    fn new(ip: IpAddr, model: MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        if let Some(v) = version {
            if semver::VersionReq::parse(">=25.7.0").unwrap().matches(&v) {
                Box::new(BraiinsV2507::new(ip, model))
            } else {
                Box::new(BraiinsV2109::new(ip, model))
            }
        } else {
            Box::new(BraiinsV2109::new(ip, model))
        }
    }
}

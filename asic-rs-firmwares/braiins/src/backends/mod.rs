pub(crate) mod util;
pub mod v21_09;
pub mod v25_03;
pub mod v25_05;
pub mod v25_07;
pub mod v26_04;

use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor, Validate},
    model::MinerModel,
};

use v21_09::BraiinsV2109;
use v25_03::BraiinsV2503;
use v25_05::BraiinsV2505;
use v25_07::BraiinsV2507;
use v26_04::BraiinsV2604;

pub struct Braiins;

impl MinerConstructor for Braiins {
    #[allow(clippy::if_same_then_else)]
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        if BraiinsV2604::validate(version.as_ref()) {
            Box::new(BraiinsV2604::new(ip, model))
        } else if BraiinsV2507::validate(version.as_ref()) {
            Box::new(BraiinsV2507::new(ip, model))
        } else if BraiinsV2505::validate(version.as_ref()) {
            Box::new(BraiinsV2505::new(ip, model))
        } else if BraiinsV2503::validate(version.as_ref()) {
            Box::new(BraiinsV2503::new(ip, model))
        } else if BraiinsV2109::validate(version.as_ref()) {
            Box::new(BraiinsV2109::new(ip, model))
        } else {
            Box::new(BraiinsV2109::new(ip, model))
        }
    }
}

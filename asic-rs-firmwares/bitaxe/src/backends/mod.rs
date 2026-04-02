use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
pub use v2_0_0::Bitaxe200;
pub use v2_9_0::Bitaxe290;

pub mod v2_0_0;
pub mod v2_9_0;

pub struct Bitaxe;

impl MinerConstructor for Bitaxe {
    #[allow(clippy::new_ret_no_self)]
    #[allow(clippy::if_same_then_else)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        if let Some(v) = version {
            if semver::VersionReq::parse(">=2.0.0, <2.9.0")
                .expect("valid hardcoded semver")
                .matches(&v)
            {
                Box::new(Bitaxe200::new(ip, model))
            } else if semver::VersionReq::parse(">=2.9.0")
                .expect("valid hardcoded semver")
                .matches(&v)
            {
                Box::new(Bitaxe290::new(ip, model))
            } else {
                Box::new(Bitaxe290::new(ip, model))
            }
        } else {
            Box::new(Bitaxe290::new(ip, model))
        }
    }
}

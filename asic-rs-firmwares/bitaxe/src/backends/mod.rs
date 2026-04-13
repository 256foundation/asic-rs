use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
use semver::Version;
pub use v2_0_0::Bitaxe200;
pub use v2_9_0::Bitaxe290;

pub mod v2_0_0;
pub mod v2_9_0;

pub struct Bitaxe;

impl MinerConstructor for Bitaxe {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        if version.is_some_and(|v| v >= Version::new(2, 0, 0) && v < Version::new(2, 9, 0)) {
            Box::new(Bitaxe200::new(ip, model))
        } else {
            Box::new(Bitaxe290::new(ip, model))
        }
    }
}

use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor, Validate},
    model::MinerModel,
};
pub use v2_0_0::Bitaxe200;
pub use v2_9_0::Bitaxe290;
pub use v2_14_0::Bitaxe2140;

pub mod v2_0_0;
pub mod v2_14_0;
pub mod v2_9_0;

pub struct Bitaxe;

impl MinerConstructor for Bitaxe {
    #[allow(clippy::new_ret_no_self)]
    #[allow(clippy::if_same_then_else)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        if Bitaxe200::validate(version.as_ref()) {
            Box::new(Bitaxe200::new(ip, model))
        } else if Bitaxe290::validate(version.as_ref()) {
            Box::new(Bitaxe290::new(ip, model))
        } else if Bitaxe2140::validate(version.as_ref()) {
            Box::new(Bitaxe2140::new(ip, model))
        } else {
            Box::new(Bitaxe290::new(ip, model))
        }
    }
}

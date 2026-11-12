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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_version_boundaries() -> Result<(), semver::Error> {
        assert!(!BraiinsV2109::validate(None));
        assert!(!BraiinsV2503::validate(None));
        assert!(!BraiinsV2505::validate(None));
        assert!(!BraiinsV2507::validate(None));
        assert!(!BraiinsV2604::validate(None));

        for (version, expected) in [
            ("0.0.0", [true, false, false, false, false]),
            ("25.3.0-rc.1", [true, false, false, false, false]),
            ("25.3.0", [false, true, false, false, false]),
            ("25.3.1", [false, true, false, false, false]),
            ("25.5.0-rc.1", [false, true, false, false, false]),
            ("25.5.0", [false, false, true, false, false]),
            ("25.5.1", [false, false, true, false, false]),
            ("25.7.0-rc.1", [false, false, true, false, false]),
            ("25.7.0", [false, false, false, true, false]),
            ("25.7.1", [false, false, false, true, false]),
            ("26.4.0-rc.1", [false, false, false, true, false]),
            ("26.4.0", [false, false, false, false, true]),
            ("26.4.1", [false, false, false, false, true]),
        ] {
            let version = semver::Version::parse(version)?;
            assert_eq!(
                [
                    BraiinsV2109::validate(Some(&version)),
                    BraiinsV2503::validate(Some(&version)),
                    BraiinsV2505::validate(Some(&version)),
                    BraiinsV2507::validate(Some(&version)),
                    BraiinsV2604::validate(Some(&version)),
                ],
                expected,
                "{version}"
            );
        }
        Ok(())
    }
}

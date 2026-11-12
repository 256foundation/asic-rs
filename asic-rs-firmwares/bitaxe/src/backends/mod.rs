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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_version_boundaries() -> Result<(), semver::Error> {
        assert!(!Bitaxe200::validate(None));
        assert!(!Bitaxe290::validate(None));
        assert!(!Bitaxe2140::validate(None));

        for (version, expected) in [
            ("0.0.0", [false, false, false]),
            ("2.0.0-rc.1", [false, false, false]),
            ("2.0.0", [true, false, false]),
            ("2.0.1", [true, false, false]),
            ("2.9.0-rc.1", [true, false, false]),
            ("2.9.0", [false, true, false]),
            ("2.9.1", [false, true, false]),
            ("2.14.0-rc.1", [false, true, false]),
            ("2.14.0", [false, false, true]),
            ("2.14.1", [false, false, true]),
        ] {
            let version = semver::Version::parse(version)?;
            assert_eq!(
                [
                    Bitaxe200::validate(Some(&version)),
                    Bitaxe290::validate(Some(&version)),
                    Bitaxe2140::validate(Some(&version)),
                ],
                expected,
                "{version}"
            );
        }
        Ok(())
    }
}

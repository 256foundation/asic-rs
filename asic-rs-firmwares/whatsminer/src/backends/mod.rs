use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor, Validate},
    model::MinerModel,
};
pub use v1::WhatsMinerV1;
pub use v2::WhatsMinerV2;
pub use v3::WhatsMinerV3;

pub mod v1;
pub mod v2;
pub mod v3;

pub struct WhatsMiner;

impl MinerConstructor for WhatsMiner {
    #[allow(clippy::new_ret_no_self)]
    #[allow(clippy::if_same_then_else)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        if WhatsMinerV3::validate(version.as_ref()) {
            Box::new(WhatsMinerV3::new(ip, model))
        } else if WhatsMinerV2::validate(version.as_ref()) {
            Box::new(WhatsMinerV2::new(ip, model))
        } else if WhatsMinerV1::validate(version.as_ref()) {
            Box::new(WhatsMinerV1::new(ip, model))
        } else {
            Box::new(WhatsMinerV1::new(ip, model))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_version_boundaries() -> Result<(), semver::Error> {
        assert!(!WhatsMinerV1::validate(None));
        assert!(!WhatsMinerV2::validate(None));
        assert!(!WhatsMinerV3::validate(None));

        for (version, expected) in [
            ("0.0.0", [true, false, false]),
            ("2022.7.28", [true, false, false]),
            ("2022.7.29", [false, true, false]),
            ("2022.7.30", [false, true, false]),
            ("2024.11.0-rc.1", [false, true, false]),
            ("2024.11.0", [false, false, true]),
            ("2024.11.1", [false, false, true]),
        ] {
            let version = semver::Version::parse(version)?;
            assert_eq!(
                [
                    WhatsMinerV1::validate(Some(&version)),
                    WhatsMinerV2::validate(Some(&version)),
                    WhatsMinerV3::validate(Some(&version)),
                ],
                expected,
                "{version}"
            );
        }
        Ok(())
    }
}

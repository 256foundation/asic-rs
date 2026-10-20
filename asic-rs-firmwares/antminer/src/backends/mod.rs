pub mod v2018;
pub mod v2020;
pub mod v2023_07;

use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor, Validate},
    model::MinerModel,
};
use v2018::AntMinerV2018;
use v2020::AntMinerV2020;
use v2023_07::AntMinerV202307;

pub struct AntMiner;

impl MinerConstructor for AntMiner {
    #[allow(clippy::new_ret_no_self)]
    #[allow(clippy::if_same_then_else)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        if AntMinerV2018::validate(version.as_ref()) {
            Box::new(AntMinerV2018::new(ip, model))
        } else if AntMinerV2020::validate(version.as_ref()) {
            Box::new(AntMinerV2020::new(ip, model))
        } else if AntMinerV202307::validate(version.as_ref()) {
            Box::new(AntMinerV202307::new(ip, model))
        } else {
            Box::new(AntMinerV202307::new(ip, model))
        }
    }
}

#[cfg(test)]
mod tests {
    use asic_rs_core::traits::miner::Validate;
    use semver::Version;

    use super::{AntMinerV2018, AntMinerV2020};

    #[test]
    fn routes_build_dates_before_2021_to_v2018() {
        for version in [
            Version::new(2017, 12, 31),
            Version::new(2018, 1, 1),
            Version::new(2019, 8, 26),
            Version::new(2020, 7, 3),
            Version::new(2020, 12, 14),
            Version::new(2020, 12, 31),
        ] {
            assert!(AntMinerV2018::validate(Some(&version)));
        }

        assert!(!AntMinerV2018::validate(Some(&Version::new(2021, 1, 1))));
        assert!(!AntMinerV2018::validate(None));
    }

    #[test]
    fn routes_2021_through_june_2023_to_v2020() {
        assert!(!AntMinerV2020::validate(Some(&Version::new(2020, 12, 31))));
        assert!(AntMinerV2020::validate(Some(&Version::new(2021, 1, 1))));
        assert!(AntMinerV2020::validate(Some(&Version::new(2023, 6, 30))));
        assert!(!AntMinerV2020::validate(Some(&Version::new(2023, 7, 1))));
        assert!(!AntMinerV2020::validate(None));
    }
}

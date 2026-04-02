use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
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
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        if let Some(v) = version {
            if semver::VersionReq::parse(">=2024.11.0")
                .expect("valid hardcoded semver")
                .matches(&v)
            {
                Box::new(WhatsMinerV3::new(ip, model))
            } else if semver::VersionReq::parse(">= 2022.7.29")
                .expect("valid hardcoded semver")
                .matches(&v)
            {
                Box::new(WhatsMinerV2::new(ip, model))
            } else {
                Box::new(WhatsMinerV1::new(ip, model))
            }
        } else {
            Box::new(WhatsMinerV1::new(ip, model))
        }
    }
}

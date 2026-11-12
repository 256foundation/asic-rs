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

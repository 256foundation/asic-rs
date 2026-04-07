pub mod avalon_a;
pub mod avalon_q;

use std::{any::Any, net::IpAddr};

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor},
    model::MinerModel,
};
use asic_rs_makes_avalon::models::AvalonMinerModel;
pub use avalon_a::AvalonAMiner;
pub use avalon_q::AvalonQMiner;

pub struct AvalonMiner;

impl MinerConstructor for AvalonMiner {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: impl MinerModel, _: Option<semver::Version>) -> Box<dyn Miner> {
        let avalon_model = (&model as &dyn Any)
            .downcast_ref::<AvalonMinerModel>()
            .cloned();
        match avalon_model {
            Some(AvalonMinerModel::AvalonHomeQ) => Box::new(AvalonQMiner::new(ip, model)),
            _ => Box::new(AvalonAMiner::new(ip, model)),
        }
    }
}

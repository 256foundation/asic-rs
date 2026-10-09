use asic_rs_core::data::device::MinerHardware;

use crate::models::GoldshellModel;

impl From<GoldshellModel> for MinerHardware {
    fn from(_: GoldshellModel) -> Self {
        Self::default()
    }
}

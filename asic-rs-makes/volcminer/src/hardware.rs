use asic_rs_core::data::device::MinerHardware;

use crate::models::VolcMinerModel;

impl From<VolcMinerModel> for MinerHardware {
    fn from(value: VolcMinerModel) -> Self {
        match value {
            VolcMinerModel::D1 => Self {
                fans: Some(4),
                boards: None,
            },
            VolcMinerModel::Unknown(_) => Default::default(),
        }
    }
}

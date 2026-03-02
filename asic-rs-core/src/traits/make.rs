use crate::data::board::MinerControlBoard;
use crate::errors::ModelSelectionError;
use crate::traits::model::MinerModel;

pub trait MinerMake: ToString {
    type Model: MinerModel;
    fn parse_model(model: String) -> Result<Self::Model, ModelSelectionError>;
    #[allow(unused_variables)]
    fn parse_control_board(&self, cb_type: &str) -> Option<MinerControlBoard> {
        None
    }
}

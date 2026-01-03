use crate::data::device::models::{MinerModelFactory, ModelSelectionError};
use crate::data::device::{MinerMake, MinerModel};
use crate::miners::util;
use std::net::IpAddr;

pub(crate) async fn get_model_avalonminer(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
    let response = util::send_rpc_command(&ip, "version").await;

    match response {
        Some(json_data) => {
            let model = json_data["VERSION"][0]["MODEL"].as_str();
            if model.is_none() {
                return Err(ModelSelectionError::UnexpectedModelResponse);
            }
            let model = model.unwrap().split("-").collect::<Vec<&str>>()[0].to_uppercase();

            MinerModelFactory::new()
                .with_make(MinerMake::AvalonMiner)
                .parse_model(&model)
        }
        None => Err(ModelSelectionError::NoModelResponse),
    }
}

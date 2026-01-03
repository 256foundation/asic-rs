use crate::data::device::models::{MinerModelFactory, ModelSelectionError};
use crate::data::device::{MinerFirmware, MinerModel};
use crate::miners::util;
use std::net::IpAddr;

pub(crate) async fn get_model_marathon(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
    let response = util::send_rpc_command(&ip, "version").await;

    match response {
        Some(json_data) => {
            let model: Option<&str> = json_data["VERSION"][0]["Model"].as_str();
            if model.is_none() {
                return Err(ModelSelectionError::UnexpectedModelResponse);
            }

            let model = model.unwrap().to_uppercase();

            MinerModelFactory::new()
                .with_firmware(MinerFirmware::Marathon)
                .parse_model(&model)
        }
        None => Err(ModelSelectionError::NoModelResponse),
    }
}

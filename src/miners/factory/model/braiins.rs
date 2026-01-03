use crate::data::device::models::{MinerModelFactory, ModelSelectionError};
use crate::data::device::{MinerFirmware, MinerModel};
use crate::miners::util;
use std::net::IpAddr;

pub(crate) async fn get_model_braiins_os(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
    let response = util::send_rpc_command(&ip, "devdetails").await;
    match response {
        Some(json_data) => {
            let model = json_data["DEVDETAILS"][0]["Model"].as_str();
            if model.is_none() {
                return Err(ModelSelectionError::UnexpectedModelResponse);
            }
            let model = model
                .unwrap()
                .to_uppercase()
                .replace("BITMAIN ", "")
                .replace("S19XP", "S19 XP");

            MinerModelFactory::new()
                .with_firmware(MinerFirmware::BraiinsOS)
                .parse_model(&model)
        }
        None => Err(ModelSelectionError::NoModelResponse),
    }
}

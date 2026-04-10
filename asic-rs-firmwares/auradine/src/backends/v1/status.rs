use asic_rs_core::{data::command::RPCCommandStatus, errors::RPCError};
use serde_json::Value;

pub(crate) trait StatusFromAuradineV1 {
    fn status_from_auradine_v1(&self) -> Result<RPCCommandStatus, RPCError>;
}

impl StatusFromAuradineV1 for Value {
    fn status_from_auradine_v1(&self) -> Result<RPCCommandStatus, RPCError> {
        if let Some(status_array) = self.get("STATUS").and_then(|v| v.as_array())
            && let Some(status_obj) = status_array.first()
            && let Some(status) = status_obj.get("STATUS").and_then(|v| v.as_str())
        {
            let message = status_obj.get("Msg").and_then(|v| v.as_str());
            return Ok(RPCCommandStatus::from_str(status, message));
        }

        if let Some(status) = self.get("STATUS").and_then(|v| v.as_str()) {
            return Ok(RPCCommandStatus::from_str(status, None));
        }

        Ok(RPCCommandStatus::Success)
    }
}

use crate::errors::RPCError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MinerCommand {
    RPC {
        command: &'static str,
        parameters: Option<Value>,
    },
    GRPC {
        command: &'static str,
    },
    WebAPI {
        command: &'static str,
        parameters: Option<Value>,
    },
    GraphQL {
        command: &'static str,
    },
    SSH {
        command: &'static str,
    },
}

#[derive(Debug)]
pub enum RPCCommandStatus {
    Success,
    Information,
    Error(String),
    Unknown,
}

impl RPCCommandStatus {
    pub fn into_result(self) -> Result<(), RPCError> {
        match self {
            RPCCommandStatus::Success => Ok(()),
            RPCCommandStatus::Information => Ok(()),
            RPCCommandStatus::Error(msg) => Err(RPCError::StatusCheckFailed(msg)),
            RPCCommandStatus::Unknown => {
                Err(RPCError::StatusCheckFailed(String::from("Unknown status")))
            }
        }
    }

    pub fn from_str(response: &str, message: Option<&str>) -> Self {
        match response {
            "S" => RPCCommandStatus::Success,
            "I" => RPCCommandStatus::Information,
            "E" => RPCCommandStatus::Error(message.unwrap_or("Unknown error").to_string()),
            _ => RPCCommandStatus::Unknown,
        }
    }
}

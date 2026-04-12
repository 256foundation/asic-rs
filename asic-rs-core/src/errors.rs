use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum ModelSelectionError {
    NoModelResponse,
    UnexpectedModelResponse,
}

impl Display for ModelSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelSelectionError::NoModelResponse => write!(f, "No response when querying model"),
            ModelSelectionError::UnexpectedModelResponse => {
                write!(f, "Response to model query was formatted unexpectedly")
            }
        }
    }
}

impl std::error::Error for ModelSelectionError {}

#[derive(Debug)]
pub enum RPCError {
    StatusCheckFailed(String),
    DeserializationFailed(serde_json::Error),
    ConnectionFailed,
    ConnectionTimeout,
    ReadTimeout,
    WriteTimeout,
    ConnectionReset,
    BrokenPipe,
}

impl RPCError {
    /// Returns true if this error represents an expected transient failure
    /// from a privileged write command (timeout or connection drop).
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::ConnectionTimeout
                | Self::ReadTimeout
                | Self::WriteTimeout
                | Self::ConnectionReset
                | Self::BrokenPipe
        )
    }
}

impl Display for RPCError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RPCError::StatusCheckFailed(message) => {
                write!(f, "Command returned with error status: {message}")
            }
            RPCError::DeserializationFailed(error) => {
                write!(f, "Failed to deserialize result: {error}")
            }
            RPCError::ConnectionFailed => {
                write!(f, "Failed to connect to RPC API")
            }
            RPCError::ConnectionTimeout => {
                write!(f, "RPC connect timed out")
            }
            RPCError::ReadTimeout => {
                write!(f, "RPC read timed out")
            }
            RPCError::WriteTimeout => {
                write!(f, "RPC write timed out")
            }
            RPCError::ConnectionReset => {
                write!(f, "Connection reset by miner")
            }
            RPCError::BrokenPipe => {
                write!(f, "Broken pipe (miner closed connection)")
            }
        }
    }
}

impl std::error::Error for RPCError {}

impl From<serde_json::Error> for RPCError {
    fn from(value: serde_json::Error) -> Self {
        Self::DeserializationFailed(value)
    }
}

impl From<std::io::Error> for RPCError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::ConnectionReset => Self::ConnectionReset,
            std::io::ErrorKind::BrokenPipe => Self::BrokenPipe,
            _ => Self::ConnectionFailed,
        }
    }
}

#[cfg(feature = "python")]
use pyo3::prelude::*;

use serde::{Deserialize, Serialize};
use strum::Display;

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum MessageSeverity {
    Error,
    Warning,
    Info,
}

#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinerMessage {
    /// The time this message was generated or occurred
    pub timestamp: u32,
    /// The message code
    /// May be set to 0 if no code is set by the device
    pub code: u64,
    /// The human-readable message being relayed by the device
    pub message: String,
    /// The severity of this message
    pub severity: MessageSeverity,
}

impl MinerMessage {
    pub fn new(timestamp: u32, code: u64, message: String, severity: MessageSeverity) -> Self {
        Self {
            timestamp,
            code,
            message,
            severity,
        }
    }
}

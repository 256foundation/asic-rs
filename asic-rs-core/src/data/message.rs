use std::fmt::Formatter;

#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[cfg_attr(feature = "python", derive(asic_rs_pydantic::PyPydanticEnum))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
/// Severity for a miner-reported message.
pub enum MessageSeverity {
    /// Error condition.
    #[cfg_attr(feature = "python", pydantic(value = "Error"))]
    Error,
    /// Warning condition.
    #[cfg_attr(feature = "python", pydantic(value = "Warning"))]
    Warning,
    /// Informational message.
    #[cfg_attr(feature = "python", pydantic(value = "Info"))]
    Info,
}

#[cfg_attr(feature = "python", pyclass(from_py_object, module = "asic_rs"))]
#[cfg_attr(feature = "python", derive(asic_rs_pydantic::PyPydanticTaggedEnum))]
#[cfg_attr(feature = "python", pydantic(discriminator = "type"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
/// Miner component referenced by an error or status message.
pub enum MinerComponent {
    /// Miner control board.
    #[serde(rename = "ControlBoard")]
    #[cfg_attr(feature = "python", pydantic(tag = "ControlBoard"))]
    ControlBoard {},
    /// Hashboard, optionally with a specific chip index.
    #[serde(rename = "HashBoard")]
    #[cfg_attr(feature = "python", pydantic(tag = "HashBoard"))]
    HashBoard {
        idx: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "python", pydantic(default = None))]
        chip_idx: Option<u16>,
    },
    /// Fan component.
    #[serde(rename = "Fan")]
    #[cfg_attr(feature = "python", pydantic(tag = "Fan"))]
    Fan { idx: u16 },
    /// Power supply component.
    #[serde(rename = "PowerSupply")]
    #[cfg_attr(feature = "python", pydantic(tag = "PowerSupply"))]
    PowerSupply { idx: u16 },
}

impl MinerComponent {
    /// Component marker for the control board.
    pub fn control_board() -> Self {
        Self::ControlBoard {}
    }

    /// Component marker for a hashboard.
    pub fn hashboard(idx: u16) -> Self {
        Self::HashBoard {
            idx,
            chip_idx: None,
        }
    }

    /// Component marker for a chip on a hashboard.
    pub fn chip(board_idx: u16, chip_idx: u16) -> Self {
        Self::HashBoard {
            idx: board_idx,
            chip_idx: Some(chip_idx),
        }
    }

    /// Component marker for a fan.
    pub fn fan(idx: u16) -> Self {
        Self::Fan { idx }
    }

    /// Component marker for a power supply.
    pub fn power_supply(idx: u16) -> Self {
        Self::PowerSupply { idx }
    }
}

impl std::fmt::Display for MinerComponent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ControlBoard {} => write!(f, "ControlBoard"),
            Self::HashBoard {
                idx,
                chip_idx: Some(chip_idx),
            } => write!(f, "HashBoard {idx}, chip {chip_idx}"),
            Self::HashBoard { idx, .. } => write!(f, "HashBoard {idx}"),
            Self::Fan { idx } => write!(f, "Fan {idx}"),
            Self::PowerSupply { idx } => write!(f, "PowerSupply {idx}"),
        }
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl MinerComponent {
    fn __repr__(&self) -> String {
        match self {
            Self::ControlBoard {} => "MinerComponent.ControlBoard".to_string(),
            Self::HashBoard {
                idx,
                chip_idx: Some(chip_idx),
            } => format!(
                "MinerComponent.HashBoard(idx={}, chip_idx={})",
                idx, chip_idx
            ),
            Self::HashBoard { idx, .. } => format!("MinerComponent.HashBoard(idx={})", idx),
            Self::Fan { idx } => format!("MinerComponent.Fan(idx={})", idx),
            Self::PowerSupply { idx } => format!("MinerComponent.PowerSupply(idx={})", idx),
        }
    }

    #[staticmethod]
    #[pyo3(name = "control_board")]
    fn py_control_board() -> Self {
        Self::control_board()
    }

    #[staticmethod]
    #[pyo3(name = "hashboard")]
    #[pyo3(signature = (idx, chip_idx=None))]
    fn py_hashboard(idx: u16, chip_idx: Option<u16>) -> Self {
        Self::HashBoard { idx, chip_idx }
    }

    #[staticmethod]
    #[pyo3(name = "chip")]
    fn py_chip(idx: u16, chip_idx: u16) -> Self {
        Self::chip(idx, chip_idx)
    }

    #[staticmethod]
    #[pyo3(name = "fan")]
    fn py_fan(idx: u16) -> Self {
        Self::fan(idx)
    }

    #[staticmethod]
    #[pyo3(name = "power_supply")]
    fn py_power_supply(idx: u16) -> Self {
        Self::power_supply(idx)
    }
}

#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Message, warning, or error reported by a miner.
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
    /// The affected or related component
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub component: Option<MinerComponent>,
}

impl MinerMessage {
    /// Create a miner message without a component.
    pub fn new(timestamp: u32, code: u64, message: String, severity: MessageSeverity) -> Self {
        Self::with_component(timestamp, code, message, severity, None)
    }

    /// Create a miner message with an optional component.
    pub fn with_component(
        timestamp: u32,
        code: u64,
        message: String,
        severity: MessageSeverity,
        component: Option<MinerComponent>,
    ) -> Self {
        Self {
            timestamp,
            code,
            message,
            severity,
            component,
        }
    }
}

#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;
use ts_rs::TS;

/// Explicit firmware-reported operating state, independent of `is_mining`.
///
/// Known states share variants across firmwares. Unrecognized labels remain
/// available in [`OperatingState::Unknown`]; missing telemetry is represented
/// by `None` in `MinerData`, never by an inferred state.
#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, eq, frozen, hash, str, module = "asic_rs")
)]
#[cfg_attr(feature = "python", derive(asic_rs_pydantic::PyPydanticTaggedEnum))]
#[cfg_attr(feature = "python", pydantic(discriminator = "type"))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, TS)]
#[serde(tag = "type")]
pub enum OperatingState {
    /// Mining is running; this alone does not imply that tuning has finished.
    #[cfg_attr(feature = "python", pydantic(tag = "Mining"))]
    Mining {},
    /// Firmware explicitly reports stable operation.
    #[cfg_attr(feature = "python", pydantic(tag = "Stable"))]
    Stable {},
    /// Hardware or mining software is being initialized.
    #[cfg_attr(feature = "python", pydantic(tag = "Initializing"))]
    Initializing {},
    /// Mining is starting or hardware is powering on.
    #[cfg_attr(feature = "python", pydantic(tag = "Starting"))]
    Starting {},
    /// Firmware is searching for suitable operating set points.
    #[cfg_attr(feature = "python", pydantic(tag = "Tuning"))]
    Tuning {},
    /// Firmware is changing clock frequencies.
    #[cfg_attr(feature = "python", pydantic(tag = "AdjustingFrequency"))]
    AdjustingFrequency {},
    /// Firmware is changing supply voltage.
    #[cfg_attr(feature = "python", pydantic(tag = "AdjustingVoltage"))]
    AdjustingVoltage {},
    /// Firmware is adjusting clock frequency and voltage together (ePIC/UMC).
    #[cfg_attr(feature = "python", pydantic(tag = "AdjustingClockVoltage"))]
    AdjustingClockVoltage {},
    /// Miner is idle, without a reported reason for the inactivity.
    #[cfg_attr(feature = "python", pydantic(tag = "Idling"))]
    Idling {},
    /// Mining is explicitly paused.
    #[cfg_attr(feature = "python", pydantic(tag = "Paused"))]
    Paused {},
    /// Firmware has suspended operation.
    #[cfg_attr(feature = "python", pydantic(tag = "Suspended"))]
    Suspended {},
    /// Firmware restricts operation (Braiins).
    #[cfg_attr(feature = "python", pydantic(tag = "Restricted"))]
    Restricted {},
    /// Mining is stopping or hardware is powering off.
    #[cfg_attr(feature = "python", pydantic(tag = "Stopping"))]
    Stopping {},
    /// Mining is stopped or has not started.
    #[cfg_attr(feature = "python", pydantic(tag = "Stopped"))]
    Stopped {},
    /// Mining software or the miner is restarting.
    #[cfg_attr(feature = "python", pydantic(tag = "Restarting"))]
    Restarting {},
    /// Firmware is waiting for hardware to cool down.
    #[cfg_attr(feature = "python", pydantic(tag = "CoolingDown"))]
    CoolingDown {},
    /// Mining continues with reduced functionality (Proto).
    #[cfg_attr(feature = "python", pydantic(tag = "DegradedMining"))]
    DegradedMining {},
    /// Firmware reports an error state; details remain in miner messages.
    #[cfg_attr(feature = "python", pydantic(tag = "Error"))]
    Error {},
    /// Unrecognized firmware label or numeric status code, preserved verbatim.
    #[cfg_attr(feature = "python", pydantic(tag = "Unknown"))]
    #[strum(to_string = "{raw}")]
    Unknown { raw: String },
}

impl OperatingState {
    /// Parse an explicit textual state. Empty labels are unavailable, and
    /// unfamiliar labels are retained without changing their spelling.
    pub fn from_label(label: &str) -> Option<Self> {
        if label.trim().is_empty() {
            return None;
        }
        Some(match label.to_ascii_lowercase().as_str() {
            "mining" => Self::Mining {},
            "stable" => Self::Stable {},
            "initializing" => Self::Initializing {},
            "starting" => Self::Starting {},
            "tuning" => Self::Tuning {},
            "adjustingfrequency" => Self::AdjustingFrequency {},
            "adjustingvoltage" => Self::AdjustingVoltage {},
            "adjustingclockvoltage" => Self::AdjustingClockVoltage {},
            "idling" => Self::Idling {},
            "paused" => Self::Paused {},
            "suspended" => Self::Suspended {},
            "restricted" => Self::Restricted {},
            "stopping" => Self::Stopping {},
            "stopped" => Self::Stopped {},
            "restarting" => Self::Restarting {},
            "coolingdown" => Self::CoolingDown {},
            "degradedmining" => Self::DegradedMining {},
            "error" => Self::Error {},
            _ => Self::Unknown {
                raw: label.to_owned(),
            },
        })
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl OperatingState {
    fn __repr__(&self) -> String {
        match self {
            Self::Unknown { raw } => format!("OperatingState.Unknown(raw={raw:?})"),
            _ => format!("OperatingState.{self}()"),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use super::*;

    #[test]
    fn state_tags_and_unknown_labels_survive_serialization() -> anyhow::Result<()> {
        for label in [
            "Mining",
            "Stable",
            "Initializing",
            "Starting",
            "Tuning",
            "AdjustingFrequency",
            "AdjustingVoltage",
            "AdjustingClockVoltage",
            "Idling",
            "Paused",
            "Suspended",
            "Restricted",
            "Stopping",
            "Stopped",
            "Restarting",
            "CoolingDown",
            "DegradedMining",
            "Error",
        ] {
            let state = OperatingState::from_label(label).context("state is available")?;
            let json = serde_json::to_value(&state)?;
            assert_eq!(json, serde_json::json!({ "type": label }));
            assert_eq!(serde_json::from_value::<OperatingState>(json)?, state);
            assert_eq!(
                OperatingState::from_label(&label.to_lowercase()),
                Some(state)
            );
        }

        let raw = "FutureState / voltage=12.6";
        let unknown = OperatingState::from_label(raw).context("unknown state is retained")?;
        let json = serde_json::to_value(&unknown)?;
        assert_eq!(json, serde_json::json!({ "type": "Unknown", "raw": raw }));
        assert_eq!(serde_json::from_value::<OperatingState>(json)?, unknown);
        assert_eq!(unknown.to_string(), raw);
        assert_eq!(OperatingState::from_label(""), None);
        assert_eq!(OperatingState::from_label("  \t\n"), None);
        Ok(())
    }

    #[test]
    fn typescript_retains_the_unknown_label_and_optional_snapshot_state() {
        let config = ts_rs::Config::default();
        let state = OperatingState::decl(&config);
        assert!(state.contains("\"Mining\""));
        assert!(state.contains("\"Unknown\""));
        assert!(state.contains("raw: string"));
        let miner = crate::data::miner::MinerData::decl(&config);
        assert!(miner.contains("operating_state: OperatingState | null"));
    }
}

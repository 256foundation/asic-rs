#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

/// An available autotune/overclock preset reported by the firmware.
///
/// Read-only: produced by the library (e.g. `Miner.get_presets`), never taken
/// as input from Python, so it is a plain `pyclass` rather than a pydantic model.
#[cfg_attr(
    feature = "python",
    pyclass(
        name = "PresetInfo",
        frozen,
        get_all,
        skip_from_py_object,
        module = "asic_rs"
    )
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetInfo {
    /// Canonical preset name the firmware expects (e.g. `"5560"`).
    pub name: String,
    /// Human-readable description (e.g. `"5560 watt ~ 175 TH"`), if provided.
    pub pretty: Option<String>,
    /// Tuning status (e.g. `"tuned"` / `"untuned"`), if provided.
    pub status: Option<String>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PresetInfo {
    fn __repr__(&self) -> String {
        format!(
            "PresetInfo(name={:?}, pretty={:?}, status={:?})",
            self.name, self.pretty, self.status
        )
    }
}

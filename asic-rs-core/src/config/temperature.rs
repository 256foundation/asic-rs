#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(
    feature = "python",
    pyclass(
        name = "TemperatureConfig",
        from_py_object,
        get_all,
        module = "asic_rs"
    )
)]
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(new, name = "TemperatureConfig")
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Configured thermal limits of the miner (°C).
///
/// Mirrors the Braiins OS layout. Fields are optional because not every
/// firmware reports every threshold (e.g. VNish exposes `minimum` and `danger`
/// but not `hot`). The target chip temperature lives on `FanConfig`.
pub struct TemperatureConfig {
    /// The temperature at which the fans run at 100%.
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub hot: Option<f64>,
    /// The temperature at/above which the miner reboots or triggers thermal scaling.
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub danger: Option<f64>,
    /// The minimum temperature the miner must preheat to before it will start.
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub minimum: Option<f64>,
}

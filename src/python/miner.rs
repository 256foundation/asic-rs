use std::{net::IpAddr, path::PathBuf, sync::Arc, time::Duration};

use super::typing::{PyAwaitable, future_into_py};
use asic_rs_core::data::collector::DataField;
use asic_rs_core::{
    config::{
        fan::FanConfig, pools::PoolGroupConfig as PoolGroup, preset::PresetInfo,
        scaling::ScalingConfig, temperature::TemperatureConfig, tuning::TuningConfig,
    },
    data::{
        board::BoardData,
        device::{HashAlgorithm, MinerHardware},
        fan::FanData,
        firmware::FirmwareImage,
        hashrate::HashRate,
        message::MinerMessage,
        miner::{MinerData, TuningTarget},
        pool::PoolGroupData,
    },
    traits::{auth::MinerAuth, miner::Miner as MinerTrait},
};
use measurements::Power;
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
};
use tokio::sync::RwLock;

/// Python handle for one supported ASIC miner.
///
/// A `Miner` is returned by `MinerFactory.get_miner`, `scan`, or scan streams.
/// Data and control methods are awaitable. Capability properties named
/// `supports_*` describe whether the current miner/firmware supports a control
/// or config operation before it is called.
#[pyclass(module = "asic_rs")]
pub(crate) struct Miner {
    inner: Arc<RwLock<Box<dyn MinerTrait>>>,
}

impl Miner {
    pub fn new(inner: Box<dyn MinerTrait>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    fn with_miner<T>(&self, py: Python<'_>, f: impl FnOnce(&dyn MinerTrait) -> T + Send) -> T
    where
        T: Send,
    {
        py.detach(|| {
            let inner = self.inner.blocking_read();
            f(inner.as_ref())
        })
    }

    fn apply_auth(&mut self, auth: MinerAuth) -> PyResult<()> {
        Arc::get_mut(&mut self.inner)
            .ok_or_else(|| PyRuntimeError::new_err("cannot set auth while miner is in use"))?
            .get_mut()
            .set_auth(auth);
        Ok(())
    }
}

impl From<Box<dyn MinerTrait>> for Miner {
    fn from(inner: Box<dyn MinerTrait>) -> Self {
        Self::new(inner)
    }
}

fn parse_optional_duration(value: Option<&Bound<'_, PyAny>>) -> PyResult<Option<Duration>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    if let Ok(duration) = value.extract::<Duration>() {
        return Ok(Some(duration));
    }
    if let Ok(seconds) = value.extract::<f64>()
        && seconds.is_finite()
        && seconds >= 0.0
    {
        return Ok(Some(Duration::from_secs_f64(seconds)));
    }
    Err(PyValueError::new_err(
        "expected datetime.timedelta, non-negative seconds, or None",
    ))
}

pub(crate) struct FirmwarePath(PathBuf);

impl<'a, 'py> pyo3::FromPyObject<'a, 'py> for FirmwarePath {
    type Error = pyo3::PyErr;

    const INPUT_TYPE: pyo3::inspect::PyStaticExpr =
        pyo3::type_hint_identifier!("_typeshed", "StrOrBytesPath");

    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        obj.extract::<PathBuf>().map(Self)
    }
}

#[pymethods]
impl Miner {
    fn __repr__(&self, py: Python<'_>) -> String {
        self.with_miner(py, |miner| {
            let device_info = miner.get_device_info();
            format!(
                "{} {} ({}): {}",
                device_info.make,
                device_info.model,
                device_info.firmware,
                miner.get_ip(),
            )
        })
    }

    /// IP address of this miner.
    #[getter]
    fn ip(&self, py: Python<'_>) -> IpAddr {
        self.with_miner(py, |miner| miner.get_ip())
    }

    /// Miner model name.
    #[getter]
    fn model(&self, py: Python<'_>) -> String {
        self.with_miner(py, |miner| miner.get_device_info().model)
    }
    /// Miner manufacturer or make.
    #[getter]
    fn make(&self, py: Python<'_>) -> String {
        self.with_miner(py, |miner| miner.get_device_info().make)
    }
    /// Firmware name or family used by this miner.
    #[getter]
    fn firmware(&self, py: Python<'_>) -> String {
        self.with_miner(py, |miner| miner.get_device_info().firmware)
    }
    /// Hash algorithm mined by this device.
    #[getter]
    fn algo(&self, py: Python<'_>) -> HashAlgorithm {
        self.with_miner(py, |miner| miner.get_device_info().algo)
    }
    /// Expected hardware shape for this miner model.
    #[getter]
    fn hardware(&self, py: Python<'_>) -> MinerHardware {
        self.with_miner(py, |miner| miner.get_device_info().hardware)
    }

    /// Expected number of hashboards, when known.
    #[getter]
    fn expected_hashboards(&self, py: Python<'_>) -> Option<u8> {
        self.with_miner(py, |miner| miner.get_expected_hashboards())
    }

    /// Expected total number of chips, when known.
    #[getter]
    fn expected_chips(&self, py: Python<'_>) -> Option<u16> {
        self.with_miner(py, |miner| miner.get_expected_chips())
    }

    /// Expected number of fans, when known.
    #[getter]
    fn expected_fans(&self, py: Python<'_>) -> Option<u8> {
        self.with_miner(py, |miner| miner.get_expected_fans())
    }

    /// Whether this miner supports changing the fault light state.
    #[getter]
    fn supports_set_fault_light(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_set_fault_light())
    }
    /// Whether this miner supports setting a power limit.
    #[getter]
    fn supports_set_power_limit(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_set_power_limit())
    }
    /// Whether this miner supports setting a manual tuning percent.
    #[getter]
    fn supports_set_tuning_percent(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_set_tuning_percent())
    }
    /// Whether this miner exposes named autotune/overclock presets.
    #[getter]
    fn supports_presets(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_presets())
    }
    /// Whether this miner supports restart commands.
    #[getter]
    fn supports_restart(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_restart())
    }
    /// Whether this miner supports pause commands.
    #[getter]
    fn supports_pause(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_pause())
    }
    /// Whether this miner supports resume commands.
    #[getter]
    fn supports_resume(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_resume())
    }
    /// Whether this miner supports changing its password.
    #[getter]
    fn supports_change_password(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_change_password())
    }
    /// Whether this miner supports reading logs.
    #[getter]
    fn supports_read_logs(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_read_logs())
    }
    /// Whether this miner supports factory reset.
    #[getter]
    fn supports_factory_reset(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_factory_reset())
    }
    /// Whether this miner supports reading and writing pool configuration.
    #[getter]
    fn supports_pools_config(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_pools_config())
    }
    /// Whether this miner supports firmware upgrades through the API.
    #[getter]
    fn supports_upgrade_firmware(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_upgrade_firmware())
    }
    /// Whether this miner supports scaling configuration.
    #[getter]
    fn supports_scaling_config(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_scaling_config())
    }
    /// Whether this miner reports configured thermal limits.
    #[getter]
    fn supports_temperature_config(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_temperature_config())
    }
    /// Whether this miner supports tuning configuration.
    #[getter]
    fn supports_tuning_config(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_tuning_config())
    }
    /// Whether this miner supports fan configuration.
    #[getter]
    fn supports_fan_config(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_fan_config())
    }
    /// Set username/password credentials used by subsequent operations on this
    /// miner.
    ///
    /// Call this before starting concurrent operations. It raises `RuntimeError`
    /// if the miner handle is already shared by an active async operation.
    pub fn set_auth(&mut self, username: String, password: String) -> PyResult<()> {
        self.apply_auth(MinerAuth::new(username, password))
    }

    /// Set a pre-issued bearer token for firmwares that accept one (e.g. VNish);
    /// backends that support it use the token instead of logging in with a
    /// password.
    ///
    /// Call this before starting concurrent operations. It raises `RuntimeError`
    /// if the miner handle is already shared by an active async operation.
    pub fn set_token(&mut self, token: String) -> PyResult<()> {
        self.apply_auth(MinerAuth::from_token(token))
    }

    // Data functions
    /// Await a full telemetry snapshot.
    ///
    /// Pass `exclude=[DataField.SomeField]` to skip expensive or unnecessary
    /// fields during collection.
    #[pyo3(signature = (exclude: "list[DataField] | None" = None))]
    pub fn get_data<'a>(
        &self,
        py: Python<'a>,
        exclude: Option<Vec<DataField>>,
    ) -> PyResult<PyAwaitable<MinerData>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            match exclude {
                None => Ok(inner.get_data().await),
                Some(excl) => Ok(inner.get_data_filtered(excl).await),
            }
        })
    }
    /// Await the miner MAC address, if exposed by the firmware.
    pub fn get_mac<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_mac().await;
            Ok(data.map(|m| m.to_string()))
        })
    }
    /// Await the miner serial number, if exposed by the firmware.
    pub fn get_serial_number<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_serial_number().await;
            Ok(data)
        })
    }
    /// Await the network hostname, if exposed by the firmware.
    pub fn get_hostname<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_hostname().await;
            Ok(data)
        })
    }
    /// Await the miner API version, if exposed by the firmware.
    pub fn get_api_version<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_api_version().await;
            Ok(data)
        })
    }
    /// Await the firmware version string, if exposed by the firmware.
    pub fn get_firmware_version<'a>(
        &self,
        py: Python<'a>,
    ) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_firmware_version().await;
            Ok(data)
        })
    }
    /// Await the control board version or name, if exposed by the firmware.
    pub fn get_control_board_version<'a>(
        &self,
        py: Python<'a>,
    ) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner
                .get_control_board_version()
                .await
                .map(|cb| cb.to_string());
            Ok(data)
        })
    }
    /// Await per-hashboard data including chip details where available.
    pub fn get_hashboards<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<BoardData>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_hashboards().await)
        })
    }
    /// Await per-hashboard data without collecting per-chip details.
    pub fn get_hashboards_no_chips<'a>(
        &self,
        py: Python<'a>,
    ) -> PyResult<PyAwaitable<Vec<BoardData>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_hashboards_no_chips().await)
        })
    }
    /// Await the current hashrate, if exposed by the firmware.
    pub fn get_hashrate<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<HashRate>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_hashrate().await;
            Ok(data)
        })
    }
    /// Await the expected or nominal hashrate, if known.
    pub fn get_expected_hashrate<'a>(
        &self,
        py: Python<'a>,
    ) -> PyResult<PyAwaitable<Option<HashRate>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_expected_hashrate().await;
            Ok(data)
        })
    }
    /// Await current fan readings.
    pub fn get_fans<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<FanData>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_fans().await)
        })
    }
    /// Await power-supply fan readings, if exposed separately.
    pub fn get_psu_fans<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<FanData>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_psu_fans().await)
        })
    }
    /// Await fluid or ambient temperature in Celsius, if available.
    pub fn get_fluid_temperature<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<f64>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_fluid_temperature().await;
            Ok(data.map(|t| t.as_celsius()))
        })
    }
    /// Await coolant exhaust (outlet fluid) temperature in Celsius, if available.
    pub fn get_outlet_fluid_temperature<'a>(
        &self,
        py: Python<'a>,
    ) -> PyResult<PyAwaitable<Option<f64>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_outlet_fluid_temperature().await;
            Ok(data.map(|t| t.as_celsius()))
        })
    }
    /// Await current power draw in watts, if available.
    pub fn get_wattage<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<f64>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_wattage().await;
            Ok(data.map(|w| w.as_watts()))
        })
    }
    /// Await the current manual throttle percent (100 = unthrottled), if exposed.
    pub fn get_tuning_percent<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<u8>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_tuning_percent().await)
        })
    }
    /// Await the active tuning target, if exposed by the firmware.
    pub fn get_tuning_target<'a>(
        &self,
        py: Python<'a>,
    ) -> PyResult<PyAwaitable<Option<TuningTarget>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_tuning_target().await)
        })
    }
    /// Await the current fault light state, if available.
    pub fn get_light_flashing<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_light_flashing().await;
            Ok(data)
        })
    }
    /// Await current miner messages and errors.
    pub fn get_messages<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<MinerMessage>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_messages().await)
        })
    }
    /// Await system uptime as `datetime.timedelta`, if available.
    pub fn get_uptime<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<Duration>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_uptime().await;
            Ok(data)
        })
    }
    /// Await whether the miner is currently mining.
    pub fn get_is_mining<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<bool>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_is_mining().await;
            Ok(data)
        })
    }
    /// Await the current mining pool status.
    pub fn get_pools<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<PoolGroupData>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_pools().await)
        })
    }

    /// Await configured pool groups, or `None` when unsupported/unavailable.
    pub fn get_pools_config<'a>(
        &self,
        py: Python<'a>,
    ) -> PyResult<PyAwaitable<Option<Vec<PoolGroup>>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_pools_config().await.ok())
        })
    }
    /// Await scaling configuration, or `None` when unsupported/unavailable.
    pub fn get_scaling_config<'a>(
        &self,
        py: Python<'a>,
    ) -> PyResult<PyAwaitable<Option<ScalingConfig>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_scaling_config().await.ok())
        })
    }
    /// Await the configured thermal limits, or `None` when unsupported/unavailable.
    pub fn get_temperature_config<'a>(
        &self,
        py: Python<'a>,
    ) -> PyResult<PyAwaitable<Option<TemperatureConfig>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_temperature_config().await.ok())
        })
    }
    /// Await tuning configuration, or `None` when unsupported/unavailable.
    pub fn get_tuning_config<'a>(
        &self,
        py: Python<'a>,
    ) -> PyResult<PyAwaitable<Option<TuningConfig>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_tuning_config().await.ok())
        })
    }
    /// Await fan configuration, or `None` when unsupported/unavailable.
    pub fn get_fan_config<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<FanConfig>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_fan_config().await.ok())
        })
    }

    // Control functions
    /// Set the fault light state.
    ///
    /// Returns `None` if the command is unsupported or rejected by the backend.
    pub fn set_fault_light<'a>(
        &self,
        py: Python<'a>,
        fault: bool,
    ) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.set_fault_light(fault).await;
            Ok(data.ok())
        })
    }
    /// Restart the miner.
    ///
    /// Returns `None` if restart is unsupported or rejected by the backend.
    pub fn restart<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.restart().await;
            Ok(data.ok())
        })
    }
    /// Pause mining immediately or after a delay.
    ///
    /// `at_time` may be `datetime.timedelta`, non-negative seconds, or `None`.
    #[pyo3(signature = (at_time: "timedelta | float | int | None" = None))]
    pub fn pause<'a>(
        &self,
        py: Python<'a>,
        at_time: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        let at_time = parse_optional_duration(at_time)?;
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.pause(at_time).await;
            Ok(data.ok())
        })
    }
    /// Resume mining immediately or after a delay.
    ///
    /// `at_time` may be `datetime.timedelta`, non-negative seconds, or `None`.
    #[pyo3(signature = (at_time: "timedelta | float | int | None" = None))]
    pub fn resume<'a>(
        &self,
        py: Python<'a>,
        at_time: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        let at_time = parse_optional_duration(at_time)?;
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.resume(at_time).await;
            Ok(data.ok())
        })
    }
    /// Factory reset the miner.
    ///
    /// Returns `None` if factory reset is unsupported or rejected by the backend.
    pub fn factory_reset<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.factory_reset().await;
            Ok(data.ok())
        })
    }
    /// Read miner logs, if supported.
    pub fn read_logs<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.read_logs().await;
            Ok(data.ok())
        })
    }
    /// Change the miner password.
    ///
    /// This may require existing credentials to have been set with `set_auth`.
    pub fn change_password<'a>(
        &self,
        py: Python<'a>,
        password: &str,
    ) -> PyResult<PyAwaitable<Option<bool>>> {
        let password = password.to_string();
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let mut inner = inner.write().await;
            Ok(inner.change_password(&password).await.ok())
        })
    }
    /// Set the power limit in watts.
    pub fn set_power_limit<'a>(
        &self,
        py: Python<'a>,
        watts: f64,
    ) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.set_power_limit(Power::from_watts(watts)).await.ok())
        })
    }
    /// Set a manual tuning percent of full power (100 = unthrottled).
    pub fn set_tuning_percent<'a>(
        &self,
        py: Python<'a>,
        percent: u8,
    ) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.set_tuning_percent(percent).await.ok())
        })
    }
    /// Await the firmware's available autotune/overclock presets.
    ///
    /// Select one with `set_tuning_config(TuningConfig.preset(name))`; the active
    /// preset is reported by `get_tuning_target()` as a `TuningTarget.preset`.
    pub fn get_presets<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<PresetInfo>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_presets().await)
        })
    }
    /// Replace the configured mining pool groups.
    #[pyo3(signature = (groups: "list[PoolGroup]"))]
    pub fn set_pools_config<'a>(
        &self,
        py: Python<'a>,
        groups: Vec<PoolGroup>,
    ) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.set_pools_config(groups).await.ok())
        })
    }
    /// Set scaling configuration.
    #[pyo3(signature = (config: "ScalingConfig"))]
    pub fn set_scaling_config<'a>(
        &self,
        py: Python<'a>,
        config: ScalingConfig,
    ) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.set_scaling_config(config).await.ok())
        })
    }
    /// Set tuning configuration, optionally with companion scaling settings.
    #[pyo3(signature = (config: "TuningConfig", scaling_config: "ScalingConfig | None" = None))]
    pub fn set_tuning_config<'a>(
        &self,
        py: Python<'a>,
        config: TuningConfig,
        scaling_config: Option<ScalingConfig>,
    ) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.set_tuning_config(config, scaling_config).await.ok())
        })
    }
    /// Set fan configuration.
    #[pyo3(signature = (config: "FanConfig"))]
    pub fn set_fan_config<'a>(
        &self,
        py: Python<'a>,
        config: FanConfig,
    ) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.set_fan_config(config).await.ok())
        })
    }
    /// Upload and apply a firmware image from a local path.
    pub fn upgrade_firmware<'a>(
        &self,
        py: Python<'a>,
        path: FirmwarePath,
    ) -> PyResult<PyAwaitable<bool>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let image = FirmwareImage::from_file_async(&path.0)
                .await
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            let inner = inner.read().await;
            inner
                .upgrade_firmware(image)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        })
    }
}

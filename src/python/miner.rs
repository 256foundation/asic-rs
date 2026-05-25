use std::{net::IpAddr, path::PathBuf, sync::Arc, time::Duration};

use asic_rs_core::{
    config::{
        fan::FanConfig, pools::PoolGroupConfig as PoolGroup, scaling::ScalingConfig,
        tuning::TuningConfig,
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

use super::typing::{PyAwaitable, future_into_py};

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

    #[getter]
    fn ip(&self, py: Python<'_>) -> IpAddr {
        self.with_miner(py, |miner| miner.get_ip())
    }

    #[getter]
    fn model(&self, py: Python<'_>) -> String {
        self.with_miner(py, |miner| miner.get_device_info().model)
    }
    #[getter]
    fn make(&self, py: Python<'_>) -> String {
        self.with_miner(py, |miner| miner.get_device_info().make)
    }
    #[getter]
    fn firmware(&self, py: Python<'_>) -> String {
        self.with_miner(py, |miner| miner.get_device_info().firmware)
    }
    #[getter]
    fn algo(&self, py: Python<'_>) -> HashAlgorithm {
        self.with_miner(py, |miner| miner.get_device_info().algo)
    }
    #[getter]
    fn hardware(&self, py: Python<'_>) -> MinerHardware {
        self.with_miner(py, |miner| miner.get_device_info().hardware)
    }

    #[getter]
    fn expected_hashboards(&self, py: Python<'_>) -> Option<u8> {
        self.with_miner(py, |miner| miner.get_expected_hashboards())
    }

    #[getter]
    fn expected_chips(&self, py: Python<'_>) -> Option<u16> {
        self.with_miner(py, |miner| miner.get_expected_chips())
    }

    #[getter]
    fn expected_fans(&self, py: Python<'_>) -> Option<u8> {
        self.with_miner(py, |miner| miner.get_expected_fans())
    }

    #[getter]
    fn supports_set_fault_light(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_set_fault_light())
    }
    #[getter]
    fn supports_set_power_limit(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_set_power_limit())
    }
    #[getter]
    fn supports_restart(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_restart())
    }
    #[getter]
    fn supports_pause(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_pause())
    }
    #[getter]
    fn supports_resume(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_resume())
    }
    #[getter]
    fn supports_change_password(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_change_password())
    }
    #[getter]
    fn supports_read_logs(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_read_logs())
    }
    #[getter]
    fn supports_factory_reset(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_factory_reset())
    }
    #[getter]
    fn supports_pools_config(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_pools_config())
    }
    #[getter]
    fn supports_upgrade_firmware(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_upgrade_firmware())
    }
    #[getter]
    fn supports_scaling_config(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_scaling_config())
    }
    #[getter]
    fn supports_tuning_config(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_tuning_config())
    }
    #[getter]
    fn supports_fan_config(&self, py: Python<'_>) -> bool {
        self.with_miner(py, |miner| miner.supports_fan_config())
    }
    pub fn set_auth(&mut self, username: String, password: String) -> PyResult<()> {
        Arc::get_mut(&mut self.inner)
            .ok_or_else(|| PyRuntimeError::new_err("cannot set auth while miner is in use"))?
            .get_mut()
            .set_auth(MinerAuth::new(username, password));
        Ok(())
    }

    // Data functions
    pub fn get_data<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<MinerData>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_data().await)
        })
    }
    pub fn get_mac<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_mac().await;
            Ok(data.map(|m| m.to_string()))
        })
    }
    pub fn get_serial_number<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_serial_number().await;
            Ok(data)
        })
    }
    pub fn get_hostname<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_hostname().await;
            Ok(data)
        })
    }
    pub fn get_api_version<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_api_version().await;
            Ok(data)
        })
    }
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
    pub fn get_hashboards<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<BoardData>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_hashboards().await)
        })
    }
    pub fn get_hashrate<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<HashRate>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_hashrate().await;
            Ok(data)
        })
    }
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
    pub fn get_fans<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<FanData>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_fans().await)
        })
    }
    pub fn get_psu_fans<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<FanData>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_psu_fans().await)
        })
    }
    pub fn get_fluid_temperature<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<f64>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_fluid_temperature().await;
            Ok(data.map(|t| t.as_celsius()))
        })
    }
    pub fn get_wattage<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<f64>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_wattage().await;
            Ok(data.map(|w| w.as_watts()))
        })
    }
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
    pub fn get_light_flashing<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_light_flashing().await;
            Ok(data)
        })
    }
    pub fn get_messages<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<MinerMessage>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_messages().await)
        })
    }
    pub fn get_uptime<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<Duration>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_uptime().await;
            Ok(data)
        })
    }
    pub fn get_is_mining<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<bool>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.get_is_mining().await;
            Ok(data)
        })
    }
    pub fn get_pools<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<PoolGroupData>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_pools().await)
        })
    }

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
    pub fn get_fan_config<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<FanConfig>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            Ok(inner.get_fan_config().await.ok())
        })
    }

    // Control functions
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
    pub fn restart<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.restart().await;
            Ok(data.ok())
        })
    }
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
    pub fn factory_reset<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<bool>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.factory_reset().await;
            Ok(data.ok())
        })
    }
    pub fn read_logs<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Option<String>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let inner = inner.read().await;
            let data = inner.read_logs().await;
            Ok(data.ok())
        })
    }
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

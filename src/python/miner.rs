use std::{net::IpAddr, sync::Arc, time::Duration};

use asic_rs_core::{
    config::pools::PoolGroupConfig,
    data::{
        device::{HashAlgorithm, MinerHardware},
        firmware::FirmwareImage,
    },
    traits::{auth::MinerAuth, miner::Miner as MinerTrait},
};
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
};

use super::data::{BoardData, FanData, MinerData, TuningTarget};

#[pyclass(module = "asic_rs")]
pub(crate) struct Miner {
    inner: Arc<Box<dyn MinerTrait>>,
}

impl Miner {
    pub fn new(inner: Box<dyn MinerTrait>) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

impl From<Box<dyn MinerTrait>> for Miner {
    fn from(inner: Box<dyn MinerTrait>) -> Self {
        Self::new(inner)
    }
}

#[pymethods]
impl Miner {
    fn __repr__(&self) -> String {
        format!(
            "{} {} ({}): {}",
            self.make(),
            self.model(),
            self.firmware(),
            self.ip(),
        )
    }

    #[getter]
    fn ip(&self) -> IpAddr {
        self.inner.get_ip()
    }

    #[getter]
    fn model(&self) -> String {
        self.inner.get_device_info().model
    }
    #[getter]
    fn make(&self) -> String {
        self.inner.get_device_info().make
    }
    #[getter]
    fn firmware(&self) -> String {
        self.inner.get_device_info().firmware
    }
    #[getter]
    fn algo(&self) -> HashAlgorithm {
        self.inner.get_device_info().algo
    }
    #[getter]
    fn hardware(&self) -> MinerHardware {
        self.inner.get_device_info().hardware
    }

    #[getter]
    fn expected_hashboards(&self) -> Option<u8> {
        self.inner.get_expected_hashboards()
    }

    #[getter]
    fn expected_chips(&self) -> Option<u16> {
        self.inner.get_expected_chips()
    }

    #[getter]
    fn expected_fans(&self) -> Option<u8> {
        self.inner.get_expected_fans()
    }

    #[getter]
    fn supports_set_fault_light(&self) -> bool {
        self.inner.supports_set_fault_light()
    }
    #[getter]
    fn supports_set_power_limit(&self) -> bool {
        self.inner.supports_set_power_limit()
    }
    #[getter]
    fn supports_restart(&self) -> bool {
        self.inner.supports_restart()
    }
    #[getter]
    fn supports_pause(&self) -> bool {
        self.inner.supports_pause()
    }
    #[getter]
    fn supports_resume(&self) -> bool {
        self.inner.supports_resume()
    }
    #[getter]
    fn supports_pools_config(&self) -> bool {
        self.inner.supports_pools_config()
    }
    #[getter]
    fn supports_upgrade_firmware(&self) -> bool {
        self.inner.supports_upgrade_firmware()
    }
    pub fn set_auth(&mut self, username: String, password: String) -> PyResult<()> {
        Arc::get_mut(&mut self.inner)
            .ok_or_else(|| PyRuntimeError::new_err("cannot set auth while miner is in use"))?
            .set_auth(MinerAuth::new(username, password));
        Ok(())
    }

    // Data functions
    pub fn get_data<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_data().await;
            Ok(MinerData::from(&data))
        })
    }
    pub fn get_mac<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_mac().await;
            Ok(data.map(|m| m.to_string()))
        })
    }
    pub fn get_serial_number<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_serial_number().await;
            Ok(data)
        })
    }
    pub fn get_hostname<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_hostname().await;
            Ok(data)
        })
    }
    pub fn get_api_version<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_api_version().await;
            Ok(data)
        })
    }
    pub fn get_firmware_version<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_firmware_version().await;
            Ok(data)
        })
    }
    pub fn get_control_board_version<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner
                .get_control_board_version()
                .await
                .map(|cb| cb.to_string());
            Ok(data)
        })
    }
    pub fn get_hashboards<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_hashboards().await;
            Ok(data.iter().map(BoardData::from).collect::<Vec<BoardData>>())
        })
    }
    pub fn get_hashrate<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_hashrate().await;
            Ok(data)
        })
    }
    pub fn get_expected_hashrate<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_expected_hashrate().await;
            Ok(data)
        })
    }
    pub fn get_fans<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_fans().await;
            Ok(data.iter().map(FanData::from).collect::<Vec<FanData>>())
        })
    }
    pub fn get_psu_fans<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_psu_fans().await;
            Ok(data.iter().map(FanData::from).collect::<Vec<FanData>>())
        })
    }
    pub fn get_fluid_temperature<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_fluid_temperature().await;
            Ok(data.map(|t| t.as_celsius()))
        })
    }
    pub fn get_wattage<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_wattage().await;
            Ok(data.map(|w| w.as_watts()))
        })
    }
    pub fn get_tuning_target<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_tuning_target().await;
            Ok(data.as_ref().map(TuningTarget::from))
        })
    }
    pub fn get_light_flashing<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_light_flashing().await;
            Ok(data)
        })
    }
    pub fn get_messages<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_messages().await;
            Ok(data)
        })
    }
    pub fn get_uptime<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_uptime().await;
            Ok(data)
        })
    }
    pub fn get_is_mining<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_is_mining().await;
            Ok(data)
        })
    }
    pub fn get_pools<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_pools().await;
            Ok(data)
        })
    }

    pub fn get_pools_config<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.get_pools_config().await;
            Ok(data.ok())
        })
    }

    // Control functions
    pub fn set_fault_light<'a>(&self, py: Python<'a>, fault: bool) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.set_fault_light(fault).await;
            Ok(data.ok())
        })
    }
    pub fn restart<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.restart().await;
            Ok(data.ok())
        })
    }
    pub fn pause<'a>(
        &self,
        py: Python<'a>,
        at_time: Option<Duration>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.pause(at_time).await;
            Ok(data.ok())
        })
    }
    pub fn resume<'a>(
        &self,
        py: Python<'a>,
        at_time: Option<Duration>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.resume(at_time).await;
            Ok(data.ok())
        })
    }
    pub fn set_pools_config<'a>(
        &self,
        py: Python<'a>,
        groups: Vec<PoolGroupConfig>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let data = inner.set_pools_config(groups).await;
            Ok(data.ok())
        })
    }
    pub fn upgrade_firmware<'a>(&self, py: Python<'a>, path: String) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let image = FirmwareImage::from_file_async(&path)
                .await
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            inner
                .upgrade_firmware(image)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        })
    }
}

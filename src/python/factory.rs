use std::{net::IpAddr, pin::Pin, str::FromStr, sync::Arc};

use asic_rs_core::traits::miner::Miner as MinerTrait;
use futures::{Stream, StreamExt};
use pyo3::{
    exceptions::{PyConnectionError, PyStopAsyncIteration, PyValueError},
    prelude::*,
    types::PyType,
};
use pyo3_async_runtimes::tokio::future_into_py;

use crate::{factory::MinerFactory as MinerFactory_Base, python::miner::Miner};

#[pyclass]
pub struct PyMinerStream {
    #[allow(clippy::type_complexity)]
    inner: Arc<tokio::sync::Mutex<Pin<Box<dyn Stream<Item = Box<dyn MinerTrait>> + Send>>>>,
}

impl PyMinerStream {
    #[allow(clippy::type_complexity)]
    fn new(inner: Pin<Box<dyn Stream<Item = Box<dyn MinerTrait>> + Send>>) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(inner)),
        }
    }
}
#[pymethods]
impl PyMinerStream {
    pub fn __aiter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    pub fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let mut stream = inner.lock().await;
            if let Some(miner) = stream.next().await {
                Ok(Miner::from(miner))
            } else {
                Err(PyStopAsyncIteration::new_err("stream complete"))
            }
        })
    }
}

#[pyclass]
pub struct PyMinerStreamWithIP {
    #[allow(clippy::type_complexity)]
    inner: Arc<
        tokio::sync::Mutex<
            Pin<Box<dyn Stream<Item = (IpAddr, Option<Box<dyn MinerTrait>>)> + Send>>,
        >,
    >,
}

impl PyMinerStreamWithIP {
    #[allow(clippy::type_complexity)]
    fn new(
        inner: Pin<Box<dyn Stream<Item = (IpAddr, Option<Box<dyn MinerTrait>>)> + Send>>,
    ) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(inner)),
        }
    }
}
#[pymethods]
impl PyMinerStreamWithIP {
    pub fn __aiter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    pub fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let mut stream = inner.lock().await;
            if let Some((ip, miner_opt)) = stream.next().await {
                Ok((ip, miner_opt.map(Miner::new)))
            } else {
                Err(PyStopAsyncIteration::new_err("stream complete"))
            }
        })
    }
}

#[pyclass(module = "asic_rs")]
pub(crate) struct MinerFactory {
    inner: Arc<MinerFactory_Base>,
}

#[pymethods]
impl MinerFactory {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MinerFactory_Base::new()),
        }
    }

    #[classmethod]
    pub fn from_subnet(_cls: &Bound<'_, PyType>, subnet: String) -> PyResult<Self> {
        let factory = MinerFactory_Base::new().with_subnet(&subnet);
        match factory {
            Ok(f) => Ok(Self { inner: Arc::new(f) }),
            Err(e) => Err(PyValueError::new_err(e.to_string())),
        }
    }

    pub fn with_subnet(&mut self, subnet: &str) -> PyResult<()> {
        let inner = Arc::<MinerFactory_Base>::make_mut(&mut self.inner).clone();
        self.inner = Arc::new(
            inner
                .with_subnet(subnet)
                .map_err(|e| PyValueError::new_err(e.to_string()))?,
        );
        Ok(())
    }

    #[classmethod]
    pub fn from_octets(
        _cls: &Bound<'_, PyType>,
        octet1: String,
        octet2: String,
        octet3: String,
        octet4: String,
    ) -> PyResult<Self> {
        let factory = MinerFactory_Base::new().with_octets(&octet1, &octet2, &octet3, &octet4);
        match factory {
            Ok(f) => Ok(Self { inner: Arc::new(f) }),
            Err(e) => Err(PyValueError::new_err(e.to_string())),
        }
    }

    pub fn with_octets(
        &mut self,
        octet1: String,
        octet2: String,
        octet3: String,
        octet4: String,
    ) -> PyResult<()> {
        let inner = Arc::<MinerFactory_Base>::make_mut(&mut self.inner).clone();
        self.inner = Arc::new(
            inner
                .with_octets(&octet1, &octet2, &octet3, &octet4)
                .map_err(|e| PyValueError::new_err(e.to_string()))?,
        );
        Ok(())
    }

    #[classmethod]
    pub fn from_range(_cls: &Bound<'_, PyType>, range: String) -> PyResult<Self> {
        let factory = MinerFactory_Base::new().with_range(&range);
        match factory {
            Ok(f) => Ok(Self { inner: Arc::new(f) }),
            Err(e) => Err(PyValueError::new_err(e.to_string())),
        }
    }

    pub fn with_range(&mut self, range: &str) -> PyResult<()> {
        let inner = Arc::<MinerFactory_Base>::make_mut(&mut self.inner).clone();
        self.inner = Arc::new(
            inner
                .with_range(range)
                .map_err(|e| PyValueError::new_err(e.to_string()))?,
        );
        Ok(())
    }

    pub fn with_concurrent_limit(&mut self, limit: usize) -> PyResult<()> {
        let inner = Arc::<MinerFactory_Base>::make_mut(&mut self.inner).clone();
        self.inner = Arc::new(inner.with_concurrent_limit(limit));
        Ok(())
    }

    pub fn with_identification_timeout_secs(&mut self, timeout_secs: u64) -> PyResult<()> {
        let inner = Arc::<MinerFactory_Base>::make_mut(&mut self.inner).clone();
        self.inner = Arc::new(inner.with_identification_timeout_secs(timeout_secs));
        Ok(())
    }

    pub fn with_connectivity_timeout_secs(&mut self, timeout_secs: u64) -> PyResult<()> {
        let inner = Arc::<MinerFactory_Base>::make_mut(&mut self.inner).clone();
        self.inner = Arc::new(inner.with_connectivity_timeout_secs(timeout_secs));
        Ok(())
    }

    pub fn with_connectivity_retries(&mut self, retries: u32) -> PyResult<()> {
        let inner = Arc::<MinerFactory_Base>::make_mut(&mut self.inner).clone();
        self.inner = Arc::new(inner.with_connectivity_retries(retries));
        Ok(())
    }

    pub fn with_port_check(&mut self, enabled: bool) -> PyResult<()> {
        let inner = Arc::<MinerFactory_Base>::make_mut(&mut self.inner).clone();
        self.inner = Arc::new(inner.with_port_check(enabled));
        Ok(())
    }

    pub fn scan<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let miners = inner.scan().await;
            match miners {
                Ok(miners) => Ok(miners.into_iter().map(Miner::from).collect::<Vec<Miner>>()),
                Err(e) => Err(PyValueError::new_err(e.to_string())),
            }
        })
    }

    pub fn scan_stream<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMinerStream>> {
        let inner = Arc::clone(&self.inner);
        Bound::new(py, PyMinerStream::new(inner.scan_stream()))
    }

    pub fn scan_stream_with_ip<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyMinerStreamWithIP>> {
        let inner = Arc::clone(&self.inner);
        Bound::new(py, PyMinerStreamWithIP::new(inner.scan_stream_with_ip()))
    }

    pub fn get_miner<'a>(&self, py: Python<'a>, ip: String) -> PyResult<Bound<'a, PyAny>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let miner = inner.get_miner(IpAddr::from_str(&ip)?).await;
            match miner {
                Ok(Some(miner)) => Ok(Some(Miner::from(miner))),
                Ok(None) => Ok(None),
                Err(e) => Err(PyConnectionError::new_err(e.to_string())),
            }
        })
    }
}

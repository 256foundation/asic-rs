use std::{fmt::Display, net::IpAddr, pin::Pin, str::FromStr, sync::Arc};

use asic_rs_core::traits::miner::Miner as MinerTrait;
use asic_rs_pydantic::py_to_string;
use futures::{Stream, StreamExt};
use pyo3::{
    exceptions::{PyConnectionError, PyStopAsyncIteration, PyValueError},
    prelude::*,
    types::PyType,
};
use pyo3_async_runtimes::tokio::future_into_py as raw_future_into_py;

use crate::{
    factory::MinerFactory as MinerFactory_Base,
    python::{
        miner::Miner,
        typing::{PyAsyncIterator, PyAwaitable, future_into_py},
    },
};

type MinerStream = Pin<Box<dyn Stream<Item = Box<dyn MinerTrait>> + Send>>;
type MinerStreamWithIp = Pin<Box<dyn Stream<Item = (IpAddr, Option<Box<dyn MinerTrait>>)> + Send>>;

#[pyclass]
pub struct PyMinerStream {
    inner: Arc<tokio::sync::Mutex<MinerStream>>,
}

impl PyMinerStream {
    fn new(inner: MinerStream) -> Self {
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
        raw_future_into_py(py, async move {
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
    inner: Arc<tokio::sync::Mutex<MinerStreamWithIp>>,
}

impl PyMinerStreamWithIP {
    fn new(inner: MinerStreamWithIp) -> Self {
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
        raw_future_into_py(py, async move {
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

impl MinerFactory {
    fn from_inner_result<E: Display>(inner: Result<MinerFactory_Base, E>) -> PyResult<Self> {
        inner
            .map(|inner| Self {
                inner: Arc::new(inner),
            })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn update_inner<'py>(
        mut slf: PyRefMut<'py, Self>,
        update: impl FnOnce(MinerFactory_Base) -> MinerFactory_Base,
    ) -> PyRefMut<'py, Self> {
        let inner = Arc::<MinerFactory_Base>::make_mut(&mut slf.inner).clone();
        slf.inner = Arc::new(update(inner));
        slf
    }

    fn try_update_inner<'py, E: Display>(
        mut slf: PyRefMut<'py, Self>,
        update: impl FnOnce(MinerFactory_Base) -> Result<MinerFactory_Base, E>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let inner = Arc::<MinerFactory_Base>::make_mut(&mut slf.inner).clone();
        slf.inner = Arc::new(update(inner).map_err(|e| PyValueError::new_err(e.to_string()))?);
        Ok(slf)
    }
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
        Self::from_inner_result(MinerFactory_Base::from_subnet(&subnet))
    }

    pub fn with_subnet<'py>(
        slf: PyRefMut<'py, Self>,
        subnet: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Self::try_update_inner(slf, |inner| inner.with_subnet(subnet))
    }

    #[classmethod]
    #[pyo3(signature = (octet1: "str | int", octet2: "str | int", octet3: "str | int", octet4: "str | int") -> "MinerFactory")]
    pub fn from_octets(
        _cls: &Bound<'_, PyType>,
        octet1: &Bound<'_, PyAny>,
        octet2: &Bound<'_, PyAny>,
        octet3: &Bound<'_, PyAny>,
        octet4: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let octet1 = py_to_string(octet1)?;
        let octet2 = py_to_string(octet2)?;
        let octet3 = py_to_string(octet3)?;
        let octet4 = py_to_string(octet4)?;
        Self::from_inner_result(MinerFactory_Base::from_octets(
            &octet1, &octet2, &octet3, &octet4,
        ))
    }

    #[pyo3(signature = (octet1: "str | int", octet2: "str | int", octet3: "str | int", octet4: "str | int") -> "MinerFactory")]
    pub fn with_octets<'py>(
        slf: PyRefMut<'py, Self>,
        octet1: &Bound<'_, PyAny>,
        octet2: &Bound<'_, PyAny>,
        octet3: &Bound<'_, PyAny>,
        octet4: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let octet1 = py_to_string(octet1)?;
        let octet2 = py_to_string(octet2)?;
        let octet3 = py_to_string(octet3)?;
        let octet4 = py_to_string(octet4)?;
        Self::try_update_inner(slf, |inner| {
            inner.with_octets(&octet1, &octet2, &octet3, &octet4)
        })
    }

    #[classmethod]
    pub fn from_range(_cls: &Bound<'_, PyType>, range: String) -> PyResult<Self> {
        Self::from_inner_result(MinerFactory_Base::from_range(&range))
    }

    pub fn with_range<'py>(slf: PyRefMut<'py, Self>, range: &str) -> PyResult<PyRefMut<'py, Self>> {
        Self::try_update_inner(slf, |inner| inner.with_range(range))
    }

    pub fn with_concurrent_limit<'py>(
        slf: PyRefMut<'py, Self>,
        limit: usize,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Ok(Self::update_inner(slf, |inner| {
            inner.with_concurrent_limit(limit)
        }))
    }

    pub fn with_identification_timeout_secs<'py>(
        slf: PyRefMut<'py, Self>,
        timeout_secs: u64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Ok(Self::update_inner(slf, |inner| {
            inner.with_identification_timeout_secs(timeout_secs)
        }))
    }

    pub fn with_connectivity_timeout_secs<'py>(
        slf: PyRefMut<'py, Self>,
        timeout_secs: u64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Ok(Self::update_inner(slf, |inner| {
            inner.with_connectivity_timeout_secs(timeout_secs)
        }))
    }

    pub fn with_connectivity_retries<'py>(
        slf: PyRefMut<'py, Self>,
        retries: u32,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Ok(Self::update_inner(slf, |inner| {
            inner.with_connectivity_retries(retries)
        }))
    }

    pub fn with_port_check<'py>(
        slf: PyRefMut<'py, Self>,
        enabled: bool,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Ok(Self::update_inner(slf, |inner| {
            inner.with_port_check(enabled)
        }))
    }

    pub fn scan<'a>(&self, py: Python<'a>) -> PyResult<PyAwaitable<Vec<Miner>>> {
        let inner = Arc::clone(&self.inner);
        future_into_py(py, async move {
            let miners = inner.scan().await;
            match miners {
                Ok(miners) => Ok(miners.into_iter().map(Miner::from).collect::<Vec<Miner>>()),
                Err(e) => Err(PyValueError::new_err(e.to_string())),
            }
        })
    }

    pub fn scan_stream<'py>(&self, py: Python<'py>) -> PyResult<PyAsyncIterator<Miner>> {
        let inner = Arc::clone(&self.inner);
        Bound::new(py, PyMinerStream::new(inner.scan_stream()))
            .map(Bound::into_any)
            .map(PyAsyncIterator::new)
    }

    pub fn scan_stream_with_ip<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAsyncIterator<(IpAddr, Option<Miner>)>> {
        let inner = Arc::clone(&self.inner);
        Bound::new(py, PyMinerStreamWithIP::new(inner.scan_stream_with_ip()))
            .map(Bound::into_any)
            .map(PyAsyncIterator::new)
    }

    pub fn get_miner<'a>(
        &self,
        py: Python<'a>,
        ip: String,
    ) -> PyResult<PyAwaitable<Option<Miner>>> {
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

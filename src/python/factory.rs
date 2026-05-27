use std::{
    fmt::Display,
    net::IpAddr,
    pin::Pin,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crate::{
    factory::MinerFactory as MinerFactory_Base,
    python::{
        miner::Miner,
        typing::{
            CancelAction, PyAsyncIterator, PyAwaitable, abortable_future_into_py_with_cancel,
            future_into_py,
        },
    },
};
use asic_rs_core::traits::miner::Miner as MinerTrait;
use asic_rs_pydantic::py_to_string;
use futures::{Stream, StreamExt};
use pyo3::{
    exceptions::{PyConnectionError, PyRuntimeError, PyStopAsyncIteration, PyValueError},
    prelude::*,
    types::PyType,
};

type MinerStream = Pin<Box<dyn Stream<Item = Box<dyn MinerTrait>> + Send>>;
type MinerStreamWithIp = Pin<Box<dyn Stream<Item = (IpAddr, Option<Box<dyn MinerTrait>>)> + Send>>;

enum StreamState<S> {
    Ready(S),
    InUse,
    Closed,
}

struct StreamLease<S> {
    state: Arc<Mutex<StreamState<S>>>,
    stream: Option<S>,
}

impl<S> StreamLease<S> {
    fn take(state: Arc<Mutex<StreamState<S>>>) -> PyResult<Self> {
        let stream = {
            let mut guard = state
                .lock()
                .map_err(|_| PyRuntimeError::new_err("stream state lock poisoned"))?;
            match std::mem::replace(&mut *guard, StreamState::InUse) {
                StreamState::Ready(stream) => stream,
                StreamState::InUse => {
                    return Err(PyRuntimeError::new_err("stream is already being polled"));
                }
                StreamState::Closed => {
                    *guard = StreamState::Closed;
                    return Err(PyStopAsyncIteration::new_err("stream complete"));
                }
            }
        };

        Ok(Self {
            state,
            stream: Some(stream),
        })
    }

    fn stream_mut(&mut self) -> PyResult<&mut S> {
        self.stream
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("stream lease missing stream"))
    }

    fn store(mut self, state: StreamState<S>) -> PyResult<()> {
        self.stream = None;
        let mut guard = self
            .state
            .lock()
            .map_err(|_| PyRuntimeError::new_err("stream state lock poisoned"))?;
        if matches!(*guard, StreamState::Closed) && matches!(state, StreamState::Ready(_)) {
            return Ok(());
        }
        *guard = state;
        Ok(())
    }

    fn store_ready(mut self) -> PyResult<()> {
        let stream = self
            .stream
            .take()
            .ok_or_else(|| PyRuntimeError::new_err("stream lease missing stream"))?;
        self.store(StreamState::Ready(stream))
    }

    fn close(self) -> PyResult<()> {
        self.store(StreamState::Closed)
    }
}

impl<S> Drop for StreamLease<S> {
    fn drop(&mut self) {
        if self.stream.is_some()
            && let Ok(mut guard) = self.state.lock()
        {
            *guard = StreamState::Closed;
        }
    }
}

fn close_stream_state<S>(state: &Arc<Mutex<StreamState<S>>>) {
    let _previous = {
        let Ok(mut guard) = state.lock() else {
            return;
        };
        std::mem::replace(&mut *guard, StreamState::Closed)
    };
}

fn close_stream_on_cancel<S: Send + 'static>(state: Arc<Mutex<StreamState<S>>>) -> CancelAction {
    Box::new(move || close_stream_state(&state))
}

#[pyclass]
pub struct PyMinerStream {
    inner: Arc<Mutex<StreamState<MinerStream>>>,
}

impl PyMinerStream {
    fn new(inner: MinerStream) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StreamState::Ready(inner))),
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
        let cancel_inner = inner.clone();
        abortable_future_into_py_with_cancel(
            py,
            async move {
                let mut lease = StreamLease::take(inner)?;
                match lease.stream_mut()?.next().await {
                    Some(miner) => {
                        let miner = Miner::from(miner);
                        lease.store_ready()?;
                        Ok(miner)
                    }
                    None => {
                        lease.close()?;
                        Err(PyStopAsyncIteration::new_err("stream complete"))
                    }
                }
            },
            Some(close_stream_on_cancel(cancel_inner)),
        )
    }
}

#[pyclass]
pub struct PyMinerStreamWithIP {
    inner: Arc<Mutex<StreamState<MinerStreamWithIp>>>,
}

impl PyMinerStreamWithIP {
    fn new(inner: MinerStreamWithIp) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StreamState::Ready(inner))),
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
        let cancel_inner = inner.clone();
        abortable_future_into_py_with_cancel(
            py,
            async move {
                let mut lease = StreamLease::take(inner)?;
                match lease.stream_mut()?.next().await {
                    Some((ip, miner_opt)) => {
                        let item = (ip, miner_opt.map(Miner::new));
                        lease.store_ready()?;
                        Ok(item)
                    }
                    None => {
                        lease.close()?;
                        Err(PyStopAsyncIteration::new_err("stream complete"))
                    }
                }
            },
            Some(close_stream_on_cancel(cancel_inner)),
        )
    }
}

/// Python handle for miner discovery and construction.
///
/// A `MinerFactory` stores the IP addresses to scan plus discovery tuning such
/// as concurrency and timeouts. Methods that add ranges return `self`, so calls
/// can be chained before awaiting `scan()`.
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
    /// Create an empty factory.
    ///
    /// Use `get_miner(ip)` for a known address, or add a range with
    /// `from_subnet`, `from_octets`, `from_range`, or the matching `with_*`
    /// methods before calling `scan()`.
    #[new]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MinerFactory_Base::new()),
        }
    }

    /// Create a factory populated from a CIDR subnet.
    ///
    /// Example: `MinerFactory.from_subnet("192.168.1.0/24")`.
    #[classmethod]
    pub fn from_subnet(_cls: &Bound<'_, PyType>, subnet: String) -> PyResult<Self> {
        Self::from_inner_result(MinerFactory_Base::from_subnet(&subnet))
    }

    /// Append all addresses from a CIDR subnet and return this factory.
    pub fn with_subnet<'py>(
        slf: PyRefMut<'py, Self>,
        subnet: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Self::try_update_inner(slf, |inner| inner.with_subnet(subnet))
    }

    /// Create a factory from four IPv4 octet selectors.
    ///
    /// Each octet may be a number or a string range such as `"1-254"`.
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

    /// Append addresses generated from four IPv4 octet selectors.
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

    /// Create a factory from an IPv4 range string.
    ///
    /// Example: `MinerFactory.from_range("192.168.1.1-254")`.
    #[classmethod]
    pub fn from_range(_cls: &Bound<'_, PyType>, range: String) -> PyResult<Self> {
        Self::from_inner_result(MinerFactory_Base::from_range(&range))
    }

    /// Append addresses generated from an IPv4 range string.
    pub fn with_range<'py>(slf: PyRefMut<'py, Self>, range: &str) -> PyResult<PyRefMut<'py, Self>> {
        Self::try_update_inner(slf, |inner| inner.with_range(range))
    }

    /// Set the maximum number of addresses scanned concurrently.
    pub fn with_concurrent_limit<'py>(
        slf: PyRefMut<'py, Self>,
        limit: usize,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Ok(Self::update_inner(slf, |inner| {
            inner.with_concurrent_limit(limit)
        }))
    }

    /// Set the maximum seconds spent identifying a miner after it responds.
    pub fn with_identification_timeout_secs<'py>(
        slf: PyRefMut<'py, Self>,
        timeout_secs: u64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Ok(Self::update_inner(slf, |inner| {
            inner.with_identification_timeout_secs(timeout_secs)
        }))
    }

    /// Set the timeout, in seconds, for quick connectivity probes.
    pub fn with_connectivity_timeout_secs<'py>(
        slf: PyRefMut<'py, Self>,
        timeout_secs: u64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Ok(Self::update_inner(slf, |inner| {
            inner.with_connectivity_timeout_secs(timeout_secs)
        }))
    }

    /// Set how many connectivity attempts are made before identification.
    pub fn with_connectivity_retries<'py>(
        slf: PyRefMut<'py, Self>,
        retries: u32,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Ok(Self::update_inner(slf, |inner| {
            inner.with_connectivity_retries(retries)
        }))
    }

    /// Enable or disable quick TCP port checks before miner identification.
    pub fn with_port_check<'py>(
        slf: PyRefMut<'py, Self>,
        enabled: bool,
    ) -> PyResult<PyRefMut<'py, Self>> {
        Ok(Self::update_inner(slf, |inner| {
            inner.with_port_check(enabled)
        }))
    }

    /// Await a scan of all queued addresses and return every supported miner.
    ///
    /// Unsupported hosts are skipped. Raises `ValueError` when no addresses have
    /// been queued or when the scan setup is invalid.
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

    /// Return an async iterator over miners as soon as they are found.
    pub fn scan_stream<'py>(&self, py: Python<'py>) -> PyResult<PyAsyncIterator<Miner>> {
        let inner = Arc::clone(&self.inner);
        Bound::new(py, PyMinerStream::new(inner.scan_stream()))
            .map(Bound::into_any)
            .map(PyAsyncIterator::new)
    }

    /// Return an async iterator of `(ip, miner)` for every scanned address.
    ///
    /// The miner value is `None` when the IP did not identify as a supported
    /// ASIC miner.
    pub fn scan_stream_with_ip<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAsyncIterator<(IpAddr, Option<Miner>)>> {
        let inner = Arc::clone(&self.inner);
        Bound::new(py, PyMinerStreamWithIP::new(inner.scan_stream_with_ip()))
            .map(Bound::into_any)
            .map(PyAsyncIterator::new)
    }

    /// Identify and construct a miner at a known IP address.
    ///
    /// Returns `None` when the address does not respond as a supported miner.
    /// Raises `ConnectionError` for discovery errors.
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

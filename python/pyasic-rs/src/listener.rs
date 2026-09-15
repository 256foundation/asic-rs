use std::{
    net::IpAddr,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::{
    miner::Miner,
    typing::{CancelAction, PyAsyncIterator, abortable_future_into_py_with_cancel},
};
use asic_rs::listener::MinerListener as MinerListenerBase;
use asic_rs_core::traits::miner::Miner as MinerTrait;
use async_stream::stream;
use futures::Stream;
use pyo3::{
    exceptions::{PyRuntimeError, PyStopAsyncIteration},
    prelude::*,
};
use tokio_stream::StreamExt;

type ListenerMinerStream =
    Pin<Box<dyn Stream<Item = anyhow::Result<Option<Box<dyn MinerTrait>>>> + Send>>;
type ListenerIpStream = Pin<Box<dyn Stream<Item = anyhow::Result<Option<IpAddr>>> + Send>>;

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

fn miner_stream(listener: Arc<MinerListenerBase>) -> ListenerMinerStream {
    Box::pin(stream! {
        let mut inner = listener.listen().await;
        while let Some(item) = inner.next().await {
            yield item;
        }
    })
}

fn ip_stream(listener: Arc<MinerListenerBase>) -> ListenerIpStream {
    Box::pin(stream! {
        let mut inner = listener.listen_ip_only().await;
        while let Some(item) = inner.next().await {
            yield item;
        }
    })
}

#[pyclass]
struct PyListenerMinerStream {
    inner: Arc<Mutex<StreamState<ListenerMinerStream>>>,
}

impl PyListenerMinerStream {
    fn new(inner: ListenerMinerStream) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StreamState::Ready(inner))),
        }
    }
}

#[pymethods]
impl PyListenerMinerStream {
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
                loop {
                    match lease.stream_mut()?.next().await {
                        Some(Ok(Some(miner))) => {
                            let miner = Miner::from(miner);
                            lease.store_ready()?;
                            return Ok(miner);
                        }
                        Some(Ok(None)) => continue,
                        Some(Err(error)) => {
                            lease.close()?;
                            return Err(PyRuntimeError::new_err(error.to_string()));
                        }
                        None => {
                            lease.close()?;
                            return Err(PyStopAsyncIteration::new_err("stream complete"));
                        }
                    }
                }
            },
            Some(close_stream_on_cancel(cancel_inner)),
        )
    }
}

#[pyclass]
struct PyListenerIpStream {
    inner: Arc<Mutex<StreamState<ListenerIpStream>>>,
}

impl PyListenerIpStream {
    fn new(inner: ListenerIpStream) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StreamState::Ready(inner))),
        }
    }
}

#[pymethods]
impl PyListenerIpStream {
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
                loop {
                    match lease.stream_mut()?.next().await {
                        Some(Ok(Some(ip))) => {
                            lease.store_ready()?;
                            return Ok(ip);
                        }
                        Some(Ok(None)) => continue,
                        Some(Err(error)) => {
                            lease.close()?;
                            return Err(PyRuntimeError::new_err(error.to_string()));
                        }
                        None => {
                            lease.close()?;
                            return Err(PyStopAsyncIteration::new_err("stream complete"));
                        }
                    }
                }
            },
            Some(close_stream_on_cancel(cancel_inner)),
        )
    }
}

/// Python listener for miner broadcast packets.
#[pyclass(module = "asic_rs")]
pub(crate) struct MinerListener {
    inner: Arc<MinerListenerBase>,
}

#[pymethods]
impl MinerListener {
    /// Create a listener for supported miner broadcast packets.
    #[new]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MinerListenerBase::new()),
        }
    }

    /// Return an async iterator over miners as broadcast packets arrive.
    pub fn listen<'py>(&self, py: Python<'py>) -> PyResult<PyAsyncIterator<Miner>> {
        Bound::new(
            py,
            PyListenerMinerStream::new(miner_stream(self.inner.clone())),
        )
        .map(Bound::into_any)
        .map(PyAsyncIterator::new)
    }

    /// Return an async iterator over miner IP addresses as broadcast packets arrive.
    pub fn listen_ip_only<'py>(&self, py: Python<'py>) -> PyResult<PyAsyncIterator<IpAddr>> {
        Bound::new(py, PyListenerIpStream::new(ip_stream(self.inner.clone())))
            .map(Bound::into_any)
            .map(PyAsyncIterator::new)
    }
}

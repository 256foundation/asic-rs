use std::{any::Any, future::Future, marker::PhantomData, panic::AssertUnwindSafe};

use futures::{
    FutureExt,
    future::{AbortHandle, Abortable},
};
use pyo3::{
    IntoPyObject, PyAny, PyErr, PyResult, Python, exceptions::PyRuntimeError, prelude::*,
    type_hint_identifier, type_hint_subscript,
};

use pyo3_async_runtimes::tokio::future_into_py as raw_future_into_py;

pub(crate) type CancelAction = Box<dyn FnOnce() + Send + Sync + 'static>;

pub(crate) struct PyAwaitable<T> {
    inner: Py<PyAny>,
    _ty: PhantomData<T>,
}

impl<T> PyAwaitable<T> {
    pub(crate) fn new(inner: Bound<'_, PyAny>) -> Self {
        Self {
            inner: inner.unbind(),
            _ty: PhantomData,
        }
    }
}

impl<'py, T> IntoPyObject<'py> for PyAwaitable<T>
where
    T: IntoPyObject<'py>,
{
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    const OUTPUT_TYPE: pyo3::inspect::PyStaticExpr = type_hint_subscript!(
        type_hint_identifier!("collections.abc", "Awaitable"),
        T::OUTPUT_TYPE
    );

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(self.inner.into_bound(py))
    }
}

pub(crate) struct PyAsyncIterator<T> {
    inner: Py<PyAny>,
    _ty: PhantomData<T>,
}

impl<T> PyAsyncIterator<T> {
    pub(crate) fn new(inner: Bound<'_, PyAny>) -> Self {
        Self {
            inner: inner.unbind(),
            _ty: PhantomData,
        }
    }
}

impl<'py, T> IntoPyObject<'py> for PyAsyncIterator<T>
where
    T: IntoPyObject<'py>,
{
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    const OUTPUT_TYPE: pyo3::inspect::PyStaticExpr = type_hint_subscript!(
        type_hint_identifier!("collections.abc", "AsyncIterator"),
        T::OUTPUT_TYPE
    );

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(self.inner.into_bound(py))
    }
}

pub(crate) fn future_into_py<'py, T, F>(py: Python<'py>, future: F) -> PyResult<PyAwaitable<T>>
where
    T: for<'a> IntoPyObject<'a> + Send + 'static,
    F: Future<Output = PyResult<T>> + Send + 'static,
{
    abortable_future_into_py(py, future).map(PyAwaitable::new)
}

pub(crate) fn abortable_future_into_py<'py, T, F>(
    py: Python<'py>,
    future: F,
) -> PyResult<Bound<'py, PyAny>>
where
    T: for<'a> IntoPyObject<'a> + Send + 'static,
    F: Future<Output = PyResult<T>> + Send + 'static,
{
    abortable_future_into_py_with_cancel(py, future, None)
}

pub(crate) fn abortable_future_into_py_with_cancel<'py, T, F>(
    py: Python<'py>,
    future: F,
    cancel_action: Option<CancelAction>,
) -> PyResult<Bound<'py, PyAny>>
where
    T: for<'a> IntoPyObject<'a> + Send + 'static,
    F: Future<Output = PyResult<T>> + Send + 'static,
{
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let abortable = Abortable::new(
        async move {
            match AssertUnwindSafe(future).catch_unwind().await {
                Ok(result) => result,
                Err(panic) => Err(PyRuntimeError::new_err(format!(
                    "rust future panicked: {}",
                    panic_message(&*panic),
                ))),
            }
        },
        abort_registration,
    );
    let awaitable = raw_future_into_py(py, async move {
        abortable.await.map_err(|_| {
            PyRuntimeError::new_err("rust future cancelled before returning a result")
        })?
    })?;

    awaitable.call_method1(
        pyo3::intern!(py, "add_done_callback"),
        (Py::new(
            py,
            RustFutureCancellationGuard {
                abort_handle: Some(abort_handle),
                cancel_action,
            },
        )?,),
    )?;
    Ok(awaitable)
}

fn panic_message(panic: &(dyn Any + Send)) -> &str {
    if let Some(message) = panic.downcast_ref::<&str>() {
        message
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.as_str()
    } else {
        "unknown error"
    }
}

#[pyclass]
struct RustFutureCancellationGuard {
    abort_handle: Option<AbortHandle>,
    cancel_action: Option<CancelAction>,
}

impl RustFutureCancellationGuard {
    fn cancel(&mut self) {
        if let Some(abort_handle) = self.abort_handle.take() {
            abort_handle.abort();
        }
        if let Some(cancel_action) = self.cancel_action.take() {
            cancel_action();
        }
    }
}

impl Drop for RustFutureCancellationGuard {
    fn drop(&mut self) {
        // If callback registration or Python future ownership fails, do not let
        // the wrapped Rust future continue without an owner.
        self.cancel();
    }
}

#[pymethods]
impl RustFutureCancellationGuard {
    fn __call__(&mut self, fut: &Bound<PyAny>) -> PyResult<()> {
        let cancelled = fut
            .call_method0(pyo3::intern!(fut.py(), "cancelled"))?
            .extract()?;
        if cancelled {
            self.cancel();
        } else {
            self.abort_handle.take();
            self.cancel_action.take();
        }
        Ok(())
    }
}

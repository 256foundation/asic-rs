use std::{future::Future, marker::PhantomData};

use pyo3::{
    IntoPyObject, PyAny, PyErr, PyResult, Python, prelude::*, type_hint_identifier,
    type_hint_subscript,
};

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
    pyo3_async_runtimes::tokio::future_into_py(py, future).map(PyAwaitable::new)
}

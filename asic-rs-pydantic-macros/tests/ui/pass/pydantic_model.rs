#![allow(dead_code)]

use std::{fmt::Display, str::FromStr};

use pyo3::{
    IntoPyObject as _,
    prelude::*,
    types::{PyAny, PyDict},
};

#[pyclass(from_py_object)]
#[derive(Clone, asic_rs_pydantic::PyPydanticModel)]
#[pydantic(schema = "schema", parse = "parse")]
struct Model {
    value: u32,
}

fn schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    _mode: asic_rs_pydantic::PydanticSchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    core_schema.call_method0("int_schema")
}

fn parse(value: &Bound<'_, PyAny>) -> PyResult<Model> {
    Ok(Model {
        value: value.extract()?,
    })
}

impl Model {
    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self.value.into_pyobject(py)?.into_any().unbind())
    }
}

#[pyclass(from_py_object)]
#[derive(Clone, asic_rs_pydantic::PyPydanticModel)]
struct ReprModel {
    value: u32,
    maybe: Option<String>,
    numbers: Vec<u16>,
}

impl ReprModel {
    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        dict.set_item("value", self.value)?;
        dict.set_item("maybe", &self.maybe)?;
        dict.set_item("numbers", &self.numbers)?;
        Ok(dict.into_any().unbind())
    }
}

#[pyclass(from_py_object)]
#[derive(Clone, asic_rs_pydantic::PyPydanticModel)]
struct DefaultLiteralModel {
    #[pydantic(literal = "fixed")]
    kind: String,
    #[pydantic(default = 1)]
    quota: u32,
    #[pydantic(default = None)]
    maybe: Option<String>,
    value: u32,
}

impl DefaultLiteralModel {
    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        dict.set_item("kind", &self.kind)?;
        dict.set_item("quota", self.quota)?;
        dict.set_item("maybe", &self.maybe)?;
        dict.set_item("value", self.value)?;
        Ok(dict.into_any().unbind())
    }
}

#[pyclass(from_py_object)]
#[derive(Clone, asic_rs_pydantic::PyPydanticModel)]
#[pydantic(no_repr)]
struct NoReprModel {
    value: u32,
}

impl NoReprModel {
    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self.value.into_pyobject(py)?.into_any().unbind())
    }
}

#[pyclass(from_py_object, str)]
#[derive(Clone, asic_rs_pydantic::PyPydanticEnum)]
enum GeneratedEnum {
    #[pydantic(value = "alpha")]
    Alpha,
    #[pydantic(value = "beta")]
    Beta,
}

impl Display for GeneratedEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alpha => write!(f, "alpha"),
            Self::Beta => write!(f, "beta"),
        }
    }
}

impl FromStr for GeneratedEnum {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "alpha" => Ok(Self::Alpha),
            "beta" => Ok(Self::Beta),
            value => Err(format!("unknown value: {value}")),
        }
    }
}

fn main() {}

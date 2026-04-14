#![allow(dead_code)]

use pyo3::{prelude::*, types::PyAny};

#[pyclass(from_py_object)]
#[derive(Clone, asic_rs_pydantic::PyPydanticModel)]
#[pydantic(parse = "parse")]
struct MissingSchema {
    value: u32,
}

fn parse(value: &Bound<'_, PyAny>) -> PyResult<MissingSchema> {
    Ok(MissingSchema {
        value: value.extract()?,
    })
}

fn main() {}

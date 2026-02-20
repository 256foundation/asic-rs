use crate::data::pool::PoolURL;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub url: PoolURL,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolGroup {
    pub name: String,
    pub quota: u32,
    pub pools: Vec<Pool>,
}

#[cfg(feature = "python")]
mod python_impls {
    use super::*;
    use crate::data::pool::PoolURL;
    use pyo3::conversion::FromPyObject;
    use pyo3::{Borrowed, PyAny, PyErr, PyResult, types::PyAnyMethods};

    impl FromPyObject<'_, '_> for Pool {
        type Error = PyErr;

        fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
            let url_ob = obj.getattr("url")?;
            let url = url_ob
                .extract::<PoolURL>()
                .or_else(|_| url_ob.extract::<String>().map(PoolURL::from))?;
            Ok(Pool {
                url,
                username: obj.getattr("username")?.extract()?,
                password: obj.getattr("password")?.extract()?,
            })
        }
    }

    impl FromPyObject<'_, '_> for PoolGroup {
        type Error = PyErr;

        fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
            Ok(PoolGroup {
                name: obj.getattr("name")?.extract()?,
                quota: obj.getattr("quota")?.extract()?,
                pools: obj.getattr("pools")?.extract()?,
            })
        }
    }
}

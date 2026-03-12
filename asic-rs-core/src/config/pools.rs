#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::pool::{PoolGroupData, PoolURL};

#[cfg_attr(
    feature = "python",
    pyclass(skip_from_py_object, get_all, module = "asic_rs")
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub url: PoolURL,
    pub username: String,
    pub password: String,
}

#[cfg_attr(
    feature = "python",
    pyclass(skip_from_py_object, get_all, module = "asic_rs")
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolGroup {
    pub name: String,
    pub quota: u32,
    pub pools: Vec<Pool>,
}

impl From<PoolGroupData> for PoolGroup {
    fn from(data: PoolGroupData) -> Self {
        PoolGroup {
            name: data.name,
            quota: data.quota,
            pools: data
                .pools
                .into_iter()
                .filter_map(|p| {
                    Some(Pool {
                        url: p.url?,
                        username: p.user.unwrap_or_default(),
                        password: String::from("x"),
                    })
                })
                .collect(),
        }
    }
}

#[cfg(feature = "python")]
mod python_impls {
    use pyo3::{Borrowed, PyAny, PyErr, PyResult, conversion::FromPyObject, types::PyAnyMethods};

    use super::*;
    use crate::data::pool::PoolURL;

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

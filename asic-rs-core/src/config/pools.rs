#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::pool::{PoolGroupData, PoolURL};

#[cfg_attr(
    feature = "python",
    pyclass(name = "Pool", skip_from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(new, name = "Pool")
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
/// A writable mining pool endpoint.
pub struct PoolConfig {
    /// Pool URL including scheme, host, port, and optional Stratum V2 pubkey.
    pub url: PoolURL,
    /// Worker username sent to the pool.
    pub username: String,
    /// Worker password sent to the pool.
    pub password: String,
}

#[cfg_attr(
    feature = "python",
    pyclass(name = "PoolGroup", skip_from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(new, name = "PoolGroup")
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
/// A writable group of mining pools.
///
/// Some firmwares support multiple pool groups with quota-based selection. For
/// simpler firmwares, use one group named `"default"` with quota `1`.
pub struct PoolGroupConfig {
    /// Pool group name.
    pub name: String,
    /// Pool group quota or priority weight.
    pub quota: u32,
    /// Pools in this group.
    pub pools: Vec<PoolConfig>,
}

impl From<PoolGroupData> for PoolGroupConfig {
    fn from(data: PoolGroupData) -> Self {
        PoolGroupConfig {
            name: data.name,
            quota: data.quota,
            pools: data
                .pools
                .into_iter()
                .filter_map(|p| {
                    Some(PoolConfig {
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
    use asic_rs_pydantic::get_required_field;
    use pyo3::{Borrowed, PyAny, PyErr, PyResult, conversion::FromPyObject, types::PyAnyMethods};

    use super::*;
    use crate::data::pool::PoolURL;

    impl FromPyObject<'_, '_> for PoolConfig {
        type Error = PyErr;

        fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
            let url_ob = get_required_field(&obj, "url")?;
            let url = url_ob
                .extract::<PoolURL>()
                .or_else(|_| url_ob.extract::<String>().map(PoolURL::from))?;
            Ok(PoolConfig {
                url,
                username: get_required_field(&obj, "username")?.extract()?,
                password: get_required_field(&obj, "password")?.extract()?,
            })
        }
    }

    impl FromPyObject<'_, '_> for PoolGroupConfig {
        type Error = PyErr;

        fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
            Ok(PoolGroupConfig {
                name: get_required_field(&obj, "name")?.extract()?,
                quota: get_required_field(&obj, "quota")?.extract()?,
                pools: get_required_field(&obj, "pools")?.extract()?,
            })
        }
    }
}

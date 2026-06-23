#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::pool::{PoolGroupData, PoolURL};

#[cfg_attr(
    feature = "python",
    pyclass(name = "Pool", from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(new, name = "Pool")
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
/// A writable mining pool endpoint.
pub struct PoolConfig {
    /// Pool URL including scheme, host, port, and optional Stratum V2 pubkey.
    #[cfg_attr(feature = "python", pydantic(input_type = "PoolURL | str"))]
    pub url: PoolURL,
    /// Worker username sent to the pool.
    pub username: String,
    /// Worker password sent to the pool.
    pub password: String,
}

#[cfg_attr(
    feature = "python",
    pyclass(name = "PoolGroup", from_py_object, get_all, module = "asic_rs")
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
    #[cfg_attr(feature = "python", pydantic(input_type = "list[Pool]"))]
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

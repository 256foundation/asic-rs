#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::data::pool::{PoolGroupData, PoolURL};

#[cfg_attr(
    feature = "python",
    pyclass(name = "Pool", from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(new, name = "Pool")
)]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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

impl PoolConfig {
    /// Append an exact suffix to the worker username.
    pub fn use_worker_suffix(mut self, suffix: &str) -> Self {
        self.username.push_str(suffix);
        self
    }

    /// Remove the worker name after the first dot, if present.
    pub fn clear_worker_suffix(mut self) -> Self {
        if let Some((username, _)) = self.username.split_once('.') {
            self.username = username.to_string();
        }
        self
    }
}

#[cfg_attr(
    feature = "python",
    pyclass(name = "PoolGroup", from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(new, name = "PoolGroup")
)]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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

impl PoolGroupConfig {
    /// Append an exact suffix to every worker username in this group.
    pub fn use_worker_suffix(mut self, suffix: &str) -> Self {
        self.pools = self
            .pools
            .into_iter()
            .map(|pool| pool.use_worker_suffix(suffix))
            .collect();
        self
    }

    /// Remove each worker name after the first dot, if present.
    pub fn clear_worker_suffix(mut self) -> Self {
        self.pools = self
            .pools
            .into_iter()
            .map(PoolConfig::clear_worker_suffix)
            .collect();
        self
    }
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

#[cfg(test)]
mod tests {
    use super::{PoolConfig, PoolGroupConfig};
    use crate::data::pool::PoolURL;

    #[test]
    fn worker_suffix_can_be_cleared_after_the_first_dot() {
        let group = PoolGroupConfig {
            name: "default".to_string(),
            quota: 1,
            pools: vec![
                PoolConfig {
                    url: PoolURL::from("stratum+tcp://first.example.com:3333".to_string()),
                    username: "account.worker".to_string(),
                    password: "x".to_string(),
                },
                PoolConfig {
                    url: PoolURL::from("stratum+tcp://second.example.com:3333".to_string()),
                    username: "address.worker.extra".to_string(),
                    password: "secret".to_string(),
                },
                PoolConfig {
                    url: PoolURL::from("stratum+tcp://third.example.com:3333".to_string()),
                    username: "solo".to_string(),
                    password: "x".to_string(),
                },
            ],
        };

        let suffixed = group.use_worker_suffix(".device-1");
        assert_eq!(suffixed.pools[0].username, "account.worker.device-1");
        assert_eq!(suffixed.pools[1].username, "address.worker.extra.device-1");

        let cleared = suffixed.clear_worker_suffix();
        assert_eq!(cleared.pools[0].username, "account");
        assert_eq!(cleared.pools[1].username, "address");
        assert_eq!(cleared.pools[2].username, "solo");
        assert_eq!(cleared.pools[1].password, "secret");
        assert_eq!(cleared.quota, 1);
    }
}

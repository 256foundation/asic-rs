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
    /// Optional firmware-specific worker ID variant. None disables unique worker IDs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "python", pydantic(default = None))]
    pub unique_worker_id: Option<String>,
    /// Pools in this group.
    #[cfg_attr(feature = "python", pydantic(input_type = "list[Pool]"))]
    pub pools: Vec<PoolConfig>,
}

impl From<PoolGroupData> for PoolGroupConfig {
    fn from(data: PoolGroupData) -> Self {
        PoolGroupConfig {
            name: data.name,
            quota: data.quota,
            unique_worker_id: None,
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
    use serde_json::json;

    use super::PoolGroupConfig;

    #[test]
    fn unique_worker_id_is_optional_in_existing_pool_group_json() {
        let old_config = json!({ "name": "default", "quota": 1, "pools": [] });
        let group: PoolGroupConfig = serde_json::from_value(old_config.clone()).unwrap();
        assert_eq!(group.unique_worker_id, None);
        assert_eq!(serde_json::to_value(&group).unwrap(), old_config);

        let configured = json!({
            "name": "default",
            "quota": 1,
            "unique_worker_id": "MacAddress",
            "pools": []
        });
        let group: PoolGroupConfig = serde_json::from_value(configured.clone()).unwrap();
        assert_eq!(group.unique_worker_id.as_deref(), Some("MacAddress"));
        assert_eq!(serde_json::to_value(&group).unwrap(), configured);
    }
}

use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

#[cfg(feature = "python")]
use asic_rs_pydantic::{PyPydanticType, PydanticSchemaMode, get_required_field};
#[cfg(feature = "python")]
use pyo3::{prelude::*, types::PyAnyMethods};
use serde::{Deserialize, Serialize};
use url::Url;

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[cfg_attr(feature = "python", derive(asic_rs_pydantic::PyPydanticEnum))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolScheme {
    #[cfg_attr(feature = "python", pydantic(value = "stratum+tcp"))]
    StratumV1,
    #[cfg_attr(feature = "python", pydantic(value = "stratum+ssl"))]
    StratumV1SSL,
    #[cfg_attr(feature = "python", pydantic(value = "stratum2+tcp"))]
    StratumV2,
}

impl From<String> for PoolScheme {
    fn from(scheme: String) -> Self {
        match scheme.as_str() {
            "stratum+tcp" => PoolScheme::StratumV1,
            "stratum+ssl" | "stratum+tls" => PoolScheme::StratumV1SSL,
            "stratum2+tcp" => PoolScheme::StratumV2,
            _ => PoolScheme::StratumV1,
        }
    }
}

impl Display for PoolScheme {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolScheme::StratumV1 => write!(f, "stratum+tcp"),
            PoolScheme::StratumV1SSL => write!(f, "stratum+ssl"),
            PoolScheme::StratumV2 => write!(f, "stratum2+tcp"),
        }
    }
}

impl FromStr for PoolScheme {
    type Err = String;

    fn from_str(scheme: &str) -> Result<Self, Self::Err> {
        match scheme {
            "stratum+tcp" => Ok(PoolScheme::StratumV1),
            "stratum+ssl" => Ok(PoolScheme::StratumV1SSL),
            "stratum2+tcp" => Ok(PoolScheme::StratumV2),
            _ => Err(format!("Unknown pool scheme: {scheme}")),
        }
    }
}

#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(
    feature = "python",
    asic_rs_pydantic::py_pydantic_model(
        schema = "pydantic_pool_url_schema",
        parse = "parse_pool_url",
        manual,
        no_repr
    )
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolURL {
    /// The scheme being used to connect to this pool
    pub scheme: PoolScheme,
    /// The public host of the pool
    pub host: String,
    /// The port being used to connect to the pool
    pub port: u16,
    /// The public key for this pool
    /// Only used for Stratum V2 pools
    pub pubkey: Option<String>,
}

impl From<String> for PoolURL {
    fn from(url: String) -> Self {
        let stratum_url = if url.starts_with("stratum+") || url.starts_with("stratum2+") {
            url.clone()
        } else {
            format!("stratum+tcp://{url}")
        };
        match Url::parse(&stratum_url) {
            Ok(parsed) => {
                let scheme = PoolScheme::from(parsed.scheme().to_string());
                let host = parsed.host_str().unwrap_or("").to_string();
                let port = parsed.port().unwrap_or(80);
                let path = parsed.path();
                let pubkey = match path {
                    "" | "/" => None,
                    _ => Some(path[1..].to_string()),
                };
                PoolURL {
                    scheme,
                    host,
                    port,
                    pubkey,
                }
            }
            Err(_) => PoolURL {
                scheme: PoolScheme::StratumV1,
                host: url,
                port: 0,
                pubkey: None,
            },
        }
    }
}

impl Display for PoolURL {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.pubkey {
            Some(key) => write!(f, "{}://{}:{}/{}", self.scheme, self.host, self.port, key),
            _ => write!(f, "{}://{}:{}", self.scheme, self.host, self.port),
        }
    }
}

#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolData {
    pub position: Option<u16>,
    pub url: Option<PoolURL>,
    pub accepted_shares: Option<u64>,
    pub rejected_shares: Option<u64>,
    pub active: Option<bool>,
    pub alive: Option<bool>,
    pub user: Option<String>,
}

#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolGroupData {
    pub name: String,
    pub quota: u32,
    pub pools: Vec<PoolData>,
}

impl PoolGroupData {
    pub fn len(&self) -> usize {
        self.pools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pools.is_empty()
    }
}

#[cfg_attr(feature = "python", pymethods)]
impl PoolURL {
    pub fn __repr__(&self) -> String {
        self.to_string()
    }
}

#[cfg(feature = "python")]
fn pydantic_pool_url_schema<'py>(
    core_schema: &Bound<'py, PyAny>,
    mode: PydanticSchemaMode,
) -> PyResult<Bound<'py, PyAny>> {
    let str_schema = core_schema.call_method0("str_schema")?;

    if mode == PydanticSchemaMode::Serialization {
        return Ok(str_schema);
    }

    let scheme_schema = PoolScheme::pydantic_schema(core_schema, mode)?;
    let host_schema = core_schema.call_method0("str_schema")?;
    let port_schema = core_schema.call_method0("int_schema")?;
    let pubkey_schema = core_schema.call_method0("str_schema")?;
    let object_schema = asic_rs_pydantic::pydantic_typed_dict_schema!(core_schema, "asic_rs.PoolURL", {
        "scheme" => required(scheme_schema),
        "host" => required(host_schema),
        "port" => required(port_schema),
        "pubkey" => nullable(pubkey_schema),
    })?;
    asic_rs_pydantic::union_schema(core_schema, [str_schema, object_schema])
}

#[cfg(feature = "python")]
fn parse_pool_url(value: &Bound<'_, PyAny>) -> PyResult<PoolURL> {
    if let Ok(model) = value.extract::<PoolURL>() {
        return Ok(model);
    }
    if let Ok(url) = value.extract::<String>() {
        return Ok(PoolURL::from(url));
    }
    Ok(PoolURL {
        scheme: PoolScheme::from_pydantic(&get_required_field(value, "scheme")?)?,
        host: get_required_field(value, "host")?.extract()?,
        port: get_required_field(value, "port")?.extract()?,
        pubkey: get_required_field(value, "pubkey")?.extract()?,
    })
}

#[cfg(feature = "python")]
impl PoolURL {
    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self.to_string().into_pyobject(py)?.into_any().unbind())
    }
}

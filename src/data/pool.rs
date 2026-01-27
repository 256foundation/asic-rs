#[cfg(feature = "python")]
use pyo3::prelude::*;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use url::Url;

#[cfg_attr(feature = "python", pyclass(str, module = "asic_rs"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolScheme {
    StratumV1,
    StratumV1SSL,
    StratumV2,
}

impl From<String> for PoolScheme {
    fn from(scheme: String) -> Self {
        match scheme.as_str() {
            "stratum+tcp" => PoolScheme::StratumV1,
            "stratum+ssl" => PoolScheme::StratumV1SSL,
            "stratum2+tcp" => PoolScheme::StratumV2,
            _ => panic!("Invalid pool scheme"),
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

#[cfg_attr(feature = "python", pyclass(get_all, module = "asic_rs"))]
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
            url
        } else {
            format!("stratum+tcp://{url}")
        };
        let parsed = Url::parse(&stratum_url).expect("Invalid pool URL");
        let scheme = PoolScheme::from(parsed.scheme().to_string());
        let host = parsed.host_str().unwrap_or("").to_string();
        let port = parsed.port().unwrap_or(80);
        let path = parsed.path();
        let pubkey = match path {
            "" => None,
            _ => Some(path[1..].to_string()),
        };
        PoolURL {
            scheme,
            host,
            port,
            pubkey,
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

#[cfg_attr(feature = "python", pyclass(get_all, module = "asic_rs"))]
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

use macaddr::MacAddr;
use serde::{Deserialize, Deserializer};

pub(crate) fn deserialize_macaddr<'de, D>(deserializer: D) -> Result<Option<MacAddr>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt_string = Option::<String>::deserialize(deserializer)?;
    match opt_string {
        Some(s) => s
            .parse::<MacAddr>()
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

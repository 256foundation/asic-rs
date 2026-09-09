use macaddr::MacAddr;
use measurements::{Frequency, Voltage};
use serde::{Deserialize, Deserializer};

pub(crate) fn deserialize_frequency<'de, D>(deserializer: D) -> Result<Option<Frequency>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<f64>::deserialize(deserializer)
        .map(|frequency| frequency.map(Frequency::from_megahertz))
}

pub(crate) fn deserialize_voltage<'de, D>(deserializer: D) -> Result<Option<Voltage>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<f64>::deserialize(deserializer).map(|voltage| voltage.map(Voltage::from_volts))
}

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

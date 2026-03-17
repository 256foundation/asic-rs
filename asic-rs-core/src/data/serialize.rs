use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};

pub(crate) fn serialize_angular_velocity<S>(
    v: &Option<AngularVelocity>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match v {
        Some(angular_velocity) => serializer.serialize_f64(angular_velocity.as_rpm()),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn serialize_temperature<S>(
    t: &Option<Temperature>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match t {
        Some(temperature) => serializer.serialize_f64(temperature.as_celsius()),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn serialize_power<S>(p: &Option<Power>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match p {
        Some(power) => serializer.serialize_f64(power.as_watts()),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn serialize_frequency<S>(
    f: &Option<Frequency>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match f {
        Some(frequency) => serializer.serialize_f64(frequency.as_megahertz()),
        None => serializer.serialize_none(),
    }
}
pub(crate) fn serialize_voltage<S>(v: &Option<Voltage>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match v {
        Some(voltage) => serializer.serialize_f64(voltage.as_volts()),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn serialize_macaddr<S>(m: &Option<MacAddr>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match m {
        Some(macaddr) => serializer.serialize_str(&macaddr.to_string()),
        None => serializer.serialize_none(),
    }
}

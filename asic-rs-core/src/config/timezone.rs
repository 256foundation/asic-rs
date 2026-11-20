//! Firmware-independent timezone configuration using canonical IANA zones.

use std::{fmt, str::FromStr};

#[cfg(feature = "python")]
use asic_rs_pydantic::{PyPydanticType, PydanticSchemaMode};
use jiff::{
    Timestamp,
    tz::{Offset, TimeZone},
};
#[cfg(feature = "python")]
use pyo3::{
    Borrowed, PyTypeInfo, exceptions::PyValueError, inspect::PyStaticExpr, prelude::*,
    types::PyTzInfo,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A validated IANA timezone name and its rules from the bundled database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tz {
    name: Box<str>,
    rules: TimeZone,
}

impl Tz {
    /// The IANA name used on the wire, including the original alias.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The UTC offset in effect at the given instant.
    pub fn offset_at(&self, instant: Timestamp) -> Offset {
        self.rules.to_offset(instant)
    }
}

impl FromStr for Tz {
    type Err = anyhow::Error;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let rules = TimeZone::get(name)?;
        // Jiff accepts case-insensitive names and the Etc/Unknown sentinel.
        // Keep the same case-sensitive IANA names accepted by the wire format.
        anyhow::ensure!(
            rules.iana_name() == Some(name),
            "Unknown IANA timezone {name:?}"
        );
        Ok(Self {
            name: name.into(),
            rules,
        })
    }
}

impl fmt::Display for Tz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl Serialize for Tz {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Tz {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "python")]
impl<'py> IntoPyObject<'py> for &Tz {
    type Target = PyTzInfo;
    type Output = Bound<'py, PyTzInfo>;
    type Error = PyErr;

    const OUTPUT_TYPE: PyStaticExpr = PyTzInfo::TYPE_HINT;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        PyTzInfo::timezone(py, self.name())
    }
}

#[cfg(feature = "python")]
impl<'py> IntoPyObject<'py> for Tz {
    type Target = PyTzInfo;
    type Output = Bound<'py, PyTzInfo>;
    type Error = PyErr;

    const OUTPUT_TYPE: PyStaticExpr = PyTzInfo::TYPE_HINT;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        (&self).into_pyobject(py)
    }
}

#[cfg(feature = "python")]
impl FromPyObject<'_, '_> for Tz {
    type Error = PyErr;

    fn extract(object: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        object
            .getattr("key")?
            .extract::<String>()?
            .parse()
            .map_err(|error: anyhow::Error| PyValueError::new_err(error.to_string()))
    }
}

#[cfg(feature = "python")]
impl PyPydanticType for Tz {
    fn pydantic_schema<'py>(
        core_schema: &Bound<'py, PyAny>,
        _mode: PydanticSchemaMode,
    ) -> PyResult<Bound<'py, PyAny>> {
        core_schema.call_method0("str_schema")
    }

    fn from_pydantic(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(tz) = value.extract::<Self>() {
            return Ok(tz);
        }
        let name = value.extract::<String>()?;
        name.parse().map_err(|error| {
            PyValueError::new_err(format!("Invalid IANA timezone {name:?}: {error}"))
        })
    }

    fn to_pydantic_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self.name().into_pyobject(py)?.into_any().unbind())
    }
}

#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Timezone configuration.
///
/// The zone is a [`Tz`] on every firmware, such as `Europe/Vienna` or
/// `Etc/GMT-2`. Each firmware backend translates to and from its native
/// representation and determines which zones it accepts.
///
/// Over the wire (serde, `model_dump`) the zones are their IANA names. From
/// Python the fields are `zoneinfo.ZoneInfo` objects; the constructor and the
/// pydantic validator take either a `ZoneInfo` or an IANA name.
pub struct TimezoneConfig {
    /// The configured timezone.
    pub timezone: Option<Tz>,
    /// The timezones the miner accepts.
    pub available: Vec<Tz>,
}

#[cfg(feature = "python")]
#[pymethods]
impl TimezoneConfig {
    #[new]
    #[pyo3(signature = (timezone: "tzinfo | str | None" = None, available: "list[tzinfo | str] | None" = None))]
    fn py_new(
        timezone: Option<&Bound<'_, PyAny>>,
        available: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        use asic_rs_pydantic::PyPydanticType;
        Ok(Self {
            timezone: timezone
                .map(<Tz as PyPydanticType>::from_pydantic)
                .transpose()?,
            available: available
                .map(<Vec<Tz> as PyPydanticType>::from_pydantic)
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On the wire the zones are their IANA names, and an unknown name is
    /// rejected on the way in rather than carried along as text.
    #[test]
    fn config_serializes_zones_as_iana_names() -> anyhow::Result<()> {
        let config = TimezoneConfig {
            timezone: Some("Europe/Vienna".parse()?),
            available: vec!["US/Eastern".parse()?, "Etc/GMT-2".parse()?],
        };
        let json = serde_json::to_value(&config)?;
        assert_eq!(
            json,
            serde_json::json!({
                "timezone": "Europe/Vienna",
                "available": ["US/Eastern", "Etc/GMT-2"],
            })
        );

        let back: TimezoneConfig = serde_json::from_value(json)?;
        assert_eq!(back.timezone, config.timezone);
        assert_eq!(back.available, config.available);

        let empty: TimezoneConfig = serde_json::from_str(r#"{"timezone":null,"available":[]}"#)?;
        assert_eq!(empty.timezone, None);

        let bogus: Result<TimezoneConfig, _> =
            serde_json::from_str(r#"{"timezone":"GMT+2","available":[]}"#);
        assert!(bogus.is_err());
        assert!("Etc/Unknown".parse::<Tz>().is_err());
        assert!("europe/vienna".parse::<Tz>().is_err());
        Ok(())
    }
}

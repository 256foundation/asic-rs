use std::{
    fmt::{Display, Formatter},
    ops::Div,
    str::FromStr,
};

#[cfg(feature = "python")]
use asic_rs_pydantic::py_to_string;
use measurements::Power;
#[cfg(feature = "python")]
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyAnyMethods, PyType},
};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "python", pyclass(from_py_object, str, module = "asic_rs"))]
#[cfg_attr(feature = "python", derive(asic_rs_pydantic::PyPydanticEnum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HashRateUnit {
    #[cfg_attr(feature = "python", pydantic(value = "H/s"))]
    Hash,
    #[cfg_attr(feature = "python", pydantic(value = "KH/s"))]
    KiloHash,
    #[cfg_attr(feature = "python", pydantic(value = "MH/s"))]
    MegaHash,
    #[cfg_attr(feature = "python", pydantic(value = "GH/s"))]
    GigaHash,
    #[cfg_attr(feature = "python", pydantic(value = "TH/s"))]
    #[default]
    TeraHash,
    #[cfg_attr(feature = "python", pydantic(value = "PH/s"))]
    PetaHash,
    #[cfg_attr(feature = "python", pydantic(value = "EH/s"))]
    ExaHash,
    #[cfg_attr(feature = "python", pydantic(value = "ZH/s"))]
    ZettaHash,
    #[cfg_attr(feature = "python", pydantic(value = "YH/s"))]
    YottaHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashRateUnitParseError {
    input: String,
}

impl Display for HashRateUnitParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown hash rate unit: {}", self.input)
    }
}

impl std::error::Error for HashRateUnitParseError {}

impl HashRateUnit {
    fn to_multiplier(self) -> u128 {
        match self {
            HashRateUnit::Hash => 1,
            HashRateUnit::KiloHash => 1_000,
            HashRateUnit::MegaHash => 1_000_000,
            HashRateUnit::GigaHash => 1_000_000_000,
            HashRateUnit::TeraHash => 1_000_000_000_000,
            HashRateUnit::PetaHash => 1_000_000_000_000_000,
            HashRateUnit::ExaHash => 1_000_000_000_000_000_000,
            HashRateUnit::ZettaHash => 1_000_000_000_000_000_000_000,
            HashRateUnit::YottaHash => 1_000_000_000_000_000_000_000_000,
        }
    }
}

impl FromStr for HashRateUnit {
    type Err = HashRateUnitParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_uppercase().replace([' ', '_'], "");

        match normalized.as_str() {
            "HASH" | "H" | "HS" | "H/S" => Ok(HashRateUnit::Hash),
            "KILOHASH" | "KH" | "KHS" | "KH/S" => Ok(HashRateUnit::KiloHash),
            "MEGAHASH" | "MH" | "MHS" | "MH/S" => Ok(HashRateUnit::MegaHash),
            "GIGAHASH" | "GH" | "GHS" | "GH/S" => Ok(HashRateUnit::GigaHash),
            "TERAHASH" | "TH" | "THS" | "TH/S" => Ok(HashRateUnit::TeraHash),
            "PETAHASH" | "PH" | "PHS" | "PH/S" => Ok(HashRateUnit::PetaHash),
            "EXAHASH" | "EH" | "EHS" | "EH/S" => Ok(HashRateUnit::ExaHash),
            "ZETTAHASH" | "ZH" | "ZHS" | "ZH/S" => Ok(HashRateUnit::ZettaHash),
            "YOTTAHASH" | "YH" | "YHS" | "YH/S" => Ok(HashRateUnit::YottaHash),
            _ => Err(HashRateUnitParseError {
                input: s.to_string(),
            }),
        }
    }
}

impl Display for HashRateUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HashRateUnit::Hash => write!(f, "H/s"),
            HashRateUnit::KiloHash => write!(f, "KH/s"),
            HashRateUnit::MegaHash => write!(f, "MH/s"),
            HashRateUnit::GigaHash => write!(f, "GH/s"),
            HashRateUnit::TeraHash => write!(f, "TH/s"),
            HashRateUnit::PetaHash => write!(f, "PH/s"),
            HashRateUnit::ExaHash => write!(f, "EH/s"),
            HashRateUnit::ZettaHash => write!(f, "ZH/s"),
            HashRateUnit::YottaHash => write!(f, "YH/s"),
        }
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl HashRateUnit {
    #[classattr]
    const H: Self = Self::Hash;

    #[classattr]
    const KH: Self = Self::KiloHash;

    #[classattr]
    const MH: Self = Self::MegaHash;

    #[classattr]
    const GH: Self = Self::GigaHash;

    #[classattr]
    const TH: Self = Self::TeraHash;

    #[classattr]
    const PH: Self = Self::PetaHash;

    #[classattr]
    const EH: Self = Self::ExaHash;

    #[classattr]
    const ZH: Self = Self::ZettaHash;

    #[classattr]
    const YH: Self = Self::YottaHash;

    #[classattr]
    #[pyo3(name = "default")]
    const DEFAULT: Self = Self::TeraHash;

    #[classmethod]
    #[pyo3(name = "from_str")]
    fn py_from_str(_cls: &Bound<'_, PyType>, value: &str) -> PyResult<Self> {
        Self::from_str(value).map_err(|error| PyValueError::new_err(error.to_string()))
    }

    #[getter]
    fn value(&self) -> u128 {
        self.to_multiplier()
    }

    fn __int__(&self) -> u128 {
        self.to_multiplier()
    }

    fn __repr__(&self) -> String {
        self.to_string()
    }
}

#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRate {
    /// The current amount of hashes being computed
    pub value: f64,
    /// The unit of the hashes in value
    #[cfg_attr(feature = "python", pydantic_data(to_string))]
    pub unit: HashRateUnit,
    /// The algorithm of the computed hashes
    pub algo: String,
}

impl HashRate {
    pub fn as_unit(self, unit: HashRateUnit) -> Self {
        let base = self.value * self.unit.to_multiplier() as f64; // Convert to base unit.

        Self {
            value: base / unit.to_multiplier() as f64,
            unit,
            algo: self.algo,
        }
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl HashRate {
    #[new]
    #[pyo3(signature = (value, unit: "HashRateUnit | None" = None, algo: "HashAlgorithm | str | None" = None))]
    fn new(
        value: f64,
        unit: Option<HashRateUnit>,
        algo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            value,
            unit: unit.unwrap_or_default(),
            algo: algo
                .map(py_to_string)
                .transpose()?
                .unwrap_or_else(|| "SHA256".to_string()),
        })
    }

    #[pyo3(signature = (unit: "HashRateUnit"))]
    pub fn into_unit(&self, unit: HashRateUnit) -> Self {
        self.clone().as_unit(unit)
    }

    #[pyo3(name = "as_unit")]
    #[pyo3(signature = (unit: "HashRateUnit"))]
    pub fn py_as_unit(&self, unit: HashRateUnit) -> Self {
        self.into_unit(unit)
    }

    fn __float__(&self) -> f64 {
        self.value
    }

    fn __str__(&self) -> String {
        self.to_string()
    }

    fn __format__(&self, py: Python<'_>, format_spec: &str) -> PyResult<String> {
        let builtins = py.import("builtins")?;
        let formatted_value: String = builtins
            .call_method1("format", (self.value, format_spec))?
            .extract()?;
        Ok(format!("{formatted_value} {}", self.unit))
    }
}

impl Display for HashRate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let precision = f.precision();

        match precision {
            Some(precision) => {
                write!(f, "{:.*} {}", precision, self.value, self.unit)
            }
            None => {
                write!(f, "{} {}", self.value, self.unit)
            }
        }
    }
}

impl PartialEq for HashRate {
    fn eq(&self, other: &Self) -> bool {
        other.clone().as_unit(self.unit).value == self.value
    }
}

impl Eq for HashRate {}

impl Div<HashRate> for Power {
    type Output = f64;

    fn div(self, hash_rate: HashRate) -> Self::Output {
        self.as_watts() / hash_rate.value
    }
}

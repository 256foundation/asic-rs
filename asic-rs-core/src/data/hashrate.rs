use std::{
    fmt::{Display, Formatter},
    ops::Div,
    str::FromStr,
};

#[cfg(feature = "python")]
use asic_rs_pydantic::py_to_string;
use measurements::Power;

use crate::data::device::HashAlgorithm;
#[cfg(feature = "python")]
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyAnyMethods, PyType},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[cfg_attr(feature = "python", pyclass(from_py_object, module = "asic_rs"))]
#[cfg_attr(feature = "python", derive(asic_rs_pydantic::PyPydanticEnum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, TS)]
/// Unit used to represent a [`HashRate`] value.
pub enum HashRateUnit {
    /// Hashes per second.
    #[cfg_attr(feature = "python", pydantic(value = "H/s"))]
    Hash,
    /// Kilohashes per second.
    #[cfg_attr(feature = "python", pydantic(value = "KH/s"))]
    KiloHash,
    /// Megahashes per second.
    #[cfg_attr(feature = "python", pydantic(value = "MH/s"))]
    MegaHash,
    /// Gigahashes per second.
    #[cfg_attr(feature = "python", pydantic(value = "GH/s"))]
    GigaHash,
    /// Terahashes per second.
    #[cfg_attr(feature = "python", pydantic(value = "TH/s"))]
    #[default]
    TeraHash,
    /// Petahashes per second.
    #[cfg_attr(feature = "python", pydantic(value = "PH/s"))]
    PetaHash,
    /// Exahashes per second.
    #[cfg_attr(feature = "python", pydantic(value = "EH/s"))]
    ExaHash,
    /// Zettahashes per second.
    #[cfg_attr(feature = "python", pydantic(value = "ZH/s"))]
    ZettaHash,
    /// Yottahashes per second.
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
    /// Compare against another [`HashRateUnit`] or against its rendered name
    /// (`"TH/s"`). See `HashAlgorithm::__eq__` for why the string form is
    /// accepted.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(other) = other.extract::<HashRateUnit>() {
            return *self == other;
        }
        other
            .extract::<String>()
            .is_ok_and(|name| name == self.to_string())
    }

    fn __ne__(&self, other: &Bound<'_, PyAny>) -> bool {
        !self.__eq__(other)
    }

    /// Defining `__eq__` drops the inherited hash, so restore it explicitly.
    fn __hash__(&self) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

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

    fn __str__(&self) -> String {
        self.to_string()
    }
}

#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
/// Hashrate value with unit and mining algorithm.
pub struct HashRate {
    /// The current amount of hashes being computed
    pub value: f64,
    /// The unit of the hashes in value
    #[cfg_attr(feature = "python", pydantic_data(to_string))]
    pub unit: HashRateUnit,
    /// The algorithm of the computed hashes
    #[cfg_attr(feature = "python", pydantic_data(to_string))]
    pub algo: HashAlgorithm,
}

impl HashRate {
    /// Return the conventional display unit for this hashrate's algorithm.
    pub const fn default_unit(&self) -> HashRateUnit {
        Self::default_unit_for_algorithm(self.algo)
    }

    /// Return the conventional display unit for a mining algorithm.
    pub const fn default_unit_for_algorithm(algo: HashAlgorithm) -> HashRateUnit {
        match algo {
            HashAlgorithm::Scrypt | HashAlgorithm::X11 => HashRateUnit::GigaHash,
            HashAlgorithm::EtHash => HashRateUnit::MegaHash,
            HashAlgorithm::Equihash => HashRateUnit::KiloHash,
            HashAlgorithm::SHA256
            | HashAlgorithm::Blake2S256
            | HashAlgorithm::Kadena
            | HashAlgorithm::KHeavyHash
            | HashAlgorithm::Eaglesong
            | HashAlgorithm::Handshake
            | HashAlgorithm::Blake256R14 => HashRateUnit::TeraHash,
            HashAlgorithm::Unknown => HashRateUnit::Hash,
        }
    }

    /// Return this hashrate converted into another unit.
    pub fn as_unit(self, unit: HashRateUnit) -> Self {
        let base = self.value * self.unit.to_multiplier() as f64; // Convert to base unit.

        Self {
            value: base / unit.to_multiplier() as f64,
            unit,
            algo: self.algo,
        }
    }

    /// Return this hashrate converted into the conventional unit for its algorithm.
    pub fn as_default_unit(self) -> Self {
        let unit = self.default_unit();
        self.as_unit(unit)
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
        let algo = algo
            .map(|algo| {
                let name = py_to_string(algo)?;
                HashAlgorithm::from_str(&name)
                    .map_err(|_| PyValueError::new_err(format!("unknown hash algorithm: {name}")))
            })
            .transpose()?
            .unwrap_or(HashAlgorithm::SHA256);

        Ok(Self {
            value,
            unit: unit.unwrap_or_else(|| Self::default_unit_for_algorithm(algo)),
            algo,
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

    /// Return the conventional display unit for this hashrate's algorithm.
    #[pyo3(name = "default_unit")]
    fn py_default_unit(&self) -> HashRateUnit {
        self.default_unit()
    }

    /// Return this hashrate converted into the conventional unit for its algorithm.
    pub fn into_default_unit(&self) -> Self {
        self.clone().as_default_unit()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorithms_have_conventional_default_units() {
        for (algo, unit) in [
            (HashAlgorithm::SHA256, HashRateUnit::TeraHash),
            (HashAlgorithm::Scrypt, HashRateUnit::GigaHash),
            (HashAlgorithm::X11, HashRateUnit::GigaHash),
            (HashAlgorithm::Blake2S256, HashRateUnit::TeraHash),
            (HashAlgorithm::Kadena, HashRateUnit::TeraHash),
            (HashAlgorithm::KHeavyHash, HashRateUnit::TeraHash),
            (HashAlgorithm::Eaglesong, HashRateUnit::TeraHash),
            (HashAlgorithm::EtHash, HashRateUnit::MegaHash),
            (HashAlgorithm::Equihash, HashRateUnit::KiloHash),
            (HashAlgorithm::Handshake, HashRateUnit::TeraHash),
            (HashAlgorithm::Blake256R14, HashRateUnit::TeraHash),
            (HashAlgorithm::Unknown, HashRateUnit::Hash),
        ] {
            let hashrate = HashRate {
                value: 1.0,
                unit: HashRateUnit::Hash,
                algo,
            };
            assert_eq!(hashrate.default_unit(), unit, "{algo}");
        }
    }

    #[test]
    fn as_default_unit_uses_the_algorithm() {
        let hashrate = HashRate {
            value: 16_200.0,
            unit: HashRateUnit::MegaHash,
            algo: HashAlgorithm::Scrypt,
        }
        .as_default_unit();

        assert_eq!(hashrate.unit, HashRateUnit::GigaHash);
        assert_eq!(hashrate.value, 16.2);
        assert_eq!(hashrate.to_string(), "16.2 GH/s");
    }
}

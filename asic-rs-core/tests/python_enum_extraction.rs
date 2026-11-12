#![cfg(feature = "python")]

use std::fmt::Debug;

use asic_rs_core::data::{
    collector::{DataField, FieldCollectionStatus},
    device::HashAlgorithm,
    hashrate::HashRateUnit,
    miner::MiningMode,
};
use pyo3::{
    PyClass, PyClassInitializer,
    prelude::*,
    pyclass::{PyClassGuardError, boolean_struct::False},
};

fn check_extraction<T>(py: Python<'_>, expected: T) -> PyResult<()>
where
    T: PyClass<Frozen = False> + Copy + Debug + PartialEq + Into<PyClassInitializer<T>>,
    for<'a, 'py> T: FromPyObject<'a, 'py, Error = PyClassGuardError<'a, 'py>>,
{
    let object = Bound::new(py, expected)?;
    assert_eq!(object.as_any().extract::<T>()?, expected);
    assert_eq!(object.as_any().extract::<T>()?, expected);
    assert!(py.None().extract::<T>(py).is_err());
    assert!("invalid".into_pyobject(py)?.extract::<T>().is_err());

    let guard = object.try_borrow_mut()?;
    assert!(object.as_any().extract::<T>().is_err());
    drop(guard);
    assert_eq!(object.as_any().extract::<T>()?, expected);
    Ok(())
}

#[test]
fn copy_enum_extraction_preserves_values_and_borrow_errors() -> PyResult<()> {
    Python::initialize();
    Python::attach(|py| {
        check_extraction(py, DataField::Hashrate)?;
        check_extraction(py, FieldCollectionStatus::Failed)?;
        check_extraction(py, HashAlgorithm::Scrypt)?;
        check_extraction(py, HashRateUnit::GigaHash)?;
        check_extraction(py, MiningMode::High)
    })
}

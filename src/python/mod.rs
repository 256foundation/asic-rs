use pyo3::prelude::*;

mod data;
mod factory;
mod miner;

#[pymodule(module = "asic_rs")]
mod asic_rs {
    use pyo3::prelude::*;

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        pyo3_log::init();
        Ok(())
    }

    #[pymodule_export]
    use asic_rs_core::data::device::HashAlgorithm;
    #[pymodule_export]
    use asic_rs_core::data::hashrate::HashRateUnit;

    #[pymodule_export]
    use super::factory::MinerFactory;
    #[pymodule_export]
    use super::miner::Miner;
}

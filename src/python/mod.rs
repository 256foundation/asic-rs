use pyo3::prelude::*;

mod factory;
mod miner;
mod typing;

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
    use asic_rs_core::data::hashrate::{HashRate, HashRateUnit};
    #[pymodule_export]
    use asic_rs_core::data::miner::MiningMode;

    #[pymodule_export]
    use super::factory::MinerFactory;
    #[pymodule_export]
    use super::miner::Miner;
    #[pymodule_export]
    use asic_rs_core::config::{
        fan::{FanConfig, FanMode},
        pools::{PoolConfig as Pool, PoolGroupConfig as PoolGroup},
        scaling::ScalingConfig,
        tuning::TuningConfig,
    };
    #[pymodule_export]
    use asic_rs_core::data::{
        board::{BoardData, ChipData, MinerControlBoard},
        device::{DeviceInfo, MinerHardware},
        fan::FanData,
        message::{MessageSeverity, MinerMessage},
        miner::MinerData,
        pool::{PoolData, PoolGroupData, PoolScheme, PoolURL},
    };
}

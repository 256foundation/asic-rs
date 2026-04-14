use pyo3::prelude::*;

mod config;
mod data;
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
    use super::config::{
        AutoFanConfig, ManualFanConfig, Pool, PoolGroup, ScalingConfig, TuningConfig,
        TuningConfigHashRate, TuningConfigMode, TuningConfigPower,
    };
    #[pymodule_export]
    use super::data::{
        BoardData, ChipData, DeviceInfo, FanData, MinerControlBoard, MinerData, MinerHardware,
        MinerMessage, PoolData, PoolGroupData, TuningTarget,
    };
    #[pymodule_export]
    use super::factory::MinerFactory;
    #[pymodule_export]
    use super::miner::Miner;
}

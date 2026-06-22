use pyo3::prelude::*;

mod factory;
mod miner;
mod typing;

#[pymodule(module = "pyasic_rs.asic_rs")]
mod asic_rs {
    use pyo3::prelude::*;

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        pyo3_log::init();
        Ok(())
    }

    #[pymodule_export]
    use asic_rs_core::data::collector::DataField;
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
        preset::PresetInfo,
        scaling::ScalingConfig,
        temperature::TemperatureConfig,
        tuning::TuningConfig,
    };
    #[pymodule_export]
    use asic_rs_core::data::{
        board::{BoardData, ChipData, MinerControlBoard},
        capabilities::{
            HashRateTuningCapabilities, PowerTuningCapabilities, PresetTuningCapabilities,
            TuningCapabilities,
        },
        device::{DeviceInfo, MinerHardware},
        fan::FanData,
        message::{MessageSeverity, MinerComponent, MinerMessage},
        miner::{MinerData, PyTuningTarget as TuningTarget},
        pool::{PoolData, PoolGroupData, PoolScheme, PoolURL},
    };
}

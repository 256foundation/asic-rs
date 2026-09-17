//! A hardware-free backend for exercising the real C entry points.
#![allow(clippy::panic, clippy::unwrap_used)]

use asic_rs::core::{
    config::collector::{ConfigCollector, ConfigField, ConfigLocation},
    data::{
        collector::{DataCollector, DataField, DataLocation},
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm, MinerHardware},
        firmware::{FirmwareStats, RestoreStockOsResult},
    },
    traits::miner::*,
};
use async_trait::async_trait;
use serde_json::Value;
use std::{net::IpAddr, time::Duration};

#[derive(Debug)]
pub(crate) struct TestMiner;

impl GetIP for TestMiner {
    fn get_ip(&self) -> IpAddr {
        IpAddr::from([127, 0, 0, 1])
    }
}
impl GetDeviceInfo for TestMiner {
    fn get_device_info(&self) -> DeviceInfo {
        DeviceInfo {
            make: "Test".into(),
            model: "Test".into(),
            firmware: "Test".into(),
            hardware: MinerHardware::default(),
            algo: HashAlgorithm::SHA256,
        }
    }
}
impl GetDataLocations for TestMiner {
    fn get_locations(&self, _: DataField) -> Vec<DataLocation> {
        vec![]
    }
}
impl CollectData for TestMiner {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}
impl GetConfigsLocations for TestMiner {
    fn get_configs_locations(&self, _: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}
impl CollectConfigs for TestMiner {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}
#[async_trait]
impl APIClient for TestMiner {
    async fn get_api_result(&self, _: &MinerCommand) -> anyhow::Result<Value> {
        anyhow::bail!("test backend has no network")
    }
}
impl HasAuth for TestMiner {
    fn set_auth(&mut self, _: MinerAuth) {}
}
impl HasDefaultAuth for TestMiner {}
#[async_trait]
impl Miner for TestMiner {
    async fn revalidate(&self) -> anyhow::Result<bool> {
        anyhow::bail!("revalidation failed")
    }
}
#[async_trait]
impl UpgradeFirmware for TestMiner {
    async fn check_firmware_update(&self) -> anyhow::Result<FirmwareStats> {
        anyhow::bail!("firmware check failed")
    }
}
#[async_trait]
impl GetUptime for TestMiner {
    async fn get_uptime(&self) -> Option<Duration> {
        Some(Duration::new(42, 123_456_789))
    }
}
#[async_trait]
impl Restart for TestMiner {
    fn supports_restart(&self) -> bool {
        true
    }
    async fn restart(&self) -> anyhow::Result<bool> {
        panic!("backend restart panic")
    }
}
#[async_trait]
impl RestoreStockOs for TestMiner {
    fn supports_restore_stock_os(&self) -> bool {
        true
    }

    async fn restore_stock_os(&self) -> anyhow::Result<RestoreStockOsResult> {
        Ok(RestoreStockOsResult::accepted(Some(7)))
    }
}
macro_rules! default_traits {
    ($($name:ident),* $(,)?) => { $(impl $name for TestMiner {})* };
}
default_traits!(
    GetMAC,
    GetSerialNumber,
    GetHostname,
    GetApiVersion,
    GetFirmwareVersion,
    GetHashboards,
    GetHashrate,
    GetExpectedHashrate,
    GetFans,
    GetPsuFans,
    GetFluidTemperature,
    GetWattage,
    GetTuningPercent,
    GetTuningTarget,
    GetScaledTuningTarget,
    GetTuningCapabilities,
    GetLightFlashing,
    GetMessages,
    GetIsMining,
    GetOperatingState,
    GetDevFeeConnected,
    GetPools,
    GetBestShare,
    GetSessionBestShare,
    SetTuningPercent,
    SupportsPresets,
    SupportsTemperatureConfig,
    SupportsTimezoneConfig,
    SupportsTuningConfig,
    SupportsFanConfig
);
macro_rules! unsupported_traits {
    ($($name:ident => $method:ident),* $(,)?) => {
        $(impl $name for TestMiner { fn $method(&self) -> bool { false } })*
    };
}
unsupported_traits!(SetFaultLight => supports_set_fault_light, SetPowerLimit => supports_set_power_limit,
    Pause => supports_pause, Resume => supports_resume, ChangePassword => supports_change_password,
    FactoryReset => supports_factory_reset, ReadLogs => supports_read_logs,
    SupportsPoolsConfig => supports_pools_config, SupportsScalingConfig => supports_scaling_config);

impl GetControlBoardVersion for TestMiner {
    fn parse_control_board_version(
        &self,
        _: &std::collections::HashMap<DataField, Value>,
    ) -> Option<asic_rs::core::data::board::MinerControlBoard> {
        Some(asic_rs::core::data::board::MinerControlBoard::unknown(
            "new-board".into(),
        ))
    }
}

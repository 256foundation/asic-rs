use anyhow;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{Power, Temperature};
use reqwest::Method;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Debug;
use std::net::IpAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing;

use crate::config::pools::PoolGroup;
use crate::data::board::BoardData;
use crate::data::device::{DeviceInfo, MinerControlBoard, MinerModel};
use crate::data::fan::FanData;
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::message::MinerMessage;
use crate::data::pool::PoolData;
use crate::miners::commands::MinerCommand;

use crate::data::miner::MinerData;
use crate::miners::data::{DataCollector, DataField, DataLocation};

pub(crate) trait MinerConstructor {
    #[allow(clippy::new_ret_no_self)]
    fn new(ip: IpAddr, model: MinerModel, version: Option<semver::Version>) -> Box<dyn Miner>;
}

pub trait Miner: GetMinerData + HasMinerControl {}

impl<T: GetMinerData + HasMinerControl> Miner for T {}

pub trait HasMinerControl:
    SetFaultLight + SetPowerLimit + SetPools + Restart + Resume + Pause
{
}

impl<T: SetFaultLight + SetPowerLimit + SetPools + Restart + Resume + Pause> HasMinerControl for T {}

/// Trait that every miner backend must implement to provide miner data.
#[async_trait]
pub trait GetMinerData:
    CollectData
    + MinerInterface
    + GetIP
    + GetDeviceInfo
    + GetExpectedHashboards
    + GetExpectedChips
    + GetExpectedFans
    + GetMAC
    + GetSerialNumber
    + GetHostname
    + GetApiVersion
    + GetFirmwareVersion
    + GetControlBoardVersion
    + GetHashboards
    + GetHashrate
    + GetExpectedHashrate
    + GetFans
    + GetPsuFans
    + GetFluidTemperature
    + GetWattage
    + GetWattageLimit
    + GetLightFlashing
    + GetMessages
    + GetUptime
    + GetIsMining
    + GetPools
{
    /// Asynchronously retrieves standardized information about a miner,
    /// returning it as a `MinerData` struct.
    async fn get_data(&self) -> MinerData;
    fn parse_data(&self, data: HashMap<DataField, Value>) -> MinerData;
}

pub trait CollectData: GetDataLocations {
    /// Returns a `DataCollector` that can be used to collect data from the miner.
    ///
    /// This method is responsible for creating and returning a `DataCollector`
    /// instance that can be used to collect data from the miner.
    fn get_collector(&self) -> DataCollector<'_>;
}

pub trait MinerInterface: GetDataLocations + APIClient {}

impl<T: GetDataLocations + APIClient> MinerInterface for T {}

pub trait GetDataLocations: Send + Sync + Debug {
    /// Returns the locations of the specified data field on the miner.
    ///
    /// This associates API commands (routes) with `DataExtractor` structs,
    /// describing how to extract the data for a given `DataField`.
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation>;
}

#[async_trait]
impl<
    T: GetIP
        + GetDeviceInfo
        + GetExpectedHashboards
        + GetExpectedChips
        + GetExpectedFans
        + GetMAC
        + GetSerialNumber
        + GetHostname
        + GetApiVersion
        + GetFirmwareVersion
        + GetControlBoardVersion
        + GetHashboards
        + GetHashrate
        + GetExpectedHashrate
        + GetFans
        + GetPsuFans
        + GetFluidTemperature
        + GetWattage
        + GetWattageLimit
        + GetLightFlashing
        + GetMessages
        + GetUptime
        + GetIsMining
        + GetPools
        + MinerInterface,
> GetMinerData for T
{
    async fn get_data(&self) -> MinerData {
        let mut collector = self.get_collector();
        let data = collector.collect_all().await;
        self.parse_data(data)
    }
    fn parse_data(&self, data: HashMap<DataField, Value>) -> MinerData {
        let schema_version = env!("CARGO_PKG_VERSION").to_string();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_secs();

        let ip = self.get_ip();
        let mac = self.parse_mac(&data);
        let serial_number = self.parse_serial_number(&data);
        let hostname = self.parse_hostname(&data);
        let api_version = self.parse_api_version(&data);
        let firmware_version = self.parse_firmware_version(&data);
        let control_board_version = self.parse_control_board_version(&data);
        let uptime = self.parse_uptime(&data);
        let hashrate = self.parse_hashrate(&data);
        let expected_hashrate = self.parse_expected_hashrate(&data);
        let wattage = self.parse_wattage(&data);
        let wattage_limit = self.parse_wattage_limit(&data);
        let fluid_temperature = self.parse_fluid_temperature(&data);
        let fans = self.parse_fans(&data);
        let psu_fans = self.parse_psu_fans(&data);
        let hashboards = self.parse_hashboards(&data);
        let light_flashing = self.parse_light_flashing(&data);
        let is_mining = self.parse_is_mining(&data);
        let messages = self.parse_messages(&data);
        let pools = self.parse_pools(&data);
        let device_info = self.get_device_info();

        // computed fields
        let total_chips = {
            let chips = hashboards
                .iter()
                .filter_map(|b| b.working_chips)
                .collect::<Vec<u16>>();

            if !chips.is_empty() {
                Some(chips.iter().sum())
            } else {
                None
            }
        };
        let average_temperature = {
            let board_temps = hashboards
                .iter()
                .map(|b| b.board_temperature)
                .filter(|x| x.is_some())
                .map(|x| x.unwrap().as_celsius())
                .collect::<Vec<f64>>();
            if !board_temps.is_empty() {
                Some(Temperature::from_celsius(
                    board_temps.iter().sum::<f64>() / hashboards.len() as f64,
                ))
            } else {
                None
            }
        };
        let efficiency = match (hashrate.as_ref(), wattage.as_ref()) {
            (Some(hr), Some(w)) => {
                let hashrate_th = hr.clone().as_unit(HashRateUnit::TeraHash).value;
                Some(w.as_watts() / hashrate_th)
            }
            _ => None,
        };

        MinerData {
            // Version information
            schema_version,
            timestamp,

            // Network identification
            ip,
            mac,

            // Device identification
            device_info,
            serial_number,
            hostname,

            // Version information
            api_version,
            firmware_version,
            control_board_version,

            // Hashboard information
            expected_hashboards: device_info.hardware.boards,
            hashboards,
            hashrate,
            expected_hashrate,

            // Chip information
            expected_chips: Some(
                device_info.hardware.chips.unwrap_or(0)
                    * device_info.hardware.boards.map(|u| u as u16).unwrap_or(0),
            ),
            total_chips,

            // Cooling information
            expected_fans: device_info.hardware.fans,
            fans,
            psu_fans,
            average_temperature,
            fluid_temperature,

            // Power information
            wattage,
            wattage_limit,
            efficiency,

            // Status information
            light_flashing,
            messages,
            uptime,
            is_mining,

            pools,
        }
    }
}

#[async_trait]
pub trait APIClient: Send + Sync {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value>;
}

#[async_trait]
pub trait WebAPIClient: APIClient {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
        method: Method,
    ) -> anyhow::Result<Value>;
}

#[async_trait]
pub trait RPCAPIClient: APIClient {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        parameters: Option<Value>,
    ) -> anyhow::Result<Value>;
}

// Data traits
pub trait GetIP: Send + Sync {
    /// Returns the IP address of the miner.
    fn get_ip(&self) -> IpAddr;
}

pub trait GetDeviceInfo: Send + Sync {
    /// Returns information about the miner.
    fn get_device_info(&self) -> DeviceInfo;
}

pub trait GetExpectedHashboards: GetDeviceInfo {
    #[allow(dead_code)]
    fn get_expected_hashboards(&self) -> Option<u8> {
        self.get_device_info().hardware.boards
    }
}
impl<T: GetDeviceInfo> GetExpectedHashboards for T {}

pub trait GetExpectedChips: GetDeviceInfo {
    #[allow(dead_code)]
    fn get_expected_chips(&self) -> Option<u16> {
        self.get_device_info().hardware.chips
    }
}
impl<T: GetDeviceInfo> GetExpectedChips for T {}

pub trait GetExpectedFans: GetDeviceInfo {
    #[allow(dead_code)]
    fn get_expected_fans(&self) -> Option<u8> {
        self.get_device_info().hardware.fans
    }
}
impl<T: GetDeviceInfo> GetExpectedFans for T {}

// MAC Address
#[async_trait]
pub trait GetMAC: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_mac(&self) -> Option<MacAddr> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::Mac]).await;
        self.parse_mac(&data)
    }
    #[allow(unused_variables)]
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        None
    }
}

// Serial Number
#[async_trait]
pub trait GetSerialNumber: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_serial_number(&self) -> Option<String> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::SerialNumber]).await;
        self.parse_serial_number(&data)
    }
    #[allow(unused_variables)]
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        None
    }
}

// Hostname
#[async_trait]
pub trait GetHostname: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_hostname(&self) -> Option<String> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::Hostname]).await;
        self.parse_hostname(&data)
    }
    #[allow(unused_variables)]
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        None
    }
}

// API Version
#[async_trait]
pub trait GetApiVersion: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_api_version(&self) -> Option<String> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::ApiVersion]).await;
        self.parse_api_version(&data)
    }
    #[allow(unused_variables)]
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        None
    }
}

// Firmware Version
#[async_trait]
pub trait GetFirmwareVersion: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_firmware_version(&self) -> Option<String> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::FirmwareVersion]).await;
        self.parse_firmware_version(&data)
    }
    #[allow(unused_variables)]
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        None
    }
}

// Control Board Version
#[async_trait]
pub trait GetControlBoardVersion: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_control_board_version(&self) -> Option<MinerControlBoard> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::ControlBoardVersion]).await;
        self.parse_control_board_version(&data)
    }
    #[allow(unused_variables)]
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        None
    }
}
// Hashboards
#[async_trait]
pub trait GetHashboards: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_hashboards(&self) -> Vec<BoardData> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::Hashboards]).await;
        self.parse_hashboards(&data)
    }
    #[allow(unused_variables)]
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        vec![]
    }
}

// Hashrate
#[async_trait]
pub trait GetHashrate: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_hashrate(&self) -> Option<HashRate> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::Hashrate]).await;
        self.parse_hashrate(&data)
            .map(|hr| hr.as_unit(HashRateUnit::default()))
    }
    #[allow(unused_variables)]
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        None
    }
}

// Expected Hashrate
#[async_trait]
pub trait GetExpectedHashrate: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_expected_hashrate(&self) -> Option<HashRate> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::ExpectedHashrate]).await;
        self.parse_expected_hashrate(&data)
            .map(|hr| hr.as_unit(HashRateUnit::default()))
    }
    #[allow(unused_variables)]
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        None
    }
}

// Fans
#[async_trait]
pub trait GetFans: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_fans(&self) -> Vec<FanData> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::Fans]).await;
        self.parse_fans(&data)
    }
    #[allow(unused_variables)]
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        vec![]
    }
}

// PSU Fans
#[async_trait]
pub trait GetPsuFans: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_psu_fans(&self) -> Vec<FanData> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::PsuFans]).await;
        self.parse_psu_fans(&data)
    }
    #[allow(unused_variables)]
    fn parse_psu_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        vec![]
    }
}

// Fluid Temperature
#[async_trait]
pub trait GetFluidTemperature: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_fluid_temperature(&self) -> Option<Temperature> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::FluidTemperature]).await;
        self.parse_fluid_temperature(&data)
    }
    #[allow(unused_variables)]
    fn parse_fluid_temperature(&self, data: &HashMap<DataField, Value>) -> Option<Temperature> {
        None
    }
}

// Wattage
#[async_trait]
pub trait GetWattage: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_wattage(&self) -> Option<Power> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::Wattage]).await;
        self.parse_wattage(&data)
    }
    #[allow(unused_variables)]
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        None
    }
}

// Wattage Limit
#[async_trait]
pub trait GetWattageLimit: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_wattage_limit(&self) -> Option<Power> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::WattageLimit]).await;
        self.parse_wattage_limit(&data)
    }
    #[allow(unused_variables)]
    fn parse_wattage_limit(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        None
    }
}

// Light Flashing
#[async_trait]
pub trait GetLightFlashing: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_light_flashing(&self) -> Option<bool> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::LightFlashing]).await;
        self.parse_light_flashing(&data)
    }
    #[allow(unused_variables)]
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        None
    }
}

// Messages
#[async_trait]
pub trait GetMessages: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_messages(&self) -> Vec<MinerMessage> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::Messages]).await;
        self.parse_messages(&data)
    }
    #[allow(unused_variables)]
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        vec![]
    }
}

// Uptime
#[async_trait]
pub trait GetUptime: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_uptime(&self) -> Option<Duration> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::Uptime]).await;
        self.parse_uptime(&data)
    }
    #[allow(unused_variables)]
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        None
    }
}

// Is Mining
#[async_trait]
pub trait GetIsMining: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_is_mining(&self) -> bool {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::IsMining]).await;
        self.parse_is_mining(&data)
    }
    #[allow(unused_variables)]
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        true
    }
}

// Pools
#[async_trait]
pub trait GetPools: CollectData {
    #[tracing::instrument(level = "debug")]
    async fn get_pools(&self) -> Vec<PoolData> {
        let mut collector = self.get_collector();
        let data = collector.collect(&[DataField::Pools]).await;
        self.parse_pools(&data)
    }
    #[allow(unused_variables)]
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolData> {
        vec![]
    }
}

// Setters
#[async_trait]
pub trait SetFaultLight {
    #[allow(unused_variables)]
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        anyhow::bail!("Setting fault light is not supported on this platform");
    }
    fn supports_set_fault_light(&self) -> bool;
}

#[async_trait]
pub trait SetPowerLimit {
    #[allow(unused_variables)]
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        anyhow::bail!("Setting power limit is not supported on this platform");
    }
    fn supports_set_power_limit(&self) -> bool;
}

#[async_trait]
pub trait SetPools {
    #[allow(unused_variables)]
    async fn set_pools(&self, config: Vec<PoolGroup>) -> anyhow::Result<bool> {
        anyhow::bail!("Setting pools is not supported on this platform");
    }
    fn supports_set_pools(&self) -> bool;
}

#[async_trait]
pub trait Restart {
    async fn restart(&self) -> anyhow::Result<bool> {
        anyhow::bail!("Restarting is not supported on this platform");
    }
    fn supports_restart(&self) -> bool;
}

#[async_trait]
pub trait Pause {
    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        anyhow::bail!("Pausing mining is not supported on this platform");
    }
    fn supports_pause(&self) -> bool;
}

#[async_trait]
pub trait Resume {
    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        anyhow::bail!("Resuming mining is not supported on this platform");
    }
    fn supports_resume(&self) -> bool;
}

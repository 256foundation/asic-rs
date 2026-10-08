use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    str::FromStr,
    time::Duration,
};

use anyhow::Result;
use asic_rs_core::{
    config::collector::{ConfigCollector, ConfigField, ConfigLocation},
    data::{
        board::BoardData,
        collector::{DataCollector, DataExtractor, DataField, DataLocation},
        command::MinerCommand,
        device::DeviceInfo,
        fan::FanData,
        hashrate::{HashRate, HashRateUnit},
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Temperature};
use serde_json::Value;

use crate::firmware::IceRiverStockFirmware;

pub mod web;
use web::IceRiverWebAPI;

pub(crate) const USERPANEL: MinerCommand = MinerCommand::WebAPI {
    command: "userpanel",
    parameters: None,
};

/// HTTP returns `data`; pyasic multicommand recordings wrap it in `userpanel`.
pub(crate) fn panel_data<'a>(response: &'a Value, _key: Option<&str>) -> Option<&'a Value> {
    response
        .get("data")
        .or_else(|| response.pointer("/userpanel/data"))
        .filter(|data| data.is_object())
}

fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.trim().parse().ok())
        .filter(|value| value.is_finite())
}

fn unsigned(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| {
            let number = value.as_f64()?;
            // Stock AL3 firmware serializes counts/positions as JSON floats.
            // Cast only exact nonnegative integers; u64::MAX rounds upward to
            // 2^64 as f64, so the upper bound must remain strict.
            (number.is_finite()
                && number >= 0.0
                && number.fract() == 0.0
                && number < u64::MAX as f64)
                .then_some(number as u64)
        })
        .or_else(|| value.as_str()?.trim().parse().ok())
}

fn boolean(value: &Value) -> Option<bool> {
    value.as_bool().or_else(|| match unsigned(value)? {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    })
}

fn rate_unit(unit: &str) -> Option<HashRateUnit> {
    let unit = unit.trim().to_ascii_uppercase();
    let unit = match unit.as_str() {
        "K" => "KH/s",
        "M" => "MH/s",
        "G" => "GH/s",
        "T" => "TH/s",
        "P" => "PH/s",
        "E" => "EH/s",
        "Z" => "ZH/s",
        "Y" => "YH/s",
        _ => &unit,
    };
    unit.parse().ok()
}

fn rate(value: &Value, fallback_unit: Option<HashRateUnit>, info: &DeviceInfo) -> Option<HashRate> {
    let (value, unit) = if let Some(text) = value.as_str() {
        let text = text.trim();
        let split = text.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '+' && c != '-');
        match split {
            Some(split) => (
                text[..split].parse::<f64>().ok()?,
                rate_unit(&text[split..])?,
            ),
            None => (text.parse::<f64>().ok()?, fallback_unit?),
        }
    } else {
        (number(value)?, fallback_unit?)
    };
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    Some(HashRate {
        value,
        unit,
        algo: info.algo,
    })
}

fn temperature(value: &Value) -> Option<Temperature> {
    number(value)
        .filter(|value| *value > 0.0)
        .map(Temperature::from_celsius)
}

#[derive(Debug)]
pub struct IceRiverV1 {
    ip: IpAddr,
    web: IceRiverWebAPI,
    device_info: DeviceInfo,
}

impl IceRiverV1 {
    pub fn with_auth(ip: IpAddr, model: impl MinerModel, auth: MinerAuth) -> Self {
        let algorithm = model.hash_algorithm();
        Self {
            ip,
            web: IceRiverWebAPI::new(ip, auth),
            device_info: DeviceInfo::new(model, IceRiverStockFirmware, algorithm),
        }
    }
}

#[async_trait]
impl APIClient for IceRiverV1 {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        self.web.get_api_result(command).await
    }
}

impl GetDataLocations for IceRiverV1 {
    fn get_locations(&self, field: DataField) -> Vec<DataLocation> {
        match field {
            DataField::Mac
            | DataField::Hostname
            | DataField::FirmwareVersion
            | DataField::Hashboards
            | DataField::Hashrate
            | DataField::Fans
            | DataField::LightFlashing
            | DataField::Uptime
            | DataField::IsMining
            | DataField::Pools => vec![(
                USERPANEL,
                DataExtractor {
                    func: panel_data,
                    key: None,
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl CollectData for IceRiverV1 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetConfigsLocations for IceRiverV1 {
    fn get_configs_locations(&self, _field: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}

impl CollectConfigs for IceRiverV1 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetIP for IceRiverV1 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for IceRiverV1 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl GetMAC for IceRiverV1 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        MacAddr::from_str(
            &data
                .get(&DataField::Mac)?
                .get("mac")?
                .as_str()?
                .replace('-', ":"),
        )
        .ok()
    }
}

impl GetHostname for IceRiverV1 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::Hostname)?
            .get("host")?
            .as_str()
            .map(str::to_owned)
    }
}

impl GetFirmwareVersion for IceRiverV1 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::FirmwareVersion)?
            .get("softver1")?
            .as_str()
            .map(str::to_owned)
    }
}

impl GetHashboards for IceRiverV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let Some(panel) = data.get(&DataField::Hashboards) else {
            return vec![];
        };
        let Some(reported) = panel.get("boards").and_then(Value::as_array) else {
            return vec![];
        };
        let unit = panel
            .get("unit")
            .and_then(Value::as_str)
            .and_then(rate_unit);
        let mut boards = self
            .device_info
            .hardware
            .boards
            .as_ref()
            .map(|expected| {
                expected
                    .iter()
                    .enumerate()
                    .filter_map(|(position, chips)| {
                        Some(BoardData::new(u8::try_from(position).ok()?, *chips))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut observed_positions = HashSet::new();
        for raw in reported {
            let Some(position) = raw
                .get("no")
                .and_then(unsigned)
                .and_then(|n| n.checked_sub(1))
                .and_then(|n| u8::try_from(n).ok())
            else {
                continue;
            };
            // Duplicate observed identifiers make the entire board snapshot
            // ambiguous. Do not select whichever row happened to arrive last,
            // or publish temperatures/chip totals from conflicting rows.
            if !observed_positions.insert(position) {
                return vec![];
            }
            let existing = boards.iter().position(|board| board.position == position);
            let index = existing.unwrap_or_else(|| {
                boards.push(BoardData::new(
                    position,
                    self.device_info.hardware.chips_for_board(position as usize),
                ));
                boards.len() - 1
            });
            let board = &mut boards[index];
            // Same sensor roles as pyasic's IceRiver handler: intmp is board,
            // outtmp is chip. These are not per-chip readings or hydro coolant.
            board.board_temperature = raw.get("intmp").and_then(temperature);
            board.outlet_chip_temperature = raw.get("outtmp").and_then(temperature);
            board.working_chips = raw
                .get("chipnum")
                .and_then(unsigned)
                .and_then(|n| u16::try_from(n).ok());
            board.hashrate = raw
                .get("rtpow")
                .and_then(|value| rate(value, unit, &self.device_info));
            board.active = board.hashrate.as_ref().map(|rate| rate.value > 0.0);
        }
        boards.sort_by_key(|board| board.position);
        boards
    }
}

impl GetHashrate for IceRiverV1 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        let panel = data.get(&DataField::Hashrate)?;
        let unit = panel
            .get("unit")
            .and_then(Value::as_str)
            .and_then(rate_unit);
        rate(panel.get("rtpow")?, unit, &self.device_info)
    }
}

impl GetFans for IceRiverV1 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        data.get(&DataField::Fans)
            .and_then(|data| data.get("fans"))
            .and_then(Value::as_array)
            .map(|fans| {
                fans.iter()
                    .enumerate()
                    .filter_map(|(position, value)| {
                        let rpm = number(value).filter(|rpm| *rpm >= 0.0)?;
                        Some(FanData {
                            position: i16::try_from(position).ok()?,
                            rpm: Some(AngularVelocity::from_rpm(rpm)),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl GetLightFlashing for IceRiverV1 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        boolean(data.get(&DataField::LightFlashing)?.get("locate")?)
    }
}

impl GetUptime for IceRiverV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        let runtime = data.get(&DataField::Uptime)?.get("runtime")?.as_str()?;
        let parts = runtime
            .split(':')
            .map(str::parse::<u64>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .ok()?;
        let [days, hours, minutes, seconds] = parts.as_slice() else {
            return None;
        };
        if *hours >= 24 || *minutes >= 60 || *seconds >= 60 {
            return None;
        }
        let seconds = days
            .checked_mul(86400)?
            .checked_add(hours.checked_mul(3600)?)?
            .checked_add(minutes.checked_mul(60)?)?
            .checked_add(*seconds)?;
        Some(Duration::from_secs(seconds))
    }
}

impl GetIsMining for IceRiverV1 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        data.get(&DataField::IsMining)
            .and_then(|panel| panel.get("powstate"))
            .and_then(boolean)
            .unwrap_or(false)
    }
}

impl GetPools for IceRiverV1 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let Some(reported) = data
            .get(&DataField::Pools)
            .and_then(|panel| panel.get("pools"))
            .and_then(Value::as_array)
        else {
            return vec![];
        };
        let pools = reported
            .iter()
            .filter_map(|raw| {
                let address = raw.get("addr").and_then(Value::as_str)?;
                if address.trim().is_empty() {
                    return None;
                }
                Some(PoolData {
                    position: raw
                        .get("no")
                        .and_then(unsigned)
                        .and_then(|n| n.checked_sub(1))
                        .and_then(|n| u16::try_from(n).ok()),
                    url: Some(PoolURL::from(address.to_owned())),
                    accepted_shares: raw.get("accepted").and_then(unsigned),
                    rejected_shares: raw.get("rejected").and_then(unsigned),
                    last_share_time: None,
                    active: raw.get("connect").and_then(boolean),
                    alive: raw.get("state").and_then(boolean),
                    user: raw.get("user").and_then(Value::as_str).map(str::to_owned),
                })
            })
            .collect();
        vec![PoolGroupData {
            name: "default".to_owned(),
            quota: 1,
            pools,
        }]
    }
}

// Fields not exposed by the source-backed stock API remain unavailable.
impl GetSerialNumber for IceRiverV1 {}
impl GetApiVersion for IceRiverV1 {}
impl GetControlBoardVersion for IceRiverV1 {}
impl GetExpectedHashrate for IceRiverV1 {}
impl GetPsuFans for IceRiverV1 {}
impl GetFluidTemperature for IceRiverV1 {}
impl GetWattage for IceRiverV1 {}
impl GetTuningPercent for IceRiverV1 {}
impl GetTuningTarget for IceRiverV1 {}
impl GetScaledTuningTarget for IceRiverV1 {}
impl GetTuningCapabilities for IceRiverV1 {}
impl GetMessages for IceRiverV1 {}
impl GetBestShare for IceRiverV1 {}
impl GetSessionBestShare for IceRiverV1 {}
impl GetOperatingState for IceRiverV1 {}
impl GetDevFeeConnected for IceRiverV1 {}

// Explicitly read-only: the default implementations reject every mutation.
impl SetFaultLight for IceRiverV1 {
    fn supports_set_fault_light(&self) -> bool {
        false
    }
}
impl SetPowerLimit for IceRiverV1 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}
impl SetHashboardsEnabled for IceRiverV1 {}
impl SetTuningPercent for IceRiverV1 {}
impl Restart for IceRiverV1 {
    fn supports_restart(&self) -> bool {
        false
    }
}
impl Pause for IceRiverV1 {
    fn supports_pause(&self) -> bool {
        false
    }
}
impl Resume for IceRiverV1 {
    fn supports_resume(&self) -> bool {
        false
    }
}
impl ChangePassword for IceRiverV1 {
    fn supports_change_password(&self) -> bool {
        false
    }
}
impl FactoryReset for IceRiverV1 {
    fn supports_factory_reset(&self) -> bool {
        false
    }
}
impl ReadLogs for IceRiverV1 {
    fn supports_read_logs(&self) -> bool {
        false
    }
}
impl RestoreStockOs for IceRiverV1 {}
impl UpgradeFirmware for IceRiverV1 {}
impl SupportsPoolsConfig for IceRiverV1 {
    fn supports_pools_config(&self) -> bool {
        false
    }
}
impl SupportsScalingConfig for IceRiverV1 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}
impl SupportsTemperatureConfig for IceRiverV1 {}
impl SupportsTimezoneConfig for IceRiverV1 {}
impl SupportsTuningConfig for IceRiverV1 {}
impl SupportsFanConfig for IceRiverV1 {}
impl SupportsPresets for IceRiverV1 {}

impl HasDefaultAuth for IceRiverV1 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("admin", "12345678")
    }
}

impl HasAuth for IceRiverV1 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

#[async_trait]
impl Validate for IceRiverV1 {
    type Firmware = IceRiverStockFirmware;

    async fn revalidate(&self) -> Result<bool> {
        let response = self.web.userpanel().await?;
        let Ok(model) = crate::firmware::model_from_userpanel(&response) else {
            return Ok(false);
        };
        Ok(model.to_string() == self.device_info.model)
    }
}

#[cfg(test)]
mod tests;

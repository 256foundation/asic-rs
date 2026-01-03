use crate::data::board::BoardData;
use crate::data::device::{
    DeviceInfo, HashAlgorithm, MinerControlBoard, MinerFirmware, MinerMake, MinerModel,
};
use crate::data::fan::FanData;
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::message::{MessageSeverity, MinerMessage};
use crate::data::pool::{PoolData, PoolURL};
use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;
use crate::miners::data::{
    DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
};
use anyhow;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fmt::Display;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

use rpc::AntMinerRPCAPI;
use web::AntMinerWebAPI;

mod rpc;
mod web;

#[derive(Debug)]
pub struct AntMinerV2020 {
    pub ip: IpAddr,
    pub rpc: AntMinerRPCAPI,
    pub web: AntMinerWebAPI,
    pub device_info: DeviceInfo,
}

enum MinerMode {
    Sleep,
    Low,
    Normal,
    High,
}

impl Display for MinerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            MinerMode::Normal => "0".to_string(),
            MinerMode::Sleep => "1".to_string(),
            MinerMode::Low => "3".to_string(),
            _ => "0".to_string(),
        };
        write!(f, "{}", str)
    }
}

impl AntMinerV2020 {
    pub fn new(ip: IpAddr, model: MinerModel) -> Self {
        AntMinerV2020 {
            ip,
            rpc: AntMinerRPCAPI::new(ip),
            web: AntMinerWebAPI::new(ip),
            device_info: DeviceInfo::new(
                MinerMake::AntMiner,
                model,
                MinerFirmware::Stock,
                HashAlgorithm::SHA256,
            ),
        }
    }

    pub fn with_auth(
        ip: IpAddr,
        model: MinerModel,
        firmware: MinerFirmware,
        username: String,
        password: String,
    ) -> Self {
        AntMinerV2020 {
            ip,
            rpc: AntMinerRPCAPI::new(ip),
            web: AntMinerWebAPI::with_auth(ip, username, password),
            device_info: DeviceInfo::new(
                MinerMake::AntMiner,
                model,
                firmware,
                HashAlgorithm::SHA256,
            ),
        }
    }

    fn parse_temp_string(temp_str: &str) -> Option<Temperature> {
        let temps: Vec<f64> = temp_str
            .split('-')
            .filter_map(|s| s.parse().ok())
            .filter(|&temp| temp > 0.0)
            .collect();

        if !temps.is_empty() {
            let avg = temps.iter().sum::<f64>() / temps.len() as f64;
            Some(Temperature::from_celsius(avg))
        } else {
            None
        }
    }

    fn _calculate_average_temp_s21_hyd(chain: &Value) -> Option<Temperature> {
        let mut temps = Vec::new();

        if let Some(temp_pic) = chain.get("temp_pic").and_then(|v| v.as_array()) {
            for i in 1..=3 {
                if let Some(temp) = temp_pic.get(i).and_then(|v| v.as_f64())
                    && temp != 0.0
                {
                    temps.push(temp);
                }
            }
        }

        if let Some(temp_pcb) = chain.get("temp_pcb").and_then(|v| v.as_array()) {
            if let Some(temp) = temp_pcb.get(1).and_then(|v| v.as_f64())
                && temp != 0.0
            {
                temps.push(temp);
            }
            if let Some(temp) = temp_pcb.get(3).and_then(|v| v.as_f64())
                && temp != 0.0
            {
                temps.push(temp);
            }
        }

        if !temps.is_empty() {
            let avg = temps.iter().sum::<f64>() / temps.len() as f64;
            Some(Temperature::from_celsius(avg))
        } else {
            None
        }
    }

    fn _calculate_average_temp_pcb(chain: &Value) -> Option<Temperature> {
        if let Some(temp_pcb) = chain.get("temp_pcb").and_then(|v| v.as_array()) {
            let temps: Vec<f64> = temp_pcb
                .iter()
                .filter_map(|v| v.as_f64())
                .filter(|&temp| temp != 0.0)
                .collect();

            if !temps.is_empty() {
                let avg = temps.iter().sum::<f64>() / temps.len() as f64;
                Some(Temperature::from_celsius(avg))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn _calculate_average_temp_chip(chain: &Value) -> Option<Temperature> {
        if let Some(temp_chip) = chain.get("temp_chip").and_then(|v| v.as_array()) {
            let temps: Vec<f64> = temp_chip
                .iter()
                .filter_map(|v| v.as_f64())
                .filter(|&temp| temp != 0.0)
                .collect();

            if !temps.is_empty() {
                let avg = temps.iter().sum::<f64>() / temps.len() as f64;
                Some(Temperature::from_celsius(avg))
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[async_trait]
impl APIClient for AntMinerV2020 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!("Unsupported command type for Antminer API")),
        }
    }
}

impl GetDataLocations for AntMinerV2020 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const RPC_VERSION: MinerCommand = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };

        const RPC_STATS: MinerCommand = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };

        const RPC_SUMMARY: MinerCommand = MinerCommand::RPC {
            command: "summary",
            parameters: None,
        };

        const RPC_POOLS: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };

        const WEB_SYSTEM_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "get_system_info",
            parameters: None,
        };

        const WEB_BLINK_STATUS: MinerCommand = MinerCommand::WebAPI {
            command: "get_blink_status",
            parameters: None,
        };

        const WEB_MINER_CONF: MinerCommand = MinerCommand::WebAPI {
            command: "get_miner_conf",
            parameters: None,
        };

        const WEB_SUMMARY: MinerCommand = MinerCommand::WebAPI {
            command: "summary",
            parameters: None,
        };

        const WEB_MINER_TYPE: MinerCommand = MinerCommand::WebAPI {
            command: "miner_type",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/macaddr"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![(
                RPC_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/VERSION/0/API"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                RPC_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/VERSION/0/CompileTime"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/hostname"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                WEB_MINER_TYPE,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/subtype"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/GHS 5s"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/1/total_rateideal"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/1"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/1"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                WEB_BLINK_STATUS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/blink"),
                    tag: None,
                },
            )],
            DataField::IsMining => vec![(
                WEB_MINER_CONF,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bitmain-work-mode"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/1/Elapsed"),
                    tag: None,
                },
            )],
            DataField::Pools => vec![(
                RPC_POOLS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/POOLS"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/1"),
                    tag: None,
                },
            )],
            DataField::SerialNumber => vec![
                (
                    WEB_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/serial_no"), // Cant find on 2022 firmware, does exist on 2025 firmware for XP
                        tag: None,
                    },
                ),
                (
                    WEB_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/serinum"), // exist on 2025 firmware for s21
                        tag: None,
                    },
                ),
            ],
            DataField::Messages => vec![(
                WEB_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/status"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for AntMinerV2020 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for AntMinerV2020 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info
    }
}

impl CollectData for AntMinerV2020 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for AntMinerV2020 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetHostname for AntMinerV2020 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for AntMinerV2020 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for AntMinerV2020 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetHashboards for AntMinerV2020 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> = Vec::new();
        let board_count = self.device_info.hardware.boards.unwrap_or(3);

        for idx in 0..board_count {
            hashboards.push(BoardData {
                hashrate: None,
                position: idx,
                expected_hashrate: None,
                board_temperature: None,
                intake_temperature: None,
                outlet_temperature: None,
                expected_chips: self.device_info.hardware.chips,
                working_chips: None,
                serial_number: None,
                chips: vec![],
                voltage: None,
                frequency: None,
                tuned: Some(false),
                active: Some(false),
            });
        }

        if let Some(stats_data) = data.get(&DataField::Hashboards) {
            for idx in 1..=board_count {
                let board_idx = (idx - 1) as usize;
                if board_idx >= hashboards.len() {
                    break;
                }

                if let Some(hashrate) = stats_data
                    .get(format!("chain_rate{}", idx))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(|f| {
                        HashRate {
                            value: f,
                            unit: HashRateUnit::GigaHash,
                            algo: String::from("SHA256"),
                        }
                        .as_unit(HashRateUnit::TeraHash)
                    })
                {
                    hashboards[board_idx].hashrate = Some(hashrate);
                }

                if let Some(working_chips) = stats_data
                    .get(format!("chain_acn{}", idx))
                    .and_then(|v| v.as_u64())
                    .map(|u| u as u16)
                {
                    hashboards[board_idx].working_chips = Some(working_chips);
                }

                if let Some(board_temp) = stats_data
                    .get(format!("temp_pcb{}", idx))
                    .and_then(|v| v.as_str())
                    .and_then(Self::parse_temp_string)
                {
                    hashboards[board_idx].board_temperature = Some(board_temp);
                }

                if let Some(frequency) = stats_data
                    .get(format!("freq{}", idx))
                    .and_then(|v| v.as_u64())
                    .map(|f| Frequency::from_megahertz(f as f64))
                {
                    hashboards[board_idx].frequency = Some(frequency);
                }

                let has_hashrate = hashboards[board_idx]
                    .hashrate
                    .as_ref()
                    .map(|h| h.value > 0.0)
                    .unwrap_or(false);
                let has_chips = hashboards[board_idx]
                    .working_chips
                    .map(|chips| chips > 0)
                    .unwrap_or(false);

                hashboards[board_idx].active = Some(has_hashrate || has_chips);
                hashboards[board_idx].tuned = Some(has_hashrate || has_chips);
            }
        }

        hashboards
    }
}

impl GetHashrate for AntMinerV2020 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| {
            HashRate {
                value: f,
                unit: HashRateUnit::GigaHash,
                algo: String::from("SHA256"),
            }
            .as_unit(HashRateUnit::TeraHash)
        })
    }
}

impl GetExpectedHashrate for AntMinerV2020 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| {
            HashRate {
                value: f,
                unit: HashRateUnit::GigaHash,
                algo: String::from("SHA256"),
            }
            .as_unit(HashRateUnit::TeraHash)
        })
    }
}

impl GetFans for AntMinerV2020 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans: Vec<FanData> = Vec::new();

        if let Some(stats_data) = data.get(&DataField::Fans) {
            for i in 1..=self.device_info.hardware.fans.unwrap_or(4) {
                if let Some(fan_speed) =
                    stats_data.get(format!("fan{}", i)).and_then(|v| v.as_f64())
                    && fan_speed > 0.0
                {
                    fans.push(FanData {
                        position: (i - 1) as i16,
                        rpm: Some(AngularVelocity::from_rpm(fan_speed)),
                    });
                }
            }
        }

        fans
    }
}

impl GetLightFlashing for AntMinerV2020 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing).or_else(|| {
            data.extract::<String>(DataField::LightFlashing)
                .map(|s| s.to_lowercase() == "true" || s == "1")
        })
    }
}

impl GetUptime for AntMinerV2020 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetIsMining for AntMinerV2020 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        data.extract::<String>(DataField::IsMining)
            .map(|status| {
                let status_lower = status.to_lowercase();
                status_lower != "stopped" && status_lower != "idle" && status_lower != "sleep"
            })
            .or_else(|| data.extract::<f64>(DataField::Hashrate).map(|hr| hr > 0.0))
            .unwrap_or(false)
    }
}

impl GetPools for AntMinerV2020 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolData> {
        let mut pools: Vec<PoolData> = Vec::new();

        if let Some(pools_data) = data.get(&DataField::Pools)
            && let Some(pools_array) = pools_data.as_array()
        {
            for (idx, pool_info) in pools_array.iter().enumerate() {
                let url = pool_info
                    .get("URL")
                    .and_then(|v| v.as_str())
                    .map(|s| PoolURL::from(s.to_string()));

                let user = pool_info
                    .get("User")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let alive = pool_info
                    .get("Status")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "Alive");

                let active = pool_info.get("Stratum Active").and_then(|v| v.as_bool());

                let accepted_shares = pool_info.get("Accepted").and_then(|v| v.as_u64());

                let rejected_shares = pool_info.get("Rejected").and_then(|v| v.as_u64());

                pools.push(PoolData {
                    position: Some(idx as u16),
                    url,
                    accepted_shares,
                    rejected_shares,
                    active,
                    alive,
                    user,
                });
            }
        }

        pools
    }
}

impl GetSerialNumber for AntMinerV2020 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::SerialNumber)
    }
}

impl GetControlBoardVersion for AntMinerV2020 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        let cb_type = data.extract::<String>(DataField::ControlBoardVersion)?;
        match cb_type.as_str() {
            s if s.to_uppercase().contains("AML") => Some(MinerControlBoard::AMLogic),
            _ => MinerControlBoard::from_str(cb_type.split("_").collect::<Vec<&str>>()[0]).ok(),
        }
    }
}

impl GetWattage for AntMinerV2020 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        if let Some(stats_data) = data.get(&DataField::Wattage) {
            if let Some(chain_power) = stats_data.get("chain_power")
                && let Some(power_str) = chain_power.as_str()
            {
                // Parse "3250 W" format
                if let Some(watt_part) = power_str.split_whitespace().next()
                    && let Ok(watts) = watt_part.parse::<f64>()
                {
                    return Some(Power::from_watts(watts));
                }
            }

            if let Some(power) = stats_data
                .get("power")
                .or_else(|| stats_data.get("Power"))
                .and_then(|v| v.as_f64())
            {
                return Some(Power::from_watts(power));
            }
        }
        None
    }
}

impl GetWattageLimit for AntMinerV2020 {}

impl GetFluidTemperature for AntMinerV2020 {
    fn parse_fluid_temperature(&self, data: &HashMap<DataField, Value>) -> Option<Temperature> {
        // For S21+ Hyd models, use inlet/outlet temperature average
        if self.device_info.model.to_string().contains("S21+ Hyd")
            && let Some(hashboards_data) = data.get(&DataField::Hashboards)
            && let Some(chains) = hashboards_data.as_array()
        {
            let mut temps = Vec::new();

            for chain in chains {
                if let Some(temp_pcb) = chain.get("temp_pcb").and_then(|v| v.as_array()) {
                    // Inlet temp (index 0) and outlet temp (index 2)
                    if let Some(inlet) = temp_pcb.first().and_then(|v| v.as_f64())
                        && inlet != 0.0
                    {
                        temps.push(inlet);
                    }
                    if let Some(outlet) = temp_pcb.get(2).and_then(|v| v.as_f64())
                        && outlet != 0.0
                    {
                        temps.push(outlet);
                    }
                }
            }

            if !temps.is_empty() {
                let avg = temps.iter().sum::<f64>() / temps.len() as f64;
                return Some(Temperature::from_celsius(avg));
            }
        }
        None
    }
}

impl GetPsuFans for AntMinerV2020 {}

impl GetMessages for AntMinerV2020 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let mut messages = Vec::new();

        if let Some(status_data) = data.get(&DataField::Messages)
            && let Some(status_array) = status_data.as_array()
        {
            for (idx, item) in status_array.iter().enumerate() {
                if let Some(status) = item.get("status").and_then(|v| v.as_str())
                    && status != "s"
                {
                    // 's' means success/ok
                    let message_text = item
                        .get("msg")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error")
                        .to_string();

                    let severity = match status.to_lowercase().as_str() {
                        "e" => MessageSeverity::Error,
                        "w" => MessageSeverity::Warning,
                        _ => MessageSeverity::Info,
                    };

                    messages.push(MinerMessage::new(0, idx as u64, message_text, severity));
                }
            }
        }

        messages
    }
}

#[async_trait]
impl SetFaultLight for AntMinerV2020 {
    #[allow(unused_variables)]
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        Ok(self.web.blink(fault).await.is_ok())
    }
}

#[async_trait]
impl SetPowerLimit for AntMinerV2020 {
    #[allow(unused_variables)]
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

#[async_trait]
impl Restart for AntMinerV2020 {
    async fn restart(&self) -> anyhow::Result<bool> {
        Ok(self.web.reboot().await.is_ok())
    }
}

impl AntMinerV2020 {
    async fn set_paused(&self, paused: bool) -> anyhow::Result<bool> {
        // Newer firmware/models: bitmain-work-mode: "0" (normal) / "1" (sleep)
        let work_mode = if paused { "1" } else { "0" };

        if self
            .web
            .set_miner_conf(json!({ "bitmain-work-mode": work_mode }))
            .await
            .is_ok()
        {
            return Ok(true);
        }

        // Older firmware fallback: miner-mode
        let legacy_mode = if paused {
            MinerMode::Sleep.to_string()
        } else {
            MinerMode::Normal.to_string()
        };

        Ok(self
            .web
            .set_miner_conf(json!({ "miner-mode": legacy_mode }))
            .await
            .is_ok())
    }
}

#[async_trait]
impl Pause for AntMinerV2020 {
    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        self.set_paused(true).await
    }
}

#[async_trait]
impl Resume for AntMinerV2020 {
    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        self.set_paused(false).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::device::models::antminer::AntMinerModel;
    use crate::test::api::MockAPIClient;
    use crate::test::json::bmminer::antminer_modern::{
        AM_DEVS, AM_POOLS, AM_STATS, AM_SUMMARY, AM_VERSION,
    };

    #[tokio::test]
    async fn test_antminer() {
        let miner = AntMinerV2020::new(
            IpAddr::from([127, 0, 0, 1]),
            MinerModel::AntMiner(AntMinerModel::S19Pro),
        );

        let mut results = HashMap::new();

        let stats_cmd = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };

        let version_cmd = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };

        let summary_cmd = MinerCommand::RPC {
            command: "summary",
            parameters: None,
        };

        let devs_cmd = MinerCommand::RPC {
            command: "devs",
            parameters: None,
        };

        let pools_cmd = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };

        results.insert(stats_cmd, Value::from_str(AM_STATS).unwrap());
        results.insert(version_cmd, Value::from_str(AM_VERSION).unwrap());
        results.insert(summary_cmd, Value::from_str(AM_SUMMARY).unwrap());
        results.insert(devs_cmd, Value::from_str(AM_DEVS).unwrap());
        results.insert(pools_cmd, Value::from_str(AM_POOLS).unwrap());

        let mock_api = MockAPIClient::new(results);

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;

        let miner_data = miner.parse_data(data);

        assert_eq!(miner_data.ip.to_string(), "127.0.0.1".to_owned());
        assert_eq!(miner_data.hashboards.len(), 3);
        assert_eq!(miner_data.light_flashing, None);
        assert_eq!(miner_data.fans.len(), 4);
        assert_eq!(
            miner_data.expected_hashrate.unwrap(),
            HashRate {
                value: 110.0,
                unit: HashRateUnit::TeraHash,
                algo: "SHA256".to_string(),
            }
        );
        assert_eq!(
            miner_data.hashrate.unwrap(),
            HashRate {
                value: 110.56689,
                unit: HashRateUnit::TeraHash,
                algo: "SHA256".to_string(),
            }
        );
    }
}

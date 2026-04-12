use std::{collections::HashMap, fmt::Display, net::IpAddr, str::FromStr, time::Duration};

use anyhow;
use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigExtractor, ConfigField, ConfigLocation},
        pools::{PoolConfig, PoolGroupConfig},
    },
    data::{
        board::{BoardData, MinerControlBoard},
        collector::{
            DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
        },
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        firmware::FirmwareImage,
        hashrate::{HashRate, HashRateUnit},
        message::{MessageSeverity, MinerMessage},
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_antminer::hardware::AntMinerControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature};
use rpc::AntMinerRPCAPI;
use serde_json::{Value, json};
use web::AntMinerWebAPI;

use self::firmware::AntMinerFirmwareImageExt;
use crate::firmware::AntMinerStockFirmware;

mod firmware;
mod rpc;
pub(crate) mod web;

#[derive(Debug)]
pub struct AntMinerV2020 {
    pub ip: IpAddr,
    pub rpc: AntMinerRPCAPI,
    pub web: AntMinerWebAPI,
    pub device_info: DeviceInfo,
}

#[allow(dead_code)]
enum MinerMode {
    Sleep,
    Low,
    Normal,
    High,
}

impl Display for MinerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MinerMode::Sleep => "1",
            MinerMode::Low => "3",
            _ => "0",
        };
        f.write_str(s)
    }
}

impl AntMinerV2020 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        let auth = Self::default_auth();
        AntMinerV2020 {
            ip,
            rpc: AntMinerRPCAPI::new(ip),
            web: AntMinerWebAPI::new(ip, auth),
            device_info: DeviceInfo::new(
                model,
                AntMinerStockFirmware::default(),
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

impl GetConfigsLocations for AntMinerV2020 {
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        const WEB_GET_MINER_CONF: MinerCommand = MinerCommand::WebAPI {
            command: "get_miner_conf",
            parameters: None,
        };
        match data_field {
            ConfigField::Pools => vec![(
                WEB_GET_MINER_CONF,
                ConfigExtractor {
                    func: get_by_pointer,
                    key: Some("/pools"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl CollectConfigs for AntMinerV2020 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
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
                WEB_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/system_filesystem_version"),
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
        self.device_info.clone()
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
                            algo: "SHA256".to_string(),
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
                algo: "SHA256".to_string(),
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
                algo: "SHA256".to_string(),
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
                status_lower != "stopped"
                    && status_lower != "idle"
                    && status_lower != "sleep"
                    && status_lower != "1"
            })
            .or_else(|| data.extract::<f64>(DataField::Hashrate).map(|hr| hr > 0.0))
            .unwrap_or(false)
    }
}

impl GetPools for AntMinerV2020 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let Some(pools_data) = data.get(&DataField::Pools) else {
            return vec![];
        };

        let Some(pools_array) = pools_data.as_array() else {
            return vec![PoolGroupData {
                name: String::new(),
                quota: 1,
                pools: vec![],
            }];
        };

        let mut rpc_pools: Vec<PoolData> = Vec::with_capacity(pools_array.len());
        for (idx, pool_info) in pools_array.iter().enumerate() {
            let url = pool_info
                .get("URL")
                .and_then(|v| v.as_str())
                .map(|s| PoolURL::from(s.to_string()));

            let accepted_shares = pool_info.get("Accepted").and_then(|v| v.as_u64());

            let rejected_shares = pool_info.get("Rejected").and_then(|v| v.as_u64());

            let active = pool_info.get("Stratum Active").and_then(|v| v.as_bool());

            let alive = pool_info
                .get("Status")
                .and_then(|v| v.as_str())
                .map(|s| s == "Alive");

            let user = pool_info
                .get("User")
                .and_then(|v| v.as_str())
                .map(String::from);

            rpc_pools.push(PoolData {
                position: Some(idx as u16),
                url,
                accepted_shares,
                rejected_shares,
                active,
                alive,
                user,
            });
        }

        vec![PoolGroupData {
            name: String::new(),
            quota: 1,
            pools: rpc_pools,
        }]
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
            s if s.to_uppercase().contains("AML") => Some(AntMinerControlBoard::AMLogic.into()),
            _ => AntMinerControlBoard::parse(cb_type.split("_").collect::<Vec<&str>>()[0])
                .map(|cb| cb.into()),
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

impl GetTuningTarget for AntMinerV2020 {}

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
    fn supports_set_fault_light(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        Ok(self.web.blink(fault).await.is_ok())
    }
}

#[async_trait]
impl SetPowerLimit for AntMinerV2020 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsPoolsConfig for AntMinerV2020 {
    fn parse_pools_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<Vec<PoolGroupConfig>> {
        let Some(pools_data) = data.get(&ConfigField::Pools) else {
            return Ok(vec![]);
        };

        let Some(pools_array) = pools_data.as_array() else {
            return Ok(vec![PoolGroupConfig {
                name: String::new(),
                quota: 1,
                pools: vec![],
            }]);
        };

        let mut pools: Vec<PoolConfig> = Vec::with_capacity(pools_array.len());
        for pool in pools_array {
            let Some(url) = pool.get("url").and_then(|v| v.as_str()) else {
                continue;
            };
            if url.is_empty() {
                continue;
            }

            let username = pool
                .get("user")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default();
            let password = pool
                .get("pass")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default();

            pools.push(PoolConfig {
                url: PoolURL::from(url.to_string()),
                username,
                password,
            });
        }

        pools.truncate(3);

        Ok(vec![PoolGroupConfig {
            name: String::new(),
            quota: 1,
            pools,
        }])
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> anyhow::Result<bool> {
        let mut pools: Vec<Value> = config
            .into_iter()
            .flat_map(|group| group.pools.into_iter())
            .map(|pool| {
                json!({
                    "url": pool.url.to_string(),
                    "user": pool.username,
                    "pass": pool.password,
                })
            })
            .collect();

        pools.truncate(3);

        Ok(self
            .web
            .set_miner_conf(json!({ "pools": pools }))
            .await
            .is_ok())
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for AntMinerV2020 {
    fn supports_restart(&self) -> bool {
        true
    }
    async fn restart(&self) -> anyhow::Result<bool> {
        Ok(self.web.reboot().await.is_ok())
    }
}

#[async_trait]
impl Pause for AntMinerV2020 {
    fn supports_pause(&self) -> bool {
        true
    }
    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        let pre = self.web.get_miner_conf().await?;

        if pre.get("miner-mode").is_some() {
            return Ok(self
                .web
                .set_miner_conf(json!({"miner-mode": MinerMode::Sleep.to_string()}))
                .await
                .is_ok());
        }

        if pre.get("bitmain-work-mode").is_some() {
            return Ok(self
                .web
                .set_miner_conf(json!({"bitmain-work-mode": MinerMode::Sleep.to_string()}))
                .await
                .is_ok());
        }

        Ok(false)
    }
}

#[async_trait]
impl Resume for AntMinerV2020 {
    fn supports_resume(&self) -> bool {
        true
    }
    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        let pre = self.web.get_miner_conf().await?;

        if pre.get("miner-mode").is_some() {
            return Ok(self
                .web
                .set_miner_conf(json!({"miner-mode": MinerMode::Normal.to_string()}))
                .await
                .is_ok());
        }

        if pre.get("bitmain-work-mode").is_some() {
            return Ok(self
                .web
                .set_miner_conf(json!({"bitmain-work-mode": MinerMode::Normal.to_string()}))
                .await
                .is_ok());
        }

        Ok(false)
    }
}

#[async_trait]
impl SupportsScalingConfig for AntMinerV2020 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for AntMinerV2020 {
    async fn upgrade_firmware(&self, image: FirmwareImage) -> anyhow::Result<bool> {
        let miner = self.get_miner_type_info().await?;
        let image = image.resolve_for_miner(&miner)?;
        self.web.upgrade_firmware(image).await?;
        Ok(true)
    }

    fn supports_upgrade_firmware(&self) -> bool {
        true
    }
}

impl HasDefaultAuth for AntMinerV2020 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("root", "root")
    }
}

impl HasAuth for AntMinerV2020 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

#[async_trait]
impl SupportsTuningConfig for AntMinerV2020 {
    fn supports_tuning_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsFanConfig for AntMinerV2020 {
    fn supports_fan_config(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::{self, Context};
    use asic_rs_core::test::{api::MockAPIClient, util::get_miner};
    use asic_rs_makes_antminer::models::AntMinerModel;

    use super::*;
    use crate::test::json::v2020::{
        AM_DEVS, AM_POOLS, AM_STATS, AM_SUMMARY, AM_SYSTEM_INFO, AM_VERSION,
    };

    #[tokio::test]
    async fn test_antminer() {
        let miner = AntMinerV2020::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19Pro);

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

        let system_info_cmd = MinerCommand::WebAPI {
            command: "get_system_info",
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
        results.insert(system_info_cmd, Value::from_str(AM_SYSTEM_INFO).unwrap());
        results.insert(devs_cmd, Value::from_str(AM_DEVS).unwrap());
        results.insert(pools_cmd, Value::from_str(AM_POOLS).unwrap());

        let mock_api = MockAPIClient::new(results);

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;

        let miner_data = miner.parse_data(data);

        assert_eq!(miner_data.ip.to_string(), "127.0.0.1".to_owned());
        assert_eq!(
            miner_data.firmware_version.as_deref(),
            Some("FR-1.12(251009-S21)")
        );
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

    #[tokio::test]
    #[ignore = "requires live miner; set MINER_IP"]
    async fn parse_data_live_test_auto_detect() -> anyhow::Result<()> {
        let ip_str = std::env::var("MINER_IP").context("MINER_IP is not set")?;
        let ip =
            IpAddr::from_str(&ip_str).with_context(|| format!("invalid MINER_IP: {ip_str}"))?;

        let miner = get_miner(ip, Arc::new(AntMinerStockFirmware::default()))
            .await?
            .context("no miner detected at MINER_IP")?;
        let miner_data = miner.get_data().await;
        let mut miner_data_print = miner_data.clone();
        for hashboard in &mut miner_data_print.hashboards {
            hashboard.chips.clear();
        }
        println!("data {}", serde_json::to_string_pretty(&miner_data_print)?);

        println!(
            "pools {}",
            serde_json::to_string_pretty(&miner.get_pools_config().await?)?
        );

        assert_eq!(miner_data.ip, ip);
        assert!(miner_data.timestamp > 0);
        assert!(!miner_data.schema_version.is_empty());

        Ok(())
    }
}

use std::{collections::HashMap, net::IpAddr, str::FromStr, time::Duration};

use asic_rs_core::config::scaling::ScalingConfig;
use asic_rs_core::{
    config::{
        collector::{
            ConfigCollector, ConfigExtractor, ConfigField, ConfigLocation,
            get_by_pointer as cfg_by_pointer,
        },
        pools::PoolGroupConfig,
        tuning::TuningConfig,
    },
    data::{
        board::{BoardData, MinerControlBoard},
        capabilities::{PowerTuningCapabilities, PresetTuningCapabilities, TuningCapabilities},
        collector::{
            DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_key,
            get_by_pointer,
        },
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        hashrate::{HashRate, HashRateUnit},
        message::{MessageSeverity, MinerComponent, MinerMessage},
        miner::{MiningMode, TuningTarget},
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_sealminer::models::SealMinerModel;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature};
use rpc::SealMinerRPCAPI;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use web::SealMinerWebAPI;

use crate::firmware::SealMinerStockFirmware;

pub(crate) mod rpc;
pub(crate) mod web;

#[derive(Debug)]
pub struct SealMinerV2025 {
    pub ip: IpAddr,
    pub rpc: SealMinerRPCAPI,
    pub web: SealMinerWebAPI,
    pub device_info: DeviceInfo,
}

impl SealMinerV2025 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        SealMinerV2025 {
            ip,
            rpc: SealMinerRPCAPI::new(ip),
            web: SealMinerWebAPI::new(ip, Self::default_auth()),
            device_info: DeviceInfo::new(
                model,
                SealMinerStockFirmware::default(),
                HashAlgorithm::SHA256,
            ),
        }
    }
}

#[async_trait]
impl APIClient for SealMinerV2025 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for {}",
                SealMinerStockFirmware::default()
            )),
        }
    }
}

impl GetConfigsLocations for SealMinerV2025 {
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        const RPC_STATS: MinerCommand = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };
        const RPC_POOLS: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };
        match data_field {
            ConfigField::Tuning => vec![(
                RPC_STATS,
                ConfigExtractor {
                    func: cfg_by_pointer,
                    key: Some("/STATS/0"),
                    tag: None,
                },
            )],
            ConfigField::Pools => vec![(
                RPC_POOLS,
                ConfigExtractor {
                    func: cfg_by_pointer,
                    key: Some("/POOLS"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl CollectConfigs for SealMinerV2025 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

#[async_trait]
impl GetDataLocations for SealMinerV2025 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const RPC_STATS: MinerCommand = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };
        const RPC_POOLS: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };
        const RPC_VERSION: MinerCommand = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };

        const CGI_SYSTEM_INFO: MinerCommand = MinerCommand::WebAPI {
            command: "get_system_info",
            parameters: None,
        };
        const CGI_PSU_STATUS: MinerCommand = MinerCommand::WebAPI {
            command: "miner-psu-status",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![
                (
                    RPC_STATS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/STATS/0/MAC"),
                        tag: None,
                    },
                ),
                (
                    CGI_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_key,
                        key: Some("macaddr"),
                        tag: None,
                    },
                ),
            ],
            DataField::SerialNumber => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/Ctrl Board SN"),
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
            DataField::FirmwareVersion => vec![
                (
                    RPC_STATS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/STATS/0/Firmware"),
                        tag: None,
                    },
                ),
                (
                    CGI_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_key,
                        key: Some("firmware_version"),
                        tag: None,
                    },
                ),
            ],
            DataField::ControlBoardVersion => vec![
                (
                    RPC_STATS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/STATS/0/Ctrl Board"),
                        tag: None,
                    },
                ),
                (
                    CGI_SYSTEM_INFO,
                    DataExtractor {
                        func: get_by_key,
                        key: Some("ctrl_version"),
                        tag: None,
                    },
                ),
            ],
            DataField::Hashboards => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MHS(2m)"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MHS(Rated)"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0"),
                    tag: None,
                },
            )],
            DataField::PsuFans => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/PSU Fan Speed"),
                    tag: None,
                },
            )],
            DataField::AverageTemperature => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/Temp"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![
                (
                    RPC_STATS,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/STATS/0/PSU Input Power"),
                        tag: None,
                    },
                ),
                (
                    CGI_PSU_STATUS,
                    DataExtractor {
                        func: get_by_key,
                        key: Some("Power"),
                        tag: None,
                    },
                ),
            ],
            DataField::TuningTarget => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/System Uptime"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                CGI_SYSTEM_INFO,
                DataExtractor {
                    func: get_by_key,
                    key: Some("led"),
                    tag: None,
                },
            )],
            DataField::Messages => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0"),
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
            _ => vec![],
        }
    }
}

impl GetIP for SealMinerV2025 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for SealMinerV2025 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for SealMinerV2025 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for SealMinerV2025 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetSerialNumber for SealMinerV2025 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::SerialNumber)
    }
}

impl GetHostname for SealMinerV2025 {}
impl SupportsTimezoneConfig for SealMinerV2025 {}

impl GetApiVersion for SealMinerV2025 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for SealMinerV2025 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetControlBoardVersion for SealMinerV2025 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| {
                asic_rs_makes_sealminer::hardware::SealMinerControlBoard::parse(&s)
                    .map(|cb| cb.into())
            })
    }
}

impl GetHashboards for SealMinerV2025 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let stats = match data.get(&DataField::Hashboards) {
            Some(v) => v,
            None => return vec![],
        };

        let board_count = stats["Board Count"].as_u64().unwrap_or(0) as usize;

        (0..board_count)
            .map(|i| {
                let online = stats
                    .get(format!("{i} Online"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let chip_count = stats
                    .get(format!("{i} Chip Count"))
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u16);
                let bad_chips = stats
                    .get(format!("{i} Bad Chip Count"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u16;
                let working_chips = chip_count.map(|c| c.saturating_sub(bad_chips));
                let serial = stats
                    .get(format!("{i} SN"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                let freq = stats
                    .get(format!("{i} Freq"))
                    .and_then(|v| v.as_f64())
                    .map(Frequency::from_megahertz);

                // 4 temperature sensors per board; filter out zeros
                let raw_temps: Vec<f64> = (0..4)
                    .filter_map(|t| {
                        stats
                            .get(format!("{i} Temp {t}"))
                            .and_then(|v| v.as_f64())
                            .filter(|&t| t > 0.0)
                    })
                    .collect();
                let board_temp = raw_temps
                    .iter()
                    .cloned()
                    .reduce(f64::max)
                    .map(Temperature::from_celsius);
                let intake_temp = raw_temps.first().copied().map(Temperature::from_celsius);
                let outlet_temp = raw_temps.last().copied().map(Temperature::from_celsius);

                let hashrate = stats
                    .get(format!("{i} MHS(2m)"))
                    .and_then(|v| v.as_f64())
                    .map(|mhs| {
                        HashRate {
                            value: mhs,
                            unit: HashRateUnit::MegaHash,
                            algo: self.device_info.algo,
                        }
                        .as_default_unit()
                    });
                let ideal_hashrate = stats
                    .get(format!("{i} MHS(Ideal)"))
                    .and_then(|v| v.as_f64())
                    .map(|mhs| {
                        HashRate {
                            value: mhs,
                            unit: HashRateUnit::MegaHash,
                            algo: self.device_info.algo,
                        }
                        .as_default_unit()
                    });

                let tuned = stats
                    .get(format!("{i} Tune Status"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_lowercase().contains("done"));

                BoardData {
                    position: i as u8,
                    hashrate,
                    expected_hashrate: ideal_hashrate,
                    board_temperature: board_temp,
                    inlet_chip_temperature: intake_temp,
                    outlet_chip_temperature: outlet_temp,
                    expected_chips: self.device_info.hardware.chips_for_board(i),
                    working_chips,
                    serial_number: serial,
                    chips: vec![],
                    voltage: None,
                    frequency: freq,
                    tuned,
                    active: Some(online),
                }
            })
            .collect()
    }
}

impl GetHashrate for SealMinerV2025 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |mhs| {
            HashRate {
                value: mhs,
                unit: HashRateUnit::MegaHash,
                algo: self.device_info.algo,
            }
            .as_default_unit()
        })
    }
}

impl GetExpectedHashrate for SealMinerV2025 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |mhs| {
            HashRate {
                value: mhs,
                unit: HashRateUnit::MegaHash,
                algo: self.device_info.algo,
            }
            .as_default_unit()
        })
    }
}

impl GetFans for SealMinerV2025 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let stats = match data.get(&DataField::Fans) {
            Some(v) => v,
            None => return vec![],
        };
        let fan_count = stats["Fan Count"].as_u64().unwrap_or(0) as usize;
        (0..fan_count)
            .map(|i| FanData {
                position: i as i16,
                rpm: stats
                    .get(format!("{i} Speed"))
                    .and_then(|v| v.as_f64())
                    .map(AngularVelocity::from_rpm),
            })
            .collect()
    }
}

impl GetPsuFans for SealMinerV2025 {
    fn parse_psu_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        data.extract_map::<f64, _>(DataField::PsuFans, |rpm| {
            vec![FanData {
                position: 0,
                rpm: Some(AngularVelocity::from_rpm(rpm)),
            }]
        })
        .unwrap_or_default()
    }
}

impl GetFluidTemperature for SealMinerV2025 {}

impl GetWattage for SealMinerV2025 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}

fn parse_stock_tuning(stats: &Value) -> Option<TuningTarget> {
    let mode = match stats.get("Mining Mode") {
        Some(mode) => Some(mode.as_str()?),
        None => None,
    };
    match mode {
        Some("Normal") => Some(TuningTarget::MiningMode(MiningMode::Normal)),
        Some("High Performance") => Some(TuningTarget::MiningMode(MiningMode::High)),
        Some("Low Power") => Some(TuningTarget::MiningMode(MiningMode::Low)),
        Some("") | None => stats
            .get("Power Limit")
            .and_then(Value::as_f64)
            .map(|watts| TuningTarget::Power(Power::from_watts(watts))),
        _ => None,
    }
}

impl GetTuningTarget for SealMinerV2025 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        parse_stock_tuning(data.get(&DataField::TuningTarget)?)
    }
}

impl GetScaledTuningTarget for SealMinerV2025 {}
impl GetTuningCapabilities for SealMinerV2025 {
    fn parse_tuning_capabilities(
        &self,
        _data: &HashMap<DataField, Value>,
    ) -> Option<TuningCapabilities> {
        let mut presets = Vec::with_capacity(3);
        if self.device_info.model == SealMinerModel::A2.as_ref() {
            presets.push(TuningTarget::MiningMode(MiningMode::Low));
        }
        presets.push(TuningTarget::MiningMode(MiningMode::Normal));
        presets.push(TuningTarget::MiningMode(MiningMode::High));
        Some(TuningCapabilities {
            power: Some(PowerTuningCapabilities::default()),
            presets: Some(PresetTuningCapabilities {
                default: None,
                presets,
            }),
            ..Default::default()
        })
    }
}
impl GetLightFlashing for SealMinerV2025 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<String>(DataField::LightFlashing)
            .map(|s| s != "off")
    }
}

impl GetMessages for SealMinerV2025 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let stats = match data.get(&DataField::Messages) {
            Some(v) => v,
            None => return vec![],
        };
        let mut messages = Vec::new();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;

        if let Some(code) = stats["Error Code"].as_str().filter(|s| !s.is_empty()) {
            messages.push(MinerMessage {
                timestamp,
                code: 0,
                message: format!("Error Code: {code}"),
                severity: MessageSeverity::Error,
                component: None,
            });
        }

        let board_count = stats["Board Count"].as_u64().unwrap_or(0) as usize;
        for i in 0..board_count {
            let bad = stats
                .get(format!("{i} Bad Chip Count"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if bad > 0 {
                messages.push(MinerMessage {
                    timestamp,
                    code: 0,
                    message: format!("Board {i}: {bad} bad chip(s)"),
                    severity: MessageSeverity::Warning,
                    component: Some(MinerComponent::hashboard(i as u16)),
                });
            }
        }

        messages
    }
}

impl GetUptime for SealMinerV2025 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetBestShare for SealMinerV2025 {}
impl GetSessionBestShare for SealMinerV2025 {}

impl GetOperatingState for SealMinerV2025 {}

impl GetDevFeeConnected for SealMinerV2025 {}

impl GetIsMining for SealMinerV2025 {}

impl GetPools for SealMinerV2025 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let pools_val = match data.get(&DataField::Pools) {
            Some(v) => v,
            None => return vec![],
        };
        let pools_arr = match pools_val.as_array() {
            Some(a) => a,
            None => return vec![],
        };

        let pools = pools_arr
            .iter()
            .enumerate()
            .map(|(i, pool)| PoolData {
                position: Some(i as u16),
                url: pool["URL"].as_str().map(|s| PoolURL::from(s.to_string())),
                accepted_shares: pool["Accepted"].as_u64(),
                rejected_shares: pool["Rejected"].as_u64(),
                last_share_time: pool
                    .get("Last Share Time")
                    .and_then(asic_rs_core::util::parse_last_share_time),
                active: pool["Stratum Active"].as_bool(),
                alive: pool["Status"].as_str().map(|s| s == "Alive"),
                user: pool["User"].as_str().map(str::to_string),
            })
            .collect();

        vec![PoolGroupData {
            name: String::new(),
            quota: 1,
            pools,
        }]
    }
}
#[async_trait]
impl SetFaultLight for SealMinerV2025 {
    fn supports_set_fault_light(&self) -> bool {
        true
    }

    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        Ok(self.web.set_led(fault).await.is_ok())
    }
}

#[async_trait]
impl SetPowerLimit for SealMinerV2025 {
    fn supports_set_power_limit(&self) -> bool {
        true
    }

    async fn set_power_limit(&self, watts: Power) -> anyhow::Result<bool> {
        // API V0.5 applies the limit and restarts bdminer itself, not the controller.
        Ok(self.rpc.set_power_limit(watts).await.is_ok())
    }
}

#[async_trait]
impl Restart for SealMinerV2025 {
    fn supports_restart(&self) -> bool {
        true
    }

    async fn restart(&self) -> anyhow::Result<bool> {
        Ok(self.web.reboot().await.is_ok())
    }
}

#[async_trait]
impl Pause for SealMinerV2025 {
    fn supports_pause(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        Ok(self.rpc.ascset("0,suspend").await.is_ok())
    }
}

#[async_trait]
impl Resume for SealMinerV2025 {
    fn supports_resume(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        // If the miner is already hashing, don't restart it.
        if let Ok(summary) = self.rpc.stats().await {
            let running = summary
                .pointer("/STATS/0/PM State")
                .map(|v| v == "Running")
                .unwrap_or(false);
            if running {
                return Ok(true);
            }
        }
        Ok(self.rpc.restart().await.is_ok())
    }
}

impl ChangePassword for SealMinerV2025 {
    fn supports_change_password(&self) -> bool {
        false
    }
}

impl ReadLogs for SealMinerV2025 {
    fn supports_read_logs(&self) -> bool {
        false
    }
}

impl FactoryReset for SealMinerV2025 {
    fn supports_factory_reset(&self) -> bool {
        false
    }
}

impl RestoreStockOs for SealMinerV2025 {}

#[async_trait]
impl SupportsPoolsConfig for SealMinerV2025 {
    async fn get_pools_config(&self) -> anyhow::Result<Vec<PoolGroupConfig>> {
        Ok(vec![self.web.get_pool_conf().await?])
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> anyhow::Result<bool> {
        let group = config
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No pool groups provided"))?;

        let pool_strings: Vec<(String, String, String)> = group
            .pools
            .iter()
            .map(|p| (p.url.to_string(), p.username.clone(), p.password.clone()))
            .collect();
        let pools: Vec<(&str, &str, &str)> = pool_strings
            .iter()
            .map(|(u, w, p)| (u.as_str(), w.as_str(), p.as_str()))
            .collect();

        if self.rpc.updatepools(&pools).await.is_err() {
            return Ok(false);
        }
        let _ = self.web.reboot().await;
        Ok(true)
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsScalingConfig for SealMinerV2025 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsTuningConfig for SealMinerV2025 {
    fn parse_tuning_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<TuningConfig> {
        let target = data
            .get(&ConfigField::Tuning)
            .and_then(parse_stock_tuning)
            .ok_or_else(|| {
                anyhow::anyhow!("Missing or unsupported mining mode/power limit in stats")
            })?;
        Ok(TuningConfig::new(target))
    }

    async fn set_tuning_config(
        &self,
        config: TuningConfig,
        _scaling_config: Option<ScalingConfig>,
    ) -> anyhow::Result<bool> {
        match config.target {
            TuningTarget::Manual { .. } => {
                anyhow::bail!(
                    "Manual tuning not supported on {}",
                    SealMinerStockFirmware::default()
                )
            }
            TuningTarget::Power(power) => self.set_power_limit(power).await,
            TuningTarget::HashRate(_) => {
                anyhow::bail!(
                    "Hashrate tuning not supported on {}",
                    SealMinerStockFirmware::default()
                )
            }
            TuningTarget::MiningMode(mode) => {
                if mode == MiningMode::Low && self.device_info.model != SealMinerModel::A2.as_ref()
                {
                    anyhow::bail!("Low-power mining mode is only supported on SealMiner A2");
                }
                Ok(self.rpc.set_mining_mode(mode).await.is_ok())
            }
            TuningTarget::Preset(_) => {
                anyhow::bail!(
                    "Preset tuning not supported on {}",
                    SealMinerStockFirmware::default()
                )
            }
        }
    }

    fn supports_tuning_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsFanConfig for SealMinerV2025 {
    fn supports_fan_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for SealMinerV2025 {
    fn supports_upgrade_firmware(&self) -> bool {
        false
    }
}

impl HasAuth for SealMinerV2025 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}

impl HasDefaultAuth for SealMinerV2025 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("seal", "seal")
    }
}

impl Validate for SealMinerV2025 {
    type Firmware = SealMinerStockFirmware;
}

impl SupportsTemperatureConfig for SealMinerV2025 {}
impl GetTuningPercent for SealMinerV2025 {}
impl SetHashboardsEnabled for SealMinerV2025 {}

impl SetTuningPercent for SealMinerV2025 {}

impl SupportsPresets for SealMinerV2025 {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use asic_rs_core::{
        data::{
            collector::DataCollector,
            command::MinerCommand,
            hashrate::{HashRate, HashRateUnit},
            miner::TuningTarget,
        },
        test::api::MockAPIClient,
    };
    use macaddr::MacAddr;
    use measurements::Power;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
    };

    use super::*;
    use crate::test::json::v2025::{
        GET_SYSTEM_INFO, MINER_PSU_STATUS, POOLS, STATS, SUMMARY, VERSION,
    };
    use asic_rs_makes_sealminer::{hardware::SealMinerControlBoard, models::SealMinerModel};

    #[tokio::test]
    async fn test_sealminer_v2025() {
        let miner = SealMinerV2025::new(IpAddr::from([10, 0, 13, 179]), SealMinerModel::A2);

        let mut results = HashMap::new();

        results.insert(
            MinerCommand::RPC {
                command: "stats",
                parameters: None,
            },
            Value::from_str(STATS).unwrap(),
        );
        results.insert(
            MinerCommand::RPC {
                command: "summary",
                parameters: None,
            },
            Value::from_str(SUMMARY).unwrap(),
        );
        results.insert(
            MinerCommand::RPC {
                command: "pools",
                parameters: None,
            },
            Value::from_str(POOLS).unwrap(),
        );
        results.insert(
            MinerCommand::RPC {
                command: "version",
                parameters: None,
            },
            Value::from_str(VERSION).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "get_system_info",
                parameters: None,
            },
            Value::from_str(GET_SYSTEM_INFO).unwrap(),
        );
        results.insert(
            MinerCommand::WebAPI {
                command: "miner-psu-status",
                parameters: None,
            },
            Value::from_str(MINER_PSU_STATUS).unwrap(),
        );

        let mock_api = MockAPIClient::new(results);
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;
        let miner_data = miner.parse_data(data);

        assert_eq!(miner_data.ip.to_string(), "10.0.13.179");
        assert_eq!(
            miner_data.mac,
            Some(MacAddr::from_str("50:d4:48:4b:86:78").unwrap())
        );
        assert_eq!(
            miner_data.serial_number,
            Some("S127200252001850".to_string())
        );
        assert_eq!(miner_data.api_version, Some("3.7".to_string()));
        assert_eq!(miner_data.firmware_version, Some("2025091001".to_string()));
        assert_eq!(
            miner_data.control_board_version,
            Some(SealMinerControlBoard::TaurusAir.into())
        );
        assert_eq!(miner_data.hashboards.len(), 3);
        assert!(miner_data.hashboards.iter().all(|b| b.active == Some(true)));
        assert!(miner_data.hashboards.iter().all(|b| b.tuned == Some(true)));
        assert_eq!(
            miner_data.hashboards[0].serial_number,
            Some("S124600252004637".to_string())
        );
        assert_eq!(miner_data.hashboards[0].working_chips, Some(153));
        assert_eq!(
            miner_data.hashrate,
            Some(HashRate {
                value: 220.298736318481,
                unit: HashRateUnit::TeraHash,
                algo: HashAlgorithm::SHA256
            })
        );
        assert_eq!(
            miner_data.expected_hashrate,
            Some(HashRate {
                value: 230.83940728559,
                unit: HashRateUnit::TeraHash,
                algo: HashAlgorithm::SHA256
            })
        );
        assert_eq!(miner_data.fans.len(), 4);
        assert_eq!(miner_data.wattage, Some(Power::from_watts(3504.0)));
        assert_eq!(
            miner_data.tuning_target,
            Some(TuningTarget::MiningMode(MiningMode::Normal))
        );
        assert_eq!(miner_data.light_flashing, Some(true));
        assert_eq!(miner_data.pools.len(), 1);
        assert_eq!(miner_data.pools[0].len(), 1);
        assert_eq!(
            miner_data.pools[0].pools[0].last_share_time,
            Some(1_773_869_544)
        );
    }

    #[tokio::test]
    async fn test_realtime_hashrate_windows_zero_and_missing() {
        let miner = SealMinerV2025::new(IpAddr::from([10, 0, 13, 179]), SealMinerModel::A2);

        // Neither lifetime averages nor short board samples are realtime rates.
        for rate in [Some(120_000_000.0), Some(0.0), None] {
            let mut stats = Value::from_str(STATS).unwrap();
            let fields = stats["STATS"][0].as_object_mut().unwrap();
            match rate {
                Some(value) => {
                    fields.insert("MHS(2m)".to_string(), Value::from(value));
                    fields.insert("0 MHS(2m)".to_string(), Value::from(value));
                }
                None => {
                    fields.remove("MHS(2m)");
                    fields.remove("0 MHS(2m)");
                }
            }
            let results = HashMap::from([
                (
                    MinerCommand::RPC {
                        command: "stats",
                        parameters: None,
                    },
                    stats,
                ),
                (
                    MinerCommand::RPC {
                        command: "summary",
                        parameters: None,
                    },
                    Value::from_str(SUMMARY).unwrap(),
                ),
            ]);
            let mock_api = MockAPIClient::new(results);
            let mut collector = DataCollector::new_with_client(&miner, &mock_api);
            let data = collector
                .collect(&[DataField::Hashrate, DataField::Hashboards])
                .await;

            let expected = rate.map(|value| HashRate {
                value: value / 1_000_000.0,
                unit: HashRateUnit::TeraHash,
                algo: HashAlgorithm::SHA256,
            });
            assert_eq!(miner.parse_hashrate(&data), expected);
            assert_eq!(miner.parse_hashboards(&data)[0].hashrate, expected);
        }
    }

    #[test]
    fn test_tuning_readback_uses_active_mode_not_saved_power_limit() {
        let miner = SealMinerV2025::new(IpAddr::from([127, 0, 0, 1]), SealMinerModel::A3ProHyd);
        for (mode, target) in [
            ("Normal", TuningTarget::MiningMode(MiningMode::Normal)),
            (
                "High Performance",
                TuningTarget::MiningMode(MiningMode::High),
            ),
            ("Low Power", TuningTarget::MiningMode(MiningMode::Low)),
        ] {
            let stats = serde_json::json!({"Mining Mode": mode, "Power Limit": 7650});
            let data = HashMap::from([(DataField::TuningTarget, stats.clone())]);
            let config = HashMap::from([(ConfigField::Tuning, stats)]);
            assert_eq!(miner.parse_tuning_target(&data), Some(target.clone()));
            assert_eq!(miner.parse_tuning_config(&config).unwrap().target, target);
        }
    }

    #[test]
    fn test_unknown_and_missing_tuning_modes() {
        let miner = SealMinerV2025::new(IpAddr::from([127, 0, 0, 1]), SealMinerModel::A3ProHyd);
        for stats in [
            serde_json::json!({"Mining Mode": "Smartboost", "Power Limit": 10000}),
            serde_json::json!({"Mining Mode": "Sleep", "Power Limit": 10000}),
            serde_json::json!({"Mining Mode": "Custom", "Power Limit": 10000}),
            serde_json::json!({"Mining Mode": null, "Power Limit": 10000}),
            serde_json::json!({}),
        ] {
            assert_eq!(parse_stock_tuning(&stats), None);
            let config = HashMap::from([(ConfigField::Tuning, stats)]);
            assert!(miner.parse_tuning_config(&config).is_err());
        }
        for stats in [
            serde_json::json!({"Power Limit": 3500}),
            serde_json::json!({"Mining Mode": "", "Power Limit": 3500}),
        ] {
            assert_eq!(
                parse_stock_tuning(&stats),
                Some(TuningTarget::Power(Power::from_watts(3500.0)))
            );
        }
        assert_eq!(
            parse_stock_tuning(&serde_json::json!({"Mining Mode": "Normal"})),
            Some(TuningTarget::MiningMode(MiningMode::Normal))
        );
    }

    #[tokio::test]
    async fn test_low_power_mode_is_rejected_on_a3() {
        let miner = SealMinerV2025::new(IpAddr::from([127, 0, 0, 1]), SealMinerModel::A3ProHyd);
        let error = miner
            .set_tuning_config(
                TuningConfig::new(TuningTarget::MiningMode(MiningMode::Low)),
                None,
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("only supported on SealMiner A2"));
        let capabilities = miner.parse_tuning_capabilities(&HashMap::new()).unwrap();
        assert!(
            !capabilities
                .presets
                .unwrap()
                .presets
                .contains(&TuningTarget::MiningMode(MiningMode::Low))
        );
    }

    #[tokio::test]
    async fn test_tuning_writes_require_explicit_success() {
        for response in [
            serde_json::json!({"STATUS": [{"STATUS": "E", "Code": 45, "Msg": "Access denied"}]}),
            serde_json::json!({"STATUS": [{"STATUS": "S", "Code": 120, "Msg": "ASC set failed"}]}),
            serde_json::json!({"STATUS": [{"STATUS": "S"}]}),
            serde_json::json!({"STATUS": []}),
            serde_json::json!({"id": 1}),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let server = tokio::spawn(async move {
                for _ in 0..2 {
                    let (socket, _) =
                        tokio::time::timeout(Duration::from_secs(2), listener.accept())
                            .await
                            .unwrap()
                            .unwrap();
                    let (reader, mut writer) = socket.into_split();
                    let mut line = String::new();
                    BufReader::new(reader).read_line(&mut line).await.unwrap();
                    writer
                        .write_all(format!("{response}\n").as_bytes())
                        .await
                        .unwrap();
                }
            });
            let ip = IpAddr::from([127, 0, 0, 1]);
            let mut miner = SealMinerV2025::new(ip, SealMinerModel::A3ProHyd);
            miner.rpc = SealMinerRPCAPI::new_with_port(ip, port);
            assert!(
                !miner
                    .set_tuning_config(
                        TuningConfig::new(TuningTarget::MiningMode(MiningMode::High)),
                        None,
                    )
                    .await
                    .unwrap()
            );
            assert!(
                !miner
                    .set_power_limit(Power::from_watts(8000.0))
                    .await
                    .unwrap()
            );
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_invalid_power_values_are_rejected_before_networking() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let api = SealMinerRPCAPI::new_with_port(IpAddr::from([127, 0, 0, 1]), port);
        for watts in [-1.0, f64::NAN, 0.5, f64::from(u32::MAX) + 1.0] {
            let error = api
                .set_power_limit(Power::from_watts(watts))
                .await
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("finite, whole, non-negative watts")
            );
        }
        let listener = listener.into_std().unwrap();
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }

    #[tokio::test]
    #[ignore = "LIVE WRITE: changes tuning, restarts mining, and leaves the requested settings in place; requires MINER_IP and SEALMINER_LIVE_TUNING_ACK=1"]
    async fn set_tuning_config_live_test() -> anyhow::Result<()> {
        use asic_rs_core::traits::entry::FirmwareEntry;

        anyhow::ensure!(
            std::env::var("SEALMINER_LIVE_TUNING_ACK").as_deref() == Ok("1"),
            "Set SEALMINER_LIVE_TUNING_ACK=1 to acknowledge live configuration changes"
        );
        let ip: IpAddr = std::env::var("MINER_IP")?.parse()?;
        let mode = std::env::var("SEALMINER_LIVE_MODE").ok();
        let power = std::env::var("SEALMINER_LIVE_POWER_WATTS").ok();
        let target = match (mode.as_deref(), power.as_deref()) {
            (Some("normal"), None) => TuningTarget::MiningMode(MiningMode::Normal),
            (Some("high_performance"), None) => TuningTarget::MiningMode(MiningMode::High),
            (Some("low_power"), None) => TuningTarget::MiningMode(MiningMode::Low),
            (None, Some(watts)) => TuningTarget::Power(Power::from_watts(watts.parse()?)),
            _ => anyhow::bail!(
                "Set exactly one of SEALMINER_LIVE_MODE=normal|high_performance|low_power or SEALMINER_LIVE_POWER_WATTS"
            ),
        };
        let miner = SealMinerStockFirmware::default()
            .build_miner(ip, None)
            .await?;
        let command = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };
        let original = miner.get_api_result(&command).await?;
        let stats = &original["STATS"][0];
        let expected_mode = match &target {
            TuningTarget::MiningMode(MiningMode::Normal) => "Normal",
            TuningTarget::MiningMode(MiningMode::High) => "High Performance",
            TuningTarget::MiningMode(MiningMode::Low) => "Low Power",
            _ => stats["Mining Mode"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing original Mining Mode"))?,
        };
        let expected_power = match &target {
            TuningTarget::Power(watts) => watts.as_watts(),
            _ => stats["Power Limit"]
                .as_f64()
                .ok_or_else(|| anyhow::anyhow!("Missing original Power Limit"))?,
        };
        anyhow::ensure!(
            miner
                .set_tuning_config(TuningConfig::new(target), None)
                .await?,
            "Firmware refused the tuning write"
        );
        tokio::time::timeout(Duration::from_secs(600), async {
            loop {
                if let Ok(response) = miner.get_api_result(&command).await {
                    let stats = &response["STATS"][0];
                    if stats["Mining Mode"].as_str() == Some(expected_mode)
                        && stats["Power Limit"].as_f64() == Some(expected_power)
                        && stats["PM State"].as_str() == Some("Running")
                    {
                        return;
                    }
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("Timed out confirming the requested mode and power cap"))?;
        Ok(())
    }
}

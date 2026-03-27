use std::{collections::HashMap, net::IpAddr, str::FromStr, time::Duration};

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
        collector::{
            DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_key,
            get_by_pointer,
        },
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        hashrate::{HashRate, HashRateUnit},
        message::{MessageSeverity, MinerMessage},
        miner::TuningTarget,
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use rpc::SealMinerRPCAPI;
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
            web: SealMinerWebAPI::new(ip),
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
            _ => Err(anyhow::anyhow!("Unsupported command type for SealMiner")),
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
                    key: Some("/STATS/0/Power Limit"),
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
        const RPC_SUMMARY: MinerCommand = MinerCommand::RPC {
            command: "summary",
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
                RPC_SUMMARY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/SUMMARY/0/MHS av"),
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
                    key: Some("/STATS/0/Power Limit"),
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
                    .get(format!("{i} MHS(Sample)"))
                    .and_then(|v| v.as_f64())
                    .map(|mhs| HashRate {
                        value: mhs,
                        unit: HashRateUnit::MegaHash,
                        algo: "SHA256".to_string(),
                    });
                let ideal_hashrate = stats
                    .get(format!("{i} MHS(Ideal)"))
                    .and_then(|v| v.as_f64())
                    .map(|mhs| HashRate {
                        value: mhs,
                        unit: HashRateUnit::MegaHash,
                        algo: "SHA256".to_string(),
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
                    intake_temperature: intake_temp,
                    outlet_temperature: outlet_temp,
                    expected_chips: self.device_info.hardware.chips,
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
        data.extract_map::<f64, _>(DataField::Hashrate, |mhs| HashRate {
            value: mhs,
            unit: HashRateUnit::MegaHash,
            algo: "SHA256".to_string(),
        })
    }
}

impl GetExpectedHashrate for SealMinerV2025 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |mhs| HashRate {
            value: mhs,
            unit: HashRateUnit::MegaHash,
            algo: "SHA256".to_string(),
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

impl GetTuningTarget for SealMinerV2025 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        data.extract_map::<f64, _>(DataField::TuningTarget, |w| {
            TuningTarget::Power(Power::from_watts(w))
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

impl GetIsMining for SealMinerV2025 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        self.parse_hashrate(data).is_some_and(|hr| hr.value > 0.0)
    }
}

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
        let param = format!("0,powerlimit,{{\"value\":\"{}\"}}", watts.as_watts() as u32);
        if self.rpc.ascset(&param).await.is_err() {
            return Ok(false);
        }
        let _ = self.web.reboot().await;
        Ok(true)
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
        let watts = data
            .get(&ConfigField::Tuning)
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow::anyhow!("No power limit in stats"))?;
        Ok(TuningConfig::new(TuningTarget::Power(Power::from_watts(
            watts,
        ))))
    }

    async fn set_tuning_config(&self, config: TuningConfig) -> anyhow::Result<bool> {
        match config.target {
            TuningTarget::Power(power) => self.set_power_limit(power).await,
            TuningTarget::HashRate(_) => {
                anyhow::bail!("Hashrate tuning not supported on SealMiner")
            }
        }
    }

    fn supports_tuning_config(&self) -> bool {
        true
    }
}

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
                value: 206860473.51,
                unit: HashRateUnit::MegaHash,
                algo: "SHA256".to_string()
            })
        );
        assert_eq!(
            miner_data.expected_hashrate,
            Some(HashRate {
                value: 230839407.28559,
                unit: HashRateUnit::MegaHash,
                algo: "SHA256".to_string()
            })
        );
        assert_eq!(miner_data.fans.len(), 4);
        assert_eq!(miner_data.wattage, Some(Power::from_watts(3504.0)));
        assert_eq!(
            miner_data.tuning_target,
            Some(TuningTarget::Power(Power::from_watts(3500.0)))
        );
        assert_eq!(miner_data.light_flashing, Some(true));
        assert_eq!(miner_data.pools.len(), 1);
        assert_eq!(miner_data.pools[0].len(), 1);
    }
}

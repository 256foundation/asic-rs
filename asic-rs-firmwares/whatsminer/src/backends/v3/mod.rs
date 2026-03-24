use std::{collections::HashMap, net::IpAddr, str::FromStr, time::Duration};

use anyhow;
use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigExtractor, ConfigField, ConfigLocation},
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
        miner::{MiningMode, TuningTarget},
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
    util::is_expected_write_error,
};
use asic_rs_makes_whatsminer::hardware::WhatsMinerControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature};
pub(crate) use rpc::WhatsMinerRPCAPI;
use serde_json::{Value, json};

use crate::backends::v2::rpc::WhatsMinerRPCAPI as WhatsMinerV2RPC;
use crate::firmware::WhatsMinerFirmware;

mod rpc;

#[derive(Debug)]
pub struct WhatsMinerV3 {
    pub ip: IpAddr,
    pub rpc: WhatsMinerRPCAPI,
    pub v2_rpc: WhatsMinerV2RPC,
    pub device_info: DeviceInfo,
}

impl WhatsMinerV3 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        let auth = Self::default_auth();
        let v2_auth = crate::backends::v2::WhatsMinerV2::default_auth();
        WhatsMinerV3 {
            ip,
            rpc: WhatsMinerRPCAPI::new(ip, None, auth),
            v2_rpc: WhatsMinerV2RPC::new(ip, None, v2_auth),
            device_info: DeviceInfo::new(
                model,
                WhatsMinerFirmware::default(),
                HashAlgorithm::SHA256,
            ),
        }
    }
}

/// V3 commands start with "get." or "set." (e.g., "get.device.info", "set.miner.mode").
/// Everything else is a V2-style command (e.g., "summary", "set_low_power").
fn is_v3_command(command: &str) -> bool {
    command.starts_with("get.") || command.starts_with("set.")
}

#[async_trait]
impl APIClient for WhatsMinerV3 {
    async fn get_api_result(&self, cmd: &MinerCommand) -> anyhow::Result<Value> {
        match cmd {
            MinerCommand::RPC { command, .. } if is_v3_command(command) => {
                self.rpc.get_api_result(cmd).await
            }
            MinerCommand::RPC { .. } => self.v2_rpc.get_api_result(cmd).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for WhatsMiner API"
            )),
        }
    }
}

impl GetConfigsLocations for WhatsMinerV3 {
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        const RPC_GET_DEVICE_INFO: MinerCommand = MinerCommand::RPC {
            command: "get.device.info",
            parameters: None,
        };
        let rpc_get_miner_status_summary: MinerCommand = MinerCommand::RPC {
            command: "get.miner.status",
            parameters: Some(json!("summary")),
        };
        match data_field {
            ConfigField::Tuning => vec![
                (
                    RPC_GET_DEVICE_INFO,
                    ConfigExtractor {
                        func: get_by_pointer,
                        key: Some("/msg/power/mode"),
                        tag: Some("mode"),
                    },
                ),
                (
                    rpc_get_miner_status_summary,
                    ConfigExtractor {
                        func: get_by_pointer,
                        key: Some("/msg/summary/power-limit"),
                        tag: Some("limit"),
                    },
                ),
            ],
            _ => vec![],
        }
    }
}

impl CollectConfigs for WhatsMinerV3 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for WhatsMinerV3 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const RPC_GET_DEVICE_INFO: MinerCommand = MinerCommand::RPC {
            command: "get.device.info",
            parameters: None,
        };
        let rpc_get_miner_status_summary: MinerCommand = MinerCommand::RPC {
            command: "get.miner.status",
            parameters: Some(json!("summary")),
        };
        let rpc_get_miner_status_pools: MinerCommand = MinerCommand::RPC {
            command: "get.miner.status",
            parameters: Some(json!("pools")),
        };
        let rpc_get_miner_status_edevs: MinerCommand = MinerCommand::RPC {
            command: "get.miner.status",
            parameters: Some(json!("edevs")),
        };

        match data_field {
            DataField::Mac => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/network/mac"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/system/api"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/system/fwversion"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/system/platform"),
                    tag: None,
                },
            )],
            DataField::SerialNumber => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/miner/miner-sn"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/network/hostname"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/system/ledstatus"),
                    tag: None,
                },
            )],
            DataField::TuningTarget => vec![
                (
                    RPC_GET_DEVICE_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/msg/power/mode"),
                        tag: Some("mode"),
                    },
                ),
                (
                    rpc_get_miner_status_summary,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/msg/summary/power-limit"),
                        tag: Some("limit"),
                    },
                ),
            ],
            DataField::Fans => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary"),
                    tag: None,
                },
            )],
            DataField::PsuFans => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/power/fanspeed"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![
                (
                    RPC_GET_DEVICE_INFO,
                    DataExtractor {
                        func: get_by_pointer,
                        key: Some("/msg/miner"),
                        tag: None,
                    },
                ),
                (
                    rpc_get_miner_status_edevs,
                    DataExtractor {
                        func: get_by_key,
                        key: Some("msg"),
                        tag: None,
                    },
                ),
            ],
            DataField::Pools => vec![(
                rpc_get_miner_status_pools,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/pools"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary/elapsed"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary/power-realtime"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary/hash-realtime"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary/factory-hash"),
                    tag: None,
                },
            )],
            DataField::FluidTemperature => vec![(
                rpc_get_miner_status_summary,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/summary/environment-temperature"),
                    tag: None,
                },
            )],
            DataField::IsMining => vec![(
                RPC_GET_DEVICE_INFO,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/msg/miner/working"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for WhatsMinerV3 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}
impl GetDeviceInfo for WhatsMinerV3 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for WhatsMinerV3 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for WhatsMinerV3 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetSerialNumber for WhatsMinerV3 {}
impl GetHostname for WhatsMinerV3 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}
impl GetApiVersion for WhatsMinerV3 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}
impl GetFirmwareVersion for WhatsMinerV3 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}
impl GetControlBoardVersion for WhatsMinerV3 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| WhatsMinerControlBoard::parse(&s).map(|cb| cb.into()))
    }
}
impl GetHashboards for WhatsMinerV3 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> = Vec::new();
        let board_count = self.device_info.hardware.boards.unwrap_or(3);
        for idx in 0..board_count {
            let hashrate = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/hash-average")))
                .and_then(|val| val.as_f64())
                .map(|f| HashRate {
                    value: f,
                    unit: HashRateUnit::TeraHash,
                    algo: String::from("SHA256"),
                });
            let expected_hashrate = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/factory-hash")))
                .and_then(|val| val.as_f64())
                .map(|f| HashRate {
                    value: f,
                    unit: HashRateUnit::TeraHash,
                    algo: String::from("SHA256"),
                });
            let board_temperature = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/chip-temp-min")))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let intake_temperature = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/chip-temp-min")))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let outlet_temperature = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/chip-temp-max")))
                .and_then(|val| val.as_f64())
                .map(Temperature::from_celsius);
            let serial_number =
                data.extract_nested::<String>(DataField::Hashboards, &format!("pcbsn{idx}"));

            let working_chips = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/effective-chips")))
                .and_then(|val| val.as_u64())
                .map(|u| u as u16);
            let frequency = data
                .get(&DataField::Hashboards)
                .and_then(|val| val.pointer(&format!("/edevs/{idx}/freq")))
                .and_then(|val| val.as_f64())
                .map(Frequency::from_megahertz);

            let active = Some(hashrate.clone().map(|h| h.value).unwrap_or(0f64) > 0f64);
            hashboards.push(BoardData {
                hashrate,
                position: idx,
                expected_hashrate,
                board_temperature,
                intake_temperature,
                outlet_temperature,
                expected_chips: self.device_info.hardware.chips,
                working_chips,
                serial_number,
                chips: vec![],
                voltage: None, // TODO
                frequency,
                tuned: Some(true),
                active,
            });
        }
        hashboards
    }
}
impl GetHashrate for WhatsMinerV3 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::TeraHash,
            algo: String::from("SHA256"),
        })
    }
}
impl GetExpectedHashrate for WhatsMinerV3 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::TeraHash,
            algo: String::from("SHA256"),
        })
    }
}
impl GetFans for WhatsMinerV3 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans: Vec<FanData> = Vec::new();
        for (idx, direction) in ["in", "out"].iter().enumerate() {
            let fan = data.extract_nested_map::<f64, _>(
                DataField::Fans,
                &format!("fan-speed-{direction}"),
                |rpm| FanData {
                    position: idx as i16,
                    rpm: Some(AngularVelocity::from_rpm(rpm)),
                },
            );
            if let Some(fan_data) = fan {
                fans.push(fan_data);
            }
        }
        fans
    }
}
impl GetPsuFans for WhatsMinerV3 {
    fn parse_psu_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut psu_fans: Vec<FanData> = Vec::new();

        let psu_fan = data.extract_map::<f64, _>(DataField::PsuFans, |rpm| FanData {
            position: 0i16,
            rpm: Some(AngularVelocity::from_rpm(rpm)),
        });
        if let Some(fan_data) = psu_fan {
            psu_fans.push(fan_data);
        }
        psu_fans
    }
}
impl GetFluidTemperature for WhatsMinerV3 {
    fn parse_fluid_temperature(&self, data: &HashMap<DataField, Value>) -> Option<Temperature> {
        data.extract_map::<f64, _>(DataField::FluidTemperature, Temperature::from_celsius)
    }
}
impl GetWattage for WhatsMinerV3 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}
/// Parses tuning target from V3 tagged data (mode from get.device.info, limit from summary).
/// Mode "0" (Low) / "2" (High) → MiningMode, "1" (Normal) / unknown → Power(limit).
fn parse_v3_tuning(tuning: &Value) -> Option<TuningTarget> {
    if let Some(mode_str) = tuning.get("mode").and_then(Value::as_str) {
        match mode_str {
            "0" => return Some(TuningTarget::MiningMode(MiningMode::Low)),
            "2" => return Some(TuningTarget::MiningMode(MiningMode::High)),
            // "1" (Normal) falls through to power limit
            other => {
                tracing::debug!("V3 power mode '{other}' treated as Normal, using power limit")
            }
        }
    }
    tuning
        .get("limit")
        .and_then(Value::as_f64)
        .map(|w| TuningTarget::Power(Power::from_watts(w)))
}

impl GetTuningTarget for WhatsMinerV3 {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        let tuning = data.get(&DataField::TuningTarget)?;
        parse_v3_tuning(tuning)
    }
}
impl GetLightFlashing for WhatsMinerV3 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract_map::<String, _>(DataField::LightFlashing, |l| l != "auto")
    }
}
impl GetMessages for WhatsMinerV3 {}
impl GetUptime for WhatsMinerV3 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}
impl GetIsMining for WhatsMinerV3 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        // working: "true" means mining is ON
        data.extract_map::<String, _>(DataField::IsMining, |l| l == "true")
            .unwrap_or(true)
    }
}
impl GetPools for WhatsMinerV3 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let mut pools: Vec<PoolData> = Vec::new();
        let pools_raw = data.get(&DataField::Pools);
        if let Some(pools_response) = pools_raw {
            for (idx, _) in pools_response
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .enumerate()
            {
                let user = data
                    .get(&DataField::Pools)
                    .and_then(|val| val.pointer(&format!("/{idx}/account")))
                    .map(|val| String::from(val.as_str().unwrap_or("")));

                let alive = data
                    .get(&DataField::Pools)
                    .and_then(|val| val.pointer(&format!("/{idx}/status")))
                    .map(|val| val.as_str())
                    .map(|val| val == Some("alive"));

                let active = data
                    .get(&DataField::Pools)
                    .and_then(|val| val.pointer(&format!("/{idx}/stratum-active")))
                    .and_then(|val| val.as_bool());

                let url = data
                    .get(&DataField::Pools)
                    .and_then(|val| val.pointer(&format!("/{idx}/url")))
                    .map(|val| PoolURL::from(String::from(val.as_str().unwrap_or(""))));

                pools.push(PoolData {
                    position: Some(idx as u16),
                    url,
                    accepted_shares: None,
                    rejected_shares: None,
                    active,
                    alive,
                    user,
                });
            }
        }
        vec![PoolGroupData {
            name: String::new(),
            quota: 1,
            pools,
        }]
    }
}

#[async_trait]
impl SetFaultLight for WhatsMinerV3 {
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        let parameters = match fault {
            false => Some(json!("auto")),
            true => Some(json!([{"color": "red", "period": 200, "duration": 100, "start": 0}])),
        };

        let data = self
            .rpc
            .send_command("set.system.led", true, parameters)
            .await;

        Ok(data.is_ok())
    }
    fn supports_set_fault_light(&self) -> bool {
        true
    }
}

#[async_trait]
impl SetPowerLimit for WhatsMinerV3 {
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        let data = self
            .rpc
            .send_command("set.miner.power_limit", true, Some(json!(limit.as_watts())))
            .await;

        Ok(data.is_ok())
    }
    fn supports_set_power_limit(&self) -> bool {
        true
    }
}
#[async_trait]
impl SupportsPoolsConfig for WhatsMinerV3 {
    async fn get_pools_config(&self) -> anyhow::Result<Vec<PoolGroupConfig>> {
        Ok(self
            .get_pools()
            .await
            .iter()
            .map(|g| g.clone().into())
            .collect())
    }

    async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> anyhow::Result<bool> {
        let group = config
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No pool groups provided"))?;

        let pools: Vec<Value> = group
            .pools
            .iter()
            .map(|pool| {
                json!({
                    "pool": pool.url.to_string(),
                    "worker": pool.username.as_str(),
                    "passwd": pool.password.as_str(),
                })
            })
            .collect();

        let res = self
            .rpc
            .send_command("set.miner.pools", true, Some(json!(pools)))
            .await;
        Ok(res.is_ok())
    }

    fn supports_pools_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for WhatsMinerV3 {
    async fn restart(&self) -> anyhow::Result<bool> {
        // Miners often reboot before responding — any error (timeout,
        // connection reset, broken pipe) likely means the miner is rebooting.
        let _ = self.rpc.send_command("set.system.reboot", true, None).await;
        Ok(true)
    }
    fn supports_restart(&self) -> bool {
        true
    }
}

#[async_trait]
impl Pause for WhatsMinerV3 {
    async fn pause(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        // might not work as intended, if issues are found then switch to "enable" + "disable"
        // see api docs - https://apidoc.whatsminer.com/#api-Miner-btminer_service_set
        let data = self
            .rpc
            .send_command("set.miner.service", true, Some(json!("stop")))
            .await;

        Ok(data.is_ok())
    }
    fn supports_pause(&self) -> bool {
        true
    }
}

#[async_trait]
impl Resume for WhatsMinerV3 {
    async fn resume(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        let data = self
            .rpc
            .send_command("set.miner.service", true, Some(json!("start")))
            .await;

        Ok(data.is_ok())
    }
    fn supports_resume(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsScalingConfig for WhatsMinerV3 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for WhatsMinerV3 {
    fn supports_upgrade_firmware(&self) -> bool {
        false
    }
}

impl HasDefaultAuth for WhatsMinerV3 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("super", "super")
    }
}

impl HasAuth for WhatsMinerV3 {
    fn set_auth(&mut self, auth: MinerAuth) {
        // WhatsMiner V3 username is always "super", V2 is always "admin"
        self.rpc
            .set_auth(MinerAuth::new("super", auth.password.expose_secret()));
        self.v2_rpc
            .set_auth(MinerAuth::new("admin", auth.password.expose_secret()));
    }
}

/// Maps a TuningConfig to the WhatsMiner V3 RPC command name and parameter.
fn tuning_config_to_v3_rpc(config: &TuningConfig) -> anyhow::Result<(&'static str, Value)> {
    match &config.target {
        TuningTarget::MiningMode(mode) => {
            let mode_str = match mode {
                MiningMode::Low => "low",
                MiningMode::Normal => "normal",
                MiningMode::High => "high",
            };
            Ok(("set.miner.mode", json!(mode_str)))
        }
        TuningTarget::Power(limit) => Ok(("set.miner.power_limit", json!(limit.as_watts()))),
        TuningTarget::HashRate(_) => {
            anyhow::bail!("HashRate tuning target is not supported on WhatsMiner")
        }
    }
}

#[async_trait]
impl SupportsTuningConfig for WhatsMinerV3 {
    async fn set_tuning_config(&self, config: TuningConfig) -> anyhow::Result<bool> {
        let is_power_target = matches!(&config.target, TuningTarget::Power(_));
        let (command, param) = tuning_config_to_v3_rpc(&config)?;
        let v3_cmd = MinerCommand::RPC {
            command,
            parameters: Some(param),
        };
        let result = self.get_api_result(&v3_cmd).await;
        if result.is_ok() {
            // Reset mode to Normal after setting a power limit so that
            // subsequent reads return Power(watts) instead of a stale mode.
            // Try V3 first; routing sends V2-style command to port 4028.
            if is_power_target {
                let v3_reset = MinerCommand::RPC {
                    command: "set.miner.mode",
                    parameters: Some(json!("normal")),
                };
                if self.get_api_result(&v3_reset).await.is_err() {
                    let v2_reset = MinerCommand::RPC {
                        command: "set_normal_power",
                        parameters: None,
                    };
                    let _ = self.get_api_result(&v2_reset).await;
                }
            }
            return Ok(true);
        }

        let err = result.unwrap_err();
        tracing::warn!("set_tuning_config V3 RPC failed: {err}");

        // Fall back to V2 for MiningMode on any V3 error — set.miner.mode is
        // not universally supported and may return an error or timeout.
        // Mining mode commands are fire-and-forget: the miner applies the
        // change but never responds, causing a read timeout.
        if let TuningTarget::MiningMode(mode) = &config.target {
            let v2_cmd = MinerCommand::RPC {
                command: match mode {
                    MiningMode::Low => "set_low_power",
                    MiningMode::Normal => "set_normal_power",
                    MiningMode::High => "set_high_power",
                },
                parameters: None,
            };
            match self.get_api_result(&v2_cmd).await {
                Ok(_) => return Ok(true),
                Err(e) if is_expected_write_error(&e) => {
                    tracing::debug!(
                        "set_tuning_config: V2 mining mode fallback didn't respond ({e}), assuming applied"
                    );
                    return Ok(true);
                }
                Err(e) => {
                    tracing::warn!("set_tuning_config V2 fallback RPC failed: {e}");
                    return Err(e);
                }
            }
        }

        Err(err)
    }

    fn parse_tuning_config(
        &self,
        data: &HashMap<ConfigField, Value>,
    ) -> anyhow::Result<TuningConfig> {
        let tuning = data
            .get(&ConfigField::Tuning)
            .ok_or_else(|| anyhow::anyhow!("No tuning data"))?;

        parse_v3_tuning(tuning)
            .map(TuningConfig::new)
            .ok_or_else(|| anyhow::anyhow!("No power mode or power limit found in tuning data"))
    }

    fn supports_tuning_config(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsFanConfig for WhatsMinerV3 {
    fn supports_fan_config(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use asic_rs_makes_whatsminer::models::WhatsMinerModel;

    use super::*;

    #[test]
    fn test_parse_is_mining_when_not_working() {
        // Arrange - working="false" means the miner is paused
        let miner = WhatsMinerV3::new(IpAddr::from([127, 0, 0, 1]), WhatsMinerModel::M60SVK30);
        let mut data = HashMap::new();
        data.insert(DataField::IsMining, Value::String("false".to_string()));

        // Act
        let is_mining = miner.parse_is_mining(&data);

        // Assert
        assert!(!is_mining);
    }

    #[test]
    fn test_tuning_config_to_rpc_mining_mode_low() {
        // Act
        let config = TuningConfig::new(TuningTarget::MiningMode(MiningMode::Low));
        let (cmd, param) = tuning_config_to_v3_rpc(&config).unwrap();

        // Assert
        assert_eq!(cmd, "set.miner.mode");
        assert_eq!(param, json!("low"));
    }

    #[test]
    fn test_tuning_config_to_rpc_mining_mode_normal() {
        // Act
        let config = TuningConfig::new(TuningTarget::MiningMode(MiningMode::Normal));
        let (cmd, param) = tuning_config_to_v3_rpc(&config).unwrap();

        // Assert
        assert_eq!(cmd, "set.miner.mode");
        assert_eq!(param, json!("normal"));
    }

    #[test]
    fn test_tuning_config_to_rpc_mining_mode_high() {
        // Act
        let config = TuningConfig::new(TuningTarget::MiningMode(MiningMode::High));
        let (cmd, param) = tuning_config_to_v3_rpc(&config).unwrap();

        // Assert
        assert_eq!(cmd, "set.miner.mode");
        assert_eq!(param, json!("high"));
    }

    #[test]
    fn test_tuning_config_to_rpc_power_limit() {
        // Act
        let config = TuningConfig::new(TuningTarget::Power(Power::from_watts(3000.0)));
        let (cmd, param) = tuning_config_to_v3_rpc(&config).unwrap();

        // Assert
        assert_eq!(cmd, "set.miner.power_limit");
        assert_eq!(param, json!(3000.0));
    }

    #[test]
    fn test_tuning_config_to_rpc_hashrate_rejected() {
        // Act
        let config = TuningConfig::new(TuningTarget::HashRate(HashRate {
            value: 200.0,
            unit: HashRateUnit::TeraHash,
            algo: "SHA256".to_string(),
        }));
        let result = tuning_config_to_v3_rpc(&config);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tuning_config_low_mode() {
        // Arrange
        let miner = WhatsMinerV3::new(IpAddr::from([127, 0, 0, 1]), WhatsMinerModel::M60SVK30);
        let mut data = HashMap::new();
        data.insert(ConfigField::Tuning, json!({"mode": "0", "limit": 3600}));

        // Act
        let config = miner.parse_tuning_config(&data).unwrap();

        // Assert
        assert_eq!(config.target, TuningTarget::MiningMode(MiningMode::Low));
    }

    #[test]
    fn test_parse_tuning_config_high_mode() {
        // Arrange
        let miner = WhatsMinerV3::new(IpAddr::from([127, 0, 0, 1]), WhatsMinerModel::M60SVK30);
        let mut data = HashMap::new();
        data.insert(ConfigField::Tuning, json!({"mode": "2", "limit": 3600}));

        // Act
        let config = miner.parse_tuning_config(&data).unwrap();

        // Assert
        assert_eq!(config.target, TuningTarget::MiningMode(MiningMode::High));
    }

    #[test]
    fn test_parse_tuning_config_normal_mode_returns_power_limit() {
        // Arrange
        let miner = WhatsMinerV3::new(IpAddr::from([127, 0, 0, 1]), WhatsMinerModel::M60SVK30);
        let mut data = HashMap::new();
        data.insert(ConfigField::Tuning, json!({"mode": "1", "limit": 3600}));

        // Act
        let config = miner.parse_tuning_config(&data).unwrap();

        // Assert
        assert_eq!(
            config.target,
            TuningTarget::Power(Power::from_watts(3600.0))
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use asic_rs_core::test::api::MockAPIClient;
    use asic_rs_makes_whatsminer::models::WhatsMinerModel;

    use super::*;
    use crate::test::json::v3::{
        GET_DEVICE_INFO_COMMAND, GET_MINER_STATUS_EDEVS_COMMAND, GET_MINER_STATUS_POOLS_COMMAND,
        GET_MINER_STATUS_SUMMARY_COMMAND,
    };

    #[tokio::test]
    async fn test_whatsminer_v3_data_parsers() -> anyhow::Result<()> {
        // Arrange
        let miner = WhatsMinerV3::new(IpAddr::from([127, 0, 0, 1]), WhatsMinerModel::M60SVK40);
        let mut results = HashMap::new();

        results.insert(
            MinerCommand::RPC {
                command: "get.device.info",
                parameters: None,
            },
            Value::from_str(GET_DEVICE_INFO_COMMAND)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "get.miner.status",
                parameters: Some(json!("summary")),
            },
            Value::from_str(GET_MINER_STATUS_SUMMARY_COMMAND)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "get.miner.status",
                parameters: Some(json!("pools")),
            },
            Value::from_str(GET_MINER_STATUS_POOLS_COMMAND)?,
        );
        results.insert(
            MinerCommand::RPC {
                command: "get.miner.status",
                parameters: Some(json!("edevs")),
            },
            Value::from_str(GET_MINER_STATUS_EDEVS_COMMAND)?,
        );

        let mock_api = MockAPIClient::new(results);

        // Act
        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;
        let miner_data = miner.parse_data(data);

        // Assert
        assert_eq!(&miner_data.ip, &miner.ip);
        assert_eq!(
            miner_data.mac,
            Some(MacAddr::from_str("CE:02:01:00:8C:36")?)
        );
        assert_eq!(miner_data.api_version, Some("3.0.2".to_string()));
        assert_eq!(
            miner_data.firmware_version,
            Some("20251209.16.Rel2".to_string())
        );
        assert_eq!(miner_data.hostname, Some("WhatsMiner".to_string()));
        assert_eq!(
            miner_data.hashrate,
            Some(HashRate {
                value: 171.259,
                unit: HashRateUnit::TeraHash,
                algo: String::from("SHA256"),
            })
        );
        assert_eq!(
            miner_data.expected_hashrate,
            Some(HashRate {
                value: 181.051,
                unit: HashRateUnit::TeraHash,
                algo: String::from("SHA256"),
            })
        );
        assert_eq!(miner_data.wattage, Some(Power::from_watts(3156.0)));
        assert_eq!(
            miner_data.tuning_target,
            Some(TuningTarget::Power(Power::from_watts(3600.0)))
        );
        assert_eq!(miner_data.uptime, Some(Duration::from_secs(50000)));
        assert!(miner_data.is_mining);
        assert_eq!(miner_data.fans.len(), 2);
        assert_eq!(miner_data.pools[0].len(), 3);

        Ok(())
    }
}

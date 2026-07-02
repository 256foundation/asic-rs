use std::{
    collections::HashMap,
    net::IpAddr,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow;
use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigField, ConfigLocation},
        pools::PoolGroupConfig,
    },
    data::{
        board::{BoardData, ChipData, MinerControlBoard},
        collector::{
            DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
        },
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        hashrate::{HashRate, HashRateUnit},
        miner::TuningTarget,
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_avalon::hardware::AvalonMinerControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use rpc::AvalonMinerRPCAPI;
use serde_json::{Value, json};

use crate::firmware::AvalonStockFirmware;

mod rpc;

#[derive(Debug)]
pub struct AvalonAMiner {
    ip: IpAddr,
    rpc: AvalonMinerRPCAPI,
    device_info: DeviceInfo,
}

impl AvalonAMiner {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        Self {
            ip,
            rpc: AvalonMinerRPCAPI::new(ip),
            device_info: DeviceInfo::new(
                model,
                AvalonStockFirmware::default(),
                HashAlgorithm::SHA256,
            ),
        }
    }
}

#[async_trait]
impl APIClient for AvalonAMiner {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported command type for AvalonMiner API"
            )),
        }
    }
}

#[async_trait]
impl Restart for AvalonAMiner {
    async fn restart(&self) -> anyhow::Result<bool> {
        let data = self.rpc.send_command("restart", false, None).await?;

        if let Some(status) = data.get("STATUS").and_then(|s| s.as_str()) {
            return Ok(status == "RESTART");
        }

        Ok(false)
    }
    fn supports_restart(&self) -> bool {
        true
    }
}
#[async_trait]
impl Pause for AvalonAMiner {
    async fn pause(&self, after: Option<Duration>) -> anyhow::Result<bool> {
        let offset = after.unwrap_or(Duration::from_secs(5));
        let shutdown_time = SystemTime::now() + offset;

        let timestamp = shutdown_time
            .duration_since(UNIX_EPOCH)
            .map_err(|_| anyhow::anyhow!("shutdown time is before UNIX epoch"))?
            .as_secs();

        let data = self
            .rpc
            .send_command(
                "ascset",
                false,
                Some(json!(["0", format!("softoff,1:{}", timestamp)])),
            )
            .await?;

        if let Some(status) = data.get("STATUS").and_then(|s| s.as_array())
            && !status.is_empty()
            && let Some(status_code) = status[0].get("STATUS").and_then(|s| s.as_str())
            && status_code == "S"
            && let Some(msg) = status[0].get("Msg").and_then(|m| m.as_str())
        {
            return Ok(msg == "ASC 0 set OK");
        }

        Ok(false)
    }
    fn supports_pause(&self) -> bool {
        true
    }
}
#[async_trait]
impl Resume for AvalonAMiner {
    async fn resume(&self, after: Option<Duration>) -> anyhow::Result<bool> {
        let offset = after.unwrap_or(Duration::from_secs(5));
        let shutdown_time = SystemTime::now() + offset;

        let timestamp = shutdown_time
            .duration_since(UNIX_EPOCH)
            .map_err(|_| anyhow::anyhow!("shutdown time is before UNIX epoch"))?
            .as_secs();

        let result = self
            .rpc
            .send_command(
                "ascset",
                false,
                Some(json!(["0", format!("softon,1:{}", timestamp)])),
            )
            .await;

        match result {
            Ok(data) => {
                if let Some(status) = data.get("STATUS").and_then(|s| s.as_array())
                    && !status.is_empty()
                    && let Some(status_code) = status[0].get("STATUS").and_then(|s| s.as_str())
                    && status_code == "S"
                    && let Some(msg) = status[0].get("Msg").and_then(|m| m.as_str())
                {
                    return Ok(msg == "ASC 0 set OK");
                }
                Ok(false)
            }
            // softon closes the connection without responding — treat as success
            Err(e)
                if e.to_string().contains("No data received")
                    || e.to_string().contains("timed out") =>
            {
                Ok(true)
            }
            Err(e) => Err(e),
        }
    }
    fn supports_resume(&self) -> bool {
        true
    }
}

impl ChangePassword for AvalonAMiner {
    fn supports_change_password(&self) -> bool {
        false
    }
}

impl ReadLogs for AvalonAMiner {
    fn supports_read_logs(&self) -> bool {
        false
    }
}

impl FactoryReset for AvalonAMiner {
    fn supports_factory_reset(&self) -> bool {
        false
    }
}

#[async_trait]
impl SetFaultLight for AvalonAMiner {
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        let command = if fault { "1-1" } else { "1-0" };

        let data = self
            .rpc
            .send_command("ascset", false, Some(json!(["0", "led", command])))
            .await?;

        if let Some(status) = data.get("STATUS").and_then(|s| s.as_array())
            && let Some(msg) = status
                .first()
                .and_then(|s| s.get("Msg"))
                .and_then(|m| m.as_str())
        {
            return Ok(msg == "ASC 0 set OK");
        }

        Err(anyhow::anyhow!("Failed to set fault light to {}", command))
    }
    fn supports_set_fault_light(&self) -> bool {
        true
    }
}

#[async_trait]
impl SetPowerLimit for AvalonAMiner {
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        let data = self
            .rpc
            .send_command(
                "ascset",
                false,
                Some(json!(["0", "worklevel,set", limit.to_string()])),
            )
            .await?;

        if let Some(status) = data.get("STATUS").and_then(|s| s.as_array())
            && !status.is_empty()
            && let Some(msg) = status[0].get("Msg").and_then(|m| m.as_str())
        {
            return Ok(msg == "ASC 0 set OK");
        }

        Err(anyhow::anyhow!("Failed to set power limit"))
    }
    fn supports_set_power_limit(&self) -> bool {
        true
    }
}

#[async_trait]
impl SupportsPoolsConfig for AvalonAMiner {
    async fn get_pools_config(&self) -> anyhow::Result<Vec<PoolGroupConfig>> {
        Ok(self
            .get_pools()
            .await
            .iter()
            .map(|g| g.clone().into())
            .collect())
    }

    fn supports_pools_config(&self) -> bool {
        false
    }
}

impl GetConfigsLocations for AvalonAMiner {
    #[allow(unused_variables)]
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}

impl CollectConfigs for AvalonAMiner {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for AvalonAMiner {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const RPC_VERSION: MinerCommand = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };
        const RPC_STATS: MinerCommand = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };
        const RPC_DEVS: MinerCommand = MinerCommand::RPC {
            command: "devs",
            parameters: None,
        };
        const RPC_POOLS: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };

        match data_field {
            DataField::Mac => vec![(
                RPC_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/VERSION/0/MAC"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                RPC_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/VERSION/0/HWTYPE"),
                    tag: None,
                },
            )],
            DataField::SerialNumber => vec![(
                RPC_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/VERSION/0/DNA"),
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
                    key: Some("/VERSION/0/VERSION"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                RPC_DEVS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/DEVS/0/MHS 1m"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0/GHSmm"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0"),
                    tag: None,
                },
            )],
            DataField::Chips => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0/PS"),
                    tag: None,
                },
            )],
            DataField::TuningTarget => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0/PS"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0/Led"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/Elapsed"),
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

impl GetIP for AvalonAMiner {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for AvalonAMiner {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for AvalonAMiner {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for AvalonAMiner {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac).and_then(|raw| {
            let mut mac = raw.trim().to_lowercase();
            // compact 12-digit → colon-separated
            if mac.len() == 12 && !mac.contains(':') {
                let mut colon = String::with_capacity(17);
                for (i, byte) in mac.chars().enumerate() {
                    if i > 0 && i % 2 == 0 {
                        colon.push(':');
                    }
                    colon.push(byte);
                }
                mac = colon;
            }
            MacAddr::from_str(&mac).ok()
        })
    }
}

impl GetSerialNumber for AvalonAMiner {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::SerialNumber)
    }
}

impl GetControlBoardVersion for AvalonAMiner {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .map(|s| {
                AvalonMinerControlBoard::parse(&s)
                    .map(|cb| cb.into())
                    .unwrap_or_else(|| MinerControlBoard::unknown(s))
            })
    }
}

impl GetHostname for AvalonAMiner {}
impl SupportsTimezoneConfig for AvalonAMiner {}

impl GetApiVersion for AvalonAMiner {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for AvalonAMiner {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetHashboards for AvalonAMiner {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> =
            (0..self.device_info.hardware.board_count().unwrap_or(0))
                .map(|idx| {
                    BoardData::new(idx, self.device_info.hardware.chips_for_board(idx as usize))
                })
                .collect();

        let Some(hb_info) = data.get(&DataField::Hashboards).and_then(|v| v.as_object()) else {
            return hashboards;
        };
        let chip_info = data.get(&DataField::Chips).and_then(|v| v.as_object());

        fn array_f64(stats: &serde_json::Map<String, Value>, key: &str, idx: usize) -> Option<f64> {
            stats
                .get(key)
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.get(idx))
                .and_then(|v| v.as_f64())
        }

        fn average_array_f64(stats: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
            let values = stats
                .get(key)
                .and_then(|v| v.as_array())?
                .iter()
                .filter_map(|v| v.as_f64())
                .collect::<Vec<_>>();

            if values.is_empty() {
                None
            } else {
                Some(values.iter().sum::<f64>() / values.len() as f64)
            }
        }

        for board in hashboards.iter_mut() {
            let idx = board.position as usize;

            board.hashrate = array_f64(hb_info, "MGHS", idx).map(|r| {
                HashRate {
                    value: r,
                    unit: HashRateUnit::GigaHash,
                    algo: "SHA256".to_string(),
                }
                .as_unit(HashRateUnit::default())
            });

            board.board_temperature =
                array_f64(hb_info, "MTavg", idx).map(Temperature::from_celsius);

            board.outlet_chip_temperature =
                array_f64(hb_info, "MTmax", idx).map(Temperature::from_celsius);

            board.inlet_chip_temperature =
                array_f64(hb_info, "ITemp", idx).map(Temperature::from_celsius);

            board.voltage = array_f64(hb_info, "MVavg", idx).map(Voltage::from_millivolts);

            board.frequency = average_array_f64(hb_info, &format!("SF{idx}"))
                .or_else(|| average_array_f64(hb_info, &format!("ATABD{idx}")))
                .or_else(|| hb_info.get("Freq").and_then(|v| v.as_f64()))
                .map(Frequency::from_megahertz);

            board.active = board.hashrate.as_ref().map(|h| h.value > 0.0);
            if chip_info.is_none() {
                board.working_chips = match (board.active, board.expected_chips) {
                    (Some(true), Some(expected_chips)) => Some(expected_chips),
                    (Some(false), _) => Some(0),
                    _ => None,
                };
            }

            if let Some(chip_info) = chip_info {
                let chip_temps: Vec<f64> = chip_info
                    .get(&format!("PVT_T{idx}"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();

                let chip_volts: Vec<f64> = chip_info
                    .get(&format!("PVT_V{idx}"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();

                let chip_works: Vec<f64> = chip_info
                    .get(&format!("MW{idx}"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();

                let max_len = chip_temps.len().max(chip_volts.len()).max(chip_works.len());

                for pos in 0..max_len {
                    let temp = chip_temps.get(pos).copied().unwrap_or(0.0);
                    let volt = chip_volts.get(pos).copied().unwrap_or(0.0);
                    let work = chip_works.get(pos).copied().unwrap_or(0.0);

                    if temp == 0.0 {
                        continue;
                    }

                    board.chips.push(ChipData {
                        position: pos as u16,
                        temperature: Some(Temperature::from_celsius(temp)),
                        voltage: Some(Voltage::from_millivolts(volt)),
                        working: Some(work > 0.0),
                        ..Default::default()
                    });
                }

                board.working_chips = Some(
                    board
                        .chips
                        .iter()
                        .filter(|chip| chip.working.unwrap_or(false))
                        .count() as u16,
                );
            }
        }

        hashboards
    }
}

impl GetHashrate for AvalonAMiner {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| {
            HashRate {
                value: f,
                unit: HashRateUnit::MegaHash,
                algo: "SHA256".to_string(),
            }
            .as_unit(HashRateUnit::default())
        })
    }
}

impl GetExpectedHashrate for AvalonAMiner {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| {
            HashRate {
                value: f,
                unit: HashRateUnit::GigaHash,
                algo: "SHA256".to_string(),
            }
            .as_unit(HashRateUnit::default())
        })
    }
}

impl GetFans for AvalonAMiner {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let stats = match data.get(&DataField::Fans) {
            Some(v) => v,
            _ => return Vec::new(),
        };

        let expected_fans = self.device_info.hardware.fans.unwrap_or(0) as usize;
        if expected_fans == 0 {
            return Vec::new();
        }

        (1..=expected_fans)
            .filter_map(|idx| {
                let key = format!("Fan{idx}");
                stats
                    .get(&key)
                    .and_then(|val| val.as_f64())
                    .map(|rpm| FanData {
                        position: idx as i16,
                        rpm: Some(AngularVelocity::from_rpm(rpm)),
                    })
            })
            .collect()
    }
}

impl GetPsuFans for AvalonAMiner {}

impl GetWattage for AvalonAMiner {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        let wattage = data.get(&DataField::Wattage).and_then(|v| v.as_array())?;
        let wattage = wattage.get(4).and_then(|watts: &Value| watts.as_f64())?;
        Some(Power::from_watts(wattage))
    }
}

impl GetTuningTarget for AvalonAMiner {
    fn parse_tuning_target(&self, data: &HashMap<DataField, Value>) -> Option<TuningTarget> {
        let limit = data
            .get(&DataField::TuningTarget)
            .and_then(|v| v.as_array())?;
        let limit = limit.get(6).and_then(|watts: &Value| watts.as_f64())?;
        Some(TuningTarget::Power(Power::from_watts(limit)))
    }
}

impl GetScaledTuningTarget for AvalonAMiner {}
impl GetTuningCapabilities for AvalonAMiner {}
impl GetLightFlashing for AvalonAMiner {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing)
    }
}

impl GetMessages for AvalonAMiner {}

impl GetUptime for AvalonAMiner {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetFluidTemperature for AvalonAMiner {}
impl GetIsMining for AvalonAMiner {}

impl GetPools for AvalonAMiner {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let pools = data
            .get(&DataField::Pools)
            .and_then(|v| v.as_array())
            .map(|slice| slice.to_vec())
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(idx, pool)| PoolData {
                url: pool
                    .get("URL")
                    .and_then(|v| v.as_str())
                    .map(|x| PoolURL::from(x.to_owned())),
                user: pool.get("User").and_then(|v| v.as_str()).map(|s| s.into()),
                position: Some(idx as u16),
                alive: pool
                    .get("Status")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "Alive"),
                active: pool.get("Stratum Active").and_then(|v| v.as_bool()),
                accepted_shares: pool.get("Accepted").and_then(|v| v.as_u64()),
                rejected_shares: pool.get("Rejected").and_then(|v| v.as_u64()),
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
impl SupportsScalingConfig for AvalonAMiner {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl UpgradeFirmware for AvalonAMiner {
    fn supports_upgrade_firmware(&self) -> bool {
        false
    }
}

impl HasAuth for AvalonAMiner {}
impl HasDefaultAuth for AvalonAMiner {}

#[async_trait]
impl SupportsTuningConfig for AvalonAMiner {
    fn supports_tuning_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsFanConfig for AvalonAMiner {
    fn supports_fan_config(&self) -> bool {
        false
    }
}

impl SupportsTemperatureConfig for AvalonAMiner {}
impl GetTuningPercent for AvalonAMiner {}
impl SetTuningPercent for AvalonAMiner {}

#[cfg(test)]
mod tests {
    use asic_rs_core::data::board::MinerControlBoard;
    use asic_rs_core::test::api::MockAPIClient;
    use asic_rs_makes_avalon::models::AvalonMinerModel;
    use serde_json::json;

    use super::*;
    use crate::test::json::AVALON_A_STATS_PARSED;

    #[tokio::test]
    async fn test_avalon_a() -> anyhow::Result<()> {
        let miner = AvalonAMiner::new(IpAddr::from([127, 0, 0, 1]), AvalonMinerModel::Avalon1246);
        let mut results = HashMap::new();
        let stats_cmd: MinerCommand = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };
        let version_cmd: MinerCommand = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };

        results.insert(stats_cmd, Value::from_str(AVALON_A_STATS_PARSED)?);
        results.insert(
            version_cmd,
            json!({
                "STATUS": [{"STATUS": "S", "Msg": "CGMiner versions"}],
                "VERSION": [{
                    "API": "3.7",
                    "DNA": "02010000cbd2fd6d",
                    "HWTYPE": "MM4v2_X3",
                    "VERSION": "24102401_25462b2_9ddf522"
                }]
            }),
        );

        let mock_api = MockAPIClient::new(results);

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect(&[DataField::Hashboards]).await;
        assert!(!data.contains_key(&DataField::Chips));
        let hashboards_without_chips = miner.parse_hashboards(&data);
        assert!(hashboards_without_chips[0].chips.is_empty());
        assert!(hashboards_without_chips[0].hashrate.is_some());
        assert_eq!(hashboards_without_chips[0].working_chips, Some(120));

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector
            .collect(&[DataField::Hashboards, DataField::Chips])
            .await;
        let hashboards_with_chips = miner.parse_hashboards(&data);
        assert_eq!(hashboards_with_chips[0].chips.len(), 120);
        assert_eq!(
            hashboards_without_chips[0].hashrate,
            hashboards_with_chips[0].hashrate
        );
        assert_eq!(
            hashboards_without_chips[0].working_chips,
            hashboards_with_chips[0].working_chips
        );

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;

        let miner_data = miner.parse_data(data);

        assert_eq!(miner_data.uptime, Some(Duration::from_secs(24684)));
        assert_eq!(
            miner_data.serial_number,
            Some("02010000cbd2fd6d".to_string())
        );
        assert_eq!(
            miner_data.control_board_version,
            Some(MinerControlBoard::known("MM4v2X3".to_string()))
        );
        assert_eq!(
            miner_data.expected_hashrate,
            Some(HashRate {
                value: 83.92304,
                unit: HashRateUnit::TeraHash,
                algo: "SHA256".to_string()
            })
        );
        assert_eq!(miner_data.wattage, Some(Power::from_watts(3189.0)));
        assert_eq!(miner_data.fans.len(), 4);
        assert_eq!(miner_data.hashboards[0].chips.len(), 120);
        assert_eq!(
            miner_data.hashboards[0].outlet_chip_temperature,
            Some(Temperature::from_celsius(77.0))
        );
        assert_eq!(
            miner_data.hashboards[0].frequency,
            Some(Frequency::from_megahertz(502.0))
        );
        assert_eq!(
            miner_data.average_temperature,
            Some(Temperature::from_celsius(65.0))
        );

        Ok(())
    }
}

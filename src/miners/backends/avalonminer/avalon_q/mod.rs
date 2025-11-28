use anyhow;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Power, Temperature, Voltage};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::data::board::{BoardData, ChipData};
use crate::data::device::MinerMake;
use crate::data::device::{DeviceInfo, HashAlgorithm, MinerFirmware, MinerModel};
use crate::data::fan::FanData;
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::pool::{PoolData, PoolURL};
use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;
use crate::miners::data::{
    DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
};

use rpc::AvalonMinerRPCAPI;

mod rpc;

#[derive(Debug)]
pub struct AvalonQMiner {
    ip: IpAddr,
    rpc: AvalonMinerRPCAPI,
    device_info: DeviceInfo,
}

impl AvalonQMiner {
    pub fn new(ip: IpAddr, model: MinerModel) -> Self {
        Self {
            ip,
            rpc: AvalonMinerRPCAPI::new(ip),
            device_info: DeviceInfo::new(
                MinerMake::AvalonMiner,
                model,
                MinerFirmware::Stock,
                HashAlgorithm::SHA256,
            ),
        }
    }

    /// Reboot the miner
    pub async fn reboot(&self) -> anyhow::Result<bool> {
        let data = self.rpc.send_command("restart", false, None).await?;

        if let Some(status) = data.get("STATUS").and_then(|s| s.as_str()) {
            return Ok(status == "RESTART");
        }

        Ok(false)
    }
}

#[async_trait]
impl APIClient for AvalonQMiner {
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
impl Pause for AvalonQMiner {
    async fn pause(&self, after: Option<Duration>) -> anyhow::Result<bool> {
        let offset = after.unwrap_or(Duration::from_secs(5));
        let shutdown_time = SystemTime::now() + offset;

        let timestamp = shutdown_time
            .duration_since(UNIX_EPOCH)
            .expect("Shutdown time is before UNIX epoch")
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
            && status_code == "I"
            && let Some(msg) = status[0].get("Msg").and_then(|m| m.as_str())
        {
            return Ok(msg.contains("success softoff"));
        }

        Ok(false)
    }
}
#[async_trait]
impl Resume for AvalonQMiner {
    async fn resume(&self, after: Option<Duration>) -> anyhow::Result<bool> {
        let offset = after.unwrap_or(Duration::from_secs(5));
        let shutdown_time = SystemTime::now() + offset;

        let timestamp = shutdown_time
            .duration_since(UNIX_EPOCH)
            .expect("Shutdown time is before UNIX epoch")
            .as_secs();

        let data = self
            .rpc
            .send_command(
                "ascset",
                false,
                Some(json!(["0", format!("softon,1:{}", timestamp)])),
            )
            .await?;

        if let Some(status) = data.get("STATUS").and_then(|s| s.as_array())
            && !status.is_empty()
            && let Some(status_code) = status[0].get("STATUS").and_then(|s| s.as_str())
            && status_code == "I"
            && let Some(msg) = status[0].get("Msg").and_then(|m| m.as_str())
        {
            return Ok(msg.contains("success softon"));
        }
        Ok(false)
    }
}
#[async_trait]
impl SetFaultLight for AvalonQMiner {
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
}

#[async_trait]
impl SetPowerLimit for AvalonQMiner {
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
}

#[async_trait]
impl Restart for AvalonQMiner {
    async fn restart(&self) -> anyhow::Result<bool> {
        anyhow::bail!("Unsupported command");
    }
}

impl GetDataLocations for AvalonQMiner {
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
                    key: Some("/VERSION/0/CGMiner"),
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
                    key: Some("/STATS/0/MM ID0:Summary/STATS/GHSmm"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/HBinfo"),
                    tag: None,
                },
            )],
            DataField::AverageTemperature => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0:Summary/STATS/ITemp"),
                    tag: None,
                },
            )],
            DataField::WattageLimit => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0:Summary/STATS/MPO"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0:Summary/STATS/WALLPOWER"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0:Summary/STATS"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0:Summary/STATS/Led"),
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

impl GetIP for AvalonQMiner {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for AvalonQMiner {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info
    }
}

impl CollectData for AvalonQMiner {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for AvalonQMiner {
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

impl GetSerialNumber for AvalonQMiner {}

impl GetHostname for AvalonQMiner {}

impl GetApiVersion for AvalonQMiner {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for AvalonQMiner {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetControlBoardVersion for AvalonQMiner {}

impl GetHashboards for AvalonQMiner {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let hw = &self.device_info.hardware;
        let board_cnt = hw.boards.unwrap_or(1) as usize;
        let chips_per = hw.chips.unwrap_or(0);

        let hb_info = match data.get(&DataField::Hashboards).and_then(|v| v.as_object()) {
            Some(v) => v,
            _ => return Vec::new(),
        };

        let summary = match data.get(&DataField::Fans) {
            Some(v) => v,
            _ => return Vec::new(),
        }; //some HB info is grouped with fan data.

        (0..board_cnt)
            .map(|idx| {
                let key = format!("HB{idx}");

                // per-board aggregates
                let intake = summary["ITemp"][idx]
                    .as_f64()
                    .map(Temperature::from_celsius);

                let board_t = summary["HBITemp"][idx]
                    .as_f64()
                    .map(Temperature::from_celsius);

                let hashrate = summary["MGHS"][idx].as_f64().map(|r| HashRate {
                    value: r,
                    unit: HashRateUnit::GigaHash,
                    algo: "SHA256".into(),
                });

                // per-chip arrays
                let temps: Vec<f64> = hb_info[&key]["PVT_T0"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();

                let volts: Vec<f64> = hb_info[&key]["PVT_V0"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();

                let works: Vec<f64> = hb_info[&key]["MW0"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();

                let chips: Vec<ChipData> = temps
                    .iter()
                    .zip(volts.iter())
                    .zip(works.iter())
                    .enumerate()
                    .map(|(pos, ((&t, &v), &w))| ChipData {
                        position: pos as u16,
                        temperature: Some(Temperature::from_celsius(t)),
                        voltage: Some(Voltage::from_millivolts(v)),
                        working: Some(w > 0.0),
                        ..Default::default()
                    })
                    .collect();

                BoardData {
                    position: idx as u8,
                    expected_chips: Some(chips_per),
                    working_chips: Some(chips.len() as u16),
                    chips: chips.clone(),
                    intake_temperature: intake,
                    board_temperature: board_t,
                    hashrate,
                    active: Some(!chips.is_empty()),
                    ..Default::default()
                }
            })
            .collect()
    }
}

impl GetHashrate for AvalonQMiner {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::MegaHash,
            algo: "SHA256".into(),
        })
    }
}

impl GetExpectedHashrate for AvalonQMiner {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".into(),
        })
    }
}

impl GetFans for AvalonQMiner {
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

impl GetPsuFans for AvalonQMiner {}

impl GetWattage for AvalonQMiner {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}

impl GetWattageLimit for AvalonQMiner {
    fn parse_wattage_limit(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::WattageLimit, Power::from_watts)
    }
}

impl GetLightFlashing for AvalonQMiner {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing)
    }
}

impl GetMessages for AvalonQMiner {}

impl GetUptime for AvalonQMiner {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetFluidTemperature for AvalonQMiner {}
impl GetIsMining for AvalonQMiner {}

impl GetPools for AvalonQMiner {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolData> {
        data.get(&DataField::Pools)
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
                group: None,
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
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::device::models::avalon::AvalonMinerModel::AvalonHomeQ;
    use crate::test::api::MockAPIClient;
    use crate::test::json::cgminer::avalon::{
        DEVS_COMMAND, PARSED_STATS_COMMAND, POOLS_COMMAND, VERSION_COMMAND,
    };

    #[tokio::test]

    async fn test_avalon_home_q() -> anyhow::Result<()> {
        let miner = AvalonQMiner::new(
            IpAddr::from([127, 0, 0, 1]),
            MinerModel::AvalonMiner(AvalonHomeQ),
        );

        let mut results = HashMap::new();
        let version_cmd: MinerCommand = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };
        let stats_cmd: MinerCommand = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };
        let devs_cmd: MinerCommand = MinerCommand::RPC {
            command: "devs",
            parameters: None,
        };
        let pools_cmd: MinerCommand = MinerCommand::RPC {
            command: "pools",
            parameters: None,
        };

        results.insert(stats_cmd, Value::from_str(PARSED_STATS_COMMAND)?);
        results.insert(devs_cmd, Value::from_str(DEVS_COMMAND)?);
        results.insert(pools_cmd, Value::from_str(POOLS_COMMAND)?);
        results.insert(version_cmd, Value::from_str(VERSION_COMMAND)?);

        let mock_api = MockAPIClient::new(results);

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;

        let miner_data = miner.parse_data(data);

        assert_eq!(miner_data.uptime, Some(Duration::from_secs(37819)));
        assert_eq!(miner_data.wattage_limit, Some(Power::from_watts(800.0)));
        assert_eq!(miner_data.fans.len(), 4);
        assert_eq!(miner_data.hashboards[0].chips.len(), 160);

        Ok(())
    }
}

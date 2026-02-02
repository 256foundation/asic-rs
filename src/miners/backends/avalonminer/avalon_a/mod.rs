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
use crate::data::device::{DeviceInfo, HashAlgorithm, MinerFirmware, MinerModel};
use crate::data::device::{MinerControlBoard, MinerMake};
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
pub struct AvalonAMiner {
    ip: IpAddr,
    rpc: AvalonMinerRPCAPI,
    device_info: DeviceInfo,
}

impl AvalonAMiner {
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
    fn supports_resume(&self) -> bool {
        true
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
                    key: Some("/STATS/0/MM ID0/STATS/GHSmm"),
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
            DataField::Wattage => vec![(
                RPC_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/STATS/0/MM ID0/PS"),
                    tag: None,
                },
            )],
            DataField::WattageLimit => vec![(
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
        self.device_info
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

impl GetSerialNumber for AvalonAMiner {}

impl GetControlBoardVersion for AvalonAMiner {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.extract::<String>(DataField::ControlBoardVersion)
            .and_then(|s| MinerControlBoard::from_str(&s).ok())
    }
}

impl GetHostname for AvalonAMiner {}

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
        let hw = &self.device_info.hardware;
        let board_cnt = hw.boards.unwrap_or(1) as usize;
        let chips_per = hw.chips.unwrap_or(0);

        let hb_info = match data.get(&DataField::Hashboards).and_then(|v| v.as_object()) {
            Some(v) => v,
            _ => return Vec::new(),
        };

        (0..board_cnt)
            .map(|idx| {
                let _chip_temp = hb_info
                    .get("MTmax")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.get(idx))
                    .and_then(|v| v.as_f64())
                    .map(Temperature::from_celsius);

                let board_temp = hb_info
                    .get("MTavg")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.get(idx))
                    .and_then(|v| v.as_f64())
                    .map(Temperature::from_celsius);

                let intake_temp = hb_info
                    .get("ITemp")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.get(idx))
                    .and_then(|v| v.as_f64())
                    .map(Temperature::from_celsius);

                let hashrate = hb_info
                    .get("MGHS")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.get(idx))
                    .and_then(|v| v.as_f64())
                    .map(|r| HashRate {
                        value: r,
                        unit: HashRateUnit::GigaHash,
                        algo: "SHA256".into(),
                    });

                let chip_temps: Vec<f64> = hb_info
                    .get(&format!("PVT_T{idx}"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();

                let chip_volts: Vec<f64> = hb_info
                    .get(&format!("PVT_V{idx}"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();

                let chip_works: Vec<f64> = hb_info
                    .get(&format!("MW{idx}"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();

                let mut chips = Vec::new();
                let max_len = chip_temps.len().max(chip_volts.len()).max(chip_works.len());

                for pos in 0..max_len {
                    let temp = chip_temps.get(pos).copied().unwrap_or(0.0);
                    let volt = chip_volts.get(pos).copied().unwrap_or(0.0);
                    let work = chip_works.get(pos).copied().unwrap_or(0.0);

                    if temp == 0.0 {
                        continue;
                    }

                    chips.push(ChipData {
                        position: pos as u16,
                        temperature: Some(Temperature::from_celsius(temp)),
                        voltage: Some(Voltage::from_millivolts(volt)),
                        working: Some(work > 0.0),
                        ..Default::default()
                    });
                }

                let working_chips = chips.len() as u16;
                let missing = working_chips == 0;

                BoardData {
                    position: idx as u8,
                    expected_chips: Some(chips_per),
                    working_chips: Some(working_chips),
                    chips,
                    intake_temperature: intake_temp,
                    board_temperature: board_temp,
                    hashrate,
                    active: Some(!missing),
                    ..Default::default()
                }
            })
            .collect()
    }
}

impl GetHashrate for AvalonAMiner {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::MegaHash,
            algo: "SHA256".into(),
        })
    }
}

impl GetExpectedHashrate for AvalonAMiner {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".into(),
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

impl GetWattageLimit for AvalonAMiner {
    fn parse_wattage_limit(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        let limit = data
            .get(&DataField::WattageLimit)
            .and_then(|v| v.as_array())?;
        let limit = limit.get(6).and_then(|watts: &Value| watts.as_f64())?;
        Some(Power::from_watts(limit))
    }
}

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
    use crate::data::device::models::avalon::AvalonMinerModel::Avalon1246;
    use crate::test::api::MockAPIClient;
    use crate::test::json::cgminer::avalon::AVALON_A_STATS_PARSED;

    #[tokio::test]
    async fn test_avalon_a() -> anyhow::Result<()> {
        let miner = AvalonAMiner::new(
            IpAddr::from([127, 0, 0, 1]),
            MinerModel::AvalonMiner(Avalon1246),
        );
        let mut results = HashMap::new();
        let stats_cmd: MinerCommand = MinerCommand::RPC {
            command: "stats",
            parameters: None,
        };

        results.insert(stats_cmd, Value::from_str(AVALON_A_STATS_PARSED)?);

        let mock_api = MockAPIClient::new(results);

        let mut collector = DataCollector::new_with_client(&miner, &mock_api);
        let data = collector.collect_all().await;

        let miner_data = miner.parse_data(data);

        assert_eq!(miner_data.uptime, Some(Duration::from_secs(24684)));
        assert_eq!(miner_data.wattage, Some(Power::from_watts(3189.0)));
        assert_eq!(miner_data.fans.len(), 4);
        assert_eq!(miner_data.hashboards[0].chips.len(), 120);
        assert_eq!(
            miner_data.average_temperature,
            Some(Temperature::from_celsius(65.0))
        );

        Ok(())
    }
}

use std::{collections::HashMap, net::IpAddr, str::FromStr, time::Duration};

use asic_rs_core::{
    config::{
        collector::{ConfigCollector, ConfigField, ConfigLocation},
        pools::PoolGroupConfig,
    },
    data::{
        board::{BoardData, MinerControlBoard},
        collector::{DataCollector, DataExtractor, DataField, DataLocation, get_by_pointer},
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        hashrate::{HashRate, HashRateUnit},
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
};
use asic_rs_makes_futurebit::hardware::FutureBitControlBoard;
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Power, Temperature};
use serde_json::Value;

use crate::firmware::ApolloFirmware;

mod web;

pub use web::ApolloGraphQLAPI;

#[derive(Debug)]
pub struct ApolloV2 {
    ip: IpAddr,
    graphql: ApolloGraphQLAPI,
    device_info: DeviceInfo,
}

impl ApolloV2 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        let auth = Self::default_auth();
        Self {
            ip,
            graphql: ApolloGraphQLAPI::new(ip, auth),
            device_info: DeviceInfo::new(model, ApolloFirmware::default(), HashAlgorithm::SHA256),
        }
    }
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
    })
}

fn as_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(|v| {
        v.as_u64().or_else(|| {
            v.as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .map(|f| f as u64)
        })
    })
}

fn is_zero_mac(mac: &MacAddr) -> bool {
    match mac {
        MacAddr::V6(mac) => mac.as_bytes().iter().all(|b| *b == 0),
        MacAddr::V8(mac) => mac.as_bytes().iter().all(|b| *b == 0),
    }
}

fn hash_rate_from_ghs(value: Option<f64>) -> Option<HashRate> {
    value.map(|f| {
        HashRate {
            value: f,
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(),
        }
        .as_unit(HashRateUnit::default())
    })
}

#[async_trait]
impl APIClient for ApolloV2 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::GraphQL { .. } => self.graphql.get_api_result(command).await,
            _ => Err(anyhow::anyhow!("Unsupported command type for Apollo API")),
        }
    }
}

impl GetConfigsLocations for ApolloV2 {
    #[allow(unused_variables)]
    fn get_configs_locations(&self, data_field: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}

impl CollectConfigs for ApolloV2 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}

impl GetDataLocations for ApolloV2 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const GQL_IDENTITY: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                Miner {
                    stats {
                        result {
                            stats {
                                version
                                statVersion
                                versions {
                                    miner
                                }
                                master {
                                    hwAddr
                                }
                                slaves {
                                    uid
                                }
                            }
                        }
                        error { message }
                    }
                }
            }"#,
        };
        const GQL_BOARD: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                Miner {
                    stats {
                        result {
                            stats {
                                master {
                                    intervals {
                                        int_30 {
                                            bySol
                                            byDiff
                                        }
                                    }
                                }
                                slots {
                                    int_0 {
                                        chips
                                        pwrOn
                                        temperature
                                        temperature1
                                        ghs
                                    }
                                }
                                slaves {
                                    uid
                                }
                            }
                        }
                        error { message }
                    }
                }
            }"#,
        };
        const GQL_HASHRATE: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                Miner {
                    stats {
                        result {
                            stats {
                                slots {
                                    int_0 {
                                        ghs
                                    }
                                }
                            }
                        }
                        error { message }
                    }
                }
            }"#,
        };
        const GQL_EXPECTED_HASHRATE: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                Miner {
                    stats {
                        result {
                            stats {
                                master {
                                    intervals {
                                        int_30 {
                                            byDiff
                                        }
                                    }
                                }
                            }
                        }
                        error { message }
                    }
                }
            }"#,
        };
        const GQL_FANS: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                Miner {
                    stats {
                        result {
                            stats {
                                fans {
                                    int_0 {
                                        rpm
                                    }
                                }
                            }
                        }
                        error { message }
                    }
                }
            }"#,
        };
        const GQL_POWER: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                Miner {
                    stats {
                        result {
                            stats {
                                master {
                                    boardsW
                                }
                            }
                        }
                        error { message }
                    }
                }
            }"#,
        };
        const GQL_UPTIME: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                Miner {
                    stats {
                        result {
                            stats {
                                master {
                                    upTime
                                }
                            }
                        }
                        error { message }
                    }
                }
            }"#,
        };
        const GQL_MCU_STATS: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                Mcu {
                    stats {
                        result {
                            stats {
                                hostname
                                network {
                                    name
                                    address
                                    mac
                                }
                            }
                        }
                        error { message }
                    }
                }
            }"#,
        };
        const GQL_POOLS: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                Miner {
                    stats {
                        result {
                            stats {
                                pool {
                                    host
                                    port
                                    userName
                                    intervals {
                                        int_0 {
                                            sharesAccepted
                                            sharesRejected
                                            inService
                                        }
                                    }
                                }
                            }
                        }
                        error { message }
                    }
                }
            }"#,
        };
        const GQL_ONLINE: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                Miner {
                    online {
                        result {
                            online {
                                status
                            }
                        }
                        error { message }
                    }
                }
            }"#,
        };

        match data_field {
            DataField::Mac => vec![(
                GQL_MCU_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Mcu/stats/result/stats/network"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                GQL_MCU_STATS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Mcu/stats/result/stats/hostname"),
                    tag: None,
                },
            )],
            DataField::SerialNumber => vec![(
                GQL_IDENTITY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/stats/result/stats/0/slaves/0/uid"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                GQL_IDENTITY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/stats/result/stats/0/versions/miner"),
                    tag: None,
                },
            )],
            DataField::ApiVersion => vec![(
                GQL_IDENTITY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/stats/result/stats/0/statVersion"),
                    tag: None,
                },
            )],
            DataField::ControlBoardVersion => vec![(
                GQL_IDENTITY,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/stats/result/stats/0/version"),
                    tag: None,
                },
            )],
            DataField::Hashboards | DataField::Chips => vec![(
                GQL_BOARD,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/stats/result/stats/0"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                GQL_HASHRATE,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/stats/result/stats/0/slots/int_0/ghs"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                GQL_EXPECTED_HASHRATE,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/stats/result/stats/0/master/intervals/int_30/byDiff"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                GQL_FANS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/stats/result/stats/0/fans/int_0/rpm"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                GQL_POWER,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/stats/result/stats/0/master/boardsW"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                GQL_UPTIME,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/stats/result/stats/0/master/upTime"),
                    tag: None,
                },
            )],
            DataField::IsMining => vec![(
                GQL_ONLINE,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/online/result/online/status"),
                    tag: None,
                },
            )],
            DataField::Pools => vec![(
                GQL_POOLS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/Miner/stats/result/stats/0/pool"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for ApolloV2 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for ApolloV2 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}

impl CollectData for ApolloV2 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

impl GetMAC for ApolloV2 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        let mac_value = data.get(&DataField::Mac)?;

        if let Some(networks) = mac_value.as_array() {
            networks
                .iter()
                .filter_map(|network| network.get("mac").and_then(Value::as_str))
                .filter_map(|s| MacAddr::from_str(s).ok())
                .find(|mac| !is_zero_mac(mac))
        } else {
            mac_value
                .as_str()
                .and_then(|s| MacAddr::from_str(s).ok())
                .filter(|mac| !is_zero_mac(mac))
        }
    }
}

impl GetSerialNumber for ApolloV2 {
    fn parse_serial_number(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::SerialNumber)
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
    }
}

impl GetHostname for ApolloV2 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::Hostname)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
    }
}

impl GetApiVersion for ApolloV2 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::ApiVersion)
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
    }
}

impl GetFirmwareVersion for ApolloV2 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.get(&DataField::FirmwareVersion)
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
    }
}

impl GetControlBoardVersion for ApolloV2 {
    fn parse_control_board_version(
        &self,
        data: &HashMap<DataField, Value>,
    ) -> Option<MinerControlBoard> {
        data.get(&DataField::ControlBoardVersion)
            .and_then(|v| v.as_str())
            .and_then(|s| {
                FutureBitControlBoard::parse(s)
                    .map(Into::into)
                    .or_else(|| Some(MinerControlBoard::unknown(s.to_string())))
            })
    }
}

impl GetHashboards for ApolloV2 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let Some(stats) = data.get(&DataField::Hashboards) else {
            return vec![];
        };
        let slot = stats.pointer("/slots/int_0").unwrap_or(stats);
        let chips = as_u64(slot.get("chips")).map(|u| u as u16);
        let ghs = as_f64(slot.get("ghs"));
        let active = ghs
            .map(|value| value > 0.0)
            .or_else(|| as_u64(slot.get("pwrOn")).map(|u| u > 0));
        let hashrate = hash_rate_from_ghs(
            ghs.or_else(|| as_f64(stats.pointer("/master/intervals/int_30/bySol"))),
        );

        let mut board = BoardData::with_state(0, chips, None, active);
        board.working_chips = chips;
        board.hashrate = hashrate;
        board.expected_hashrate =
            hash_rate_from_ghs(as_f64(stats.pointer("/master/intervals/int_30/byDiff")));
        board.board_temperature = as_f64(slot.get("temperature")).map(Temperature::from_celsius);
        board.inlet_chip_temperature = board.board_temperature;
        board.outlet_chip_temperature =
            as_f64(slot.get("temperature1")).map(Temperature::from_celsius);
        board.serial_number = stats
            .pointer("/slaves/0/uid")
            .and_then(Value::as_str)
            .map(ToString::to_string);

        vec![board]
    }
}

impl GetHashrate for ApolloV2 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        hash_rate_from_ghs(as_f64(data.get(&DataField::Hashrate)))
    }
}

impl GetExpectedHashrate for ApolloV2 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        hash_rate_from_ghs(as_f64(data.get(&DataField::ExpectedHashrate)))
    }
}

impl GetFans for ApolloV2 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let Some(rpm_value) = data.get(&DataField::Fans) else {
            return vec![];
        };

        if let Some(rpms) = rpm_value.as_array() {
            rpms.iter()
                .enumerate()
                .filter_map(|(idx, v)| {
                    Some(FanData {
                        position: i16::try_from(idx).ok()?,
                        rpm: as_f64(Some(v)).map(AngularVelocity::from_rpm),
                    })
                })
                .collect()
        } else {
            as_f64(Some(rpm_value))
                .map(|rpm| {
                    vec![FanData {
                        position: 0,
                        rpm: Some(AngularVelocity::from_rpm(rpm)),
                    }]
                })
                .unwrap_or_default()
        }
    }
}

impl GetPsuFans for ApolloV2 {}

impl GetFluidTemperature for ApolloV2 {}

impl GetWattage for ApolloV2 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        as_f64(data.get(&DataField::Wattage)).map(Power::from_watts)
    }
}

impl GetTuningTarget for ApolloV2 {}

impl GetScaledTuningTarget for ApolloV2 {}
impl GetTuningCapabilities for ApolloV2 {}
impl GetLightFlashing for ApolloV2 {}

impl GetMessages for ApolloV2 {}

impl GetUptime for ApolloV2 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        as_u64(data.get(&DataField::Uptime)).map(Duration::from_secs)
    }
}

impl GetIsMining for ApolloV2 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        let Some(status) = data.get(&DataField::IsMining) else {
            return false;
        };

        status.as_bool().unwrap_or_else(|| {
            status.as_str().is_some_and(|s| {
                s.eq_ignore_ascii_case("online")
                    || s.eq_ignore_ascii_case("pending")
                    || s.eq_ignore_ascii_case("true")
            })
        })
    }
}

impl GetPools for ApolloV2 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let Some(pool) = data.get(&DataField::Pools) else {
            return vec![];
        };

        let Some(host) = pool.get("host").and_then(|v| v.as_str()) else {
            return vec![];
        };

        let port = as_u64(pool.get("port")).unwrap_or(0) as u16;
        let accepted = as_u64(pool.pointer("/intervals/int_0/sharesAccepted"));
        let rejected = as_u64(pool.pointer("/intervals/int_0/sharesRejected"));
        let active = as_u64(pool.pointer("/intervals/int_0/inService")).map(|u| u > 0);

        vec![PoolGroupData {
            name: String::new(),
            quota: 1,
            pools: vec![PoolData {
                position: Some(0),
                url: Some(PoolURL::from(format!("{host}:{port}"))),
                accepted_shares: accepted,
                rejected_shares: rejected,
                active,
                alive: active,
                user: pool
                    .get("userName")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string),
            }],
        }]
    }
}

#[async_trait]
impl SetFaultLight for ApolloV2 {
    fn supports_set_fault_light(&self) -> bool {
        false
    }
}

#[async_trait]
impl SetPowerLimit for ApolloV2 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsPoolsConfig for ApolloV2 {
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

#[async_trait]
impl Restart for ApolloV2 {
    async fn restart(&self) -> anyhow::Result<bool> {
        let data = self
            .graphql
            .send_graphql_command("mutation { Miner { restart { error { message } } } }", None)
            .await?;
        if let Some(message) = data
            .pointer("/Miner/restart/error/message")
            .and_then(|v| v.as_str())
        {
            anyhow::bail!("Apollo restart failed: {message}");
        }
        Ok(true)
    }

    fn supports_restart(&self) -> bool {
        false
    }
}

#[async_trait]
impl Pause for ApolloV2 {
    async fn pause(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        let data = self
            .graphql
            .send_graphql_command("mutation { Miner { stop { error { message } } } }", None)
            .await?;
        if let Some(message) = data
            .pointer("/Miner/stop/error/message")
            .and_then(|v| v.as_str())
        {
            anyhow::bail!("Apollo stop failed: {message}");
        }
        Ok(true)
    }

    fn supports_pause(&self) -> bool {
        false
    }
}

#[async_trait]
impl Resume for ApolloV2 {
    async fn resume(&self, _at_time: Option<Duration>) -> anyhow::Result<bool> {
        let data = self
            .graphql
            .send_graphql_command("mutation { Miner { start { error { message } } } }", None)
            .await?;
        if let Some(message) = data
            .pointer("/Miner/start/error/message")
            .and_then(|v| v.as_str())
        {
            anyhow::bail!("Apollo start failed: {message}");
        }
        Ok(true)
    }

    fn supports_resume(&self) -> bool {
        false
    }
}

impl ChangePassword for ApolloV2 {
    fn supports_change_password(&self) -> bool {
        false
    }
}

impl ReadLogs for ApolloV2 {
    fn supports_read_logs(&self) -> bool {
        false
    }
}

impl FactoryReset for ApolloV2 {
    fn supports_factory_reset(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsScalingConfig for ApolloV2 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}

#[async_trait]
impl SupportsTuningConfig for ApolloV2 {}

#[async_trait]
impl SupportsFanConfig for ApolloV2 {}

#[async_trait]
impl UpgradeFirmware for ApolloV2 {
    fn supports_upgrade_firmware(&self) -> bool {
        false
    }
}

impl HasAuth for ApolloV2 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.graphql.set_auth(auth);
    }
}

impl HasDefaultAuth for ApolloV2 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("futurebit", "futurebit123")
    }
}

impl SupportsTemperatureConfig for ApolloV2 {}
impl GetTuningPercent for ApolloV2 {}
impl SetTuningPercent for ApolloV2 {}

#[cfg(test)]
mod tests {
    use super::*;
    use asic_rs_core::data::collector::DataField;
    use asic_rs_makes_futurebit::models::FutureBitModel;
    use serde_json::json;

    fn miner() -> ApolloV2 {
        ApolloV2::new(IpAddr::from([127, 0, 0, 1]), FutureBitModel::Apollo2)
    }

    #[test]
    fn parse_hashboard_uses_slot_ghs_zero_over_stale_interval() {
        let miner = miner();
        let mut data = HashMap::new();
        data.insert(
            DataField::Hashboards,
            json!({
                "master": {
                    "intervals": {
                        "int_30": {
                            "bySol": 7060.6,
                            "byDiff": 0
                        }
                    }
                },
                "slots": {
                    "int_0": {
                        "chips": 44,
                        "pwrOn": 1,
                        "temperature": 68,
                        "temperature1": 0,
                        "ghs": 0
                    }
                },
                "slaves": [
                    {"uid": "590055001351323532343337"}
                ]
            }),
        );

        let boards = miner.parse_hashboards(&data);

        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].hashrate.as_ref().map(|hr| hr.value), Some(0.0));
        assert_eq!(boards[0].active, Some(false));
        assert_eq!(
            boards[0].serial_number,
            Some("590055001351323532343337".to_string())
        );
    }

    #[test]
    fn parse_is_mining_accepts_boolean_status() {
        let miner = miner();
        let mut data = HashMap::new();

        data.insert(DataField::IsMining, Value::Bool(false));
        assert!(!miner.parse_is_mining(&data));

        data.insert(DataField::IsMining, Value::Bool(true));
        assert!(miner.parse_is_mining(&data));
    }

    #[test]
    fn parse_mcu_network_mac_ignores_zero_mac() {
        let miner = miner();
        let mut data = HashMap::new();
        data.insert(
            DataField::Mac,
            json!([
                {"name": "miner", "mac": "00:00:00:00:00:00"},
                {"name": "eth0", "mac": "16:a1:04:59:2d:7d"}
            ]),
        );

        assert_eq!(
            miner.parse_mac(&data),
            MacAddr::from_str("16:a1:04:59:2d:7d").ok()
        );
    }

    #[test]
    fn parse_hostname_and_unknown_control_board() {
        let miner = miner();
        let mut data = HashMap::new();
        data.insert(
            DataField::Hostname,
            Value::String("futurebit-apollo-2".to_string()),
        );
        data.insert(
            DataField::ControlBoardVersion,
            Value::String("v2".to_string()),
        );

        assert_eq!(
            miner.parse_hostname(&data),
            Some("futurebit-apollo-2".to_string())
        );
        assert_eq!(
            miner.parse_control_board_version(&data),
            Some(MinerControlBoard::unknown("v2".to_string()))
        );
    }
}

impl SupportsPresets for ApolloV2 {}

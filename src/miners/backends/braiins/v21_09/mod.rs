use crate::data::board::BoardData;
use crate::data::device::{DeviceInfo, HashAlgorithm, MinerFirmware, MinerMake, MinerModel};
use crate::data::fan::FanData;
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::message::{MessageSeverity, MinerMessage};
use crate::data::pool::{PoolData, PoolGroupData, PoolURL};
use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;
use crate::miners::data::{
    DataCollector, DataExtensions, DataExtractor, DataField, DataLocation, get_by_pointer,
};
use async_trait::async_trait;
use graphql::BraiinsGraphQLAPI;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use rpc::BraiinsRPCAPI;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;
use web::BraiinsWebAPI;

mod graphql;
mod rpc;
mod web;

#[derive(Debug)]
pub struct BraiinsV2109 {
    pub ip: IpAddr,
    pub rpc: BraiinsRPCAPI,
    pub graphql: BraiinsGraphQLAPI,
    pub web: BraiinsWebAPI,
    pub device_info: DeviceInfo,
}

impl BraiinsV2109 {
    pub fn new(ip: IpAddr, model: MinerModel) -> Self {
        BraiinsV2109 {
            ip,
            rpc: BraiinsRPCAPI::new(ip),
            graphql: BraiinsGraphQLAPI::new(ip),
            web: BraiinsWebAPI::new(ip),
            device_info: DeviceInfo::new(
                MinerMake::from(model),
                model,
                MinerFirmware::BraiinsOS,
                HashAlgorithm::SHA256,
            ),
        }
    }
}

#[async_trait]
impl APIClient for BraiinsV2109 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC { .. } => self.rpc.get_api_result(command).await,
            MinerCommand::GraphQL { .. } => self.graphql.get_api_result(command).await,
            MinerCommand::WebAPI { .. } => self.web.get_api_result(command).await,
            _ => Err(anyhow::anyhow!("Unsupported command type for Braiins API")),
        }
    }
}

impl GetDataLocations for BraiinsV2109 {
    fn get_locations(&self, data_field: DataField) -> Vec<DataLocation> {
        const GQL_SYSTEM: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                bos {
                    hostname
                    faultLight
                    info { version { full } }
                    uptime { durationS }
                }
                bosminer {
                    info {
                        workSolver {
                            realHashrate { mhs5S }
                            nominalMhs
                        }
                        fans { name speed rpm }
                        summary {
                            power { limitW approxConsumptionW }
                        }
                    }
                }
            }"#,
        };
        const GQL_BOARDS: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                bosminer {
                    info {
                        workSolver {
                            childSolvers {
                                name
                                realHashrate { mhs5S }
                                nominalMhs
                                hwDetails { chips frequencyMhz voltageV }
                                temperatures { name degreesC }
                            }
                        }
                    }
                }
            }"#,
        };
        const GQL_POOLS: MinerCommand = MinerCommand::GraphQL {
            command: r#"{
                bosminer {
                    config {
                        ... on BosminerConfig {
                            groups {
                                id
                                strategy {
                                    ... on QuotaStrategy {
                                        quota
                                    }
                                }
                            }
                        }
                    }
                    info {
                        poolGroups {
                            name
                            pools {
                                url
                                user
                                status
                                active
                                shares { acceptedSolutions rejectedSolutions }
                            }
                        }
                    }
                }
            }"#,
        };
        const RPC_VERSION: MinerCommand = MinerCommand::RPC {
            command: "version",
            parameters: None,
        };
        const WEB_NET_CONF: MinerCommand = MinerCommand::WebAPI {
            command: "admin/network/iface_status/lan",
            parameters: None,
        };

        match data_field {
            DataField::ApiVersion => vec![(
                RPC_VERSION,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/VERSION/0/API"),
                    tag: None,
                },
            )],
            DataField::FirmwareVersion => vec![(
                GQL_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bos/info/version/full"),
                    tag: None,
                },
            )],
            DataField::Hostname => vec![(
                GQL_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bos/hostname"),
                    tag: None,
                },
            )],
            DataField::Mac => vec![(
                WEB_NET_CONF,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/0/macaddr"),
                    tag: None,
                },
            )],
            DataField::LightFlashing => vec![(
                GQL_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bos/faultLight"),
                    tag: None,
                },
            )],
            DataField::Uptime => vec![(
                GQL_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bos/uptime/durationS"),
                    tag: None,
                },
            )],
            DataField::Hashrate => vec![(
                GQL_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bosminer/info/workSolver/realHashrate/mhs5S"),
                    tag: None,
                },
            )],
            DataField::ExpectedHashrate => vec![(
                GQL_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bosminer/info/workSolver/nominalMhs"),
                    tag: None,
                },
            )],
            DataField::Hashboards => vec![(
                GQL_BOARDS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bosminer/info/workSolver/childSolvers"),
                    tag: None,
                },
            )],
            DataField::Fans => vec![(
                GQL_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bosminer/info/fans"),
                    tag: None,
                },
            )],
            DataField::Pools => vec![(
                GQL_POOLS,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bosminer"),
                    tag: None,
                },
            )],
            DataField::Wattage => vec![(
                GQL_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bosminer/info/summary/power/approxConsumptionW"),
                    tag: None,
                },
            )],
            DataField::WattageLimit => vec![(
                GQL_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bosminer/info/summary/power/limitW"),
                    tag: None,
                },
            )],
            DataField::IsMining => vec![(
                GQL_SYSTEM,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/bosminer/info/workSolver"),
                    tag: None,
                },
            )],
            DataField::Messages => vec![(
                MinerCommand::GraphQL {
                    command: "{ events { appeals { kind timestamp message } } }",
                },
                DataExtractor {
                    func: get_by_pointer,
                    key: Some("/events/appeals"),
                    tag: None,
                },
            )],
            _ => vec![],
        }
    }
}

impl GetIP for BraiinsV2109 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}

impl GetDeviceInfo for BraiinsV2109 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info
    }
}

impl CollectData for BraiinsV2109 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}

#[async_trait]
impl GetMAC for BraiinsV2109 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        data.extract::<String>(DataField::Mac)
            .and_then(|s| MacAddr::from_str(&s).ok())
    }
}

impl GetHostname for BraiinsV2109 {
    fn parse_hostname(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::Hostname)
    }
}

impl GetApiVersion for BraiinsV2109 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::ApiVersion)
    }
}

impl GetFirmwareVersion for BraiinsV2109 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        data.extract::<String>(DataField::FirmwareVersion)
    }
}

impl GetControlBoardVersion for BraiinsV2109 {}

impl GetHashboards for BraiinsV2109 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let mut hashboards: Vec<BoardData> = Vec::new();

        let solvers = data.get(&DataField::Hashboards).and_then(|v| v.as_array());

        if let Some(solvers_array) = solvers {
            for (idx, solver) in solvers_array.iter().enumerate() {
                let hashrate = solver
                    .pointer("/realHashrate/mhs5S")
                    .and_then(|v| v.as_f64())
                    .map(|f| HashRate {
                        value: f,
                        unit: HashRateUnit::MegaHash,
                        algo: String::from("SHA256"),
                    });

                let expected_hashrate =
                    solver
                        .get("nominalMhs")
                        .and_then(|v| v.as_f64())
                        .map(|f| HashRate {
                            value: f,
                            unit: HashRateUnit::MegaHash,
                            algo: String::from("SHA256"),
                        });

                let active = hashrate.as_ref().map(|hr| hr.value > 0.0);

                let working_chips = solver
                    .pointer("/hwDetails/chips")
                    .and_then(|v| v.as_u64())
                    .map(|u| u as u16);

                let frequency = solver
                    .pointer("/hwDetails/frequencyMhz")
                    .and_then(|v| v.as_f64())
                    .map(Frequency::from_megahertz);

                let voltage = solver
                    .pointer("/hwDetails/voltageV")
                    .and_then(|v| v.as_f64())
                    .map(Voltage::from_volts);

                // Temperatures are in a list with name/degreesC pairs
                let temps = solver.get("temperatures").and_then(|v| v.as_array());
                let mut board_temperature = None;
                let mut chip_temperature = None;
                if let Some(temps_array) = temps {
                    for temp in temps_array {
                        let name = temp.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let degrees = temp.get("degreesC").and_then(|v| v.as_f64());
                        if name.contains("Board") {
                            board_temperature = degrees.map(Temperature::from_celsius);
                        } else if name.contains("Chip") {
                            chip_temperature = degrees.map(Temperature::from_celsius);
                        }
                    }
                }

                hashboards.push(BoardData {
                    position: idx as u8,
                    hashrate,
                    expected_hashrate,
                    board_temperature,
                    intake_temperature: chip_temperature,
                    outlet_temperature: chip_temperature,
                    expected_chips: self.device_info.hardware.chips,
                    working_chips,
                    serial_number: None,
                    chips: Vec::new(),
                    voltage,
                    frequency,
                    tuned: None,
                    active,
                });
            }
        }

        hashboards
    }
}

impl GetHashrate for BraiinsV2109 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::Hashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::MegaHash,
            algo: String::from("SHA256"),
        })
    }
}

impl GetExpectedHashrate for BraiinsV2109 {
    fn parse_expected_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        data.extract_map::<f64, _>(DataField::ExpectedHashrate, |f| HashRate {
            value: f,
            unit: HashRateUnit::MegaHash,
            algo: String::from("SHA256"),
        })
    }
}

impl GetFans for BraiinsV2109 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans: Vec<FanData> = Vec::new();

        if let Some(fans_data) = data.get(&DataField::Fans)
            && let Some(fans_array) = fans_data.as_array()
        {
            for idx in 0..self.device_info.hardware.fans.unwrap_or(0) {
                if let Some(fan) = fans_array.get(idx as usize)
                    && let Some(rpm) = fan.get("rpm").and_then(|v| v.as_i64())
                {
                    fans.push(FanData {
                        position: idx as i16,
                        rpm: Some(AngularVelocity::from_rpm(rpm as f64)),
                    });
                }
            }
        }

        fans
    }
}

impl GetPsuFans for BraiinsV2109 {}

impl GetFluidTemperature for BraiinsV2109 {}

impl GetWattage for BraiinsV2109 {
    fn parse_wattage(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::Wattage, Power::from_watts)
    }
}

impl GetWattageLimit for BraiinsV2109 {
    fn parse_wattage_limit(&self, data: &HashMap<DataField, Value>) -> Option<Power> {
        data.extract_map::<f64, _>(DataField::WattageLimit, Power::from_watts)
    }
}

impl GetLightFlashing for BraiinsV2109 {
    fn parse_light_flashing(&self, data: &HashMap<DataField, Value>) -> Option<bool> {
        data.extract::<bool>(DataField::LightFlashing)
    }
}

impl GetMessages for BraiinsV2109 {
    fn parse_messages(&self, data: &HashMap<DataField, Value>) -> Vec<MinerMessage> {
        let mut messages: Vec<MinerMessage> = Vec::new();

        if let Some(appeals_data) = data.get(&DataField::Messages)
            && let Some(appeals_array) = appeals_data.as_array()
        {
            for appeal in appeals_array {
                let timestamp = appeal
                    .get("timestamp")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;

                let message = appeal
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let severity = match appeal.get("kind").and_then(|v| v.as_str()) {
                    Some(k) if k.eq_ignore_ascii_case("error") => MessageSeverity::Error,
                    Some(k) if k.eq_ignore_ascii_case("warning") => MessageSeverity::Warning,
                    _ => MessageSeverity::Info,
                };

                messages.push(MinerMessage::new(timestamp, 0, message, severity));
            }
        }

        messages
    }
}

impl GetUptime for BraiinsV2109 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        data.extract_map::<u64, _>(DataField::Uptime, Duration::from_secs)
    }
}

impl GetIsMining for BraiinsV2109 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        data.get(&DataField::IsMining).is_some_and(|v| !v.is_null())
    }
}

impl GetPools for BraiinsV2109 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let mut pools: Vec<PoolGroupData> = Vec::new();

        if let Some(groups_data) = data.get(&DataField::Pools)
            && let Some(groups_array) = groups_data
                .pointer("/info/poolGroups")
                .and_then(|v| v.as_array())
        {
            let config_groups = groups_data
                .pointer("/config/groups")
                .and_then(|v| v.as_array());

            let mut idx = 0u16;
            for (group_idx, group) in groups_array.iter().enumerate() {
                let mut group_pools: Vec<PoolData> = Vec::new();
                if let Some(pools_array) = group.get("pools").and_then(|v| v.as_array()) {
                    for pool in pools_array {
                        let url = pool
                            .get("url")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .map(PoolURL::from);

                        let user = pool.get("user").and_then(|v| v.as_str()).map(String::from);

                        let accepted_shares = pool
                            .pointer("/shares/acceptedSolutions")
                            .and_then(|v| v.as_u64());
                        let rejected_shares = pool
                            .pointer("/shares/rejectedSolutions")
                            .and_then(|v| v.as_u64());

                        let active = pool.get("active").and_then(|v| v.as_bool());
                        let alive = pool
                            .get("status")
                            .and_then(|v| v.as_str())
                            .map(|s| s == "Running" || s == "Active");

                        group_pools.push(PoolData {
                            position: Some(idx),
                            url,
                            accepted_shares,
                            rejected_shares,
                            active,
                            alive,
                            user,
                        });
                        idx += 1;
                    }
                }

                let quota = config_groups
                    .and_then(|cg| cg.get(group_idx))
                    .and_then(|cg| cg.pointer("/strategy/quota"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as u32;

                pools.push(PoolGroupData {
                    name: group
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    quota,
                    pools: group_pools,
                });
            }
        }

        pools
    }
}

impl GetSerialNumber for BraiinsV2109 {}

#[async_trait]
impl SetFaultLight for BraiinsV2109 {
    async fn set_fault_light(&self, fault: bool) -> anyhow::Result<bool> {
        let mutation = r#"mutation ($enable: Boolean!) {
            bos {
                setFaultLight(
                    enable: $enable
                ) {
                    enabled
                }
            }
        }"#;
        let variables = json!({ "enable": fault });
        Ok(self
            .graphql
            .send_command(mutation, true, Some(variables))
            .await
            .is_ok())
    }
    fn supports_set_fault_light(&self) -> bool {
        true
    }
}

#[async_trait]
impl SetPowerLimit for BraiinsV2109 {
    async fn set_power_limit(&self, limit: Power) -> anyhow::Result<bool> {
        let mutation = r#"mutation ($limit: Int!) {
            bosminer {
                config {
                    updateAutotuning(
                        input: {
                            powerTarget: $limit
                        },
                        apply: true
                    ) {
                        ... on AutotuningOut {
                            autotuning {
                                powerTarget,
                                enabled
                            }
                        }
                    }
                }
            }
        }"#;
        let variables = json!({ "limit": limit.as_watts() as u64 });
        Ok(self
            .graphql
            .send_command(mutation, true, Some(variables))
            .await
            .is_ok())
    }
    fn supports_set_power_limit(&self) -> bool {
        true
    }
}

#[async_trait]
impl Restart for BraiinsV2109 {
    async fn restart(&self) -> anyhow::Result<bool> {
        let mutation = r#"mutation {
            bos {
                reboot {
                    ... on VoidResult {
                        void
                    }
                }
            }
        }"#;
        Ok(self
            .graphql
            .send_command(mutation, true, None)
            .await
            .is_ok())
    }
    fn supports_restart(&self) -> bool {
        true
    }
}

#[async_trait]
impl Pause for BraiinsV2109 {
    #[allow(unused_variables)]
    async fn pause(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        let mutation = r#"mutation {
            bosminer {
                stop {
                    ... on VoidResult {
                        void
                    }
                }
            }
        }"#;
        Ok(self
            .graphql
            .send_command(mutation, true, None)
            .await
            .is_ok())
    }
    fn supports_pause(&self) -> bool {
        true
    }
}

#[async_trait]
impl Resume for BraiinsV2109 {
    #[allow(unused_variables)]
    async fn resume(&self, at_time: Option<Duration>) -> anyhow::Result<bool> {
        let mutation = r#"mutation {
            bosminer {
                start {
                    ... on VoidResult {
                        void
                    }
                }
            }
        }"#;
        Ok(self
            .graphql
            .send_command(mutation, true, None)
            .await
            .is_ok())
    }
    fn supports_resume(&self) -> bool {
        true
    }
}

impl SetPools for BraiinsV2109 {
    fn supports_set_pools(&self) -> bool {
        false
    }
}

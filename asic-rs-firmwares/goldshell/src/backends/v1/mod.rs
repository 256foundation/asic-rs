pub mod web;

use crate::firmware::GoldshellFirmware;
use asic_rs_core::{
    config::collector::{ConfigCollector, ConfigField, ConfigLocation},
    data::{
        board::BoardData,
        collector::{DataCollector, DataExtractor, DataField, DataLocation, get_by_pointer},
        command::MinerCommand,
        device::{DeviceInfo, HashAlgorithm},
        fan::FanData,
        hashrate::{HashRate, HashRateUnit},
        pool::{PoolData, PoolGroupData, PoolURL},
    },
    traits::{miner::*, model::MinerModel},
    util::send_rpc_command,
};
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Temperature};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    net::IpAddr,
    str::FromStr,
    time::Duration,
};
use web::GoldshellWebAPI;

#[derive(Debug)]
pub struct GoldshellV1 {
    ip: IpAddr,
    web: GoldshellWebAPI,
    device_info: DeviceInfo,
}
impl GoldshellV1 {
    pub fn new(ip: IpAddr, model: impl MinerModel) -> Self {
        let algo = model.hash_algorithm();
        Self {
            ip,
            web: GoldshellWebAPI::new(ip, Self::default_auth()),
            device_info: DeviceInfo::new(model, GoldshellFirmware, algo),
        }
    }
    pub fn allows_rpc(command: &str) -> bool {
        matches!(
            command,
            "version" | "summary" | "stats" | "devs" | "devdetails" | "pools"
        )
    }
}

fn number(value: Option<&Value>) -> Option<f64> {
    value
        .and_then(|v| v.as_f64().or_else(|| v.as_str()?.parse().ok()))
        .filter(|v| v.is_finite() && *v >= 0.0)
}
fn integer(value: Option<&Value>) -> Option<u64> {
    value.and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
}
fn temperature(value: Option<&Value>) -> Option<f64> {
    number(value).filter(|value| *value > 0.0 && *value <= 200.0)
}
fn string(data: &HashMap<DataField, Value>, field: DataField) -> Option<String> {
    data.get(&field)?.as_str().map(str::to_string)
}
fn details<'a>(data: &'a Value, _: Option<&str>) -> Option<&'a Value> {
    data.get("DEVS").or_else(|| data.get("DEVDETAILS"))
}
fn device_fans<'a>(data: &'a Value, _: Option<&str>) -> Option<&'a Value> {
    let devices = data.get("DEVS")?;
    devices
        .as_array()?
        .iter()
        .any(|row| {
            row.as_object().is_some_and(|row| {
                row.iter().any(|(key, value)| {
                    key.strip_prefix("fan")
                        .and_then(|suffix| suffix.parse::<u16>().ok())
                        .is_some()
                        && number(Some(value)).is_some()
                })
            })
        })
        .then_some(devices)
}
fn location(
    command: MinerCommand,
    pointer: &'static str,
    tag: Option<&'static str>,
) -> DataLocation {
    (
        command,
        DataExtractor {
            func: get_by_pointer,
            key: Some(pointer),
            tag,
        },
    )
}
fn rpc(command: &'static str) -> MinerCommand {
    MinerCommand::RPC {
        command,
        parameters: None,
    }
}
fn web(command: &'static str) -> MinerCommand {
    MinerCommand::WebAPI {
        command,
        parameters: None,
    }
}
fn declared_rate(row: &Value, algo: HashAlgorithm) -> Option<HashRate> {
    let selected = ["MHS 20s", "MHS 5s", "MHS 1m", "MHS av"]
        .into_iter()
        .find_map(|key| row.get(key))?;
    let value = number(Some(selected))?;
    let rate = HashRate {
        value,
        unit: HashRateUnit::MegaHash,
        algo,
    };
    Some(if algo == HashAlgorithm::Unknown {
        rate
    } else {
        rate.as_default_unit()
    })
}

#[async_trait]
impl APIClient for GoldshellV1 {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC {
                command,
                parameters: None,
            } if Self::allows_rpc(command) => send_rpc_command(&self.ip, command)
                .await
                .ok_or_else(|| anyhow::anyhow!("Goldshell read RPC failed")),
            MinerCommand::WebAPI {
                command,
                parameters: None,
            } if GoldshellWebAPI::allows_read(command) => self.web.read(command).await,
            _ => anyhow::bail!(
                "Goldshell telemetry backend accepts read-only commands without parameters"
            ),
        }
    }
}
impl GetDataLocations for GoldshellV1 {
    fn get_locations(&self, field: DataField) -> Vec<DataLocation> {
        match field {
            DataField::Mac => vec![location(web("setting"), "/name", None)],
            DataField::ApiVersion => vec![location(rpc("version"), "/VERSION/0/API", None)],
            DataField::FirmwareVersion => vec![location(web("status"), "/firmware", None)],
            DataField::Hashrate | DataField::IsMining => {
                vec![location(rpc("summary"), "/SUMMARY/0", None)]
            }
            DataField::Uptime => vec![location(rpc("summary"), "/SUMMARY/0/Elapsed", None)],
            DataField::Hashboards => vec![
                location(rpc("devs"), "/DEVS", Some("devices")),
                (
                    rpc("devdetails"),
                    DataExtractor {
                        func: details,
                        key: None,
                        tag: Some("details"),
                    },
                ),
            ],
            // Live SC5Pro/ARI31 report RPM in DEVS while their STATS reply is
            // not valid JSON. Keep the older STATS route as a fallback.
            DataField::Fans => vec![
                (
                    rpc("devs"),
                    DataExtractor {
                        func: device_fans,
                        key: None,
                        tag: None,
                    },
                ),
                location(rpc("stats"), "/STATS", None),
            ],
            DataField::Pools => vec![location(rpc("pools"), "/POOLS", None)],
            _ => vec![],
        }
    }
}
impl GetConfigsLocations for GoldshellV1 {
    fn get_configs_locations(&self, _: ConfigField) -> Vec<ConfigLocation> {
        vec![]
    }
}
impl CollectConfigs for GoldshellV1 {
    fn get_config_collector(&self) -> ConfigCollector<'_> {
        ConfigCollector::new(self)
    }
}
impl CollectData for GoldshellV1 {
    fn get_collector(&self) -> DataCollector<'_> {
        DataCollector::new(self)
    }
}
impl GetIP for GoldshellV1 {
    fn get_ip(&self) -> IpAddr {
        self.ip
    }
}
impl GetDeviceInfo for GoldshellV1 {
    fn get_device_info(&self) -> DeviceInfo {
        self.device_info.clone()
    }
}
impl GetMAC for GoldshellV1 {
    fn parse_mac(&self, data: &HashMap<DataField, Value>) -> Option<MacAddr> {
        MacAddr::from_str(data.get(&DataField::Mac)?.as_str()?).ok()
    }
}
impl GetApiVersion for GoldshellV1 {
    fn parse_api_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        string(data, DataField::ApiVersion)
    }
}
impl GetFirmwareVersion for GoldshellV1 {
    fn parse_firmware_version(&self, data: &HashMap<DataField, Value>) -> Option<String> {
        string(data, DataField::FirmwareVersion)
    }
}
impl GetHashrate for GoldshellV1 {
    fn parse_hashrate(&self, data: &HashMap<DataField, Value>) -> Option<HashRate> {
        declared_rate(data.get(&DataField::Hashrate)?, self.device_info.algo)
    }
}
impl GetIsMining for GoldshellV1 {
    fn parse_is_mining(&self, data: &HashMap<DataField, Value>) -> bool {
        data.get(&DataField::IsMining)
            .and_then(|r| {
                ["MHS 20s", "MHS 5s", "MHS 1m"]
                    .into_iter()
                    .find_map(|key| r.get(key))
                    .and_then(|value| number(Some(value)))
            })
            .is_some_and(|rate| rate > 0.0)
    }
}
impl GetUptime for GoldshellV1 {
    fn parse_uptime(&self, data: &HashMap<DataField, Value>) -> Option<Duration> {
        integer(data.get(&DataField::Uptime)).map(Duration::from_secs)
    }
}
impl GetHashboards for GoldshellV1 {
    fn parse_hashboards(&self, data: &HashMap<DataField, Value>) -> Vec<BoardData> {
        let Some(raw) = data.get(&DataField::Hashboards) else {
            return vec![];
        };
        let mut boards: BTreeMap<u8, BoardData> = BTreeMap::new();
        for row in raw
            .get("devices")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(position) = integer(row.get("ID")).and_then(|p| u8::try_from(p).ok()) else {
                continue;
            };
            let board = boards
                .entry(position)
                .or_insert_with(|| BoardData::new(position, None));
            board.hashrate = declared_rate(row, self.device_info.algo);
            board.board_temperature =
                temperature(row.get("tstemp-2")).map(Temperature::from_celsius);
            let chip_temperatures = [
                temperature(row.get("tstemp-0")),
                temperature(row.get("tstemp-1")),
            ];
            // Retain coolest/hottest reported chip sensor channels; ordering
            // does not establish a physical inlet/outlet sensor location.
            board.inlet_chip_temperature = chip_temperatures
                .iter()
                .flatten()
                .copied()
                .reduce(f64::min)
                .map(Temperature::from_celsius);
            board.outlet_chip_temperature = chip_temperatures
                .iter()
                .flatten()
                .copied()
                .reduce(f64::max)
                .map(Temperature::from_celsius);
            board.active = board.hashrate.as_ref().map(|r| r.value > 0.0);
        }
        for row in raw
            .get("details")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(position) = integer(row.get("ID")).and_then(|p| u8::try_from(p).ok()) else {
                continue;
            };
            let board = boards
                .entry(position)
                .or_insert_with(|| BoardData::new(position, None));
            board.working_chips = integer(row.get("chips-nr")).and_then(|v| u16::try_from(v).ok());
        }
        boards.into_values().collect()
    }
}
impl GetFans for GoldshellV1 {
    fn parse_fans(&self, data: &HashMap<DataField, Value>) -> Vec<FanData> {
        let mut fans: BTreeMap<i16, FanData> = BTreeMap::new();
        for row in data
            .get(&DataField::Fans)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            for (key, value) in row.as_object().into_iter().flatten() {
                let Some(position) = key
                    .strip_prefix("fan")
                    .and_then(|p| p.parse::<i16>().ok())
                    .filter(|p| *p >= 0)
                else {
                    continue;
                };
                if let Some(rpm) = number(Some(value)) {
                    // Firmware repeats global fan indices in each DEVS row.
                    // Preserve the slowest observed value when snapshots differ
                    // instead of silently choosing the final board's row.
                    fans.entry(position)
                        .and_modify(|fan| {
                            let observed = fan.rpm.map(|value| value.as_rpm()).unwrap_or(rpm);
                            fan.rpm = Some(AngularVelocity::from_rpm(observed.min(rpm)));
                        })
                        .or_insert(FanData {
                            position,
                            rpm: Some(AngularVelocity::from_rpm(rpm)),
                        });
                }
            }
        }
        fans.into_values().collect()
    }
}
impl GetPools for GoldshellV1 {
    fn parse_pools(&self, data: &HashMap<DataField, Value>) -> Vec<PoolGroupData> {
        let Some(rows) = data.get(&DataField::Pools).and_then(Value::as_array) else {
            return vec![];
        };
        let pools = rows
            .iter()
            .map(|row| PoolData {
                position: integer(row.get("POOL")).and_then(|v| u16::try_from(v).ok()),
                url: row
                    .get("URL")
                    .and_then(Value::as_str)
                    .map(|s| PoolURL::from(s.to_string())),
                user: row.get("User").and_then(Value::as_str).map(str::to_string),
                accepted_shares: integer(row.get("Accepted")),
                rejected_shares: integer(row.get("Rejected")),
                active: row.get("Stratum Active").and_then(Value::as_bool),
                alive: row
                    .get("Status")
                    .and_then(Value::as_str)
                    .map(|s| s.eq_ignore_ascii_case("Alive")),
                last_share_time: None,
            })
            .collect();
        vec![PoolGroupData {
            name: String::new(),
            quota: 1,
            pools,
        }]
    }
}

impl GetSerialNumber for GoldshellV1 {}
impl GetHostname for GoldshellV1 {}
impl GetControlBoardVersion for GoldshellV1 {}
impl GetExpectedHashrate for GoldshellV1 {}
impl GetPsuFans for GoldshellV1 {}
impl GetFluidTemperature for GoldshellV1 {}
impl GetWattage for GoldshellV1 {}
impl GetTuningPercent for GoldshellV1 {}
impl GetTuningTarget for GoldshellV1 {}
impl GetScaledTuningTarget for GoldshellV1 {}
impl GetTuningCapabilities for GoldshellV1 {}
impl GetLightFlashing for GoldshellV1 {}
impl GetMessages for GoldshellV1 {}
impl GetBestShare for GoldshellV1 {}
impl GetSessionBestShare for GoldshellV1 {}
impl GetOperatingState for GoldshellV1 {}
impl GetDevFeeConnected for GoldshellV1 {}
impl SetFaultLight for GoldshellV1 {
    fn supports_set_fault_light(&self) -> bool {
        false
    }
}
impl SetPowerLimit for GoldshellV1 {
    fn supports_set_power_limit(&self) -> bool {
        false
    }
}
impl Restart for GoldshellV1 {
    fn supports_restart(&self) -> bool {
        false
    }
}
impl Pause for GoldshellV1 {
    fn supports_pause(&self) -> bool {
        false
    }
}
impl Resume for GoldshellV1 {
    fn supports_resume(&self) -> bool {
        false
    }
}
impl ChangePassword for GoldshellV1 {
    fn supports_change_password(&self) -> bool {
        false
    }
}
impl FactoryReset for GoldshellV1 {
    fn supports_factory_reset(&self) -> bool {
        false
    }
}
impl ReadLogs for GoldshellV1 {
    fn supports_read_logs(&self) -> bool {
        false
    }
}
impl UpgradeFirmware for GoldshellV1 {
    fn supports_upgrade_firmware(&self) -> bool {
        false
    }
}
impl RestoreStockOs for GoldshellV1 {}
impl SetHashboardsEnabled for GoldshellV1 {}
impl SetTuningPercent for GoldshellV1 {}
impl SupportsPresets for GoldshellV1 {}
impl SupportsPoolsConfig for GoldshellV1 {
    fn supports_pools_config(&self) -> bool {
        false
    }
}
impl SupportsScalingConfig for GoldshellV1 {
    fn supports_scaling_config(&self) -> bool {
        false
    }
}
impl SupportsTemperatureConfig for GoldshellV1 {}
impl SupportsTimezoneConfig for GoldshellV1 {}
impl SupportsTuningConfig for GoldshellV1 {}
impl SupportsFanConfig for GoldshellV1 {}
impl HasAuth for GoldshellV1 {
    fn set_auth(&mut self, auth: MinerAuth) {
        self.web.set_auth(auth);
    }
}
impl HasDefaultAuth for GoldshellV1 {
    fn default_auth() -> MinerAuth {
        MinerAuth::new("admin", "123456789")
    }
}
#[async_trait]
impl Validate for GoldshellV1 {
    type Firmware = GoldshellFirmware;

    async fn revalidate(&self) -> anyhow::Result<bool> {
        let Ok(status) = self.web.read("status").await else {
            return Ok(false);
        };
        let Ok(model) = crate::firmware::model_from_status(&status) else {
            return Ok(false);
        };
        Ok(model.to_string() == self.device_info.model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asic_rs_makes_goldshell::models::GoldshellModel;
    use serde_json::json;
    fn miner() -> GoldshellV1 {
        GoldshellV1::new(
            IpAddr::from([127, 0, 0, 1]),
            GoldshellModel::Unknown("Goldshell".into()),
        )
    }
    #[test]
    fn present_invalid_current_rate_does_not_fall_back_to_older_samples() {
        let miner = miner();
        for invalid in [json!("NaN"), json!(-1), json!(null), json!(true)] {
            let row = json!({"MHS 20s": invalid, "MHS 5s": 25, "MHS av": 999});
            let data = HashMap::from([
                (DataField::Hashrate, row.clone()),
                (DataField::IsMining, row),
            ]);
            assert!(miner.parse_hashrate(&data).is_none());
            assert!(!miner.parse_is_mining(&data));
        }
        let row = json!({"MHS 5s": 25, "MHS av": 999});
        let data = HashMap::from([
            (DataField::Hashrate, row.clone()),
            (DataField::IsMining, row),
        ]);
        let rate = miner.parse_hashrate(&data).unwrap();
        assert_eq!(rate.value, 25.0);
        assert_eq!(rate.unit, HashRateUnit::MegaHash);
        assert_eq!(rate.algo, HashAlgorithm::Unknown);
        assert!(miner.parse_is_mining(&data));
        assert!(!miner.parse_is_mining(&HashMap::from([(
            DataField::IsMining,
            json!({"MHS 20s": 0, "MHS av": 999}),
        )])));
        assert!(!miner.parse_is_mining(&HashMap::new()));
    }
    #[test]
    fn observed_boards_do_not_require_expected_hardware_or_fabricate_chips() {
        let boards = miner().parse_hashboards(&HashMap::from([(
            DataField::Hashboards,
            json!({
                "devices": [{"ID": 0}, {"ID": 1}],
                "details": [{"ID": 0, "chips-nr": 16}, {"ID": 1, "chips-nr": 0}],
            }),
        )]));
        assert_eq!(boards.len(), 2);
        assert_eq!(boards[0].working_chips, Some(16));
        assert_eq!(boards[1].working_chips, Some(0));
        assert!(boards.iter().all(|b| b.expected_chips.is_none()));
        assert!(miner().get_expected_hashboards().is_none());
        assert!(miner().parse_wattage(&HashMap::new()).is_none());
    }
    #[test]
    fn live_devs_sensor_channels_and_fans_remain_separate_from_pcb_and_unknown_hardware() {
        let raw: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/live_sc5pro_2_2_0.json"
        ))
        .unwrap();
        let model = crate::firmware::model_from_status(&raw["status"]).unwrap();
        assert_eq!(model, GoldshellModel::SC5Pro);
        let live_miner = GoldshellV1::new(IpAddr::from([127, 0, 0, 1]), model);
        let boards = live_miner.parse_hashboards(&HashMap::from([(
            DataField::Hashboards,
            json!({"devices": raw["DEVS"], "details": raw["DEVDETAILS"]}),
        )]));
        assert_eq!(boards.len(), 4);
        assert!(
            boards
                .iter()
                .all(|board| board.working_chips == Some(84) && board.expected_chips.is_none())
        );
        assert_eq!(boards[0].board_temperature.unwrap().as_celsius(), 78.69);
        assert_eq!(boards[0].inlet_chip_temperature.unwrap().as_celsius(), 88.0);
        assert_eq!(
            boards[0].outlet_chip_temperature.unwrap().as_celsius(),
            88.0
        );
        assert!(boards[0].voltage.is_none());
        assert!(boards[0].frequency.is_none());
        let rate = boards[0].hashrate.as_ref().unwrap();
        assert_eq!(rate.algo, HashAlgorithm::Blake2b);
        assert_eq!(rate.unit, HashRateUnit::TeraHash);
        assert!((rate.value - 2.770500143).abs() < 1e-9);
        assert!(live_miner.get_expected_hashboards().is_none());
        let fan_rows = device_fans(&raw, None).unwrap();
        let fans = miner().parse_fans(&HashMap::from([(DataField::Fans, fan_rows.clone())]));
        assert_eq!(fans.len(), 4);
        for (fan, expected) in fans.iter().zip([2040.0, 2040.0, 2040.0, 2100.0]) {
            assert!((fan.rpm.unwrap().as_rpm() - expected).abs() < 1e-6);
        }
        let data = HashMap::from([(DataField::Hashrate, json!({"MHS 20s": 10_861_769.547}))]);
        let rate = live_miner.parse_hashrate(&data).unwrap();
        assert_eq!(rate.algo, HashAlgorithm::Blake2b);
        assert_eq!(rate.unit, HashRateUnit::TeraHash);
        assert!((rate.value - 10.861769547).abs() < 1e-9);
        assert!(live_miner.parse_wattage(&HashMap::new()).is_none());
    }
    #[test]
    fn live_unknown_ari31_preserves_declared_units_and_observed_boards() {
        let raw: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/live_ari31_2_2_3.json"
        ))
        .unwrap();
        let model = crate::firmware::model_from_status(&raw["status"]).unwrap();
        let miner = GoldshellV1::new(IpAddr::from([127, 0, 0, 1]), model);
        let rate = miner
            .parse_hashrate(&HashMap::from([(
                DataField::Hashrate,
                raw["SUMMARY"][0].clone(),
            )]))
            .unwrap();
        assert_eq!(rate.algo, HashAlgorithm::Unknown);
        assert_eq!(rate.unit, HashRateUnit::MegaHash);
        assert!(rate.value > 0.0);
        let boards = miner.parse_hashboards(&HashMap::from([(
            DataField::Hashboards,
            json!({"devices": raw["DEVS"], "details": raw["DEVDETAILS"]}),
        )]));
        assert_eq!(boards.len(), 4);
        assert!(boards.iter().all(|board| board.working_chips == Some(128)
            && board.expected_chips.is_none()
            && board.hashrate.as_ref().unwrap().algo == HashAlgorithm::Unknown
            && board.hashrate.as_ref().unwrap().unit == HashRateUnit::MegaHash));
        assert!(miner.get_expected_hashboards().is_none());
    }
    #[test]
    fn missing_chip_channels_are_not_filled_from_pcb_or_zero_sentinels() {
        let boards = miner().parse_hashboards(&HashMap::from([(
            DataField::Hashboards,
            json!({"devices": [
                {"ID": 0, "tstemp-2": 60, "tstemp-0": 0, "tstemp-1": -1},
                {"ID": 1, "tstemp-2": 61, "tstemp-0": 87, "tstemp-1": 90},
            ]}),
        )]));
        assert!(boards[0].inlet_chip_temperature.is_none());
        assert!(boards[0].outlet_chip_temperature.is_none());
        assert_eq!(boards[0].board_temperature.unwrap().as_celsius(), 60.0);
        assert_eq!(boards[1].inlet_chip_temperature.unwrap().as_celsius(), 87.0);
        assert_eq!(
            boards[1].outlet_chip_temperature.unwrap().as_celsius(),
            90.0
        );
        assert!(device_fans(&json!({"DEVS": [{"ID":0,"fan0":"invalid"}]}), None).is_none());
        assert!(device_fans(&json!({"DEVS": [{"ID":0,"fan0":0}]}), None).is_some());
    }
    #[tokio::test]
    async fn writes_are_rejected_before_a_connection_is_attempted() {
        let miner = miner();
        assert!(!miner.supports_pause());
        assert!(!miner.supports_restart());
        assert!(!miner.supports_resume());
        assert!(!miner.supports_pools_config());
        assert!(!miner.supports_change_password());
        assert!(miner.pause(None).await.is_err());
        assert!(miner.get_api_result(&rpc("restart")).await.is_err());
        assert!(
            miner
                .get_api_result(&MinerCommand::WebAPI {
                    command: "setting",
                    parameters: Some(json!({"select":0}))
                })
                .await
                .is_err()
        );
    }
}

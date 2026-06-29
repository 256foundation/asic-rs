use anyhow::{self, Context};
use asic_rs_core::{
    data::{command::MinerCommand, hashrate::HashRateUnit},
    test::api::MockAPIClient,
    traits::entry::FirmwareEntry,
};
use asic_rs_makes_volcminer::{hardware::VolcMinerControlBoard, models::VolcMinerModel};
use serde_json::json;

use super::rpc::VolcMinerRPCAPI;
use super::*;
use crate::firmware::VolcMinerStockFirmware;

const RPC_VERSION: MinerCommand = MinerCommand::RPC {
    command: "version",
    parameters: None,
};
const RPC_SUMMARY: MinerCommand = MinerCommand::RPC {
    command: "summary",
    parameters: None,
};
const RPC_STATS: MinerCommand = MinerCommand::RPC {
    command: "stats",
    parameters: None,
};
const RPC_POOLS: MinerCommand = MinerCommand::RPC {
    command: "pools",
    parameters: None,
};

const WEB_MINER_STATUS: MinerCommand = MinerCommand::WebAPI {
    command: "get_miner_status",
    parameters: None,
};
const WEB_SYSTEM_INFO: MinerCommand = MinerCommand::WebAPI {
    command: "get_system_info",
    parameters: None,
};

fn miner_ip_from_env() -> anyhow::Result<IpAddr> {
    let ip_str = std::env::var("MINER_IP").context("MINER_IP is not set")?;
    IpAddr::from_str(&ip_str).with_context(|| format!("invalid MINER_IP: {ip_str}"))
}

fn miner_auth_from_env() -> Option<MinerAuth> {
    std::env::var("MINER_PASSWORD").ok().map(|password| {
        let default_auth = VolcMinerV1::default_auth();
        let username =
            std::env::var("MINER_USERNAME").unwrap_or_else(|_| default_auth.username().to_string());
        MinerAuth::new(username, password)
    })
}

fn live_test_pool_urls_from_env() -> anyhow::Result<Vec<String>> {
    let urls = std::env::var("MINER_POOL_URLS").context("MINER_POOL_URLS is not set")?;
    let urls = urls
        .split(',')
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if urls.is_empty() {
        anyhow::bail!("MINER_POOL_URLS is empty");
    }
    Ok(urls)
}

fn live_test_pool_password(current: &[PoolGroupConfig], url: &str, username: &str) -> String {
    std::env::var("MINER_POOL_PASSWORD")
        .ok()
        .or_else(|| {
            current
                .iter()
                .flat_map(|group| group.pools.iter())
                .find_map(|pool| {
                    if pool.url.to_string() == url
                        && pool.username == username
                        && !pool.password.is_empty()
                    {
                        Some(pool.password.clone())
                    } else {
                        None
                    }
                })
        })
        .unwrap_or_else(|| "x".to_string())
}

#[test]
fn set_auth_updates_web_client_auth() {
    let mut miner = VolcMinerV1::new(IpAddr::from([127, 0, 0, 1]), VolcMinerModel::D1);
    let auth = MinerAuth::new("admin", "secret");

    miner.set_auth(auth);

    assert_eq!(miner.web_auth().username(), "admin");
    assert_eq!(miner.web_auth().password(), "secret");
}

#[test]
fn rpc_parses_malformed_stats_response() -> anyhow::Result<()> {
    let rpc = VolcMinerRPCAPI::new(IpAddr::from([127, 0, 0, 1]));
    let response = r#"{"STATUS":[{"STATUS":"S","Msg":"CGMiner stats"}],"STATS":[{"CGMiner":"2.3.3"}{"STATS":0,"fan1":5610,"chain_acn1":105}],"id":1}"#;

    let value = rpc.parse_rpc_result(response)?;

    assert_eq!(value.pointer("/STATS/1/fan1"), Some(&json!(5610)));
    assert_eq!(value.pointer("/STATS/1/chain_acn1"), Some(&json!(105)));

    Ok(())
}

#[test]
fn rpc_returns_error_status_as_error() {
    let rpc = VolcMinerRPCAPI::new(IpAddr::from([127, 0, 0, 1]));
    let response = r#"{"STATUS":[{"STATUS":"E","Msg":"Invalid command"}],"id":1}"#;

    assert!(rpc.parse_rpc_result(response).is_err());
}

#[tokio::test]
async fn test_volcminer_v1_parse_data() -> anyhow::Result<()> {
    let miner = VolcMinerV1::new(IpAddr::from([127, 0, 0, 1]), VolcMinerModel::D1);
    let mut results = HashMap::new();
    results.insert(
        WEB_SYSTEM_INFO,
        json!({
            "macaddr": "48:FA:68:34:68:01",
            "hostname": "VolcMiner D1",
            "system_filesystem_version": "2025-10-08 04-26-50 CST",
            "system_kernel_version": "Linux 4.4.0-xilinx #1 SMP PREEMPT Mon Apr 27 16:35:56 CST ",
            "cgminer_version": "4.12.0"
        }),
    );
    results.insert(
        WEB_MINER_STATUS,
        json!({
            "summary": {
                "elapsed": "42",
                "ghs5s": "18,500.5",
                "ghsav": "18,400.0"
            },
            "pools": [{
                "index": "0",
                "url": "stratum+tcp://pool.invalid:3333",
                "user": "worker",
                "status": "Alive",
                "accepted": "12",
                "rejected": "1"
            }],
            "fan1": "1,200",
            "fan2": "1300",
            "fan3": "0",
            "fan4": "0",
            "devs": [{
                "index": "1",
                "chain_acn": "120",
                "freq": "2000",
                "temp": [55, 57],
                "chain_acs": "oooooooo"
            }]
        }),
    );

    let mock_api = MockAPIClient::new(results);
    let mut collector = DataCollector::new_with_client(&miner, &mock_api);
    let data = collector.collect_all().await;
    let miner_data = miner.parse_data(data);

    assert_eq!(
        miner_data.mac,
        Some(MacAddr::from_str("48:FA:68:34:68:01")?)
    );
    assert_eq!(
        miner_data.firmware_version.as_deref(),
        Some("2025-10-08 04-26-50 CST")
    );
    assert_eq!(miner_data.hashrate.as_ref().map(|h| h.value), Some(18500.5));
    assert_eq!(
        miner_data.hashrate.as_ref().map(|h| h.unit),
        Some(HashRateUnit::MegaHash)
    );
    assert_eq!(
        miner_data.hashrate.as_ref().map(|h| h.algo.as_str()),
        Some("Scrypt")
    );
    assert_eq!(
        miner_data.control_board_version,
        Some(VolcMinerControlBoard::TVXilinx.into())
    );
    assert_eq!(miner_data.fans.len(), 2);
    assert_eq!(miner_data.hashboards.len(), 1);
    assert_eq!(miner_data.hashboards[0].working_chips, Some(120));
    assert_eq!(miner_data.pools.len(), 1);
    assert!(miner_data.is_mining);

    Ok(())
}

#[tokio::test]
async fn test_volcminer_v1_parse_rpc_data() -> anyhow::Result<()> {
    let miner = VolcMinerV1::new(IpAddr::from([127, 0, 0, 1]), VolcMinerModel::D1);
    let mut results = HashMap::new();
    results.insert(
        WEB_SYSTEM_INFO,
        json!({
            "macaddr": "50:DF:4D:84:79:EF",
            "hostname": "VolcMiner D1",
            "system_filesystem_version": "2025-02-18 15-51-04 CST",
            "system_kernel_version": "Linux 4.4.0-xilinx #1 SMP PREEMPT Mon Apr 27 16:35:56 CST 2020",
            "cgminer_version": "2.3.3"
        }),
    );
    results.insert(
        RPC_VERSION,
        json!({
            "STATUS": [{"STATUS": "S", "Msg": "CGMiner versions", "Description": "ccminer 2.3.3"}],
            "VERSION": [{"CGMiner": "2.3.3", "API": "3.1", "Miner": "1.3.0.8", "Type": "VolcMiner D1"}],
            "id": 1
        }),
    );
    results.insert(
        RPC_SUMMARY,
        json!({
            "STATUS": [{"STATUS": "S", "Msg": "Summary", "Description": "ccminer 2.3.3"}],
            "SUMMARY": [{"Elapsed": 1355, "MHS 5s": "15498.96", "MHS av": "15407.57"}],
            "id": 1
        }),
    );
    results.insert(
        RPC_STATS,
        json!({
            "STATUS": [{"STATUS": "S", "Msg": "CGMiner stats", "Description": "ccminer 2.3.3"}],
            "STATS": [
                {"CGMiner": "2.3.3", "Miner": "1.3.0.8", "Type": "VolcMiner D1"},
                {
                    "STATS": 0,
                    "MHS 5s": "15662.70",
                    "frequency": "1900",
                    "fan1": 5640,
                    "fan2": 5640,
                    "fan3": 5640,
                    "fan4": 5640,
                    "temp1": 64,
                    "temp2": 63,
                    "temp3": 0,
                    "temp4": 63,
                    "chain_acn1": 105,
                    "chain_acn2": 105,
                    "chain_acn3": 0,
                    "chain_acn4": 105,
                    "chain_acs1": "oooooooo",
                    "chain_acs2": "oooooooo",
                    "chain_acs3": "",
                    "chain_acs4": "oooooooo",
                    "chain_rate1": "5051.7909",
                    "chain_rate2": "5304.1120",
                    "chain_rate3": "",
                    "chain_rate4": "5306.7960"
                }
            ],
            "id": 1
        }),
    );
    results.insert(
        RPC_POOLS,
        json!({
            "STATUS": [{"STATUS": "S", "Msg": "3 Pool(s)", "Description": "ccminer 2.3.3"}],
            "POOLS": [
                {"POOL": 0, "URL": "stratum+tcp://pool.invalid:3333", "Status": "Alive", "Accepted": 101, "Rejected": 3, "User": "worker"},
                {"POOL": 1, "URL": "stratum+tcp://backup.invalid:3333", "Status": "Dead", "Accepted": 0, "Rejected": 0, "User": "worker"}
            ],
            "id": 1
        }),
    );

    let mock_api = MockAPIClient::new(results);
    let mut collector = DataCollector::new_with_client(&miner, &mock_api);
    let data = collector.collect_all().await;
    let miner_data = miner.parse_data(data);

    assert_eq!(miner_data.api_version.as_deref(), Some("3.1"));
    assert_eq!(
        miner_data.hashrate.as_ref().map(|h| h.value),
        Some(15498.96)
    );
    assert_eq!(miner_data.uptime, Some(Duration::from_secs(1355)));
    assert_eq!(miner_data.fans.len(), 4);
    assert_eq!(miner_data.hashboards.len(), 3);
    assert_eq!(miner_data.hashboards[0].position, 0);
    assert_eq!(miner_data.hashboards[2].position, 3);
    assert_eq!(miner_data.hashboards[0].working_chips, Some(105));
    assert_eq!(
        miner_data.hashboards[0].hashrate.as_ref().map(|h| h.value),
        Some(5051.7909)
    );
    assert_eq!(miner_data.pools.len(), 1);
    assert_eq!(miner_data.pools[0].pools.len(), 2);
    assert_eq!(miner_data.pools[0].pools[0].accepted_shares, Some(101));
    assert_eq!(miner_data.pools[0].pools[1].alive, Some(false));

    Ok(())
}

#[test]
fn test_parse_pools_config() -> anyhow::Result<()> {
    let miner = VolcMinerV1::new(IpAddr::from([127, 0, 0, 1]), VolcMinerModel::D1);
    let mut data = HashMap::new();
    data.insert(
        ConfigField::Pools,
        json!({
            "pools": [
                {"url": "stratum+tcp://pool.invalid:3333", "user": "worker", "pass": "x"},
                {"url": "", "user": "", "pass": ""}
            ],
            "freq": "2000",
            "coin-type": "ltc"
        }),
    );

    let groups = miner.parse_pools_config(&data)?;

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].pools.len(), 1);
    assert_eq!(groups[0].pools[0].username, "worker");
    assert_eq!(groups[0].pools[0].password, "x");

    Ok(())
}

#[tokio::test]
#[ignore = "requires live miner; set MINER_IP"]
async fn parse_data_live_test() -> anyhow::Result<()> {
    let ip = miner_ip_from_env()?;
    let auth = miner_auth_from_env();

    let miner = VolcMinerStockFirmware::default()
        .build_miner(ip, auth.as_ref())
        .await
        .context("no miner detected at MINER_IP")?;
    let miner_data = miner.get_data().await;
    println!("data {}", serde_json::to_string_pretty(&miner_data)?);

    println!(
        "pools {}",
        serde_json::to_string_pretty(&miner.get_pools_config().await?)?
    );

    assert_eq!(miner_data.ip, ip);
    assert!(miner_data.timestamp > 0);
    assert!(!miner_data.schema_version.is_empty());

    Ok(())
}

#[tokio::test]
#[ignore = "requires live miner and writes pool config; set MINER_IP, MINER_POOL_URLS, and MINER_POOL_USERNAME"]
async fn set_pools_config_live_test() -> anyhow::Result<()> {
    let ip = miner_ip_from_env()?;
    let auth = miner_auth_from_env();
    let pool_urls = live_test_pool_urls_from_env()?;
    let pool_username =
        std::env::var("MINER_POOL_USERNAME").context("MINER_POOL_USERNAME is not set")?;

    let miner = VolcMinerStockFirmware::default()
        .build_miner(ip, auth.as_ref())
        .await
        .context("no miner detected at MINER_IP")?;

    let current = miner.get_pools_config().await?;
    println!("current pools {}", serde_json::to_string_pretty(&current)?);

    let pools = pool_urls
        .iter()
        .map(|url| PoolConfig {
            url: PoolURL::from(url.to_string()),
            username: pool_username.clone(),
            password: live_test_pool_password(&current, url, &pool_username),
        })
        .collect::<Vec<_>>();
    let target = vec![PoolGroupConfig {
        name: "default".to_string(),
        quota: 1,
        pools: pools.clone(),
    }];

    assert!(miner.set_pools_config(target).await?);

    let updated = miner.get_pools_config().await?;
    println!("updated pools {}", serde_json::to_string_pretty(&updated)?);

    for expected in pools {
        let updated_pool = updated
            .iter()
            .flat_map(|group| group.pools.iter())
            .find(|pool| pool.url == expected.url && pool.username == expected.username)
            .with_context(|| format!("target pool config was not written: {}", expected.url))?;

        assert_eq!(updated_pool.password, expected.password);
    }

    Ok(())
}

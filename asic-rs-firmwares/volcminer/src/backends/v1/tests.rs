use anyhow::{self, Context};
use asic_rs_core::{test::api::MockAPIClient, traits::entry::FirmwareEntry};
use asic_rs_makes_volcminer::models::VolcMinerModel;
use serde_json::json;

use super::*;
use crate::firmware::VolcMinerFirmware;

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
            "cgminer_version": "4.12.0"
        }),
    );
    results.insert(
        WEB_STATUS,
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
    assert_eq!(miner_data.fans.len(), 4);
    assert_eq!(miner_data.hashboards.len(), 1);
    assert_eq!(miner_data.hashboards[0].working_chips, Some(120));
    assert_eq!(miner_data.pools.len(), 1);
    assert!(miner_data.is_mining);

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

    let miner = VolcMinerFirmware::default()
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

    let miner = VolcMinerFirmware::default()
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

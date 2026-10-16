use anyhow::Context;
use asic_rs_core::data::device::HashAlgorithm;
use asic_rs_core::test::api::MockAPIClient;
use asic_rs_makes_antminer::models::AntMinerModel;

use super::*;

#[tokio::test]
async fn legacy_http_stats_supply_temperatures_fans_and_uptime() {
    let miner = AntMinerV2018::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::Z11j);
    let mut results = HashMap::new();
    results.insert(
        MinerCommand::WebAPI {
            command: "miner_stats",
            parameters: None,
        },
        json!({"STATS": [{}, {
            "Elapsed": 3274,
            "chain_rate1": "22.94",
            "chain_acn1": 2,
            "temp1": 49,
            "temp2_1": 71,
            "fan1": 0,
            "fan2": 0,
            "fan3": 3840,
            "fan4": 2880,
            "total_rateideal": 320000.0
        }]}),
    );
    results.insert(
        MinerCommand::RPC {
            command: "summary",
            parameters: None,
        },
        json!({"SUMMARY": [{"GHS 5s": "92399.72"}]}),
    );

    let mock_api = MockAPIClient::new(results);
    let mut collector = DataCollector::new_with_client(&miner, &mock_api);
    let data = collector
        .collect(&[
            DataField::Hashboards,
            DataField::Fans,
            DataField::Uptime,
            DataField::Hashrate,
            DataField::ExpectedHashrate,
        ])
        .await;

    let boards = miner.parse_hashboards(&data);
    assert_eq!(boards[0].board_temperature.unwrap().as_celsius(), 49.0);
    assert_eq!(
        boards[0].outlet_chip_temperature.unwrap().as_celsius(),
        71.0
    );
    assert_eq!(
        boards[0].hashrate.as_ref().unwrap().unit,
        HashRateUnit::KiloHash
    );
    assert!((boards[0].hashrate.as_ref().unwrap().value - 22.94).abs() < 0.001);
    assert_eq!(miner.parse_fans(&data).len(), 2);
    assert_eq!(miner.parse_uptime(&data), Some(Duration::from_secs(3274)));

    let hashrate = miner.parse_hashrate(&data).unwrap();
    assert_eq!(hashrate.unit, HashRateUnit::KiloHash);
    assert!((hashrate.value - 92.39972).abs() < 0.001);
    let expected = miner.parse_expected_hashrate(&data).unwrap();
    assert_eq!(expected.unit, HashRateUnit::KiloHash);
    assert!((expected.value - 320.0).abs() < 0.001);
}

#[test]
fn saved_pool_password_must_match() {
    let expected = vec![json!({"url": "pool.example:3333", "user": "worker", "pass": "new"})];
    let actual = json!({"pools": [{"url": "pool.example:3333", "user": "worker", "pass": "old"}]});

    assert!(!pool_slots_match(&expected, &actual));
    assert!(!pool_slots_match(
        &[json!({"url": "", "user": "", "pass": ""})],
        &json!({"pools": []})
    ));
}

#[tokio::test]
async fn scrypt_model_is_read_in_the_unit_it_reports() {
    let miner = AntMinerV2018::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::L9);

    let mut results = HashMap::new();
    results.insert(
        MinerCommand::RPC {
            command: "summary",
            parameters: None,
        },
        json!({ "SUMMARY": [{ "MHS 5s": 16200.0, "MHS av": 16150.0 }] }),
    );
    results.insert(
        MinerCommand::WebAPI {
            command: "miner_stats",
            parameters: None,
        },
        json!({
            "STATS": [
                {},
                {
                    "rate_unit": "MH",
                    "total_rateideal": 16000.0,
                    "chain_rate1": 5400.0,
                    "chain_rate2": 5400.0,
                    "chain_rate3": 5400.0
                }
            ]
        }),
    );

    let mock_api = MockAPIClient::new(results);
    let mut collector = DataCollector::new_with_client(&miner, &mock_api);
    let miner_data = miner.parse_data(collector.collect_all().await);

    let hashrate = miner_data.hashrate.expect("hashrate");
    assert_eq!(hashrate.algo, HashAlgorithm::Scrypt);
    assert_eq!(hashrate.unit, HashRateUnit::GigaHash);
    assert_mega_hash(hashrate, 16200.0);

    let expected = miner_data.expected_hashrate.expect("expected hashrate");
    assert_eq!(expected.algo, HashAlgorithm::Scrypt);
    assert_eq!(expected.unit, HashRateUnit::GigaHash);
    assert_mega_hash(expected, 16000.0);

    let board = miner_data.hashboards[0].hashrate.clone().expect("board 0");
    assert_eq!(board.algo, HashAlgorithm::Scrypt);
    assert_eq!(board.unit, HashRateUnit::GigaHash);
    assert_mega_hash(board, 5400.0);
}

#[track_caller]
fn assert_mega_hash(hashrate: HashRate, expected: f64) {
    let value = hashrate.as_unit(HashRateUnit::MegaHash).value;
    assert!(
        (value - expected).abs() < 1e-6,
        "expected {expected} MH/s, got {value}"
    );
}

struct MockHttpServer {
    port: u16,
    task: tokio::task::JoinHandle<anyhow::Result<Vec<String>>>,
}

/// Answers each request in turn with the next `(status, reason, body)` and
/// hands back the requests it saw.
async fn mock_http_server(
    responses: Vec<(u16, &'static str, &'static str)>,
) -> anyhow::Result<MockHttpServer> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, reason, body) in responses {
            let (mut socket, _) = tokio::time::timeout(Duration::from_secs(2), listener.accept())
                .await
                .context("timed out waiting for HTTP request")??;

            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                if let Some(header_end) = request
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .map(|position| position + 4)
                {
                    let headers = std::str::from_utf8(&request[..header_end])?;
                    let content_length = headers
                        .lines()
                        .filter_map(|line| line.split_once(':'))
                        .find_map(|(name, value)| {
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or_default();
                    if request.len() >= header_end + content_length {
                        break;
                    }
                }
                let bytes_read = socket.read(&mut chunk).await?;
                if bytes_read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..bytes_read]);
            }
            requests.push(String::from_utf8(request)?);

            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await?;
        }
        Ok(requests)
    });

    Ok(MockHttpServer { port, task })
}

fn miner_with_mock_web(port: u16) -> AntMinerV2018 {
    let ip = IpAddr::from([127, 0, 0, 1]);
    let mut miner = AntMinerV2018::new(ip, AntMinerModel::S19);
    miner.web = AntMinerWebAPI::new_with_port(ip, port, MinerAuth::new("root", "root"));
    miner
}

#[tokio::test]
async fn set_pools_config_reports_why_the_miner_refused() -> anyhow::Result<()> {
    let MockHttpServer { port, task } = mock_http_server(vec![(
        500,
        "Internal Server Error",
        "set_miner_conf.cgi: config is locked",
    )])
    .await?;
    let miner = miner_with_mock_web(port);

    let error = miner.set_pools_config(vec![]).await.unwrap_err();

    let requests = task.await??;
    let request = requests.last().unwrap();
    assert!(request.starts_with("POST /cgi-bin/set_miner_conf.cgi HTTP/1.1\r\n"));
    assert!(request.contains("application/x-www-form-urlencoded"));
    let error = error.to_string();
    assert!(error.contains("500"), "{error}");
    assert!(
        error.contains("set_miner_conf.cgi: config is locked"),
        "{error}"
    );

    Ok(())
}

#[tokio::test]
async fn legacy_form_repairs_a_malformed_pool_config() -> anyhow::Result<()> {
    let MockHttpServer { port, task } = mock_http_server(vec![
            (200, "OK", "<html>miner restarting</html>"),
            (
                200,
                "OK",
                r#"{"pools":[{"url":"stratum+tcp://pool.example:3333","user":"worker","pass":"x"},{"url":"","user":"","pass":""},{"url":"","user":"","pass":""}]}"#,
            ),
        ])
        .await?;
    let miner = miner_with_mock_web(port);
    let pools = vec![PoolGroupConfig {
        name: "default".to_string(),
        quota: 1,
        pools: vec![PoolConfig {
            url: PoolURL::from("stratum+tcp://pool.example:3333".to_string()),
            username: "worker".to_string(),
            password: "x".to_string(),
        }],
    }];

    assert!(miner.set_pools_config(pools).await?);

    let requests = task.await??;
    assert!(requests[0].starts_with("POST /cgi-bin/set_miner_conf.cgi "));
    assert!(requests[0].contains("application/x-www-form-urlencoded"));
    assert!(requests[0].contains("stratum%2Btcp%3A%2F%2Fpool.example%3A3333"));

    Ok(())
}

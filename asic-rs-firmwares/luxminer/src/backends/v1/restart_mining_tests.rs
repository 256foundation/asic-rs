use std::net::IpAddr;

use asic_rs_core::traits::miner::RestartMining;
use asic_rs_makes_antminer::models::AntMinerModel;
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};

use super::{LuxMinerV1, rpc::LUXMinerRPCAPI};

struct MockRpcServer {
    port: u16,
    task: tokio::task::JoinHandle<anyhow::Result<Vec<Value>>>,
}

/// Answer `session`, then `resetminer` with the given status and message,
/// and return the requests that were received.
async fn mock_reset_server(
    status: &'static str,
    message: &'static str,
) -> anyhow::Result<MockRpcServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for request_idx in 0..2 {
            let (socket, _) =
                tokio::time::timeout(std::time::Duration::from_secs(2), listener.accept())
                    .await
                    .map_err(|_| {
                        anyhow::anyhow!("timed out waiting for request {}", request_idx + 1)
                    })??;
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            reader.read_line(&mut line).await?;

            let request: Value = serde_json::from_str(line.trim_end())?;
            let response = match request.get("command").and_then(Value::as_str) {
                Some("session") => json!({
                    "SESSION": [{ "SessionID": "test-session-id" }],
                    "STATUS": [{ "STATUS": "S", "Msg": "Session found" }]
                }),
                Some("resetminer") => json!({
                    "STATUS": [{ "STATUS": status, "Msg": message }]
                }),
                other => anyhow::bail!("unexpected command: {other:?}"),
            };
            requests.push(request);

            writer.write_all(format!("{response}\n").as_bytes()).await?;
        }

        Ok(requests)
    });

    Ok(MockRpcServer { port, task })
}

fn miner_with_mock_rpc(port: u16) -> LuxMinerV1 {
    let mut miner = LuxMinerV1::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19KPro);
    miner.rpc = LUXMinerRPCAPI::new_with_port(miner.ip, port);
    miner
}

#[tokio::test]
async fn restart_mining_sends_authenticated_resetminer_command() -> anyhow::Result<()> {
    let MockRpcServer { port, task } = mock_reset_server("S", "Miner reset").await?;
    let miner = miner_with_mock_rpc(port);

    assert!(miner.supports_restart_mining());
    assert!(miner.restart_mining().await?);

    assert_eq!(
        task.await??,
        vec![
            json!({ "command": "session" }),
            json!({
                "command": "resetminer",
                "parameter": "test-session-id"
            })
        ]
    );

    Ok(())
}

#[tokio::test]
async fn restart_mining_returns_rpc_rejection() -> anyhow::Result<()> {
    let MockRpcServer { port, task } = mock_reset_server("E", "Reset denied").await?;

    let error = miner_with_mock_rpc(port)
        .restart_mining()
        .await
        .unwrap_err();

    task.await??;
    assert!(error.to_string().contains("Reset denied"));

    Ok(())
}

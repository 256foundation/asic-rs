use std::net::IpAddr;

use asic_rs_core::traits::miner::{MinerAuth, RestartMining};
use asic_rs_makes_antminer::models::AntMinerModel;

use super::{BraiinsV2604, web::BraiinsWebAPI};
use crate::test::http::{MockResponse, MockSequenceServer, mock_sequence_server};

fn miner_with_mock_web(port: u16) -> BraiinsV2604 {
    let ip = IpAddr::from([127, 0, 0, 1]);
    let mut miner = BraiinsV2604::new(ip, AntMinerModel::S21Pro);
    miner.web = BraiinsWebAPI::new_with_port(ip, port, MinerAuth::from_token("test-token"));
    miner
}

#[tokio::test]
async fn restart_mining_puts_restart_action_not_reboot() -> anyhow::Result<()> {
    let MockSequenceServer { port, task } =
        mock_sequence_server(vec![MockResponse::ok("true")]).await?;
    let miner = miner_with_mock_web(port);

    assert!(miner.supports_restart_mining());
    assert!(miner.restart_mining().await?);

    let requests = task.await??;
    assert!(requests[0].starts_with("PUT /api/v1/actions/restart HTTP/1.1\r\n"));
    assert!(
        requests[0]
            .lines()
            .any(|line| line.eq_ignore_ascii_case("authorization: test-token"))
    );

    Ok(())
}

#[tokio::test]
async fn restart_mining_surfaces_firmware_rejection() -> anyhow::Result<()> {
    let MockSequenceServer { port, task } = mock_sequence_server(vec![MockResponse {
        status: "500 Internal Server Error",
        headers: &[],
        body: "bosminer is not running",
    }])
    .await?;
    let miner = miner_with_mock_web(port);

    let error = miner.restart_mining().await.unwrap_err();

    task.await??;
    assert!(error.to_string().contains("bosminer is not running"));

    Ok(())
}

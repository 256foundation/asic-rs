use std::net::IpAddr;

use asic_rs_core::traits::miner::RestartMining;
use asic_rs_makes_antminer::models::AntMinerModel;

use super::AntMinerV2020;

#[tokio::test]
async fn restart_mining_is_unsupported_on_stock() {
    let miner = AntMinerV2020::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19);

    assert!(!miner.supports_restart_mining());
    let error = miner.restart_mining().await.unwrap_err();
    assert!(error.to_string().contains("not supported"));
}

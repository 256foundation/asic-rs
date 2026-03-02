use anyhow::Result;
use asic_rs_core::traits::miner::WebAPIClient;
use async_trait::async_trait;
use reqwest::Method;
use serde_json::Value;

pub use super::super::v2_0_0::web::BitaxeWebAPI;

#[async_trait]
#[allow(dead_code)]
trait Bitaxe290WebAPI: WebAPIClient {
    /// Get ASIC information
    async fn asic_info(&self) -> Result<Value> {
        self.send_command("system/asic", false, None, Method::GET)
            .await
    }
}

impl Bitaxe290WebAPI for BitaxeWebAPI {}

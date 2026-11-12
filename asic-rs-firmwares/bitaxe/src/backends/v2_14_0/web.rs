use anyhow::Result;
use asic_rs_core::traits::miner::WebAPIClient;
use async_trait::async_trait;
use reqwest::Method;
use serde_json::Value;

pub use super::super::v2_0_0::web::BitaxeWebAPI;

#[async_trait]
#[allow(dead_code)]
pub(in crate::backends) trait Bitaxe2140WebAPI: WebAPIClient {
    /// Pause mining without restarting the device.
    #[allow(clippy::double_must_use)] // async_trait adds #[must_use] to a must-use future.
    async fn pause(&self) -> Result<Value> {
        self.send_command("system/pause", false, None, Method::POST)
            .await
    }

    /// Resume mining after a pause.
    #[allow(clippy::double_must_use)] // async_trait adds #[must_use] to a must-use future.
    async fn resume(&self) -> Result<Value> {
        self.send_command("system/resume", false, None, Method::POST)
            .await
    }

    /// Get ASIC information
    #[allow(clippy::double_must_use)] // async_trait adds #[must_use] to a must-use future.
    async fn asic_info(&self) -> Result<Value> {
        self.send_command("system/asic", false, None, Method::GET)
            .await
    }
}

impl Bitaxe2140WebAPI for BitaxeWebAPI {}

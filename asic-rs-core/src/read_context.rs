//! Explicit response reuse across separate read APIs for one miner.
//!
//! A context retains successful raw command responses, fetching each on first use.
//! Telemetry and configuration still use their own collectors, parsers, and result
//! types. Getters can be called in any order or concurrently. Ordinary miner
//! methods keep their existing fresh-read behavior.
//!
//! Create a new context for each polling operation. Drop it after that operation,
//! and before changing settings, credentials, or firmware. There is no TTL or
//! automatic invalidation: reusing a context deliberately reuses its responses.
//!
//! ```no_run
//! use asic_rs_core::{read_context::ReadContext, traits::miner::Miner};
//!
//! async fn read(miner: &dyn Miner) -> anyhow::Result<()> {
//!     let reads = ReadContext::new(miner);
//!     // This works without calling get_data() first.
//!     let tuning = reads.get_tuning_config().await?;
//!     let telemetry = reads.get_data().await;
//!     // Shared commands are fetched once; the results remain separate.
//!     Ok(())
//! }
//! ```

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde_json::Value;
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, OnceCell};

use crate::{
    config::{
        collector::{ConfigCollector, ConfigField},
        fan::FanConfig,
        tuning::TuningConfig,
    },
    data::{
        collector::{DataCollector, DataField},
        command::MinerCommand,
        firmware::FirmwareStats,
        miner::MinerData,
    },
    traits::miner::{APIClient, Miner},
};

/// A read-only grouping of telemetry and config reads for one miner.
///
/// No writes are exposed. Callers must finish this context before performing
/// writes through the original miner or starting the next poll.
pub struct ReadContext<'a> {
    miner: &'a dyn Miner,
    client: ResponseCache<'a>,
}

impl<'a> ReadContext<'a> {
    /// Start an empty context for this miner and its current credentials.
    pub fn new(miner: &'a dyn Miner) -> Self {
        Self::new_with_client(miner, miner)
    }

    /// Use a custom API client for the same miner and credentials.
    ///
    /// This supports instrumented clients and testing. The supplied client must
    /// read the miner described by `miner`; cache entries never cross contexts.
    pub fn new_with_client(miner: &'a dyn Miner, client: &'a dyn APIClient) -> Self {
        Self {
            miner,
            client: ResponseCache::new(client),
        }
    }

    /// Whether this backend can check releases through the shared read client.
    pub fn supports_check_firmware_update(&self) -> bool {
        self.miner.supports_check_firmware_update_with_client()
    }

    /// Check releases separately from telemetry, reusing any cached miner responses.
    /// Required responses are fetched if this is the first read in the context.
    /// The vendor release lookup retains its normal behavior and cache lifetime.
    pub async fn check_firmware_update(&self) -> anyhow::Result<FirmwareStats> {
        self.miner
            .check_firmware_update_with_client(&self.client)
            .await
    }

    /// Read telemetry, reusing raw responses fetched earlier in this context.
    /// This does not collect configuration or check vendor releases.
    pub async fn get_data(&self) -> MinerData {
        self.get_data_filtered(Vec::new()).await
    }

    /// Read telemetry excluding the specified fields, as with the miner getter.
    /// Excluded fields are not parsed even if their responses are already cached.
    pub async fn get_data_filtered(&self, exclude: Vec<DataField>) -> MinerData {
        let fields = DataField::iter()
            .filter(|field| !exclude.contains(field))
            .collect::<Vec<_>>();
        let mut collector = DataCollector::new_with_client(self.miner, &self.client);
        let data = collector.collect(&fields).await;
        self.miner.parse_data(data)
    }

    /// Read fan configuration independently of telemetry or tuning.
    /// Any missing command responses are fetched on demand.
    pub async fn get_fan_config(&self) -> anyhow::Result<FanConfig> {
        let mut collector = ConfigCollector::new_with_client(self.miner, &self.client);
        let config = collector.collect(&[ConfigField::Fan]).await;
        self.miner.parse_fan_config(&config)
    }

    /// Read tuning configuration independently of telemetry.
    /// Any missing command responses are fetched on demand.
    pub async fn get_tuning_config(&self) -> anyhow::Result<TuningConfig> {
        let mut collector = ConfigCollector::new_with_client(self.miner, &self.client);
        let config = collector.collect(&[ConfigField::Tuning]).await;
        self.miner.parse_tuning_config(&config)
    }
}

/// Cache only successful responses. Each entry coordinates concurrent readers;
/// distinct commands remain concurrent. Failed or cancelled requests can retry.
/// No Debug implementation: command responses can contain configuration secrets.
struct ResponseCache<'a> {
    client: &'a dyn APIClient,
    entries: Mutex<HashMap<MinerCommand, Arc<OnceCell<Value>>>>,
}

impl<'a> ResponseCache<'a> {
    fn new(client: &'a dyn APIClient) -> Self {
        Self {
            client,
            entries: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl APIClient for ResponseCache<'_> {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        let entry = {
            let mut entries = self.entries.lock().await;
            Arc::clone(entries.entry(command.clone()).or_default())
        };
        let response = entry
            .get_or_try_init(|| self.client.get_api_result(command))
            .await?;
        Ok(response.clone())
    }
}

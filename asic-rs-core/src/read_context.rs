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
        tuning::TuningConfig,
    },
    data::{
        collector::{DataCollector, DataField},
        command::MinerCommand,
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

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use serde_json::json;
    use tokio::sync::{Barrier, Notify};

    use super::*;

    const SUMMARY: MinerCommand = MinerCommand::WebAPI {
        command: "summary",
        parameters: None,
    };
    const NETWORK: MinerCommand = MinerCommand::WebAPI {
        command: "network",
        parameters: None,
    };

    struct CountingClient(AtomicUsize);

    #[async_trait]
    impl APIClient for CountingClient {
        async fn get_api_result(&self, _: &MinerCommand) -> anyhow::Result<Value> {
            let call = self.0.fetch_add(1, Ordering::SeqCst) + 1;
            tokio::task::yield_now().await;
            Ok(json!({"call": call}))
        }
    }

    #[tokio::test]
    async fn concurrent_identical_reads_share_one_request_and_new_scopes_refresh()
    -> anyhow::Result<()> {
        let client = CountingClient(AtomicUsize::new(0));
        let cache = ResponseCache::new(&client);
        let (one, two) = tokio::join!(
            cache.get_api_result(&SUMMARY),
            cache.get_api_result(&SUMMARY)
        );
        assert_eq!(one?, json!({"call": 1}));
        assert_eq!(two?, json!({"call": 1}));
        let mut response = cache.get_api_result(&SUMMARY).await?;
        response["call"] = json!(999);
        assert_eq!(cache.get_api_result(&SUMMARY).await?, json!({"call": 1}));
        assert_eq!(client.0.load(Ordering::SeqCst), 1);
        let next = ResponseCache::new(&client);
        assert_eq!(next.get_api_result(&SUMMARY).await?, json!({"call": 2}));
        Ok(())
    }

    #[tokio::test]
    async fn cache_keys_include_transport_and_parameters() -> anyhow::Result<()> {
        let client = CountingClient(AtomicUsize::new(0));
        let cache = ResponseCache::new(&client);
        for command in [
            SUMMARY,
            MinerCommand::WebAPI {
                command: "summary",
                parameters: Some(json!({"board": 1})),
            },
            MinerCommand::WebAPI {
                command: "summary",
                parameters: Some(json!({"board": 2})),
            },
            MinerCommand::RPC {
                command: "summary",
                parameters: None,
            },
        ] {
            cache.get_api_result(&command).await?;
            cache.get_api_result(&command).await?;
        }
        assert_eq!(client.0.load(Ordering::SeqCst), 4);
        Ok(())
    }

    struct ParallelClient(Barrier);

    #[async_trait]
    impl APIClient for ParallelClient {
        async fn get_api_result(&self, _: &MinerCommand) -> anyhow::Result<Value> {
            self.0.wait().await;
            Ok(json!({}))
        }
    }

    #[tokio::test]
    async fn unrelated_commands_are_not_serialized_by_the_cache() -> anyhow::Result<()> {
        let client = ParallelClient(Barrier::new(2));
        let cache = ResponseCache::new(&client);
        let (one, two) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(
                cache.get_api_result(&SUMMARY),
                cache.get_api_result(&NETWORK)
            )
        })
        .await?;
        one?;
        two?;
        Ok(())
    }

    struct FailingClient(AtomicUsize);

    #[async_trait]
    impl APIClient for FailingClient {
        async fn get_api_result(&self, _: &MinerCommand) -> anyhow::Result<Value> {
            if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
                anyhow::bail!("temporary failure");
            }
            Ok(json!({"recovered": true}))
        }
    }

    #[tokio::test]
    async fn failures_are_not_cached() -> anyhow::Result<()> {
        let client = FailingClient(AtomicUsize::new(0));
        let cache = ResponseCache::new(&client);
        assert!(cache.get_api_result(&SUMMARY).await.is_err());
        assert_eq!(
            cache.get_api_result(&SUMMARY).await?,
            json!({"recovered": true})
        );
        cache.get_api_result(&SUMMARY).await?;
        assert_eq!(client.0.load(Ordering::SeqCst), 2);
        Ok(())
    }

    struct CancelledClient {
        calls: AtomicUsize,
        started: Notify,
    }

    #[async_trait]
    impl APIClient for CancelledClient {
        async fn get_api_result(&self, _: &MinerCommand) -> anyhow::Result<Value> {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                self.started.notify_one();
                std::future::pending::<()>().await;
            }
            Ok(json!({"retried": true}))
        }
    }

    #[tokio::test]
    async fn cancelling_an_initializer_does_not_poison_the_context() -> anyhow::Result<()> {
        let client = CancelledClient {
            calls: AtomicUsize::new(0),
            started: Notify::new(),
        };
        let cache = ResponseCache::new(&client);
        {
            let request = cache.get_api_result(&SUMMARY);
            tokio::pin!(request);
            tokio::select! {
                _ = client.started.notified() => {},
                result = &mut request => anyhow::bail!("request unexpectedly completed: {result:?}"),
            }
        }
        let response =
            tokio::time::timeout(Duration::from_secs(2), cache.get_api_result(&SUMMARY)).await??;
        assert_eq!(response, json!({"retried": true}));
        assert_eq!(client.calls.load(Ordering::SeqCst), 2);
        Ok(())
    }
}

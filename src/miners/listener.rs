use std::pin::Pin;

use anyhow;
use async_stream::stream;
use tokio::net::UdpSocket;
use tokio_stream::{Stream, StreamExt};

use super::backends::traits::*;
use super::factory::MinerFactory;

pub struct MinerListener {
    antminer_listener: AntMinerListener,
    whatsminer_listener: WhatsMinerListener,
}

impl Default for MinerListener {
    fn default() -> Self {
        Self::new()
    }
}

impl MinerListener {
    pub fn new() -> Self {
        MinerListener {
            antminer_listener: AntMinerListener::new(),
            whatsminer_listener: WhatsMinerListener::new(),
        }
    }

    /// Listen for miners on the network.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use asic_rs::miners::listener::MinerListener;
    /// use futures::pin_mut;
    /// use tokio_stream::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> () {
    ///     let listener = MinerListener::new();
    ///     let stream = listener.listen().await;
    ///     pin_mut!(stream);
    ///
    ///     while let Some(miner) = stream.next().await {
    ///         println!("Found miner: {miner:?}")
    ///     }
    /// }
    /// ```
    pub async fn listen(
        &self,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<Option<Box<dyn Miner>>>> + '_>> {
        let am_stream = self.antminer_listener.listen().await;
        let wm_stream = self.whatsminer_listener.listen().await;

        let stream = am_stream.merge(wm_stream);

        Box::pin(stream)
    }
}

struct AntMinerListener {}

impl AntMinerListener {
    pub fn new() -> Self {
        AntMinerListener {}
    }

    pub(crate) async fn listen(
        &self,
    ) -> impl Stream<Item = anyhow::Result<Option<Box<dyn Miner>>>> {
        stream! {
            let factory = MinerFactory::new();
            let sock = UdpSocket::bind("0.0.0.0:14235").await.expect("Failed to bind to port 14235 to listen for AntMiners.");
            let mut buf = Vec::with_capacity(256);

            loop {
                let (_len, addr) = sock.recv_buf_from(&mut buf).await.unwrap();
                yield factory.get_miner(addr.ip()).await;
            }
        }
    }
}

struct WhatsMinerListener {}

impl WhatsMinerListener {
    pub fn new() -> Self {
        WhatsMinerListener {}
    }

    pub(crate) async fn listen(
        &self,
    ) -> impl Stream<Item = anyhow::Result<Option<Box<dyn Miner>>>> {
        stream! {
            let factory = MinerFactory::new();
            let sock = UdpSocket::bind("0.0.0.0:8888").await.expect("Failed to bind to port 8888 to listen for WhatsMiners.");
            let mut buf = Vec::with_capacity(256);

            loop {
                let (_len, addr) = sock.recv_buf_from(&mut buf).await.unwrap();
                yield factory.get_miner(addr.ip()).await;
            }
        }
    }
}

//! Data types for asic-rs
//!
//! The most important data type is [`MinerData`][`miner::MinerData`], it contains all the data asic-rs gathers with `get_data`.

pub mod board;
pub(crate) mod deserialize;
pub mod device;
pub mod fan;
pub mod hashrate;
pub mod message;
pub mod miner;
pub mod mode;
pub mod pool;
pub(crate) mod serialize;

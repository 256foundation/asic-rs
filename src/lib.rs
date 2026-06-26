#![doc = include_str!("../docs-shared/guide.md")]

pub use factory::MinerFactory;
pub use listener::MinerListener;

#[cfg(feature = "core")]
pub use asic_rs_core as core;

#[cfg(feature = "antminer")]
pub use asic_rs_firmwares_antminer as antminer;
#[cfg(feature = "avalonminer")]
pub use asic_rs_firmwares_avalonminer as avalonminer;
#[cfg(feature = "bitaxe")]
pub use asic_rs_firmwares_bitaxe as bitaxe;
#[cfg(feature = "braiins")]
pub use asic_rs_firmwares_braiins as braiins;
#[cfg(feature = "epic")]
pub use asic_rs_firmwares_epic as epic;
#[cfg(feature = "futurebit")]
pub use asic_rs_firmwares_futurebit as futurebit;
#[cfg(feature = "luxminer")]
pub use asic_rs_firmwares_luxminer as luxminer;
#[cfg(feature = "marathon")]
pub use asic_rs_firmwares_marathon as marathon;
#[cfg(feature = "nerdaxe")]
pub use asic_rs_firmwares_nerdaxe as nerdaxe;
#[cfg(feature = "proto")]
pub use asic_rs_firmwares_proto as proto;
#[cfg(feature = "sealminer")]
pub use asic_rs_firmwares_sealminer as sealminer;
#[cfg(feature = "vnish")]
pub use asic_rs_firmwares_vnish as vnish;
#[cfg(feature = "volcminer")]
pub use asic_rs_firmwares_volcminer as volcminer;
#[cfg(feature = "whatsminer")]
pub use asic_rs_firmwares_whatsminer as whatsminer;

pub mod factory;
pub mod listener;
#[cfg(feature = "python")]
mod python;

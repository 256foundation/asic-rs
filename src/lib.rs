//! asic-rs is a miner management and control library, designed to abstract away the complexity of working with different types of ASIC miners.
//! # Getting Started
//! The first step to controlling a miner with asic-rs is to get the struct that represents it, with methods used for data gathering and control.
//!
//! ### Getting a miner
//! If you know the IP address of your miner, it is fairly easy to discover it.  Use the [`MinerFactory`] to select the correct type.
//! ```no_run
//! use asic_rs::MinerFactory;
//! use std::str::FromStr;
//! use std::net::IpAddr;
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() {
//!     let factory = MinerFactory::new();
//!     let ip = IpAddr::from_str("192.168.1.10").unwrap();
//!     let miner = factory.get_miner(ip).await.unwrap();
//!     // now we can do data gathering or control
//! }
//! ```
//!
//! ### Miner discovery
//! If you don't know the specific IP of your miner, asic-rs can discover it on your network.
//! ```no_run
//! use asic_rs::MinerFactory;
//! use std::str::FromStr;
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() {
//!     let subnet = "192.168.1.0/24";
//!     let factory = MinerFactory::from_subnet(subnet).unwrap();
//!     let miners = factory.scan().await.unwrap();
//! }
//! ```
//!
//! There are other ways to define a discovery range to be scanned, such as:
//!
//! - Octets
//! ```no_run
//! # use asic_rs::MinerFactory;
//! # use std::str::FromStr;
//! # use tokio;
//! # #[tokio::main]
//! # async fn main() {
//! #     let subnet = "192.168.1.0/24";
//!     let factory = MinerFactory::from_octets("192", "168", "1", "1-255").unwrap();
//! #     let miners = factory.scan().await.unwrap();
//! # }
//! ```
//! - Range string
//! ```no_run
//! # use asic_rs::MinerFactory;
//! # use std::str::FromStr;
//! # use tokio;
//! # #[tokio::main]
//! # async fn main() {
//! #     let subnet = "192.168.1.0/24";
//!     let factory = MinerFactory::from_range("192.168.1.1-255").unwrap();
//! #     let miners = factory.scan().await.unwrap();
//! # }
//! ```
//!
//! These also have corresponding methods for appending to an existing factory, or overwriting existing ranges.
//! See [`MinerFactory`] for more details.
//!
//! ### Discovery tuning
//! For large scans, `MinerFactory` automatically tries to raise process file descriptor limits when needed.
//! On Unix this uses `RLIMIT_NOFILE`, and on Windows it uses stdio max-file limits.
//! This is fail-open: if the OS does not allow raising the limit, scanning still runs.
//! ```no_run
//! # use asic_rs::MinerFactory;
//! # use tokio;
//! # #[tokio::main]
//! # async fn main() {
//! let factory = MinerFactory::from_subnet("192.168.1.0/24")
//!     .unwrap()
//!     .with_concurrent_limit(2500)
//!     .with_nofile_limit(20000);
//! let miners = factory.scan().await.unwrap();
//! # }
//! ```
//! Disable automatic nofile adjustment if needed:
//! ```no_run
//! # use asic_rs::MinerFactory;
//! let factory = MinerFactory::new().with_nofile_adjustment(false);
//! ```
//!
//! ### Data gathering
//! Getting data is very simple with asic-rs, everything you need can be gathered with a single call.
//! Extending the "Getting a miner" example:
//! ```no_run
//! use asic_rs::MinerFactory;
//! use std::str::FromStr;
//! use std::net::IpAddr;
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() {
//!     let factory = MinerFactory::new();
//!     let ip = IpAddr::from_str("192.168.1.10").unwrap();
//!     let miner_opt = factory.get_miner(ip).await.unwrap();
//!     // First unwrap represents an error getting the miner
//!     // Now make sure there is actually a valid, supported miner
//!     if let Some(miner) = miner_opt {
//!         let data = miner.get_data().await;
//!     }
//! }
//! ```
//!
//! If you only want specific data, that can be done with individual function calls:
//! ```no_run
//! # use asic_rs::MinerFactory;
//! # use std::str::FromStr;
//! # use std::net::IpAddr;
//! # use tokio;
//! # #[tokio::main]
//! # async fn main() {
//! #     let factory = MinerFactory::new();
//! #     let ip = IpAddr::from_str("192.168.1.10").unwrap();
//! #     let miner_opt = factory.get_miner(ip).await.unwrap();
//! #     // First unwrap represents an error getting the miner
//! #     // Now make sure there is actually a valid, supported miner
//! #     if let Some(miner) = miner_opt {
//!         let mac = miner.get_mac().await;
//! #     }
//! # }
//! ```
//!
//! Most data points from [`MinerData`][`data::miner::MinerData`] have a corresponding `get_...` function.
//! See the [`GetMinerData`][`miners::backends::traits::GetMinerData`] trait for more info.
//!
//! ### Miner control
//! Controlling a miner is very similar to getting data in asic-rs.
//! Each miner has some control functions defined by the [`HasMinerControl`][`miners::backends::traits::HasMinerControl`] trait.
//! Again extending the "Getting a miner" example:
//! ```no_run
//! use asic_rs::MinerFactory;
//! use std::str::FromStr;
//! use std::net::IpAddr;
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() {
//!     let factory = MinerFactory::new();
//!     let ip = IpAddr::from_str("192.168.1.10").unwrap();
//!     let miner_opt = factory.get_miner(ip).await.unwrap();
//!     // First unwrap represents an error getting the miner
//!     // Now make sure there is actually a valid, supported miner
//!     if let Some(miner) = miner_opt {
//!         let result = miner.restart().await;
//!         if let Ok(true) = result {
//!             println!("Miner restart succeeded")
//!         }
//!     }
//! }
//! ```
//!
//! ### Authentication
//! By default, each backend uses its built-in default credentials (e.g. `root/root` for AntMiner,
//! `admin/admin` for WhatsMiner). To use custom credentials, call `set_auth` on the miner:
//! ```no_run
//! use asic_rs::MinerFactory;
//! use asic_rs_core::traits::auth::MinerAuth;
//! use std::str::FromStr;
//! use std::net::IpAddr;
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() {
//!     let factory = MinerFactory::new();
//!     let ip = IpAddr::from_str("192.168.1.10").unwrap();
//!     let miner_opt = factory.get_miner(ip).await.unwrap();
//!     if let Some(mut miner) = miner_opt {
//!         miner.set_auth(MinerAuth::new("myuser", "mypassword"));
//!         // All subsequent operations use the custom credentials
//!         let data = miner.get_data().await;
//!     }
//! }
//! ```
//!
//! Credentials can also be passed during discovery via `build_miner`, which applies them
//! to both discovery (e.g. AntMiner digest auth) and runtime operations.
//!
//! # Contributing
//!
//! Contributions are welcome! This project uses the [Conventional Commits](https://www.conventionalcommits.org/) specification for commit messages.
//! Please format your commits accordingly, for example:
//!
//! - `feat: add new miner support`
//! - `fix: correct hashrate parsing`
//! - `fix(python): fix missing reference to rust function`
//! - `docs: update getting started guide`
//!
//! ## Setting up pre-commit hooks
//!
//! This project uses [pre-commit](https://pre-commit.com/) to enforce commit message formatting and code quality.
//! To set up the hooks:
//!
//! ```sh
//! pip install pre-commit
//! pre-commit install --hook-type commit-msg --hook-type pre-commit
//! ```
//!
//! ## README
//! The README is auto generated with `doc2readme`, please do not edit it manually.
//! Instead, changes can be made in `lib.rs`.

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
#[cfg(feature = "luxminer")]
pub use asic_rs_firmwares_luxminer as luxminer;
#[cfg(feature = "marathon")]
pub use asic_rs_firmwares_marathon as marathon;
#[cfg(feature = "nerdaxe")]
pub use asic_rs_firmwares_nerdaxe as nerdaxe;
#[cfg(feature = "vnish")]
pub use asic_rs_firmwares_vnish as vnish;
#[cfg(feature = "whatsminer")]
pub use asic_rs_firmwares_whatsminer as whatsminer;

pub mod factory;
pub mod listener;
#[cfg(feature = "python")]
mod python;

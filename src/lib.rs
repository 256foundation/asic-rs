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
//! # Contributing
//!
//! Contributions are welcome! This project uses the [Conventional Commits](https://www.conventionalcommits.org/) specification for commit messages.
//! Please format your commits accordingly, for example:
//!
//! - `feat: add new miner support`
//! - `fix: correct hashrate parsing`
//! - `docs: update getting started guide`

pub use crate::miners::factory::MinerFactory;
pub use crate::miners::listener::MinerListener;

pub mod data;
pub mod miners;
pub(crate) mod test;

#[cfg(feature = "python")]
mod python;

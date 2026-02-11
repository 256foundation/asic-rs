# asic-rs ![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue) [![asic-rs on crates.io](https://img.shields.io/crates/v/asic-rs)](https://crates.io/crates/asic-rs) [![asic-rs on docs.rs](https://docs.rs/asic-rs/badge.svg)](https://docs.rs/asic-rs) [![Conventional Commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-%23FE5196?logo=conventionalcommits&logoColor=white)](https://conventionalcommits.org)

asic-rs is a miner management and control library, designed to abstract away the complexity of working with different types of ASIC miners.

## Getting Started

The first step to controlling a miner with asic-rs is to get the struct that represents it, with methods used for data gathering and control.

#### Getting a miner

If you know the IP address of your miner, it is fairly easy to discover it.  Use the [`MinerFactory`][__link0] to select the correct type.

```rust
use asic_rs::MinerFactory;
use std::str::FromStr;
use std::net::IpAddr;
use tokio;

#[tokio::main]
async fn main() {
    let factory = MinerFactory::new();
    let ip = IpAddr::from_str("192.168.1.10").unwrap();
    let miner = factory.get_miner(ip).await.unwrap();
    // now we can do data gathering or control
}
```

#### Miner discovery

If you don’t know the specific IP of your miner, asic-rs can discover it on your network.

```rust
use asic_rs::MinerFactory;
use std::str::FromStr;
use tokio;

#[tokio::main]
async fn main() {
    let subnet = "192.168.1.0/24";
    let factory = MinerFactory::from_subnet(subnet).unwrap();
    let miners = factory.scan().await.unwrap();
}
```

There are other ways to define a discovery range to be scanned, such as:

* Octets

```rust
    let factory = MinerFactory::from_octets("192", "168", "1", "1-255").unwrap();
```

* Range string

```rust
    let factory = MinerFactory::from_range("192.168.1.1-255").unwrap();
```

These also have corresponding methods for appending to an existing factory, or overwriting existing ranges.
See [`MinerFactory`][__link1] for more details.

#### Data gathering

Getting data is very simple with asic-rs, everything you need can be gathered with a single call.
Extending the “Getting a miner” example:

```rust
use asic_rs::MinerFactory;
use std::str::FromStr;
use std::net::IpAddr;
use tokio;

#[tokio::main]
async fn main() {
    let factory = MinerFactory::new();
    let ip = IpAddr::from_str("192.168.1.10").unwrap();
    let miner_opt = factory.get_miner(ip).await.unwrap();
    // First unwrap represents an error getting the miner
    // Now make sure there is actually a valid, supported miner
    if let Some(miner) = miner_opt {
        let data = miner.get_data().await;
    }
}
```

If you only want specific data, that can be done with individual function calls:

```rust
        let mac = miner.get_mac().await;
```

Most data points from [`MinerData`][__link2] have a corresponding `get_...` function.
See the [`GetMinerData`][__link3] trait for more info.

#### Miner control

Controlling a miner is very similar to getting data in asic-rs.
Each miner has some control functions defined by the [`HasMinerControl`][__link4] trait.
Again extending the “Getting a miner” example:

```rust
use asic_rs::MinerFactory;
use std::str::FromStr;
use std::net::IpAddr;
use tokio;

#[tokio::main]
async fn main() {
    let factory = MinerFactory::new();
    let ip = IpAddr::from_str("192.168.1.10").unwrap();
    let miner_opt = factory.get_miner(ip).await.unwrap();
    // First unwrap represents an error getting the miner
    // Now make sure there is actually a valid, supported miner
    if let Some(miner) = miner_opt {
        let result = miner.restart().await;
        if let Ok(true) = result {
            println!("Miner restart succeeded")
        }
    }
}
```

## Contributing

Contributions are welcome! This project uses the [Conventional Commits][__link5] specification for commit messages.
Please format your commits accordingly, for example:

* `feat: add new miner support`
* `fix: correct hashrate parsing`
* `docs: update getting started guide`


 [__cargo_doc2readme_dependencies_info]: ggGkYW0CYXSEG7zYzQEoMuUiG7bjUS7Q1YKmGzZY4az0AsGrGx0Aapd5TVHgYXKEG9xeSlsevzdnG8T8_cfSQ6K5G5dA_j5Tq3_LG69X3u_Pf7yXYWSBg2dhc2ljLXJzZTAuMS42Z2FzaWNfcnM
 [__link0]: https://docs.rs/asic-rs/0.1.6/asic_rs/?search=miners::factory::MinerFactory
 [__link1]: https://docs.rs/asic-rs/0.1.6/asic_rs/?search=miners::factory::MinerFactory
 [__link2]: https://docs.rs/asic-rs/0.1.6/asic_rs/?search=data::miner::MinerData
 [__link3]: https://docs.rs/asic-rs/0.1.6/asic_rs/?search=miners::backends::traits::GetMinerData
 [__link4]: https://docs.rs/asic-rs/0.1.6/asic_rs/?search=miners::backends::traits::HasMinerControl
 [__link5]: https://www.conventionalcommits.org/

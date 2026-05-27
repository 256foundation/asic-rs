asic-rs is an async miner management and control library for ASIC miners.
It provides one set of concepts across Rust and Python: a factory discovers
miners, a miner object gathers data and performs supported control operations,
and shared data/config models describe the result.

The Rust crate is published as `asic-rs`. The Python bindings are published as
`pyasic_rs` and expose the same high-level API through PyO3 classes and
Pydantic-compatible data models.

## API Map

| Concept | Rust | Python |
| --- | --- | --- |
| Discovery and miner construction | [`MinerFactory`][minerfactory] | `pyasic_rs.MinerFactory` |
| Miner handle | `Box<dyn Miner>` | `pyasic_rs.Miner` |
| Full telemetry snapshot | `MinerData` | `pyasic_rs.data.MinerData` |
| Hashrate values | `HashRate`, `HashRateUnit` | `HashRate`, `HashRateUnit` |
| Pool configuration | `PoolGroupConfig`, `PoolConfig` | `PoolGroup`, `Pool` |
| Fan configuration | `FanConfig` | `FanConfig` |
| Tuning configuration | `TuningConfig` | `TuningConfig` |
| Optional controls/configs | `supports_*` methods | `supports_*` properties |

All network operations are asynchronous. Rust methods generally return
`Result<T>` and use `Option<T>` when a miner does not expose a value. Python
methods are awaitable and return the Python equivalent, using `None` for missing
or unsupported values.

## Examples

The paired examples below use stable markers so documentation tools can render
Rust and Python snippets as language tabs while GitHub, PyPI, and docs.rs still
show both examples plainly.

### Get One Miner

If the miner IP is known, ask `MinerFactory` to identify the firmware and build
the correct miner implementation.

<!-- asic-rs-example:get-miner rust -->

```rust,no_run
use asic_rs::MinerFactory;
use std::{net::IpAddr, str::FromStr};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let factory = MinerFactory::new();
    let ip = IpAddr::from_str("192.168.1.10")?;

    if let Some(miner) = factory.get_miner(ip).await? {
        println!("Found {} {} at {}", miner.get_device_info().make, miner.get_device_info().model, ip);
    }

    Ok(())
}
```

<!-- asic-rs-example:get-miner python -->

```python
import asyncio

from pyasic_rs import MinerFactory


async def main() -> None:
    factory = MinerFactory()
    miner = await factory.get_miner("192.168.1.10")

    if miner is not None:
        print(f"Found {miner.make} {miner.model} at {miner.ip}")


if __name__ == "__main__":
    asyncio.run(main())
```

### Scan A Network

When the exact IP is not known, add a subnet, octet range, or range string to
the factory and scan it. Large scans automatically use bounded concurrency.

<!-- asic-rs-example:scan rust -->

```rust,no_run
use asic_rs::MinerFactory;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let miners = MinerFactory::from_subnet("192.168.1.0/24")?
        .with_concurrent_limit(2500)
        .scan()
        .await?;

    println!("Found {} miner(s)", miners.len());
    Ok(())
}
```

<!-- asic-rs-example:scan python -->

```python
import asyncio

from pyasic_rs import MinerFactory


async def main() -> None:
    miners = await (
        MinerFactory.from_subnet("192.168.1.0/24")
        .with_concurrent_limit(2500)
        .scan()
    )

    print(f"Found {len(miners)} miner(s)")


if __name__ == "__main__":
    asyncio.run(main())
```

Other range constructors are available in both languages:

<!-- asic-rs-example:ranges rust -->

```rust,no_run
# use asic_rs::MinerFactory;
# fn main() -> anyhow::Result<()> {
let by_octets = MinerFactory::from_octets("192", "168", "1", "1-255")?;
let by_range = MinerFactory::from_range("192.168.1.1-255")?;
# let _ = (by_octets, by_range);
# Ok(())
# }
```

<!-- asic-rs-example:ranges python -->

```python
from pyasic_rs import MinerFactory

by_octets = MinerFactory.from_octets("192", "168", "1", "1-255")
by_range = MinerFactory.from_range("192.168.1.1-255")
```

### Stream Scan Results

Use streaming scans when you want to act on miners as soon as they are found
instead of waiting for the whole scan to finish.

<!-- asic-rs-example:scan-stream rust -->

```rust,no_run
use asic_rs::MinerFactory;
use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut stream = MinerFactory::from_subnet("192.168.1.0/24")?.scan_stream();

    while let Some(miner) = stream.next().await {
        println!("{} {}", miner.get_device_info().make, miner.get_device_info().model);
    }

    Ok(())
}
```

<!-- asic-rs-example:scan-stream python -->

```python
import asyncio

from pyasic_rs import MinerFactory


async def main() -> None:
    factory = MinerFactory.from_subnet("192.168.1.0/24")

    async for miner in factory.scan_stream():
        print(f"{miner.make} {miner.model}")


if __name__ == "__main__":
    asyncio.run(main())
```

### Gather Data

`get_data` returns a full `MinerData` snapshot. Individual `get_*` calls are
available when only one field is needed.

<!-- asic-rs-example:data rust -->

```rust,no_run
use asic_rs::MinerFactory;
use std::{net::IpAddr, str::FromStr};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let factory = MinerFactory::new();
    let ip = IpAddr::from_str("192.168.1.10")?;

    if let Some(miner) = factory.get_miner(ip).await? {
        let data = miner.get_data().await;
        let mac = miner.get_mac().await;

        println!("{} is mining: {}", data.ip, data.is_mining);
        println!("MAC: {mac:?}");
    }

    Ok(())
}
```

<!-- asic-rs-example:data python -->

```python
import asyncio

from pyasic_rs import MinerFactory


async def main() -> None:
    miner = await MinerFactory().get_miner("192.168.1.10")
    if miner is None:
        return

    data = await miner.get_data()
    mac = await miner.get_mac()

    print(f"{data.ip} is mining: {data.is_mining}")
    print(f"MAC: {mac}")


if __name__ == "__main__":
    asyncio.run(main())
```

To reduce collection work, exclude fields from a full data snapshot.

<!-- asic-rs-example:data-exclude rust -->

```rust,no_run
# use asic_rs::MinerFactory;
use asic_rs::core::data::collector::DataField;
# use std::{net::IpAddr, str::FromStr};
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let factory = MinerFactory::new();
# let ip = IpAddr::from_str("192.168.1.10")?;
# if let Some(miner) = factory.get_miner(ip).await? {
let data = miner
    .get_data_filtered(vec![DataField::Hashboards, DataField::Chips])
    .await;
# let _ = data;
# }
# Ok(())
# }
```

<!-- asic-rs-example:data-exclude python -->

```python
from pyasic_rs.data import DataField

data = await miner.get_data(exclude=[DataField.Hashboards, DataField.Chips])
```

### Authentication

Backends use their built-in default credentials unless you override them.
Set credentials before starting other operations on that miner.

<!-- asic-rs-example:auth rust -->

```rust,no_run
use asic_rs::MinerFactory;
use asic_rs::core::traits::auth::MinerAuth;
use std::{net::IpAddr, str::FromStr};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let factory = MinerFactory::new();
    let ip = IpAddr::from_str("192.168.1.10")?;

    if let Some(mut miner) = factory.get_miner(ip).await? {
        miner.set_auth(MinerAuth::new("admin", "secret"));
        let data = miner.get_data().await;
        println!("{:?}", data.hashrate);
    }

    Ok(())
}
```

<!-- asic-rs-example:auth python -->

```python
miner = await MinerFactory().get_miner("192.168.1.10")
if miner is not None:
    miner.set_auth("admin", "secret")
    data = await miner.get_data()
```

### Control A Miner

Control support depends on the miner and firmware. Check the matching
`supports_*` value before issuing a control command in user-facing tools.

<!-- asic-rs-example:control rust -->

```rust,no_run
# use asic_rs::MinerFactory;
# use std::{net::IpAddr, str::FromStr};
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let factory = MinerFactory::new();
# let ip = IpAddr::from_str("192.168.1.10")?;
# if let Some(miner) = factory.get_miner(ip).await? {
if miner.supports_restart() {
    let restarted = miner.restart().await?;
    println!("Restart accepted: {restarted}");
}
# }
# Ok(())
# }
```

<!-- asic-rs-example:control python -->

```python
if miner.supports_restart:
    restarted = await miner.restart()
    print(f"Restart accepted: {restarted}")
```

### Configure Pools, Fans, And Tuning

Configuration methods follow the same support pattern as controls. The Python
models are Pydantic-compatible, so they can be validated, dumped, and embedded
in your own Pydantic models.

<!-- asic-rs-example:config rust -->

```rust,no_run
# use asic_rs::MinerFactory;
use asic_rs::core::config::{
    fan::FanConfig,
    pools::{PoolConfig, PoolGroupConfig},
    tuning::TuningConfig,
};
use asic_rs::core::data::{miner::TuningTarget, pool::PoolURL};
# use std::{net::IpAddr, str::FromStr};
# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
# let factory = MinerFactory::new();
# let ip = IpAddr::from_str("192.168.1.10")?;
# if let Some(miner) = factory.get_miner(ip).await? {
if miner.supports_pools_config() {
    let group = PoolGroupConfig {
        name: "default".to_string(),
        quota: 1,
        pools: vec![PoolConfig {
            url: PoolURL::from("stratum+tcp://pool.example.com:3333".to_string()),
            username: "worker.1".to_string(),
            password: "x".to_string(),
        }],
    };
    miner.set_pools_config(vec![group]).await?;
}

if miner.supports_fan_config() {
    miner.set_fan_config(FanConfig::manual(80)).await?;
}

if miner.supports_tuning_config() {
    let config = TuningConfig::new(TuningTarget::from_watts(3200.0));
    miner.set_tuning_config(config, None).await?;
}
# }
# Ok(())
# }
```

<!-- asic-rs-example:config python -->

```python
from pyasic_rs.config import FanConfig, Pool, PoolGroup, TuningConfig

if miner.supports_pools_config:
    group = PoolGroup(
        name="default",
        quota=1,
        pools=[
            Pool(
                url="stratum+tcp://pool.example.com:3333",
                username="worker.1",
                password="x",
            )
        ],
    )
    await miner.set_pools_config([group])

if miner.supports_fan_config:
    await miner.set_fan_config(FanConfig.manual(80))

if miner.supports_tuning_config:
    await miner.set_tuning_config(TuningConfig.power(3200.0))
```

## Python Data Models

Python data/config classes are backed by Rust structs and implement a
Pydantic-style surface:

```python
from pydantic import BaseModel

from pyasic_rs.data import HashRate


class Snapshot(BaseModel):
    hashrate: HashRate


snapshot = Snapshot.model_validate(
    {"hashrate": {"value": 100.0, "unit": "TH/s", "algo": "SHA256"}}
)
print(snapshot.model_dump())
```

Use `model_validate`, `model_dump`, and `model_json_schema` on supported model
classes when integrating with Python validation or API layers.

[minerfactory]: https://docs.rs/asic-rs/latest/asic_rs/struct.MinerFactory.html

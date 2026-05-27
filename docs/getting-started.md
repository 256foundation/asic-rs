# Getting Started

All network operations are asynchronous. Rust uses `async` methods returning
`Result<T>` where an operation can fail. Python exposes awaitable methods and
uses `None` for values or operations that are unavailable for a miner.

## Get One Miner

If you know a miner's IP address, let the factory identify the firmware and
construct the matching miner implementation.

=== "Rust"

    ```rust
    use asic_rs::MinerFactory;
    use std::{net::IpAddr, str::FromStr};

    #[tokio::main]
    async fn main() -> anyhow::Result<()> {
        let factory = MinerFactory::new();
        let ip = IpAddr::from_str("192.168.1.10")?;

        if let Some(miner) = factory.get_miner(ip).await? {
            println!(
                "Found {} {} at {}",
                miner.get_device_info().make,
                miner.get_device_info().model,
                ip
            );
        }

        Ok(())
    }
    ```

=== "Python"

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

## Scan A Network

Use a subnet, octet selectors, or range string when the exact IP address is not
known. Large scans use bounded concurrency.

=== "Rust"

    ```rust
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

=== "Python"

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

Range helpers are available in both languages.

=== "Rust"

    ```rust
    let by_octets = MinerFactory::from_octets("192", "168", "1", "1-255")?;
    let by_range = MinerFactory::from_range("192.168.1.1-255")?;
    ```

=== "Python"

    ```python
    by_octets = MinerFactory.from_octets("192", "168", "1", "1-255")
    by_range = MinerFactory.from_range("192.168.1.1-255")
    ```

## Stream Results

Streaming scans let you act on miners as soon as they are found.

=== "Rust"

    ```rust
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

=== "Python"

    ```python
    factory = MinerFactory.from_subnet("192.168.1.0/24")

    async for miner in factory.scan_stream():
        print(f"{miner.make} {miner.model}")
    ```

Use the IP-preserving stream when you need to track unsupported or offline
addresses.

=== "Rust"

    ```rust
    let mut stream = MinerFactory::from_subnet("192.168.1.0/24")?.scan_stream_with_ip();

    while let Some((ip, miner)) = stream.next().await {
        println!("{ip}: {}", miner.is_some());
    }
    ```

=== "Python"

    ```python
    async for ip, miner in factory.scan_stream_with_ip():
        print(ip, miner is not None)
    ```

## Gather Data

`get_data()` returns a full standardized telemetry snapshot. Focused `get_*`
methods are useful when you only need one field.

=== "Rust"

    ```rust
    let data = miner.get_data().await;
    let mac = miner.get_mac().await;

    println!("{} is mining: {}", data.ip, data.is_mining);
    println!("MAC: {mac:?}");
    ```

=== "Python"

    ```python
    data = await miner.get_data()
    mac = await miner.get_mac()

    print(f"{data.ip} is mining: {data.is_mining}")
    print(f"MAC: {mac}")
    ```

Skip expensive fields when you do not need them.

=== "Rust"

    ```rust
    use asic_rs::core::data::collector::DataField;

    let data = miner
        .get_data_filtered(vec![DataField::Hashboards, DataField::Chips])
        .await;
    ```

=== "Python"

    ```python
    from pyasic_rs.data import DataField

    data = await miner.get_data(exclude=[DataField.Hashboards, DataField.Chips])
    ```

## Authenticate

Backends use their built-in default credentials unless you override them. Set
credentials before starting concurrent operations on that miner handle.

=== "Rust"

    ```rust
    use asic_rs::core::traits::auth::MinerAuth;

    miner.set_auth(MinerAuth::new("admin", "secret"));
    let data = miner.get_data().await;
    ```

=== "Python"

    ```python
    miner.set_auth("admin", "secret")
    data = await miner.get_data()
    ```

## Control A Miner

Control support depends on miner make, model, and firmware. Check the matching
support value before exposing controls in user-facing tools.

=== "Rust"

    ```rust
    if miner.supports_restart() {
        let restarted = miner.restart().await?;
        println!("Restart accepted: {restarted}");
    }
    ```

=== "Python"

    ```python
    if miner.supports_restart:
        restarted = await miner.restart()
        print(f"Restart accepted: {restarted}")
    ```

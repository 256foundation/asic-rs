# API Guide

The Rust and Python APIs intentionally share names and behavior. This page
summarizes the user-facing surface and points out the few language-specific
differences.

## Discovery

`MinerFactory` owns the scan range and discovery tuning.

=== "Rust"

    ```rust
    let factory = MinerFactory::from_subnet("192.168.1.0/24")?
        .with_concurrent_limit(2500)
        .with_connectivity_timeout_secs(1)
        .with_identification_timeout_secs(10);
    ```

=== "Python"

    ```python
    factory = (
        MinerFactory.from_subnet("192.168.1.0/24")
        .with_concurrent_limit(2500)
        .with_connectivity_timeout_secs(1)
        .with_identification_timeout_secs(10)
    )
    ```

| Operation | Rust | Python |
| --- | --- | --- |
| Known IP | `get_miner(ip).await?` | `await get_miner(ip)` |
| Full scan | `scan().await?` | `await scan()` |
| Stream found miners | `scan_stream()` | `scan_stream()` |
| Stream every IP | `scan_stream_with_ip()` | `scan_stream_with_ip()` |

## Miner Identity

Miner identity is available without awaiting because it is known when the miner
handle is constructed.

| Value | Rust | Python |
| --- | --- | --- |
| IP address | `miner.get_ip()` | `miner.ip` |
| Make | `miner.get_device_info().make` | `miner.make` |
| Model | `miner.get_device_info().model` | `miner.model` |
| Firmware | `miner.get_device_info().firmware` | `miner.firmware` |
| Algorithm | `miner.get_device_info().algo` | `miner.algo` |
| Hardware shape | `miner.get_device_info().hardware` | `miner.hardware` |

## Data Collection

The full telemetry snapshot is `MinerData`. Individual getters return focused
fields when a caller does not need the whole snapshot.

=== "Rust"

    ```rust
    let data = miner.get_data().await;
    let hashrate = miner.get_hashrate().await;
    let fans = miner.get_fans().await;
    ```

=== "Python"

    ```python
    data = await miner.get_data()
    hashrate = await miner.get_hashrate()
    fans = await miner.get_fans()
    ```

Common telemetry methods:

| Field | Method |
| --- | --- |
| MAC address | `get_mac` |
| Serial number | `get_serial_number` |
| Hostname | `get_hostname` |
| Firmware version | `get_firmware_version` |
| Hashboards | `get_hashboards` |
| Hashrate | `get_hashrate` |
| Fans | `get_fans` |
| Wattage | `get_wattage` |
| Messages | `get_messages` |
| Pools | `get_pools` |
| Mining state | `get_is_mining` |

## Controls And Capability Checks

Not every miner supports every control. Rust exposes `supports_*()` methods;
Python exposes matching `supports_*` properties.

| Capability | Control |
| --- | --- |
| `supports_restart` | `restart()` |
| `supports_pause` | `pause()` |
| `supports_resume` | `resume()` |
| `supports_set_fault_light` | `set_fault_light(...)` |
| `supports_set_power_limit` | `set_power_limit(...)` |
| `supports_change_password` | `change_password(...)` |
| `supports_read_logs` | `read_logs()` |
| `supports_factory_reset` | `factory_reset()` |
| `supports_upgrade_firmware` | `upgrade_firmware(...)` |

=== "Rust"

    ```rust
    if miner.supports_set_power_limit() {
        miner.set_power_limit(measurements::Power::from_watts(3200.0)).await?;
    }
    ```

=== "Python"

    ```python
    if miner.supports_set_power_limit:
        await miner.set_power_limit(3200.0)
    ```

## Configuration Models

Configuration objects are shared concepts across Rust and Python. Python models
are backed by Rust structs and expose Pydantic-style helpers.

=== "Rust"

    ```rust
    use asic_rs::core::config::{
        fan::FanConfig,
        pools::{PoolConfig, PoolGroupConfig},
        tuning::TuningConfig,
    };
    use asic_rs::core::data::{miner::TuningTarget, pool::PoolURL};

    let fan = FanConfig::manual(80);
    let tuning = TuningConfig::new(TuningTarget::from_watts(3200.0));
    let pool = PoolGroupConfig {
        name: "default".to_string(),
        quota: 1,
        pools: vec![PoolConfig {
            url: PoolURL::from("stratum+tcp://pool.example.com:3333".to_string()),
            username: "worker.1".to_string(),
            password: "x".to_string(),
        }],
    };
    ```

=== "Python"

    ```python
    from pyasic_rs.config import FanConfig, Pool, PoolGroup, TuningConfig

    fan = FanConfig.manual(80)
    tuning = TuningConfig.power(3200.0)
    pool = PoolGroup(
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
    ```

Use the matching config support property before reading or writing config:

| Capability | Get | Set |
| --- | --- | --- |
| `supports_pools_config` | `get_pools_config()` | `set_pools_config(...)` |
| `supports_fan_config` | `get_fan_config()` | `set_fan_config(...)` |
| `supports_tuning_config` | `get_tuning_config()` | `set_tuning_config(...)` |
| `supports_scaling_config` | `get_scaling_config()` | `set_scaling_config(...)` |

## Python Pydantic Interop

Python data/config classes can be used inside Pydantic models and support
`model_validate`, `model_dump`, and `model_json_schema` where applicable.

=== "Python"

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

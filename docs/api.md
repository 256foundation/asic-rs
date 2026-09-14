# API Guide

The Rust, Python, and Go APIs intentionally share names and behavior. This page
summarizes the user-facing surface and points out the few language-specific
differences. Go methods are synchronous and return `error`; a missing miner is
`asic_go.ErrNotFound`. See `go/README.md` for cgo build notes.

## Discovery

`MinerFactory` owns the scan range and discovery tuning. Connectivity retries
are additional attempts after the initial probe. Setting retries to zero still
performs one probe; later attempts use bounded exponential backoff and remain
under the same scan concurrency limit. The default remains three retries;
callers that want one pass can explicitly set zero.

Common miner ports are probed concurrently, returning after the first success.
The scan concurrency limit also bounds the total number of active TCP probes,
including probes made by connectivity retries.

The identification timeout is an end-to-end deadline covering discovery
commands and firmware-specific miner construction. Discovery HTTP clients also
apply explicit connection and total-request deadlines.

=== "Rust"

    ```rust
    let factory = MinerFactory::from_subnet("192.168.1.0/24")?
        .with_concurrent_limit(2500)
        .with_connectivity_timeout_secs(1)
        .with_connectivity_retries(0)
        .with_identification_timeout_secs(10);
    ```

=== "Python"

    ```python
    factory = (
        MinerFactory.from_subnet("192.168.1.0/24")
        .with_concurrent_limit(2500)
        .with_connectivity_timeout_secs(1)
        .with_connectivity_retries(0)
        .with_identification_timeout_secs(10)
    )
    ```

=== "Go"

    ```go
    factory, err := asic_go.NewMinerFactoryFromSubnet("192.168.1.0/24")
    if err != nil {
        log.Fatal(err)
    }
    factory.WithConcurrentLimit(2500).
        WithConnectivityTimeoutSecs(1).
        WithConnectivityRetries(0).
        WithIdentificationTimeoutSecs(10)
    ```

| Operation | Rust | Python | Go |
| --- | --- | --- | --- |
| Known IP | `get_miner(ip).await?` | `await get_miner(ip)` | `GetMiner(ip)` |
| Full scan | `scan().await?` | `await scan()` | `Scan()` |
| Stream found miners | `scan_stream()` | `scan_stream()` | not wrapped yet |
| Stream every IP | `scan_stream_with_ip()` | `scan_stream_with_ip()` | not wrapped yet |

## Miner Identity

Miner identity is available without awaiting because it is known when the miner
handle is constructed.

In Go, retrieve the identity and handle its error before reading fields:

```go
info, err := miner.GetDeviceInfo()
if err != nil {
    log.Fatal(err)
}
```

| Value | Rust | Python | Go |
| --- | --- | --- | --- |
| IP address | `miner.get_ip()` | `miner.ip` | `miner.GetIP()` |
| Make | `miner.get_device_info().make` | `miner.make` | `info.Make` |
| Model | `miner.get_device_info().model` | `miner.model` | `info.Model` |
| Firmware | `miner.get_device_info().firmware` | `miner.firmware` | `info.Firmware` |
| Algorithm | `miner.get_device_info().algo` | `miner.algo` | `info.Algo` |
| Hardware shape | `miner.get_device_info().hardware` | `miner.hardware` | `info.Hardware` |

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

=== "Go"

    ```go
    data, err := miner.GetData()
    if err != nil {
        log.Fatal(err)
    }
    hashrate, err := miner.GetHashrate()
    fans, err := miner.GetFans()
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
| Best share difficulty | `get_best_share` |
| Session best share difficulty | `get_session_best_share` |
| Messages | `get_messages` |
| Pools | `get_pools` |
| Mining state | `get_is_mining` |

## Controls And Capability Checks

Not every miner supports every control. Rust exposes `supports_*()` methods;
Python exposes matching `supports_*` properties; Go returns them from
`Supports()`.

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
| `supports_prepare_firmware` | `prepare_firmware(...)` |
| `supports_upgrade_firmware` | `upgrade_firmware(...)` |

All miner handles also expose `revalidate()`, which re-runs that backend's
firmware discovery checks against the same IP and returns whether the device is
still valid for the existing miner handle.

`prepare_firmware(...)` is read-only. It validates backend-specific containers
and may query miner metadata to select a compatible payload, but it never starts
an upload. The returned `FirmwareImage` contains the filename and bytes that an
upgrade would use.

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

=== "Go"

    ```go
    caps, err := miner.Supports()
    if err != nil {
        log.Fatal(err)
    }
    if caps.SetPowerLimit {
        _, err := miner.SetPowerLimit(3200.0)
        if err != nil {
            log.Fatal(err)
        }
    }
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

## Binding Names And Wire Formats

Shared type names are retained in Go: `MinerFactory`, `DeviceInfo`,
`MinerHardware`, `HashAlgorithm`, `MiningMode`, `MinerControlBoard`,
`MinerComponent`, and the tuning-capability models. Go exports names in
PascalCase, retaining initialisms (`IP`, `MAC`, `API`, `PSU`). Identity and
telemetry getters consistently use `Get…`; constructors use `New…` followed
by the type and, where needed, the variant.

Python additionally exports `PoolConfig` and `PoolGroupConfig` as aliases for
its existing `Pool` and `PoolGroup` classes. `MinerHardware.total_chips` is the
shared count name; Python's `chips` property remains an alias. Go exposes
`TotalChips`, `BoardCount`, and `ChipsForBoard`, returning a value and a boolean
that distinguishes missing counts from zero.

Go tuning targets expose `Variant`, `Watts`, `TargetHashrate`, `TargetMode`,
`PresetName`, and `Boards`. These correspond to Python's `variant`, `watts`,
`target_hashrate`, `target_mode`, `preset_name`, and `boards`. `ManualTuningValues`
uses numeric board IDs in Go, Rust, and Python. The complete Go constructor set
is `NewTuningTargetManual`, `NewTuningTargetPower`, `NewTuningTargetHashrate`,
`NewTuningTargetMiningMode`, and `NewTuningTargetPreset`.

Some established Python representations differ from Rust's Serde schema.
The C bridge and Go JSON output retain the Rust schema, also described by the
Rust TypeScript derives; they do not rename existing wire fields.

| Value | Rust / C / Go JSON output | Existing Python representation |
| --- | --- | --- |
| Hashrate unit | `"TeraHash"` | `"TH/s"` |
| Pool scheme | `"StratumV1"` | `"stratum+tcp"` |
| Pool URL | Object containing scheme/host/port/pubkey | URL string |
| Fan mode | `"Auto"` / `"Manual"` | `"auto"` / `"manual"` |
| Power target | `{"Power":{"watts":3200}}` | `{"type":"power","value":3200}` |
| Mining-mode target | `{"MiningMode":"High"}` | `{"type":"mode","value":"High"}` |

Go accepts these Python input forms for hash units, pool schemes/URLs, fan modes,
and tuning targets, then emits canonical Rust JSON. Invalid hash units and
algorithms return errors instead of silently becoming hashes per second or
SHA-256. `HashRate.DefaultUnit` and `IntoDefaultUnit` use the algorithm-specific
unit, matching Rust/Python.

`Supports()` and `Expected()` remain Go conveniences for grouped results;
individual `GetExpectedHashboards`, `GetExpectedChips`, and `GetExpectedFans`
methods also match the shared API. Go's `GetControlBoardVersion` and the snapshot
both preserve `{known, name}`; Python's established individual getter continues
to return a display string. Go firmware checks and revalidation propagate backend
errors, whereas the existing Python methods return `None` for those failures.

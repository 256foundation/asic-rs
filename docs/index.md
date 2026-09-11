![Logo](logo/light/logo.svg#only-light){ width="150" }
![Logo](logo/dark/logo.svg#only-dark){ width="150" }
/// caption
///

# asic-rs

asic-rs is an async miner management and control library for ASIC miners. It
keeps the Rust crate, Python bindings, and Go bindings aligned around the same
concepts: discover miners, gather standardized telemetry, and run supported
controls.

=== "Rust"

    ```rust
    use asic_rs::MinerFactory;
    ```

    Use the `asic-rs` crate when building native Rust services, daemons, and
    tooling.

=== "Python"

    ```python
    from pyasic_rs import MinerFactory
    ```

    Use `pyasic_rs` when integrating miner management into Python automation,
    data pipelines, or API services.

=== "Go"

    ```go
    import "github.com/256foundation/asic-rs/go/asicrs"
    ```

    Use the in-tree Go module when integrating miner management into Go
    services. See `go/README.md` for cgo packaging.

## One API Shape

| Concept | Rust | Python | Go |
| --- | --- | --- | --- |
| Discovery | `MinerFactory` | `MinerFactory` | `asicrs.Factory` |
| Miner handle | `Box<dyn Miner>` | `Miner` | `asicrs.Miner` |
| Full telemetry | `MinerData` | `pyasic_rs.data.MinerData` | `asicrs.MinerData` |
| Pool config | `PoolGroupConfig`, `PoolConfig` | `PoolGroup`, `Pool` | `PoolGroupConfig`, `PoolConfig` |
| Fan config | `FanConfig` | `FanConfig` | `FanConfig` |
| Tuning config | `TuningConfig` | `TuningConfig` | `TuningConfig` |

The bindings are not a separate design. Python methods mirror the Rust surface,
with Python-native conventions where appropriate: awaitables for async work,
`None` for missing optional values, and Pydantic-compatible model helpers for
data/config objects. Go methods are synchronous and return `error`; a missing
miner is `asicrs.ErrNotFound`.

## Common Workflow

1. Build a `MinerFactory`.
2. Identify one miner by IP or scan a range.
3. Read telemetry with `get_data()` or focused `get_*` calls.
4. Check `supports_*` before optional controls.
5. Apply supported control or configuration changes.

Continue with [Getting started](getting-started.md) for paired Rust, Python, and
Go examples.

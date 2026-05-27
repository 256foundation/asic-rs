# asic-rs

asic-rs is an async miner management and control library for ASIC miners. It
keeps the Rust crate and Python bindings aligned around the same concepts:
discover miners, gather standardized telemetry, and run supported controls.

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

## One API Shape

| Concept | Rust | Python |
| --- | --- | --- |
| Discovery | `MinerFactory` | `MinerFactory` |
| Miner handle | `Box<dyn Miner>` | `Miner` |
| Full telemetry | `MinerData` | `pyasic_rs.data.MinerData` |
| Pool config | `PoolGroupConfig`, `PoolConfig` | `PoolGroup`, `Pool` |
| Fan config | `FanConfig` | `FanConfig` |
| Tuning config | `TuningConfig` | `TuningConfig` |

The bindings are not a separate design. Python methods mirror the Rust surface,
with Python-native conventions where appropriate: awaitables for async work,
`None` for missing optional values, and Pydantic-compatible model helpers for
data/config objects.

## Common Workflow

1. Build a `MinerFactory`.
2. Identify one miner by IP or scan a range.
3. Read telemetry with `get_data()` or focused `get_*` calls.
4. Check `supports_*` before optional controls.
5. Apply supported control or configuration changes.

Continue with [Getting started](getting-started.md) for paired Rust and Python
examples.

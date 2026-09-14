# Go bindings

Go bindings for asic-rs. The public package is `github.com/256foundation/asic-rs/go/asic_go`.

Complex values (`MinerData`, configs) cross a small C ABI as JSON. Async work
runs inside an in-process Tokio runtime, so the Go API is synchronous. Always
`Close()` factory and miner handles (finalizers are a safety net only).

## Requirements

- Go 1.23+
- Rust stable (`cargo`, `rustc`) to build `asic-rs-ffi`
- A C toolchain (Xcode CLT on macOS, `build-essential` on Linux)
- `CGO_ENABLED=1`

## Build and test

```sh
make -C go ffi
make -C go test
```

The build installs the artifact paths reported by Cargo, honoring `CARGO_TARGET_DIR`
and Cargo target-directory/target configuration.

Optional examples, run from the repository root (need a miner on the network):

```sh
ASIC_MINER_IP=192.168.1.42 go -C go run ./examples/get_data
ASIC_SUBNET=192.168.1.0/24 go -C go run ./examples/scan
```

## Use as a dependency

```sh
go get github.com/256foundation/asic-rs/go/asic_go@latest
```

`go get` pulls Go sources, not `libasic_rs_ffi`. Build the native library from
this repository (`make -C go ffi`) and point cgo at `go/asic_go/{include,lib}`,
or copy those artifacts into your module.

The default cgo directives look for libraries under `asic_go/lib` relative to
the package source (`${SRCDIR}/lib`) and set an rpath on macOS/Linux.

## API shape

| Concept | Go |
| --- | --- |
| Discovery | `asic_go.MinerFactory` |
| Miner handle | `asic_go.Miner` |
| Telemetry | `asic_go.MinerData` |
| Missing miner | `errors.Is(err, asic_go.ErrNotFound)` |

`GetMiner` returns `ErrNotFound` when the address is not a supported miner,
matching Rust `Ok(None)` and Python `None`. Check `Supports()` before optional
control and config calls.

Streaming scans, `MinerListener`, and `prepare_firmware` are not wrapped yet;
use `Scan()` / `GetMiner`. `GetOperatingState` is wrapped.

## Shared names

Use `NewMinerFactory`, `GetIP`, and `GetDeviceInfo` for discovery and identity.
Config constructors include `NewPoolConfig`, `NewFanConfigAuto`, and
`NewFanConfigManual`. Tuning constructors use `NewTuningTarget` followed by
`Manual`, `Power`, `Hashrate`, `MiningMode`, or `Preset`.

The Go models retain the Rust type names, including `HashAlgorithm`,
`MiningMode`, `MinerControlBoard`, `MinerComponent`, and `TuningCapabilities`.
Hashrate conversions return an error for invalid units. For a display value in
the conventional unit for its algorithm, use `HashRate.IntoDefaultUnit()`.
See [the API guide](../docs/api.md#binding-names-and-wire-formats) for the naming
map, Python compatibility aliases, and JSON format differences.

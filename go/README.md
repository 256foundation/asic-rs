# Go bindings

Go bindings for asic-rs. The public package is `github.com/256foundation/asic-rs/go/asicrs`.

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

Optional examples (need a miner on the network):

```sh
ASIC_MINER_IP=192.168.1.42 go run ./examples/get_data
ASIC_SUBNET=192.168.1.0/24 go run ./examples/scan
```

## Use as a dependency

```sh
go get github.com/256foundation/asic-rs/go/asicrs@latest
```

`go get` pulls Go sources, not `libasic_rs_ffi`. Build the native library from
this repository (`make -C go ffi`) and point cgo at `go/asicrs/{include,lib}`,
or copy those artifacts into your module.

The default cgo directives look for libraries under `asicrs/lib` relative to
the package source (`${SRCDIR}/lib`) and set an rpath on macOS/Linux.

## API shape

| Concept | Go |
| --- | --- |
| Discovery | `asicrs.Factory` |
| Miner handle | `asicrs.Miner` |
| Telemetry | `asicrs.MinerData` |
| Missing miner | `errors.Is(err, asicrs.ErrNotFound)` |

`GetMiner` returns `ErrNotFound` when the address is not a supported miner,
matching Rust `Ok(None)` and Python `None`. Check `Supports()` before optional
control and config calls.

Streaming scans, `MinerListener`, and `prepare_firmware` are not wrapped yet;
use `Scan()` / `GetMiner`. `GetOperatingState` is wrapped.

// Package asic_go is the Go binding for [asic-rs], an ASIC miner management library.
//
// It wraps a small in-tree Rust FFI crate (asic-rs-ffi) that exposes asic-rs over a
// C ABI. Complex values (MinerData, configs) cross the boundary as JSON. Async
// work is handled inside Rust via a shared Tokio runtime, so Go callers see
// ordinary synchronous methods.
//
// # Quick start
//
//	factory := asic_go.NewFactory()
//	defer factory.Close()
//
//	miner, err := factory.GetMiner("192.168.1.42")
//	if errors.Is(err, asic_go.ErrNotFound) {
//	    return
//	}
//	if err != nil {
//	    log.Fatal(err)
//	}
//	defer miner.Close()
//
//	data, err := miner.GetData()
//	if err != nil {
//	    log.Fatal(err)
//	}
//	hashrate, err := data.HashrateTH()
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Printf("%s @ %s: %.2f TH/s\n", data.DeviceInfo.Model, data.IP, hashrate)
//
// # Building
//
// This package uses cgo and requires a prebuilt libasic_rs_ffi shared library
// in asic_go/lib (produced by `make -C go ffi`). See go/README.md for packaging
// this into larger Go projects.
//
// [asic-rs]: https://github.com/256foundation/asic-rs
package asic_go

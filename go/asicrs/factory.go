package asicrs

/*
#include "asic_rs_ffi.h"
#include <stdlib.h>
*/
import "C"

import (
	"fmt"
	"runtime"
	"sync"
	"unsafe"
)

// Factory discovers and constructs miners on a network.
//
// Always call Close when finished (or use defer). Close must not run
// concurrently with other methods on the same Factory.
type Factory struct {
	mu  sync.Mutex
	ptr *C.AsicFactory
}

func newFactory(ptr *C.AsicFactory) *Factory {
	f := &Factory{ptr: ptr}
	runtime.SetFinalizer(f, (*Factory).Close)
	return f
}

// NewFactory creates an empty factory with no hosts configured.
func NewFactory() *Factory {
	return newFactory(C.asic_rs_factory_new())
}

func newFactoryFromC(ptr *C.AsicFactory) (*Factory, error) {
	if ptr == nil {
		return nil, lastError()
	}
	return newFactory(ptr), nil
}

// NewFactoryFromSubnet creates a factory pre-loaded with hosts from a CIDR
// subnet (for example "192.168.1.0/24").
func NewFactoryFromSubnet(subnet string) (*Factory, error) {
	cs, err := cString(subnet)
	if err != nil {
		return nil, err
	}
	defer freeGoCString(cs)
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	return newFactoryFromC(C.asic_rs_factory_from_subnet(cs))
}

// NewFactoryFromRange creates a factory from a compact range string
// (for example "192.168.1.1-255").
func NewFactoryFromRange(rangeStr string) (*Factory, error) {
	cs, err := cString(rangeStr)
	if err != nil {
		return nil, err
	}
	defer freeGoCString(cs)
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	return newFactoryFromC(C.asic_rs_factory_from_range(cs))
}

// NewFactoryFromOctets creates a factory from four octet descriptors
// (for example "192", "168", "1", "1-255").
func NewFactoryFromOctets(o1, o2, o3, o4 string) (*Factory, error) {
	c1, err := cString(o1)
	if err != nil {
		return nil, err
	}
	c2, err := cString(o2)
	if err != nil {
		freeGoCString(c1)
		return nil, err
	}
	c3, err := cString(o3)
	if err != nil {
		freeGoCString(c1)
		freeGoCString(c2)
		return nil, err
	}
	c4, err := cString(o4)
	if err != nil {
		freeGoCString(c1)
		freeGoCString(c2)
		freeGoCString(c3)
		return nil, err
	}
	defer freeGoCString(c1)
	defer freeGoCString(c2)
	defer freeGoCString(c3)
	defer freeGoCString(c4)
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	return newFactoryFromC(C.asic_rs_factory_from_octets(c1, c2, c3, c4))
}

// Close frees the factory. Safe to call multiple times. Do not call Close
// concurrently with other methods on the same Factory.
func (f *Factory) Close() {
	if f == nil {
		return
	}
	f.mu.Lock()
	defer f.mu.Unlock()
	if f.ptr == nil {
		return
	}
	runtime.LockOSThread()
	C.asic_rs_factory_free(f.ptr)
	runtime.UnlockOSThread()
	f.ptr = nil
	runtime.SetFinalizer(f, nil)
}

func (f *Factory) withLive(fn func(ptr *C.AsicFactory) error) error {
	if f == nil {
		return fmt.Errorf("factory is closed or nil")
	}
	f.mu.Lock()
	defer f.mu.Unlock()
	if f.ptr == nil {
		return fmt.Errorf("factory is closed or nil")
	}
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	return fn(f.ptr)
}

// WithSubnet appends hosts from a CIDR subnet.
func (f *Factory) WithSubnet(subnet string) error {
	cs, err := cString(subnet)
	if err != nil {
		return err
	}
	defer freeGoCString(cs)
	return f.withLive(func(ptr *C.AsicFactory) error {
		if C.asic_rs_factory_with_subnet(ptr, cs) != 0 {
			return lastError()
		}
		return nil
	})
}

// WithRange appends hosts from a range string.
func (f *Factory) WithRange(rangeStr string) error {
	cs, err := cString(rangeStr)
	if err != nil {
		return err
	}
	defer freeGoCString(cs)
	return f.withLive(func(ptr *C.AsicFactory) error {
		if C.asic_rs_factory_with_range(ptr, cs) != 0 {
			return lastError()
		}
		return nil
	})
}

// WithOctets appends hosts from four octet descriptors.
func (f *Factory) WithOctets(o1, o2, o3, o4 string) error {
	c1, err := cString(o1)
	if err != nil {
		return err
	}
	c2, err := cString(o2)
	if err != nil {
		freeGoCString(c1)
		return err
	}
	c3, err := cString(o3)
	if err != nil {
		freeGoCString(c1)
		freeGoCString(c2)
		return err
	}
	c4, err := cString(o4)
	if err != nil {
		freeGoCString(c1)
		freeGoCString(c2)
		freeGoCString(c3)
		return err
	}
	defer freeGoCString(c1)
	defer freeGoCString(c2)
	defer freeGoCString(c3)
	defer freeGoCString(c4)
	return f.withLive(func(ptr *C.AsicFactory) error {
		if C.asic_rs_factory_with_octets(ptr, c1, c2, c3, c4) != 0 {
			return lastError()
		}
		return nil
	})
}

func (f *Factory) apply(fn func(ptr *C.AsicFactory)) *Factory {
	_ = f.withLive(func(ptr *C.AsicFactory) error {
		fn(ptr)
		return nil
	})
	return f
}

// WithPortCheck enables or disables the initial port connectivity check.
func (f *Factory) WithPortCheck(enabled bool) *Factory {
	return f.apply(func(ptr *C.AsicFactory) {
		C.asic_rs_factory_set_port_check(ptr, C.bool(enabled))
	})
}

// WithConcurrentLimit sets the maximum concurrent discovery tasks.
func (f *Factory) WithConcurrentLimit(limit int) *Factory {
	return f.apply(func(ptr *C.AsicFactory) {
		C.asic_rs_factory_set_concurrent_limit(ptr, C.uintptr_t(limit))
	})
}

// WithIdentificationTimeoutSecs sets how long identification may take.
func (f *Factory) WithIdentificationTimeoutSecs(secs uint64) *Factory {
	return f.apply(func(ptr *C.AsicFactory) {
		C.asic_rs_factory_set_identification_timeout_secs(ptr, C.uint64_t(secs))
	})
}

// WithConnectivityTimeoutSecs sets the TCP connect timeout for port checks.
func (f *Factory) WithConnectivityTimeoutSecs(secs uint64) *Factory {
	return f.apply(func(ptr *C.AsicFactory) {
		C.asic_rs_factory_set_connectivity_timeout_secs(ptr, C.uint64_t(secs))
	})
}

// WithConnectivityRetries sets how many extra connectivity attempts to make
// after the initial probe.
func (f *Factory) WithConnectivityRetries(retries uint32) *Factory {
	return f.apply(func(ptr *C.AsicFactory) {
		C.asic_rs_factory_set_connectivity_retries(ptr, C.uint32_t(retries))
	})
}

// WithNofileLimit sets a desired RLIMIT_NOFILE / maxstdio target for large scans.
func (f *Factory) WithNofileLimit(limit uint64) *Factory {
	return f.apply(func(ptr *C.AsicFactory) {
		C.asic_rs_factory_set_nofile_limit(ptr, C.uint64_t(limit))
	})
}

// WithNofileAdjustment enables or disables automatic nofile raising.
func (f *Factory) WithNofileAdjustment(enabled bool) *Factory {
	return f.apply(func(ptr *C.AsicFactory) {
		C.asic_rs_factory_set_nofile_adjustment(ptr, C.bool(enabled))
	})
}

// WithAdaptiveConcurrency picks a concurrency limit based on the host list size.
func (f *Factory) WithAdaptiveConcurrency() *Factory {
	return f.apply(func(ptr *C.AsicFactory) {
		C.asic_rs_factory_set_adaptive_concurrency(ptr)
	})
}

// Len returns the number of hosts currently configured for scanning.
func (f *Factory) Len() int {
	var n int
	_ = f.withLive(func(ptr *C.AsicFactory) error {
		got := C.asic_rs_factory_len(ptr)
		if got < 0 {
			n = 0
			return lastError()
		}
		n = int(got)
		return nil
	})
	return n
}

// IsEmpty reports whether no hosts are configured.
func (f *Factory) IsEmpty() bool {
	empty := true
	_ = f.withLive(func(ptr *C.AsicFactory) error {
		empty = bool(C.asic_rs_factory_is_empty(ptr))
		return nil
	})
	return empty
}

// Hosts returns the configured host IP strings.
func (f *Factory) Hosts() ([]string, error) {
	var hosts []string
	err := f.withLive(func(ptr *C.AsicFactory) error {
		return takeJSON(C.asic_rs_factory_hosts_json(ptr), &hosts)
	})
	return hosts, err
}

// GetMiner identifies and constructs a miner at ip.
// Returns ErrNotFound when the address does not respond as a supported miner.
func (f *Factory) GetMiner(ip string) (*Miner, error) {
	cs, err := cString(ip)
	if err != nil {
		return nil, err
	}
	defer freeGoCString(cs)
	var miner *Miner
	err = f.withLive(func(ptr *C.AsicFactory) error {
		var out *C.AsicMiner
		code := C.asic_rs_factory_get_miner(ptr, cs, &out)
		m, err := lookupResult(code, out)
		miner = m
		return err
	})
	return miner, err
}

// ScanMiner scans a single IP with the factory's port pre-check logic.
// Returns ErrNotFound when no miner responds.
func (f *Factory) ScanMiner(ip string) (*Miner, error) {
	cs, err := cString(ip)
	if err != nil {
		return nil, err
	}
	defer freeGoCString(cs)
	var miner *Miner
	err = f.withLive(func(ptr *C.AsicFactory) error {
		var out *C.AsicMiner
		code := C.asic_rs_factory_scan_miner(ptr, cs, &out)
		m, err := lookupResult(code, out)
		miner = m
		return err
	})
	return miner, err
}

// Scan discovers miners on all configured hosts.
// The caller must Close each returned Miner.
func (f *Factory) Scan() ([]*Miner, error) {
	var miners []*Miner
	err := f.withLive(func(ptr *C.AsicFactory) error {
		var out **C.AsicMiner
		var n C.uintptr_t
		if C.asic_rs_factory_scan(ptr, &out, &n) != 0 {
			return lastError()
		}
		if n == 0 {
			if out != nil {
				C.asic_rs_free_miner_list(out, 0)
			}
			miners = []*Miner{}
			return nil
		}
		defer C.asic_rs_free_miner_list(out, n)
		slice := unsafe.Slice(out, int(n))
		miners = make([]*Miner, 0, int(n))
		for _, p := range slice {
			if p != nil {
				miners = append(miners, newMiner(p))
			}
		}
		return nil
	})
	return miners, err
}

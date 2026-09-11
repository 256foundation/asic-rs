package asicrs

import (
	"errors"
	"strings"
	"testing"
)

func TestVersion(t *testing.T) {
	v := Version()
	if v == "" {
		t.Fatal("empty version")
	}
	if !strings.Contains(v, ".") {
		t.Fatalf("version %q does not look like semver", v)
	}
}

func TestFactoryHostsFromSubnet(t *testing.T) {
	f, err := NewFactoryFromSubnet("10.0.0.0/30")
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()

	if f.IsEmpty() {
		t.Fatal("expected hosts from /30")
	}
	n := f.Len()
	if n <= 0 {
		t.Fatalf("Len() = %d", n)
	}
	hosts, err := f.Hosts()
	if err != nil {
		t.Fatal(err)
	}
	if len(hosts) != n {
		t.Fatalf("Hosts len %d != Len %d", len(hosts), n)
	}
}

func TestFactoryFromRangeAndOctets(t *testing.T) {
	f, err := NewFactoryFromRange("192.168.1.1-3")
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	if f.Len() != 3 {
		t.Fatalf("range Len = %d, want 3", f.Len())
	}

	f2, err := NewFactoryFromOctets("10", "0", "0", "1-2")
	if err != nil {
		t.Fatal(err)
	}
	defer f2.Close()
	if f2.Len() != 2 {
		t.Fatalf("octets Len = %d, want 2", f2.Len())
	}
}

func TestFactoryWithSubnetAppend(t *testing.T) {
	f := NewFactory()
	defer f.Close()
	if !f.IsEmpty() {
		t.Fatal("new factory should be empty")
	}
	if err := f.WithSubnet("172.16.0.0/30"); err != nil {
		t.Fatal(err)
	}
	n1 := f.Len()
	if err := f.WithRange("172.16.1.1-2"); err != nil {
		t.Fatal(err)
	}
	if f.Len() <= n1 {
		t.Fatalf("expected more hosts after append, got %d then %d", n1, f.Len())
	}
}

func TestFactoryInvalidInputs(t *testing.T) {
	if _, err := NewFactoryFromSubnet("not-a-subnet"); err == nil {
		t.Fatal("expected error for invalid subnet")
	}
	if _, err := NewFactoryFromRange("nope"); err == nil {
		t.Fatal("expected error for invalid range")
	}
	f := NewFactory()
	defer f.Close()
	if err := f.WithOctets("a", "b", "c", "d"); err == nil {
		t.Fatal("expected error for invalid octets")
	}
}

func TestFactoryTuningOptions(t *testing.T) {
	f := NewFactory()
	defer f.Close()
	f.WithPortCheck(false).
		WithConcurrentLimit(50).
		WithIdentificationTimeoutSecs(5).
		WithConnectivityTimeoutSecs(1).
		WithConnectivityRetries(0).
		WithNofileAdjustment(false).
		WithNofileLimit(4096)
	if err := f.WithSubnet("10.255.255.0/30"); err != nil {
		t.Fatal(err)
	}
	f.WithAdaptiveConcurrency()
}

func TestClosedFactory(t *testing.T) {
	f := NewFactory()
	f.Close()
	f.Close()
	if err := f.WithSubnet("10.0.0.0/30"); err == nil {
		t.Fatal("expected error on closed factory")
	}
	if _, err := f.GetMiner("10.0.0.1"); err == nil {
		t.Fatal("expected error on closed factory GetMiner")
	}
	if _, err := f.Scan(); err == nil {
		t.Fatal("expected error on closed factory Scan")
	}
}

func TestInteriorNULRejected(t *testing.T) {
	if _, err := NewFactoryFromSubnet("10.0.0.0/30\x00"); err == nil {
		t.Fatal("expected error for subnet with interior NUL")
	}
}

func TestGetMinerInvalidIP(t *testing.T) {
	f := NewFactory()
	defer f.Close()
	_, err := f.GetMiner("not-an-ip")
	if err == nil {
		t.Fatal("expected error for invalid IP")
	}
	if errors.Is(err, ErrNotFound) {
		t.Fatal("invalid IP should not be ErrNotFound")
	}
}

func TestGetMinerUnreachable(t *testing.T) {
	f := NewFactory().
		WithPortCheck(true).
		WithIdentificationTimeoutSecs(1).
		WithConnectivityTimeoutSecs(1).
		WithConnectivityRetries(0)
	defer f.Close()

	_, err := f.GetMiner("192.0.2.1")
	if !errors.Is(err, ErrNotFound) {
		t.Fatalf("GetMiner TEST-NET-1: %v, want ErrNotFound", err)
	}
	_, err = f.ScanMiner("192.0.2.1")
	if !errors.Is(err, ErrNotFound) {
		t.Fatalf("ScanMiner TEST-NET-1: %v, want ErrNotFound", err)
	}
}

func TestScanEmptyFactory(t *testing.T) {
	f := NewFactory()
	defer f.Close()
	_, err := f.Scan()
	if err == nil {
		t.Fatal("expected error scanning an empty factory")
	}
}

func TestScanNoMiners(t *testing.T) {
	f, err := NewFactoryFromRange("192.0.2.1-1")
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	f.WithPortCheck(true).
		WithIdentificationTimeoutSecs(1).
		WithConnectivityTimeoutSecs(1).
		WithConnectivityRetries(0)

	miners, err := f.Scan()
	if err != nil {
		t.Fatal(err)
	}
	if len(miners) != 0 {
		t.Fatalf("expected no miners on TEST-NET-1, got %d", len(miners))
	}
}

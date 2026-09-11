package asicrs

import "testing"

func TestClosedMiner(t *testing.T) {
	m := &Miner{}
	if _, err := m.IP(); err == nil {
		t.Fatal("expected error on nil miner")
	}
	if _, err := m.GetData(); err == nil {
		t.Fatal("expected error on nil miner GetData")
	}
	if err := m.SetAuth("a", "b"); err == nil {
		t.Fatal("expected error on nil miner SetAuth")
	}
}

func TestDataFieldNamesMatchRust(t *testing.T) {
	// These strings are parsed by DataField::from_str in asic-rs-ffi.
	want := []DataField{
		DataFieldHashboards,
		DataFieldChips,
		DataFieldBestShare,
		DataFieldSessionBestShare,
	}
	for _, field := range want {
		if field == "" {
			t.Fatal("empty data field")
		}
	}
}

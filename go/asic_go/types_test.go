package asic_go

import (
	"encoding/json"
	"os"
	"reflect"
	"strings"
	"testing"
	"time"
)

func TestHashRateAsUnit(t *testing.T) {
	hr := HashRate{Value: 100, Unit: HashRateUnitTeraHash, Algo: "SHA256"}
	if got, err := hr.TH(); err != nil || got != 100 {
		t.Fatalf("TH() = %v, want 100", got)
	}
	gh, err := hr.AsUnit(HashRateUnitGigaHash)
	if err != nil {
		t.Fatal(err)
	}
	if gh.Value != 100_000 {
		t.Fatalf("AsUnit(GigaHash).Value = %v, want 100000", gh.Value)
	}
}

func TestDurationSecsUnmarshal(t *testing.T) {
	var d DurationSecs
	if err := json.Unmarshal([]byte(`{"secs":3600,"nanos":5}`), &d); err != nil {
		t.Fatal(err)
	}
	if d.Secs != 3600 || d.Nanos != 5 {
		t.Fatalf("got %+v", d)
	}
	if d.Duration() != time.Hour+5*time.Nanosecond {
		t.Fatalf("Duration() = %v", d.Duration())
	}

	var d2 DurationSecs
	if err := json.Unmarshal([]byte(`42`), &d2); err != nil {
		t.Fatal(err)
	}
	if d2.Secs != 42 {
		t.Fatalf("bare number: got %d", d2.Secs)
	}
}

func TestPowerWattsUnmarshal(t *testing.T) {
	var p PowerWatts
	if err := json.Unmarshal([]byte(`{"watts":3500.5}`), &p); err != nil {
		t.Fatal(err)
	}
	if p.Float64() != 3500.5 {
		t.Fatalf("got %v", p)
	}
	var p2 PowerWatts
	if err := json.Unmarshal([]byte(`1200`), &p2); err != nil {
		t.Fatal(err)
	}
	if p2.Float64() != 1200 {
		t.Fatalf("got %v", p2)
	}
}

func TestTuningTargetRoundTrip(t *testing.T) {
	cases := []string{
		`{"Power":{"watts":3200}}`,
		`{"HashRate":{"value":100,"unit":"TeraHash","algo":"SHA256"}}`,
		`{"MiningMode":"Normal"}`,
		`{"Preset":"5560"}`,
		`{"Manual":{"boards":{"0":[480.0,12.6],"3":[490.0,12.7],"7":[null,12.5],"8":[500.0,null],"9":[null,null]}}}`,
	}
	for _, raw := range cases {
		var tt TuningTarget
		if err := json.Unmarshal([]byte(raw), &tt); err != nil {
			t.Fatalf("unmarshal %s: %v", raw, err)
		}
		out, err := json.Marshal(tt)
		if err != nil {
			t.Fatalf("marshal: %v", err)
		}
		var again TuningTarget
		if err := json.Unmarshal(out, &again); err != nil {
			t.Fatalf("re-unmarshal %s: %v", out, err)
		}
		if again.Variant != tt.Variant {
			t.Fatalf("kind %q != %q for %s", again.Variant, tt.Variant, raw)
		}
		if tt.Variant == "Manual" && len(again.Boards) != 5 {
			t.Fatalf("manual boards = %+v", again.Boards)
		}
	}
}

func TestMinerDataUnmarshal(t *testing.T) {
	raw := `{
		"schema_version": "1.0",
		"timestamp": 1700000000,
		"ip": "192.168.1.10",
		"mac": "aa:bb:cc:dd:ee:ff",
		"device_info": {
			"make": "Bitmain",
			"model": "S19",
			"hardware": {"fans": 4, "boards": [null, null, null]},
			"firmware": "Stock",
			"algo": "SHA256"
		},
		"serial_number": null,
		"hostname": "miner-1",
		"api_version": "1.0",
		"firmware_version": "1.2.3",
		"control_board_version": null,
		"expected_hashboards": 3,
		"hashboards": [],
		"hashrate": {"value": 95.5, "unit": "TeraHash", "algo": "SHA256"},
		"expected_hashrate": {"value": 100, "unit": "TeraHash", "algo": "SHA256"},
		"expected_chips": 200,
		"total_chips": 198,
		"expected_fans": 4,
		"fans": [{"position": 0, "rpm": 4500}],
		"psu_fans": [],
		"average_temperature": 65.2,
		"fluid_temperature": null,
		"outlet_fluid_temperature": null,
		"wattage": 3250.0,
		"tuning_percent": null,
		"tuning_target": {"Power": {"watts": 3300}},
		"scaled_tuning_target": null,
		"tuning_capabilities": null,
		"efficiency": 34.0,
		"light_flashing": false,
		"messages": [],
		"uptime": {"secs": 86400, "nanos": 0},
		"is_mining": true,
		"pools": [],
		"operating_state": {"type": "Mining"},
		"best_share": 483000.0,
		"session_best_share": 0.0
	}`
	var data MinerData
	if err := json.Unmarshal([]byte(raw), &data); err != nil {
		t.Fatal(err)
	}
	if data.IP != "192.168.1.10" {
		t.Fatalf("ip = %q", data.IP)
	}
	if data.DeviceInfo.Make != "Bitmain" {
		t.Fatalf("make = %q", data.DeviceInfo.Make)
	}
	if got, err := data.HashrateTH(); err != nil || got != 95.5 {
		t.Fatalf("HashrateTH = %v, %v", got, err)
	}
	if data.Uptime == nil || data.Uptime.Duration() != 24*time.Hour {
		t.Fatalf("uptime = %+v", data.Uptime)
	}
	if data.TuningTarget == nil || data.TuningTarget.Variant != "Power" || data.TuningTarget.Watts == nil {
		t.Fatalf("tuning_target = %+v", data.TuningTarget)
	}
	if n, ok := data.DeviceInfo.Hardware.BoardCount(); !ok || n != 3 {
		t.Fatalf("BoardCount = %d, %v", n, ok)
	}
	if data.BestShare == nil || *data.BestShare != 483000 {
		t.Fatalf("best_share = %v", data.BestShare)
	}
	if data.SessionBestShare == nil || *data.SessionBestShare != 0 {
		t.Fatalf("session_best_share = %v", data.SessionBestShare)
	}
}

func TestFanConfigJSON(t *testing.T) {
	cfg := NewFanConfigManual(80)
	b, err := json.Marshal(cfg)
	if err != nil {
		t.Fatal(err)
	}
	var again FanConfig
	if err := json.Unmarshal(b, &again); err != nil {
		t.Fatal(err)
	}
	if again.Mode != "Manual" || again.FanSpeed == nil || *again.FanSpeed != 80 {
		t.Fatalf("got %+v", again)
	}

	auto := NewFanConfigAuto(65, nil)
	b, err = json.Marshal(auto)
	if err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(b, &again); err != nil {
		t.Fatal(err)
	}
	if again.Mode != "Auto" || again.TargetTemp == nil || *again.TargetTemp != 65 {
		t.Fatalf("auto got %+v", again)
	}
}

func TestParsePoolURL(t *testing.T) {
	u, err := ParsePoolURL("stratum+tcp://pool.example.com:3333")
	if err != nil {
		t.Fatal(err)
	}
	if u.Host != "pool.example.com" || u.Port != 3333 || u.Scheme != PoolSchemeStratumV1 {
		t.Fatalf("got %+v", u)
	}
	if u.String() != "stratum+tcp://pool.example.com:3333" {
		t.Fatalf("String = %q", u.String())
	}

	pool, err := NewPoolConfig("stratum+tcp://pool.example.com:3333", "worker.1", "x")
	if err != nil {
		t.Fatal(err)
	}
	if pool.Username != "worker.1" || pool.URL.Host != "pool.example.com" {
		t.Fatalf("got %+v", pool)
	}
}

func TestTimezoneAndTemperatureJSON(t *testing.T) {
	tz := TimezoneConfig{Timezone: strPtr("Europe/Vienna"), Available: []string{"Europe/Vienna", "Etc/GMT-2"}}
	b, err := json.Marshal(tz)
	if err != nil {
		t.Fatal(err)
	}
	var again TimezoneConfig
	if err := json.Unmarshal(b, &again); err != nil {
		t.Fatal(err)
	}
	if again.Timezone == nil || *again.Timezone != "Europe/Vienna" {
		t.Fatalf("got %+v", again)
	}

	hot := 75.0
	temp := TemperatureConfig{Hot: &hot}
	b, err = json.Marshal(temp)
	if err != nil {
		t.Fatal(err)
	}
	var t2 TemperatureConfig
	if err := json.Unmarshal(b, &t2); err != nil {
		t.Fatal(err)
	}
	if t2.Hot == nil || *t2.Hot != 75 {
		t.Fatalf("got %+v", t2)
	}
}

func strPtr(s string) *string { return &s }

func TestHashRateUnitValidation(t *testing.T) {
	for _, name := range []string{"TeraHash", "TH/s", " th_s "} {
		var hr HashRate
		if err := json.Unmarshal([]byte(`{"value":100,"unit":"`+name+`","algo":"SHA256"}`), &hr); err != nil {
			t.Fatal(err)
		}
		if got, err := hr.TH(); err != nil || got != 100 {
			t.Fatalf("%s: %v, %v", name, got, err)
		}
		b, err := json.Marshal(hr)
		if err != nil {
			t.Fatal(err)
		}
		if !strings.Contains(string(b), `"unit":"TeraHash"`) {
			t.Fatalf("noncanonical unit: %s", b)
		}
	}
	var unit HashRateUnit
	if err := json.Unmarshal([]byte(`"bogus"`), &unit); err == nil {
		t.Fatal("unknown unit accepted")
	}
	if _, err := (HashRate{Value: 100, Unit: "bogus"}).TH(); err == nil {
		t.Fatal("invalid source converted")
	}
	if _, err := (HashRate{Value: 100, Unit: HashRateUnitTeraHash}).AsUnit("bogus"); err == nil {
		t.Fatal("invalid target converted")
	}
}

func TestPoolURLIPv6RoundTrip(t *testing.T) {
	for _, raw := range []string{"stratum+tcp://[2001:db8::1]:3333", "stratum2+tcp://[2001:db8::1]:3333/pubkey"} {
		u, err := ParsePoolURL(raw)
		if err != nil {
			t.Fatal(err)
		}
		if u.String() != raw {
			t.Fatalf("%s became %s", raw, u.String())
		}
		// Rust formats its stored host verbatim; JSON must preserve brackets too.
		b, err := json.Marshal(u)
		if err != nil {
			t.Fatal(err)
		}
		var again PoolURL
		if err := json.Unmarshal(b, &again); err != nil {
			t.Fatal(err)
		}
		if again.Host != "[2001:db8::1]" {
			t.Fatalf("Rust host: %q", again.Host)
		}
	}
}

func TestTuningTargetReuseClearsInactiveFields(t *testing.T) {
	var target TuningTarget
	if err := json.Unmarshal([]byte(`{"MiningMode":"High"}`), &target); err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal([]byte(`{"Power":{"watts":3200}}`), &target); err != nil {
		t.Fatal(err)
	}
	if target.TargetMode != nil || target.Variant != "Power" || target.Watts == nil || *target.Watts != 3200 {
		t.Fatalf("stale union: %+v", target)
	}
	for _, invalid := range []string{`{"Power":{"watts":1},"Preset":"x"}`, `{"Manual":{"boards":{"256":[1,2]}}}`, `{"Manual":{"boards":{"0":[1]}}}`} {
		if err := json.Unmarshal([]byte(invalid), &target); err == nil {
			t.Fatalf("invalid target accepted: %s", invalid)
		}
		if target.Variant != "Power" || *target.Watts != 3200 {
			t.Fatal("failed decode mutated receiver")
		}
	}
}

func TestRequiredCollectionsMatchRustFixtures(t *testing.T) {
	raw, err := os.ReadFile("testdata/configs.json")
	if err != nil {
		t.Fatal(err)
	}
	var fixtures map[string]json.RawMessage
	if err := json.Unmarshal(raw, &fixtures); err != nil {
		t.Fatal(err)
	}
	cases := map[string]any{
		"timezone":   TimezoneConfig{Timezone: strPtr("Europe/Vienna")},
		"pool_group": PoolGroupConfig{Name: "default", Quota: 1},
	}
	for name, value := range cases {
		got, err := json.Marshal(value)
		if err != nil {
			t.Fatal(err)
		}
		var actual, expected any
		json.Unmarshal(got, &actual)
		json.Unmarshal(fixtures[name], &expected)
		if !reflect.DeepEqual(actual, expected) {
			t.Fatalf("%s: %s, want %s", name, got, fixtures[name])
		}
	}
}

func TestAlgorithmDefaultsAndHardwareHelpers(t *testing.T) {
	cases := map[HashAlgorithm]HashRateUnit{
		HashAlgorithmSHA256:   HashRateUnitTeraHash,
		HashAlgorithmScrypt:   HashRateUnitGigaHash,
		HashAlgorithmEtHash:   HashRateUnitMegaHash,
		HashAlgorithmEquihash: HashRateUnitKiloHash,
		HashAlgorithmUnknown:  HashRateUnitHash,
	}
	for algo, want := range cases {
		hr, err := (HashRate{Value: 1000, Unit: HashRateUnitHash, Algo: algo}).IntoDefaultUnit()
		if err != nil || hr.Unit != want || hr.Algo != algo {
			t.Fatalf("%s: %+v, %v", algo, hr, err)
		}
	}
	var algo HashAlgorithm
	if err := json.Unmarshal([]byte(`"typo"`), &algo); err == nil {
		t.Fatal("unknown algorithm accepted")
	}
	first, third := uint16(78), uint16(80)
	hardware := MinerHardware{Boards: []*uint16{&first, nil, &third}}
	if n, ok := hardware.BoardCount(); !ok || n != 3 {
		t.Fatalf("board count: %d, %v", n, ok)
	}
	if n, ok := hardware.TotalChips(); !ok || n != 158 {
		t.Fatalf("total chips: %d, %v", n, ok)
	}
	if n, ok := hardware.ChipsForBoard(2); !ok || n != 80 {
		t.Fatalf("board chips: %d, %v", n, ok)
	}
	for _, position := range []int{-1, 1, 3} {
		if _, ok := hardware.ChipsForBoard(position); ok {
			t.Fatalf("unexpected count for board %d", position)
		}
	}
	if _, ok := (MinerHardware{}).TotalChips(); ok {
		t.Fatal("missing boards became a known count")
	}
}

func TestPythonWireAliasesNormalizeToRust(t *testing.T) {
	var target TuningTarget
	if err := json.Unmarshal([]byte(`{"type":"mode","value":"High"}`), &target); err != nil {
		t.Fatal(err)
	}
	if target.Variant != TuningTargetMiningMode || target.TargetMode == nil || *target.TargetMode != MiningModeHigh {
		t.Fatalf("mode: %+v", target)
	}
	b, err := json.Marshal(target)
	if err != nil || string(b) != `{"MiningMode":"High"}` {
		t.Fatalf("mode wire: %s, %v", b, err)
	}
	if err := json.Unmarshal([]byte(`{"type":"manual","value":{"0":[480,12.6]}}`), &target); err != nil {
		t.Fatal(err)
	}
	if len(target.Boards) != 1 || *target.Boards[0].FrequencyMHz != 480 {
		t.Fatalf("manual: %+v", target)
	}
	var fan FanConfig
	if err := json.Unmarshal([]byte(`{"mode":"manual","fan_speed":80}`), &fan); err != nil {
		t.Fatal(err)
	}
	if fan.Mode != FanModeManual {
		t.Fatalf("fan mode: %s", fan.Mode)
	}
	b, err = json.Marshal(fan)
	if err != nil || !strings.Contains(string(b), `"mode":"Manual"`) {
		t.Fatalf("fan wire: %s, %v", b, err)
	}
	var pool PoolURL
	if err := json.Unmarshal([]byte(`"stratum+ssl://pool.example.com:3333"`), &pool); err != nil {
		t.Fatal(err)
	}
	if pool.Scheme != PoolSchemeStratumV1SSL {
		t.Fatalf("pool scheme: %s", pool.Scheme)
	}
	b, err = json.Marshal(pool)
	if err != nil || !strings.Contains(string(b), `"scheme":"StratumV1SSL"`) {
		t.Fatalf("pool wire: %s, %v", b, err)
	}
}

func TestStructuredTelemetryModels(t *testing.T) {
	var data MinerData
	raw := `{"control_board_version":{"known":false,"name":"new-board"},"messages":[{"component":{"type":"HashBoard","idx":2,"chip_idx":0}}],"tuning_capabilities":{"power":{"maximum":{"Power":{"watts":3500}}}}}`
	if err := json.Unmarshal([]byte(raw), &data); err != nil {
		t.Fatal(err)
	}
	if data.ControlBoardVersion == nil || data.ControlBoardVersion.Known || data.ControlBoardVersion.Name != "new-board" {
		t.Fatalf("control board: %+v", data.ControlBoardVersion)
	}
	component := data.Messages[0].Component
	if component.Type != MinerComponentHashBoard || *component.Idx != 2 || *component.ChipIdx != 0 {
		t.Fatalf("component: %+v", component)
	}
	if *data.TuningCapabilities.Power.Maximum.Watts != 3500 {
		t.Fatalf("capabilities: %+v", data.TuningCapabilities)
	}
}

package asicrs

import (
	"encoding/json"
	"testing"
	"time"
)

func TestHashRateAsUnit(t *testing.T) {
	hr := HashRate{Value: 100, Unit: HashRateUnitTeraHash, Algo: "SHA256"}
	if got := hr.TH(); got != 100 {
		t.Fatalf("TH() = %v, want 100", got)
	}
	gh := hr.AsUnit(HashRateUnitGigaHash)
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
		if again.Kind != tt.Kind {
			t.Fatalf("kind %q != %q for %s", again.Kind, tt.Kind, raw)
		}
		if tt.Kind == "Manual" && len(again.Boards) != 5 {
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
	if data.HashrateTH() != 95.5 {
		t.Fatalf("HashrateTH = %v", data.HashrateTH())
	}
	if data.Uptime == nil || data.Uptime.Duration() != 24*time.Hour {
		t.Fatalf("uptime = %+v", data.Uptime)
	}
	if data.TuningTarget == nil || data.TuningTarget.Kind != "Power" || data.TuningTarget.Watts == nil {
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

	pool, err := NewPool("stratum+tcp://pool.example.com:3333", "worker.1", "x")
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

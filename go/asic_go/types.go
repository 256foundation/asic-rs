package asic_go

import (
	"encoding/json"
	"fmt"
	"net/url"
	"strconv"
	"strings"
	"time"
)

// DataField names a MinerData field that can be excluded from GetData.
type DataField string

const (
	DataFieldSchemaVersion          DataField = "SchemaVersion"
	DataFieldTimestamp              DataField = "Timestamp"
	DataFieldIP                     DataField = "Ip"
	DataFieldMAC                    DataField = "Mac"
	DataFieldDeviceInfo             DataField = "DeviceInfo"
	DataFieldSerialNumber           DataField = "SerialNumber"
	DataFieldHostname               DataField = "Hostname"
	DataFieldAPIVersion             DataField = "ApiVersion"
	DataFieldFirmwareVersion        DataField = "FirmwareVersion"
	DataFieldControlBoardVersion    DataField = "ControlBoardVersion"
	DataFieldHashboards             DataField = "Hashboards"
	DataFieldChips                  DataField = "Chips"
	DataFieldHashrate               DataField = "Hashrate"
	DataFieldExpectedHashrate       DataField = "ExpectedHashrate"
	DataFieldFans                   DataField = "Fans"
	DataFieldPSUFans                DataField = "PsuFans"
	DataFieldAverageTemperature     DataField = "AverageTemperature"
	DataFieldFluidTemperature       DataField = "FluidTemperature"
	DataFieldOutletFluidTemperature DataField = "OutletFluidTemperature"
	DataFieldWattage                DataField = "Wattage"
	DataFieldTuningPercent          DataField = "TuningPercent"
	DataFieldTuningTarget           DataField = "TuningTarget"
	DataFieldTuningCapabilities     DataField = "TuningCapabilities"
	DataFieldEfficiency             DataField = "Efficiency"
	DataFieldLightFlashing          DataField = "LightFlashing"
	DataFieldMessages               DataField = "Messages"
	DataFieldUptime                 DataField = "Uptime"
	DataFieldIsMining               DataField = "IsMining"
	DataFieldPools                  DataField = "Pools"
	DataFieldOperatingState         DataField = "OperatingState"
	DataFieldDevFeeConnected        DataField = "DevFeeConnected"
	DataFieldBestShare              DataField = "BestShare"
	DataFieldSessionBestShare       DataField = "SessionBestShare"
)

// HashRate is a hashrate value with unit and algorithm.
type HashRate struct {
	Value float64       `json:"value"`
	Unit  HashRateUnit  `json:"unit"`
	Algo  HashAlgorithm `json:"algo"`
}

// HashRateUnit is the scale of a HashRate.Value.
type HashRateUnit string

const (
	HashRateUnitHash      HashRateUnit = "Hash"
	HashRateUnitKiloHash  HashRateUnit = "KiloHash"
	HashRateUnitMegaHash  HashRateUnit = "MegaHash"
	HashRateUnitGigaHash  HashRateUnit = "GigaHash"
	HashRateUnitTeraHash  HashRateUnit = "TeraHash"
	HashRateUnitPetaHash  HashRateUnit = "PetaHash"
	HashRateUnitExaHash   HashRateUnit = "ExaHash"
	HashRateUnitZettaHash HashRateUnit = "ZettaHash"
	HashRateUnitYottaHash HashRateUnit = "YottaHash"
)

// ParseHashRateUnit accepts Rust variant names and the unit aliases accepted by Rust/Python.
func ParseHashRateUnit(value string) (HashRateUnit, error) {
	normalized := strings.NewReplacer(" ", "", "_", "").Replace(strings.ToUpper(strings.TrimSpace(value)))
	names := []HashRateUnit{HashRateUnitHash, HashRateUnitKiloHash, HashRateUnitMegaHash,
		HashRateUnitGigaHash, HashRateUnitTeraHash, HashRateUnitPetaHash, HashRateUnitExaHash,
		HashRateUnitZettaHash, HashRateUnitYottaHash}
	prefixes := []string{"", "K", "M", "G", "T", "P", "E", "Z", "Y"}
	for i, name := range names {
		short := prefixes[i] + "H"
		if normalized == strings.ToUpper(string(name)) || normalized == short || normalized == short+"S" || normalized == short+"/S" {
			return name, nil
		}
	}
	return "", fmt.Errorf("unknown hash rate unit: %q", value)
}

// UnmarshalJSON validates a unit and normalizes display aliases to Rust's wire names.
func (u *HashRateUnit) UnmarshalJSON(b []byte) error {
	var name string
	if err := json.Unmarshal(b, &name); err != nil {
		return err
	}
	unit, err := ParseHashRateUnit(name)
	if err != nil {
		return err
	}
	*u = unit
	return nil
}

// MarshalJSON always uses the Rust wire name, including for a display alias.
func (u HashRateUnit) MarshalJSON() ([]byte, error) {
	unit, err := ParseHashRateUnit(string(u))
	if err != nil {
		return nil, err
	}
	return json.Marshal(string(unit))
}

// Multiplier returns the factor to convert this unit to H/s, rejecting invalid units.
func (u HashRateUnit) Multiplier() (float64, error) {
	unit, err := ParseHashRateUnit(string(u))
	if err != nil {
		return 0, err
	}
	switch unit {
	case HashRateUnitHash:
		return 1, nil
	case HashRateUnitKiloHash:
		return 1e3, nil
	case HashRateUnitMegaHash:
		return 1e6, nil
	case HashRateUnitGigaHash:
		return 1e9, nil
	case HashRateUnitTeraHash:
		return 1e12, nil
	case HashRateUnitPetaHash:
		return 1e15, nil
	case HashRateUnitExaHash:
		return 1e18, nil
	case HashRateUnitZettaHash:
		return 1e21, nil
	case HashRateUnitYottaHash:
		return 1e24, nil
	default:
		return 0, fmt.Errorf("unknown hash rate unit: %q", unit)
	}
}

// AsUnit converts hr into the requested unit.
func (hr HashRate) AsUnit(unit HashRateUnit) (HashRate, error) {
	source, err := hr.Unit.Multiplier()
	if err != nil {
		return HashRate{}, err
	}
	target, err := unit.Multiplier()
	if err != nil {
		return HashRate{}, err
	}
	unit, err = ParseHashRateUnit(string(unit))
	if err != nil {
		return HashRate{}, err
	}
	base := hr.Value * source
	return HashRate{
		Value: base / target,
		Unit:  unit,
		Algo:  hr.Algo,
	}, nil
}

// TH returns the hashrate in TH/s.
func (hr HashRate) TH() (float64, error) {
	converted, err := hr.AsUnit(HashRateUnitTeraHash)
	return converted.Value, err
}

// DeviceInfo is static identity information for a miner.
type DeviceInfo struct {
	Make     string        `json:"make"`
	Model    string        `json:"model"`
	Hardware MinerHardware `json:"hardware"`
	Firmware string        `json:"firmware"`
	Algo     HashAlgorithm `json:"algo"`
}

// MinerHardware describes expected fans and per-board chip counts.
type MinerHardware struct {
	Fans   *uint8    `json:"fans"`
	Boards []*uint16 `json:"boards"`
}

// BoardCount returns the expected number of hashboards when available.
func (h MinerHardware) BoardCount() (uint8, bool) {
	if h.Boards == nil || len(h.Boards) > 255 {
		return 0, false
	}
	return uint8(len(h.Boards)), true
}

// ChipData is optional per-chip telemetry.
type ChipData struct {
	Position    uint16    `json:"position"`
	Hashrate    *HashRate `json:"hashrate"`
	Temperature *float64  `json:"temperature"`
	Voltage     *float64  `json:"voltage"`
	Frequency   *float64  `json:"frequency"`
	Working     *bool     `json:"working"`
	Tuned       *bool     `json:"tuned"`
}

// BoardData is per-hashboard telemetry.
type BoardData struct {
	Position              uint8      `json:"position"`
	Hashrate              *HashRate  `json:"hashrate"`
	ExpectedHashrate      *HashRate  `json:"expected_hashrate"`
	BoardTemperature      *float64   `json:"board_temperature"`
	InletChipTemperature  *float64   `json:"inlet_chip_temperature"`
	OutletChipTemperature *float64   `json:"outlet_chip_temperature"`
	ExpectedChips         *uint16    `json:"expected_chips"`
	WorkingChips          *uint16    `json:"working_chips"`
	SerialNumber          *string    `json:"serial_number"`
	Chips                 []ChipData `json:"chips"`
	Voltage               *float64   `json:"voltage"`
	Frequency             *float64   `json:"frequency"`
	Tuned                 *bool      `json:"tuned"`
	Active                *bool      `json:"active"`
}

// FanData is a single fan reading (RPM as float when present).
type FanData struct {
	Position int16    `json:"position"`
	RPM      *float64 `json:"rpm"`
}

// MessageSeverity is the severity of a miner message.
type MessageSeverity string

const (
	MessageSeverityError   MessageSeverity = "Error"
	MessageSeverityWarning MessageSeverity = "Warning"
	MessageSeverityInfo    MessageSeverity = "Info"
)

// MinerMessage is a status/error message from the device.
type MinerMessage struct {
	Timestamp uint32          `json:"timestamp"`
	Code      uint64          `json:"code"`
	Message   string          `json:"message"`
	Severity  MessageSeverity `json:"severity"`
	Component *MinerComponent `json:"component,omitempty"`
}

// PoolScheme is the stratum protocol scheme as serialized by asic-rs.
type PoolScheme string

const (
	PoolSchemeStratumV1    PoolScheme = "StratumV1"
	PoolSchemeStratumV1SSL PoolScheme = "StratumV1SSL"
	PoolSchemeStratumV2    PoolScheme = "StratumV2"
)

// PoolURL is a parsed mining pool endpoint.
type PoolURL struct {
	Scheme PoolScheme `json:"scheme"`
	Host   string     `json:"host"`
	Port   uint16     `json:"port"`
	Pubkey *string    `json:"pubkey"`
}

func (u PoolURL) schemeString() string { return u.Scheme.String() }

// String formats the pool URL.
func poolHost(host string) string {
	if strings.Contains(host, ":") && !strings.HasPrefix(host, "[") {
		return "[" + host + "]"
	}
	return host
}

func (u PoolURL) MarshalJSON() ([]byte, error) {
	type wire PoolURL
	u.Host = poolHost(u.Host)
	return json.Marshal(wire(u))
}

func (u PoolURL) String() string {
	u.Host = poolHost(u.Host)
	if u.Pubkey != nil && *u.Pubkey != "" {
		return fmt.Sprintf("%s://%s:%d/%s", u.schemeString(), u.Host, u.Port, *u.Pubkey)
	}
	return fmt.Sprintf("%s://%s:%d", u.schemeString(), u.Host, u.Port)
}

// ParsePoolURL parses a stratum URL into a PoolURL.
func ParsePoolURL(raw string) (PoolURL, error) {
	if !strings.Contains(raw, "://") {
		raw = "stratum+tcp://" + raw
	}
	parsed, err := url.Parse(raw)
	if err != nil {
		return PoolURL{}, err
	}
	scheme, err := ParsePoolScheme(parsed.Scheme)
	if err != nil {
		return PoolURL{}, err
	}
	port := uint16(80)
	if parsed.Port() != "" {
		n, err := strconv.ParseUint(parsed.Port(), 10, 16)
		if err != nil {
			return PoolURL{}, err
		}
		port = uint16(n)
	}
	var pubkey *string
	if path := strings.TrimPrefix(parsed.Path, "/"); path != "" {
		pubkey = &path
	}
	return PoolURL{
		Scheme: scheme,
		Host:   poolHost(parsed.Hostname()),
		Port:   port,
		Pubkey: pubkey,
	}, nil
}

// PoolData is runtime status for one configured pool.
type PoolData struct {
	Position       *uint16  `json:"position"`
	URL            *PoolURL `json:"url"`
	AcceptedShares *uint64  `json:"accepted_shares"`
	RejectedShares *uint64  `json:"rejected_shares"`
	Active         *bool    `json:"active"`
	Alive          *bool    `json:"alive"`
	User           *string  `json:"user"`
}

// PoolGroupData is a group of pools with a quota.
type PoolGroupData struct {
	Name  string     `json:"name"`
	Quota uint32     `json:"quota"`
	Pools []PoolData `json:"pools"`
}

// DurationSecs unmarshals a Rust std::time::Duration ({secs,nanos}) or a bare number of seconds.
type DurationSecs struct {
	Secs  uint64 `json:"secs"`
	Nanos uint32 `json:"nanos"`
}

// Duration converts to time.Duration.
func (d DurationSecs) Duration() time.Duration {
	return time.Duration(d.Secs)*time.Second + time.Duration(d.Nanos)*time.Nanosecond
}

// UnmarshalJSON accepts {"secs":N,"nanos":M} or a numeric seconds value.
func (d *DurationSecs) UnmarshalJSON(b []byte) error {
	if string(b) == "null" {
		return nil
	}
	var n uint64
	if err := json.Unmarshal(b, &n); err == nil {
		d.Secs = n
		d.Nanos = 0
		return nil
	}
	type raw DurationSecs
	return json.Unmarshal(b, (*raw)(d))
}

// PowerWatts unmarshals measurements::Power which serializes as {"watts": N},
// or a bare number (when asic-rs custom serializers are used).
type PowerWatts float64

// UnmarshalJSON accepts {"watts":N} or a bare number.
func (p *PowerWatts) UnmarshalJSON(b []byte) error {
	if string(b) == "null" {
		return nil
	}
	var n float64
	if err := json.Unmarshal(b, &n); err == nil {
		*p = PowerWatts(n)
		return nil
	}
	var obj struct {
		Watts float64 `json:"watts"`
	}
	if err := json.Unmarshal(b, &obj); err != nil {
		return err
	}
	*p = PowerWatts(obj.Watts)
	return nil
}

// Float64 returns the wattage.
func (p PowerWatts) Float64() float64 { return float64(p) }

// MarshalJSON encodes Power as {"watts": N} for asic-rs TuningTarget::Power.
func (p PowerWatts) MarshalJSON() ([]byte, error) {
	return json.Marshal(map[string]float64{"watts": float64(p)})
}

// ManualBoardSetpoint is a (MHz, volts) pair. Nulls mean the firmware omitted that value.
type ManualBoardSetpoint struct {
	FrequencyMHz *float64
	Volts        *float64
}

// TuningTarget is a firmware tuning target. The Rust enum is externally tagged:
//
//	{"Manual":{"boards":{"0":[480,12.6]}}} | {"Power":{"watts":3500}} | {"HashRate":{...}} | {"MiningMode":"Normal"} | {"Preset":"5560"}
type TuningTarget struct {
	Variant        TuningTargetVariant
	Watts          *float64
	TargetHashrate *HashRate
	TargetMode     *MiningMode
	PresetName     *string
	Boards         ManualTuningValues
	Raw            json.RawMessage
}

// UnmarshalJSON decodes the externally-tagged TuningTarget enum.
func (t *TuningTarget) UnmarshalJSON(b []byte) error {
	var next TuningTarget
	if err := next.decodeJSON(b); err != nil {
		return err
	}
	*t = next
	return nil
}

func (t *TuningTarget) decodeJSON(b []byte) error {
	t.Raw = append([]byte(nil), b...)
	var m map[string]json.RawMessage
	if err := json.Unmarshal(b, &m); err != nil {
		var s MiningMode
		if err2 := json.Unmarshal(b, &s); err2 == nil {
			t.Variant = "MiningMode"
			t.TargetMode = &s
			return nil
		}
		return err
	}
	if tag, ok := m["type"]; ok {
		if len(m) != 2 || m["value"] == nil {
			return fmt.Errorf("Python tuning target requires type and value")
		}
		var name string
		if err := json.Unmarshal(tag, &name); err != nil {
			return err
		}
		variants := map[string]string{"manual": "Manual", "power": "Power", "hashrate": "HashRate", "mode": "MiningMode", "preset": "Preset"}
		variant, ok := variants[name]
		if !ok {
			return fmt.Errorf("unknown tuning target type: %q", name)
		}
		value := m["value"]
		if name == "manual" {
			wrapped, err := json.Marshal(map[string]json.RawMessage{"boards": value})
			if err != nil {
				return err
			}
			value = wrapped
		}
		m = map[string]json.RawMessage{variant: value}
	}
	if len(m) != 1 {
		return fmt.Errorf("TuningTarget must contain exactly one variant")
	}
	if raw, ok := m["Manual"]; ok {
		t.Variant = "Manual"
		var payload struct {
			Boards map[uint8][]*float64 `json:"boards"`
		}
		if err := json.Unmarshal(raw, &payload); err != nil {
			return err
		}
		t.Boards = make(ManualTuningValues, len(payload.Boards))
		for id, pair := range payload.Boards {
			if len(pair) != 2 {
				return fmt.Errorf("board %d requires a frequency/voltage pair", id)
			}
			sp := ManualBoardSetpoint{}
			if len(pair) > 0 {
				sp.FrequencyMHz = pair[0]
			}
			if len(pair) > 1 {
				sp.Volts = pair[1]
			}
			t.Boards[id] = sp
		}
		return nil
	}
	if raw, ok := m["Power"]; ok {
		t.Variant = "Power"
		var pw PowerWatts
		if err := json.Unmarshal(raw, &pw); err != nil {
			return err
		}
		w := pw.Float64()
		t.Watts = &w
		return nil
	}
	if raw, ok := m["HashRate"]; ok {
		t.Variant = "HashRate"
		var hr HashRate
		if err := json.Unmarshal(raw, &hr); err != nil {
			return err
		}
		t.TargetHashrate = &hr
		return nil
	}
	if raw, ok := m["MiningMode"]; ok {
		t.Variant = "MiningMode"
		var s MiningMode
		if err := json.Unmarshal(raw, &s); err != nil {
			return err
		}
		t.TargetMode = &s
		return nil
	}
	if raw, ok := m["Preset"]; ok {
		t.Variant = "Preset"
		var s string
		if err := json.Unmarshal(raw, &s); err != nil {
			return err
		}
		t.PresetName = &s
		return nil
	}
	return fmt.Errorf("unknown TuningTarget variant: %s", string(b))
}

// MarshalJSON encodes TuningTarget in the Rust externally-tagged form.
func (t TuningTarget) MarshalJSON() ([]byte, error) {
	switch t.Variant {
	case "Manual":
		boards := make(map[uint8][2]*float64, len(t.Boards))
		for id, sp := range t.Boards {
			boards[id] = [2]*float64{sp.FrequencyMHz, sp.Volts}
		}
		return json.Marshal(map[string]any{"Manual": map[string]any{"boards": boards}})
	case "Power":
		w := 0.0
		if t.Watts != nil {
			w = *t.Watts
		}
		return json.Marshal(map[string]any{"Power": map[string]float64{"watts": w}})
	case "HashRate":
		return json.Marshal(map[string]any{"HashRate": t.TargetHashrate})
	case "MiningMode":
		var mode MiningMode
		if t.TargetMode != nil {
			mode = *t.TargetMode
		}
		return json.Marshal(map[string]any{"MiningMode": mode})
	case "Preset":
		name := ""
		if t.PresetName != nil {
			name = *t.PresetName
		}
		return json.Marshal(map[string]any{"Preset": name})
	default:
		if len(t.Raw) > 0 {
			return t.Raw, nil
		}
		return nil, fmt.Errorf("empty TuningTarget")
	}
}

// NewTuningTargetManual builds a manual TuningTarget (perpetual/autotune disabled).
func NewTuningTargetManual(boards ManualTuningValues) TuningTarget {
	if boards == nil {
		boards = ManualTuningValues{}
	}
	return TuningTarget{Variant: "Manual", Boards: boards}
}

// NewTuningTargetPower builds a power TuningTarget.
func NewTuningTargetPower(watts float64) TuningTarget {
	w := watts
	return TuningTarget{Variant: "Power", Watts: &w}
}

// NewTuningTargetPreset builds a named-preset TuningTarget.
func NewTuningTargetPreset(name string) TuningTarget {
	n := name
	return TuningTarget{Variant: "Preset", PresetName: &n}
}

// MinerData is a full telemetry snapshot from a miner.
type MinerData struct {
	SchemaVersion          string              `json:"schema_version"`
	Timestamp              uint64              `json:"timestamp"`
	IP                     string              `json:"ip"`
	MAC                    *string             `json:"mac"`
	DeviceInfo             DeviceInfo          `json:"device_info"`
	SerialNumber           *string             `json:"serial_number"`
	Hostname               *string             `json:"hostname"`
	APIVersion             *string             `json:"api_version"`
	FirmwareVersion        *string             `json:"firmware_version"`
	ControlBoardVersion    *MinerControlBoard  `json:"control_board_version"`
	ExpectedHashboards     *uint8              `json:"expected_hashboards"`
	Hashboards             []BoardData         `json:"hashboards"`
	Hashrate               *HashRate           `json:"hashrate"`
	ExpectedHashrate       *HashRate           `json:"expected_hashrate"`
	ExpectedChips          *uint16             `json:"expected_chips"`
	TotalChips             *uint16             `json:"total_chips"`
	ExpectedFans           *uint8              `json:"expected_fans"`
	Fans                   []FanData           `json:"fans"`
	PSUFans                []FanData           `json:"psu_fans"`
	AverageTemperature     *float64            `json:"average_temperature"`
	FluidTemperature       *float64            `json:"fluid_temperature"`
	OutletFluidTemperature *float64            `json:"outlet_fluid_temperature"`
	Wattage                *float64            `json:"wattage"`
	TuningPercent          *uint8              `json:"tuning_percent"`
	TuningTarget           *TuningTarget       `json:"tuning_target"`
	ScaledTuningTarget     *TuningTarget       `json:"scaled_tuning_target"`
	TuningCapabilities     *TuningCapabilities `json:"tuning_capabilities"`
	Efficiency             *float64            `json:"efficiency"`
	LightFlashing          *bool               `json:"light_flashing"`
	Messages               []MinerMessage      `json:"messages"`
	Uptime                 *DurationSecs       `json:"uptime"`
	IsMining               bool                `json:"is_mining"`
	Pools                  []PoolGroupData     `json:"pools"`
	OperatingState         *OperatingState     `json:"operating_state"`
	DevFeeConnected        *bool               `json:"devfee_connected"`
	BestShare              *float64            `json:"best_share"`
	SessionBestShare       *float64            `json:"session_best_share"`
}

// OperatingState is the firmware-reported runtime state (`{"type":"Mining"}`).
type OperatingState struct {
	Type OperatingStateType `json:"type"`
	Raw  *string            `json:"raw,omitempty"`
}

// HashrateTH returns current hashrate in TH/s, or 0 if unknown.
func (d MinerData) HashrateTH() (float64, error) {
	if d.Hashrate == nil {
		return 0, nil
	}
	return d.Hashrate.TH()
}

// TimestampTime returns the data timestamp as time.Time (Unix seconds).
func (d MinerData) TimestampTime() time.Time {
	return time.Unix(int64(d.Timestamp), 0).UTC()
}

// PoolConfig is a single pool endpoint for SetPoolsConfig.
type PoolConfig struct {
	URL      PoolURL `json:"url"`
	Username string  `json:"username"`
	Password string  `json:"password"`
}

// NewPoolConfig builds a PoolConfig from a stratum URL string.
func NewPoolConfig(rawURL, username, password string) (PoolConfig, error) {
	u, err := ParsePoolURL(rawURL)
	if err != nil {
		return PoolConfig{}, err
	}
	return PoolConfig{URL: u, Username: username, Password: password}, nil
}

// PoolGroupConfig is a named group of pools.
type PoolGroupConfig struct {
	Name  string       `json:"name"`
	Quota uint32       `json:"quota"`
	Pools []PoolConfig `json:"pools"`
}

// ScalingConfig controls power/hashrate step scaling.
type ScalingConfig struct {
	Step             uint32   `json:"step"`
	Minimum          uint32   `json:"minimum"`
	Shutdown         *bool    `json:"shutdown"`
	ShutdownDuration *float32 `json:"shutdown_duration"`
}

// TuningConfig is a target plus optional algorithm string.
type TuningConfig struct {
	Target    TuningTarget `json:"target"`
	Algorithm *string      `json:"algorithm"`
}

// FanConfig is a tagged enum: Auto or Manual.
type FanConfig struct {
	Mode       FanMode  `json:"mode"`
	TargetTemp *float64 `json:"target_temp,omitempty"`
	IdleSpeed  *uint64  `json:"idle_speed,omitempty"`
	FanSpeed   *uint64  `json:"fan_speed,omitempty"`
}

// NewFanConfigAuto builds an automatic fan config.
func NewFanConfigAuto(targetTemp float64, idleSpeed *uint64) FanConfig {
	return FanConfig{Mode: "Auto", TargetTemp: &targetTemp, IdleSpeed: idleSpeed}
}

// NewFanConfigManual builds a manual fan speed config.
func NewFanConfigManual(fanSpeed uint64) FanConfig {
	return FanConfig{Mode: "Manual", FanSpeed: &fanSpeed}
}

// TemperatureConfig is configured thermal limits in °C.
type TemperatureConfig struct {
	Hot     *float64 `json:"hot"`
	Danger  *float64 `json:"danger"`
	Minimum *float64 `json:"minimum"`
}

// TimezoneConfig uses IANA timezone names.
type TimezoneConfig struct {
	Timezone  *string  `json:"timezone"`
	Available []string `json:"available"`
}

// PresetInfo is an autotune/overclock preset reported by the firmware.
type PresetInfo struct {
	Name   string  `json:"name"`
	Pretty *string `json:"pretty"`
	Status *string `json:"status"`
}

// FirmwareStats is the result of an on-demand firmware-update check.
type FirmwareStats struct {
	CurrentVersion  *string         `json:"current_version"`
	LatestVersion   *string         `json:"latest_version"`
	UpdateAvailable bool            `json:"update_available"`
	Firmware        json.RawMessage `json:"firmware"`
}

// RestoreStockOSResult reports acceptance of a stock-OS restore request.
// A non-nil RebootAfterSeconds is the firmware's expected reboot delay.
type RestoreStockOSResult struct {
	Accepted           bool    `json:"accepted"`
	RebootAfterSeconds *uint64 `json:"reboot_after_seconds"`
}

// ExpectedCounts is the expected hardware shape reported by a miner handle.
type ExpectedCounts struct {
	Hashboards *uint8  `json:"hashboards"`
	Chips      *uint16 `json:"chips"`
	Fans       *uint8  `json:"fans"`
}

// Supports reports which control/config features this miner backend exposes.
type Supports struct {
	SetFaultLight       bool `json:"set_fault_light"`
	SetPowerLimit       bool `json:"set_power_limit"`
	SetTuningPercent    bool `json:"set_tuning_percent"`
	Presets             bool `json:"presets"`
	Restart             bool `json:"restart"`
	Pause               bool `json:"pause"`
	Resume              bool `json:"resume"`
	ChangePassword      bool `json:"change_password"`
	ReadLogs            bool `json:"read_logs"`
	FactoryReset        bool `json:"factory_reset"`
	RestoreStockOS      bool `json:"restore_stock_os"`
	PoolsConfig         bool `json:"pools_config"`
	UpgradeFirmware     bool `json:"upgrade_firmware"`
	PrepareFirmware     bool `json:"prepare_firmware"`
	CheckFirmwareUpdate bool `json:"check_firmware_update"`
	TimezoneConfig      bool `json:"timezone_config"`
	ScalingConfig       bool `json:"scaling_config"`
	TemperatureConfig   bool `json:"temperature_config"`
	TuningConfig        bool `json:"tuning_config"`
	FanConfig           bool `json:"fan_config"`
}

// Required Rust collections are sequences, even when a Go slice is nil.
func (c TimezoneConfig) MarshalJSON() ([]byte, error) {
	type wire TimezoneConfig
	if c.Available == nil {
		c.Available = []string{}
	}
	return json.Marshal(wire(c))
}

func (c PoolGroupConfig) MarshalJSON() ([]byte, error) {
	type wire PoolGroupConfig
	if c.Pools == nil {
		c.Pools = []PoolConfig{}
	}
	return json.Marshal(wire(c))
}

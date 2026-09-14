package asic_go

import (
	"encoding/json"
	"fmt"
)

// HashAlgorithm identifies a mining algorithm using the shared Rust/Python names.
type HashAlgorithm string

const (
	HashAlgorithmSHA256      HashAlgorithm = "SHA256"
	HashAlgorithmScrypt      HashAlgorithm = "Scrypt"
	HashAlgorithmX11         HashAlgorithm = "X11"
	HashAlgorithmBlake2S256  HashAlgorithm = "Blake2S256"
	HashAlgorithmKadena      HashAlgorithm = "Kadena"
	HashAlgorithmKHeavyHash  HashAlgorithm = "KHeavyHash"
	HashAlgorithmEaglesong   HashAlgorithm = "Eaglesong"
	HashAlgorithmEtHash      HashAlgorithm = "EtHash"
	HashAlgorithmEquihash    HashAlgorithm = "Equihash"
	HashAlgorithmHandshake   HashAlgorithm = "Handshake"
	HashAlgorithmBlake256R14 HashAlgorithm = "Blake256R14"
	HashAlgorithmUnknown     HashAlgorithm = "Unknown"
)

// DefaultHashrateUnit returns the conventional display unit for an algorithm.
func (a HashAlgorithm) DefaultHashrateUnit() (HashRateUnit, error) {
	switch a {
	case HashAlgorithmScrypt, HashAlgorithmX11:
		return HashRateUnitGigaHash, nil
	case HashAlgorithmEtHash:
		return HashRateUnitMegaHash, nil
	case HashAlgorithmEquihash:
		return HashRateUnitKiloHash, nil
	case HashAlgorithmSHA256, HashAlgorithmBlake2S256, HashAlgorithmKadena,
		HashAlgorithmKHeavyHash, HashAlgorithmEaglesong, HashAlgorithmHandshake, HashAlgorithmBlake256R14:
		return HashRateUnitTeraHash, nil
	case HashAlgorithmUnknown:
		return HashRateUnitHash, nil
	default:
		return "", fmt.Errorf("unknown hash algorithm: %q", a)
	}
}

// ParseHashAlgorithm validates a shared algorithm name. Unknown is explicit;
// misspelled names never silently become SHA256 or Unknown.
func ParseHashAlgorithm(name string) (HashAlgorithm, error) {
	algorithm := HashAlgorithm(name)
	_, err := algorithm.DefaultHashrateUnit()
	if err != nil {
		return "", err
	}
	return algorithm, nil
}

func (a *HashAlgorithm) UnmarshalJSON(b []byte) error {
	var name string
	if err := json.Unmarshal(b, &name); err != nil {
		return err
	}
	algorithm, err := ParseHashAlgorithm(name)
	if err != nil {
		return err
	}
	*a = algorithm
	return nil
}

func (a HashAlgorithm) MarshalJSON() ([]byte, error) {
	if _, err := ParseHashAlgorithm(string(a)); err != nil {
		return nil, err
	}
	return json.Marshal(string(a))
}

// MiningMode is a firmware-defined performance mode.
type MiningMode string

const (
	MiningModeLow    MiningMode = "Low"
	MiningModeNormal MiningMode = "Normal"
	MiningModeHigh   MiningMode = "High"
)

func (m MiningMode) validate() error {
	switch m {
	case MiningModeLow, MiningModeNormal, MiningModeHigh:
		return nil
	default:
		return fmt.Errorf("unknown mining mode: %q", m)
	}
}
func (m *MiningMode) UnmarshalJSON(b []byte) error {
	var name string
	if err := json.Unmarshal(b, &name); err != nil {
		return err
	}
	mode := MiningMode(name)
	if err := mode.validate(); err != nil {
		return err
	}
	*m = mode
	return nil
}
func (m MiningMode) MarshalJSON() ([]byte, error) {
	if err := m.validate(); err != nil {
		return nil, err
	}
	return json.Marshal(string(m))
}

// TuningTargetVariant names the variants of Rust's TuningTarget.
type TuningTargetVariant string

const (
	TuningTargetManual     TuningTargetVariant = "Manual"
	TuningTargetPower      TuningTargetVariant = "Power"
	TuningTargetHashRate   TuningTargetVariant = "HashRate"
	TuningTargetMiningMode TuningTargetVariant = "MiningMode"
	TuningTargetPreset     TuningTargetVariant = "Preset"
)

// FanMode uses Rust's wire names; Python's lowercase aliases are also accepted.
type FanMode string

const (
	FanModeAuto   FanMode = "Auto"
	FanModeManual FanMode = "Manual"
)

func parseFanMode(name string) (FanMode, error) {
	switch name {
	case "Auto", "auto":
		return FanModeAuto, nil
	case "Manual", "manual":
		return FanModeManual, nil
	default:
		return "", fmt.Errorf("unknown fan mode: %q", name)
	}
}
func (m *FanMode) UnmarshalJSON(b []byte) error {
	var name string
	if err := json.Unmarshal(b, &name); err != nil {
		return err
	}
	mode, err := parseFanMode(name)
	if err != nil {
		return err
	}
	*m = mode
	return nil
}
func (m FanMode) MarshalJSON() ([]byte, error) {
	mode, err := parseFanMode(string(m))
	if err != nil {
		return nil, err
	}
	return json.Marshal(string(mode))
}

// OperatingStateType identifies a firmware-reported operating state.
type OperatingStateType string

const (
	OperatingStateMining                OperatingStateType = "Mining"
	OperatingStateStable                OperatingStateType = "Stable"
	OperatingStateInitializing          OperatingStateType = "Initializing"
	OperatingStateStarting              OperatingStateType = "Starting"
	OperatingStateTuning                OperatingStateType = "Tuning"
	OperatingStateAdjustingFrequency    OperatingStateType = "AdjustingFrequency"
	OperatingStateAdjustingVoltage      OperatingStateType = "AdjustingVoltage"
	OperatingStateAdjustingClockVoltage OperatingStateType = "AdjustingClockVoltage"
	OperatingStateIdling                OperatingStateType = "Idling"
	OperatingStatePaused                OperatingStateType = "Paused"
	OperatingStateSuspended             OperatingStateType = "Suspended"
	OperatingStateRestricted            OperatingStateType = "Restricted"
	OperatingStateStopping              OperatingStateType = "Stopping"
	OperatingStateStopped               OperatingStateType = "Stopped"
	OperatingStateRestarting            OperatingStateType = "Restarting"
	OperatingStateCoolingDown           OperatingStateType = "CoolingDown"
	OperatingStateDegradedMining        OperatingStateType = "DegradedMining"
	OperatingStateError                 OperatingStateType = "Error"
	OperatingStateUnknown               OperatingStateType = "Unknown"
)

// ParsePoolScheme accepts shared variant names and stratum URL scheme names.
func ParsePoolScheme(name string) (PoolScheme, error) {
	switch name {
	case "StratumV1", "stratum+tcp":
		return PoolSchemeStratumV1, nil
	case "StratumV1SSL", "stratum+ssl", "stratum+tls":
		return PoolSchemeStratumV1SSL, nil
	case "StratumV2", "stratum2+tcp":
		return PoolSchemeStratumV2, nil
	default:
		return "", fmt.Errorf("unknown pool scheme: %q", name)
	}
}
func (s PoolScheme) String() string {
	scheme, err := ParsePoolScheme(string(s))
	if err != nil {
		return string(s)
	}
	switch scheme {
	case PoolSchemeStratumV1SSL:
		return "stratum+ssl"
	case PoolSchemeStratumV2:
		return "stratum2+tcp"
	default:
		return "stratum+tcp"
	}
}
func (s *PoolScheme) UnmarshalJSON(b []byte) error {
	var name string
	if err := json.Unmarshal(b, &name); err != nil {
		return err
	}
	scheme, err := ParsePoolScheme(name)
	if err != nil {
		return err
	}
	*s = scheme
	return nil
}
func (s PoolScheme) MarshalJSON() ([]byte, error) {
	scheme, err := ParsePoolScheme(string(s))
	if err != nil {
		return nil, err
	}
	return json.Marshal(string(scheme))
}

// UnmarshalJSON accepts the structured Rust PoolURL or Python's URL string.
func (u *PoolURL) UnmarshalJSON(b []byte) error {
	var raw string
	if err := json.Unmarshal(b, &raw); err == nil {
		parsed, err := ParsePoolURL(raw)
		if err != nil {
			return err
		}
		*u = parsed
		return nil
	}
	type wire PoolURL
	var parsed wire
	if err := json.Unmarshal(b, &parsed); err != nil {
		return err
	}
	parsed.Host = poolHost(parsed.Host)
	*u = PoolURL(parsed)
	return nil
}

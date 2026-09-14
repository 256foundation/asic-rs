package asic_go

// ManualTuningValues maps board positions to scalar MHz/volt setpoints.
type ManualTuningValues map[uint8]ManualBoardSetpoint

// MinerControlBoard preserves both the known flag and the reported name.
type MinerControlBoard struct {
	Known bool   `json:"known"`
	Name  string `json:"name"`
}

// MinerComponentType identifies the component associated with a miner message.
type MinerComponentType string

const (
	MinerComponentControlBoard MinerComponentType = "ControlBoard"
	MinerComponentHashBoard    MinerComponentType = "HashBoard"
	MinerComponentFan          MinerComponentType = "Fan"
	MinerComponentPowerSupply  MinerComponentType = "PowerSupply"
)

// MinerComponent is the shared tagged component model. Idx is absent for a
// control board; ChipIdx is optional for a hashboard.
type MinerComponent struct {
	Type    MinerComponentType `json:"type"`
	Idx     *uint16            `json:"idx,omitempty"`
	ChipIdx *uint16            `json:"chip_idx,omitempty"`
}

// PowerTuningCapabilities describes the supported power targets.
type PowerTuningCapabilities struct {
	Default *TuningTarget `json:"default"`
	Minimum *TuningTarget `json:"minimum"`
	Maximum *TuningTarget `json:"maximum"`
}

// HashRateTuningCapabilities describes the supported hashrate targets.
type HashRateTuningCapabilities struct {
	Default *TuningTarget `json:"default"`
	Minimum *TuningTarget `json:"minimum"`
	Maximum *TuningTarget `json:"maximum"`
}

// PresetTuningCapabilities describes selectable presets or mining modes.
type PresetTuningCapabilities struct {
	Default *TuningTarget  `json:"default"`
	Presets []TuningTarget `json:"presets"`
}

// TuningCapabilities groups the firmware tuning envelopes by domain.
type TuningCapabilities struct {
	Power    *PowerTuningCapabilities    `json:"power"`
	Hashrate *HashRateTuningCapabilities `json:"hashrate"`
	Presets  *PresetTuningCapabilities   `json:"presets"`
}

// TotalChips returns the expected total chip count, when representable.
func (h MinerHardware) TotalChips() (uint16, bool) {
	if h.Boards == nil {
		return 0, false
	}
	var total uint32
	for _, chips := range h.Boards {
		if chips != nil {
			total += uint32(*chips)
		}
		if total > 65535 {
			return 0, false
		}
	}
	return uint16(total), true
}

// ChipsForBoard returns the expected chip count for one board position.
func (h MinerHardware) ChipsForBoard(position int) (uint16, bool) {
	if position < 0 || position >= len(h.Boards) || h.Boards[position] == nil {
		return 0, false
	}
	return *h.Boards[position], true
}

// DefaultUnit returns the conventional unit for this hashrate's algorithm.
func (hr HashRate) DefaultUnit() (HashRateUnit, error) { return hr.Algo.DefaultHashrateUnit() }

// IntoDefaultUnit converts a hashrate to its algorithm's conventional unit.
func (hr HashRate) IntoDefaultUnit() (HashRate, error) {
	unit, err := hr.DefaultUnit()
	if err != nil {
		return HashRate{}, err
	}
	return hr.AsUnit(unit)
}

// NewTuningTargetHashrate builds a hashrate tuning target.
func NewTuningTargetHashrate(hashrate HashRate) TuningTarget {
	return TuningTarget{Variant: TuningTargetHashRate, TargetHashrate: &hashrate}
}

// NewTuningTargetMiningMode builds a mining-mode tuning target.
func NewTuningTargetMiningMode(mode MiningMode) TuningTarget {
	return TuningTarget{Variant: TuningTargetMiningMode, TargetMode: &mode}
}

// GetExpectedHashboards returns the expected board count.
func (m *Miner) GetExpectedHashboards() (*uint8, error) {
	counts, err := m.Expected()
	return counts.Hashboards, err
}

// GetExpectedChips returns the expected total chip count.
func (m *Miner) GetExpectedChips() (*uint16, error) {
	counts, err := m.Expected()
	return counts.Chips, err
}

// GetExpectedFans returns the expected fan count.
func (m *Miner) GetExpectedFans() (*uint8, error) {
	counts, err := m.Expected()
	return counts.Fans, err
}

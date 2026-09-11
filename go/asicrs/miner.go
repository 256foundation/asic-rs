package asicrs

/*
#include "asic_rs_ffi.h"
#include <stdlib.h>
*/
import "C"

import (
	"encoding/json"
	"fmt"
	"runtime"
	"sync"
	"time"
)

// Miner is a handle to a discovered ASIC miner.
//
// Always call Close when finished (or use defer). Data and control methods may
// be called concurrently; Close must not run at the same time as other methods
// on the same Miner.
type Miner struct {
	mu  sync.Mutex
	ptr *C.AsicMiner
}

func newMiner(ptr *C.AsicMiner) *Miner {
	m := &Miner{ptr: ptr}
	runtime.SetFinalizer(m, (*Miner).Close)
	return m
}

// Close frees the miner handle. Safe to call multiple times. Do not call Close
// concurrently with other methods on the same Miner.
func (m *Miner) Close() {
	if m == nil {
		return
	}
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.ptr == nil {
		return
	}
	runtime.LockOSThread()
	C.asic_rs_miner_free(m.ptr)
	runtime.UnlockOSThread()
	m.ptr = nil
	runtime.SetFinalizer(m, nil)
}

func (m *Miner) withLive(fn func(ptr *C.AsicMiner) error) error {
	if m == nil {
		return fmt.Errorf("miner is closed or nil")
	}
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.ptr == nil {
		return fmt.Errorf("miner is closed or nil")
	}
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	return fn(m.ptr)
}

func (m *Miner) getJSON(fn func(ptr *C.AsicMiner) *C.char, dest any) error {
	return m.withLive(func(ptr *C.AsicMiner) error {
		return takeJSON(fn(ptr), dest)
	})
}

func (m *Miner) getOptionalString(fn func(ptr *C.AsicMiner) *C.char) (*string, error) {
	var v *string
	return v, m.getJSON(fn, &v)
}

func (m *Miner) marshalConfig(v any) (*C.char, error) {
	b, err := json.Marshal(v)
	if err != nil {
		return nil, err
	}
	return cString(string(b))
}

func (m *Miner) setJSON(fn func(ptr *C.AsicMiner, cs *C.char) C.int32_t, v any) (bool, error) {
	cs, err := m.marshalConfig(v)
	if err != nil {
		return false, err
	}
	defer freeGoCString(cs)
	var ok bool
	err = m.withLive(func(ptr *C.AsicMiner) error {
		var err error
		ok, err = controlResult(fn(ptr, cs))
		return err
	})
	return ok, err
}

func (m *Miner) control(fn func(ptr *C.AsicMiner) C.int32_t) (bool, error) {
	var ok bool
	err := m.withLive(func(ptr *C.AsicMiner) error {
		var err error
		ok, err = controlResult(fn(ptr))
		return err
	})
	return ok, err
}

func (m *Miner) ownedString(fn func(ptr *C.AsicMiner) *C.char) (string, error) {
	var s string
	err := m.withLive(func(ptr *C.AsicMiner) error {
		cs := fn(ptr)
		if cs == nil {
			return lastError()
		}
		s = goStringOwned(cs)
		return nil
	})
	return s, err
}

// IP returns the miner's IP address.
func (m *Miner) IP() (string, error) {
	return m.ownedString(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_ip(ptr) })
}

// DeviceInfo returns static make/model/firmware information.
func (m *Miner) DeviceInfo() (DeviceInfo, error) {
	var info DeviceInfo
	err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_device_info_json(ptr) }, &info)
	return info, err
}

// Summary returns a short human-readable description.
func (m *Miner) Summary() (string, error) {
	return m.ownedString(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_summary(ptr) })
}

// Expected returns expected hashboard/chip/fan counts.
func (m *Miner) Expected() (ExpectedCounts, error) {
	var v ExpectedCounts
	err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_expected_json(ptr) }, &v)
	return v, err
}

// Supports returns capability flags for this miner.
func (m *Miner) Supports() (Supports, error) {
	var s Supports
	err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_supports_json(ptr) }, &s)
	return s, err
}

// SetAuth sets username/password credentials for subsequent privileged operations.
func (m *Miner) SetAuth(username, password string) error {
	cu, err := cString(username)
	if err != nil {
		return err
	}
	cp, err := cString(password)
	if err != nil {
		freeGoCString(cu)
		return err
	}
	defer freeGoCString(cu)
	defer freeGoCString(cp)
	return m.withLive(func(ptr *C.AsicMiner) error {
		if C.asic_rs_miner_set_auth(ptr, cu, cp) != 0 {
			return lastError()
		}
		return nil
	})
}

// SetToken sets a pre-issued bearer token (for firmwares such as VNish).
func (m *Miner) SetToken(token string) error {
	cs, err := cString(token)
	if err != nil {
		return err
	}
	defer freeGoCString(cs)
	return m.withLive(func(ptr *C.AsicMiner) error {
		if C.asic_rs_miner_set_token(ptr, cs) != 0 {
			return lastError()
		}
		return nil
	})
}

func (m *Miner) excludeCString(exclude []DataField) (*C.char, error) {
	if len(exclude) == 0 {
		return nil, nil
	}
	names := make([]string, len(exclude))
	for i, f := range exclude {
		names[i] = string(f)
	}
	return m.marshalConfig(names)
}

// GetData collects a full MinerData snapshot.
// Pass DataField values to skip expensive fields, matching Python get_data(exclude=...).
func (m *Miner) GetData(exclude ...DataField) (*MinerData, error) {
	excludeC, err := m.excludeCString(exclude)
	if err != nil {
		return nil, err
	}
	if excludeC != nil {
		defer freeGoCString(excludeC)
	}
	var data MinerData
	err = m.withLive(func(ptr *C.AsicMiner) error {
		return takeJSON(C.asic_rs_miner_get_data_json(ptr, excludeC), &data)
	})
	if err != nil {
		return nil, err
	}
	return &data, nil
}

// GetDataJSON returns the raw MinerData JSON bytes without unmarshaling.
func (m *Miner) GetDataJSON(exclude ...DataField) ([]byte, error) {
	excludeC, err := m.excludeCString(exclude)
	if err != nil {
		return nil, err
	}
	if excludeC != nil {
		defer freeGoCString(excludeC)
	}
	var raw []byte
	err = m.withLive(func(ptr *C.AsicMiner) error {
		var err error
		raw, err = takeJSONBytes(C.asic_rs_miner_get_data_json(ptr, excludeC))
		return err
	})
	return raw, err
}

// CheckFirmwareUpdate queries the vendor for an available update.
func (m *Miner) CheckFirmwareUpdate() (*FirmwareStats, error) {
	var v *FirmwareStats
	if err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_check_firmware_update_json(ptr) }, &v); err != nil {
		return nil, err
	}
	return v, nil
}

// GetMAC returns the miner MAC address string, if available.
func (m *Miner) GetMAC() (*string, error) {
	return m.getOptionalString(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_mac_json(ptr) })
}

// GetSerialNumber returns the control-board serial, if available.
func (m *Miner) GetSerialNumber() (*string, error) {
	return m.getOptionalString(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_serial_number_json(ptr) })
}

// GetHostname returns the network hostname, if available.
func (m *Miner) GetHostname() (*string, error) {
	return m.getOptionalString(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_hostname_json(ptr) })
}

// GetAPIVersion returns the device API version string, if available.
func (m *Miner) GetAPIVersion() (*string, error) {
	return m.getOptionalString(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_api_version_json(ptr) })
}

// GetFirmwareVersion returns the firmware version string, if available.
func (m *Miner) GetFirmwareVersion() (*string, error) {
	return m.getOptionalString(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_firmware_version_json(ptr) })
}

// GetControlBoardVersion returns the control board type string, if available.
func (m *Miner) GetControlBoardVersion() (*string, error) {
	return m.getOptionalString(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_control_board_version_json(ptr) })
}

// GetHashboards returns per-board telemetry.
func (m *Miner) GetHashboards() ([]BoardData, error) {
	var v []BoardData
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_hashboards_json(ptr) }, &v)
}

// GetHashboardsNoChips returns per-board telemetry without per-chip details.
func (m *Miner) GetHashboardsNoChips() ([]BoardData, error) {
	var v []BoardData
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_hashboards_no_chips_json(ptr) }, &v)
}

// GetHashrate returns the current hashrate, if available.
func (m *Miner) GetHashrate() (*HashRate, error) {
	var v *HashRate
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_hashrate_json(ptr) }, &v)
}

// GetExpectedHashrate returns the expected/factory hashrate, if available.
func (m *Miner) GetExpectedHashrate() (*HashRate, error) {
	var v *HashRate
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_expected_hashrate_json(ptr) }, &v)
}

// GetFans returns chassis fan readings.
func (m *Miner) GetFans() ([]FanData, error) {
	var v []FanData
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_fans_json(ptr) }, &v)
}

// GetPSUFans returns PSU fan readings.
func (m *Miner) GetPSUFans() ([]FanData, error) {
	var v []FanData
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_psu_fans_json(ptr) }, &v)
}

// GetFluidTemperature returns environment/fluid temperature in °C, if available.
func (m *Miner) GetFluidTemperature() (*float64, error) {
	var v *float64
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_fluid_temperature_json(ptr) }, &v)
}

// GetOutletFluidTemperature returns coolant exhaust temperature in °C, if available.
func (m *Miner) GetOutletFluidTemperature() (*float64, error) {
	var v *float64
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_outlet_fluid_temperature_json(ptr) }, &v)
}

// GetWattage returns power draw in watts, if available.
func (m *Miner) GetWattage() (*float64, error) {
	var v *float64
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_wattage_json(ptr) }, &v)
}

// GetBestShare returns all-time best share difficulty, if reported.
func (m *Miner) GetBestShare() (*float64, error) {
	var v *float64
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_best_share_json(ptr) }, &v)
}

// GetSessionBestShare returns session best share difficulty, if reported.
func (m *Miner) GetSessionBestShare() (*float64, error) {
	var v *float64
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_session_best_share_json(ptr) }, &v)
}

// GetTuningPercent returns the current manual throttle percent, if exposed.
func (m *Miner) GetTuningPercent() (*uint8, error) {
	var v *uint8
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_tuning_percent_json(ptr) }, &v)
}

// GetTuningTarget returns the current tuning target, if available.
func (m *Miner) GetTuningTarget() (*TuningTarget, error) {
	var v *TuningTarget
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_tuning_target_json(ptr) }, &v)
}

// GetScaledTuningTarget returns the scaling-adjusted tuning target, if available.
func (m *Miner) GetScaledTuningTarget() (*TuningTarget, error) {
	var v *TuningTarget
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_scaled_tuning_target_json(ptr) }, &v)
}

// GetTuningCapabilities returns the firmware tuning envelope, if available.
func (m *Miner) GetTuningCapabilities() (json.RawMessage, error) {
	var v json.RawMessage
	err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_tuning_capabilities_json(ptr) }, &v)
	return v, err
}

// GetLightFlashing returns the fault-light state, if available.
func (m *Miner) GetLightFlashing() (*bool, error) {
	var v *bool
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_light_flashing_json(ptr) }, &v)
}

// GetMessages returns device status/error messages.
func (m *Miner) GetMessages() ([]MinerMessage, error) {
	var v []MinerMessage
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_messages_json(ptr) }, &v)
}

// GetUptime returns system uptime, if available.
func (m *Miner) GetUptime() (*time.Duration, error) {
	var secs *uint64
	if err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_uptime_secs_json(ptr) }, &secs); err != nil {
		return nil, err
	}
	if secs == nil {
		return nil, nil
	}
	d := time.Duration(*secs) * time.Second
	return &d, nil
}

// GetIsMining reports whether hashing is currently running.
func (m *Miner) GetIsMining() (bool, error) {
	var v bool
	if err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_is_mining_json(ptr) }, &v); err != nil {
		return false, err
	}
	return v, nil
}

// GetOperatingState returns the firmware-reported operating state, if available.
func (m *Miner) GetOperatingState() (*OperatingState, error) {
	var v *OperatingState
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char {
		return C.asic_rs_miner_get_operating_state_json(ptr)
	}, &v)
}

// GetPools returns configured pool groups with runtime stats.
func (m *Miner) GetPools() ([]PoolGroupData, error) {
	var v []PoolGroupData
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_pools_json(ptr) }, &v)
}

// GetPoolsConfig returns the writable pools configuration.
func (m *Miner) GetPoolsConfig() ([]PoolGroupConfig, error) {
	var v []PoolGroupConfig
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_pools_config_json(ptr) }, &v)
}

// GetScalingConfig returns the scaling configuration.
func (m *Miner) GetScalingConfig() (*ScalingConfig, error) {
	var v ScalingConfig
	if err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_scaling_config_json(ptr) }, &v); err != nil {
		return nil, err
	}
	return &v, nil
}

// GetTemperatureConfig returns configured thermal limits.
func (m *Miner) GetTemperatureConfig() (*TemperatureConfig, error) {
	var v TemperatureConfig
	if err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_temperature_config_json(ptr) }, &v); err != nil {
		return nil, err
	}
	return &v, nil
}

// GetTuningConfig returns the tuning configuration.
func (m *Miner) GetTuningConfig() (*TuningConfig, error) {
	var v TuningConfig
	if err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_tuning_config_json(ptr) }, &v); err != nil {
		return nil, err
	}
	return &v, nil
}

// GetFanConfig returns the fan configuration.
func (m *Miner) GetFanConfig() (*FanConfig, error) {
	var v FanConfig
	if err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_fan_config_json(ptr) }, &v); err != nil {
		return nil, err
	}
	return &v, nil
}

// GetTimezoneConfig returns timezone configuration.
func (m *Miner) GetTimezoneConfig() (*TimezoneConfig, error) {
	var v TimezoneConfig
	if err := m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_timezone_config_json(ptr) }, &v); err != nil {
		return nil, err
	}
	return &v, nil
}

// GetPresets returns available autotune/overclock presets.
func (m *Miner) GetPresets() ([]PresetInfo, error) {
	var v []PresetInfo
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_get_presets_json(ptr) }, &v)
}

// SetPoolsConfig applies a pools configuration.
func (m *Miner) SetPoolsConfig(groups []PoolGroupConfig) (bool, error) {
	return m.setJSON(func(ptr *C.AsicMiner, cs *C.char) C.int32_t {
		return C.asic_rs_miner_set_pools_config_json(ptr, cs)
	}, groups)
}

// SetScalingConfig applies a scaling configuration.
func (m *Miner) SetScalingConfig(cfg ScalingConfig) (bool, error) {
	return m.setJSON(func(ptr *C.AsicMiner, cs *C.char) C.int32_t {
		return C.asic_rs_miner_set_scaling_config_json(ptr, cs)
	}, cfg)
}

// SetTemperatureConfig applies thermal limits.
func (m *Miner) SetTemperatureConfig(cfg TemperatureConfig) (bool, error) {
	return m.setJSON(func(ptr *C.AsicMiner, cs *C.char) C.int32_t {
		return C.asic_rs_miner_set_temperature_config_json(ptr, cs)
	}, cfg)
}

// SetTuningConfig applies a tuning configuration, with optional scaling.
func (m *Miner) SetTuningConfig(cfg TuningConfig, scaling *ScalingConfig) (bool, error) {
	cc, err := m.marshalConfig(cfg)
	if err != nil {
		return false, err
	}
	defer freeGoCString(cc)
	var sc *C.char
	if scaling != nil {
		sc, err = m.marshalConfig(scaling)
		if err != nil {
			return false, err
		}
		defer freeGoCString(sc)
	}
	return m.control(func(ptr *C.AsicMiner) C.int32_t {
		return C.asic_rs_miner_set_tuning_config_json(ptr, cc, sc)
	})
}

// SetFanConfig applies a fan configuration.
func (m *Miner) SetFanConfig(cfg FanConfig) (bool, error) {
	return m.setJSON(func(ptr *C.AsicMiner, cs *C.char) C.int32_t {
		return C.asic_rs_miner_set_fan_config_json(ptr, cs)
	}, cfg)
}

// SetTimezoneConfig applies timezone configuration.
func (m *Miner) SetTimezoneConfig(cfg TimezoneConfig) (bool, error) {
	return m.setJSON(func(ptr *C.AsicMiner, cs *C.char) C.int32_t {
		return C.asic_rs_miner_set_timezone_config_json(ptr, cs)
	}, cfg)
}

// SetFaultLight turns the fault/alert light on or off.
func (m *Miner) SetFaultLight(fault bool) (bool, error) {
	return m.control(func(ptr *C.AsicMiner) C.int32_t {
		return C.asic_rs_miner_set_fault_light(ptr, C.bool(fault))
	})
}

// SetPowerLimit sets a power limit in watts.
func (m *Miner) SetPowerLimit(watts float64) (bool, error) {
	return m.control(func(ptr *C.AsicMiner) C.int32_t {
		return C.asic_rs_miner_set_power_limit(ptr, C.double(watts))
	})
}

// SetTuningPercent sets a manual throttle percent (100 = unthrottled).
func (m *Miner) SetTuningPercent(percent uint8) (bool, error) {
	return m.control(func(ptr *C.AsicMiner) C.int32_t {
		return C.asic_rs_miner_set_tuning_percent(ptr, C.uint8_t(percent))
	})
}

// Restart reboots the miner.
func (m *Miner) Restart() (bool, error) {
	return m.control(func(ptr *C.AsicMiner) C.int32_t {
		return C.asic_rs_miner_restart(ptr)
	})
}

// Pause pauses mining. A nil atTime means immediately.
func (m *Miner) Pause(atTime *time.Duration) (bool, error) {
	secs := -1.0
	if atTime != nil {
		secs = atTime.Seconds()
	}
	return m.control(func(ptr *C.AsicMiner) C.int32_t {
		return C.asic_rs_miner_pause(ptr, C.double(secs))
	})
}

// Resume resumes mining. A nil atTime means immediately.
func (m *Miner) Resume(atTime *time.Duration) (bool, error) {
	secs := -1.0
	if atTime != nil {
		secs = atTime.Seconds()
	}
	return m.control(func(ptr *C.AsicMiner) C.int32_t {
		return C.asic_rs_miner_resume(ptr, C.double(secs))
	})
}

// Revalidate re-runs this backend's discovery checks against the same IP.
func (m *Miner) Revalidate() (*bool, error) {
	var v *bool
	return v, m.getJSON(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_revalidate_json(ptr) }, &v)
}

// FactoryReset factory-resets the miner.
func (m *Miner) FactoryReset() (bool, error) {
	return m.control(func(ptr *C.AsicMiner) C.int32_t {
		return C.asic_rs_miner_factory_reset(ptr)
	})
}

// ReadLogs returns miner logs, if supported.
func (m *Miner) ReadLogs() (string, error) {
	return m.ownedString(func(ptr *C.AsicMiner) *C.char { return C.asic_rs_miner_read_logs(ptr) })
}

// ChangePassword changes the miner password.
func (m *Miner) ChangePassword(password string) (bool, error) {
	cs, err := cString(password)
	if err != nil {
		return false, err
	}
	defer freeGoCString(cs)
	return m.control(func(ptr *C.AsicMiner) C.int32_t {
		return C.asic_rs_miner_change_password(ptr, cs)
	})
}

// UpgradeFirmware uploads and applies a firmware image from a local file path.
func (m *Miner) UpgradeFirmware(path string) (bool, error) {
	cs, err := cString(path)
	if err != nil {
		return false, err
	}
	defer freeGoCString(cs)
	return m.control(func(ptr *C.AsicMiner) C.int32_t {
		return C.asic_rs_miner_upgrade_firmware(ptr, cs)
	})
}

use std::os::raw::c_char;
use std::ptr;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Duration;

use asic_rs::core::config::fan::FanConfig;
use asic_rs::core::config::pools::PoolGroupConfig;
use asic_rs::core::config::scaling::ScalingConfig;
use asic_rs::core::config::temperature::TemperatureConfig;
use asic_rs::core::config::timezone::TimezoneConfig;
use asic_rs::core::config::tuning::TuningConfig;
use asic_rs::core::data::collector::DataField;
use asic_rs::core::data::firmware::{FirmwareImage, FirmwareStats, FirmwareUpdate};
use asic_rs::core::traits::auth::MinerAuth;
use asic_rs::core::traits::miner::Miner as MinerTrait;
use measurements::Power;
use serde::Serialize;
use serde_json::json;

use crate::error::{
    clear_error, cstr_to_str, json_to_c_string, parse_json, set_error, to_c_string,
};
use crate::runtime::block_on;

/// Opaque miner handle. Owned by the caller; free with [`asic_rs_miner_free`].
pub struct AsicMiner {
    inner: Mutex<Box<dyn MinerTrait>>,
}

impl AsicMiner {
    pub(crate) fn new(inner: Box<dyn MinerTrait>) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }
}

fn with_miner<T>(
    miner: *const AsicMiner,
    f: impl FnOnce(&dyn MinerTrait) -> Result<T, String>,
) -> Result<T, String> {
    if miner.is_null() {
        return Err("null miner handle".to_string());
    }
    // SAFETY: caller owns a live miner handle for the duration of the call.
    let miner = unsafe { &*miner };
    let guard = miner
        .inner
        .lock()
        .map_err(|e| format!("miner lock poisoned: {e}"))?;
    f(guard.as_ref())
}

fn with_miner_mut<T>(
    miner: *mut AsicMiner,
    f: impl FnOnce(&mut dyn MinerTrait) -> Result<T, String>,
) -> Result<T, String> {
    if miner.is_null() {
        return Err("null miner handle".to_string());
    }
    // SAFETY: caller owns a live miner handle for the duration of the call.
    let miner = unsafe { &*miner };
    let mut guard = miner
        .inner
        .lock()
        .map_err(|e| format!("miner lock poisoned: {e}"))?;
    f(guard.as_mut())
}

fn miner_json<T: Serialize>(
    miner: *const AsicMiner,
    f: impl FnOnce(&dyn MinerTrait) -> Result<T, String>,
) -> *mut c_char {
    clear_error();
    match with_miner(miner, f) {
        Ok(value) => json_to_c_string(&value),
        Err(e) => {
            set_error(e);
            ptr::null_mut()
        }
    }
}

fn miner_control(
    miner: *const AsicMiner,
    f: impl FnOnce(&dyn MinerTrait) -> Result<bool, String>,
) -> i32 {
    clear_error();
    match with_miner(miner, f) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(e) => {
            set_error(e);
            -1
        }
    }
}

fn result_bool(result: anyhow::Result<bool>) -> Result<bool, String> {
    result.map_err(|e| e.to_string())
}

fn parse_exclude(exclude_json: *const c_char) -> Result<Vec<DataField>, String> {
    if exclude_json.is_null() {
        return Ok(Vec::new());
    }
    let names: Vec<String> = parse_json(exclude_json)?;
    names
        .into_iter()
        .map(|name| DataField::from_str(&name).map_err(|_| format!("unknown data field: {name}")))
        .collect()
}

fn firmware_stats_json(stats: &FirmwareStats) -> serde_json::Value {
    json!({
        "current_version": stats.current_version.as_ref().map(ToString::to_string),
        "latest_version": stats.latest_version.as_ref().map(ToString::to_string),
        "update_available": stats.update_available(),
        "firmware": match &stats.firmware {
            Some(FirmwareUpdate::Remote(url)) => json!({"kind": "remote", "url": url}),
            Some(FirmwareUpdate::Local(_)) => json!({"kind": "local"}),
            None => serde_json::Value::Null,
        }
    })
}

fn optional_duration_secs(secs: f64) -> Option<Duration> {
    if secs < 0.0 {
        None
    } else {
        Some(Duration::from_secs_f64(secs))
    }
}

/// Free a miner handle.
///
/// # Safety
/// `miner` must be null or a pointer previously returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_free(miner: *mut AsicMiner) {
    if !miner.is_null() {
        drop(Box::from_raw(miner));
    }
}

/// IP address as a newly allocated C string. Free with [`asic_rs_free_string`].
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_ip(miner: *const AsicMiner) -> *mut c_char {
    clear_error();
    match with_miner(miner, |m| Ok(m.get_ip().to_string())) {
        Ok(ip) => to_c_string(ip),
        Err(e) => {
            set_error(e);
            ptr::null_mut()
        }
    }
}

/// Device info as JSON. Free with [`asic_rs_free_string`].
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_device_info_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| Ok(m.get_device_info()))
}

/// Human-readable summary: "Make Model (Firmware): IP". Free with free_string.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_summary(miner: *const AsicMiner) -> *mut c_char {
    clear_error();
    match with_miner(miner, |m| {
        let info = m.get_device_info();
        Ok(format!(
            "{} {} ({}): {}",
            info.make,
            info.model,
            info.firmware,
            m.get_ip()
        ))
    }) {
        Ok(summary) => to_c_string(summary),
        Err(e) => {
            set_error(e);
            ptr::null_mut()
        }
    }
}

/// Expected hashboards / chips / fans as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_expected_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| {
        Ok(json!({
            "hashboards": m.get_expected_hashboards(),
            "chips": m.get_expected_chips(),
            "fans": m.get_expected_fans(),
        }))
    })
}

/// Capability flags as JSON. Free with [`asic_rs_free_string`].
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_supports_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| {
        Ok(json!({
            "set_fault_light": m.supports_set_fault_light(),
            "set_power_limit": m.supports_set_power_limit(),
            "set_tuning_percent": m.supports_set_tuning_percent(),
            "presets": m.supports_presets(),
            "restart": m.supports_restart(),
            "pause": m.supports_pause(),
            "resume": m.supports_resume(),
            "change_password": m.supports_change_password(),
            "read_logs": m.supports_read_logs(),
            "factory_reset": m.supports_factory_reset(),
            "pools_config": m.supports_pools_config(),
            "upgrade_firmware": m.supports_upgrade_firmware(),
            "prepare_firmware": m.supports_prepare_firmware(),
            "check_firmware_update": m.supports_check_firmware_update(),
            "timezone_config": m.supports_timezone_config(),
            "scaling_config": m.supports_scaling_config(),
            "temperature_config": m.supports_temperature_config(),
            "tuning_config": m.supports_tuning_config(),
            "fan_config": m.supports_fan_config(),
        }))
    })
}

/// Set username/password credentials. Returns 0 on success, -1 on error.
///
/// # Safety
/// `miner` must be a live handle; username and password must be valid C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_set_auth(
    miner: *mut AsicMiner,
    username: *const c_char,
    password: *const c_char,
) -> i32 {
    clear_error();
    let user = match cstr_to_str(username) {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    let pass = match cstr_to_str(password) {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    match with_miner_mut(miner, |m| {
        m.set_auth(MinerAuth::new(user, pass));
        Ok(())
    }) {
        Ok(()) => 0,
        Err(e) => {
            set_error(e);
            -1
        }
    }
}

/// Set a pre-issued bearer token. Returns 0 on success, -1 on error.
///
/// # Safety
/// `miner` must be a live handle; `token` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_set_token(
    miner: *mut AsicMiner,
    token: *const c_char,
) -> i32 {
    clear_error();
    let token = match cstr_to_str(token) {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    match with_miner_mut(miner, |m| {
        m.set_auth(MinerAuth::from_token(token));
        Ok(())
    }) {
        Ok(()) => 0,
        Err(e) => {
            set_error(e);
            -1
        }
    }
}

/// Full MinerData as JSON. `exclude_json` is an optional JSON array of DataField
/// names to skip. Free with [`asic_rs_free_string`].
///
/// # Safety
/// `miner` must be a live handle; `exclude_json` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_data_json(
    miner: *const AsicMiner,
    exclude_json: *const c_char,
) -> *mut c_char {
    let exclude = match parse_exclude(exclude_json) {
        Ok(v) => v,
        Err(e) => {
            set_error(e);
            return ptr::null_mut();
        }
    };
    miner_json(miner, |m| {
        if exclude.is_empty() {
            block_on(m.get_data())
        } else {
            block_on(m.get_data_filtered(exclude))
        }
    })
}

/// Check for an available firmware update. JSON object or JSON null.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_check_firmware_update_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| {
        let stats = block_on(m.check_firmware_update())?;
        Ok(stats.ok().map(|stats| firmware_stats_json(&stats)))
    })
}

/// MAC address as JSON (string or null).
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_mac_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| {
        Ok(block_on(m.get_mac())?.map(|mac| mac.to_string()))
    })
}

/// Serial number as JSON (string or null).
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_serial_number_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_serial_number()))
}

/// Hostname as JSON (string or null).
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_hostname_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_hostname()))
}

/// API version as JSON (string or null).
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_api_version_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_api_version()))
}

/// Firmware version as JSON (string or null).
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_firmware_version_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_firmware_version()))
}

/// Control board version as JSON (string or null).
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_control_board_version_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| {
        Ok(block_on(m.get_control_board_version())?.map(|cb| cb.to_string()))
    })
}

/// Per-board telemetry as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_hashboards_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_hashboards()))
}

/// Per-board telemetry without per-chip details as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_hashboards_no_chips_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_hashboards_no_chips()))
}

/// Current hashrate as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_hashrate_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_hashrate()))
}

/// Expected hashrate as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_expected_hashrate_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_expected_hashrate()))
}

/// Chassis fans as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_fans_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_fans()))
}

/// PSU fans as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_psu_fans_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_psu_fans()))
}

/// Fluid/ambient temperature in °C as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_fluid_temperature_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| {
        Ok(block_on(m.get_fluid_temperature())?.map(|t| t.as_celsius()))
    })
}

/// Outlet fluid temperature in °C as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_outlet_fluid_temperature_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| {
        Ok(block_on(m.get_outlet_fluid_temperature())?.map(|t| t.as_celsius()))
    })
}

/// Power draw in watts as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_wattage_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| {
        Ok(block_on(m.get_wattage())?.map(|w| w.as_watts()))
    })
}

/// All-time best share difficulty as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_best_share_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_best_share()))
}

/// Session best share difficulty as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_session_best_share_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_session_best_share()))
}

/// Manual tuning percent as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_tuning_percent_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_tuning_percent()))
}

/// Tuning target as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_tuning_target_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_tuning_target()))
}

/// Scaled tuning target as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_scaled_tuning_target_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_scaled_tuning_target()))
}

/// Tuning capabilities as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_tuning_capabilities_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_tuning_capabilities()))
}

/// Fault-light state as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_light_flashing_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_light_flashing()))
}

/// Miner messages as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_messages_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_messages()))
}

/// Uptime in seconds as JSON (number or null).
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_uptime_secs_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(
        miner,
        |m| Ok(block_on(m.get_uptime())?.map(|d| d.as_secs())),
    )
}

/// Whether hashing is running, as JSON bool.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_is_mining_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_is_mining()))
}

/// Operating state as JSON (object or null).
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_operating_state_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_operating_state()))
}

/// Runtime pool groups as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_pools_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_pools()))
}

/// Writable pools configuration as JSON. Null pointer on error.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_pools_config_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| {
        block_on(m.get_pools_config())?.map_err(|e| e.to_string())
    })
}

/// Scaling configuration as JSON. Null pointer on error.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_scaling_config_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| {
        block_on(m.get_scaling_config())?.map_err(|e| e.to_string())
    })
}

/// Temperature configuration as JSON. Null pointer on error.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_temperature_config_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| {
        block_on(m.get_temperature_config())?.map_err(|e| e.to_string())
    })
}

/// Tuning configuration as JSON. Null pointer on error.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_tuning_config_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| {
        block_on(m.get_tuning_config())?.map_err(|e| e.to_string())
    })
}

/// Fan configuration as JSON. Null pointer on error.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_fan_config_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| {
        block_on(m.get_fan_config())?.map_err(|e| e.to_string())
    })
}

/// Timezone configuration as JSON. Null pointer on error.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_timezone_config_json(
    miner: *const AsicMiner,
) -> *mut c_char {
    miner_json(miner, |m| {
        block_on(m.get_timezone_config())?.map_err(|e| e.to_string())
    })
}

/// Available presets as JSON.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_get_presets_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| block_on(m.get_presets()))
}

/// Apply pools configuration from JSON. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle; `json` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_set_pools_config_json(
    miner: *const AsicMiner,
    json: *const c_char,
) -> i32 {
    let cfg: Vec<PoolGroupConfig> = match parse_json(json) {
        Ok(v) => v,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    miner_control(miner, |m| result_bool(block_on(m.set_pools_config(cfg))?))
}

/// Apply scaling configuration from JSON. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle; `json` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_set_scaling_config_json(
    miner: *const AsicMiner,
    json: *const c_char,
) -> i32 {
    let cfg: ScalingConfig = match parse_json(json) {
        Ok(v) => v,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    miner_control(miner, |m| result_bool(block_on(m.set_scaling_config(cfg))?))
}

/// Apply temperature configuration from JSON. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle; `json` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_set_temperature_config_json(
    miner: *const AsicMiner,
    json: *const c_char,
) -> i32 {
    let cfg: TemperatureConfig = match parse_json(json) {
        Ok(v) => v,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    miner_control(miner, |m| {
        result_bool(block_on(m.set_temperature_config(cfg))?)
    })
}

/// Apply tuning configuration from JSON. `scaling_json` may be null. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle; `config_json` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_set_tuning_config_json(
    miner: *const AsicMiner,
    config_json: *const c_char,
    scaling_json: *const c_char,
) -> i32 {
    let cfg: TuningConfig = match parse_json(config_json) {
        Ok(v) => v,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    let scaling: Option<ScalingConfig> = if scaling_json.is_null() {
        None
    } else {
        match parse_json(scaling_json) {
            Ok(v) => Some(v),
            Err(e) => {
                set_error(e);
                return -1;
            }
        }
    };
    miner_control(miner, |m| {
        result_bool(block_on(m.set_tuning_config(cfg, scaling))?)
    })
}

/// Apply fan configuration from JSON. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle; `json` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_set_fan_config_json(
    miner: *const AsicMiner,
    json: *const c_char,
) -> i32 {
    let cfg: FanConfig = match parse_json(json) {
        Ok(v) => v,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    miner_control(miner, |m| result_bool(block_on(m.set_fan_config(cfg))?))
}

/// Apply timezone configuration from JSON. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle; `json` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_set_timezone_config_json(
    miner: *const AsicMiner,
    json: *const c_char,
) -> i32 {
    let cfg: TimezoneConfig = match parse_json(json) {
        Ok(v) => v,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    miner_control(miner, |m| {
        result_bool(block_on(m.set_timezone_config(cfg))?)
    })
}

/// Set the fault light. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_set_fault_light(
    miner: *const AsicMiner,
    fault: bool,
) -> i32 {
    miner_control(miner, |m| result_bool(block_on(m.set_fault_light(fault))?))
}

/// Set a power limit in watts. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_set_power_limit(miner: *const AsicMiner, watts: f64) -> i32 {
    miner_control(miner, |m| {
        result_bool(block_on(m.set_power_limit(Power::from_watts(watts)))?)
    })
}

/// Set a manual tuning percent. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_set_tuning_percent(
    miner: *const AsicMiner,
    percent: u8,
) -> i32 {
    miner_control(miner, |m| {
        result_bool(block_on(m.set_tuning_percent(percent))?)
    })
}

/// Restart the miner. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_restart(miner: *const AsicMiner) -> i32 {
    miner_control(miner, |m| result_bool(block_on(m.restart())?))
}

/// Pause mining. `at_time_secs` < 0 means immediately. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_pause(miner: *const AsicMiner, at_time_secs: f64) -> i32 {
    miner_control(miner, |m| {
        result_bool(block_on(m.pause(optional_duration_secs(at_time_secs)))?)
    })
}

/// Resume mining. `at_time_secs` < 0 means immediately. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_resume(miner: *const AsicMiner, at_time_secs: f64) -> i32 {
    miner_control(miner, |m| {
        result_bool(block_on(m.resume(optional_duration_secs(at_time_secs)))?)
    })
}

/// Re-run discovery checks. JSON bool or null on error.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_revalidate_json(miner: *const AsicMiner) -> *mut c_char {
    miner_json(miner, |m| Ok(block_on(m.revalidate())?.ok()))
}

/// Factory reset. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_factory_reset(miner: *const AsicMiner) -> i32 {
    miner_control(miner, |m| result_bool(block_on(m.factory_reset())?))
}

/// Read logs as a newly allocated C string (or null on error / unsupported).
///
/// # Safety
/// `miner` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_read_logs(miner: *const AsicMiner) -> *mut c_char {
    clear_error();
    match with_miner(miner, |m| {
        block_on(m.read_logs())?.map_err(|e| e.to_string())
    }) {
        Ok(logs) => to_c_string(logs),
        Err(e) => {
            set_error(e);
            ptr::null_mut()
        }
    }
}

/// Change the miner password. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle; `password` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_change_password(
    miner: *mut AsicMiner,
    password: *const c_char,
) -> i32 {
    clear_error();
    let password = match cstr_to_str(password) {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    match with_miner_mut(miner, |m| {
        result_bool(block_on(m.change_password(&password))?)
    }) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(e) => {
            set_error(e);
            -1
        }
    }
}

/// Upgrade firmware from a local file path. Returns 1/0/-1.
///
/// # Safety
/// `miner` must be a live handle; `path` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_miner_upgrade_firmware(
    miner: *const AsicMiner,
    path: *const c_char,
) -> i32 {
    clear_error();
    let path = match cstr_to_str(path) {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    miner_control(miner, |m| {
        let image = block_on(FirmwareImage::from_file_async(&path))?.map_err(|e| e.to_string())?;
        result_bool(block_on(m.upgrade_firmware(image))?)
    })
}

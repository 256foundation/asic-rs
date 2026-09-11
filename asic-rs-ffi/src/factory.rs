use std::net::IpAddr;
use std::os::raw::c_char;
use std::ptr;
use std::sync::Mutex;

use asic_rs::MinerFactory;
use asic_rs::core::traits::miner::Miner as MinerTrait;

use crate::error::{clear_error, cstr_to_str, json_to_c_string, set_error, set_error_from};
use crate::miner::AsicMiner;
use crate::runtime::block_on;

/// Opaque factory handle. Owned by the caller; free with [`asic_rs_factory_free`].
pub struct AsicFactory {
    inner: Mutex<MinerFactory>,
}

fn with_factory<T>(
    factory: *const AsicFactory,
    f: impl FnOnce(&MinerFactory) -> Result<T, String>,
) -> Result<T, String> {
    if factory.is_null() {
        return Err("null factory handle".to_string());
    }
    // SAFETY: caller owns a live factory handle for the duration of the call.
    let factory = unsafe { &*factory };
    let guard = factory
        .inner
        .lock()
        .map_err(|e| format!("factory lock poisoned: {e}"))?;
    f(&guard)
}

fn with_factory_mut<T>(
    factory: *const AsicFactory,
    f: impl FnOnce(&mut MinerFactory) -> Result<T, String>,
) -> Result<T, String> {
    if factory.is_null() {
        return Err("null factory handle".to_string());
    }
    // SAFETY: caller owns a live factory handle for the duration of the call.
    let factory = unsafe { &*factory };
    let mut guard = factory
        .inner
        .lock()
        .map_err(|e| format!("factory lock poisoned: {e}"))?;
    f(&mut guard)
}

fn factory_update(
    factory: *const AsicFactory,
    update: impl FnOnce(MinerFactory) -> Result<MinerFactory, String>,
) -> Result<(), String> {
    with_factory_mut(factory, |inner| {
        let current = inner.clone();
        *inner = update(current)?;
        Ok(())
    })
}

fn parse_four_octets<'a>(
    o1: *const c_char,
    o2: *const c_char,
    o3: *const c_char,
    o4: *const c_char,
) -> Result<(&'a str, &'a str, &'a str, &'a str), &'static str> {
    Ok((
        cstr_to_str(o1)?,
        cstr_to_str(o2)?,
        cstr_to_str(o3)?,
        cstr_to_str(o4)?,
    ))
}

fn wrap_factory(inner: MinerFactory) -> *mut AsicFactory {
    Box::into_raw(Box::new(AsicFactory {
        inner: Mutex::new(inner),
    }))
}

fn wrap_miner(miner: Box<dyn MinerTrait>) -> *mut AsicMiner {
    Box::into_raw(Box::new(AsicMiner::new(miner)))
}

/// Create a new empty miner factory.
#[unsafe(no_mangle)]
pub extern "C" fn asic_rs_factory_new() -> *mut AsicFactory {
    clear_error();
    wrap_factory(MinerFactory::new())
}

/// Create a factory pre-loaded with hosts from a CIDR subnet (e.g. "192.168.1.0/24").
/// Returns null on error; see [`asic_rs_last_error`].
///
/// # Safety
/// `subnet` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_from_subnet(subnet: *const c_char) -> *mut AsicFactory {
    clear_error();
    let subnet = match cstr_to_str(subnet) {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return ptr::null_mut();
        }
    };
    match MinerFactory::from_subnet(subnet) {
        Ok(inner) => wrap_factory(inner),
        Err(e) => {
            set_error_from(e);
            ptr::null_mut()
        }
    }
}

/// Create a factory from an octet-range description (e.g. "192","168","1","1-255").
///
/// # Safety
/// All octet pointers must be valid C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_from_octets(
    o1: *const c_char,
    o2: *const c_char,
    o3: *const c_char,
    o4: *const c_char,
) -> *mut AsicFactory {
    clear_error();
    let (a, b, c, d) = match parse_four_octets(o1, o2, o3, o4) {
        Ok(v) => v,
        Err(e) => {
            set_error(e);
            return ptr::null_mut();
        }
    };
    match MinerFactory::from_octets(a, b, c, d) {
        Ok(inner) => wrap_factory(inner),
        Err(e) => {
            set_error_from(e);
            ptr::null_mut()
        }
    }
}

/// Create a factory from a compact range string (e.g. "192.168.1.1-255").
///
/// # Safety
/// `range` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_from_range(range: *const c_char) -> *mut AsicFactory {
    clear_error();
    let range = match cstr_to_str(range) {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return ptr::null_mut();
        }
    };
    match MinerFactory::from_range(range) {
        Ok(inner) => wrap_factory(inner),
        Err(e) => {
            set_error_from(e);
            ptr::null_mut()
        }
    }
}

/// Free a factory handle.
///
/// # Safety
/// `factory` must be null or a pointer previously returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_free(factory: *mut AsicFactory) {
    if !factory.is_null() {
        drop(Box::from_raw(factory));
    }
}

/// Append hosts from a CIDR subnet. Returns 0 on success, -1 on error.
///
/// # Safety
/// `factory` must be a live handle; `subnet` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_with_subnet(
    factory: *mut AsicFactory,
    subnet: *const c_char,
) -> i32 {
    clear_error();
    let subnet = match cstr_to_str(subnet) {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    match factory_update(factory, |inner| {
        inner.with_subnet(subnet).map_err(|e| e.to_string())
    }) {
        Ok(()) => 0,
        Err(e) => {
            set_error(e);
            -1
        }
    }
}

/// Append hosts from a range string. Returns 0 on success, -1 on error.
///
/// # Safety
/// `factory` must be a live handle; `range` must be a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_with_range(
    factory: *mut AsicFactory,
    range: *const c_char,
) -> i32 {
    clear_error();
    let range = match cstr_to_str(range) {
        Ok(s) => s,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    match factory_update(factory, |inner| {
        inner.with_range(range).map_err(|e| e.to_string())
    }) {
        Ok(()) => 0,
        Err(e) => {
            set_error(e);
            -1
        }
    }
}

/// Append hosts from octet ranges. Returns 0 on success, -1 on error.
///
/// # Safety
/// `factory` must be a live handle; octet pointers must be valid C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_with_octets(
    factory: *mut AsicFactory,
    o1: *const c_char,
    o2: *const c_char,
    o3: *const c_char,
    o4: *const c_char,
) -> i32 {
    clear_error();
    let (a, b, c, d) = match parse_four_octets(o1, o2, o3, o4) {
        Ok(v) => v,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    match factory_update(factory, |inner| {
        inner.with_octets(a, b, c, d).map_err(|e| e.to_string())
    }) {
        Ok(()) => 0,
        Err(e) => {
            set_error(e);
            -1
        }
    }
}

fn apply_factory(factory: *mut AsicFactory, update: impl FnOnce(MinerFactory) -> MinerFactory) {
    clear_error();
    if let Err(e) = factory_update(factory, |inner| Ok(update(inner))) {
        set_error(e);
    }
}

/// Enable or disable the initial port connectivity check.
///
/// # Safety
/// `factory` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_set_port_check(factory: *mut AsicFactory, enabled: bool) {
    apply_factory(factory, |inner| inner.with_port_check(enabled));
}

/// Set concurrent discovery limit.
///
/// # Safety
/// `factory` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_set_concurrent_limit(
    factory: *mut AsicFactory,
    limit: usize,
) {
    apply_factory(factory, |inner| inner.with_concurrent_limit(limit));
}

/// Set identification timeout in seconds.
///
/// # Safety
/// `factory` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_set_identification_timeout_secs(
    factory: *mut AsicFactory,
    secs: u64,
) {
    apply_factory(factory, |inner| {
        inner.with_identification_timeout_secs(secs)
    });
}

/// Set connectivity timeout in seconds.
///
/// # Safety
/// `factory` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_set_connectivity_timeout_secs(
    factory: *mut AsicFactory,
    secs: u64,
) {
    apply_factory(factory, |inner| inner.with_connectivity_timeout_secs(secs));
}

/// Set connectivity retries.
///
/// # Safety
/// `factory` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_set_connectivity_retries(
    factory: *mut AsicFactory,
    retries: u32,
) {
    apply_factory(factory, |inner| inner.with_connectivity_retries(retries));
}

/// Set nofile (RLIMIT_NOFILE) target for large scans.
///
/// # Safety
/// `factory` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_set_nofile_limit(factory: *mut AsicFactory, limit: u64) {
    apply_factory(factory, |inner| inner.with_nofile_limit(limit));
}

/// Enable or disable automatic nofile adjustment.
///
/// # Safety
/// `factory` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_set_nofile_adjustment(
    factory: *mut AsicFactory,
    enabled: bool,
) {
    apply_factory(factory, |inner| inner.with_nofile_adjustment(enabled));
}

/// Apply adaptive concurrency based on the current host list size.
///
/// # Safety
/// `factory` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_set_adaptive_concurrency(factory: *mut AsicFactory) {
    apply_factory(factory, |inner| inner.with_adaptive_concurrency());
}

/// Number of hosts currently configured for scanning. Returns -1 on error.
///
/// # Safety
/// `factory` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_len(factory: *const AsicFactory) -> i32 {
    clear_error();
    match with_factory(factory, |inner| Ok(inner.len() as i32)) {
        Ok(len) => len,
        Err(e) => {
            set_error(e);
            -1
        }
    }
}

/// Whether the factory has no hosts configured.
///
/// # Safety
/// `factory` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_is_empty(factory: *const AsicFactory) -> bool {
    clear_error();
    match with_factory(factory, |inner| Ok(inner.is_empty())) {
        Ok(empty) => empty,
        Err(e) => {
            set_error(e);
            true
        }
    }
}

/// JSON array of configured host IP strings. Free with [`asic_rs_free_string`].
///
/// # Safety
/// `factory` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_hosts_json(factory: *const AsicFactory) -> *mut c_char {
    clear_error();
    match with_factory(factory, |inner| {
        Ok(inner
            .hosts()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>())
    }) {
        Ok(hosts) => json_to_c_string(&hosts),
        Err(e) => {
            set_error(e);
            ptr::null_mut()
        }
    }
}

fn parse_ip(ip: *const c_char) -> Result<IpAddr, String> {
    let ip_str = cstr_to_str(ip).map_err(str::to_string)?;
    ip_str.parse::<IpAddr>().map_err(|e| e.to_string())
}

/// Discover and construct a miner at `ip`.
///
/// Returns 0 if found (`*out_miner` set), 1 if no supported miner responded
/// (`*out_miner` null), or -1 on error.
///
/// # Safety
/// `factory` must be a live handle, `ip` a valid C string, `out_miner` non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_get_miner(
    factory: *mut AsicFactory,
    ip: *const c_char,
    out_miner: *mut *mut AsicMiner,
) -> i32 {
    lookup_miner(factory, ip, out_miner, false)
}

/// Scan a single IP with the factory's port pre-check.
///
/// Return codes match [`asic_rs_factory_get_miner`].
///
/// # Safety
/// `factory` must be a live handle, `ip` a valid C string, `out_miner` non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_scan_miner(
    factory: *mut AsicFactory,
    ip: *const c_char,
    out_miner: *mut *mut AsicMiner,
) -> i32 {
    lookup_miner(factory, ip, out_miner, true)
}

fn lookup_miner(
    factory: *mut AsicFactory,
    ip: *const c_char,
    out_miner: *mut *mut AsicMiner,
    scan: bool,
) -> i32 {
    clear_error();
    if out_miner.is_null() {
        set_error("null out_miner");
        return -1;
    }
    unsafe {
        *out_miner = ptr::null_mut();
    }
    let ip_addr = match parse_ip(ip) {
        Ok(ip) => ip,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    let cloned = match with_factory(factory, |inner| Ok(inner.clone())) {
        Ok(inner) => inner,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    let result = if scan {
        block_on(cloned.scan_miner(ip_addr))
    } else {
        block_on(cloned.get_miner(ip_addr))
    };
    match result {
        Ok(Ok(Some(miner))) => {
            unsafe {
                *out_miner = wrap_miner(miner);
            }
            0
        }
        Ok(Ok(None)) => 1,
        Ok(Err(e)) => {
            set_error_from(e);
            -1
        }
        Err(e) => {
            set_error(e);
            -1
        }
    }
}

/// Scan all configured hosts.
///
/// On success: allocates `*out_miners` as an array of `*mut AsicMiner` of length
/// `*out_len`. Free each miner with [`asic_rs_miner_free`], then free the array
/// with [`asic_rs_free_miner_list`].
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
/// `factory` must be a live handle; `out_miners` and `out_len` must be non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_factory_scan(
    factory: *mut AsicFactory,
    out_miners: *mut *mut *mut AsicMiner,
    out_len: *mut usize,
) -> i32 {
    clear_error();
    if out_miners.is_null() || out_len.is_null() {
        set_error("null out_miners or out_len");
        return -1;
    }
    let cloned = match with_factory(factory, |inner| Ok(inner.clone())) {
        Ok(inner) => inner,
        Err(e) => {
            set_error(e);
            return -1;
        }
    };
    match block_on(cloned.scan()) {
        Ok(Ok(miners)) => {
            let handles: Box<[*mut AsicMiner]> = miners.into_iter().map(wrap_miner).collect();
            let len = handles.len();
            let ptr = Box::into_raw(handles) as *mut *mut AsicMiner;
            unsafe {
                *out_miners = ptr;
                *out_len = len;
            }
            0
        }
        Ok(Err(e)) => {
            set_error_from(e);
            -1
        }
        Err(e) => {
            set_error(e);
            -1
        }
    }
}

/// Free an array of miner pointers returned by [`asic_rs_factory_scan`].
/// Does **not** free the individual miners — call [`asic_rs_miner_free`] first.
///
/// # Safety
/// `list` must be null or the pointer previously returned via `out_miners`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_free_miner_list(list: *mut *mut AsicMiner, len: usize) {
    if list.is_null() {
        return;
    }
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(list, len)));
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::error::asic_rs_last_error;
    use std::ffi::CString;

    #[test]
    fn factory_from_range_has_three_hosts() {
        let range = CString::new("192.168.1.1-3").expect("static");
        let factory = unsafe { asic_rs_factory_from_range(range.as_ptr()) };
        assert!(!factory.is_null());
        let len = unsafe { asic_rs_factory_len(factory) };
        assert_eq!(len, 3);
        unsafe { asic_rs_factory_free(factory) };
    }

    #[test]
    fn factory_invalid_subnet_sets_error() {
        let subnet = CString::new("not-a-subnet").expect("static");
        let factory = unsafe { asic_rs_factory_from_subnet(subnet.as_ptr()) };
        assert!(factory.is_null());
        let err = asic_rs_last_error();
        assert!(!err.is_null());
    }
}

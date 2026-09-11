use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

pub(crate) fn set_error(msg: impl AsRef<str>) {
    let cleaned = msg.as_ref().replace('\0', "");
    let cstr = CString::new(cleaned).unwrap_or_else(|_| CString::from(c"error"));
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(cstr));
}

pub(crate) fn clear_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

pub(crate) fn set_error_from(err: impl std::fmt::Display) {
    set_error(err.to_string());
}

/// Returns the last error message for this thread (borrowed; do not free).
/// Valid until the next call into the library on this thread that sets an error.
#[unsafe(no_mangle)]
pub extern "C" fn asic_rs_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| match &*slot.borrow() {
        Some(s) => s.as_ptr(),
        None => ptr::null(),
    })
}

pub(crate) fn cstr_to_str<'a>(ptr: *const c_char) -> Result<&'a str, &'static str> {
    if ptr.is_null() {
        return Err("null string pointer");
    }
    // SAFETY: caller guarantees a valid C string for the duration of the call.
    let s = unsafe { CStr::from_ptr(ptr) };
    s.to_str().map_err(|_| "invalid UTF-8 string")
}

pub(crate) fn to_c_string(s: impl Into<String>) -> *mut c_char {
    match CString::new(s.into()) {
        Ok(cs) => cs.into_raw(),
        Err(_) => {
            set_error("string contained interior NUL");
            ptr::null_mut()
        }
    }
}

pub(crate) fn json_to_c_string<T: serde::Serialize>(value: &T) -> *mut c_char {
    match serde_json::to_string(value) {
        Ok(json) => to_c_string(json),
        Err(e) => {
            set_error_from(e);
            ptr::null_mut()
        }
    }
}

pub(crate) fn parse_json<'a, T: serde::Deserialize<'a>>(json: *const c_char) -> Result<T, String> {
    let s = cstr_to_str(json).map_err(str::to_string)?;
    serde_json::from_str(s).map_err(|e| e.to_string())
}

/// Free a string previously returned by this library.
///
/// # Safety
/// `s` must be null or a pointer previously returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asic_rs_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

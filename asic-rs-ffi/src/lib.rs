#![allow(unsafe_op_in_unsafe_fn)]

//! C ABI for [`asic-rs`](asic_rs).
//!
//! Complex values cross the FFI boundary as UTF-8 JSON strings. Opaque
//! pointers ([`AsicFactory`], [`AsicMiner`]) hide Rust ownership. All async
//! work is driven by an internal multi-thread Tokio runtime so callers see
//! synchronous C functions.
//!
//! # Memory
//! - Strings returned by this library must be freed with [`asic_rs_free_string`].
//! - Factory and miner handles must be freed with their respective free functions.
//! - On error, most functions return null / false / -1 and set a thread-local
//!   error message retrievable via [`asic_rs_last_error`].

mod error;
mod factory;
mod miner;
mod runtime;

pub use factory::AsicFactory;
pub use miner::AsicMiner;

use std::os::raw::c_char;

/// Library (FFI crate) version as a static C string. Do not free.
#[unsafe(no_mangle)]
pub extern "C" fn asic_rs_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

pub use error::{asic_rs_free_string, asic_rs_last_error};
pub use factory::asic_rs_free_miner_list;

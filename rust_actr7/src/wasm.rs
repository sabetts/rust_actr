//! WebAssembly entry points (built only for `wasm32-unknown-unknown`).
//!
//! A tiny C ABI over [`crate::run_from_source`], so the browser needs no
//! `wasm-bindgen`/`wasm-pack` tooling: JavaScript copies the model source into
//! wasm linear memory with [`alloc`], calls [`run_model`], then reads the trace
//! back through [`result_ptr`]/[`result_len`] and frees its buffer with
//! [`dealloc`].
//!
//! Build (the target's std ships in the conda-forge Rust toolchain's sysroot,
//! so no `rustup` is needed):
//!
//! ```text
//! cargo build --release --target wasm32-unknown-unknown
//! # -> target/wasm32-unknown-unknown/release/actr_engine.wasm
//! ```
//!
//! The wasm is single-threaded, so the result buffer is a `thread_local`
//! `RefCell` — no locks, no synchronization.

use std::cell::RefCell;

use crate::{run_from_source, DeviceSetup};

thread_local! {
    /// The most recent trace (or `error: …` text), read back via
    /// [`result_ptr`]/[`result_len`]. Reused across runs (capacity kept).
    static RESULT: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

/// Allocate `len` bytes in wasm memory and return the pointer. JavaScript
/// writes the model source there, calls [`run_model`], then frees it with
/// [`dealloc`].
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Free a buffer previously returned by [`alloc`].
///
/// # Safety
/// `ptr`/`len` must be exactly the pair returned by [`alloc`].
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    // SAFETY: the caller contract mirrors `alloc`.
    unsafe {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

/// Parse and run the model whose UTF-8 source sits at `src_ptr..src_ptr+len`.
///
/// Returns `0` on success and `1` on a load/run error; either way the trace, or
/// an `error: …` message, is readable via [`result_ptr`]/[`result_len`].
///
/// # Safety
/// `src_ptr` must point to `len` readable bytes (see [`alloc`]).
#[no_mangle]
pub unsafe extern "C" fn run_model(
    src_ptr: *const u8,
    len: usize,
    seed0: u64,
    seed1: u64,
    run_secs: f64,
) -> u32 {
    // SAFETY: the caller contract mirrors `alloc`.
    let bytes = unsafe { std::slice::from_raw_parts(src_ptr, len) };
    let source = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            set_result("error: model source is not valid UTF-8");
            return 1;
        }
    };
    match run_from_source(source, (seed0, seed1), run_secs, &DeviceSetup::default()) {
        Ok(trace) => {
            set_result(&trace);
            0
        }
        Err(e) => {
            set_result(&format!("error: {e}"));
            1
        }
    }
}

/// Pointer to the last result's bytes. Valid until the next [`run_model`].
#[no_mangle]
pub extern "C" fn result_ptr() -> *const u8 {
    RESULT.with(|r| r.borrow().as_ptr())
}

/// Byte length of the last result.
#[no_mangle]
pub extern "C" fn result_len() -> usize {
    RESULT.with(|r| r.borrow().len())
}

fn set_result(s: &str) {
    RESULT.with(|r| {
        let mut r = r.borrow_mut();
        r.clear();
        r.extend_from_slice(s.as_bytes());
    });
}

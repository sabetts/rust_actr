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

use crate::json::Json;
use crate::web_session::WebSession;
use crate::{run_from_source, DeviceSetup};

thread_local! {
    /// The most recent trace (or `error: �?�` text), read back via
    /// [`result_ptr`]/[`result_len`]. Reused across runs (capacity kept).
    static RESULT: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    static SESSION: RefCell<Option<WebSession>> = RefCell::new(None);
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

/// Create a fresh AGI command session.
#[no_mangle]
pub extern "C" fn session_init(seed0: u64, seed1: u64) -> u32 {
    match WebSession::new((seed0, seed1)) {
        Ok(session) => {
            SESSION.with(|slot| *slot.borrow_mut() = Some(session));
            0
        }
        Err(e) => {
            set_result(&format!("error: {e}"));
            1
        }
    }
}

/// Load model source into the current AGI session.
///
/// # Safety
/// `src_ptr` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn session_load_source(src_ptr: *const u8, len: usize) -> u32 {
    let source = match unsafe { read_text(src_ptr, len) } {
        Ok(text) => text,
        Err(e) => {
            set_result(&format!("error: {e}"));
            return 1;
        }
    };
    let result = SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(session) = slot.as_mut() else {
            return Err("session is not initialized".to_string());
        };
        session.load_source(&source)
    });
    match result {
        Ok(()) => {
            set_result("{\"ok\":true}");
            0
        }
        Err(e) => {
            set_result(&format!(
                "{{\"ok\":false,\"error\":{}}}",
                Json::Str(e).write()
            ));
            1
        }
    }
}

/// Evaluate an RPC/AGI command in the current session.
///
/// # Safety
/// The pointers must point to their stated number of readable bytes.
#[no_mangle]
pub unsafe extern "C" fn session_eval(
    name_ptr: *const u8,
    name_len: usize,
    args_ptr: *const u8,
    args_len: usize,
) -> u32 {
    let name = match unsafe { read_text(name_ptr, name_len) } {
        Ok(text) => text,
        Err(e) => {
            set_result(&format!("error: {e}"));
            return 1;
        }
    };
    let args_text = match unsafe { read_text(args_ptr, args_len) } {
        Ok(text) => text,
        Err(e) => {
            set_result(&format!("error: {e}"));
            return 1;
        }
    };
    let args = if args_text.trim().is_empty() {
        Vec::new()
    } else {
        match crate::json::parse(&args_text) {
            Ok(Json::Arr(items)) => items,
            Ok(_) => {
                set_result("error: session_eval arguments must be a JSON array");
                return 1;
            }
            Err(e) => {
                set_result(&format!("error: {e}"));
                return 1;
            }
        }
    };

    let result = SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(session) = slot.as_mut() else {
            return Err("session is not initialized".to_string());
        };
        session.eval(&name, &args)
    });
    match result {
        Ok(values) => {
            set_result(
                &Json::Obj(vec![
                    ("ok".into(), Json::Bool(true)),
                    ("result".into(), Json::Arr(values)),
                ])
                .write(),
            );
            0
        }
        Err(e) => {
            set_result(
                &Json::Obj(vec![
                    ("ok".into(), Json::Bool(false)),
                    ("error".into(), Json::Str(e)),
                ])
                .write(),
            );
            1
        }
    }
}

/// Take queued window, trace, and fired-command events as a JSON array.
#[no_mangle]
pub extern "C" fn session_take_events() -> u32 {
    let events = SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(session) = slot.as_mut() else {
            return Err("session is not initialized".to_string());
        };
        Ok(session.take_events())
    });
    match events {
        Ok(events) => {
            set_result(&Json::Arr(events).write());
            0
        }
        Err(e) => {
            set_result(&format!("error: {e}"));
            1
        }
    }
}

/// Prepare a stepped run in the current AGI session.
///
/// # Safety
/// `name_ptr` must point to `name_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn session_run_begin(name_ptr: *const u8, name_len: usize, secs: f64) -> u32 {
    let name = match unsafe { read_text(name_ptr, name_len) } {
        Ok(text) => text,
        Err(e) => {
            set_result(&format!("error: {e}"));
            return 1;
        }
    };
    let result = SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(session) = slot.as_mut() else {
            return Err("session is not initialized".to_string());
        };
        Ok(session.run_begin(&name, secs))
    });
    match result {
        Ok((limit, full, should_run)) => {
            set_result(
                &Json::Obj(vec![
                    ("limit".into(), Json::Int(limit)),
                    ("full".into(), Json::Bool(full)),
                    ("shouldRun".into(), Json::Bool(should_run)),
                ])
                .write(),
            );
            0
        }
        Err(e) => {
            set_result(&format!("error: {e}"));
            1
        }
    }
}

/// Run one AGI session slice.
#[no_mangle]
pub extern "C" fn session_run_chunk(limit_ms: f64, full: u32, max_events: u32) -> u32 {
    let result = SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(session) = slot.as_mut() else {
            return Err("session is not initialized".to_string());
        };
        session.run_chunk(limit_ms.round() as i64, full != 0, max_events as u64)
    });
    match result {
        Ok(more) => {
            set_result(&Json::Obj(vec![("more".into(), Json::Bool(more))]).write());
            0
        }
        Err(e) => {
            set_result(&format!("error: {e}"));
            1
        }
    }
}

unsafe fn read_text(ptr: *const u8, len: usize) -> Result<String, String> {
    if len == 0 {
        return Ok(String::new());
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| "argument is not valid UTF-8".to_string())
}

/// Pointer to the last result's bytes. Valid until the next operation.
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

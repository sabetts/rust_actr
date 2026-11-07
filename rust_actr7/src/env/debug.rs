//! Opt-in diagnostics for the native Environment's RPC client and windows.
//!
//! Set `ACTR_ENV_DEBUG=1` and redirect stderr to capture the request,
//! callback, and window-op sequence:
//!
//! ```powershell
//! $env:ACTR_ENV_DEBUG = "1"
//! cargo run --features env -- env 2> env-debug.log
//! ```

use std::sync::OnceLock;

pub(crate) fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("ACTR_ENV_DEBUG")
            .map(|v| !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false"))
            .unwrap_or(false)
    })
}

pub(crate) fn log(args: std::fmt::Arguments<'_>) {
    if enabled() {
        eprintln!("[actr-env] {args}");
    }
}

macro_rules! env_debug {
    ($($arg:tt)*) => {
        $crate::env::debug::log(format_args!($($arg)*))
    };
}

pub(crate) use env_debug;

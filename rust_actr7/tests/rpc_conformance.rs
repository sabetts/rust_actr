//! RPC conformance: start the `actr` binary (`actr server`), drive the vendored
//! `actr7.x/tutorial/python/actr.py` client against it, and compare the Python
//! test-models to the shared `.ref` files (`fixtures/rpc-enabled.txt`).
//!
//! Run with:
//!   cargo test --test rpc_conformance -- --ignored --nocapture
//!
//! Requires Python 3 and an ACT-R checkout; `cargo test` (non-ignored) never
//! invokes it, so the default build stays free of both.  The fixture driver
//! translates the upstream client's `ACT-R:` names to absolute filesystem
//! paths before they reach the server; the server itself is path-only.

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "requires Python 3 and an ACT-R checkout"]
fn rpc_conformance() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = crate_dir.join("fixtures").join("rpc-conformance.sh");
    if !script.is_file() {
        panic!("missing {}", script.display());
    }
    let status = Command::new("bash")
        .arg(&script)
        .arg("--expect")
        .status()
        .expect("failed to spawn rpc-conformance.sh");
    assert!(status.success(), "RPC conformance failed");
}

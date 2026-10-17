//! RPC conformance: start the `actr` binary (`actr server`), drive the vendored
//! `actr7.x/tutorial/python/actr.py` client against it, and compare the Python
//! test-models to the shared `.ref` files (`golden/rpc-enabled.txt`).
//!
//! Run with:
//!   cargo test --test rpc_conformance -- --ignored --nocapture
//!
//! Requires Python 3 and an ACT-R checkout; `cargo test` (non-ignored) never
//! invokes it, so the default build stays free of both.

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore]
fn rpc_conformance() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = crate_dir.join("golden").join("rpc-conformance.sh");
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

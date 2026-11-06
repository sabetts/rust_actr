//! Live differential cases: run the small fixtures under SBCL and the Rust
//! server, then compare their client traces. Requires SBCL, Python 3, and the
//! vendored ACT-R checkout, so this is ignored by default.
//!
//! Run with:
//!   cargo test --test differential -- --ignored --nocapture

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "requires Python 3, SBCL, and an ACT-R checkout"]
fn differential_cases() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = crate_dir
        .join("fixtures")
        .join("differential")
        .join("run_cases.py");
    if !script.is_file() {
        panic!("missing {}", script.display());
    }
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python".to_string());
    let status = Command::new(python)
        .arg(&script)
        .arg("--server")
        .arg("both")
        .arg("--isolate")
        .status()
        .expect("failed to spawn differential harness");
    assert!(status.success(), "differential cases failed");
}

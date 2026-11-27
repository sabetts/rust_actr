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

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

#[test]
#[ignore = "requires Python 3 and an ACT-R checkout"]
fn rpc_goal_conformance() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = crate_dir.join("fixtures").join("rpc-goal-smoke.py");
    let port = "2697";
    let mut server = Command::new(env!("CARGO_BIN_EXE_actr"))
        .args(["server", "--port", port])
        .spawn()
        .expect("spawn actr server");
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port.parse::<u16>().unwrap())).is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python".into());
    let mut child = Command::new(python)
        .arg(&script)
        .spawn()
        .expect("run goal smoke driver");
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                println!("PY-CLEAN-EXIT code={status}");
                break Some(status);
            }
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(100));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                println!("PY-TIMEOUT");
                break None;
            }
            Err(error) => panic!("waiting for goal smoke driver: {error}"),
        }
    };
    let _ = server.kill();
    let _ = server.wait();
    let status = status.expect("goal smoke driver timed out");
    assert!(status.success(), "goal replacement smoke test failed");
}

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

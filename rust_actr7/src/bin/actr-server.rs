//! ACT-R RPC command dispatcher — a drop-in replacement for the Lisp
//! `start-des` server (`framework/dispatcher.lisp:1330`) for the
//! Python/other-language clients.
//!
//! Usage:
//!   actr-server [--host 127.0.0.1] [--port 2650] [--actr <actr7.x>] \
//!               [--seed <n> <m>] [--no-port-file]
//!
//! Connects the vendored `tutorial/python/actr.py` client over the same
//! `\x04`-framed JSON protocol.

use std::path::PathBuf;
use std::process::ExitCode;

use actr_engine::rpc::server::{serve, write_port_file};

fn main() -> ExitCode {
    let mut host = "127.0.0.1".to_string();
    let mut port: u16 = 2650;
    let mut root = std::env::var("ACTR_DIR").unwrap_or_else(|_| default_root());
    let mut seed: (u64, u64) = (100, 0);
    let mut port_file = true;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                host = args.get(i + 1).cloned().unwrap_or(host);
                i += 2;
            }
            "--port" => {
                if let Some(p) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                    port = p;
                }
                i += 2;
            }
            "--actr" | "--actr-dir" => {
                if let Some(p) = args.get(i + 1) {
                    root = p.clone();
                }
                i += 2;
            }
            "--seed" => {
                let a = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(100);
                let b = args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(0);
                seed = (a, b);
                i += 3;
            }
            "--no-port-file" => {
                port_file = false;
                i += 1;
            }
            "-h" | "--help" => {
                eprintln!(
                    "actr-server [--host H] [--port P] [--actr DIR] [--seed S C] [--no-port-file]"
                );
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("actr-server: unknown argument {other:?}");
                return ExitCode::from(2);
            }
        }
    }

    let root = PathBuf::from(root);
    if !root.is_dir() {
        eprintln!(
            "actr-server: ACT-R checkout not found at {}",
            root.display()
        );
        return ExitCode::from(2);
    }

    let addr = format!("{host}:{port}");
    if port_file {
        if let Err(e) = write_port_file(port) {
            eprintln!("actr-server: could not write port file: {e}");
        }
    }
    eprintln!(
        "actr-server: listening on {addr} (ACT-R root {})",
        root.display()
    );
    match serve(&addr, root, seed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("actr-server: {e}");
            ExitCode::FAILURE
        }
    }
}

fn default_root() -> String {
    // <crate>/../actr7.x
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .map(|p| p.join("actr7.x"))
        .unwrap_or_else(|| PathBuf::from("actr7.x"))
        .to_string_lossy()
        .into_owned()
}

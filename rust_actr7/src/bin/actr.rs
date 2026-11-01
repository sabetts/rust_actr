//! ACT-R 7 single-threaded kernel (Rust port) — command-line entry point.
//!
//! Usage:
//!   actr run    [--seed <n> <m>] [--run <secs>] <model-file>
//!   actr server [--host <h>] [--port <p>] [--seed <n> <m>] [--no-port-file]
//!
//! `run` executes a single model file through the offline engine and prints its
//! trace exactly as `(sgp :v ...)` would write it: lines from reset through the
//! end of the run, including the `------ Stopped because ...` terminator.
//!
//! `<model-file>` is an absolute or current-directory-relative filesystem
//! path.  ACT-R logical names (`ACT-R:tutorial;...`) are not part of the
//! runtime.
//!
//! The last `define-model` in the file is the one that runs (ACT-R's
//! `define-model` currency). This is the single-file entry point; models that
//! need a task/setup file (device scaffolding, a scripted window, experiment
//! code) are beyond it — the library's `run_model_steps` plus a `DeviceSetup`
//! is the harness form.
//!
//! `server` starts the RPC dispatcher, a drop-in replacement for the Lisp
//! `start-des` server (`framework/dispatcher.lisp:1330`) for the
//! Python/other-language clients. It speaks the `\x04`-framed JSON protocol on
//! TCP (default `127.0.0.1:2650`) and writes `~/act-r-port-num.txt` so the
//! vendored `tutorial/python/actr.py` client auto-connects.

use std::process::ExitCode;

use actr_engine::rpc::server::{serve, write_port_file};
use actr_engine::{run_model, DeviceSetup};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("run") => cmd_run(&args[1..]),
        Some("server") => cmd_server(&args[1..]),
        #[cfg(feature = "env")]
        Some("env") => cmd_env(&args[1..]),
        // The GUI is feature-gated (see Cargo.toml); say so instead of
        // reporting "unknown command".
        #[cfg(not(feature = "env"))]
        Some("env") => {
            eprintln!(
                "actr env: the Environment GUI is not compiled into this build.\n\
                 Rebuild with the `env` feature:\n\n  \
                 cargo run --features env -- env\n"
            );
            ExitCode::from(2)
        }
        Some("-h") | Some("--help") | None => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("actr: unknown command {other:?}\n");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn print_help() {
    print!(
        "actr — ACT-R 7 single-threaded kernel (Rust port)

Usage:
  actr run    [--seed S C] [--run SECS] <model-file>
  actr server [--host H] [--port P] [--seed S C] [--no-port-file]

Commands:
  run     Run a single model file and print its trace.
  server  Start the RPC dispatcher (drop-in ACT-R server).
"
    );
    #[cfg(feature = "env")]
    print!("  env     Launch the native Environment GUI.\n");
    print!(
        "
Model arguments are filesystem paths (absolute or relative to the current directory).
"
    );
}

/// `actr run`: run one model file and print its trace.
fn cmd_run(args: &[String]) -> ExitCode {
    let mut seed: (u64, u64) = (100, 0);
    let mut run_time: f64 = 10.0;
    let mut model: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                let a = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(100);
                let b = args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(0);
                seed = (a, b);
                i += 3;
            }
            "--run" => {
                if let Some(t) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                    run_time = t;
                }
                i += 2;
            }
            "-h" | "--help" => {
                println!("actr run [--seed S C] [--run SECS] <model-file>");
                return ExitCode::SUCCESS;
            }
            other => {
                model = Some(other.to_string());
                i += 1;
            }
        }
    }

    let Some(model) = model else {
        eprintln!("actr run: missing model path (try --help)");
        return ExitCode::from(2);
    };
    if model
        .trim_start()
        .to_ascii_uppercase()
        .starts_with("ACT-R:")
        || model.contains(';')
    {
        eprintln!("actr run: ACT-R logical paths are not supported; pass a filesystem path");
        return ExitCode::from(2);
    }

    match run_model(&model, seed, run_time, &DeviceSetup::default()) {
        Ok(trace) => {
            print!("{trace}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("actr run: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `actr server`: start the RPC dispatcher.
fn cmd_server(args: &[String]) -> ExitCode {
    let mut host = "127.0.0.1".to_string();
    let mut port: u16 = 2650;
    let mut seed: (u64, u64) = (100, 0);
    let mut port_file = true;

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
                println!("actr server [--host H] [--port P] [--seed S C] [--no-port-file]");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("actr server: unknown argument {other:?}");
                return ExitCode::from(2);
            }
        }
    }

    let addr = format!("{host}:{port}");
    if port_file {
        if let Err(e) = write_port_file(&host, port) {
            eprintln!("actr server: could not write port file: {e}");
        }
    }
    eprintln!("actr server: listening on {addr}");
    match serve(&addr, seed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("actr server: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `actr env`: launch the native Environment GUI.
#[cfg(feature = "env")]
fn cmd_env(args: &[String]) -> ExitCode {
    let mut seed: (u64, u64) = (100, 0);
    let mut model: Option<String> = None;
    let mut connect = false;
    let mut host = "127.0.0.1".to_string();
    let mut port: u16 = 2650;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                let a = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(100);
                let b = args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(0);
                seed = (a, b);
                i += 3;
            }
            "--connect" => {
                connect = true;
                // An optional `host:port` may follow.
                if let Some(v) = args.get(i + 1) {
                    if let Some((h, p)) = v.split_once(':') {
                        host = h.to_string();
                        if let Ok(pv) = p.parse() {
                            port = pv;
                        }
                        i += 2;
                        continue;
                    }
                }
                i += 1;
            }
            "-h" | "--help" => {
                println!("actr env [--seed S C] [--connect [HOST:PORT]] [model-file]");
                return ExitCode::SUCCESS;
            }
            other => {
                model = Some(other.to_string());
                i += 1;
            }
        }
    }

    actr_engine::env::run(actr_engine::env::EnvArgs {
        host,
        port,
        connect,
        seed,
        model,
    })
}

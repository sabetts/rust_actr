//! ACT-R 7 single-threaded kernel (Rust port) — command-line entry point.
//!
//! Usage:
//!   actr run    [--seed <n> <m>] [--run <secs>] <model-file>
//!   actr server [--host <h>] [--port <p>] [--seed <n> <m>] [--no-port-file]
//!               [--env [model-file]]
//!   actr env    [--in-process | --connect [HOST:PORT]] [model-file]
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
//! vendored `tutorial/python/actr.py` client auto-connects. All connections
//! share one model session, as they do in Lisp. `--env` also opens the native
//! Environment as another client of that shared session.
//!
//! `env` is an RPC client by default: it reads `~/act-r-address.txt` and
//! `~/act-r-port-num.txt` like the Tcl Environment. Use `--in-process` to run
//! the engine locally instead, or `--connect HOST:PORT` to bypass the files.

#[cfg(feature = "env")]
use actr_engine::env::{EnvArgs, EnvTarget};
#[cfg(feature = "env")]
use actr_engine::rpc::server::serve_listener;
use actr_engine::rpc::server::{serve, write_port_file};
use actr_engine::{run_model, DeviceSetup};
#[cfg(feature = "env")]
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::process::ExitCode;

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
              [--env [MODEL]]

Commands:
  run     Run a single model file and print its trace.
  server  Start the RPC dispatcher (drop-in ACT-R server).
"
    );
    #[cfg(feature = "env")]
    print!(
        "          Add --env to open the native Environment attached to the same\n\
         \x20         shared session; an optional MODEL loads at startup.\n"
    );
    #[cfg(feature = "env")]
    print!(concat!(
        "  env     Launch the native Environment GUI (RPC client by default;\n",
        "          reads ~/act-r-address.txt and ~/act-r-port-num.txt).\n",
        "          Use --in-process for a local engine.\n",
    ));
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

#[derive(Debug, PartialEq, Eq)]
struct ParsedServerArgs {
    host: String,
    port: u16,
    seed: (u64, u64),
    port_file: bool,
    env: bool,
    model: Option<String>,
}

/// Parse `actr server` arguments. `Ok(None)` means help was requested.
fn parse_server_args(args: &[String]) -> Result<Option<ParsedServerArgs>, String> {
    let mut host = "127.0.0.1".to_string();
    let mut port: u16 = 2650;
    let mut seed: (u64, u64) = (100, 0);
    let mut port_file = true;
    let mut env = false;
    let mut model = None;

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
            "--env" => {
                env = true;
                i += 1;
                if let Some(next) = args.get(i) {
                    if !next.starts_with('-') {
                        model = Some(next.clone());
                        i += 1;
                    }
                }
            }
            "--model" => {
                model = args.get(i + 1).cloned();
                i += 2;
            }
            "-h" | "--help" => return Ok(None),
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    if model.is_some() && !env {
        return Err("--model requires --env".into());
    }
    Ok(Some(ParsedServerArgs {
        host,
        port,
        seed,
        port_file,
        env,
        model,
    }))
}

/// `actr server`: start the RPC dispatcher, optionally with the Environment.
fn cmd_server(args: &[String]) -> ExitCode {
    let parsed = match parse_server_args(args) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => {
            println!(
                "actr server [--host H] [--port P] [--seed S C] [--no-port-file] [--env [MODEL]]"
            );
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("actr server: {e}");
            return ExitCode::from(2);
        }
    };

    if parsed.env {
        #[cfg(feature = "env")]
        return cmd_server_with_env(parsed);
        #[cfg(not(feature = "env"))]
        {
            eprintln!(
                "actr server --env: the Environment GUI is not compiled into this build.\n\
                 Rebuild with the `env` feature:\n\n  \
                 cargo run --features env -- server --env"
            );
            return ExitCode::from(2);
        }
    }

    let addr = format!("{}:{}", parsed.host, parsed.port);
    if parsed.port_file {
        if let Err(e) = write_port_file(&parsed.host, parsed.port) {
            eprintln!("actr server: could not write port file: {e}");
        }
    }
    eprintln!("actr server: listening on {addr}");
    match serve(&addr, parsed.seed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("actr server: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `actr server --env`: bind the shared server, then run the Environment as a
/// normal RPC client of that server in the main thread.
#[cfg(feature = "env")]
fn cmd_server_with_env(parsed: ParsedServerArgs) -> ExitCode {
    let addr = format!("{}:{}", parsed.host, parsed.port);
    let listener = match TcpListener::bind(&addr) {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("actr server: {e}");
            return ExitCode::FAILURE;
        }
    };
    let bound = match listener.local_addr() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("actr server: {e}");
            return ExitCode::FAILURE;
        }
    };
    if parsed.port_file {
        if let Err(e) = write_port_file(&parsed.host, bound.port()) {
            eprintln!("actr server: could not write port file: {e}");
        }
    }
    eprintln!("actr server: listening on {bound}");
    let seed = parsed.seed;
    std::thread::spawn(move || {
        if let Err(e) = serve_listener(listener, seed) {
            eprintln!("actr server: {e}");
        }
    });
    let (host, port) = env_connect_address(bound);
    actr_engine::env::run(EnvArgs {
        target: EnvTarget::Server { host, port },
        seed,
        model: parsed.model,
    })
}

/// A wildcard bind address cannot be used as a client destination. The
/// Environment runs on the same machine as the server.
#[cfg(feature = "env")]
fn env_connect_address(addr: SocketAddr) -> (String, u16) {
    let host = match addr.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => "127.0.0.1".to_string(),
        IpAddr::V6(ip) if ip.is_unspecified() => "::1".to_string(),
        ip => ip.to_string(),
    };
    (host, addr.port())
}

#[cfg(feature = "env")]
#[derive(Debug, PartialEq, Eq)]
struct ParsedEnvArgs {
    target: EnvTarget,
    seed: (u64, u64),
    model: Option<String>,
}

/// Parse `actr env` arguments. `Ok(None)` means help was requested.
#[cfg(feature = "env")]
fn parse_env_args(args: &[String]) -> Result<Option<ParsedEnvArgs>, String> {
    let mut seed: (u64, u64) = (100, 0);
    let mut model: Option<String> = None;
    let mut connect_requested = false;
    let mut in_process = false;
    let mut explicit: Option<(String, u16)> = None;

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
                connect_requested = true;
                explicit = args.get(i + 1).and_then(|v| parse_host_port(v));
                i += if explicit.is_some() { 2 } else { 1 };
            }
            "--in-process" => {
                in_process = true;
                i += 1;
            }
            "-h" | "--help" => return Ok(None),
            other => {
                model = Some(other.to_string());
                i += 1;
            }
        }
    }

    if connect_requested && in_process {
        return Err("--connect and --in-process cannot be used together".into());
    }
    let target = if in_process {
        EnvTarget::InProcess
    } else if let Some((host, port)) = explicit {
        EnvTarget::Server { host, port }
    } else {
        EnvTarget::Discover
    };
    Ok(Some(ParsedEnvArgs {
        target,
        seed,
        model,
    }))
}

#[cfg(feature = "env")]
fn parse_host_port(value: &str) -> Option<(String, u16)> {
    let (host, port) = value.rsplit_once(':')?;
    let host = host.trim();
    let host = host.strip_prefix('[').unwrap_or(host);
    let host = host.strip_suffix(']').unwrap_or(host);
    let port = port.trim().parse::<u16>().ok()?;
    if host.is_empty() || port == 0 {
        None
    } else {
        Some((host.to_string(), port))
    }
}

/// `actr env`: launch the native Environment GUI.
#[cfg(feature = "env")]
fn cmd_env(args: &[String]) -> ExitCode {
    let parsed = match parse_env_args(args) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => {
            println!("actr env [--in-process | --connect [HOST:PORT]] [--seed S C] [model-file]");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("actr env: {e}");
            return ExitCode::from(2);
        }
    };
    actr_engine::env::run(EnvArgs {
        target: parsed.target,
        seed: parsed.seed,
        model: parsed.model,
    })
}

#[cfg(test)]
mod server_args_tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    fn parsed(values: &[&str]) -> ParsedServerArgs {
        parse_server_args(&args(values))
            .expect("parse")
            .expect("not help")
    }

    #[test]
    fn defaults_to_local_server_without_env() {
        let p = parsed(&[]);
        assert_eq!(p.host, "127.0.0.1");
        assert_eq!(p.port, 2650);
        assert_eq!(p.seed, (100, 0));
        assert!(p.port_file);
        assert!(!p.env);
        assert_eq!(p.model, None);
    }

    #[test]
    fn env_can_load_an_optional_model() {
        let p = parsed(&["--env"]);
        assert!(p.env);
        assert_eq!(p.model, None);
        let p = parsed(&["--env", "model.lisp"]);
        assert!(p.env);
        assert_eq!(p.model.as_deref(), Some("model.lisp"));
        let p = parsed(&["--env", "--port", "2651"]);
        assert!(p.env);
        assert_eq!(p.port, 2651);
        assert_eq!(p.model, None);
    }

    #[test]
    fn model_requires_env() {
        assert!(parse_server_args(&args(&["--model", "model.lisp"])).is_err());
    }

    #[test]
    fn unknown_args_are_rejected() {
        assert!(parse_server_args(&args(&["--bogus"])).is_err());
    }
}

#[cfg(all(test, feature = "env"))]
mod env_args_tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    fn parsed(values: &[&str]) -> ParsedEnvArgs {
        parse_env_args(&args(values))
            .expect("parse")
            .expect("not help")
    }

    #[test]
    fn defaults_to_home_discovery() {
        let p = parsed(&[]);
        assert_eq!(p.target, EnvTarget::Discover);
        assert_eq!(p.seed, (100, 0));
        assert_eq!(p.model, None);
    }

    #[test]
    fn in_process_is_an_opt_out() {
        assert_eq!(parsed(&["--in-process"]).target, EnvTarget::InProcess);
    }

    #[test]
    fn connect_without_address_uses_discovery() {
        assert_eq!(parsed(&["--connect"]).target, EnvTarget::Discover);
    }

    #[test]
    fn explicit_address_overrides_discovery() {
        let p = parsed(&["--connect", "10.0.0.11:2651", "model.lisp"]);
        assert_eq!(
            p.target,
            EnvTarget::Server {
                host: "10.0.0.11".into(),
                port: 2651,
            }
        );
        assert_eq!(p.model.as_deref(), Some("model.lisp"));
    }

    #[test]
    fn connect_and_in_process_conflict() {
        assert!(parse_env_args(&args(&["--connect", "--in-process"])).is_err());
        assert!(parse_env_args(&args(&["--connect", "127.0.0.1:2650", "--in-process"])).is_err());
    }

    #[test]
    fn wildcard_bind_uses_loopback_for_the_environment() {
        assert_eq!(
            env_connect_address("0.0.0.0:2650".parse().unwrap()),
            ("127.0.0.1".to_string(), 2650)
        );
        assert_eq!(
            env_connect_address("[::]:2650".parse().unwrap()),
            ("::1".to_string(), 2650)
        );
        assert_eq!(
            env_connect_address("192.168.1.5:2650".parse().unwrap()),
            ("192.168.1.5".to_string(), 2650)
        );
    }
}

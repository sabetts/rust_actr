//! Native ACT-R Environment (`actr env`).
//!
//! A GUI front-end that drives the engine through [`EnvBackend`]:
//! - **in-process** (default) — the GUI owns the [`crate::rpc::session::Session`];
//! - **client** (`--connect`) — the GUI is a real RPC client of `actr server`,
//!   speaking the same `\x04`-framed JSON protocol as `actr.py` and the Tcl
//!   environment.
//!
//! Compiled only with the `env` feature so the core engine, the golden test
//! suite, and the wasm cdylib stay dependency-free.

mod app;
mod backend;
mod client;
mod inproc;
mod settings;

pub use backend::EnvBackend;
pub use client::Client;
pub use inproc::InProc;

use std::path::PathBuf;
use std::process::ExitCode;

use crate::json::Json;

/// Parsed `actr env` options.
pub struct EnvArgs {
    /// The `actr7.x` checkout (in-process mode).
    pub actr_root: PathBuf,
    /// RPC host (client mode).
    pub host: String,
    /// RPC port (client mode).
    pub port: u16,
    /// Attach to a running server instead of hosting the engine in-process.
    pub connect: bool,
    /// `(sgp :seed …)` for in-process mode.
    pub seed: (u64, u64),
    /// An optional model to load at startup.
    pub model: Option<String>,
}

/// Launch the native Environment and return the process exit code.
pub fn run(args: EnvArgs) -> ExitCode {
    let mut backend: Box<dyn EnvBackend> = if args.connect {
        match Client::connect(&args.host, args.port) {
            Ok(c) => Box::new(c),
            Err(e) => {
                eprintln!("actr env: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        Box::new(InProc::new(args.actr_root.clone(), args.seed))
    };
    // The file dialog opens at the ACT-R root in-process; there is no local
    // root in client mode.
    let start_dir = if args.connect {
        None
    } else {
        Some(args.actr_root.clone())
    };

    if let Some(model) = &args.model {
        if let Err(e) = backend.evaluate("load-act-r-model", None, vec![Json::Str(model.clone())]) {
            eprintln!("actr env: could not load {model}: {e}");
        }
    }

    // Restore the last window size (the Tcl env persists window geometry).
    let settings = settings::load();
    let size = settings.window.unwrap_or((900.0, 600.0));
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([size.0, size.1])
            .with_title("ACT-R Environment"),
        ..Default::default()
    };
    match eframe::run_native(
        "ACT-R Environment",
        options,
        Box::new(move |_cc| Ok(Box::new(app::App::new(backend, start_dir, settings)))),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("actr env: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actr_dir() -> PathBuf {
        std::env::var("ACTR_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("actr7.x")
            })
    }

    /// The in-process backend loads, runs, and drains a trace headlessly —
    /// the plumbing the GUI panels sit on.
    #[test]
    fn inproc_loads_runs_and_drains_trace() {
        let root = actr_dir();
        if !root.is_dir() {
            eprintln!("skipping: no ACT-R checkout at {}", root.display());
            return;
        }
        let mut b = InProc::new(root, (100, 0));
        b.evaluate(
            "load-act-r-model",
            None,
            vec![Json::Str("tutorial;unit1;count.lisp".into())],
        )
        .expect("load-act-r-model");
        assert_eq!(b.current_model().as_deref(), Some("COUNT"));
        b.evaluate("run", None, vec![Json::Float(10.0)])
            .expect("run");
        let trace = b.take_trace();
        assert!(
            trace.iter().any(|l| l.contains("Stopped because")),
            "expected a stop line in trace: {trace:?}"
        );

        // The Phase 2 viewer commands are reachable through the backend.
        let prods = b
            .evaluate("all-productions", None, Vec::new())
            .expect("all-productions");
        assert!(
            matches!(prods.first(), Some(Json::Arr(a)) if !a.is_empty()),
            "expected productions, got {prods:?}"
        );
        let chunks = b
            .evaluate("filter-dm-chunks", None, vec![Json::Str("_none_".into())])
            .expect("filter-dm-chunks");
        assert!(matches!(chunks.first(), Some(Json::Arr(_))));
    }

    /// The in-process backend collects the visible-window display ops the
    /// engine pushes to the registered handler.
    #[test]
    fn inproc_collects_window_ops() {
        let root = actr_dir();
        if !root.is_dir() {
            eprintln!("skipping: no ACT-R checkout at {}", root.display());
            return;
        }
        let mut b = InProc::new(root, (100, 0));
        b.evaluate(
            "load-act-r-model",
            None,
            vec![Json::Str("tutorial;unit1;count.lisp".into())],
        )
        .expect("load-act-r-model");
        b.register_window_handler("env-vv-handler");
        b.evaluate(
            "add-virtual-window-handler",
            None,
            vec![Json::Str("env-vv-handler".into())],
        )
        .expect("add-virtual-window-handler");
        b.evaluate(
            "open-exp-window",
            None,
            vec![
                Json::Str("W".into()),
                Json::Obj(vec![
                    ("x".into(), Json::Int(10)),
                    ("y".into(), Json::Int(20)),
                ]),
            ],
        )
        .expect("open-exp-window");
        b.evaluate(
            "add-text-to-exp-window",
            None,
            vec![
                Json::Str("hi".into()),
                Json::Obj(vec![("x".into(), Json::Int(1)), ("y".into(), Json::Int(2))]),
            ],
        )
        .expect("add-text-to-exp-window");

        let ops = b.take_window_ops();
        let verbs: Vec<&str> = ops
            .iter()
            .filter_map(|o| o.first().and_then(|x| x.as_str()))
            .collect();
        assert!(verbs.contains(&"open"), "ops: {verbs:?}");
        assert!(verbs.contains(&"text"), "ops: {verbs:?}");
    }

    /// The interactive (chunked) run reaches its terminator and yields the
    /// trace, driven one chunk per `poll_run` as the GUI does.
    #[test]
    fn inproc_chunked_run_completes() {
        let root = actr_dir();
        if !root.is_dir() {
            eprintln!("skipping: no ACT-R checkout at {}", root.display());
            return;
        }
        let mut b = InProc::new(root, (100, 0));
        b.set_real_time(false);
        b.evaluate(
            "load-act-r-model",
            None,
            vec![Json::Str("tutorial;unit1;count.lisp".into())],
        )
        .expect("load-act-r-model");
        assert!(b.begin_run(10.0, false));
        let mut chunks = 0;
        while b.is_running() {
            if !b.poll_run().expect("poll_run") {
                break;
            }
            chunks += 1;
            assert!(chunks < 100_000, "run did not terminate");
        }
        let trace = b.take_trace();
        assert!(
            trace.iter().any(|l| l.contains("Stopped because")),
            "expected a terminator: {trace:?}"
        );
    }
}

//! ACT-R `test-models` corpus: Rust engine vs the vendored `.ref` traces.
//!
//! One test per ref lives in `tests/cases/<ident>.rs`; this file is the
//! shared harness ([`Case`] for engine drivers, [`SourceCase`] for source
//! echoes, device/spec parsing, session routing) and the flat module index
//! (`cases/mod.rs`). The refs and the model/task/tutorial sources are vendored
//! under `tests/cases/` and `fixtures/models/`, so the default test run needs
//! neither an ACT-R checkout nor SBCL. `ACTR_DIR` is only used by the
//! regeneration tools.
//!
//! Comparison mirrors ACT-R's own `compare-test-model-files`
//! (`actr7.x/test-models/model-tester.cl`): each line is right-trimmed and
//! compared case-insensitively (`string-equal`), line by line. That is what
//! the refs were produced/validated against.
//!
//! The data-heavy experiment drivers still reuse a recovered `.spec`
//! (`fixtures/regression/specs/`); every other engine case, including the
//! event-hook cases, owns its driver here. Source-echo cases use
//! `source_case!`, the engine's Lisp reader, or a small transform and check
//! the vendored source, not the engine.
//! Promote a case by removing its `#[ignore]` in its own file.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use actr_engine::json::Json;
use actr_engine::rpc::session::{sexp_to_json, Remote, Session};
use actr_engine::sexpr::{parse_all, Sexp};
use actr_engine::trace::Detail;
use actr_engine::{load_models, run_model_steps, ChunkDesc, DeviceSetup, DeviceText, Step};

mod common;
mod hooks;
use common::{crate_dir, first_diff, models_dir};

// ---------------------------------------------------------------- spec model

#[derive(Debug, Clone)]
struct Spec {
    name: String,
    ref_name: String,
    models: Vec<String>,
    /// Effective seed of each run, in order (only the first is needed: the
    /// engine seeds once; later changes are explicit `Step::Seed`s).
    run_seeds: Vec<(u64, u64)>,
    steps: Vec<Step>,
    /// Set when a step uses an op the scripted engine does not support
    /// (unused at run time; kept for the spec reader).
    #[allow(dead_code)]
    steps_unsupported: Option<String>,
    #[allow(dead_code)]
    scope: String,
    #[allow(dead_code)]
    error: Option<String>,
}

fn specs_dir() -> PathBuf {
    crate_dir()
        .join("fixtures")
        .join("regression")
        .join("specs")
}

/// Vendored expected outputs, one per case, next to its Rust test file.
fn refs_dir() -> PathBuf {
    crate_dir().join("tests").join("cases")
}

/// Hand-authored device sidecars (`<name>.device.sexp`).
fn devices_dir() -> PathBuf {
    crate_dir().join("fixtures").join("devices")
}

// ---------------------------------------------------------------- parsing

fn plist_get<'a>(items: &'a [Sexp], key: &str) -> Option<&'a Sexp> {
    let mut i = 0;
    while i + 1 < items.len() {
        if let Sexp::Sym(k) = &items[i] {
            if k == key {
                return Some(&items[i + 1]);
            }
        }
        i += 2;
    }
    None
}

fn as_str(e: &Sexp) -> Option<String> {
    match e {
        Sexp::Str(s) => Some(s.clone()),
        Sexp::Sym(s) => Some(s.clone()),
        _ => None,
    }
}

/// A bare symbol (not a string literal) — e.g. a `:module` name in a device
/// sidecar.
fn as_sym(e: &Sexp) -> Option<String> {
    match e {
        Sexp::Sym(s) => Some(s.clone()),
        _ => None,
    }
}

fn as_i64(e: &Sexp) -> Option<i64> {
    match e {
        Sexp::Int(i) => Some(*i),
        Sexp::Float(x) => Some(*x as i64),
        _ => None,
    }
}

fn list(e: &Sexp) -> Option<&Vec<Sexp>> {
    match e {
        Sexp::List(v) => Some(v),
        _ => None,
    }
}

/// Like `list`, but `NIL` becomes an empty list (specs for out-of-scope
/// tests legitimately have `:RUNS NIL` / `:MODELS NIL`).
fn list_or_empty(e: &Sexp) -> Vec<Sexp> {
    match e {
        Sexp::List(v) => v.clone(),
        Sexp::Sym(s) if s == "NIL" => Vec::new(),
        _ => Vec::new(),
    }
}

fn parse_seed(e: &Sexp) -> Option<(u64, u64)> {
    let v = list(e)?;
    if v.len() != 2 {
        return None;
    }
    let a = as_i64(&v[0])?;
    let b = as_i64(&v[1])?;
    Some((a as u64, b as u64))
}

fn parse_spec(src: &str) -> Result<Spec, String> {
    let forms = parse_all(src).map_err(|e| e.to_string())?;
    let items = forms
        .first()
        .and_then(list)
        .ok_or_else(|| "spec is not a top-level list".to_string())?;

    let name = plist_get(items, ":NAME")
        .and_then(as_str)
        .ok_or("no :NAME")?;
    let ref_name = plist_get(items, ":REF").and_then(as_str).ok_or("no :REF")?;
    let models = plist_get(items, ":MODELS")
        .map(list_or_empty)
        .unwrap_or_default()
        .iter()
        .filter_map(as_str)
        .collect();
    let run_seeds = plist_get(items, ":RUNS")
        .map(list_or_empty)
        .unwrap_or_default()
        .iter()
        .map(|r| {
            let rr = list(r).ok_or("run not a list")?;
            parse_seed(&rr[1]).ok_or("bad run seed".to_string())
        })
        .collect::<Result<Vec<_>, String>>()?;
    let (steps, steps_unsupported) = parse_steps(
        &plist_get(items, ":STEPS")
            .map(list_or_empty)
            .unwrap_or_default(),
    )?;
    let scope = plist_get(items, ":SCOPE")
        .and_then(as_str)
        .map(|s| s.trim_start_matches(':').to_string())
        .ok_or("no :SCOPE")?;
    let error = plist_get(items, ":ERROR")
        .and_then(as_str)
        .filter(|s| s != "NIL");

    Ok(Spec {
        name,
        ref_name,
        models,
        run_seeds,
        steps,
        steps_unsupported,
        scope,
        error,
    })
}

/// Parse the ordered `:STEPS` driver list. Returns the steps plus the first
/// unsupported op seen, if any (so the spec is ignored rather than misrun).
fn parse_steps(items: &[Sexp]) -> Result<(Vec<Step>, Option<String>), String> {
    let mut steps = Vec::new();
    let mut unsupported: Option<String> = None;
    let note = |u: &mut Option<String>, msg: String| {
        if u.is_none() {
            *u = Some(msg);
        }
    };
    for st in items {
        let v = match st {
            Sexp::List(v) => v,
            _ => continue,
        };
        let head = match v.first().and_then(|s| s.as_sym()) {
            Some(h) => h,
            None => continue,
        };
        match head {
            ":RUN" => {
                let secs = match v.get(1) {
                    Some(Sexp::Int(i)) => *i as f64,
                    Some(Sexp::Float(x)) => *x,
                    _ => {
                        note(&mut unsupported, format!("bad :RUN in {st:?}"));
                        continue;
                    }
                };
                let trace = matches!(v.get(3), Some(Sexp::Sym(s)) if s != "NIL");
                let width = v.get(4).and_then(as_i64).unwrap_or(22).max(1) as usize;
                steps.push(Step::Run { secs, trace, width });
            }
            ":RUN-FULL-TIME" => {
                let secs = match v.get(1) {
                    Some(Sexp::Int(i)) => *i as f64,
                    Some(Sexp::Float(x)) => *x,
                    _ => {
                        note(&mut unsupported, format!("bad :RUN-FULL-TIME in {st:?}"));
                        continue;
                    }
                };
                let trace = matches!(v.get(3), Some(Sexp::Sym(s)) if s != "NIL");
                let width = v.get(4).and_then(as_i64).unwrap_or(22).max(1) as usize;
                // The recorded seed is the RNG state at the run's start; unlike
                // `(:run ...)` it is never canonicalized to the `(100 0)`
                // placeholder, so re-apply it to resync the stream.
                let seed = v.get(2).and_then(parse_seed);
                steps.push(Step::RunFullTime {
                    secs,
                    trace,
                    width,
                    seed,
                });
            }
            ":RUN-UNTIL-TIME" => {
                let time = match v.get(1) {
                    Some(Sexp::Int(i)) => *i as f64,
                    Some(Sexp::Float(x)) => *x,
                    _ => {
                        note(&mut unsupported, format!("bad :RUN-UNTIL-TIME in {st:?}"));
                        continue;
                    }
                };
                let trace = matches!(v.get(3), Some(Sexp::Sym(s)) if s != "NIL");
                let width = v.get(4).and_then(as_i64).unwrap_or(22).max(1) as usize;
                steps.push(Step::RunUntilTime { time, trace, width });
            }
            ":RUN-N-EVENTS" => {
                let count = match v.get(1).and_then(as_i64) {
                    Some(i) if i > 0 => i as u64,
                    _ => {
                        note(&mut unsupported, format!("bad :RUN-N-EVENTS in {st:?}"));
                        continue;
                    }
                };
                let trace = matches!(v.get(3), Some(Sexp::Sym(s)) if s != "NIL");
                let width = v.get(4).and_then(as_i64).unwrap_or(22).max(1) as usize;
                steps.push(Step::RunNEvents {
                    count,
                    trace,
                    width,
                });
            }
            ":MODEL-OUTPUT" => {
                let s = match v.get(1) {
                    Some(Sexp::Str(s)) => s.clone(),
                    _ => {
                        note(&mut unsupported, format!("bad :MODEL-OUTPUT in {st:?}"));
                        continue;
                    }
                };
                steps.push(Step::ModelOutput(s));
            }
            ":GOAL-FOCUS" => {
                let item = match v.get(1) {
                    Some(i) => i,
                    None => {
                        note(&mut unsupported, "empty :GOAL-FOCUS".into());
                        continue;
                    }
                };
                let desc = match item {
                    Sexp::List(forms) => match actr_engine::model::parse_chunk_desc(forms) {
                        Ok(d) => d,
                        Err(e) => {
                            note(&mut unsupported, format!("goal-focus: {e}"));
                            continue;
                        }
                    },
                    Sexp::Sym(name) => ChunkDesc::Named(name.clone()),
                    Sexp::Str(name) => ChunkDesc::Named(name.clone()),
                    other => {
                        note(&mut unsupported, format!("bad :GOAL-FOCUS {other:?}"));
                        continue;
                    }
                };
                steps.push(Step::GoalFocus(desc));
            }
            ":RESET" => steps.push(Step::Reset),
            ":CONSUME-RNG" => match v.get(1).and_then(as_i64) {
                Some(n) if n >= 0 => steps.push(Step::ConsumeRng(n as u64)),
                other => note(&mut unsupported, format!("consume-rng {other:?}")),
            },
            ":SDM" => {
                let args: Vec<String> = v[1..]
                    .iter()
                    .map(|s| match s {
                        Sexp::Sym(x) => x.clone(),
                        Sexp::Str(x) => x.clone(),
                        other => format!("{other:?}"),
                    })
                    .collect();
                steps.push(Step::Sdm(args));
            }
            ":RECORD-HISTORY" => {
                let args: Vec<String> = v[1..]
                    .iter()
                    .filter_map(|s| s.as_sym().map(str::to_string))
                    .collect();
                steps.push(Step::RecordHistory(args));
            }
            ":PREDICT-BOLD" => steps.push(Step::PredictBold),
            ":PDISABLE" | ":PENABLE" => {
                let names: Vec<String> = v[1..]
                    .iter()
                    .filter_map(|s| s.as_sym().map(str::to_string))
                    .collect();
                if head == ":PDISABLE" {
                    steps.push(Step::PDisable(names));
                } else {
                    steps.push(Step::PEnable(names));
                }
            }
            ":DUMMY-EVENT" => match v.get(1).and_then(as_i64) {
                Some(ms) => steps.push(Step::DummyEvent(ms)),
                None => note(&mut unsupported, format!("dummy-event {st:?}")),
            },
            ":EMIT-WARNING" => match v.get(1) {
                Some(Sexp::Str(s)) => steps.push(Step::EmitWarning(s.clone())),
                other => note(&mut unsupported, format!("emit-warning {other:?}")),
            },
            ":START-HAND-AT-TOUCH" => match v.get(1).and_then(|x| x.as_sym()) {
                Some(h) => steps.push(Step::StartHandAtTouch(h.to_string())),
                None => note(&mut unsupported, format!("start-hand-at-touch {st:?}")),
            },
            ":MOD-FOCUS" => {
                let mut slots = Vec::new();
                let mut i = 1;
                while i + 1 < v.len() {
                    match v[i].as_sym() {
                        Some(k) => slots.push((k.to_string(), v[i + 1].clone())),
                        None => note(&mut unsupported, format!("mod-focus {st:?}")),
                    }
                    i += 2;
                }
                steps.push(Step::ModFocus(slots));
            }
            ":ADD-VISICON" => {
                let mut slots = Vec::new();
                for it in &v[1..] {
                    if let Sexp::List(p) = it {
                        if let Some(k) = p.first().and_then(|x| x.as_sym()) {
                            if let Some(val) = p.get(1) {
                                slots.push((k.to_string(), val.clone()));
                            }
                        }
                    }
                }
                steps.push(Step::AddVisicon(slots));
            }
            ":TRACKER-SETUP" => {
                let mut slots = Vec::new();
                for it in &v[1..] {
                    if let Sexp::List(p) = it {
                        if let Some(k) = p.first().and_then(|x| x.as_sym()) {
                            if let Some(val) = p.get(1) {
                                slots.push((k.to_string(), val.clone()));
                            }
                        }
                    }
                }
                steps.push(Step::TrackerSetup(slots));
            }
            ":ZZ-DRIVER" => steps.push(Step::ZzDriver),
            ":AL-DRIVER" => steps.push(Step::AlDriver),
            ":MODIFY-VISICON" => {
                let at = v.get(1).and_then(as_i64);
                let name = v.get(2).and_then(|x| x.as_sym()).map(str::to_string);
                let x = v.get(3).and_then(as_i64);
                let y = v.get(4).and_then(as_i64);
                match (at, name, x, y) {
                    (Some(at_ms), Some(name), Some(x), Some(y)) => {
                        steps.push(Step::ModifyVisicon { at_ms, name, x, y })
                    }
                    _ => note(&mut unsupported, format!("modify-visicon {st:?}")),
                }
            }
            ":BUFFER-CHUNK" => {
                let mut names = Vec::new();
                for it in &v[1..] {
                    match it {
                        Sexp::Sym(s) => names.push(s.clone()),
                        Sexp::Str(s) => names.push(s.clone()),
                        other => note(&mut unsupported, format!("buffer-chunk {other:?}")),
                    }
                }
                if !names.is_empty() {
                    steps.push(Step::BufferChunk(names));
                }
            }
            ":COMMAND" => {
                let name = match v.get(1) {
                    Some(Sexp::Sym(n)) | Some(Sexp::Str(n)) => n.to_lowercase(),
                    other => {
                        note(&mut unsupported, format!("bad :COMMAND {other:?}"));
                        continue;
                    }
                };
                // The generic wrappers record the `-fct` convention: a single
                // list argument is the command's argument list
                // (`(pp-fct '(RETRIEVE0))`), and `NIL` means no arguments.
                let raw: Vec<Sexp> = v[2..].to_vec();
                let args = if raw.len() == 1 {
                    match &raw[0] {
                        Sexp::List(items) => items.clone(),
                        Sexp::Sym(s) if s == "NIL" => Vec::new(),
                        other => vec![other.clone()],
                    }
                } else {
                    raw
                };
                steps.push(Step::Command { name, args });
            }
            ":SGP" | ":SSP" => {
                let mut i = 1;
                while i + 1 < v.len() {
                    let key = v[i].as_sym().unwrap_or("");
                    let val = &v[i + 1];
                    match key {
                        ":SEED" => match parse_seed(val) {
                            Some(s) => steps.push(Step::Seed(s)),
                            None => note(&mut unsupported, format!("seed step {val:?}")),
                        },
                        ":TRACE-DETAIL" => match val.as_sym() {
                            Some("HIGH") => steps.push(Step::SetDetail(Detail::High)),
                            Some("MEDIUM") => steps.push(Step::SetDetail(Detail::Medium)),
                            Some("LOW") => steps.push(Step::SetDetail(Detail::Low)),
                            _ => note(&mut unsupported, format!("trace-detail {val:?}")),
                        },
                        // `:v` is already encoded in each Run step's trace flag.
                        ":V" => {}
                        other => steps.push(Step::Sgp(other.to_string(), val.clone())),
                    }
                    i += 2;
                }
            }
            other => note(&mut unsupported, format!("step {other}")),
        }
    }
    Ok((steps, unsupported))
}

/// Parse the spec for one case by name (`specs/<name>.spec`).
fn read_named_spec(name: &str) -> Result<Spec, String> {
    let path = specs_dir().join(format!("{name}.spec"));
    let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_spec(&src).map_err(|e| format!("{}: {e}", path.display()))
}

// ---------------------------------------------------------------- compare

/// ACT-R `compare-test-model-files`: right-trim, case-insensitive, line by line.
fn normalize(s: &str) -> Vec<String> {
    s.lines()
        .map(|l| l.trim_end_matches([' ', '\r']).to_lowercase())
        .collect()
}

/// Per-case device sidecar `<name>.device.sexp` in `fixtures/devices/`, mirroring
/// the task file's post-reset `add-text-to-exp-window` / `clear-exp-window`
/// calls (same form as `tests/extra_traces.rs`): `((ADD-TEXT ms "text" x y) ...)`,
/// plus optional `(CLEAR ms)`, `(END ms)`, `(REWARD ms v)`, `(SIMILARITY a b v)`,
/// and `(RNG-DRAW ms count limit)` for task random draws before setup text.
fn load_spec_device(name: &str) -> Result<DeviceSetup, String> {
    let path = devices_dir().join(format!("{name}.device.sexp"));
    if !path.is_file() {
        return Ok(DeviceSetup::default());
    }
    let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let forms = parse_all(&src).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut device = DeviceSetup::default();
    // `(RUN n (ADD-TEXT …) …)` groups ops to a reset; a flat op re-arms on
    // every reset.
    let mut flat: Vec<Sexp> = Vec::new();
    for form in &forms {
        if let Some(v) = list(form) {
            if v.first().and_then(|s| s.as_sym()) == Some("RUN") {
                let run = as_i64(v.get(1).ok_or("missing run")?).ok_or("bad run")? as u32;
                for (i, op) in v[2..].iter().enumerate() {
                    parse_device_op(op, Some(run), i as u32, &mut device)
                        .map_err(|e| format!("{}: {e}", path.display()))?;
                }
                continue;
            }
        }
        flat.push(form.clone());
    }
    for (i, form) in flat.iter().enumerate() {
        parse_device_op(form, None, i as u32, &mut device)
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(device)
}

fn parse_device_op(
    form: &Sexp,
    run: Option<u32>,
    seq: u32,
    device: &mut DeviceSetup,
) -> Result<(), String> {
    {
        let v = list(form).ok_or_else(|| format!("bad device form: {form:?}"))?;
        let op = v.first().and_then(|s| s.as_sym()).unwrap_or("");
        let ms = || as_i64(v.get(1).ok_or("missing ms")?).ok_or("bad ms".to_string());
        match op {
            "RNG-DRAW" => device.rng_draws.push(actr_engine::engine::DeviceRng {
                at_ms: as_i64(v.get(1).ok_or("missing ms")?).ok_or("bad ms")?,
                count: as_i64(v.get(2).ok_or("missing count")?).ok_or("bad count")? as u64,
                limit: as_i64(v.get(3).ok_or("missing limit")?).ok_or("bad limit")? as u64,
                run,
            }),
            "ADD-TEXT" => device.texts.push(DeviceText {
                at_ms: as_i64(v.get(1).ok_or("missing ms")?).ok_or("bad ms")?,
                text: as_str(v.get(2).ok_or("missing text")?).ok_or("bad text")?,
                x: as_i64(v.get(3).ok_or("missing x")?).ok_or("bad x")?,
                y: as_i64(v.get(4).ok_or("missing y")?).ok_or("bad y")?,
                run,
                seq,
            }),
            "ADD-BUTTON" => device.buttons.push(actr_engine::engine::DeviceButton {
                at_ms: as_i64(v.get(1).ok_or("missing ms")?).ok_or("bad ms")?,
                text: as_str(v.get(2).ok_or("missing text")?).ok_or("bad text")?,
                x: as_i64(v.get(3).ok_or("missing x")?).ok_or("bad x")?,
                y: as_i64(v.get(4).ok_or("missing y")?).ok_or("bad y")?,
                width: as_i64(v.get(5).ok_or("missing width")?).ok_or("bad width")?,
                height: as_i64(v.get(6).ok_or("missing height")?).ok_or("bad height")?,
                color: as_str(v.get(7).ok_or("missing color")?).ok_or("bad color")?,
                // The `:action` form, kept verbatim as written by the task.
                action: v.get(8).and_then(as_str),
                run,
            }),
            "ADD-LINE" => device.lines.push(actr_engine::engine::DeviceLine {
                at_ms: as_i64(v.get(1).ok_or("missing ms")?).ok_or("bad ms")?,
                x1: as_i64(v.get(2).ok_or("missing x1")?).ok_or("bad x1")?,
                y1: as_i64(v.get(3).ok_or("missing y1")?).ok_or("bad y1")?,
                x2: as_i64(v.get(4).ok_or("missing x2")?).ok_or("bad x2")?,
                y2: as_i64(v.get(5).ok_or("missing y2")?).ok_or("bad y2")?,
                color: as_str(v.get(6).ok_or("missing color")?).ok_or("bad color")?,
                run,
            }),
            // `(MODIFY-LINE ms x1 y1 x2 y2 nx1 ny1 nx2 ny2)`: redraw the line
            // whose old endpoints match (window-relative).
            "MODIFY-LINE" => device
                .line_modifies
                .push(actr_engine::engine::DeviceLineModify {
                    at_ms: as_i64(v.get(1).ok_or("missing ms")?).ok_or("bad ms")?,
                    x1: as_i64(v.get(2).ok_or("missing x1")?).ok_or("bad x1")?,
                    y1: as_i64(v.get(3).ok_or("missing y1")?).ok_or("bad y1")?,
                    x2: as_i64(v.get(4).ok_or("missing x2")?).ok_or("bad x2")?,
                    y2: as_i64(v.get(5).ok_or("missing y2")?).ok_or("bad y2")?,
                    nx1: as_i64(v.get(6).ok_or("missing nx1")?).ok_or("bad nx1")?,
                    ny1: as_i64(v.get(7).ok_or("missing ny1")?).ok_or("bad ny1")?,
                    nx2: as_i64(v.get(8).ok_or("missing nx2")?).ok_or("bad nx2")?,
                    ny2: as_i64(v.get(9).ok_or("missing ny2")?).ok_or("bad ny2")?,
                    run,
                }),
            // `(REMOVE-LINE ms x1 y1 x2 y2)`: drop the line whose endpoints
            // match (window-relative).
            "REMOVE-LINE" => device
                .line_removes
                .push(actr_engine::engine::DeviceLineRemove {
                    at_ms: as_i64(v.get(1).ok_or("missing ms")?).ok_or("bad ms")?,
                    x1: as_i64(v.get(2).ok_or("missing x1")?).ok_or("bad x1")?,
                    y1: as_i64(v.get(3).ok_or("missing y1")?).ok_or("bad y1")?,
                    x2: as_i64(v.get(4).ok_or("missing x2")?).ok_or("bad x2")?,
                    y2: as_i64(v.get(5).ok_or("missing y2")?).ok_or("bad y2")?,
                    run,
                }),
            "MODIFY-TEXT" => device.modifies.push(actr_engine::engine::DeviceModify {
                at_ms: as_i64(v.get(1).ok_or("missing ms")?).ok_or("bad ms")?,
                text: as_str(v.get(2).ok_or("missing text")?).ok_or("bad text")?,
                x: as_i64(v.get(3).ok_or("missing x")?).ok_or("bad x")?,
                y: as_i64(v.get(4).ok_or("missing y")?).ok_or("bad y")?,
                run,
            }),
            "CLEAR" => device.clears.push((ms()?, run)),
            "REMOVE-TEXT" => device.removes.push(actr_engine::engine::DeviceRemove {
                at_ms: as_i64(v.get(1).ok_or("missing ms")?).ok_or("bad ms")?,
                text: as_str(v.get(2).ok_or("missing text")?).ok_or("bad text")?,
                run,
                seq,
            }),
            // `(PRINT-VISICON ms)` / `(PRINT-VISICON-LATE ms)`: a task
            // `print-visicon` called inside a run, which cannot be a step.
            // `(PRINT-VISICON-UPDATE ms)` is a monitor callback on the
            // `visicon-update` command (Sperling).
            "PRINT-VISICON" | "PRINT-VISICON-LATE" | "PRINT-VISICON-UPDATE" => {
                device.prints.push(actr_engine::engine::DevicePrint {
                    at_ms: ms()?,
                    late: op == "PRINT-VISICON-LATE",
                    after_update: op == "PRINT-VISICON-UPDATE",
                    run,
                    seq,
                })
            }
            // `(INSTALL)`: the task called `install-device`. Not a timed op;
            // it only records that a device exists (see
            // `DeviceSetup::installed`).
            "INSTALL" => device.installed = true,
            "WINDOW-VISIBLE" => {
                let value = v.get(1).ok_or("missing visibility")?;
                device.window_visible = Some(!matches!(value, Sexp::Sym(s) if s == "NIL"));
            }
            "COMMAND" => {
                // `(COMMAND ms output [module] details)`: the module is
                // present only for commands the task scheduled with an
                // explicit `:module` (e.g. `new-sound` → `AUDIO`).
                let (module, details) = if v.len() >= 5 {
                    (
                        as_sym(v.get(3).ok_or("missing module")?)
                            .ok_or("bad module")?
                            .to_uppercase(),
                        as_str(v.get(4).ok_or("missing details")?).ok_or("bad details")?,
                    )
                } else {
                    (
                        "NONE".to_string(),
                        as_str(v.get(3).ok_or("missing details")?).ok_or("bad details")?,
                    )
                };
                device.commands.push(actr_engine::engine::DeviceCommand {
                    at_ms: ms()?,
                    output: as_str(v.get(2).ok_or("missing output")?)
                        .ok_or("bad output")?
                        .to_lowercase(),
                    module,
                    details,
                    run,
                });
            }
            "SOUND" => device.sounds.push(actr_engine::engine::DeviceSound {
                at_ms: ms()?,
                freq: as_i64(v.get(2).ok_or("missing freq")?).ok_or("bad freq")?,
                duration: as_i64(v.get(3).ok_or("missing duration")?).ok_or("bad duration")?,
                run,
            }),
            "CLEAR-ON-KEYPRESS" => device.clear_on_keypress = true,
            "CONSUME-RNG" => {
                device.consume_rng =
                    as_i64(v.get(1).ok_or("missing count")?).ok_or("bad count")? as u64
            }
            "END" => device.end = Some(ms()?),
            "REWARD" => device.rewards.push((
                ms()?,
                as_i64(v.get(2).ok_or("missing reward")?).ok_or("bad reward")? as f32,
            )),
            "SIMILARITY" => device.sims.push((
                as_str(v.get(1).ok_or("missing a")?)
                    .ok_or("bad a")?
                    .to_uppercase(),
                as_str(v.get(2).ok_or("missing b")?)
                    .ok_or("bad b")?
                    .to_uppercase(),
                as_i64(v.get(3).ok_or("missing v")?).ok_or("bad v")? as f32,
            )),
            other => return Err(format!("bad device op {other} in {form:?}")),
        }
    }
    Ok(())
}

fn run_spec(root: &Path, spec: &Spec) -> Result<Vec<String>, String> {
    // A command-only spec has no run; the seed is unused, so any default
    // works (`(100 0)` is the corpus's canonical placeholder).
    let seed = spec.run_seeds.first().copied().unwrap_or((100, 0));
    let device = load_spec_device(&spec.name)?;
    // A spec with a recorded command step is driven through the RPC command
    // layer (`Session::evaluate`), so the spec exercises the same dispatch a
    // Python client hits; everything else keeps the engine-level driver.
    //
    // A `:STARTING-PARAMETERS` step also needs the session: its
    // `:testing-function` drives the reset hook, which lives there (`fan-no-pm`
    // and `fan` rely on `trace-fan-run` turning `:act` on for every run).
    // A file with more than one `define-model` needs the session's
    // multi-model meta-process (the model-name trace column).
    let multi_model = spec.models.iter().any(|m| {
        let rel = m.replace(';', std::path::MAIN_SEPARATOR_STR);
        load_models(&root.join(rel))
            .map(|defs| defs.len() > 1)
            .unwrap_or(false)
    });
    let needs_session = multi_model
        || spec.steps.iter().any(|s| match s {
            Step::Command { .. } => true,
            Step::Sgp(k, _) => k.eq_ignore_ascii_case(":STARTING-PARAMETERS"),
            _ => false,
        });
    let actual = if needs_session {
        run_spec_session(root, spec, &device)?
    } else {
        run_model_steps(root, &spec.models, seed, &spec.steps, &device)
            .map_err(|e| format!("engine error: {e}"))?
    };
    Ok(normalize(&actual))
}

/// A remote-command sink that refuses every callback: the spec corpus never
/// registers a client-owned command.
struct NoRemote;

impl Remote for NoRemote {
    fn call(&mut self, _n: &str, _m: Option<&str>, _a: Vec<Json>) -> Result<Vec<Json>, String> {
        Err("no remote client".into())
    }
}

/// Drive a spec through a [`Session`]: load the model chain, then apply each
/// step — commands via `evaluate`, the rest at the engine level, mirroring
/// `run_model_steps` so the two drivers cannot diverge on trace shape.
fn run_spec_session(root: &Path, spec: &Spec, device: &DeviceSetup) -> Result<String, String> {
    let seed = spec.run_seeds.first().copied().unwrap_or((100, 0));
    let mut session = Session::new(root.to_path_buf(), seed);
    let mut remote = NoRemote;
    for m in &spec.models {
        session
            .evaluate(
                "load-act-r-model",
                None,
                &[Json::Str(m.clone())],
                &mut remote,
            )
            .map_err(|e| format!("load {m}: {e}"))?;
    }
    session.set_device(device.clone())?;
    for step in &spec.steps {
        apply_session_step(&mut session, step, &mut remote)?;
    }
    Ok(session.take_trace_output())
}

fn apply_session_step(
    session: &mut Session,
    step: &Step,
    remote: &mut dyn Remote,
) -> Result<(), String> {
    let no_model = || "no model loaded".to_string();
    match step {
        Step::Command { name, args } => {
            let jargs: Vec<Json> = args.iter().map(cmd_arg_json).collect();
            session
                .evaluate(name, None, &jargs, remote)
                .map_err(|e| format!("command {name}: {e}"))?;
        }
        Step::Run { secs, trace, width } => {
            if session.is_multi() {
                session.run_multi_secs(*secs, *trace, *width)?;
            } else {
                let eng = session.engine_mut().ok_or_else(no_model)?;
                eng.set_module_width(*width);
                eng.set_trace(*trace);
                eng.run_secs(*secs).map_err(|e| e.to_string())?;
            }
        }
        Step::RunFullTime {
            secs,
            trace,
            width,
            seed,
        } => {
            let eng = session.engine_mut().ok_or_else(no_model)?;
            // `(100 0)` is the extractor's placeholder for a run seed it could
            // not match to a literal, so it is not the RNG state ACT-R was in.
            if let Some(s) = seed {
                if *s != (100, 0) {
                    eng.set_seed(*s);
                }
            }
            eng.set_module_width(*width);
            eng.set_trace(*trace);
            eng.run_full_time(*secs).map_err(|e| e.to_string())?;
        }
        Step::RunUntilTime { time, trace, width } => {
            let eng = session.engine_mut().ok_or_else(no_model)?;
            eng.set_module_width(*width);
            eng.set_trace(*trace);
            eng.run_until(*time).map_err(|e| e.to_string())?;
        }
        Step::RunNEvents {
            count,
            trace,
            width,
        } => {
            let eng = session.engine_mut().ok_or_else(no_model)?;
            eng.set_module_width(*width);
            eng.set_trace(*trace);
            eng.run_n_events(*count).map_err(|e| e.to_string())?;
        }
        Step::ModelOutput(s) => session.engine_mut().ok_or_else(no_model)?.model_output(s),
        Step::GoalFocus(desc) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .goal_focus(desc)
            .map_err(|e| e.to_string())?,
        Step::SetBufferChunk { buffer, desc } => session
            .engine_mut()
            .ok_or_else(no_model)?
            .set_buffer_chunk_desc(buffer, desc)
            .map_err(|e| e.to_string())?,
        Step::Seed(s) => session.engine_mut().ok_or_else(no_model)?.set_seed(*s),
        Step::ConsumeRng(n) => session.engine_mut().ok_or_else(no_model)?.consume_rng(*n),
        Step::Sdm(spec) => {
            session.engine_mut().ok_or_else(no_model)?.print_sdm(spec);
        }
        Step::SdmNames(spec) => {
            let eng = session.engine_mut().ok_or_else(no_model)?;
            let mark = eng.out_len();
            let names = eng.print_sdm(spec);
            eng.truncate_out(mark);
            eng.model_output(&format!("({})", names.join(" ")));
        }
        Step::SdmActivationTraces { spec, time_ms } => session
            .engine_mut()
            .ok_or_else(no_model)?
            .print_sdm_activation_traces(spec, *time_ms),
        Step::RetrievalHistoryTrace { at_ms, select } => session
            .engine_mut()
            .ok_or_else(no_model)?
            .print_retrieval_history_trace(*at_ms, select),
        Step::RegisterTaskCommand(name) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .register_task_command(name),
        Step::AddDm(descs) => {
            let eng = session.engine_mut().ok_or_else(no_model)?;
            for desc in descs {
                eng.add_dm_desc(desc).map_err(|e| e.to_string())?;
            }
        }
        Step::MergeDm(descs) => {
            let eng = session.engine_mut().ok_or_else(no_model)?;
            for desc in descs {
                eng.merge_dm_desc(desc).map_err(|e| e.to_string())?;
            }
        }
        Step::EmptyModuleRequest(buffer) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .empty_module_request(buffer)
            .map_err(|e| e.to_string())?,
        Step::SetTaskCommandValue { name, value } => session
            .engine_mut()
            .ok_or_else(no_model)?
            .set_task_command_value_sexp(name, value)
            .map_err(|e| e.to_string())?,
        Step::MoveAttention(loc) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .schedule_move_attention(loc),
        Step::AgiSetup => session.engine_mut().ok_or_else(no_model)?.agi_setup(),
        Step::AgiModify(phase) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .agi_modify(*phase),
        Step::CreatingImageSetup => session
            .engine_mut()
            .ok_or_else(no_model)?
            .creating_image_setup(),
        Step::BaseLevelInhibition => session
            .engine_mut()
            .ok_or_else(no_model)?
            .base_level_inhibition_driver()
            .map_err(|e| e.to_string())?,
        Step::AdaptiveNoiseTrace => session
            .engine_mut()
            .ok_or_else(no_model)?
            .adaptive_noise_trace()
            .map_err(|e| e.to_string())?,
        Step::RecordHistory(buffers) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .set_bold_buffers(buffers),
        Step::PredictBold => session.engine_mut().ok_or_else(no_model)?.print_bold(),
        Step::SetDetail(d) => session.engine_mut().ok_or_else(no_model)?.set_detail(*d),
        Step::Sgp(k, v) => {
            // `:starting-parameters` must go through the session, not straight
            // to the engine: its `:testing-function` drives the reset hook
            // (`invoke_testing_fn`), which reads the session's system-parameter
            // store. Routing it to the engine alone left the hook unknown, so a
            // spec whose runs depend on it (e.g. the fan stubs'
            // `trace-fan-run`) silently lost the trace it enables.
            if std::env::var("DBG_SGP").is_ok() {
                eprintln!("SGP step {k} = {v:?}");
            }
            if k.eq_ignore_ascii_case(":STARTING-PARAMETERS") {
                session.set_starting_params(v);
            } else {
                session
                    .engine_mut()
                    .ok_or_else(no_model)?
                    .apply_sgp(k, v)
                    .map_err(|e| e.to_string())?;
            }
        }
        Step::Reset => session.reset_model()?,
        Step::ResetSilent => {
            let eng = session.engine_mut().ok_or_else(no_model)?;
            eng.reset_silent().map_err(|e| e.to_string())?
        }
        Step::PDisable(names) => session.engine_mut().ok_or_else(no_model)?.pdisable(names),
        Step::PEnable(names) => session.engine_mut().ok_or_else(no_model)?.penable(names),
        Step::PBreak(names) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .pbreak_quiet(names),
        Step::BufferChunk(buffers) => {
            session
                .engine_mut()
                .ok_or_else(no_model)?
                .print_buffer_chunk(buffers);
        }
        Step::PrintedBufferChunk(buffers) => {
            session
                .engine_mut()
                .ok_or_else(no_model)?
                .print_printed_buffer_chunk(buffers);
        }
        Step::DummyEvent(at_ms) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .schedule_dummy(*at_ms),
        Step::EmitWarning(msg) => session.engine_mut().ok_or_else(no_model)?.emit_warning(msg),
        Step::StartHandAtTouch(hand) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .start_hand_at_touch(hand),
        Step::ModFocus(slots) => session.engine_mut().ok_or_else(no_model)?.mod_focus(slots),
        Step::AddVisicon(slots) => {
            session
                .engine_mut()
                .ok_or_else(no_model)?
                .add_visicon(slots);
        }
        Step::TrackerSetup(slots) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .tracker_setup(slots),
        Step::ZzDriver => session
            .engine_mut()
            .ok_or_else(no_model)?
            .zz_driver()
            .map_err(|e| e.to_string())?,
        Step::AlDriver => session
            .engine_mut()
            .ok_or_else(no_model)?
            .al_driver()
            .map_err(|e| e.to_string())?,
        Step::Zbrodoff(mode) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .zbrodoff_driver(*mode)
            .map_err(|e| e.to_string())?,
        Step::ZbrodoffProblem {
            addend1,
            addend2,
            sum,
            answer,
        } => session
            .engine_mut()
            .ok_or_else(no_model)?
            .zbrodoff_problem(addend1, addend2, sum, answer)
            .map_err(|e| e.to_string())?,
        Step::Onehit(mode) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .onehit_driver(*mode)
            .map_err(|e| e.to_string())?,
        Step::Bst(mode) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .bst_driver(*mode)
            .map_err(|e| e.to_string())?,
        Step::PcompGame(cfg) => session
            .engine_mut()
            .ok_or_else(no_model)?
            .pcomp_game(*cfg)
            .map_err(|e| e.to_string())?,
        Step::PcompUtilityNames => session
            .engine_mut()
            .ok_or_else(no_model)?
            .pcomp_utility_names(),
        Step::FanCollect => session
            .engine_mut()
            .ok_or_else(no_model)?
            .fan_collect_start(),
        Step::FanSummary => session
            .engine_mut()
            .ok_or_else(no_model)?
            .fan_no_pm_summary(),
        Step::FanExperimentSummary => session
            .engine_mut()
            .ok_or_else(no_model)?
            .fan_experiment_summary(),
        Step::PairedCollect => session
            .engine_mut()
            .ok_or_else(no_model)?
            .paired_collect_start(),
        Step::PairedSummary => session.engine_mut().ok_or_else(no_model)?.paired_summary(),
        Step::SperlingCollect => session
            .engine_mut()
            .ok_or_else(no_model)?
            .sperling_collect_start(),
        Step::SperlingSummary => session
            .engine_mut()
            .ok_or_else(no_model)?
            .sperling_summary(),
        Step::BstCollect => session
            .engine_mut()
            .ok_or_else(no_model)?
            .bst_collect_start(),
        Step::BstSummary => session.engine_mut().ok_or_else(no_model)?.bst_summary(),
        Step::SieglerDriver {
            first_seed,
            sets,
            rt,
            mp,
            ans,
            esc,
            fact_base,
            all_base,
        } => session
            .engine_mut()
            .ok_or_else(no_model)?
            .siegler_driver(
                *first_seed,
                *sets,
                *rt,
                *mp,
                *ans,
                *esc,
                *fact_base,
                *all_base,
            )
            .map_err(|e| e.to_string())?,
        Step::ModifyVisicon { at_ms, name, x, y } => session
            .engine_mut()
            .ok_or_else(no_model)?
            .schedule_modify_visicon(*at_ms, name, *x, *y),
    }
    Ok(())
}

/// Compare already-built output lines with a case's `.ref`. Used by pure
/// data/format drivers that do not need an engine instance.
pub fn check_output(name: &str, actual: &[String]) -> Result<(), String> {
    let ref_path = refs_dir().join(format!("{name}.ref"));
    let expected = normalize(
        &fs::read_to_string(&ref_path).map_err(|e| format!("{}: {e}", ref_path.display()))?,
    );
    let actual = normalize(&actual.join("\n"));
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{name}: ref mismatch vs {}\n{}",
            ref_path.display(),
            first_diff(&expected, &actual)
        ))
    }
}

// ---------------------------------------------------------------- harness

/// One per-case driver: models/seed/steps + the expected `.ref`.
///
/// Generated case files build this, then call [`Case::check`]. The driver is
/// the authority; only data-heavy experiments read a `.spec`.
pub struct Case {
    name: String,
    ref_name: String,
    models: Vec<String>,
    seed: (u64, u64),
    steps: Vec<Step>,
}

impl Case {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ref_name: format!("{name}.ref"),
            models: Vec::new(),
            seed: (100, 0),
            steps: Vec::new(),
        }
    }

    /// A case whose driver is still a recovered `.spec`: data-heavy
    /// experiment drivers (tens of thousands of runs).
    pub fn from_spec(name: &str) -> Self {
        let spec = read_named_spec(name).unwrap_or_else(|e| panic!("reading spec for {name}: {e}"));
        Self {
            name: spec.name,
            ref_name: spec.ref_name,
            models: spec.models,
            seed: spec.run_seeds.first().copied().unwrap_or((100, 0)),
            steps: spec.steps,
        }
    }

    pub fn ref_name(mut self, ref_name: &str) -> Self {
        self.ref_name = ref_name.to_string();
        self
    }

    pub fn steps(mut self, steps: Vec<Step>) -> Self {
        self.steps = steps;
        self
    }

    /// Wrap a recovered experiment spec with the paired-associate collector
    /// and its final `output-data` summary.
    pub fn paired_summary(mut self) -> Self {
        self.steps.insert(0, Step::PairedCollect);
        self.steps.push(Step::PairedSummary);
        self
    }

    /// Wrap the Sperling experiment spec with its per-trial score collector
    /// and final summary. The collector reports each completed 400-trial run,
    /// so the task's second experiment prompt stays between the two tables.
    pub fn sperling_summary(mut self) -> Self {
        // The extractor records the four-onset `permute-list` of a block as
        // task draws before the block's first trial. The block order is already
        // fixed by the device sidecar, and the task reseeds the model before
        // every trial, so only the 23 draws made after `:seed` affect that
        // trial's noise stream. Drop the four block-order draws from the
        // scripted consume so the trial stream matches ACT-R.
        for step in &mut self.steps {
            if let Step::ConsumeRng(n) = step {
                if *n == 27 {
                    *n = 23;
                }
            }
        }
        self.steps.insert(0, Step::SperlingCollect);
        self.steps.push(Step::SperlingSummary);
        self
    }

    /// Collect the 15 building-sticks PPM choices and print their experiment
    /// summary after the final run.
    pub fn bst_summary(mut self) -> Self {
        let at = self
            .steps
            .iter()
            .rposition(|s| matches!(s, Step::Command { name, .. } if name == "chunk-type"))
            .map(|i| i + 1)
            .unwrap_or(0);
        self.steps.insert(at, Step::BstCollect);
        self.steps.push(Step::BstSummary);
        self
    }

    pub fn model(mut self, model: &str) -> Self {
        self.models.push(model.to_string());
        self
    }

    pub fn models(mut self, models: &[&str]) -> Self {
        self.models = models.iter().map(|m| m.to_string()).collect();
        self
    }

    pub fn seed(mut self, seed: (u64, u64)) -> Self {
        self.seed = seed;
        self
    }

    pub fn step(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    /// Sugar for the common single-model `(run secs)` step.
    pub fn run(self, secs: f64) -> Self {
        self.step(Step::Run {
            secs,
            trace: true,
            width: 22,
        })
    }

    fn to_spec(&self) -> Spec {
        Spec {
            name: self.name.clone(),
            ref_name: self.ref_name.clone(),
            models: self.models.clone(),
            run_seeds: vec![self.seed],
            steps: self.steps.clone(),
            steps_unsupported: None,
            scope: "MULTI".to_string(),
            error: None,
        }
    }

    fn trace(&self) -> Result<Vec<String>, String> {
        let root = models_dir();
        if !root.is_dir() {
            return Err(format!(
                "vendored model fixtures not found at {}",
                root.display()
            ));
        }
        run_spec(&root, &self.to_spec())
    }

    /// Run the driver and compare with the case's `.ref` under ACT-R's own
    /// comparator (`compare-test-model-files`). `Err` carries the first
    /// divergence.
    pub fn check(self) -> Result<(), String> {
        let ref_path = refs_dir().join(&self.ref_name);
        let expected = normalize(
            &fs::read_to_string(&ref_path).map_err(|e| format!("{}: {e}", ref_path.display()))?,
        );
        let actual = self.trace()?;
        if actual == expected {
            Ok(())
        } else {
            Err(format!(
                "{}: ref mismatch vs {}\n{}",
                self.name,
                ref_path.display(),
                first_diff(&expected, &actual)
            ))
        }
    }

    /// Dev aid: run the driver and return the formatted trace.
    #[allow(dead_code)]
    pub fn run_only(self) -> Result<String, String> {
        Ok(self.trace()?.join("\n"))
    }
}

/// A `SCRAPE` case: the upstream stub re-read a tutorial/source file and
/// echoed a fragment. The Rust port has the same vendored sources, so the
/// oracle is that the ref fragment is still present in them, in order.
///
/// This deliberately does not emulate Common Lisp's `read-line`/paren/format
/// loops. The five refs whose upstream stubs used `(read f)` instead replay
/// through [`define_model_forms`]/[`echo_forms`]; only the four core-module
/// refs need engine-level chunk-type checks.
pub struct SourceCase {
    name: String,
    sources: Vec<String>,
    allow_prefix: bool,
}

impl SourceCase {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            sources: Vec::new(),
            allow_prefix: false,
        }
    }

    pub fn sources(mut self, sources: &[&str]) -> Self {
        self.sources = sources.iter().map(|s| s.to_string()).collect();
        self
    }

    /// The upstream stub truncated the final line (e.g. `(p read` for
    /// `(p read-and-create`); accept a ref line that prefixes a source line.
    pub fn allow_prefix(mut self) -> Self {
        self.allow_prefix = true;
        self
    }

    pub fn check(self) -> Result<(), String> {
        if self.sources.is_empty() {
            return Err(format!("{}: no source paths", self.name));
        }
        let ref_path = refs_dir().join(format!("{}.ref", self.name));
        let expected = normalize(
            &fs::read_to_string(&ref_path).map_err(|e| format!("{}: {e}", ref_path.display()))?,
        );
        let mut source = Vec::new();
        for path in &self.sources {
            source.extend(source_lines(path)?);
        }
        check_source_fragment(&self.name, &expected, &source, self.allow_prefix)
    }
}

/// Read one vendored source file by its ACT-R logical path (`;` separated).
pub fn source_text(logical: &str) -> Result<String, String> {
    let path = models_dir().join(logical.replace(';', std::path::MAIN_SEPARATOR_STR));
    fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Read a vendored source as lines.
pub fn source_lines(logical: &str) -> Result<Vec<String>, String> {
    Ok(source_text(logical)?.lines().map(str::to_owned).collect())
}

/// Parse a source with the engine's Lisp reader and return the children of
/// its first `(define-model ...)` form.
pub fn define_model_forms(logical: &str) -> Result<Vec<Sexp>, String> {
    let src = source_text(logical)?;
    let forms = parse_all(&src).map_err(|e| format!("{logical}: {e}"))?;
    for form in forms {
        if let Sexp::List(items) = form {
            if items.first().and_then(Sexp::as_sym) == Some("DEFINE-MODEL") {
                return Ok(items);
            }
        }
    }
    Err(format!("{logical}: no define-model form"))
}

/// Render a form like Lisp's `~(~s~)`: single spaces, lowercased symbols.
pub fn sexp_show(form: &Sexp) -> String {
    match form {
        Sexp::Sym(s) => s.to_lowercase(),
        Sexp::Int(i) => i.to_string(),
        Sexp::Float(x) => x.to_string(),
        Sexp::Str(s) => format!(
            "\"{}\"",
            s.replace('\\', "\\\\").replace('"', "\\\"").to_lowercase()
        ),
        Sexp::List(items) => format!(
            "({})",
            items.iter().map(sexp_show).collect::<Vec<_>>().join(" ")
        ),
    }
}

/// Select child forms of `(define-model ...)` by head symbol and optional
/// second element, then render them with [`sexp_show`].
pub fn echo_forms(logical: &str, head: &str, name: Option<&str>) -> Result<Vec<String>, String> {
    let head = head.to_uppercase();
    let name = name.map(str::to_uppercase);
    let forms = define_model_forms(logical)?;
    Ok(forms
        .iter()
        .filter_map(|form| {
            let Sexp::List(items) = form else {
                return None;
            };
            if items.first().and_then(Sexp::as_sym) != Some(head.as_str()) {
                return None;
            }
            if let Some(name) = &name {
                if items.get(1).and_then(Sexp::as_sym) != Some(name.as_str()) {
                    return None;
                }
            }
            Some(sexp_show(form))
        })
        .collect())
}

fn check_source_fragment(
    name: &str,
    expected: &[String],
    source: &[String],
    allow_prefix: bool,
) -> Result<(), String> {
    let mut at = 0usize;
    let mut seen = false;
    for (idx, want) in expected.iter().enumerate() {
        if want.trim().is_empty() {
            continue;
        }
        seen = true;
        let found = source.iter().enumerate().skip(at).find(|(_, got)| {
            let got = got.trim_end().to_lowercase();
            got == *want || (allow_prefix && got.starts_with(want.as_str()))
        });
        let Some((i, _)) = found else {
            return Err(format!(
                "{name}: ref line {} not found in source after line {}: {:?}",
                idx + 1,
                at,
                want
            ));
        };
        at = i + 1;
    }
    if !seen {
        return Err(format!("{name}: empty ref"));
    }
    Ok(())
}

/// Capture a balanced-parenthesis block starting at the first line containing
/// `start`, truncating the final line when the upstream stub did.
pub fn balanced_block(lines: &[String], start: &str) -> Result<Vec<String>, String> {
    let needle = start.to_lowercase();
    let start_idx = lines
        .iter()
        .position(|line| line.to_lowercase().contains(&needle))
        .ok_or_else(|| format!("source does not contain {start:?}"))?;
    let mut out = Vec::new();
    let mut open = 0i32;
    for line in &lines[start_idx..] {
        out.push(line.clone());
        open += line.matches('(').count() as i32;
        open -= line.matches(')').count() as i32;
        if open <= 0 {
            if open < 0 {
                let drop = (-open) as usize;
                let last = out.last_mut().unwrap();
                let keep = last.chars().count().saturating_sub(drop);
                *last = last.chars().take(keep).collect();
            }
            return Ok(out);
        }
    }
    Err(format!("unbalanced block starting at {start:?}"))
}

/// Recreate the upstream `format` column layouts used by the paired/triple
/// production refs: each block is padded to the widest line, then separated by
/// `gap` spaces.
pub fn columns(lines: &[String], starts: &[&str], gap: usize) -> Result<Vec<String>, String> {
    if starts.len() < 2 {
        return Err("columns needs at least two blocks".to_string());
    }
    let mut blocks = Vec::new();
    for start in starts {
        blocks.push(balanced_block(lines, start)?);
    }
    let rows = blocks.iter().map(Vec::len).max().unwrap_or(0);
    let width = blocks
        .iter()
        .flatten()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = Vec::with_capacity(rows);
    for row in 0..rows {
        let mut line = String::new();
        for (i, block) in blocks.iter().enumerate() {
            let text = block.get(row).map(String::as_str).unwrap_or("");
            if i + 1 == blocks.len() {
                line.push_str(text);
            } else {
                line.push_str(&format!("{text:<width$}"));
                line.push_str(&" ".repeat(gap));
            }
        }
        out.push(line);
    }
    Ok(out)
}

/// Compare custom source-echo output with the case's `.ref`.
pub fn check_echo(name: &str, actual: Vec<String>) -> Result<(), String> {
    let ref_path = refs_dir().join(format!("{name}.ref"));
    let expected = normalize(
        &fs::read_to_string(&ref_path).map_err(|e| format!("{}: {e}", ref_path.display()))?,
    );
    let actual = normalize(&actual.join("\n"));
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{name}: ref mismatch vs {}\n{}",
            ref_path.display(),
            first_diff(&expected, &actual)
        ))
    }
}

/// Parse one Lisp form. Used by generated drivers for `Sexp`-valued steps.
pub fn sexp(src: &str) -> Sexp {
    let mut forms = parse_all(src).unwrap_or_else(|e| panic!("bad sexp {src:?}: {e}"));
    assert_eq!(forms.len(), 1, "expected one form in {src:?}");
    forms.remove(0)
}

/// Parse a Lisp argument list; `NIL`/empty is the empty list.
pub fn sexps(src: &str) -> Vec<Sexp> {
    let s = src.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("nil") {
        return Vec::new();
    }
    let forms = parse_all(s).unwrap_or_else(|e| panic!("bad sexp list {src:?}: {e}"));
    if forms.len() == 1 {
        match forms.into_iter().next().unwrap() {
            Sexp::List(items) => items,
            other => vec![other],
        }
    } else {
        forms
    }
}

/// Parse a goal-focus chunk description.
pub fn chunk_desc(src: &str) -> ChunkDesc {
    match sexp(src) {
        Sexp::List(items) => actr_engine::model::parse_chunk_desc(&items)
            .unwrap_or_else(|e| panic!("bad chunk desc {src:?}: {e}")),
        Sexp::Sym(name) | Sexp::Str(name) => ChunkDesc::Named(name),
        other => panic!("bad chunk desc {other:?}"),
    }
}

/// A case that exists in the upstream corpus but has no Rust driver yet.
/// The body panics so `--ignored` reports the gap instead of silently passing.
macro_rules! ignored_case {
    ($ident:ident, $name:literal, $reason:literal) => {
        #[test]
        #[ignore = $reason]
        fn $ident() {
            panic!("case {} is not implemented: {}", $name, $reason);
        }
    };
}

/// A source-echo case: assert the upstream ref fragment still exists in the
/// vendored source, in order. See [`SourceCase`].
macro_rules! source_case {
    ($ident:ident, $name:literal, [$($src:literal),+ $(,)?] $(,)?) => {
        #[test]
        fn $ident() -> Result<(), String> {
            crate::SourceCase::new($name)
                .sources(&[$($src),+])
                .check()
        }
    };
    ($ident:ident, $name:literal, [$($src:literal),+ $(,)?], prefix $(,)?) => {
        #[test]
        fn $ident() -> Result<(), String> {
            crate::SourceCase::new($name)
                .sources(&[$($src),+])
                .allow_prefix()
                .check()
        }
    };
}

mod cases;

/// The per-case files and `cases/mod.rs` are hand-maintained; this catches a
/// forgotten `mod` line (which would otherwise silently skip a test).
#[test]
fn case_files_cover_every_ref() -> Result<(), String> {
    let dir = refs_dir();
    let mut refs = BTreeSet::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|x| x.to_str()) == Some("ref") {
            refs.insert(path.file_stem().unwrap().to_string_lossy().into_owned());
        }
    }

    let mut files = BTreeSet::new();
    let mut cases = BTreeSet::new();
    let mut case_refs = BTreeSet::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|x| x.to_str()) != Some("rs") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        if stem == "mod" {
            continue;
        }
        files.insert(stem);
        let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut case = None;
        let mut ref_name = None;
        for line in src.lines().take(8) {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("//! case:") {
                case = Some(v.trim().to_string());
            }
            if let Some(v) = line.strip_prefix("//! ref:") {
                ref_name = Some(v.trim().to_string());
            }
        }
        let case = case.ok_or_else(|| format!("{}: missing `//! case:`", path.display()))?;
        if !cases.insert(case.clone()) {
            return Err(format!("duplicate case header: {case}"));
        }
        let ref_name = ref_name.unwrap_or_else(|| format!("{case}.ref"));
        let ref_stem = ref_name.strip_suffix(".ref").unwrap_or(&ref_name);
        case_refs.insert(ref_stem.to_string());
    }

    let index = fs::read_to_string(dir.join("mod.rs"))
        .map_err(|e| format!("{}: {e}", dir.join("mod.rs").display()))?;
    let mods: BTreeSet<String> = index
        .lines()
        .filter_map(|l| {
            l.trim()
                .strip_prefix("mod ")?
                .strip_suffix(';')
                .map(str::to_string)
        })
        .collect();

    let mut problems = Vec::new();
    for r in refs.difference(&case_refs) {
        problems.push(format!("ref has no case file: {r}"));
    }
    for r in case_refs.difference(&refs) {
        problems.push(format!("case file names a missing ref: {r}"));
    }
    for f in files.difference(&mods) {
        problems.push(format!("case file not declared in cases/mod.rs: {f}.rs"));
    }
    for m in mods.difference(&files) {
        problems.push(format!("cases/mod.rs names a missing file: {m}.rs"));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// `sexp_to_json` for a recorded command's arguments, but keeping Lisp strings
/// distinct from symbol names (`json_to_sexp` cannot tell them apart, and the
/// generic command path uppercases symbols).
fn cmd_arg_json(s: &Sexp) -> Json {
    match s {
        Sexp::Str(x) => Json::Obj(vec![("str".to_string(), Json::Str(x.clone()))]),
        Sexp::List(items) => Json::Arr(items.iter().map(cmd_arg_json).collect()),
        other => sexp_to_json(other),
    }
}

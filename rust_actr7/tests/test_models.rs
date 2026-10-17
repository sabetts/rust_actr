//! ACT-R `test-models` corpus: Rust engine vs the checked-in `.ref` traces.
//!
//! Phase 1 (`fixtures/regen-specs.sh`) runs the Lisp stubs under
//! instrumentation and writes `fixtures/regression/specs/*.spec` describing
//! each test's model chain, resolved seed, and ordered driver steps. This
//! harness consumes those specs plus `actr7.x/test-models/*.ref`, so
//! `cargo test` never needs SBCL.
//!
//! Comparison mirrors ACT-R's own `compare-test-model-files`
//! (`actr7.x/test-models/model-tester.cl`): each line is right-trimmed and
//! compared case-insensitively (`string-equal`), line by line. That is what
//! the refs were produced/validated against.
//!
//! `PURE`/`MULTI` trace specs the scripted engine can drive (repeated `run`,
//! `goal-focus`, `:v`/`:trace-detail`/`:seed`) are runnable; the rest are
//! reported as ignored with a reason so the coverage gap is visible and only
//! shrinks. `enabled.txt` is the checked-in allowlist that must stay green;
//! `probe_runnable` (ignored test) reports which other specs already pass.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use actr_engine::json::Json;
use actr_engine::rpc::session::{sexp_to_json, Remote, Session};
use actr_engine::sexpr::{parse_all, Sexp};
use actr_engine::trace::Detail;
use actr_engine::{run_model_steps, ChunkDesc, DeviceSetup, DeviceText, Step};

mod common;
use common::{actr_dir, crate_dir, first_diff};

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
    /// Set when a step uses an op the scripted engine does not support.
    steps_unsupported: Option<String>,
    scope: String,
    error: Option<String>,
}

fn specs_dir() -> PathBuf {
    crate_dir().join("fixtures").join("regression").join("specs")
}

fn refs_dir() -> PathBuf {
    actr_dir().join("test-models")
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

/// Parse the spec for one case by name (`specs/<name>.spec`). Cases are
/// enumerated statically by the generated list, never scanned from the dir.
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

/// True when the spec's files define more than one *distinct* model (a
/// multi-model trace has an extra model-name column the single-model
/// formatter lacks). Same-name `define-model` redefinitions are one model:
/// the last definition wins (`run_model_steps`), as in ACT-R.
fn is_multi_model(spec: &Spec) -> bool {
    let mut names = BTreeSet::new();
    for m in &spec.models {
        let rel = m.replace(';', std::path::MAIN_SEPARATOR_STR);
        let path = actr_dir().join(rel);
        if !actr_engine::file_defines_model(&path) {
            continue;
        }
        if let Ok(src) = fs::read_to_string(&path) {
            if let Ok(forms) = parse_all(&src) {
                for f in &forms {
                    if let Sexp::List(v) = f {
                        match v.first().and_then(|s| s.as_sym()) {
                            // `(clear-all)` drops every model, so a later
                            // `define-model` starts a fresh (single) set —
                            // `paired-learning` clears the `paired` model
                            // loaded by its task file.
                            Some("CLEAR-ALL") | Some("CLEAR-ALL-FCT") => names.clear(),
                            Some("DEFINE-MODEL") | Some("DEFINE-MODEL-FCT") => {
                                if let Some(n) = v.get(1).and_then(as_str) {
                                    names.insert(n.to_uppercase());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    names.len() > 1
}

/// Why a spec is not runnable in the current engine (None = runnable).
fn ignore_reason(spec: &Spec) -> Option<String> {
    if let Some(e) = &spec.error {
        return Some(format!(
            "extraction error: {}",
            e.lines().next().unwrap_or("")
        ));
    }
    match spec.scope.as_str() {
        "PURE" | "MULTI" => {}
        "DEVICE" => return Some("device/test scaffolding not in spec yet".into()),
        "COMMAND" => return Some("command-layer output in ref".into()),
        "DRIVER" => return Some("unsupported run-it driver op (pdisable/...)".into()),
        "OTHER" => return Some("no run recorded".into()),
        other => return Some(format!("scope {other}")),
    }
    if spec.models.is_empty() {
        return Some("no model files".into());
    }
    if is_multi_model(spec) {
        return Some("multi-model trace (model column)".into());
    }
    if let Some(u) = &spec.steps_unsupported {
        return Some(format!("unsupported step: {u}"));
    }
    if !spec.steps.iter().any(|s| {
        matches!(
            s,
            Step::Run { .. } | Step::RunFullTime { .. } | Step::ZzDriver | Step::AlDriver
        )
    }) {
        return Some("no run step".into());
    }
    None
}

/// Per-spec device sidecar `<name>.device.sexp` in the specs dir, mirroring
/// the task file's post-reset `add-text-to-exp-window` / `clear-exp-window`
/// calls (same form as `tests/extra_traces.rs`): `((ADD-TEXT ms "text" x y) ...)`,
/// plus optional `(CLEAR ms)`, `(END ms)`, `(REWARD ms v)`, `(SIMILARITY a b v)`.
fn load_spec_device(name: &str) -> Result<DeviceSetup, String> {
    let path = specs_dir().join(format!("{name}.device.sexp"));
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
            "MODIFY-LINE" => device.line_modifies.push(actr_engine::engine::DeviceLineModify {
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
            "REMOVE-LINE" => device.line_removes.push(actr_engine::engine::DeviceLineRemove {
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
            "PRINT-VISICON" | "PRINT-VISICON-LATE" => {
                device.prints.push(actr_engine::engine::DevicePrint {
                    at_ms: ms()?,
                    late: op == "PRINT-VISICON-LATE",
                    run,
                    seq,
                })
            }
            // `(INSTALL)`: the task called `install-device`. Not a timed op;
            // it only records that a device exists (see
            // `DeviceSetup::installed`).
            "INSTALL" => device.installed = true,
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
    let seed = spec.run_seeds.first().copied().ok_or("no run seed")?;
    let device = load_spec_device(&spec.name)?;
    // A spec with a recorded command step is driven through the RPC command
    // layer (`Session::evaluate`), so the spec exercises the same dispatch a
    // Python client hits; everything else keeps the engine-level driver.
    //
    // A `:STARTING-PARAMETERS` step also needs the session: its
    // `:testing-function` drives the reset hook, which lives there (`fan-no-pm`
    // and `fan` rely on `trace-fan-run` turning `:act` on for every run).
    let needs_session = spec.steps.iter().any(|s| match s {
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
fn run_spec_session(
    root: &Path,
    spec: &Spec,
    device: &DeviceSetup,
) -> Result<String, String> {
    let seed = spec.run_seeds.first().copied().ok_or("no run seed")?;
    let mut session = Session::new(root.to_path_buf(), seed);
    let mut remote = NoRemote;
    for m in &spec.models {
        session
            .evaluate("load-act-r-model", None, &[Json::Str(m.clone())], &mut remote)
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
            let eng = session.engine_mut().ok_or_else(no_model)?;
            eng.set_module_width(*width);
            eng.set_trace(*trace);
            eng.run_secs(*secs).map_err(|e| e.to_string())?;
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
        Step::Seed(s) => session.engine_mut().ok_or_else(no_model)?.set_seed(*s),
        Step::ConsumeRng(n) => session.engine_mut().ok_or_else(no_model)?.consume_rng(*n),
        Step::Sdm(spec) => {
            session.engine_mut().ok_or_else(no_model)?.print_sdm(spec);
        }
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
        Step::PDisable(names) => session.engine_mut().ok_or_else(no_model)?.pdisable(names),
        Step::PEnable(names) => session.engine_mut().ok_or_else(no_model)?.penable(names),
        Step::BufferChunk(buffers) => {
            session
                .engine_mut()
                .ok_or_else(no_model)?
                .print_buffer_chunk(buffers);
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
            session.engine_mut().ok_or_else(no_model)?.add_visicon(slots);
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
        Step::ModifyVisicon { at_ms, name, x, y } => session
            .engine_mut()
            .ok_or_else(no_model)?
            .schedule_modify_visicon(*at_ms, name, *x, *y),
    }
    Ok(())
}

// ---------------------------------------------------------------- harness

/// Emit a `#[test]` per case. The list is static (generated by
/// `tools/gen_test_cases.py`), so no runtime directory discovery happens.
macro_rules! spec_case {
    ($ident:ident, $name:literal) => {
        #[test]
        fn $ident() {
            if let Err(e) = run_case($name) {
                panic!("{e}");
            }
        }
    };
}

/// An out-of-scope case, `#[ignore]`d with the reason it cannot run yet.
/// `--ignored` runs it anyway, to see whether it has become runnable.
macro_rules! spec_case_ignored {
    ($ident:ident, $name:literal, $reason:literal) => {
        #[test]
        #[ignore = $reason]
        fn $ident() {
            if let Err(e) = run_case($name) {
                panic!("{e}");
            }
        }
    };
}

include!("cases/test_models_cases.rs");

/// Run one ACT-R regression case and compare with its upstream `.ref` under
/// ACT-R's own comparator. `Err` carries the first divergence.
fn run_case(name: &str) -> Result<(), String> {
    let root = actr_dir();
    if !root.is_dir() {
        return Err(format!(
            "ACT-R checkout not found at {} (set ACTR_DIR)",
            root.display()
        ));
    }
    let spec = read_named_spec(name)?;
    if let Some(reason) = ignore_reason(&spec) {
        return Err(format!("out of scope: {reason}"));
    }
    let ref_path = refs_dir().join(&spec.ref_name);
    let expected = normalize(
        &fs::read_to_string(&ref_path).map_err(|e| format!("{}: {e}", ref_path.display()))?,
    );
    let actual = run_spec(&root, &spec)?;
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

/// Dev aid: dump the formatted trace of one spec (no `.ref` comparison).
/// `cargo test --test test_models dump_spec -- --ignored --nocapture`
#[test]
#[ignore]
fn dump_spec() {
    let name = std::env::var("DUMP_SPEC").unwrap_or_else(|_| "zz-extended-motor-actions".into());
    let root = actr_dir();
    let spec = read_named_spec(&name).unwrap();
    let actual = run_spec(&root, &spec).unwrap();
    for (i, l) in actual.iter().enumerate() {
        println!("{:>4}: {}", i + 1, l);
    }
}

/// `sexp_to_json` for a recorded command's arguments, but keeping Lisp strings
/// distinct from symbol names (`json_to_sexp` cannot tell them apart, and the
/// generic command path uppercases symbols).
fn cmd_arg_json(s: &Sexp) -> Json {
    match s {
        Sexp::Str(x) => Json::Obj(vec![("str".to_string(), Json::Str(x.clone()))]),
        other => sexp_to_json(other),
    }
}

//! ACT-R `test-models` corpus: Rust engine vs the checked-in `.ref` traces.
//!
//! Phase 1 (`golden/regen-specs.sh`) runs the Lisp stubs under
//! instrumentation and writes `golden/test-models/specs/*.spec` describing
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

use actr_engine::sexpr::{parse_all, Sexp};
use actr_engine::trace::Detail;
use actr_engine::{run_model_steps, ChunkDesc, DeviceSetup, DeviceText, Step};

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

fn crate_dir() -> PathBuf {    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn actr_dir() -> PathBuf {
    std::env::var("ACTR_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| crate_dir().join("..").join("actr7.x"))
}

fn specs_dir() -> PathBuf {
    crate_dir().join("golden").join("test-models").join("specs")
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

    let name = plist_get(items, ":NAME").and_then(as_str).ok_or("no :NAME")?;
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
    let (steps, steps_unsupported) =
        parse_steps(&plist_get(items, ":STEPS").map(list_or_empty).unwrap_or_default())?;
    let scope = plist_get(items, ":SCOPE")
        .and_then(as_str)
        .map(|s| s.trim_start_matches(':').to_string())
        .ok_or("no :SCOPE")?;
    let error = plist_get(items, ":ERROR").and_then(as_str).filter(|s| s != "NIL");

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
                steps.push(Step::Run {
                    secs,
                    trace,
                    width,
                });
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

fn discover_specs() -> Result<Vec<Spec>, String> {
    let dir = specs_dir();
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|e| format!("cannot list {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "spec").unwrap_or(false))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for p in paths {
        let src = fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        out.push(parse_spec(&src).map_err(|e| format!("{}: {e}", p.display()))?);
    }
    Ok(out)
}

// ---------------------------------------------------------------- compare

/// ACT-R `compare-test-model-files`: right-trim, case-insensitive, line by line.
fn normalize(s: &str) -> Vec<String> {
    s.lines()
        .map(|l| l.trim_end_matches([' ', '\r']).to_lowercase())
        .collect()
}

fn first_diff(expected: &[String], actual: &[String]) -> String {
    let n = expected.len().min(actual.len());
    for i in 0..n {
        if expected[i] != actual[i] {
            let lo = i.saturating_sub(3);
            let mut out = format!(
                "first diff at line {} ({} expected vs {} actual lines):\n",
                i + 1,
                expected.len(),
                actual.len()
            );
            for j in lo..(i + 4).min(n) {
                let mark = if j == i { ">>" } else { "  " };
                out.push_str(&format!(
                    "{mark} L{:>4} exp={:?} got={:?}\n",
                    j + 1,
                    expected.get(j),
                    actual.get(j)
                ));
            }
            return out;
        }
    }
    format!(
        "common prefix identical; length differs: {} expected vs {} actual lines",
        expected.len(),
        actual.len()
    )
}

/// True when the spec's files define more than one model (a multi-model
/// trace has an extra model-name column the single-model formatter lacks).
fn is_multi_model(spec: &Spec) -> bool {
    let mut count = 0usize;
    for m in &spec.models {
        let rel = m.replace(';', std::path::MAIN_SEPARATOR_STR);
        let path = actr_dir().join(rel);
        if !actr_engine::file_defines_model(&path) {
            continue;
        }
        if let Ok(src) = fs::read_to_string(&path) {
            if let Ok(forms) = parse_all(&src) {
                count += forms
                    .iter()
                    .filter(|f| match f {
                        Sexp::List(v) => matches!(
                            v.first().and_then(|s| s.as_sym()),
                            Some("DEFINE-MODEL") | Some("DEFINE-MODEL-FCT")
                        ),
                        _ => false,
                    })
                    .count();
            }
        }
    }
    count > 1
}

/// Why a spec is not runnable in the current engine (None = runnable).
fn ignore_reason(spec: &Spec) -> Option<String> {
    if let Some(e) = &spec.error {
        return Some(format!("extraction error: {}", e.lines().next().unwrap_or("")));
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
    if !spec.steps.iter().any(|s| matches!(s, Step::Run { .. })) {
        return Some("no run step".into());
    }
    None
}

/// Per-spec device sidecar `<name>.device.sexp` in the specs dir, mirroring
/// the task file's post-reset `add-text-to-exp-window` / `clear-exp-window`
/// calls (same form as `tests/golden.rs`): `((ADD-TEXT ms "text" x y) ...)`,
/// plus optional `(CLEAR ms)`, `(END ms)`, `(REWARD ms v)`, `(SIMILARITY a b v)`.
fn load_spec_device(name: &str) -> Result<DeviceSetup, String> {
    let path = specs_dir().join(format!("{name}.device.sexp"));
    if !path.is_file() {
        return Ok(DeviceSetup::default());
    }
    let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let forms = parse_all(&src).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut device = DeviceSetup::default();
    for form in &forms {
        let v = list(form).ok_or_else(|| format!("bad device form in {}: {form:?}", path.display()))?;
        let op = v.first().and_then(|s| s.as_sym()).unwrap_or("");
        let ms = || as_i64(v.get(1).ok_or("missing ms")?).ok_or("bad ms".to_string());
        match op {
            "ADD-TEXT" => device.texts.push(DeviceText {
                at_ms: as_i64(v.get(1).ok_or("missing ms")?).ok_or("bad ms")?,
                text: as_str(v.get(2).ok_or("missing text")?).ok_or("bad text")?,
                x: as_i64(v.get(3).ok_or("missing x")?).ok_or("bad x")?,
                y: as_i64(v.get(4).ok_or("missing y")?).ok_or("bad y")?,
            }),
            "CLEAR" => device.clears.push(ms()?),
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
                as_str(v.get(1).ok_or("missing a")?).ok_or("bad a")?.to_uppercase(),
                as_str(v.get(2).ok_or("missing b")?).ok_or("bad b")?.to_uppercase(),
                as_i64(v.get(3).ok_or("missing v")?).ok_or("bad v")? as f32,
            )),
            other => return Err(format!("bad device op {other} in {}", path.display())),
        }
    }
    Ok(device)
}

fn run_spec(root: &Path, spec: &Spec) -> Result<Vec<String>, String> {
    let seed = spec
        .run_seeds
        .first()
        .copied()
        .ok_or("no run seed")?;
    let device = load_spec_device(&spec.name)?;
    let actual = run_model_steps(root, &spec.models, seed, &spec.steps, &device)
        .map_err(|e| format!("engine error: {e}"))?;
    Ok(normalize(&actual))
}

fn load_enabled() -> BTreeSet<String> {
    let path = crate_dir().join("golden").join("test-models").join("enabled.txt");
    fs::read_to_string(&path)
        .map(|s| {
            s.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------- tests

/// The checked-in allowlist must reproduce its refs byte-equivalently (under
/// the ACT-R comparator). `cargo test` fails if the engine regresses.
#[test]
fn enabled_test_models_match() {
    let root = actr_dir();
    assert!(
        root.is_dir(),
        "ACT-R checkout not found at {} (set ACTR_DIR)",
        root.display()
    );
    let specs = discover_specs().expect("spec discovery failed (run golden/regen-specs.sh)");
    let enabled = load_enabled();
    assert!(
        !enabled.is_empty(),
        "golden/test-models/enabled.txt is empty or missing"
    );

    let mut failures = Vec::new();
    let mut passed = 0;
    let mut total = 0;
    for spec in &specs {
        if !enabled.contains(&spec.name) {
            continue;
        }
        total += 1;
        if let Some(reason) = ignore_reason(spec) {
            failures.push(format!("{}: listed enabled but not runnable: {reason}", spec.name));
            continue;
        }
        let ref_path = refs_dir().join(&spec.ref_name);
        let expected = normalize(
            &fs::read_to_string(&ref_path)
                .unwrap_or_else(|e| panic!("{}: {e}", ref_path.display())),
        );
        match run_spec(&root, spec) {
            Ok(actual) if actual == expected => passed += 1,
            Ok(actual) => failures.push(format!(
                "{}: ref mismatch vs {}\n{}",
                spec.name,
                ref_path.display(),
                first_diff(&expected, &actual)
            )),
            Err(e) => failures.push(format!("{}: {e}", spec.name)),
        }
    }
    assert!(
        failures.is_empty(),
        "{passed}/{total} enabled test-models passed:\n{}",
        failures.join("\n")
    );
}

/// Report every runnable spec that currently passes, so `enabled.txt` can
/// grow. Run with:
/// `cargo test --test test_models probe_runnable -- --ignored --nocapture`
#[test]
#[ignore]
fn probe_runnable() {
    let root = actr_dir();
    let specs = discover_specs().expect("spec discovery failed");
    let mut pass = Vec::new();
    let mut fail = Vec::new();
    let mut ignored = 0;
    for spec in &specs {
        if ignore_reason(spec).is_some() {
            ignored += 1;
            continue;
        }
        let ref_path = refs_dir().join(&spec.ref_name);
        let expected = normalize(&fs::read_to_string(&ref_path).unwrap());
        match run_spec(&root, spec) {
            Ok(actual) if actual == expected => pass.push(format!("{} [{}]", spec.name, spec.scope)),
            Ok(actual) => fail.push(format!(
                "{} [{}] (mismatch)\n{}",
                spec.name,
                spec.scope,
                first_diff(&expected, &actual)
            )),
            Err(e) => fail.push(format!("{} [{}] ({e})", spec.name, spec.scope)),
        }
    }
    println!(
        "\n=== pass ({}) ({} ignored as out of scope) ===\n{}",
        pass.len(),
        ignored,
        pass.join("\n")
    );
    println!("\n=== fail ({}) ===\n{}", fail.len(), fail.join("\n"));
}

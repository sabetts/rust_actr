//! ACT-R `test-models` corpus: Rust engine vs the checked-in `.ref` traces.
//!
//! Phase 1 (`golden/regen-specs.sh`) runs the Lisp stubs under
//! instrumentation and writes `golden/test-models/specs/*.spec` describing
//! each test's model chain, resolved seed, and run steps. This harness
//! consumes those specs plus `actr7.x/test-models/*.ref`, so `cargo test`
//! never needs SBCL.
//!
//! Comparison mirrors ACT-R's own `compare-test-model-files`
//! (`actr7.x/test-models/model-tester.cl`): each line is right-trimmed and
//! compared case-insensitively (`string-equal`), line by line. That is what
//! the refs were produced/validated against.
//!
//! Only `scope = PURE` specs (one run, one model, no device/command output)
//! are runnable today; the rest are reported as ignored with a reason so the
//! coverage gap is visible and only shrinks. `enabled.txt` is the checked-in
//! allowlist that must stay green; `probe_all_pure` (ignored test) reports
//! which other pure specs already pass, to promote over time.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use actr_engine::sexpr::{parse_all, Sexp};
use actr_engine::DeviceSetup;

// ---------------------------------------------------------------- spec model

#[derive(Debug, Clone)]
struct Spec {
    name: String,
    ref_name: String,
    models: Vec<String>,
    runs: Vec<Run>,
    goal_focus: i64,
    resets: i64,
    sgp_nonempty: bool,
    scope: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct Run {
    secs: f64,
    seed: (u64, u64),
    v: bool,
    width: usize,
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
    let runs = plist_get(items, ":RUNS")
        .map(list_or_empty)
        .unwrap_or_default()
        .iter()
        .map(|r| {
            let rr = list(r).ok_or("run not a list")?;
            let secs = match &rr[0] {
                Sexp::Int(i) => *i as f64,
                Sexp::Float(x) => *x,
                _ => return Err("run secs not numeric".to_string()),
            };
            let seed = parse_seed(&rr[1]).ok_or("bad run seed")?;
            let v = matches!(&rr[2], Sexp::Sym(s) if s != "NIL");
            let width = rr.get(3).and_then(as_i64).unwrap_or(22).max(1) as usize;
            Ok(Run {
                secs,
                seed,
                v,
                width,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let goal_focus = plist_get(items, ":GOAL-FOCUS").and_then(as_i64).unwrap_or(0);
    let resets = plist_get(items, ":RESETS").and_then(as_i64).unwrap_or(0);
    let sgp_nonempty = matches!(plist_get(items, ":SGP"), Some(Sexp::List(v)) if !v.is_empty());
    let scope = plist_get(items, ":SCOPE")
        .and_then(as_str)
        .map(|s| s.trim_start_matches(':').to_string())
        .ok_or("no :SCOPE")?;
    let error = plist_get(items, ":ERROR").and_then(as_str).filter(|s| s != "NIL");

    Ok(Spec {
        name,
        ref_name,
        models,
        runs,
        goal_focus,
        resets,
        sgp_nonempty,
        scope,
        error,
    })
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

/// Why a spec is not runnable in the current engine (None = runnable).
fn ignore_reason(spec: &Spec) -> Option<String> {
    if let Some(e) = &spec.error {
        return Some(format!("extraction error: {}", e.lines().next().unwrap_or("")));
    }
    match spec.scope.as_str() {
        "PURE" => {}
        "MULTI" => return Some("multi-run/step driver (needs scripted engine)".into()),
        "DEVICE" => return Some("device/test scaffolding not in spec yet".into()),
        "COMMAND" => return Some("command-layer output in ref".into()),
        "DRIVER" => return Some("unsupported run-it driver op (pdisable/...)".into()),
        "OTHER" => return Some("no single run recorded".into()),
        other => return Some(format!("scope {other}")),
    }
    if spec.models.len() != 1 {
        return Some(format!("{} model files", spec.models.len()));
    }
    if spec.runs.len() != 1 {
        return Some(format!("{} runs", spec.runs.len()));
    }
    if spec.goal_focus != 0 || spec.resets != 0 {
        return Some("goal-focus/reset steps".into());
    }
    if spec.sgp_nonempty {
        return Some("mid-driver sgp setters".into());
    }
    if !spec.runs[0].v {
        return Some("run with trace disabled".into());
    }
    None
}

fn run_spec(root: &Path, spec: &Spec) -> Result<Vec<String>, String> {
    let run = &spec.runs[0];
    let actual = actr_engine::run_model_with(
        root,
        &spec.models[0],
        run.seed,
        run.secs,
        &DeviceSetup::default(),
        run.width,
    )
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

/// Report every pure spec that currently passes, so `enabled.txt` can grow.
/// Run with: `cargo test --test test_models probe_all_pure -- --ignored --nocapture`
#[test]
#[ignore]
fn probe_all_pure() {
    let root = actr_dir();
    let specs = discover_specs().expect("spec discovery failed");
    let mut pass = Vec::new();
    let mut fail = Vec::new();
    for spec in &specs {
        if spec.scope != "PURE" || ignore_reason(spec).is_some() {
            continue;
        }
        let ref_path = refs_dir().join(&spec.ref_name);
        let expected = normalize(&fs::read_to_string(&ref_path).unwrap());
        match run_spec(&root, spec) {
            Ok(actual) if actual == expected => pass.push(spec.name.clone()),
            Ok(actual) => fail.push(format!(
                "{} (mismatch)\n{}",
                spec.name,
                first_diff(&expected, &actual)
            )),
            Err(e) => fail.push(format!("{} ({e})", spec.name)),
        }
    }
    println!("\n=== pass ({}) ===\n{}", pass.len(), pass.join("\n"));
    println!("\n=== fail ({}) ===\n{}", fail.len(), fail.join("\n"));
}

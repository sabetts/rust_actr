//! Project-authored trace tests — **not** from ACT-R's regression suite.
//!
//! ACT-R ships no tests for these scenarios, so they are our own ground
//! truth: `fixtures/extra-traces/<name>.trace`, captured from the Lisp
//! system (`fixtures/regen.sh`) and compared **byte-exact** (CRLF -> LF
//! only), unlike the trim/case-insensitive comparator used for the ACT-R
//! refs in `test_models.rs`.
//!
//! The sibling `<name>.meta` supplies the model location, seed and run time;
//! an optional `<name>.device.sexp` scripts window input. These cases exist
//! because they exercise behavior the upstream corpus does not (activation
//! noise, base-level learning, device/window scaffolding).
//!
//! The cases are a static list of `#[test]`s, one per scenario.

use std::fs;
use std::path::{Path, PathBuf};

use actr_engine::{run_model, DeviceSetup, DeviceText};

mod common;
use common::{actr_dir, crate_dir, first_diff, meta_value, normalize_eol, parse_seed_str};

macro_rules! trace_case {
    ($ident:ident, $name:literal) => {
        #[test]
        fn $ident() {
            if let Err(e) = run_case($name) {
                panic!("{e}");
            }
        }
    };
}

trace_case!(noisy_count, "noisy-count");
trace_case!(noisy_count_7, "noisy-count-7");
trace_case!(bll_count, "bll-count");
trace_case!(attend, "attend");
trace_case!(mp_test, "mp-test");
trace_case!(ul_test, "ul-test");

fn traces_dir() -> PathBuf {
    crate_dir().join("fixtures").join("extra-traces")
}

fn run_case(name: &str) -> Result<(), String> {
    let root = actr_dir();
    if !root.is_dir() {
        return Err(format!(
            "ACT-R checkout not found at {} (set ACTR_DIR)",
            root.display()
        ));
    }
    let trace_path = traces_dir().join(format!("{name}.trace"));
    let expected =
        fs::read_to_string(&trace_path).map_err(|e| format!("{}: {e}", trace_path.display()))?;
    let meta_path = trace_path.with_extension("meta");
    let meta =
        fs::read_to_string(&meta_path).map_err(|e| format!("{}: {e}", meta_path.display()))?;
    let model = meta_value(&meta, "model")?;
    let seed = parse_seed_str(&meta_value(&meta, "seed")?)?;
    let run_time: f64 = meta_value(&meta, "run")?
        .parse()
        .map_err(|_| format!("{name}: bad run time"))?;

    let device = load_device(&trace_path)?;
    let actual = run_model(&root, &model, seed, run_time, &device)
        .map_err(|e| format!("{name}: engine error: {e}"))?;

    if normalize_eol(&actual) == normalize_eol(&expected) {
        Ok(())
    } else {
        let e: Vec<&str> = expected.lines().collect();
        let a: Vec<&str> = actual.lines().collect();
        Err(format!(
            "{name}: trace mismatch vs {}\n{}",
            trace_path.display(),
            first_diff(&e, &a)
        ))
    }
}

/// Load `<name>.device.sexp` if present: `((ADD-TEXT ms "text" x y) ...)`.
fn load_device(trace_path: &Path) -> Result<DeviceSetup, String> {
    let sidecar = trace_path.with_extension("device.sexp");
    if !sidecar.is_file() {
        return Ok(DeviceSetup::default());
    }
    let src = fs::read_to_string(&sidecar).map_err(|e| format!("{}: {e}", sidecar.display()))?;
    // Minimal reader (the crate's sexpr parser lives behind the lib boundary).
    let mut device = DeviceSetup::default();
    for form in split_forms(&src) {
        let parts = split_atoms(&form)
            .ok_or_else(|| format!("bad device form in {}: {form}", sidecar.display()))?;
        match parts.as_slice() {
            [op, ms, text, x, y] if op == "ADD-TEXT" => {
                device.texts.push(DeviceText {
                    at_ms: ms.parse().map_err(|_| format!("bad ms in {form}"))?,
                    text: unquote(text).ok_or_else(|| format!("bad text in {form}"))?,
                    x: x.parse().map_err(|_| format!("bad x in {form}"))?,
                    y: y.parse().map_err(|_| format!("bad y in {form}"))?,
                    run: None,
                    seq: 0,
                });
            }
            [op, ms] if op == "CLEAR" => {
                device
                    .clears
                    .push((ms.parse().map_err(|_| format!("bad ms in {form}"))?, None));
            }
            [op, ms] if op == "END" => {
                device.end = Some(ms.parse().map_err(|_| format!("bad ms in {form}"))?);
            }
            [op, ms, v] if op == "REWARD" => {
                device.rewards.push((
                    ms.parse().map_err(|_| format!("bad ms in {form}"))?,
                    v.parse().map_err(|_| format!("bad value in {form}"))?,
                ));
            }
            [op, a, b, v] if op == "SIMILARITY" => {
                // Lisp symbols fold to uppercase (as do model files).
                device.sims.push((
                    a.to_uppercase(),
                    b.to_uppercase(),
                    v.parse().map_err(|_| format!("bad value in {form}"))?,
                ));
            }
            _ => return Err(format!("bad device form in {}: {form}", sidecar.display())),
        }
    }
    Ok(device)
}

/// Split top-level `(...)` forms (no nesting needed here).
fn split_forms(src: &str) -> Vec<String> {
    let mut forms = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    let mut instr = false;
    let mut prev_bs = false;
    let mut incomment = false;
    for c in src.chars() {
        if incomment {
            if c == '\n' {
                incomment = false;
            }
            continue;
        }
        if instr {
            cur.push(c);
            if c == '"' && !prev_bs {
                instr = false;
            }
            prev_bs = c == '\\' && !prev_bs;
            continue;
        }
        match c {
            '"' => {
                instr = true;
                cur.push(c);
            }
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
                if depth == 0 && !cur.trim().is_empty() {
                    forms.push(std::mem::take(&mut cur));
                }
            }
            ';' if depth == 0 => {
                incomment = true;
            }
            _ => {
                if depth > 0 {
                    cur.push(c);
                }
            }
        }
    }
    forms
}

/// Split one `(...)` form into atoms, honoring quotes. Returns None on
/// unbalanced input.
fn split_atoms(form: &str) -> Option<Vec<String>> {
    let inner = form.trim().strip_prefix('(')?.strip_suffix(')')?;
    let mut atoms = Vec::new();
    let mut cur = String::new();
    let mut instr = false;
    let mut prev_bs = false;
    let mut had = false;
    for c in inner.chars() {
        if instr {
            cur.push(c);
            if c == '"' && !prev_bs {
                instr = false;
            }
            prev_bs = c == '\\' && !prev_bs;
            had = true;
            continue;
        }
        match c {
            '"' => {
                instr = true;
                cur.push(c);
                had = true;
            }
            c if c.is_whitespace() => {
                if had {
                    atoms.push(std::mem::take(&mut cur));
                    had = false;
                }
            }
            _ => {
                cur.push(c);
                had = true;
            }
        }
    }
    if instr {
        return None;
    }
    if had {
        atoms.push(cur);
    }
    Some(atoms)
}

fn unquote(s: &str) -> Option<String> {
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.replace("\\\"", "\"").replace("\\\\", "\\"))
}

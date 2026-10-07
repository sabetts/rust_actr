//! M1 golden test: the Rust engine must reproduce the M0 Lisp traces byte-exact.
//!
//! For every `golden/traces/*.trace`, the sibling `.meta` supplies the
//! parameters (model location, seed, run time — same values as
//! `golden/manifest.lisp`). The test runs `actr_engine::run_model` and
//! diffs stdout-free: `output == trace bytes`, exactly.
//!
//! Adding a golden = adding a `manifest.lisp` entry + `./regen.sh`.
//! No Rust edits needed: cases are auto-discovered from `traces/`.
//!
//! `ACTR_DIR` env overrides the `actr7.x` checkout location
//! (default: `<crate>/../actr7.x`).

use std::fs;
use std::path::{Path, PathBuf};

use actr_engine::{run_model, DeviceSetup, DeviceText};

// Device sidecars: `<name>.device.sexp` next to the trace holds the
// scripted window input, e.g. `((ADD-TEXT 0 "Q" 100 100))`, mirroring
// the task file's post-reset `add-text-to-exp-window` calls. Absent
// file means no device.

struct Case {
    name: String,
    trace_path: PathBuf,
    expected: String,
    model: String,
    seed: (u64, u64),
    run_time: f64,
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn actr_dir() -> PathBuf {
    std::env::var("ACTR_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| crate_dir().join("..").join("actr7.x"))
}

/// Parse one `key: value` line out of a `.meta` file.
fn meta_value(meta: &str, key: &str) -> Result<String, String> {
    for line in meta.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim() == key {
                return Ok(v.trim().to_string());
            }
        }
    }
    Err(format!("meta missing key `{key}`"))
}

/// Parse a seed of the form `(100 0)` (see GOTCHAS.md #5).
fn parse_seed(s: &str) -> Result<(u64, u64), String> {
    let inner = s
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| format!("bad seed `{s}`: want `(S C)`"))?;
    let mut it = inner.split_whitespace();
    let a = it.next().ok_or_else(|| format!("bad seed `{s}`"))?;
    let b = it.next().ok_or_else(|| format!("bad seed `{s}`"))?;
    if it.next().is_some() {
        return Err(format!("bad seed `{s}`"));
    }
    Ok((
        a.parse::<u64>().map_err(|_| format!("bad seed `{s}`"))?,
        b.parse::<u64>().map_err(|_| format!("bad seed `{s}`"))?,
    ))
}

fn discover() -> Result<Vec<Case>, String> {
    let dir = crate_dir().join("golden").join("traces");
    let mut entries: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|e| format!("cannot list {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "trace").unwrap_or(false))
        .collect();
    entries.sort();
    let mut cases = Vec::new();
    for trace_path in entries {
        let name = trace_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("bad trace name: {}", trace_path.display()))?
            .to_string();
        let expected = fs::read_to_string(&trace_path)
            .map_err(|e| format!("{}: {e}", trace_path.display()))?;
        let meta_path = trace_path.with_extension("meta");
        let meta =
            fs::read_to_string(&meta_path).map_err(|e| format!("{}: {e}", meta_path.display()))?;
        let model = meta_value(&meta, "model")?;
        let seed = parse_seed(&meta_value(&meta, "seed")?);
        let seed = seed?;
        let run_time: f64 = meta_value(&meta, "run")?
            .parse()
            .map_err(|_| format!("{name}: bad run time"))?;
        cases.push(Case {
            name,
            trace_path,
            expected,
            model,
            seed,
            run_time,
        });
    }
    Ok(cases)
}

/// First divergence between expected and actual, with context lines.
fn first_diff(expected: &str, actual: &str) -> String {
    let e: Vec<&str> = expected.lines().collect();
    let a: Vec<&str> = actual.lines().collect();
    let n = e.len().min(a.len());
    for i in 0..n {
        if e[i] != a[i] {
            let lo = i.saturating_sub(3);
            let mut out = format!(
                "first diff at line {} ({} expected vs {} actual lines):\n",
                i + 1,
                e.len(),
                a.len()
            );
            for j in lo..(i + 4).min(n) {
                let mark = if j == i { ">>" } else { "  " };
                out.push_str(&format!(
                    "{mark} L{:>4} exp={:?} got={:?}\n",
                    j + 1,
                    e.get(j),
                    a.get(j)
                ));
            }
            return out;
        }
    }
    format!(
        "common prefix identical, length differs: {} expected vs {} actual lines",
        e.len(),
        a.len()
    )
}

fn run_case(root: &Path, case: &Case) -> Result<(), String> {
    let device = load_device(&case.trace_path)?;
    let actual = run_model(root, &case.model, case.seed, case.run_time, &device)
        .map_err(|e| format!("{}: engine error: {e}", case.name))?;
    if actual == case.expected {
        Ok(())
    } else {
        Err(format!(
            "{}: trace mismatch vs {}\n{}",
            case.name,
            case.trace_path.display(),
            first_diff(&case.expected, &actual)
        ))
    }
}

#[test]
fn golden_traces_match() {
    let cases = discover().expect("golden discovery failed (run golden/regen.sh)");
    assert!(
        !cases.is_empty(),
        "no golden traces in golden/traces (run golden/regen.sh)"
    );
    let root = actr_dir();
    assert!(
        root.is_dir(),
        "ACT-R checkout not found at {} (set ACTR_DIR)",
        root.display()
    );
    let mut failures = Vec::new();
    let mut passed = 0;
    for case in &cases {
        match run_case(&root, case) {
            Ok(()) => passed += 1,
            Err(e) => failures.push(e),
        }
    }
    assert!(
        failures.is_empty(),
        "{passed}/{} golden cases passed:\n{}",
        cases.len(),
        failures.join("\n")
    );
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

#[cfg(test)]
mod harness_tests {
    use super::*;

    #[test]
    fn seed_parses() {
        assert_eq!(parse_seed("(100 0)").unwrap(), (100, 0));
        assert!(parse_seed("100 0").is_err());
        assert!(parse_seed("(100)").is_err());
        assert!(parse_seed("(100 0 5)").is_err());
        assert!(parse_seed("(-1 0)").is_err());
    }

    #[test]
    fn diff_points_at_first_divergence() {
        let msg = first_diff("a\nb\nc\n", "a\nX\nc\n");
        assert!(msg.contains("line 2"), "unexpected:\n{msg}");
        assert!(msg.contains("exp=") && msg.contains("got="));
        let msg = first_diff("a\nb\n", "a\nb\nc\n");
        assert!(msg.contains("length differs"), "unexpected:\n{msg}");
    }

    #[test]
    fn meta_lookup() {
        let meta = "name: count\nseed: (100 0)\nrun: 10\n";
        assert_eq!(meta_value(meta, "seed").unwrap(), "(100 0)");
        assert!(meta_value(meta, "nope").is_err());
    }
}

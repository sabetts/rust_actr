//! Helpers shared by the integration-test suites.
//!
//! Two oracles live under `tests/`:
//!
//! * `test_models.rs` — ACT-R's regression corpus. Ground truth is the
//!   vendored `tests/cases/*.ref` (copied from `actr7.x/test-models`),
//!   compared the way ACT-R's `compare-test-model-files` does (right-trim,
//!   case-insensitive).
//! * `extra_traces.rs` — project-authored scenarios with no upstream `.ref`.
//!   Ground truth is `fixtures/extra-traces/*.trace`, compared byte-exact
//!   (CRLF normalized to LF only).
//!
//! Model/task sources are vendored under [`models_dir`], so the default
//! `cargo test` needs neither an ACT-R checkout nor SBCL.

use std::fs;
use std::path::{Path, PathBuf};

use actr_engine::{DeviceSetup, DeviceText};

/// The crate root (`rust_actr7/`).
pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The vendored model/task sources used by the default test run.
pub fn models_dir() -> PathBuf {
    crate_dir().join("fixtures").join("models")
}

/// Resolve a fixture manifest name (`tutorial;unit1;count.lisp`) to the
/// vendored filesystem path the runtime expects.
#[allow(dead_code)]
pub fn fixture_model(name: &str) -> String {
    models_dir()
        .join(name.replace(';', std::path::MAIN_SEPARATOR_STR))
        .to_string_lossy()
        .into_owned()
}

/// Normalize the newline convention only (CRLF -> LF). Content stays exact.
/// The engine emits LF, but a Windows checkout with `core.autocrlf=true`
/// gives the stored files CRLF, which would otherwise fail a byte comparison.
pub fn normalize_eol(s: &str) -> String {
    s.replace("\r\n", "\n")
}

/// First divergence between `expected` and `actual`, with context lines.
/// Both slices may be `&[&str]` or `&[String]`.
pub fn first_diff(expected: &[impl AsRef<str>], actual: &[impl AsRef<str>]) -> String {
    let n = expected.len().min(actual.len());
    for i in 0..n {
        if expected[i].as_ref() != actual[i].as_ref() {
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
                    expected[j].as_ref(),
                    actual[j].as_ref()
                ));
            }
            return out;
        }
    }
    format!(
        "common prefix identical, length differs: {} expected vs {} actual lines",
        expected.len(),
        actual.len()
    )
}

/// Parse one `key: value` line out of a `.meta` file.
pub fn meta_value(meta: &str, key: &str) -> Result<String, String> {
    for line in meta.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim() == key {
                return Ok(v.trim().to_string());
            }
        }
    }
    Err(format!("meta missing key `{key}`"))
}

/// Parse a seed of the form `(100 0)` (see PORTING-NOTES.md #5).
pub fn parse_seed_str(s: &str) -> Result<(u64, u64), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_points_at_first_divergence() {
        let msg = first_diff(&["a", "b", "c"], &["a", "X", "c"]);
        assert!(msg.contains("line 2"), "unexpected:\n{msg}");
        assert!(msg.contains("exp=") && msg.contains("got="));
        let msg = first_diff(&["a", "b"], &["a", "b", "c"]);
        assert!(msg.contains("length differs"), "unexpected:\n{msg}");
    }

    #[test]
    fn seed_and_meta_parse() {
        assert_eq!(parse_seed_str("(100 0)").unwrap(), (100, 0));
        assert!(parse_seed_str("100 0").is_err());
        assert!(parse_seed_str("(100)").is_err());
        assert!(parse_seed_str("(100 0 5)").is_err());
        assert!(parse_seed_str("(-1 0)").is_err());
        let meta = "name: count\nseed: (100 0)\nrun: 10\n";
        assert_eq!(meta_value(meta, "seed").unwrap(), "(100 0)");
        assert!(meta_value(meta, "nope").is_err());
    }

    #[test]
    fn eol_normalized() {
        assert_eq!(normalize_eol("a\r\nb\r\n"), "a\nb\n");
    }
}

/// Load `<name>.device.sexp` if present: `((ADD-TEXT ms "text" x y) ...)`.
#[allow(dead_code)]
pub fn load_device(trace_path: &Path) -> Result<DeviceSetup, String> {
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

#[allow(dead_code)]
fn unquote(s: &str) -> Option<String> {
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.replace("\\\"", "\"").replace("\\\\", "\\"))
}

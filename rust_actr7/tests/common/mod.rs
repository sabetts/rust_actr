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

use std::path::PathBuf;

/// The crate root (`rust_actr7/`).
pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The vendored model/task sources used by the default test run.
pub fn models_dir() -> PathBuf {
    crate_dir().join("fixtures").join("models")
}

/// The `actr7.x` checkout, used only by regeneration/dev tools. `ACTR_DIR`
/// overrides the default (`<crate>/../actr7.x`); the default test run uses
/// [`models_dir`] instead.
#[allow(dead_code)]
pub fn actr_dir() -> PathBuf {
    std::env::var("ACTR_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| crate_dir().join("..").join("actr7.x"))
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

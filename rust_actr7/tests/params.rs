//! Every `sgp` key the vendored corpus or a driver step uses must be
//! classified by `sgp_support`. Unknown keys now fail at run time; this test
//! catches them before a case runs and documents the accepted no-ops.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use actr_engine::{sgp_support, SgpSupport};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Strip Lisp `;` comments, `#| |#` blocks, and string literals so parameter
/// keywords inside them are not mistaken for real settings.
fn strip_lisp(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ';' => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '"' => {
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        chars.next();
                    } else if c == '"' {
                        break;
                    }
                }
                out.push(' ');
            }
            '#' if chars.peek() == Some(&'|') => {
                chars.next();
                let mut prev = '\0';
                for c in chars.by_ref() {
                    if prev == '|' && c == '#' {
                        break;
                    }
                    prev = c;
                }
                out.push(' ');
            }
            _ => out.push(c),
        }
    }
    out
}

/// Balanced `(sgp …)` / `(ssp …)` / `(sgp-fct …)` / `(ssp-fct …)` forms.
fn parameter_forms(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != '(' {
            i += 1;
            continue;
        }
        let rest: String = bytes[i + 1..].iter().take(6).collect::<String>();
        let head = rest.trim_start().to_ascii_lowercase();
        if !(head.starts_with("sgp") || head.starts_with("ssp")) {
            i += 1;
            continue;
        }
        let mut depth = 0usize;
        let mut j = i;
        while j < bytes.len() {
            match bytes[j] {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        if j < bytes.len() {
            out.push(bytes[i..=j].iter().collect());
            i = j + 1;
        } else {
            break;
        }
    }
    out
}

fn keys_in(form: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let bytes: Vec<char> = form.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != ':' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        if i >= bytes.len() || !bytes[i].is_ascii_alphabetic() {
            continue;
        }
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == '-' || bytes[i] == '_')
        {
            i += 1;
        }
        keys.push(
            bytes[start..i]
                .iter()
                .collect::<String>()
                .to_ascii_uppercase(),
        );
    }
    keys
}

fn walk_lisp(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_lisp(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("lisp") {
            out.push(path);
        }
    }
}

/// Keys from `Step::Sgp(":KEY", …)` and direct `apply_sgp(":KEY", …)` calls
/// in the case drivers.
fn driver_keys(src: &str) -> Vec<String> {
    let bytes: Vec<char> = src.chars().collect();
    let mut keys = Vec::new();
    for name in ["Sgp(", "apply_sgp("] {
        let pat: Vec<char> = name.chars().collect();
        let mut i = 0;
        while i + pat.len() <= bytes.len() {
            if bytes[i..i + pat.len()] != pat[..] {
                i += 1;
                continue;
            }
            let mut j = i + pat.len();
            while j < bytes.len() && bytes[j].is_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == '"' {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == ':' {
                let start = j;
                j += 1;
                while j < bytes.len()
                    && (bytes[j].is_ascii_alphanumeric() || bytes[j] == '-' || bytes[j] == '_')
                {
                    j += 1;
                }
                keys.push(
                    bytes[start..j]
                        .iter()
                        .collect::<String>()
                        .to_ascii_uppercase(),
                );
            }
            i = j.max(i + 1);
        }
    }
    keys
}

#[test]
fn corpus_sgp_keys_are_classified() {
    let crate_dir = crate_dir();
    let mut seen: BTreeMap<String, (String, &'static str)> = BTreeMap::new();

    let mut files = Vec::new();
    walk_lisp(&crate_dir.join("fixtures").join("models"), &mut files);
    for path in files {
        let Ok(src) = fs::read_to_string(&path) else {
            continue;
        };
        let stripped = strip_lisp(&src);
        for form in parameter_forms(&stripped) {
            for key in keys_in(&form) {
                seen.entry(key)
                    .or_insert_with(|| (path.display().to_string(), "fixture"));
            }
        }
    }

    let cases = crate_dir.join("tests").join("cases");
    if let Ok(entries) = fs::read_dir(&cases) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let Ok(src) = fs::read_to_string(&path) else {
                continue;
            };
            for key in driver_keys(&src) {
                seen.entry(key)
                    .or_insert_with(|| (path.display().to_string(), "driver"));
            }
        }
    }

    let mut unknown = Vec::new();
    let mut driver_model_only = Vec::new();
    for (key, (path, origin)) in &seen {
        match sgp_support(key) {
            None => unknown.push(format!("{key} ({origin}: {path})")),
            Some(SgpSupport::ModelOnly) if *origin == "driver" => {
                driver_model_only.push(format!("{key} ({path})"));
            }
            _ => {}
        }
    }

    assert!(
        unknown.is_empty(),
        "unclassified sgp parameters:\n  {}",
        unknown.join("\n  ")
    );
    assert!(
        driver_model_only.is_empty(),
        "driver steps set model-load-only parameters (add a runtime setter):\n  {}",
        driver_model_only.join("\n  ")
    );
}

#[test]
fn support_categories_are_well_formed() {
    assert_eq!(sgp_support(":act"), Some(SgpSupport::Both));
    assert_eq!(sgp_support(":Act"), Some(SgpSupport::Both));
    assert_eq!(sgp_support(":UAN"), Some(SgpSupport::Both));
    assert!(matches!(
        sgp_support(":BL-HOOK"),
        Some(SgpSupport::Ignored(_))
    ));
    assert!(matches!(
        sgp_support(":SHOW-FOCUS"),
        Some(SgpSupport::Ignored(_))
    ));
    assert_eq!(sgp_support(":NOT-A-REAL-PARAM"), None);
}

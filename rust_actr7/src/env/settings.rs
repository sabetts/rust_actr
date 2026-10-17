//! Persistent Environment settings (window size, panel layout, options),
//! stored as JSON in `~/.actr-env.json` — the Rust counterpart of the Tcl
//! environment's `init/10-userguisettings.tcl`.

use std::path::PathBuf;

use crate::json::{self, Json};

#[derive(Debug, Clone)]
pub struct Settings {
    /// Last main-window inner size `(w, h)`.
    pub window: Option<(f32, f32)>,
    /// Width of the controls panel.
    pub controls_width: f32,
    /// Alphabetically sort the viewer lists.
    pub sort_lists: bool,
    /// Pace runs to wall-clock time by default.
    pub real_time: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            window: None,
            controls_width: 200.0,
            sort_lists: false,
            real_time: true,
        }
    }
}

/// `~/.actr-env.json` (`$HOME`, falling back to `%USERPROFILE%`).
pub fn settings_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".actr-env.json")
}

pub fn load() -> Settings {
    load_from(&settings_path())
}

pub fn save(s: &Settings) {
    save_to(&settings_path(), s);
}

fn load_from(path: &std::path::Path) -> Settings {
    let mut s = Settings::default();
    let Ok(text) = std::fs::read_to_string(path) else {
        return s;
    };
    let Ok(v) = json::parse(&text) else {
        return s;
    };
    let num = |k: &str| v.get(k).and_then(|x| x.as_f64());
    if let (Some(w), Some(h)) = (num("window_w"), num("window_h")) {
        s.window = Some((w as f32, h as f32));
    }
    if let Some(w) = num("controls_width") {
        s.controls_width = w as f32;
    }
    if let Some(Json::Bool(b)) = v.get("sort_lists") {
        s.sort_lists = *b;
    }
    if let Some(Json::Bool(b)) = v.get("real_time") {
        s.real_time = *b;
    }
    s
}

fn save_to(path: &std::path::Path, s: &Settings) {
    let mut obj = vec![
        (
            "controls_width".to_string(),
            Json::Float(s.controls_width as f64),
        ),
        ("sort_lists".to_string(), Json::Bool(s.sort_lists)),
        ("real_time".to_string(), Json::Bool(s.real_time)),
    ];
    if let Some((w, h)) = s.window {
        obj.push(("window_w".to_string(), Json::Float(w as f64)));
        obj.push(("window_h".to_string(), Json::Float(h as f64)));
    }
    let _ = std::fs::write(path, Json::Obj(obj).write());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("actr-env-test-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let s = Settings {
            window: Some((1234.0, 678.0)),
            controls_width: 321.0,
            sort_lists: true,
            real_time: false,
        };
        save_to(&path, &s);
        let back = load_from(&path);
        assert_eq!(back.window, Some((1234.0, 678.0)));
        assert_eq!(back.controls_width, 321.0);
        assert!(back.sort_lists);
        assert!(!back.real_time);
        let _ = std::fs::remove_file(&path);

        // A missing file yields the defaults.
        let missing = load_from(&dir.join("actr-env-does-not-exist.json"));
        assert_eq!(missing.controls_width, 200.0);
        assert!(missing.real_time);
    }
}

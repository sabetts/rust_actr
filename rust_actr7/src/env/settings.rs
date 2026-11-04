//! Persistent Environment settings (window size, panel layout, options),
//! stored as JSON in `~/.actr-env.json` — the Rust counterpart of the Tcl
//! environment's `init/10-userguisettings.tcl` and `init/99-userconfig.tcl`.

use std::collections::HashMap;
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

    // --- Tcl `99-userconfig.tcl` options ---
    pub update_when_stepped: bool,
    pub use_env_window: bool,
    pub show_copyrights: bool,
    pub use_localhost: bool,
    pub kill_model_windows: bool,
    pub use_smart_load: bool,
    pub save_before_reload: bool,
    pub save_backups: bool,
    pub edit_warn: bool,

    /// Production-grid colours (`init/10-userguisettings.tcl`).
    pub p_selected: String,
    pub p_matched: String,
    pub p_mismatched: String,

    /// Per-tool window geometry `(x, y, w, h)`.
    pub tool_geometry: HashMap<String, [f32; 4]>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            window: None,
            controls_width: 200.0,
            sort_lists: true,
            real_time: true,
            update_when_stepped: true,
            use_env_window: true,
            show_copyrights: true,
            use_localhost: false,
            kill_model_windows: true,
            use_smart_load: false,
            save_before_reload: true,
            save_backups: true,
            edit_warn: true,
            p_selected: "#44da22".into(),
            p_matched: "#fca31d".into(),
            p_mismatched: "#e1031e".into(),
            tool_geometry: HashMap::new(),
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
    let flag = |k: &str, d: bool| match v.get(k) {
        Some(Json::Bool(b)) => *b,
        _ => d,
    };
    let string = |k: &str, d: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| d.to_string())
    };
    if let (Some(w), Some(h)) = (num("window_w"), num("window_h")) {
        s.window = Some((w as f32, h as f32));
    }
    if let Some(w) = num("controls_width") {
        s.controls_width = w as f32;
    }
    s.sort_lists = flag("sort_lists", s.sort_lists);
    s.real_time = flag("real_time", s.real_time);
    s.update_when_stepped = flag("update_when_stepped", s.update_when_stepped);
    s.use_env_window = flag("use_env_window", s.use_env_window);
    s.show_copyrights = flag("show_copyrights", s.show_copyrights);
    s.use_localhost = flag("use_localhost", s.use_localhost);
    s.kill_model_windows = flag("kill_model_windows", s.kill_model_windows);
    s.use_smart_load = flag("use_smart_load", s.use_smart_load);
    s.save_before_reload = flag("save_before_reload", s.save_before_reload);
    s.save_backups = flag("save_backups", s.save_backups);
    s.edit_warn = flag("edit_warn", s.edit_warn);
    s.p_selected = string("p_selected", &s.p_selected);
    s.p_matched = string("p_matched", &s.p_matched);
    s.p_mismatched = string("p_mismatched", &s.p_mismatched);
    if let Some(Json::Obj(entries)) = v.get("tool_geometry") {
        for (name, value) in entries {
            if let Json::Arr(a) = value {
                let vals: Vec<f32> = a
                    .iter()
                    .filter_map(|x| x.as_f64().map(|n| n as f32))
                    .collect();
                if vals.len() == 4 {
                    s.tool_geometry
                        .insert(name.clone(), [vals[0], vals[1], vals[2], vals[3]]);
                }
            }
        }
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
        (
            "update_when_stepped".to_string(),
            Json::Bool(s.update_when_stepped),
        ),
        ("use_env_window".to_string(), Json::Bool(s.use_env_window)),
        ("show_copyrights".to_string(), Json::Bool(s.show_copyrights)),
        ("use_localhost".to_string(), Json::Bool(s.use_localhost)),
        (
            "kill_model_windows".to_string(),
            Json::Bool(s.kill_model_windows),
        ),
        ("use_smart_load".to_string(), Json::Bool(s.use_smart_load)),
        (
            "save_before_reload".to_string(),
            Json::Bool(s.save_before_reload),
        ),
        ("save_backups".to_string(), Json::Bool(s.save_backups)),
        ("edit_warn".to_string(), Json::Bool(s.edit_warn)),
        ("p_selected".to_string(), Json::Str(s.p_selected.clone())),
        ("p_matched".to_string(), Json::Str(s.p_matched.clone())),
        (
            "p_mismatched".to_string(),
            Json::Str(s.p_mismatched.clone()),
        ),
    ];
    if let Some((w, h)) = s.window {
        obj.push(("window_w".to_string(), Json::Float(w as f64)));
        obj.push(("window_h".to_string(), Json::Float(h as f64)));
    }
    if !s.tool_geometry.is_empty() {
        let geometry = s
            .tool_geometry
            .iter()
            .map(|(name, [x, y, w, h])| {
                (
                    name.clone(),
                    Json::Arr(vec![
                        Json::Float(*x as f64),
                        Json::Float(*y as f64),
                        Json::Float(*w as f64),
                        Json::Float(*h as f64),
                    ]),
                )
            })
            .collect();
        obj.push(("tool_geometry".to_string(), Json::Obj(geometry)));
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
            update_when_stepped: false,
            use_env_window: false,
            show_copyrights: false,
            use_localhost: true,
            kill_model_windows: false,
            use_smart_load: true,
            save_before_reload: false,
            save_backups: false,
            edit_warn: false,
            p_selected: "#112233".into(),
            p_matched: "#445566".into(),
            p_mismatched: "#778899".into(),
            tool_geometry: HashMap::from([("Event Queue".to_string(), [10.0, 20.0, 300.0, 200.0])]),
        };
        save_to(&path, &s);
        let back = load_from(&path);
        assert_eq!(back.window, Some((1234.0, 678.0)));
        assert_eq!(back.controls_width, 321.0);
        assert!(back.sort_lists);
        assert!(!back.real_time);
        assert!(!back.update_when_stepped);
        assert!(!back.use_env_window);
        assert!(back.use_localhost);
        assert!(back.use_smart_load);
        assert_eq!(back.p_selected, "#112233");
        assert_eq!(
            back.tool_geometry.get("Event Queue"),
            Some(&[10.0, 20.0, 300.0, 200.0])
        );
        let _ = std::fs::remove_file(&path);

        // A missing file yields the defaults.
        let missing = load_from(&dir.join("actr-env-does-not-exist.json"));
        assert_eq!(missing.controls_width, 200.0);
        assert!(missing.real_time);
        assert!(missing.sort_lists);
    }
}

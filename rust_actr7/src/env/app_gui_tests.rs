//! Headless click-through tests for the native Environment.

use std::cell::Cell;
use std::rc::Rc;

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;

use crate::json::Json;

use super::super::backend::{BackendEvent, EnvBackend};
use super::*;

#[path = "app_gui_tests/support.rs"]
mod support;
#[path = "app_gui_tests/controls.rs"]
mod controls;
#[path = "app_gui_tests/viewers.rs"]
mod viewers;
#[path = "app_gui_tests/histories.rs"]
mod histories;
#[path = "app_gui_tests/visualizations.rs"]
mod visualizations;
#[path = "app_gui_tests/windows.rs"]
mod windows;
#[path = "app_gui_tests/editors.rs"]
mod editors;

use support::*;

struct ScriptedBackend {
    gets: Rc<Cell<usize>>,
}

impl EnvBackend for ScriptedBackend {
    fn evaluate(
        &mut self,
        name: &str,
        _model: Option<&str>,
        _args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        match name {
            "current-model" => Ok(vec![Json::Str("COUNT".into())]),
            "mp-models" => Ok(vec![Json::Arr(vec![Json::Str("COUNT".into())])]),
            "history-data-available" => Ok(vec![Json::Bool(true)]),
            "start-incremental-history-data" => Ok(vec![Json::Int(7)]),
            "get-incremental-history-data" => {
                let n = self.gets.get() + 1;
                self.gets.set(n);
                match n {
                    1 => Ok(vec![Json::Str(
                        r#"[[0,"LOW","COUNT","history line"]]"#.into(),
                    )]),
                    2 => Ok(vec![Json::Str("null".into())]),
                    _ => Err("history id used after null".into()),
                }
            }
            "act-r-version" => Ok(vec![Json::Str("test".into())]),
            _ => Ok(vec![Json::Bool(true)]),
        }
    }

    fn take_events(&mut self) -> Vec<BackendEvent> {
        Vec::new()
    }

    fn current_model(&self) -> Option<String> {
        Some("COUNT".into())
    }

    fn models(&self) -> Vec<String> {
        vec!["COUNT".into()]
    }

    fn set_current_model(&mut self, _name: Option<String>) {}

    fn describe(&self) -> String {
        "scripted".into()
    }

    fn register_window_handler(&mut self, _name: &str) {}

    fn register_stepper_handlers(&mut self, _next: &str, _update: &str) -> Result<(), String> {
        Ok(())
    }

    fn command_doc(&mut self, _name: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn begin_run(&mut self, _secs: f64, _full: bool) -> bool {
        false
    }

    fn poll_run(&mut self) -> Result<bool, String> {
        Ok(false)
    }

    fn stop_run(&mut self) {}

    fn is_running(&self) -> bool {
        false
    }

    fn set_real_time(&mut self, _on: bool) {}
}

#[test]
fn control_panel_labels_are_clean_and_run_controls_are_present() {
    let mut h = inproc_harness();
    h.run();

    for label in [
        "Load code...",
        "Load model...",
        "Options...",
        "Close Exp Windows",
        "Run",
        "Run full time",
        "Step",
        "Stop",
    ] {
        assert!(
            h.query_all_by_label(label).next().is_some(),
            "missing control label {label:?}"
        );
    }
    for label in ["Load codeâ€¦", "Load modelâ€¦", "Optionsâ€¦"] {
        assert!(
            h.query_all_by_label(label).next().is_none(),
            "mojibake label still present: {label:?}"
        );
    }
}

#[test]
fn every_control_panel_tool_button_opens_and_closes() {
    let mut h = inproc_harness();
    h.run();
    load_count(h.state_mut());

    let tools = [
        ("Declarative", ToolKind::Declarative),
        ("Chunk Tree", ToolKind::ChunkTree),
        ("Buffers", ToolKind::Buffers),
        ("Buffer Status", ToolKind::BufferStatus),
        ("Procedural", ToolKind::Procedural),
        ("Parameters", ToolKind::Parameters),
        ("Visicon", ToolKind::Visicon),
        ("Audicon", ToolKind::Audicon),
        ("Text Trace", ToolKind::TextTrace),
        ("Retrieval", ToolKind::RetrievalHistory),
        ("Blending", ToolKind::BlendingHistory),
        ("Buffer", ToolKind::BufferHistory),
        ("Visicon History", ToolKind::VisiconHistory),
        ("Audicon History", ToolKind::AudiconHistory),
        ("Production Grid", ToolKind::ProductionGrid),
        ("Production Graph", ToolKind::ProductionGraph),
        ("Graphic Trace →", ToolKind::HorizontalGraphicTrace),
        ("Graphic Trace ↓", ToolKind::VerticalGraphicTrace),
        ("BOLD Graph", ToolKind::BoldGraph),
        ("BOLD 2D", ToolKind::Bold2D),
        ("BOLD 3D", ToolKind::Bold3D),
        ("Select Buffers", ToolKind::BufferPicker),
        ("Stepper", ToolKind::Stepper),
        ("Event Queue", ToolKind::EventQueue),
        ("Connections", ToolKind::Connections),
        ("About", ToolKind::About),
    ];

    for (label, kind) in tools {
        h.get_by_label(label).click();
        h.run();
        let app = h.state_mut();
        let Some(tool) = app.tools.iter_mut().find(|t| t.kind == kind) else {
            panic!("{label:?} did not open the expected tool window");
        };
        assert!(!tool.title.contains("##"), "dirty title: {:?}", tool.title);
        tool.open = false;
        h.run();
    }
}

#[test]
fn opening_stepper_uses_a_clean_window_title() {
    let mut h = inproc_harness();
    h.run();
    h.get_by_label("Stepper").click();
    h.run();

    let app = h.state();
    assert!(app.stepper.id.is_some(), "stepper did not start");
    assert!(
        app.tools.iter().all(|t| !t.title.contains("##")),
        "tool titles contain egui id suffixes: {:?}",
        app.tools.iter().map(|t| &t.title).collect::<Vec<_>>()
    );
}

#[test]
fn text_trace_all_detail_uses_lisp_t() {
    let (mut h, calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.eval("run", vec![Json::Float(1.0)]);
        app.open_tool(ToolKind::TextTrace);
    }
    h.run();
    {
        let app = h.state_mut();
        let tool = app
            .tools
            .iter_mut()
            .find(|t| t.kind == ToolKind::TextTrace)
            .expect("text trace tool");
        tool.history_detail = "all".into();
        tool.refresh_pending = true;
    }
    h.run();

    let calls = calls.borrow();
    assert!(
        calls.iter().any(|c| {
            c.name == "history-data-available" && c.args.get(3).and_then(Json::as_str) == Some("t")
        }),
        "all detail was not mapped to t for availability: {calls:?}",
        calls = calls.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
    assert!(
        calls.iter().any(|c| {
            c.name == "start-incremental-history-data"
                && c.args.get(5).and_then(Json::as_str) == Some("t")
        }),
        "all detail was not mapped to t for the transfer"
    );
}

#[test]
fn close_exp_windows_button_clears_window_state() {
    let mut h = inproc_harness();
    h.run();
    load_count(h.state_mut());
    h.state_mut().apply_window_op(vec![
        Json::Str("open".into()),
        Json::Str("Demo".into()),
        Json::Int(10),
        Json::Int(20),
        Json::Int(300),
        Json::Int(200),
    ]);
    h.run();
    assert_eq!(h.state().windows.len(), 1, "window was not opened");

    h.get_by_label("Close Exp Windows").click();
    h.run();
    assert!(
        h.state().windows.is_empty(),
        "Close Exp Windows left experiment windows behind"
    );
}

fn lisp_harness() -> Harness<'static, App> {
    let address = super::super::connection::discover();
    let client = super::super::Client::connect(&address.host, address.port)
        .unwrap_or_else(|e| panic!("connect to {}: {e}", address.label()));
    app_with_backend(Box::new(client))
}

fn wait_for_windows(app: &mut App, expected: usize) {
    for _ in 0..200 {
        app.drain_events();
        if app.windows.len() == expected {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!(
        "expected {expected} experiment windows, have {:?}",
        app.windows.iter().map(|w| &w.title).collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "requires a running Lisp server; use fixtures/gui/run_lisp_gui.py"]
fn lisp_gui_smoke() {
    let mut h = lisp_harness();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::TextTrace);
        app.eval("reset", Vec::new());
        app.eval("run", vec![Json::Float(1.0)]);
        let tool = app
            .tools
            .iter_mut()
            .find(|t| t.kind == ToolKind::TextTrace)
            .expect("text trace tool");
        tool.history_detail = "all".into();
        tool.refresh_pending = true;
    }
    h.run();

    {
        let app = h.state();
        assert!(
            !app.trace.contains("Invalid detail level"),
            "Lisp rejected the all detail level: {}",
            app.trace
        );
        let tool = app
            .tools
            .iter()
            .find(|t| t.kind == ToolKind::TextTrace)
            .expect("text trace tool");
        assert!(
            tool.history_text.contains("PRODUCTION-FIRED") || tool.history_text.contains("Trace"),
            "no trace history rendered: {:?}",
            tool.history_text
        );
    }

    // The same real dispatcher connection also exercises Lisp's nested AGI
    // callback shape.
    {
        let app = h.state_mut();
        app.eval(
            "open-exp-window",
            vec![
                Json::Str("GUI E2E".into()),
                Json::Arr(vec![
                    Json::Arr(vec![Json::Str("visible".into()), Json::Bool(true)]),
                    Json::Arr(vec![Json::Str("width".into()), Json::Int(320)]),
                    Json::Arr(vec![Json::Str("height".into()), Json::Int(200)]),
                ]),
            ],
        );
        wait_for_windows(app, 1);
        assert_eq!(app.windows[0].title, "GUI E2E");
        app.eval("close-all-exp-windows", Vec::new());
        wait_for_windows(app, 0);
    }
}

#[test]
fn history_transfer_stops_after_lisp_null() {
    let gets = Rc::new(Cell::new(0));
    let backend = ScriptedBackend { gets: gets.clone() };
    let mut h = app_with_backend(Box::new(backend));
    h.run();
    h.state_mut().open_tool(ToolKind::TextTrace);
    h.run();

    assert_eq!(gets.get(), 2, "history id was requested after null");
    assert!(
        h.state().tools[0].history_text.contains("history line"),
        "history data was not rendered: {:?}",
        h.state().tools[0].history_text
    );
}

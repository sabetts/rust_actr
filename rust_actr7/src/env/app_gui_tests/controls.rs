//! Control-panel, status, run, and Options coverage.

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;

use crate::json::Json;

use super::super::super::backend::{BackendEvent, TraceStream};
use super::super::*;
use super::support::*;

fn has_call(calls: &[Call], name: &str) -> bool {
    calls.iter().any(|c| c.name == name)
}

#[test]
fn status_bar_shows_backend_model_and_error() {
    let (mut h, _calls, _state) = mock_harness();
    {
        let app = h.state_mut();
        app.error = Some("backend exploded".into());
    }
    h.run();

    assert!(h.query_by_label("mock").is_some());
    assert!(h.query_by_label("model: COUNT").is_some());
    assert!(h.query_by_label("backend exploded").is_some());
}

#[test]
fn current_model_menu_switches_model_and_scopes_new_tools() {
    let (mut h, calls, state) = mock_harness();
    {
        let mut state = state.borrow_mut();
        state.models = vec!["A".into(), "B".into()];
        state.current_model = Some("A".into());
    }
    h.run();
    h.get_by_role_and_label(Role::Button, "Refresh").click();
    h.run();

    h.get_by_role(Role::ComboBox).click();
    h.run();
    h.get_by_label("B").click();
    h.run();

    assert_eq!(h.state().selected_model.as_deref(), Some("B"));
    assert!(
        calls
            .borrow()
            .iter()
            .any(|c| c.name == "set-current-model"
                && c.args.first().and_then(Json::as_str) == Some("B")),
        "model menu did not select B through the backend"
    );

    h.state_mut().open_tool(ToolKind::Declarative);
    h.run();
    assert!(
        h.state()
            .tools
            .iter()
            .any(|t| t.kind == ToolKind::Declarative && t.title == "Declarative (B)"),
        "model-scoped tool did not use the selected model"
    );
}

#[test]
fn refresh_model_menu_reads_models_and_selects_a_current_model() {
    let (mut h, _calls, state) = mock_harness();
    {
        let mut state = state.borrow_mut();
        state.models = vec!["A".into()];
        state.current_model = Some("A".into());
    }
    h.run();
    {
        let mut state = state.borrow_mut();
        state.models = vec!["C".into()];
    }

    h.get_by_role_and_label(Role::Button, "Refresh").click();
    h.run();

    assert_eq!(h.state().selected_model.as_deref(), Some("C"));
    assert_eq!(h.state().models, vec!["C".to_string()]);
}

#[test]
fn trace_filters_hide_and_clear_streams() {
    let (mut h, _calls, state) = mock_harness();
    h.run();
    h.state_mut().trace.clear();
    h.run();

    h.get_by_label("warning").click();
    h.run();
    state.borrow_mut().event(BackendEvent::Trace {
        stream: TraceStream::Warning,
        line: "warning line".into(),
    });
    state.borrow_mut().event(BackendEvent::Trace {
        stream: TraceStream::Model,
        line: "model line".into(),
    });
    h.run();

    assert!(!h.state().trace.contains("warning line"));
    assert!(h.state().trace.contains("model line"));

    h.get_by_role_and_label(Role::Button, "Clear trace").click();
    h.run();
    assert!(h.state().trace.is_empty());
}

#[test]
fn evaluate_debug_command_runs_on_go() {
    let (mut h, calls, state) = mock_harness();
    state
        .borrow_mut()
        .response("run", vec![Json::Str("ok".into())]);
    h.run();

    h.get_by_role(Role::TextInput).focus();
    h.run();
    h.get_by_role(Role::TextInput).type_text("run");
    h.run();
    assert_eq!(h.state().command, "run");
    h.get_by_role_and_label(Role::Button, "Go")
        .click_accesskit();
    h.run();

    assert!(
        h.state().result.contains("ok"),
        "result: {:?}",
        h.state().result
    );
    assert!(has_call(&calls.borrow(), "run"));
}

#[test]
fn run_step_and_stop_use_the_expected_backend_paths() {
    let (mut h, calls, state) = mock_harness();
    state.borrow_mut().begin_run_result = true;
    h.run();

    h.get_by_role_and_label(Role::Button, "Run").click();
    h.run_steps(1);
    assert!(h.state().running);
    assert!(calls.borrow().iter().any(|c| {
        c.name == "begin-run"
            && c.args.first() == Some(&Json::Float(10.0))
            && c.args.get(1) == Some(&Json::Bool(false))
    }));

    h.get_by_role_and_label(Role::Button, "Stop").click();
    h.run_steps(1);
    assert!(!state.borrow().running);
    assert!(has_call(&calls.borrow(), "stop-run"));

    state.borrow_mut().begin_run_result = false;
    h.state_mut().running = false;
    h.run();
    h.get_by_role_and_label(Role::Button, "Run full time")
        .click();
    h.run_steps(1);
    assert!(calls
        .borrow()
        .iter()
        .any(|c| { c.name == "begin-run" && c.args.get(1) == Some(&Json::Bool(true)) }));

    h.state_mut().running = false;
    h.run();
    h.get_by_role_and_label(Role::Button, "Step").click();
    h.run_steps(1);
    assert!(has_call(&calls.borrow(), "step-once"));
}

#[test]
fn reset_reload_and_clear_trace_are_wired() {
    let (mut h, calls, _state) = mock_harness();
    h.run();

    h.get_by_role_and_label(Role::Button, "Reset").click();
    h.run_steps(1);
    h.get_by_role_and_label(Role::Button, "Reload").click();
    h.run_steps(1);

    let calls = calls.borrow();
    assert!(has_call(&calls, "reset"));
    assert!(has_call(&calls, "reload"));
}

#[test]
fn options_toggle_settings_under_a_temporary_home() {
    let _home = TempHome::new();
    let (mut h, calls, state) = mock_harness();
    h.run();

    h.get_by_role_and_label(Role::Button, "Options...").click();
    h.run();
    assert!(h.state().show_options);

    h.get_by_role_and_label(Role::Window, "Options")
        .get_by_label("sort viewer lists")
        .click();
    h.run();
    assert!(!h.state().settings.sort_lists);

    h.get_by_role_and_label(Role::Window, "Options")
        .get_by_label("real time")
        .click();
    h.run();
    assert!(!h.state().settings.real_time);
    assert!(!state.borrow().real_time);
    assert!(
        calls.borrow().iter().any(|c| c.name == "set-real-time"),
        "real-time option did not update the backend"
    );

    let saved = crate::env::settings::load();
    assert!(!saved.sort_lists);
    assert!(!saved.real_time);

    h.get_by_role_and_label(Role::Window, "Options")
        .get_by_label("Close window")
        .click();
    h.run();
    assert!(!h.state().show_options);
}

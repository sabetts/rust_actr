//! Viewer-tool and lifecycle interactions.

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;

use crate::json::Json;

use super::super::super::backend::{ConnectionInfo, TraceStream};
use super::super::*;
use super::support::*;

fn tool_after<'a>(h: &'a egui_kittest::Harness<'static, App>, kind: ToolKind) -> &'a ToolWindow {
    h.state()
        .tools
        .iter()
        .find(|t| t.kind == kind)
        .unwrap_or_else(|| panic!("missing tool"))
}

fn tool_after_mut<'a>(
    h: &'a mut egui_kittest::Harness<'static, App>,
    kind: ToolKind,
) -> &'a mut ToolWindow {
    h.state_mut()
        .tools
        .iter_mut()
        .find(|t| t.kind == kind)
        .unwrap_or_else(|| panic!("missing tool"))
}

#[test]
fn declarative_selects_chunks_filters_and_whynot() {
    let (mut h, calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::Declarative);
    }
    h.run();

    let title = "Declarative (COUNT)";
    let (chunk, filter) = {
        let tool = tool_after(&h, ToolKind::Declarative);
        (
            tool.dm_chunks.first().cloned().expect("default DM chunk"),
            tool.dm_filters
                .iter()
                .find(|f| f.as_str() != "_none_")
                .cloned()
                .unwrap_or_else(|| "_none_".into()),
        )
    };

    h.get_by_role_and_label(Role::Window, title)
        .get_by_label(&chunk)
        .click_accesskit();
    h.run();
    let tool = tool_after(&h, ToolKind::Declarative);
    assert!(tool.dm_sel.is_some());
    assert!(!tool.dm_detail.is_empty(), "chunk detail was empty");

    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("Why not?")
        .click_accesskit();
    h.run();
    assert!(
        calls.borrow().iter().any(|c| c.name == "dm-whynot-text"),
        "Why not? did not query dm-whynot-text"
    );

    h.get_by_role_and_label(Role::Window, title)
        .get_by_role(Role::ComboBox)
        .click();
    h.run();
    h.get_by_role_and_label(Role::Button, &filter)
        .click_accesskit();
    h.run();
    assert_eq!(
        tool_after(&h, ToolKind::Declarative).dm_filter,
        filter,
        "filter selection was not retained"
    );
}

#[test]
fn chunk_tree_renders_the_selected_chunk() {
    let (mut h, _calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::ChunkTree);
    }
    h.run();

    let title = "Chunk Tree (COUNT)";
    let chunk = tool_after(&h, ToolKind::ChunkTree)
        .tree_chunks
        .first()
        .cloned()
        .expect("tree chunk");
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label(&chunk)
        .click_accesskit();
    h.run();

    let tool = tool_after(&h, ToolKind::ChunkTree);
    assert!(tool.tree_sel.is_some());
    assert!(
        tool.tree_text.contains(&chunk),
        "tree detail: {:?}",
        tool.tree_text
    );
}

#[test]
fn buffers_switch_modes_and_query_details() {
    let (mut h, calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::Buffers);
    }
    h.run();

    let title = "Buffers (COUNT)";
    let buffer = tool_after(&h, ToolKind::Buffers)
        .buffers
        .first()
        .cloned()
        .expect("model buffer");
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label(&buffer)
        .click_accesskit();
    h.run();
    assert!(
        !tool_after(&h, ToolKind::Buffers).buf_detail.is_empty(),
        "buffer contents were empty"
    );

    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("Status")
        .click_accesskit();
    h.run();
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label(&buffer)
        .click_accesskit();
    h.run();
    assert!(calls
        .borrow()
        .iter()
        .any(|c| c.name == "printed-buffer-status"));
}

#[test]
fn procedural_queries_details_and_whynot() {
    let (mut h, calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::Procedural);
    }
    h.run();

    let title = "Procedural (COUNT)";
    let production = tool_after(&h, ToolKind::Procedural)
        .prods
        .first()
        .cloned()
        .expect("production");
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label(&production)
        .click_accesskit();
    h.run();
    assert!(
        !tool_after(&h, ToolKind::Procedural).prod_detail.is_empty(),
        "production detail was empty"
    );

    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("Why not?")
        .click_accesskit();
    h.run();
    assert!(calls.borrow().iter().any(|c| c.name == "whynot-text"));
}

#[test]
fn parameters_walk_module_and_parameter_lists() {
    let (mut h, calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::Parameters);
    }
    h.run();

    let title = "Parameters (COUNT)";
    let module = tool_after(&h, ToolKind::Parameters)
        .modules
        .first()
        .cloned()
        .expect("parameterized module");
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label(&module)
        .click_accesskit();
    h.run();

    let parameter = tool_after(&h, ToolKind::Parameters)
        .params
        .first()
        .cloned()
        .expect("module parameter");
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label(&parameter)
        .click_accesskit();
    h.run();

    assert!(!tool_after(&h, ToolKind::Parameters).param_detail.is_empty());
    assert!(calls
        .borrow()
        .iter()
        .any(|c| c.name == "printed-parameter-details"));
}

#[test]
fn buffer_picker_records_and_stops_history() {
    let (mut h, calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::BufferPicker);
    }
    h.run();

    let title = "Select Buffers (COUNT)";
    let buffer = tool_after(&h, ToolKind::BufferPicker)
        .buffers
        .first()
        .cloned()
        .expect("pickable buffer");
    let checkbox = h
        .get_by_role_and_label(Role::Window, title)
        .get_by_role_and_label(Role::CheckBox, &buffer);
    checkbox.click_accesskit();
    h.run();
    assert!(calls.borrow().iter().any(|c| {
        c.name == "record-history" && c.args.first().and_then(Json::as_str) == Some(&buffer)
    }));

    h.get_by_role_and_label(Role::Window, title)
        .get_by_role_and_label(Role::CheckBox, &buffer)
        .click_accesskit();
    h.run();
    assert!(calls.borrow().iter().any(|c| {
        c.name == "stop-recording-history" && c.args.first().and_then(Json::as_str) == Some(&buffer)
    }));
}

#[test]
fn connections_load_rows_and_command_documentation() {
    let (mut h, _calls, state) = mock_harness();
    {
        let mut state = state.borrow_mut();
        state.connections = vec![ConnectionInfo {
            name: "client".into(),
            address: "127.0.0.1:2650".into(),
            sent: 4,
            received: 5,
            commands: vec!["run".into()],
            ..Default::default()
        }];
        state.docs.insert("run".into(), Some("run docs".into()));
    }
    h.run();
    h.state_mut().open_tool(ToolKind::Connections);
    h.run();

    let title = "Connections & Commands";
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("Get Status")
        .click_accesskit();
    h.run();
    assert_eq!(
        tool_after(&h, ToolKind::Connections).connections[0].name,
        "client"
    );

    tool_after_mut(&mut h, ToolKind::Connections).command_query = "run".into();
    h.run();
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("Documentation")
        .click_accesskit();
    h.run();
    assert_eq!(tool_after(&h, ToolKind::Connections).text, "run docs");
}

#[test]
fn connections_with_long_command_lists_stay_narrow() {
    let (mut h, _calls, state) = mock_harness();
    state.borrow_mut().connections = vec![ConnectionInfo {
        name: "client".into(),
        address: "127.0.0.1:2650".into(),
        sent: 100,
        received: 100,
        commands: (0..40)
            .map(|i| format!("very-long-command-name-{i:02}"))
            .collect(),
        ..Default::default()
    }];
    state
        .borrow_mut()
        .docs
        .insert("very-long-command-name-00".into(), Some("x".repeat(200)));
    h.run();
    h.state_mut()
        .settings
        .tool_geometry
        .insert("Connections & Commands".into(), [40.0, 40.0, 320.0, 360.0]);
    h.state_mut().open_tool(ToolKind::Connections);
    h.run();

    let width = h
        .state()
        .settings
        .tool_geometry
        .get("Connections & Commands")
        .map(|g| g[2])
        .unwrap_or(0.0);
    assert!(width > 0.0);
    assert!(width <= 380.0, "connections window is too wide: {width}");
    assert_eq!(
        tool_after(&h, ToolKind::Connections).connections[0]
            .commands
            .len(),
        40
    );
    assert_eq!(tool_after(&h, ToolKind::Connections).conn_sel, Some(0));

    h.get_by_role_and_label(Role::Window, "Connections & Commands")
        .get_by_label("very-long-command-name-00")
        .click_accesskit();
    h.run();
    assert_eq!(
        tool_after(&h, ToolKind::Connections).command_query,
        "very-long-command-name-00"
    );
    assert_eq!(tool_after(&h, ToolKind::Connections).text, "x".repeat(200));
    let width = h
        .state()
        .settings
        .tool_geometry
        .get("Connections & Commands")
        .map(|g| g[2])
        .unwrap_or(0.0);
    assert!(
        width <= 380.0,
        "documentation widened the connections window: {width}"
    );
}

#[test]
fn event_queue_and_about_refresh_through_the_backend() {
    let (mut h, calls, state) = mock_harness();
    state
        .borrow_mut()
        .response("mp-queue-text", vec![Json::Str("queue text".into())]);
    state
        .borrow_mut()
        .response("act-r-version", vec![Json::Str("7.0-mock".into())]);
    h.run();

    h.state_mut().open_tool(ToolKind::EventQueue);
    h.state_mut().open_tool(ToolKind::About);
    h.run();
    assert_eq!(
        tool_after(&h, ToolKind::EventQueue).text,
        "queue text",
        "event queue refresh"
    );
    assert_eq!(
        tool_after(&h, ToolKind::About).text,
        "7.0-mock",
        "about refresh"
    );
    assert!(calls.borrow().iter().any(|c| c.name == "mp-queue-text"));
    assert!(calls.borrow().iter().any(|c| c.name == "act-r-version"));
}

#[test]
fn visicon_viewer_and_history_use_real_device_events() {
    let mut h = device_count_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::VisiconHistory);
        app.eval("run", vec![Json::Float(1.0)]);
        app.open_tool(ToolKind::Visicon);
    }
    h.run();

    assert!(
        !tool_after(&h, ToolKind::Visicon).text.is_empty(),
        "printed visicon was empty"
    );
    assert!(
        !tool_after(&h, ToolKind::VisiconHistory)
            .history_records
            .is_empty(),
        "visicon history was empty"
    );
}

#[test]
fn audicon_viewer_and_history_use_real_sound_events() {
    let mut h = inproc_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::AudiconHistory);
        app.eval(
            "new-tone-sound",
            vec![
                Json::Int(1000),
                Json::Float(0.1),
                Json::Float(0.0),
                Json::Bool(true),
            ],
        );
        app.eval("run", vec![Json::Float(1.0)]);
        app.open_tool(ToolKind::Audicon);
    }
    h.run();

    assert!(
        !tool_after(&h, ToolKind::AudiconHistory)
            .history_records
            .is_empty(),
        "audicon history was empty"
    );
    assert!(
        !tool_after(&h, ToolKind::Audicon).text.is_empty(),
        "printed audicon was empty"
    );
}

#[test]
fn closing_history_tool_releases_recording() {
    let (mut h, calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::RetrievalHistory);
    }
    h.run();
    assert!(tool_after(&h, ToolKind::RetrievalHistory).history_recording);

    h.get_by_role_and_label(Role::Window, "Retrieval History (COUNT)")
        .get_by_label("Close window")
        .click_accesskit();
    h.run();
    assert!(!h
        .state()
        .tools
        .iter()
        .any(|t| t.kind == ToolKind::RetrievalHistory));
    assert!(calls
        .borrow()
        .iter()
        .any(|c| c.name == "stop-recording-history"));
}

#[test]
fn closing_stepper_stops_it() {
    let named = |app: &App| app.stepper.id.clone();
    let mut h = inproc_harness();
    h.run();
    {
        let app = h.state_mut();
        load_model(app, &stepper_model());
        app.open_tool(ToolKind::Stepper);
    }
    h.run();
    assert!(named(h.state()).is_some());

    h.get_by_role_and_label(Role::Window, "Stepper")
        .get_by_label("Close window")
        .click_accesskit();
    h.run();
    assert!(named(h.state()).is_none());
}

#[test]
fn restarting_stepper_does_not_reregister_handlers() {
    let (mut h, _calls, state) = mock_harness();
    h.run();
    state.borrow_mut().response(
        "start-stepper",
        vec![Json::Bool(true), Json::Str("STEPPER1".into())],
    );
    h.state_mut().open_tool(ToolKind::Stepper);
    h.run();
    assert_eq!(h.state().stepper.id.as_deref(), Some("STEPPER1"));
    assert_eq!(state.borrow().stepper_registrations, 1);

    h.state_mut().stop_stepper();
    h.run();
    assert!(h.state().stepper.id.is_none());

    state.borrow_mut().response(
        "start-stepper",
        vec![Json::Bool(true), Json::Str("STEPPER2".into())],
    );
    h.state_mut().start_stepper();
    h.run();
    assert_eq!(h.state().stepper.id.as_deref(), Some("STEPPER2"));
    assert_eq!(
        state.borrow().stepper_registrations,
        1,
        "handlers should only be registered once per connection"
    );
}

#[test]
fn model_delete_closes_scoped_tools_and_keeps_global_tools() {
    use crate::env::backend::BackendEvent;

    let (mut h, _calls, state) = mock_harness();
    h.run();
    h.state_mut().open_tool(ToolKind::Declarative);
    h.state_mut().open_tool(ToolKind::EventQueue);
    h.run();

    state
        .borrow_mut()
        .event(BackendEvent::ModelDeleted("COUNT".into()));
    h.run();

    assert!(!h
        .state()
        .tools
        .iter()
        .any(|t| t.kind == ToolKind::Declarative));
    assert!(h
        .state()
        .tools
        .iter()
        .any(|t| t.kind == ToolKind::EventQueue));
    assert_eq!(h.state().selected_model, None);
}

#[test]
fn reopened_tools_are_raised_instead_of_duplicated() {
    let mut h = inproc_harness();
    h.run();
    load_count(h.state_mut());
    h.state_mut().open_tool(ToolKind::Declarative);
    h.run();
    h.state_mut().open_tool(ToolKind::Declarative);
    h.run();
    assert_eq!(
        h.state()
            .tools
            .iter()
            .filter(|t| t.kind == ToolKind::Declarative)
            .count(),
        1
    );
}

#[test]
fn trace_stream_helpers_stay_visible() {
    let mut h = inproc_harness();
    h.run();
    h.state_mut().drain_events();
    assert!(h.state().trace_visible(TraceStream::Model));
}

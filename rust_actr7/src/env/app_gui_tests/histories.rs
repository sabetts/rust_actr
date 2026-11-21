//! History-pane interactions and save/load protocol coverage.

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;

use crate::env::history::HistoryView;
use crate::json::Json;

use super::super::HistoryKind;
use super::super::*;
use super::support::*;

fn seeded_tool(
    h: &mut egui_kittest::Harness<'static, App>,
    kind: ToolKind,
    history: HistoryKind,
    records: Vec<Json>,
) {
    {
        let app = h.state_mut();
        app.open_tool(kind);
    }
    {
        let app = h.state_mut();
        let tool = app.tools.iter_mut().find(|t| t.kind == kind).unwrap();
        tool.refresh_pending = false;
        tool.history_view = App::parse_history_view(history, &records);
        tool.history_records = records;
    }
    h.run();
}

#[test]
fn retrieval_history_real_count_run_and_time_chunk_selection() {
    let (mut h, _calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::RetrievalHistory);
        app.eval("reset", Vec::new());
        app.eval("run", vec![Json::Float(1.0)]);
    }
    h.run();

    let tool = h
        .state()
        .tools
        .iter()
        .find(|t| t.kind == ToolKind::RetrievalHistory)
        .unwrap();
    assert!(
        !tool.history_records.is_empty(),
        "real retrieval history was empty"
    );

    let time = super::super::super::history::format_time(
        super::super::super::history::parse_retrieval(&tool.history_records)[0].time_ms,
    );
    let chunk = super::super::super::history::parse_retrieval(&tool.history_records)[0].chunks[0]
        .name
        .clone();
    h.get_by_role_and_label(Role::Window, "Retrieval History (COUNT)")
        .get_by_label(&time)
        .click_accesskit();
    h.run();
    h.get_by_role_and_label(Role::Window, "Retrieval History (COUNT)")
        .get_by_label(&chunk)
        .click_accesskit();
    h.run();

    let tool = h
        .state()
        .tools
        .iter()
        .find(|t| t.kind == ToolKind::RetrievalHistory)
        .unwrap();
    assert!(tool.history_time_sel.is_some());
    assert!(tool.history_item_sel.is_some());
}

#[test]
fn buffer_history_real_count_run_and_action_selection() {
    let (mut h, _calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.eval("reset", Vec::new());
        app.open_tool(ToolKind::BufferHistory);
        app.eval("run", vec![Json::Float(1.0)]);
    }
    h.run();

    let tool = h
        .state()
        .tools
        .iter()
        .find(|t| t.kind == ToolKind::BufferHistory)
        .unwrap();
    assert!(
        !tool.history_records.is_empty(),
        "real buffer history was empty"
    );
    let entries = super::super::super::history::parse_buffer(&tool.history_records);
    let time = super::super::super::history::format_time(entries[0].time_ms);
    let action = format!(
        "{} {}",
        entries[0].actions[0].buffer, entries[0].actions[0].action
    );
    h.get_by_role_and_label(Role::Window, "Buffer History (COUNT)")
        .get_by_label(&time)
        .click_accesskit();
    h.run();
    h.get_by_role_and_label(Role::Window, "Buffer History (COUNT)")
        .get_by_label(&action)
        .click_accesskit();
    h.run();

    let tool = h
        .state()
        .tools
        .iter()
        .find(|t| t.kind == ToolKind::BufferHistory)
        .unwrap();
    assert!(tool.history_time_sel.is_some());
    assert!(tool.history_item_sel.is_some());
}

#[test]
fn seeded_blending_and_perceptual_panes_select_times() {
    let mut h = mock_harness().0;
    h.run();
    seeded_tool(
        &mut h,
        ToolKind::BlendingHistory,
        HistoryKind::Blending,
        blending_records(),
    );
    h.get_by_role_and_label(Role::Window, "Blending History (COUNT)")
        .get_by_label("0.300")
        .click_accesskit();
    h.run();
    assert!(h
        .state()
        .tools
        .iter()
        .find(|t| t.kind == ToolKind::BlendingHistory)
        .unwrap()
        .history_time_sel
        .is_some());

    seeded_tool(
        &mut h,
        ToolKind::VisiconHistory,
        HistoryKind::Visicon,
        perceptual_records(),
    );
    h.get_by_role_and_label(Role::Window, "Visicon History (COUNT)")
        .get_by_label("0.500")
        .click_accesskit();
    h.run();
    assert!(h
        .state()
        .tools
        .iter()
        .find(|t| t.kind == ToolKind::VisiconHistory)
        .unwrap()
        .history_time_sel
        .is_some());
}

#[test]
fn trace_detail_start_and_end_reach_the_history_commands() {
    let (mut h, calls, state) = mock_harness();
    state
        .borrow_mut()
        .response("start-incremental-history-data", vec![Json::Int(7)]);
    h.run();
    h.state_mut().open_tool(ToolKind::TextTrace);
    h.run();

    {
        let app = h.state_mut();
        let tool = app
            .tools
            .iter_mut()
            .find(|t| t.kind == ToolKind::TextTrace)
            .unwrap();
        tool.history_detail = "high".into();
        tool.history_start = "0.25".into();
        tool.history_end = "1.5".into();
        tool.refresh_pending = true;
    }
    h.run();

    let calls = calls.borrow();
    assert!(calls.iter().any(|c| {
        c.name == "history-data-available" && c.args.get(3).and_then(Json::as_str) == Some("high")
    }));
    assert!(calls.iter().any(|c| {
        c.name == "start-incremental-history-data"
            && c.args.get(3) == Some(&Json::Float(0.25))
            && c.args.get(4) == Some(&Json::Float(1.5))
            && c.args.get(5).and_then(Json::as_str) == Some("high")
    }));
}

#[test]
fn empty_history_renders_no_data() {
    let (mut h, _calls, state) = mock_harness();
    state
        .borrow_mut()
        .response("history-data-available", vec![Json::Bool(false)]);
    h.run();
    h.state_mut().open_tool(ToolKind::TextTrace);
    h.run();

    assert_eq!(
        h.state().tools[0].history_text,
        "No Data available",
        "empty history marker"
    );
}

#[test]
fn save_and_load_history_use_the_expected_protocol() {
    let (mut h, calls, state) = mock_harness();
    state
        .borrow_mut()
        .response("start-incremental-history-data", vec![Json::Int(9)]);
    h.run();
    h.state_mut().open_tool(ToolKind::TextTrace);
    h.run();

    let path = temp_path("history.txt");
    let path_text = path.to_string_lossy().into_owned();
    {
        let app = h.state_mut();
        let mut tool = app.tools.remove(0);
        app.save_history(&tool, &path);
        app.load_history(&mut tool, &path);
        app.tools.push(tool);
    }

    let calls = calls.borrow();
    assert!(calls.iter().any(|c| {
        c.name == "save-history-data"
            && c.args.first().and_then(Json::as_str) == Some("trace-history")
    }));
    assert!(calls.iter().any(|c| {
        c.name == "start-incremental-history-data"
            && c.args.get(2).and_then(Json::as_str) == Some(path_text.as_str())
    }));
    let _ = std::fs::remove_file(path);
}

#[test]
fn history_recording_is_released_when_the_tool_closes() {
    let (mut h, calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.open_tool(ToolKind::BlendingHistory);
    }
    h.run();

    h.get_by_role_and_label(Role::Window, "Blending History (COUNT)")
        .get_by_label("Close window")
        .click_accesskit();
    h.run();
    assert!(calls
        .borrow()
        .iter()
        .any(|c| c.name == "stop-recording-history"));
}

#[test]
fn history_windows_honor_small_sizes_and_do_not_snap_on_bottom_press() {
    let (mut h, _calls) = recording_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
    }

    let views = [
        (
            ToolKind::RetrievalHistory,
            "Retrieval History (COUNT)",
            HistoryView::Retrieval(Vec::new()),
        ),
        (
            ToolKind::BlendingHistory,
            "Blending History (COUNT)",
            HistoryView::Blending(Vec::new()),
        ),
        (
            ToolKind::BufferHistory,
            "Buffer History (COUNT)",
            HistoryView::Buffer(Vec::new()),
        ),
        (
            ToolKind::VisiconHistory,
            "Visicon History (COUNT)",
            HistoryView::Perceptual(Vec::new()),
        ),
        (
            ToolKind::AudiconHistory,
            "Audicon History (COUNT)",
            HistoryView::Perceptual(Vec::new()),
        ),
    ];

    for (kind, title, view) in views {
        {
            let app = h.state_mut();
            app.settings
                .tool_geometry
                .insert(title.into(), [40.0, 40.0, 520.0, 200.0]);
            app.open_tool(kind);
        }
        {
            let app = h.state_mut();
            let tool = app.tools.iter_mut().find(|t| t.kind == kind).unwrap();
            tool.refresh_pending = false;
            tool.history_view = Some(view);
        }
        h.run();

        let initial = h.get_by_role_and_label(Role::Window, title).rect();
        assert!(
            initial.height() <= 220.0,
            "{title} ignored the requested 200px height: {}",
            initial.height()
        );

        for _ in 0..30 {
            h.run();
        }
        let stable = h.get_by_role_and_label(Role::Window, title).rect();
        assert!(
            (stable.height() - initial.height()).abs() <= 1.0,
            "{title} grew from {} to {} while idle",
            initial.height(),
            stable.height()
        );
        let before = stable;

        let pos = egui::pos2(before.center().x, before.max.y - 1.0);
        h.hover_at(pos);
        h.run();
        h.drag_at(pos);
        h.run();
        h.run();

        let after = h.get_by_role_and_label(Role::Window, title).rect();
        assert!(
            (after.height() - before.height()).abs() <= 1.0,
            "{title} snapped from {} to {} on a plain press",
            before.height(),
            after.height()
        );

        h.drop_at(pos);
        h.run();
        if let Some(tool) = h.state_mut().tools.iter_mut().find(|t| t.kind == kind) {
            tool.open = false;
        }
        h.run();
    }
}

#[test]
fn buffer_history_selected_action_stays_stable() {
    let records = vec![Json::Arr(vec![
        Json::Int(0),
        Json::Arr(vec![Json::Arr(vec![
            Json::Str("retrieval".into()),
            Json::Str("set-buffer-chunk".into()),
            Json::Str("chunk".into()),
            Json::Str("free".into()),
            Json::Str("previous".into()),
            Json::Str("free".into()),
            Json::Str("current".into()),
            Json::Str("free".into()),
        ])]),
    ])];

    let (mut h, _calls) = recording_harness();
    h.run();
    load_count(h.state_mut());
    seeded_tool(
        &mut h,
        ToolKind::BufferHistory,
        HistoryKind::Buffer,
        records,
    );

    let title = "Buffer History (COUNT)";
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("0.000")
        .click_accesskit();
    h.run();
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("retrieval set-buffer-chunk")
        .click_accesskit();
    h.run();

    let tool = h
        .state()
        .tools
        .iter()
        .find(|t| t.kind == ToolKind::BufferHistory)
        .unwrap();
    assert!(tool.history_time_sel.is_some());
    assert!(tool.history_item_sel.is_some());

    let initial = h.get_by_role_and_label(Role::Window, title).rect();
    for _ in 0..30 {
        h.run();
    }
    let stable = h.get_by_role_and_label(Role::Window, title).rect();
    assert!(
        (stable.height() - initial.height()).abs() <= 1.0,
        "buffer history grew from {} to {} after selecting an action",
        initial.height(),
        stable.height()
    );

    let pos = egui::pos2(stable.center().x, stable.max.y - 1.0);
    h.hover_at(pos);
    h.run();
    h.drag_at(pos);
    h.run();
    h.run();
    let pressed = h.get_by_role_and_label(Role::Window, title).rect();
    assert!(
        (pressed.height() - stable.height()).abs() <= 1.0,
        "buffer history snapped from {} to {} on a plain press",
        stable.height(),
        pressed.height()
    );
    h.drop_at(pos);
    h.run();

    if let Some(tool) = h
        .state_mut()
        .tools
        .iter_mut()
        .find(|t| t.kind == ToolKind::BufferHistory)
    {
        tool.open = false;
    }
    h.run();
}

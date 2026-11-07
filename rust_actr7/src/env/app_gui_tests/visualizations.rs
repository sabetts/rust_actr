//! Production grid/graph, graphic-trace, and BOLD controls.

use egui_kittest::kittest::Queryable;
use egui::accesskit::Role;

use crate::json::Json;

use super::super::*;
use super::support::*;

fn find_tool<'a>(h: &'a egui_kittest::Harness<'static, App>, kind: ToolKind) -> &'a ToolWindow {
    h.state().tools.iter().find(|t| t.kind == kind).unwrap()
}

#[test]
fn production_grid_uses_real_count_history_and_scales() {
    let mut h = inproc_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.eval("reset", Vec::new());
        app.open_tool(ToolKind::ProductionGrid);
        app.eval("run", vec![Json::Float(1.0)]);
    }
    h.run();

    let tool = find_tool(&h, ToolKind::ProductionGrid);
    assert!(
        !tool.history_records.is_empty(),
        "production grid history was empty"
    );
    assert!(!tool.grid_hide_empty);

    h.get_by_label("hide unselected columns").click_accesskit();
    h.run();
    let before = find_tool(&h, ToolKind::ProductionGrid).grid_scale;
    h.get_by_label("+").click_accesskit();
    h.run();
    let after = find_tool(&h, ToolKind::ProductionGrid).grid_scale;
    assert!(after > before, "grid scale did not increase");

    h.get_by_label("−").click_accesskit();
    h.run();
    assert_eq!(find_tool(&h, ToolKind::ProductionGrid).grid_scale, before);
    assert!(find_tool(&h, ToolKind::ProductionGrid).grid_hide_empty);
}

#[test]
fn production_graph_switches_modes_and_hides_unused_nodes() {
    let mut h = inproc_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.eval("reset", Vec::new());
        app.open_tool(ToolKind::ProductionGraph);
        app.eval("run", vec![Json::Float(1.0)]);
    }
    h.run();

    let graph = find_tool(&h, ToolKind::ProductionGraph);
    assert!(
        graph.graph_data.as_ref().is_some_and(|g| !g.nodes.is_empty()),
        "production graph had no nodes"
    );
    assert!(!graph.graph_hide_unused);

    h.get_by_role_and_label(Role::Window, "Production Graph (COUNT)")
        .get_by_label("Frequencies")
        .click_accesskit();
    h.run();
    assert!(find_tool(&h, ToolKind::ProductionGraph).graph_data.is_some());
    h.get_by_label("hide unused productions").click_accesskit();
    h.run();
    assert!(find_tool(&h, ToolKind::ProductionGraph).graph_hide_unused);
}

#[test]
fn graphic_trace_uses_real_buffer_trace_and_toggles_controls() {
    let mut h = inproc_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.eval("reset", Vec::new());
        app.eval("record-history", vec![Json::Str("goal".into())]);
        app.open_tool(ToolKind::HorizontalGraphicTrace);
        app.eval("run", vec![Json::Float(1.0)]);
    }
    h.run();

    let title = "Graphic Trace (Horizontal) (COUNT)";
    let tool = find_tool(&h, ToolKind::HorizontalGraphicTrace);
    assert!(
        !tool.history_records.is_empty(),
        "graphic trace history was empty"
    );
    assert!(!tool.graphic_hide_text);
    assert!(!tool.graphic_hide_grid);
    let before = tool.graphic_scale;

    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("+")
        .click_accesskit();
    h.run();
    assert!(
        find_tool(&h, ToolKind::HorizontalGraphicTrace).graphic_scale > before
    );
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("Hide text")
        .click_accesskit();
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("Hide grid")
        .click_accesskit();
    h.run();
    let tool = find_tool(&h, ToolKind::HorizontalGraphicTrace);
    assert!(tool.graphic_hide_text);
    assert!(tool.graphic_hide_grid);
}

#[test]
fn bold_graph_uses_real_buffer_trace() {
    let mut h = inproc_harness();
    h.run();
    {
        let app = h.state_mut();
        load_count(app);
        app.eval("reset", Vec::new());
        app.eval("record-history", vec![Json::Str("goal".into())]);
        app.open_tool(ToolKind::BoldGraph);
        app.eval("run", vec![Json::Float(1.0)]);
    }
    h.run();

    assert!(
        !find_tool(&h, ToolKind::BoldGraph)
            .history_records
            .is_empty(),
        "BOLD history was empty"
    );
}

#[test]
fn bold_views_render_seeded_records_and_scale_controls() {
    let (mut h, _calls, _state) = mock_harness();
    h.run();
    {
        let app = h.state_mut();
        app.open_tool(ToolKind::BoldGraph);
        app.open_tool(ToolKind::Bold2D);
        app.open_tool(ToolKind::Bold3D);
        for kind in [ToolKind::BoldGraph, ToolKind::Bold2D, ToolKind::Bold3D] {
            let tool = app.tools.iter_mut().find(|t| t.kind == kind).unwrap();
            tool.refresh_pending = false;
            tool.history_records = bold_records();
        }
    }
    h.run();

    assert!(h.query_by_label("2D brain view").is_some());
    assert!(h
        .query_by_label("Simplified 3D brain view (scan values as voxel grid)")
        .is_some());

    h.get_by_role_and_label(Role::Window, "BOLD Graph (COUNT)")
        .get_by_role(Role::ComboBox)
        .click();
    h.run();
    h.get_by_role_and_label(Role::Button, "Each")
        .click_accesskit();
    h.run();
    assert_eq!(find_tool(&h, ToolKind::BoldGraph).bold_scale, 1);
}

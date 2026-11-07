//! Real-dispatcher GUI smoke cases. Run through `fixtures/gui/run_lisp_gui.py`.

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;

use crate::json::Json;

use super::super::super::connection;
use super::super::super::Client;
use super::super::*;
use super::support::*;

fn lisp_harness() -> egui_kittest::Harness<'static, App> {
    let address = connection::discover();
    let client = Client::connect(&address.host, address.port)
        .unwrap_or_else(|e| panic!("connect to {}: {e}", address.label()));
    app_with_backend(Box::new(client))
}

fn open_history_and_run(app: &mut App, kind: ToolKind) {
    load_count(app);
    app.eval("reset", Vec::new());
    app.open_tool(kind);
    app.eval("run", vec![Json::Float(1.0)]);
}

#[test]
#[ignore = "requires a running Lisp server; use fixtures/gui/run_lisp_gui.py"]
fn lisp_retrieval_history_smoke() {
    let mut h = lisp_harness();
    {
        let app = h.state_mut();
        open_history_and_run(app, ToolKind::RetrievalHistory);
    }
    h.run();

    let tool = h
        .state()
        .tools
        .iter()
        .find(|t| t.kind == ToolKind::RetrievalHistory)
        .expect("retrieval history tool");
    assert!(
        !tool.history_records.is_empty(),
        "Lisp retrieval history was empty: {:?}",
        tool.history_text
    );
    assert!(!tool.history_text.contains("No Data available"));
}

#[test]
#[ignore = "requires a running Lisp server; use fixtures/gui/run_lisp_gui.py"]
fn lisp_buffer_history_smoke() {
    let mut h = lisp_harness();
    {
        let app = h.state_mut();
        open_history_and_run(app, ToolKind::BufferHistory);
    }
    h.run();

    let tool = h
        .state()
        .tools
        .iter()
        .find(|t| t.kind == ToolKind::BufferHistory)
        .expect("buffer history tool");
    assert!(
        !tool.history_records.is_empty(),
        "Lisp buffer history was empty: {:?}",
        tool.history_text
    );
    assert!(!tool.history_text.contains("No Data available"));
}

#[test]
#[ignore = "requires a running Lisp server; use fixtures/gui/run_lisp_gui.py"]
fn lisp_production_grid_smoke() {
    let mut h = lisp_harness();
    {
        let app = h.state_mut();
        open_history_and_run(app, ToolKind::ProductionGrid);
    }
    h.run();

    let tool = h
        .state()
        .tools
        .iter()
        .find(|t| t.kind == ToolKind::ProductionGrid)
        .expect("production grid tool");
    assert!(
        !tool.history_records.is_empty(),
        "Lisp production grid was empty: {:?}",
        tool.history_text
    );
}

#[test]
#[ignore = "requires a running Lisp server; use fixtures/gui/run_lisp_gui.py"]
fn lisp_connections_and_documentation_smoke() {
    let mut h = lisp_harness();
    h.run();
    h.state_mut().open_tool(ToolKind::Connections);
    h.run();

    let title = "Connections & Commands";
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("Get Status")
        .click_accesskit();
    h.run();
    assert!(
        !h.state().tools[0].connections.is_empty(),
        "Lisp returned no connections"
    );

    h.state_mut().tools[0].command_query = "run".into();
    h.run();
    h.get_by_role_and_label(Role::Window, title)
        .get_by_label("Documentation")
        .click_accesskit();
    h.run();
    assert!(
        !h.state().tools[0].text.is_empty(),
        "Lisp returned no command documentation"
    );
}

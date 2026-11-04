//! The egui application shell: control panel, trace pane, and the
//! read-only inspectors (declarative, buffers, procedural).

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use eframe::egui;

use crate::json::Json;

use super::backend::{result_text, BackendEvent, EnvBackend};
use super::settings::{self, Settings};

/// The central-area views.
/// A floating tool window (the Rust equivalent of a Tcl dialog).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolKind {
    Declarative,
    ChunkTree,
    Buffers,
    BufferStatus,
    Procedural,
    Parameters,
    Visicon,
    Audicon,
    TextTrace,
    ProductionGraph,
    EventQueue,
    Connections,
    About,
    Stepper,
}

impl ToolKind {
    fn title(self) -> &'static str {
        match self {
            ToolKind::Declarative => "Declarative",
            ToolKind::ChunkTree => "Chunk Tree",
            ToolKind::Buffers => "Buffers",
            ToolKind::BufferStatus => "Buffer Status",
            ToolKind::Procedural => "Procedural",
            ToolKind::Parameters => "Parameters",
            ToolKind::Visicon => "Visicon",
            ToolKind::Audicon => "Audicon",
            ToolKind::TextTrace => "Text Trace",
            ToolKind::ProductionGraph => "Production Graph",
            ToolKind::EventQueue => "Event Queue",
            ToolKind::Connections => "Connections & Commands",
            ToolKind::About => "About ACT-R",
            ToolKind::Stepper => "Stepper",
        }
    }

    fn model_scoped(self) -> bool {
        matches!(
            self,
            ToolKind::Declarative
                | ToolKind::ChunkTree
                | ToolKind::Buffers
                | ToolKind::BufferStatus
                | ToolKind::Procedural
                | ToolKind::Parameters
                | ToolKind::Visicon
                | ToolKind::Audicon
                | ToolKind::ProductionGraph
        )
    }
}

/// Which pane a buffer viewer is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BufferMode {
    Contents,
    Status,
}

/// One open floating tool and its transient UI state.
struct ToolWindow {
    id: u64,
    kind: ToolKind,
    title: String,
    model: Option<String>,
    open: bool,
    text: String,
    connections: Vec<crate::env::backend::ConnectionInfo>,
    refresh_pending: bool,

    // Declarative viewer.
    dm_filters: Vec<String>,
    dm_filter: String,
    dm_chunks: Vec<String>,
    dm_sel: Option<usize>,
    dm_detail: String,

    // Chunk tree.
    tree_chunks: Vec<String>,
    tree_sel: Option<usize>,
    tree_text: String,

    // Buffer viewer / status.
    buffers: Vec<String>,
    buf_sel: Option<usize>,
    buf_detail: String,
    buf_mode: BufferMode,

    // Procedural viewer.
    prods: Vec<String>,
    prod_sel: Option<usize>,
    prod_detail: String,
    prod_matched: Vec<bool>,

    // Parameter viewer.
    modules: Vec<String>,
    mod_sel: Option<usize>,
    params: Vec<String>,
    param_sel: Option<usize>,
    param_detail: String,

    // Text trace / production graph.
    history_text: String,
    graph_nodes: Vec<String>,
    graph_edges: Vec<(String, String, u32)>,
}

impl ToolWindow {
    fn new(id: u64, kind: ToolKind, model: Option<String>) -> ToolWindow {
        let title = match &model {
            Some(m) => format!("{} ({m})", kind.title()),
            None => kind.title().to_string(),
        };
        ToolWindow {
            id,
            kind,
            title,
            model,
            open: true,
            text: String::new(),
            connections: Vec::new(),
            refresh_pending: true,
            dm_filters: vec!["_none_".to_string()],
            dm_filter: "_none_".to_string(),
            dm_chunks: Vec::new(),
            dm_sel: None,
            dm_detail: String::new(),
            tree_chunks: Vec::new(),
            tree_sel: None,
            tree_text: String::new(),
            buffers: Vec::new(),
            buf_sel: None,
            buf_detail: String::new(),
            buf_mode: BufferMode::Contents,
            prods: Vec::new(),
            prod_sel: None,
            prod_detail: String::new(),
            prod_matched: Vec::new(),
            modules: Vec::new(),
            mod_sel: None,
            params: Vec::new(),
            param_sel: None,
            param_detail: String::new(),
            history_text: String::new(),
            graph_nodes: Vec::new(),
            graph_edges: Vec::new(),
        }
    }
}

/// A visible experiment window the engine is displaying, with its items.
#[derive(Default, Clone)]
struct VisWin {
    title: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    items: Vec<WinItem>,
    attention: Option<(f32, f32, egui::Color32)>,
    cursor: Option<(f32, f32, egui::Color32)>,
}

/// One item in a visible window (`env-window-features`).
#[derive(Clone)]
enum WinItem {
    Button {
        id: String,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: String,
        color: egui::Color32,
    },
    Text {
        id: String,
        x: f32,
        y: f32,
        text: String,
        color: egui::Color32,
        size: f32,
    },
    Line {
        id: String,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: egui::Color32,
    },
}

impl WinItem {
    fn id(&self) -> &str {
        match self {
            WinItem::Button { id, .. } | WinItem::Text { id, .. } | WinItem::Line { id, .. } => id,
        }
    }
}

/// An ACT-R colour name â†’ egui colour (`color->name`, `misc-utils.lisp`).
fn color(name: &str) -> egui::Color32 {
    use egui::Color32 as C;
    match name.to_lowercase().as_str() {
        "black" => C::BLACK,
        "white" => C::WHITE,
        "red" => C::RED,
        "green" => C::GREEN,
        "blue" => C::BLUE,
        "yellow" => C::YELLOW,
        "cyan" => C::from_rgb(0, 255, 255),
        "magenta" => C::from_rgb(255, 0, 255),
        "pink" => C::from_rgb(255, 192, 203),
        "brown" => C::from_rgb(150, 75, 0),
        "gray" | "grey" => C::GRAY,
        "light-gray" => C::LIGHT_GRAY,
        "dark-gray" => C::DARK_GRAY,
        "purple" => C::from_rgb(128, 0, 128),
        "dark-red" => C::from_rgb(139, 0, 0),
        "dark-green" => C::from_rgb(0, 100, 0),
        "dark-blue" => C::from_rgb(0, 0, 139),
        "dark-cyan" => C::from_rgb(0, 139, 139),
        "dark-magenta" => C::from_rgb(139, 0, 139),
        "dark-yellow" => C::from_rgb(139, 139, 0),
        "light-blue" => C::from_rgb(173, 216, 230),
        _ => C::BLACK,
    }
}

pub struct App {
    backend: Box<dyn EnvBackend>,
    /// Where the file dialog opens (the current directory; `None` in client
    /// mode).
    start_dir: Option<PathBuf>,
    trace: String,
    command: String,
    result: String,
    /// `run` duration in seconds.
    run_secs: f64,
    /// Persistent GUI settings.
    settings: Settings,
    /// Models in load order and the client-selected model.
    models: Vec<String>,
    selected_model: Option<String>,
    /// Whether an interactive run is in progress (from run signals).
    running: bool,
    /// Whether the Options window is open.
    show_options: bool,
    /// Last error, shown in the status bar.
    error: Option<String>,

    /// Global refresh request forwarded to every open tool.
    refresh_pending: bool,
    /// The visible experiment windows the engine is displaying.
    windows: Vec<VisWin>,
    /// Floating tool windows (viewers, Event Queue, Connections, About, …).
    tools: Vec<ToolWindow>,
    next_tool_id: u64,
}

impl App {
    pub fn new(
        mut backend: Box<dyn EnvBackend>,
        start_dir: Option<PathBuf>,
        settings: Settings,
    ) -> App {
        // Register the client command the engine calls with window ops.
        let handler = "env-vv-handler";
        backend.register_window_handler(handler);
        let _ = backend.evaluate(
            "add-virtual-window-handler",
            None,
            vec![Json::Str(handler.to_string())],
        );
        backend.set_real_time(settings.real_time);
        // Record the trace from the start so the History tab has data.
        let _ = backend.evaluate(
            "record-history",
            None,
            vec![Json::Str("trace-history".to_string())],
        );
        let models = backend.models();
        let selected_model = backend.current_model().or_else(|| models.last().cloned());
        let mut app = App {
            backend,
            start_dir,
            trace: String::new(),
            command: String::new(),
            result: String::new(),
            run_secs: 10.0,
            settings,
            models,
            selected_model,
            running: false,
            show_options: false,
            error: None,
            refresh_pending: true,
            windows: Vec::new(),
            tools: Vec::new(),
            next_tool_id: 1,
        };
        if app.settings.show_copyrights {
            app.open_tool(ToolKind::About);
        }
        app
    }

    /// Open (or raise) a floating tool window for the selected model.
    fn open_tool(&mut self, kind: ToolKind) {
        let model = if kind.model_scoped() {
            self.selected_model.clone()
        } else {
            None
        };
        if let Some(tool) = self
            .tools
            .iter_mut()
            .find(|t| t.kind == kind && t.model == model)
        {
            tool.open = true;
            tool.refresh_pending = true;
            return;
        }
        let id = self.next_tool_id;
        self.next_tool_id += 1;
        self.tools.push(ToolWindow::new(id, kind, model));
    }
    /// Refresh one tool's data using the tool's own model.
    fn refresh_tool(&mut self, tool: &mut ToolWindow) {
        let model = tool.model.clone();
        match tool.kind {
            ToolKind::Declarative => {
                let filters = self.list_for(model.clone(), "dm-slot-filters", Vec::new());
                if !filters.is_empty() {
                    tool.dm_filters = filters;
                }
                if !tool.dm_filters.iter().any(|f| *f == tool.dm_filter) {
                    tool.dm_filter = tool.dm_filters.first().cloned().unwrap_or_default();
                }
                let filter = tool.dm_filter.clone();
                tool.dm_chunks = self.list_for(model, "filter-dm-chunks", vec![Json::Str(filter)]);
                if self.settings.sort_lists {
                    tool.dm_chunks.sort();
                }
                tool.dm_sel = None;
                tool.dm_detail.clear();
            }
            ToolKind::ChunkTree => {
                tool.tree_chunks = self.list_for(model, "chunks", Vec::new());
                if self.settings.sort_lists {
                    tool.tree_chunks.sort();
                }
                tool.tree_sel = None;
                tool.tree_text.clear();
            }
            ToolKind::Buffers | ToolKind::BufferStatus => {
                tool.buffers = self.list_for(model, "model-buffers", Vec::new());
                if self.settings.sort_lists {
                    tool.buffers.sort();
                }
                tool.buf_sel = None;
                tool.buf_detail.clear();
            }
            ToolKind::Procedural => {
                tool.prods = self.list_for(model.clone(), "all-productions", Vec::new());
                if self.settings.sort_lists {
                    tool.prods.sort();
                }
                let matched = self
                    .eval_for(model, "no-output", vec![Json::Str("whynot".into())])
                    .and_then(|v| v.into_iter().next());
                let matched: Vec<String> = match matched {
                    Some(Json::Arr(a)) => a
                        .iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect(),
                    _ => Vec::new(),
                };
                tool.prod_matched = tool
                    .prods
                    .iter()
                    .map(|p| matched.iter().any(|m| m.eq_ignore_ascii_case(p)))
                    .collect();
                tool.prod_sel = None;
                tool.prod_detail.clear();
            }
            ToolKind::Parameters => {
                tool.modules = self.list_for(model, "modules-with-parameters", Vec::new());
                if self.settings.sort_lists {
                    tool.modules.sort();
                }
                tool.mod_sel = None;
                tool.params.clear();
                tool.param_sel = None;
                tool.param_detail.clear();
            }
            ToolKind::Visicon => {
                tool.text = self
                    .eval_for(model, "printed-visicon", Vec::new())
                    .and_then(|v| v.into_iter().next())
                    .and_then(|x| x.as_str().map(str::to_string))
                    .unwrap_or_default();
            }
            ToolKind::Audicon => {
                tool.text = self
                    .eval_for(model, "printed-audicon", Vec::new())
                    .and_then(|v| v.into_iter().next())
                    .and_then(|x| x.as_str().map(str::to_string))
                    .unwrap_or_default();
            }
            ToolKind::TextTrace => {
                tool.history_text = self.fetch_trace_history();
            }
            ToolKind::ProductionGraph => {
                let (n, e) = self.fetch_graph(model);
                tool.graph_nodes = n;
                tool.graph_edges = e;
            }
            ToolKind::EventQueue => {
                tool.text = self
                    .eval("mp-queue-text", vec![Json::Int(1)])
                    .map_or_else(String::new, |v| {
                        v.first().and_then(|x| x.as_str()).unwrap_or("").to_string()
                    });
            }
            ToolKind::Connections => match self.backend.connections() {
                Ok(c) => tool.connections = c,
                Err(e) => self.error = Some(format!("list-connections: {e}")),
            },
            ToolKind::About => {
                tool.text = self
                    .eval("act-r-version", Vec::new())
                    .and_then(|v| v.first().and_then(|x| x.as_str()).map(str::to_string))
                    .unwrap_or_else(|| "ACT-R".to_string());
            }
            ToolKind::Stepper => {}
        }
    }
    /// Draw the floating tools. Each is taken out of the list so it can borrow
    /// `self` while it renders, then put back unless the user closed it.
    fn render_tools(&mut self, ctx: &egui::Context) {
        let mut i = 0;
        while i < self.tools.len() {
            let mut tool = self.tools.remove(i);
            if tool.refresh_pending {
                self.refresh_tool(&mut tool);
                tool.refresh_pending = false;
            }
            let mut open = tool.open;
            let mut win = egui::Window::new(format!("{}##{}", tool.title, tool.id)).open(&mut open);
            if let Some([x, y, w, h]) = self.settings.tool_geometry.get(&tool.title).copied() {
                win = win.default_pos([x, y]).default_size([w, h]);
            } else {
                win = win.default_width(520.0).default_height(360.0);
            }
            let shown = win.show(ctx, |ui| self.tool_body(ui, &mut tool));
            if let Some(inner) = shown {
                let r = inner.response.rect;
                self.settings.tool_geometry.insert(
                    tool.title.clone(),
                    [r.min.x, r.min.y, r.width(), r.height()],
                );
            }
            tool.open = open;
            if open {
                self.tools.insert(i, tool);
                i += 1;
            }
        }
    }

    fn tool_body(&mut self, ui: &mut egui::Ui, tool: &mut ToolWindow) {
        match tool.kind {
            ToolKind::Connections => {
                if ui.button("Get Status").clicked() {
                    tool.refresh_pending = true;
                }
                ui.separator();
                egui::Grid::new(("connections", tool.id))
                    .num_columns(4)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Name");
                        ui.strong("Sent");
                        ui.strong("Received");
                        ui.strong("Commands");
                        ui.end_row();
                        for c in &tool.connections {
                            ui.label(&c.name);
                            ui.label(c.sent.to_string());
                            ui.label(c.received.to_string());
                            ui.label(c.commands.join(" "));
                            ui.end_row();
                        }
                    });
            }
            ToolKind::EventQueue | ToolKind::About => {
                if ui.button("Refresh").clicked() {
                    tool.refresh_pending = true;
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt(("tool-scroll", tool.id))
                    .show(ui, |ui| {
                        ui.monospace(&tool.text);
                    });
            }
            ToolKind::Declarative => self.dm_panel(tool, ui),
            ToolKind::ChunkTree => self.chunk_tree_panel(tool, ui),
            ToolKind::Buffers => self.buffer_panel(tool, ui),
            ToolKind::BufferStatus => self.buffer_status_panel(tool, ui),
            ToolKind::Procedural => self.proc_panel(tool, ui),
            ToolKind::Parameters => self.param_panel(tool, ui),
            ToolKind::Visicon | ToolKind::Audicon => {
                if ui.button("Refresh").clicked() {
                    tool.refresh_pending = true;
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt(("tool-text", tool.id))
                    .show(ui, |ui| {
                        ui.monospace(&tool.text);
                    });
            }
            ToolKind::TextTrace => self.history_panel(tool, ui),
            ToolKind::ProductionGraph => self.graph_panel(tool, ui),
            ToolKind::Stepper => {
                ui.label("Stepper tools arrive in M5.");
            }
        }
    }

    /// Fetch the production transition graph from the backend.
    fn fetch_graph(&mut self, model: Option<String>) -> (Vec<String>, Vec<(String, String, u32)>) {
        let v = match self.eval_for(model, "production-graph", Vec::new()) {
            Some(v) => v,
            None => return (Vec::new(), Vec::new()),
        };
        let arr = match v.into_iter().next() {
            Some(Json::Arr(a)) => a,
            _ => return (Vec::new(), Vec::new()),
        };
        let nodes = match arr.first() {
            Some(Json::Arr(a)) => a
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        };
        let edges = match arr.get(1) {
            Some(Json::Arr(a)) => a
                .iter()
                .filter_map(|x| match x {
                    Json::Arr(f) => Some((
                        f.first()?.as_str()?.to_string(),
                        f.get(1)?.as_str()?.to_string(),
                        f.get(2)?.as_f64()? as u32,
                    )),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        (nodes, edges)
    }

    fn graph_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        if tool.graph_nodes.is_empty() {
            ui.label("No production-history graph yet. Record \"production-history\", run, then Refresh.");
            return;
        }
        // Relax edge constraints to assign layers (cycles stabilize).
        let mut layer: HashMap<&str, usize> = tool
            .graph_nodes
            .iter()
            .map(|n| (n.as_str(), 0usize))
            .collect();
        for _ in 0..tool.graph_nodes.len().min(20) {
            let mut changed = false;
            for (a, b, _) in &tool.graph_edges {
                let la = layer[a.as_str()];
                let lb = layer[b.as_str()];
                if lb <= la {
                    layer.insert(b.as_str(), (la + 1).min(tool.graph_nodes.len()));
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let max_layer = *layer.values().max().unwrap_or(&0);
        let mut by_layer: Vec<Vec<&String>> = vec![Vec::new(); max_layer + 1];
        for n in &tool.graph_nodes {
            by_layer[layer[n.as_str()]].push(n);
        }
        let box_w = 130.0f32;
        let box_h = 30.0f32;
        let h_gap = 20.0f32;
        let v_gap = 70.0f32;
        let width =
            by_layer.iter().map(|l| l.len()).max().unwrap_or(1) as f32 * (box_w + h_gap) + 40.0;
        let height = (max_layer + 1) as f32 * (box_h + v_gap) + 40.0;
        let mut pos: HashMap<&str, (f32, f32)> = HashMap::new();
        for (li, layer_nodes) in by_layer.iter().enumerate() {
            for (xi, n) in layer_nodes.iter().enumerate() {
                pos.insert(
                    n.as_str(),
                    (
                        20.0 + xi as f32 * (box_w + h_gap),
                        20.0 + li as f32 * (box_h + v_gap),
                    ),
                );
            }
        }
        egui::ScrollArea::both()
            .id_salt(("prod-graph", tool.id))
            .show(ui, |ui| {
                let (rect, _resp) =
                    ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                let p = ui.painter_at(rect);
                for (a, b, c) in &tool.graph_edges {
                    if let (Some(&(ax, ay)), Some(&(bx, by))) =
                        (pos.get(a.as_str()), pos.get(b.as_str()))
                    {
                        let from = rect.min + egui::vec2(ax + box_w / 2.0, ay + box_h);
                        let to = rect.min + egui::vec2(bx + box_w / 2.0, by);
                        let w = 1.0 + (*c as f32).min(6.0);
                        p.line_segment([from, to], egui::Stroke::new(w, egui::Color32::GRAY));
                    }
                }
                for n in &tool.graph_nodes {
                    if let Some(&(x, y)) = pos.get(n.as_str()) {
                        let r = egui::Rect::from_min_size(
                            rect.min + egui::vec2(x, y),
                            egui::vec2(box_w, box_h),
                        );
                        p.rect_filled(
                            r,
                            egui::CornerRadius::ZERO,
                            egui::Color32::from_rgb(230, 230, 230),
                        );
                        p.rect_stroke(
                            r,
                            egui::CornerRadius::ZERO,
                            egui::Stroke::new(1.0, egui::Color32::BLACK),
                            egui::StrokeKind::Inside,
                        );
                        p.text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            n,
                            egui::FontId::proportional(12.0),
                            egui::Color32::BLACK,
                        );
                    }
                }
            });
    }
    /// Fetch the recorded `trace-history` via the incremental protocol.
    fn fetch_trace_history(&mut self) -> String {
        let id = match self.eval(
            "start-incremental-history-data",
            vec![Json::Str("trace-history".into()), Json::Int(16000)],
        ) {
            Some(v) => v.into_iter().next(),
            None => None,
        };
        let Some(Json::Int(id)) = id else {
            return String::new();
        };
        let mut out = String::new();
        loop {
            let chunk = match self.eval("get-incremental-history-data", vec![Json::Int(id)]) {
                Some(v) => v
                    .into_iter()
                    .next()
                    .and_then(|x| x.as_str().map(str::to_string)),
                None => None,
            };
            let Some(c) = chunk else { break };
            if c.is_empty() {
                break;
            }
            if let Ok(Json::Arr(recs)) = crate::json::parse(&c) {
                for r in recs {
                    if let Json::Arr(fields) = r {
                        if let Some(Json::Str(line)) = fields.get(3) {
                            out.push_str(line);
                            if !line.ends_with('\n') {
                                out.push('\n');
                            }
                        }
                    }
                }
            }
        }
        out
    }

    fn history_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                tool.refresh_pending = true;
            }
            if ui.button("Save…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Text", &["txt", "log"])
                    .save_file()
                {
                    self.eval(
                        "save-history-data",
                        vec![
                            Json::Str("trace-history".into()),
                            Json::Str(path.to_string_lossy().into_owned()),
                            Json::Str("Data saved from the ACT-R Environment".into()),
                        ],
                    );
                }
            }
        });
        egui::ScrollArea::vertical()
            .id_salt(("history", tool.id))
            .show(ui, |ui| {
                ui.monospace(&tool.history_text);
            });
    }
    /// Apply one display op from the handler to the window list.
    fn apply_window_op(&mut self, op: Vec<Json>) {
        let s = |i: usize| op.get(i).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let n = |i: usize| op.get(i).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
        match op.first().and_then(|x| x.as_str()).unwrap_or("") {
            "open" => {
                let title = s(1);
                let win = VisWin {
                    title: title.clone(),
                    x: n(2),
                    y: n(3),
                    w: n(4),
                    h: n(5),
                    ..Default::default()
                };
                self.windows.retain(|w| w.title != title);
                self.windows.push(win);
            }
            "close" => {
                let title = s(1);
                self.windows.retain(|w| w.title != title);
            }
            "select" => {}
            "text" => {
                let (title, id) = (s(1), s(2));
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    upsert(
                        &mut w.items,
                        WinItem::Text {
                            id,
                            x: n(3),
                            y: n(4),
                            text: s(5),
                            color: color(&s(6)),
                            size: n(7).max(6.0),
                        },
                    );
                }
            }
            "button" => {
                let (title, id) = (s(1), s(2));
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    upsert(
                        &mut w.items,
                        WinItem::Button {
                            id,
                            x: n(3),
                            y: n(4),
                            w: n(5),
                            h: n(6),
                            text: s(7),
                            color: color(&s(8)),
                        },
                    );
                }
            }
            "line" => {
                let (title, id) = (s(1), s(2));
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    upsert(
                        &mut w.items,
                        WinItem::Line {
                            id,
                            x1: n(3),
                            y1: n(4),
                            x2: n(5),
                            y2: n(6),
                            color: color(&s(7)),
                        },
                    );
                }
            }
            "remove" => {
                let (title, id) = (s(1), s(2));
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    w.items.retain(|it| it.id() != id);
                }
            }
            "attention" => {
                let title = s(1);
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    w.attention = Some((n(3), n(4), color(&s(5))));
                }
            }
            "clearattention" => {
                let title = s(1);
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    w.attention = None;
                }
            }
            "cursor" => {
                let title = s(1);
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    w.cursor = Some((n(3), n(4), color(&s(5))));
                }
            }
            "clearcursor" => {
                let title = s(1);
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    w.cursor = None;
                }
            }
            _ => {}
        }
    }

    /// Draw each visible window and forward clicks back to the engine.
    fn render_windows(&mut self, ctx: &egui::Context) {
        let wins = self.windows.clone();
        let mut click: Option<(String, f64, f64)> = None;
        for w in &wins {
            egui::Window::new(&w.title)
                .default_pos([w.x, w.y])
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(w.w, w.h), egui::Sense::click());
                    let p = ui.painter_at(rect);
                    p.rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::WHITE);
                    for it in &w.items {
                        match it {
                            WinItem::Line {
                                x1,
                                y1,
                                x2,
                                y2,
                                color,
                                ..
                            } => {
                                p.line_segment(
                                    [
                                        rect.min + egui::vec2(*x1, *y1),
                                        rect.min + egui::vec2(*x2, *y2),
                                    ],
                                    egui::Stroke::new(2.0, *color),
                                );
                            }
                            WinItem::Text {
                                x,
                                y,
                                text,
                                color,
                                size,
                                ..
                            } => {
                                p.text(
                                    rect.min + egui::vec2(*x, *y),
                                    egui::Align2::LEFT_TOP,
                                    text,
                                    egui::FontId::monospace(*size),
                                    *color,
                                );
                            }
                            WinItem::Button {
                                x,
                                y,
                                w: bw,
                                h: bh,
                                text,
                                color,
                                ..
                            } => {
                                let r = egui::Rect::from_min_size(
                                    rect.min + egui::vec2(*x, *y),
                                    egui::vec2(*bw, *bh),
                                );
                                p.rect_filled(r, egui::CornerRadius::ZERO, *color);
                                p.rect_stroke(
                                    r,
                                    egui::CornerRadius::ZERO,
                                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                                    egui::StrokeKind::Inside,
                                );
                                p.text(
                                    r.center(),
                                    egui::Align2::CENTER_CENTER,
                                    text,
                                    egui::FontId::proportional(14.0),
                                    egui::Color32::BLACK,
                                );
                            }
                        }
                    }
                    if let Some((x, y, c)) = w.attention {
                        p.circle_stroke(
                            rect.min + egui::vec2(x, y),
                            10.0,
                            egui::Stroke::new(4.0, c),
                        );
                    }
                    if let Some((x, y, c)) = w.cursor {
                        p.text(
                            rect.min + egui::vec2(x, y),
                            egui::Align2::LEFT_TOP,
                            "\u{279A}",
                            egui::FontId::proportional(16.0),
                            c,
                        );
                    }
                    if resp.clicked() {
                        if let Some(q) = resp.interact_pointer_pos() {
                            click = Some((
                                w.title.clone(),
                                (q.x - rect.min.x) as f64,
                                (q.y - rect.min.y) as f64,
                            ));
                        }
                    }
                });
        }
        if let Some((title, x, y)) = click {
            self.eval(
                "visible-virtual-window-mouse-click",
                vec![Json::Str(title), Json::Float(x), Json::Float(y)],
            );
        }
    }

    /// Drain backend events: trace, model/run signals, and window ops.
    fn drain_events(&mut self) {
        for event in self.backend.take_events() {
            match event {
                BackendEvent::Trace { line, .. } => {
                    self.trace.push_str(&line);
                    if !line.ends_with('\n') {
                        self.trace.push('\n');
                    }
                }
                BackendEvent::ModelCreated(name) => {
                    if !self.models.iter().any(|m| m == &name) {
                        self.models.push(name.clone());
                    }
                    // A newly created model becomes the selected one, as in
                    // the Tcl model menu.
                    self.selected_model = Some(name);
                }
                BackendEvent::ModelDeleted(name) => {
                    self.models.retain(|m| m != &name);
                    if self.selected_model.as_deref() == Some(name.as_str()) {
                        self.selected_model = self.models.first().cloned();
                    }
                    if self.settings.kill_model_windows {
                        self.tools
                            .retain(|t| t.model.as_deref() != Some(name.as_str()));
                    }
                }
                BackendEvent::RunStarted { .. } => self.running = true,
                BackendEvent::RunStopped { .. } => {
                    self.running = false;
                    self.refresh_pending = true;
                }
                BackendEvent::WindowOp(op) => self.apply_window_op(op),
                BackendEvent::Status(_) => {}
                BackendEvent::StepperPause { .. } | BackendEvent::StepperUpdate { .. } => {}
            }
        }
    }

    /// Evaluate a global command, recording any error for the status bar.
    fn eval(&mut self, name: &str, args: Vec<Json>) -> Option<Vec<Json>> {
        self.eval_for(None, name, args)
    }

    fn eval_for(
        &mut self,
        model: Option<String>,
        name: &str,
        args: Vec<Json>,
    ) -> Option<Vec<Json>> {
        match self.backend.evaluate(name, model.as_deref(), args) {
            Ok(v) => {
                self.error = None;
                self.drain_events();
                Some(v)
            }
            Err(e) => {
                let message = format!("{name}: {e}");
                self.error = Some(message.clone());
                self.log_error(&message);
                self.drain_events();
                None
            }
        }
    }

    /// Append an error to `~/.actr-env.log` (Tcl writes `error.log`).
    fn log_error(&self, message: &str) {
        let path = settings::settings_path().with_file_name(".actr-env.log");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{message}");
        }
    }

    /// Evaluate a model-scoped command and take the first result as a string.
    fn text_for(&mut self, model: Option<String>, name: &str, args: Vec<Json>) -> String {
        self.eval_for(model, name, args)
            .and_then(|v| v.first().and_then(|x| x.as_str()).map(str::to_string))
            .unwrap_or_default()
    }

    /// Evaluate a model-scoped command whose first result is a list of names.
    fn list_for(&mut self, model: Option<String>, name: &str, args: Vec<Json>) -> Vec<String> {
        match self
            .eval_for(model, name, args)
            .and_then(|v| v.into_iter().next())
        {
            Some(Json::Arr(a)) => a
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Start an interactive (chunked) run; `poll_run` advances it each frame.
    fn start_run(&mut self, full: bool) {
        self.drain_events();
        if self.backend.begin_run(self.run_secs, full) {
            self.running = true;
        } else {
            self.refresh_pending = true;
        }
        self.drain_events();
    }

    fn load_dialog(&mut self, command: &str) {
        let mut dlg = rfd::FileDialog::new()
            .add_filter("ACT-R model", &["lisp", "cl"])
            .add_filter("All files", &["*"]);
        if let Some(dir) = &self.start_dir {
            dlg = dlg.set_directory(dir);
        }
        if let Some(path) = dlg.pick_file() {
            // A different model gets a fresh trace pane.
            self.trace.clear();
            let mut args = vec![Json::Str(path.to_string_lossy().into_owned())];
            if command == "load-act-r-code" {
                args.push(Json::Bool(self.settings.use_smart_load));
            }
            self.eval(command, args);
            self.drain_events();
            self.refresh_pending = true;
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Controls");
        ui.separator();

        // Current Model menu (`06-ctrl-panel-show-loaded.tcl`).
        ui.horizontal(|ui| {
            ui.label("Current Model");
            let selected = self
                .selected_model
                .clone()
                .unwrap_or_else(|| "No Current Model".to_string());
            let mut chosen = None;
            egui::ComboBox::from_id_salt("current-model")
                .selected_text(selected.clone())
                .show_ui(ui, |ui| {
                    if self.models.is_empty() {
                        ui.label("No Current Model");
                    }
                    for name in &self.models {
                        if ui.selectable_label(selected == *name, name).clicked() {
                            chosen = Some(name.clone());
                        }
                    }
                });
            if let Some(name) = chosen {
                self.selected_model = Some(name.clone());
                self.backend.set_current_model(Some(name));
                self.refresh_pending = true;
            }
            if ui.small_button("Refresh").clicked() {
                self.models = self.backend.models();
                if self
                    .selected_model
                    .as_ref()
                    .map_or(true, |m| !self.models.contains(m))
                {
                    self.selected_model = self.models.last().cloned();
                }
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Load codeâ€¦").clicked() {
                self.load_dialog("load-act-r-code");
            }
            if ui.button("Load modelâ€¦").clicked() {
                self.load_dialog("load-act-r-model");
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Reload").clicked() {
                self.trace.clear();
                self.eval("reload", Vec::new());
                self.drain_events();
                self.refresh_pending = true;
            }
            if ui.button("Reset").clicked() {
                // A reset starts a fresh run context: clear the trace pane
                // (then show only the reset's own output).
                self.trace.clear();
                self.eval("reset", Vec::new());
                self.drain_events();
                self.refresh_pending = true;
            }
            if ui.button("Clear trace").clicked() {
                self.trace.clear();
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Run");
            ui.add(
                egui::DragValue::new(&mut self.run_secs)
                    .speed(0.5)
                    .range(0.0..=600.0),
            );
            ui.label("s");
        });
        if ui
            .checkbox(&mut self.settings.real_time, "real time")
            .changed()
        {
            self.backend.set_real_time(self.settings.real_time);
            settings::save(&self.settings);
        }
        if ui.button("Optionsâ€¦").clicked() {
            self.show_options = true;
        }
        ui.separator();
        ui.label("Current Data");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Declarative").clicked() {
                self.open_tool(ToolKind::Declarative);
            }
            if ui.button("Chunk Tree").clicked() {
                self.open_tool(ToolKind::ChunkTree);
            }
            if ui.button("Buffers").clicked() {
                self.open_tool(ToolKind::Buffers);
            }
            if ui.button("Buffer Status").clicked() {
                self.open_tool(ToolKind::BufferStatus);
            }
            if ui.button("Procedural").clicked() {
                self.open_tool(ToolKind::Procedural);
            }
            if ui.button("Parameters").clicked() {
                self.open_tool(ToolKind::Parameters);
            }
            if ui.button("Visicon").clicked() {
                self.open_tool(ToolKind::Visicon);
            }
            if ui.button("Audicon").clicked() {
                self.open_tool(ToolKind::Audicon);
            }
        });
        ui.separator();
        ui.label("Recordable Data");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Text Trace").clicked() {
                self.open_tool(ToolKind::TextTrace);
            }
            if ui.button("Production Graph").clicked() {
                self.open_tool(ToolKind::ProductionGraph);
            }
        });
        ui.separator();
        ui.label("Miscellaneous");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Stepper").clicked() {
                self.open_tool(ToolKind::Stepper);
            }
            if ui.button("Event Queue").clicked() {
                self.open_tool(ToolKind::EventQueue);
            }
            if ui.button("Connections").clicked() {
                self.open_tool(ToolKind::Connections);
            }
            if ui.button("About").clicked() {
                self.open_tool(ToolKind::About);
            }
        });
        let running = self.running || self.backend.is_running();
        ui.horizontal(|ui| {
            if running {
                ui.colored_label(egui::Color32::from_rgb(0, 128, 0), "Running");
            } else {
                ui.colored_label(egui::Color32::from_rgb(160, 0, 0), "Stopped");
            }
            if ui.add_enabled(!running, egui::Button::new("Run")).clicked() {
                self.start_run(false);
            }
            if ui
                .add_enabled(!running, egui::Button::new("Run full time"))
                .clicked()
            {
                self.start_run(true);
            }
            if ui
                .add_enabled(!running, egui::Button::new("Step"))
                .clicked()
            {
                self.eval("step-once", Vec::new());
                self.drain_events();
                self.refresh_pending = true;
            }
            if ui.add_enabled(running, egui::Button::new("Stop")).clicked() {
                self.backend.stop_run();
                self.drain_events();
            }
        });
        ui.separator();
        ui.label("Evaluate (debug)");
        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.command)
                    .desired_width(150.0)
                    .hint_text("(sgp)"),
            );
            let go = ui.button("Go").clicked()
                || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if go && !self.command.trim().is_empty() {
                let cmd = self.command.trim().to_string();
                self.result = match self.eval(&cmd, Vec::new()) {
                    Some(v) => result_text(&v),
                    None => String::new(),
                };
                self.drain_events();
            }
        });
        if !self.result.is_empty() {
            ui.label(format!("â†’ {}", self.result));
        }
    }

    fn trace_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.trace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(30)
                        .code_editor(),
                );
            });
    }

    fn dm_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        let filters = tool.dm_filters.clone();
        let mut chosen: Option<String> = None;
        ui.horizontal(|ui| {
            ui.label("Filter:");
            egui::ComboBox::from_id_salt(("dm-filter", tool.id))
                .selected_text(tool.dm_filter.clone())
                .show_ui(ui, |ui| {
                    for f in &filters {
                        if ui.selectable_label(*f == tool.dm_filter, f).clicked() {
                            chosen = Some(f.clone());
                        }
                    }
                });
        });
        if let Some(f) = chosen {
            tool.dm_filter = f;
            let filter = tool.dm_filter.clone();
            tool.dm_chunks = self.list_for(
                tool.model.clone(),
                "filter-dm-chunks",
                vec![Json::Str(filter)],
            );
            if self.settings.sort_lists {
                tool.dm_chunks.sort();
            }
            tool.dm_sel = None;
            tool.dm_detail.clear();
        }
        ui.separator();

        let chunks = tool.dm_chunks.clone();
        let mut sel = tool.dm_sel;
        let detail = tool.dm_detail.clone();
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt(("dm-list", tool.id))
                .show(&mut cols[0], |ui| {
                    for (i, name) in chunks.iter().enumerate() {
                        if ui.selectable_label(sel == Some(i), name).clicked() {
                            sel = Some(i);
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt(("dm-detail", tool.id))
                .show(&mut cols[1], |ui| {
                    ui.monospace(detail);
                });
        });
        if sel != tool.dm_sel {
            tool.dm_sel = sel;
            tool.dm_detail = match sel {
                Some(i) => self.text_for(
                    tool.model.clone(),
                    "dm-chunk-details",
                    vec![Json::Str(chunks[i].clone())],
                ),
                None => String::new(),
            };
        }
        if ui.button("Why not?").clicked() {
            if let Some(i) = tool.dm_sel {
                tool.dm_detail = self.text_for(
                    tool.model.clone(),
                    "dm-whynot-text",
                    vec![Json::Str(chunks[i].clone())],
                );
            }
        }
    }

    fn chunk_tree_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        let chunks = tool.tree_chunks.clone();
        let mut sel = tool.tree_sel;
        let tree = tool.tree_text.clone();
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt(("tree-list", tool.id))
                .show(&mut cols[0], |ui| {
                    for (i, name) in chunks.iter().enumerate() {
                        if ui.selectable_label(sel == Some(i), name).clicked() {
                            sel = Some(i);
                        }
                    }
                });
            egui::ScrollArea::both()
                .id_salt(("tree-detail", tool.id))
                .show(&mut cols[1], |ui| {
                    ui.monospace(tree);
                });
        });
        if sel != tool.tree_sel {
            tool.tree_sel = sel;
            tool.tree_text = match sel {
                Some(i) => {
                    let mut path = Vec::new();
                    self.chunk_tree_text(&tool.model, &chunks[i], 0, &mut path)
                }
                None => String::new(),
            };
        }
    }

    fn chunk_tree_text(
        &mut self,
        model: &Option<String>,
        chunk: &str,
        depth: usize,
        path: &mut Vec<String>,
    ) -> String {
        let indent = "  ".repeat(depth);
        let mut out = format!("{indent}{chunk}\n");
        if path.iter().any(|p| p.eq_ignore_ascii_case(chunk)) {
            return out;
        }
        path.push(chunk.to_string());
        let slots = self.list_for(
            model.clone(),
            "chunk-filled-slots-list",
            vec![Json::Str(chunk.to_string()), Json::Bool(true)],
        );
        for slot in slots {
            let value = self.text_for(
                model.clone(),
                "chunk-slot-value",
                vec![Json::Str(chunk.to_string()), Json::Str(slot.clone())],
            );
            out.push_str(&format!("{indent}  {slot} = {value}\n"));
            let is_chunk = matches!(
                self.eval_for(model.clone(), "chunk-p", vec![Json::Str(value.clone())])
                    .and_then(|v| v.into_iter().next()),
                Some(Json::Bool(true))
            );
            if is_chunk {
                out.push_str(&self.chunk_tree_text(model, &value, depth + 2, path));
            }
        }
        path.pop();
        out
    }
    fn buffer_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(tool.buf_mode == BufferMode::Contents, "Contents")
                .clicked()
            {
                tool.buf_mode = BufferMode::Contents;
                tool.buf_sel = None;
                tool.buf_detail.clear();
            }
            if ui
                .selectable_label(tool.buf_mode == BufferMode::Status, "Status")
                .clicked()
            {
                tool.buf_mode = BufferMode::Status;
                tool.buf_sel = None;
                tool.buf_detail.clear();
            }
        });
        ui.separator();
        let buffers = tool.buffers.clone();
        let mut sel = tool.buf_sel;
        let detail = tool.buf_detail.clone();
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt(("buf-list", tool.id))
                .show(&mut cols[0], |ui| {
                    for (i, name) in buffers.iter().enumerate() {
                        if ui.selectable_label(sel == Some(i), name).clicked() {
                            sel = Some(i);
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt(("buf-detail", tool.id))
                .show(&mut cols[1], |ui| {
                    ui.monospace(detail);
                });
        });
        if sel != tool.buf_sel {
            tool.buf_sel = sel;
            tool.buf_detail = match sel {
                Some(i) => self.buffer_detail(tool.model.clone(), &buffers[i], tool.buf_mode),
                None => String::new(),
            };
        }
    }

    fn buffer_status_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        let buffers = tool.buffers.clone();
        let mut sel = tool.buf_sel;
        let detail = tool.buf_detail.clone();
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt(("bufstatus-list", tool.id))
                .show(&mut cols[0], |ui| {
                    for (i, name) in buffers.iter().enumerate() {
                        if ui.selectable_label(sel == Some(i), name).clicked() {
                            sel = Some(i);
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt(("bufstatus-detail", tool.id))
                .show(&mut cols[1], |ui| {
                    ui.monospace(detail);
                });
        });
        if sel != tool.buf_sel {
            tool.buf_sel = sel;
            tool.buf_detail = match sel {
                Some(i) => self.text_for(
                    tool.model.clone(),
                    "printed-buffer-status",
                    vec![Json::Str(buffers[i].clone())],
                ),
                None => String::new(),
            };
        }
    }

    fn buffer_detail(&mut self, model: Option<String>, buffer: &str, mode: BufferMode) -> String {
        if mode == BufferMode::Status {
            return self.text_for(
                model,
                "printed-buffer-status",
                vec![Json::Str(buffer.to_string())],
            );
        }
        let chunk = self.text_for(
            model.clone(),
            "buffer-read",
            vec![Json::Str(buffer.to_string())],
        );
        if chunk.is_empty() {
            let multi = self.text_for(
                model.clone(),
                "multi-buffer-p",
                vec![Json::Str(buffer.to_string())],
            );
            if !multi.is_empty() {
                let chunks = self.list_for(
                    model.clone(),
                    "get-m-buffer-chunks",
                    vec![Json::Str(buffer.to_string())],
                );
                let mut out = String::new();
                for c in chunks {
                    out.push_str(&self.text_for(
                        model.clone(),
                        "printed-chunk",
                        vec![Json::Str(c)],
                    ));
                    out.push('\n');
                }
                return out;
            }
            return "Buffer is empty".to_string();
        }
        self.text_for(
            model,
            "printed-buffer-chunk",
            vec![Json::Str(buffer.to_string())],
        )
    }

    fn proc_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        let prods = tool.prods.clone();
        let matched = tool.prod_matched.clone();
        let mut sel = tool.prod_sel;
        let detail = tool.prod_detail.clone();
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt(("prod-list", tool.id))
                .show(&mut cols[0], |ui| {
                    for (i, name) in prods.iter().enumerate() {
                        let yes = matched.get(i).copied().unwrap_or(false);
                        ui.horizontal(|ui| {
                            let (text, color) = if yes {
                                ("Yes", egui::Color32::GREEN)
                            } else {
                                ("No", egui::Color32::RED)
                            };
                            ui.colored_label(color, text);
                            if ui.selectable_label(sel == Some(i), name).clicked() {
                                sel = Some(i);
                            }
                        });
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt(("prod-detail", tool.id))
                .show(&mut cols[1], |ui| {
                    ui.monospace(detail);
                });
        });
        if sel != tool.prod_sel {
            tool.prod_sel = sel;
            tool.prod_detail = match sel {
                Some(i) => self.text_for(
                    tool.model.clone(),
                    "production-details",
                    vec![Json::Str(prods[i].clone())],
                ),
                None => String::new(),
            };
        }
        if ui.button("Why not?").clicked() {
            if let Some(i) = tool.prod_sel {
                tool.prod_detail = self.text_for(
                    tool.model.clone(),
                    "whynot-text",
                    vec![Json::Str(prods[i].clone())],
                );
            }
        }
    }

    fn param_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        let modules = tool.modules.clone();
        let params = tool.params.clone();
        let mut mod_sel = tool.mod_sel;
        let mut param_sel = tool.param_sel;
        let detail = tool.param_detail.clone();
        ui.columns(3, |cols| {
            egui::ScrollArea::vertical()
                .id_salt(("mod-list", tool.id))
                .show(&mut cols[0], |ui| {
                    for (i, name) in modules.iter().enumerate() {
                        if ui.selectable_label(mod_sel == Some(i), name).clicked() {
                            mod_sel = Some(i);
                            param_sel = None;
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt(("param-list", tool.id))
                .show(&mut cols[1], |ui| {
                    for (i, name) in params.iter().enumerate() {
                        if ui.selectable_label(param_sel == Some(i), name).clicked() {
                            param_sel = Some(i);
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt(("param-detail", tool.id))
                .show(&mut cols[2], |ui| {
                    ui.monospace(detail);
                });
        });
        if mod_sel != tool.mod_sel {
            tool.mod_sel = mod_sel;
            tool.param_sel = None;
            tool.param_detail.clear();
            tool.params = match mod_sel {
                Some(i) => self.list_for(
                    tool.model.clone(),
                    "modules-parameters",
                    vec![Json::Str(modules[i].clone())],
                ),
                None => Vec::new(),
            };
        } else if param_sel != tool.param_sel {
            tool.param_sel = param_sel;
            tool.param_detail = match param_sel {
                Some(i) => self.text_for(
                    tool.model.clone(),
                    "printed-parameter-details",
                    vec![Json::Str(params[i].clone())],
                ),
                None => String::new(),
            };
        }
    }
}

/// Replace an item with the same id, or append it.
fn upsert(items: &mut Vec<WinItem>, item: WinItem) {
    if let Some(slot) = items.iter_mut().find(|it| it.id() == item.id()) {
        *slot = item;
    } else {
        items.push(item);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_events();

        // Advance an interactive run one chunk per frame (keeps the UI live
        // and honours Stop).
        if self.backend.is_running() {
            match self.backend.poll_run() {
                Ok(true) => {
                    self.drain_events();
                    ui.ctx().request_repaint();
                }
                Ok(false) => {
                    self.drain_events();
                    self.refresh_pending = true;
                }
                Err(e) => {
                    self.error = Some(e);
                }
            }
        }

        let model = self
            .selected_model
            .clone()
            .or_else(|| self.backend.current_model())
            .unwrap_or_else(|| "(none)".to_string());
        let desc = self.backend.describe();

        egui::Panel::top("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("ACT-R Environment");
                ui.separator();
                ui.label(format!("model: {model}"));
                ui.separator();
                ui.label(desc);
                if let Some(e) = &self.error {
                    ui.separator();
                    ui.colored_label(egui::Color32::RED, e);
                }
            });
        });

        let controls = egui::Panel::left("controls")
            .resizable(true)
            .default_size(self.settings.controls_width)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.controls(ui));
            });
        self.settings.controls_width = controls.response.rect.width();

        egui::CentralPanel::default().show(ui, |ui| self.trace_panel(ui));

        if self.refresh_pending {
            for tool in &mut self.tools {
                tool.refresh_pending = true;
            }
            self.refresh_pending = false;
        }
        let ctx = ui.ctx().clone();
        // Remember the window size for the next launch.
        if let Some(r) = ctx.input(|i| i.viewport().inner_rect) {
            self.settings.window = Some((r.width(), r.height()));
        }
        if self.show_options {
            let mut open = true;
            egui::Window::new("Options")
                .open(&mut open)
                .show(&ctx, |ui| {
                    let mut changed = false;
                    changed |= ui
                        .checkbox(
                            &mut self.settings.update_when_stepped,
                            "update inspector windows automatically when stepping",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut self.settings.use_env_window,
                            "use an Environment window for visible experiment windows",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut self.settings.show_copyrights,
                            "show the ACT-R copyrights screen",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut self.settings.use_localhost,
                            "use localhost instead of 127.0.0.1",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut self.settings.kill_model_windows,
                            "close inspector windows for deleted models",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut self.settings.use_smart_load,
                            "compile an ACT-R file when loaded or reloaded",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut self.settings.save_before_reload,
                            "automatically save all open files when reload pressed",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut self.settings.save_backups,
                            "save a backup file when saving opened files",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut self.settings.edit_warn,
                            "show the editor limitations warning",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(&mut self.settings.sort_lists, "sort viewer lists")
                        .changed();
                    if ui
                        .checkbox(&mut self.settings.real_time, "real time")
                        .changed()
                    {
                        self.backend.set_real_time(self.settings.real_time);
                        changed = true;
                    }
                    if changed {
                        self.refresh_pending = true;
                    }
                    ui.separator();
                    ui.label(format!("settings: {}", settings::settings_path().display()));
                    if changed {
                        settings::save(&self.settings);
                    }
                });
            self.show_options = open;
        }
        self.render_tools(&ctx);
        self.render_windows(&ctx);
    }

    fn on_exit(&mut self) {
        settings::save(&self.settings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::InProc;

    fn test_app() -> App {
        let settings = Settings {
            show_copyrights: false,
            ..Default::default()
        };
        App::new(Box::new(InProc::new((100, 0))), None, settings)
    }

    fn load_count(app: &mut App) {
        let model = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("models")
            .join("tutorial")
            .join("unit1")
            .join("count.lisp")
            .to_string_lossy()
            .into_owned();
        app.eval("load-act-r-model", vec![Json::Str(model)]);
    }

    #[test]
    fn tool_windows_are_per_model() {
        let mut app = test_app();
        app.models = vec!["A".into(), "B".into()];
        app.selected_model = Some("A".into());
        app.open_tool(ToolKind::Declarative);
        app.selected_model = Some("B".into());
        app.open_tool(ToolKind::Declarative);
        assert_eq!(app.tools.len(), 2);
        assert_eq!(app.tools[0].title, "Declarative (A)");
        assert_eq!(app.tools[1].title, "Declarative (B)");
    }

    #[test]
    fn chunk_tree_renders_slot_values() {
        let mut app = test_app();
        load_count(&mut app);
        let model = Some("COUNT".to_string());
        let chunk = app
            .list_for(model.clone(), "chunks", Vec::new())
            .into_iter()
            .next()
            .expect("a DM chunk");
        let mut path = Vec::new();
        let text = app.chunk_tree_text(&model, &chunk, 0, &mut path);
        assert!(text.starts_with(&chunk), "tree: {text}");
    }

    #[test]
    fn buffer_detail_reads_the_goal_buffer() {
        let mut app = test_app();
        load_count(&mut app);
        let detail = app.buffer_detail(Some("COUNT".to_string()), "goal", BufferMode::Contents);
        assert!(!detail.is_empty(), "goal buffer detail is empty");
    }
    #[test]
    fn tool_windows_reuse_and_title_by_model() {
        let mut app = test_app();
        app.models = vec!["A".into(), "B".into()];
        app.selected_model = Some("A".into());

        app.open_tool(ToolKind::Declarative);
        assert_eq!(app.tools.len(), 1);
        assert_eq!(app.tools[0].title, "Declarative (A)");

        // Opening the same kind again raises the existing window.
        app.open_tool(ToolKind::Declarative);
        assert_eq!(app.tools.len(), 1);

        app.open_tool(ToolKind::EventQueue);
        assert_eq!(app.tools.len(), 2);
        assert_eq!(app.tools[1].title, "Event Queue");
    }

    #[test]
    fn load_events_update_the_model_menu() {
        let mut app = test_app();
        let model = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("models")
            .join("tutorial")
            .join("unit1")
            .join("count.lisp")
            .to_string_lossy()
            .into_owned();
        app.eval("load-act-r-model", vec![Json::Str(model)]);
        assert_eq!(app.models, vec!["COUNT".to_string()]);
        assert_eq!(app.selected_model.as_deref(), Some("COUNT"));
    }
}

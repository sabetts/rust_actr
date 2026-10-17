//! The egui application shell: control panel, trace pane, and the Phase 2
//! read-only inspectors (declarative, buffers, procedural).

use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui;

use crate::json::Json;

use super::backend::{result_text, EnvBackend};
use super::settings::{self, Settings};

/// The central-area views.
#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    Trace,
    Declarative,
    Buffers,
    Procedural,
    Parameters,
    History,
    Graph,
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

/// An ACT-R colour name → egui colour (`color->name`, `misc-utils.lisp`).
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
    /// Where the file dialog opens (the ACT-R root; `None` in client mode).
    start_dir: Option<PathBuf>,
    trace: String,
    command: String,
    result: String,
    /// `run` duration in seconds.
    run_secs: f64,
    /// Persistent GUI settings.
    settings: Settings,
    /// Whether the Options window is open.
    show_options: bool,
    /// Last error, shown in the status bar.
    error: Option<String>,

    // --- inspector state ---
    tab: Tab,
    last_tab: Tab,
    refresh_pending: bool,
    dm_filters: Vec<String>,
    dm_filter: String,
    dm_chunks: Vec<String>,
    dm_sel: Option<usize>,
    dm_detail: String,
    buffers: Vec<String>,
    buf_sel: Option<usize>,
    buf_detail: String,
    prods: Vec<String>,
    prod_sel: Option<usize>,
    prod_detail: String,
    modules: Vec<String>,
    mod_sel: Option<usize>,
    params: Vec<String>,
    param_sel: Option<usize>,
    param_detail: String,
    /// The visible experiment windows the engine is displaying.
    windows: Vec<VisWin>,
    /// The recorded `trace-history` text.
    history_text: String,
    /// The production transition graph (nodes and counted edges).
    graph_nodes: Vec<String>,
    graph_edges: Vec<(String, String, u32)>,
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
        App {
            backend,
            start_dir,
            trace: String::new(),
            command: String::new(),
            result: String::new(),
            run_secs: 10.0,
            settings,
            show_options: false,
            error: None,
            tab: Tab::Trace,
            last_tab: Tab::Trace,
            refresh_pending: true,
            dm_filters: vec!["_none_".to_string()],
            dm_filter: "_none_".to_string(),
            dm_chunks: Vec::new(),
            dm_sel: None,
            dm_detail: String::new(),
            buffers: Vec::new(),
            buf_sel: None,
            buf_detail: String::new(),
            prods: Vec::new(),
            prod_sel: None,
            prod_detail: String::new(),
            modules: Vec::new(),
            mod_sel: None,
            params: Vec::new(),
            param_sel: None,
            param_detail: String::new(),
            windows: Vec::new(),
            history_text: String::new(),
            graph_nodes: Vec::new(),
            graph_edges: Vec::new(),
        }
    }

    /// Fetch the production transition graph from the backend.
    fn fetch_graph(&mut self) -> (Vec<String>, Vec<(String, String, u32)>) {
        let v = match self.eval("production-graph", Vec::new()) {
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

    fn graph_panel(&mut self, ui: &mut egui::Ui) {
        if self.graph_nodes.is_empty() {
            ui.label("No production-history graph yet. Record \"production-history\", run, then Refresh.");
            return;
        }
        // Relax edge constraints to assign layers (cycles stabilize).
        let mut layer: HashMap<&str, usize> = self
            .graph_nodes
            .iter()
            .map(|n| (n.as_str(), 0usize))
            .collect();
        for _ in 0..self.graph_nodes.len().min(20) {
            let mut changed = false;
            for (a, b, _) in &self.graph_edges {
                let la = layer[a.as_str()];
                let lb = layer[b.as_str()];
                if lb <= la {
                    layer.insert(b.as_str(), (la + 1).min(self.graph_nodes.len()));
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let max_layer = *layer.values().max().unwrap_or(&0);
        let mut by_layer: Vec<Vec<&String>> = vec![Vec::new(); max_layer + 1];
        for n in &self.graph_nodes {
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
            .id_salt("prod-graph")
            .show(ui, |ui| {
                let (rect, _resp) =
                    ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                let p = ui.painter_at(rect);
                for (a, b, c) in &self.graph_edges {
                    if let (Some(&(ax, ay)), Some(&(bx, by))) =
                        (pos.get(a.as_str()), pos.get(b.as_str()))
                    {
                        let from = rect.min + egui::vec2(ax + box_w / 2.0, ay + box_h);
                        let to = rect.min + egui::vec2(bx + box_w / 2.0, by);
                        let w = 1.0 + (*c as f32).min(6.0);
                        p.line_segment([from, to], egui::Stroke::new(w, egui::Color32::GRAY));
                    }
                }
                for n in &self.graph_nodes {
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

    fn history_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                self.refresh_pending = true;
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
            .id_salt("history")
            .show(ui, |ui| {
                ui.monospace(&self.history_text);
            });
    }

    /// Apply the display ops the handler received to the window list.
    fn apply_window_ops(&mut self) {
        for op in self.backend.take_window_ops() {
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

    /// Drain any trace the engine produced into the pane.
    fn drain_trace(&mut self) {
        for line in self.backend.take_trace() {
            self.trace.push_str(&line);
            if !line.ends_with('\n') {
                self.trace.push('\n');
            }
        }
    }

    /// Evaluate a command, recording any error for the status bar.
    fn eval(&mut self, name: &str, args: Vec<Json>) -> Option<Vec<Json>> {
        match self.backend.evaluate(name, None, args) {
            Ok(v) => {
                self.error = None;
                Some(v)
            }
            Err(e) => {
                self.error = Some(format!("{name}: {e}"));
                self.drain_trace();
                None
            }
        }
    }

    /// Evaluate a command and take the first result as a string.
    fn text(&mut self, name: &str, args: Vec<Json>) -> String {
        self.eval(name, args)
            .and_then(|v| v.first().and_then(|x| x.as_str()).map(str::to_string))
            .unwrap_or_default()
    }

    /// Evaluate a command whose first result is a list of names.
    fn list(&mut self, name: &str, args: Vec<Json>) -> Vec<String> {
        match self.eval(name, args).and_then(|v| v.into_iter().next()) {
            Some(Json::Arr(a)) => a
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Start an interactive (chunked) run; `poll_run` advances it each frame.
    fn start_run(&mut self, full: bool) {
        self.drain_trace();
        if !self.backend.begin_run(self.run_secs, full) {
            self.refresh_pending = true;
        }
    }

    fn load_dialog(&mut self) {
        let mut dlg = rfd::FileDialog::new()
            .add_filter("ACT-R model", &["lisp", "cl"])
            .add_filter("All files", &["*"]);
        if let Some(dir) = &self.start_dir {
            dlg = dlg.set_directory(dir);
        }
        if let Some(path) = dlg.pick_file() {
            // A different model gets a fresh trace pane.
            self.trace.clear();
            self.eval(
                "load-act-r-model",
                vec![Json::Str(path.to_string_lossy().into_owned())],
            );
            self.drain_trace();
            self.refresh_pending = true;
        }
    }

    /// Re-query the active inspector's data.
    fn refresh_active(&mut self) {
        match self.tab {
            Tab::Trace => {}
            Tab::Declarative => {
                let filters = self.list("dm-slot-filters", Vec::new());
                if !filters.is_empty() {
                    self.dm_filters = filters;
                }
                if !self.dm_filters.iter().any(|f| *f == self.dm_filter) {
                    self.dm_filter = self.dm_filters.first().cloned().unwrap_or_default();
                }
                self.refresh_dm_list();
            }
            Tab::Buffers => {
                self.buffers = self.list("model-buffers", Vec::new());
                if self.settings.sort_lists {
                    self.buffers.sort();
                }
                self.buf_sel = None;
                self.buf_detail.clear();
            }
            Tab::Procedural => {
                self.prods = self.list("all-productions", Vec::new());
                if self.settings.sort_lists {
                    self.prods.sort();
                }
                self.prod_sel = None;
                self.prod_detail.clear();
            }
            Tab::Parameters => {
                self.modules = self.list("modules-with-parameters", Vec::new());
                if self.settings.sort_lists {
                    self.modules.sort();
                }
                self.mod_sel = None;
                self.params.clear();
                self.param_sel = None;
                self.param_detail.clear();
            }
            Tab::History => {
                self.history_text = self.fetch_trace_history();
            }
            Tab::Graph => {
                let (n, e) = self.fetch_graph();
                self.graph_nodes = n;
                self.graph_edges = e;
            }
        }
    }

    fn refresh_dm_list(&mut self) {
        let filter = self.dm_filter.clone();
        self.dm_chunks = self.list("filter-dm-chunks", vec![Json::Str(filter)]);
        if self.settings.sort_lists {
            self.dm_chunks.sort();
        }
        self.dm_sel = None;
        self.dm_detail.clear();
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Controls");
        ui.separator();
        if ui.button("Load model…").clicked() {
            self.load_dialog();
        }
        ui.horizontal(|ui| {
            if ui.button("Reset").clicked() {
                // A reset starts a fresh run context: clear the trace pane
                // (then show only the reset's own output).
                self.trace.clear();
                self.eval("reset", Vec::new());
                self.drain_trace();
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
        if ui.button("Options…").clicked() {
            self.show_options = true;
        }
        let running = self.backend.is_running();
        ui.horizontal(|ui| {
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
                self.drain_trace();
                self.refresh_pending = true;
            }
            if ui.add_enabled(running, egui::Button::new("Stop")).clicked() {
                self.backend.stop_run();
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
                self.drain_trace();
            }
        });
        if !self.result.is_empty() {
            ui.label(format!("→ {}", self.result));
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

    fn dm_panel(&mut self, ui: &mut egui::Ui) {
        // Filter dropdown.
        let filters = self.dm_filters.clone();
        let mut chosen: Option<String> = None;
        ui.horizontal(|ui| {
            ui.label("Filter:");
            egui::ComboBox::from_id_salt("dm-filter")
                .selected_text(self.dm_filter.clone())
                .show_ui(ui, |ui| {
                    for f in &filters {
                        if ui.selectable_label(*f == self.dm_filter, f).clicked() {
                            chosen = Some(f.clone());
                        }
                    }
                });
        });
        if let Some(f) = chosen {
            self.dm_filter = f;
            self.refresh_dm_list();
        }
        ui.separator();

        let chunks = self.dm_chunks.clone();
        let mut sel = self.dm_sel;
        let detail = self.dm_detail.clone();
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt("dm-list")
                .show(&mut cols[0], |ui| {
                    for (i, name) in chunks.iter().enumerate() {
                        if ui.selectable_label(sel == Some(i), name).clicked() {
                            sel = Some(i);
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("dm-detail")
                .show(&mut cols[1], |ui| {
                    ui.monospace(detail);
                });
        });
        if sel != self.dm_sel {
            self.dm_sel = sel;
            self.dm_detail = match sel {
                Some(i) => {
                    let name = chunks[i].clone();
                    self.text("dm-chunk-details", vec![Json::Str(name)])
                }
                None => String::new(),
            };
        }
    }

    fn buffer_panel(&mut self, ui: &mut egui::Ui) {
        let buffers = self.buffers.clone();
        let mut sel = self.buf_sel;
        let detail = self.buf_detail.clone();
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt("buf-list")
                .show(&mut cols[0], |ui| {
                    for (i, name) in buffers.iter().enumerate() {
                        if ui.selectable_label(sel == Some(i), name).clicked() {
                            sel = Some(i);
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("buf-detail")
                .show(&mut cols[1], |ui| {
                    ui.monospace(detail);
                });
        });
        if sel != self.buf_sel {
            self.buf_sel = sel;
            self.buf_detail = match sel {
                Some(i) => {
                    let name = buffers[i].clone();
                    let chunk = self.text("printed-buffer-chunk", vec![Json::Str(name.clone())]);
                    let status = self.text("printed-buffer-status", vec![Json::Str(name)]);
                    format!("{status}\n{chunk}")
                }
                None => String::new(),
            };
        }
    }

    fn proc_panel(&mut self, ui: &mut egui::Ui) {
        let prods = self.prods.clone();
        let mut sel = self.prod_sel;
        let detail = self.prod_detail.clone();
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt("prod-list")
                .show(&mut cols[0], |ui| {
                    for (i, name) in prods.iter().enumerate() {
                        if ui.selectable_label(sel == Some(i), name).clicked() {
                            sel = Some(i);
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("prod-detail")
                .show(&mut cols[1], |ui| {
                    ui.monospace(detail);
                });
        });
        if sel != self.prod_sel {
            self.prod_sel = sel;
            self.prod_detail = match sel {
                Some(i) => {
                    let name = prods[i].clone();
                    self.text("production-details", vec![Json::Str(name)])
                }
                None => String::new(),
            };
        }
    }

    fn param_panel(&mut self, ui: &mut egui::Ui) {
        let modules = self.modules.clone();
        let params = self.params.clone();
        let mut mod_sel = self.mod_sel;
        let mut param_sel = self.param_sel;
        let detail = self.param_detail.clone();
        ui.columns(3, |cols| {
            egui::ScrollArea::vertical()
                .id_salt("mod-list")
                .show(&mut cols[0], |ui| {
                    for (i, name) in modules.iter().enumerate() {
                        if ui.selectable_label(mod_sel == Some(i), name).clicked() {
                            mod_sel = Some(i);
                            param_sel = None;
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("param-list")
                .show(&mut cols[1], |ui| {
                    for (i, name) in params.iter().enumerate() {
                        if ui.selectable_label(param_sel == Some(i), name).clicked() {
                            param_sel = Some(i);
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("param-detail")
                .show(&mut cols[2], |ui| {
                    ui.monospace(detail);
                });
        });
        if mod_sel != self.mod_sel {
            self.mod_sel = mod_sel;
            self.param_sel = None;
            self.param_detail.clear();
            self.params = match mod_sel {
                Some(i) => self.list("modules-parameters", vec![Json::Str(modules[i].clone())]),
                None => Vec::new(),
            };
        } else if param_sel != self.param_sel {
            self.param_sel = param_sel;
            self.param_detail = match param_sel {
                Some(i) => self.text(
                    "printed-parameter-details",
                    vec![Json::Str(params[i].clone())],
                ),
                None => String::new(),
            };
        }
    }

    fn central(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Trace, "Trace");
            ui.selectable_value(&mut self.tab, Tab::Declarative, "Declarative");
            ui.selectable_value(&mut self.tab, Tab::Buffers, "Buffers");
            ui.selectable_value(&mut self.tab, Tab::Procedural, "Procedural");
            ui.selectable_value(&mut self.tab, Tab::Parameters, "Parameters");
            ui.selectable_value(&mut self.tab, Tab::History, "History");
            ui.selectable_value(&mut self.tab, Tab::Graph, "Prod Graph");
            if ui.button("Refresh").clicked() {
                self.refresh_pending = true;
            }
        });
        ui.separator();
        if self.tab != self.last_tab {
            self.refresh_pending = true;
            self.last_tab = self.tab;
        }
        if self.refresh_pending {
            self.refresh_active();
            self.refresh_pending = false;
        }
        match self.tab {
            Tab::Trace => self.trace_panel(ui),
            Tab::Declarative => self.dm_panel(ui),
            Tab::Buffers => self.buffer_panel(ui),
            Tab::Procedural => self.proc_panel(ui),
            Tab::Parameters => self.param_panel(ui),
            Tab::History => self.history_panel(ui),
            Tab::Graph => self.graph_panel(ui),
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
        self.drain_trace();
        self.apply_window_ops();

        // Advance an interactive run one chunk per frame (keeps the UI live
        // and honours Stop).
        if self.backend.is_running() {
            match self.backend.poll_run() {
                Ok(true) => {
                    self.drain_trace();
                    self.apply_window_ops();
                    ui.ctx().request_repaint();
                }
                Ok(false) => {
                    self.drain_trace();
                    self.refresh_pending = true;
                }
                Err(e) => {
                    self.error = Some(e);
                }
            }
        }

        let model = self
            .backend
            .current_model()
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

        egui::CentralPanel::default().show(ui, |ui| self.central(ui));

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
                    if ui
                        .checkbox(&mut self.settings.sort_lists, "sort lists")
                        .changed()
                    {
                        self.refresh_pending = true;
                        changed = true;
                    }
                    if ui
                        .checkbox(&mut self.settings.real_time, "real time")
                        .changed()
                    {
                        self.backend.set_real_time(self.settings.real_time);
                        changed = true;
                    }
                    ui.separator();
                    ui.label(format!("settings: {}", settings::settings_path().display()));
                    if changed {
                        settings::save(&self.settings);
                    }
                });
            self.show_options = open;
        }
        self.render_windows(&ctx);
    }

    fn on_exit(&mut self) {
        settings::save(&self.settings);
    }
}

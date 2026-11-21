//! The egui application shell: control panel, trace pane, and the
//! read-only inspectors (declarative, buffers, procedural).

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use eframe::egui;

use crate::engine::{ProductionGraph, ProductionGraphMode};
use crate::json::Json;

use super::backend::{result_text, BackendEvent, EnvBackend, TraceStream};
use super::debug::env_debug;
use super::export;
use super::graph_parse;
use super::history::{
    self, BlendingEntry, BufferEntry, HistoryView, PerceptualEntry, RetrievalEntry,
};
use super::settings::{self, Settings};

#[cfg(test)]
#[path = "app_gui_tests.rs"]
mod gui_tests;

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
    RetrievalHistory,
    BlendingHistory,
    BufferHistory,
    VisiconHistory,
    AudiconHistory,
    ProductionGraph,
    ProductionGrid,
    HorizontalGraphicTrace,
    VerticalGraphicTrace,
    BoldGraph,
    Bold2D,
    Bold3D,
    BufferPicker,
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
            ToolKind::RetrievalHistory => "Retrieval History",
            ToolKind::BlendingHistory => "Blending History",
            ToolKind::BufferHistory => "Buffer History",
            ToolKind::VisiconHistory => "Visicon History",
            ToolKind::AudiconHistory => "Audicon History",
            ToolKind::ProductionGraph => "Production Graph",
            ToolKind::ProductionGrid => "Production Grid",
            ToolKind::HorizontalGraphicTrace => "Graphic Trace (Horizontal)",
            ToolKind::VerticalGraphicTrace => "Graphic Trace (Vertical)",
            ToolKind::BoldGraph => "BOLD Graph",
            ToolKind::Bold2D => "BOLD 2D Brain",
            ToolKind::Bold3D => "BOLD 3D Brain",
            ToolKind::BufferPicker => "Select Buffers",
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
                | ToolKind::RetrievalHistory
                | ToolKind::BlendingHistory
                | ToolKind::BufferHistory
                | ToolKind::VisiconHistory
                | ToolKind::AudiconHistory
                | ToolKind::ProductionGraph
                | ToolKind::ProductionGrid
                | ToolKind::HorizontalGraphicTrace
                | ToolKind::VerticalGraphicTrace
                | ToolKind::BoldGraph
                | ToolKind::Bold2D
                | ToolKind::Bold3D
                | ToolKind::BufferPicker
        )
    }

    /// The `record-history` source a tool needs while it is open.
    fn recording_source(self) -> Option<&'static str> {
        if let Some(kind) = self.history() {
            Some(kind.source())
        } else if self == ToolKind::ProductionGraph {
            Some("production-history")
        } else {
            None
        }
    }

    fn history(self) -> Option<HistoryKind> {
        match self {
            ToolKind::TextTrace => Some(HistoryKind::Trace),
            ToolKind::RetrievalHistory => Some(HistoryKind::Retrieval),
            ToolKind::BlendingHistory => Some(HistoryKind::Blending),
            ToolKind::BufferHistory => Some(HistoryKind::Buffer),
            ToolKind::VisiconHistory => Some(HistoryKind::Visicon),
            ToolKind::AudiconHistory => Some(HistoryKind::Audicon),
            ToolKind::ProductionGrid => Some(HistoryKind::ProductionGrid),
            ToolKind::HorizontalGraphicTrace => Some(HistoryKind::GraphicHorizontal),
            ToolKind::VerticalGraphicTrace => Some(HistoryKind::GraphicVertical),
            ToolKind::BoldGraph => Some(HistoryKind::BoldGraph),
            ToolKind::Bold2D => Some(HistoryKind::Bold2D),
            ToolKind::Bold3D => Some(HistoryKind::Bold3D),
            _ => None,
        }
    }
}

/// A Tcl Environment history viewer backed by an ACT-R history stream.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HistoryKind {
    Trace,
    Retrieval,
    Blending,
    Buffer,
    Visicon,
    Audicon,
    ProductionGrid,
    GraphicHorizontal,
    GraphicVertical,
    BoldGraph,
    Bold2D,
    Bold3D,
}

impl HistoryKind {
    fn source(self) -> &'static str {
        match self {
            HistoryKind::Trace => "trace-history",
            HistoryKind::Retrieval => "retrieval-history",
            HistoryKind::Blending => "blending-trace",
            HistoryKind::Buffer => "buffer-history",
            HistoryKind::Visicon => "visicon-history",
            HistoryKind::Audicon => "audicon-history",
            HistoryKind::ProductionGrid => "production-history",
            HistoryKind::GraphicHorizontal
            | HistoryKind::GraphicVertical
            | HistoryKind::BoldGraph
            | HistoryKind::Bold2D
            | HistoryKind::Bold3D => "buffer-trace",
        }
    }

    fn processor(self) -> &'static str {
        match self {
            HistoryKind::Trace => "trace-history",
            HistoryKind::Retrieval => "retrieval-history",
            HistoryKind::Blending => "blending-environment-history",
            HistoryKind::Buffer => "buffer-history",
            HistoryKind::Visicon => "visicon-history",
            HistoryKind::Audicon => "audicon-history",
            HistoryKind::ProductionGrid => "production-grid",
            HistoryKind::GraphicHorizontal => "horizontal-graphic-trace",
            HistoryKind::GraphicVertical => "vertical-graphic-trace",
            HistoryKind::BoldGraph => "bold-prediction-with-time",
            HistoryKind::Bold2D | HistoryKind::Bold3D => "bold-prediction-with-time-scaled",
        }
    }

    fn label(self) -> &'static str {
        match self {
            HistoryKind::Trace => "Trace",
            HistoryKind::Retrieval => "Retrieval",
            HistoryKind::Blending => "Blending",
            HistoryKind::Buffer => "Buffer",
            HistoryKind::Visicon => "Visicon",
            HistoryKind::Audicon => "Audicon",
            HistoryKind::ProductionGrid => "Production",
            HistoryKind::GraphicHorizontal | HistoryKind::GraphicVertical => "Graphic Trace",
            HistoryKind::BoldGraph | HistoryKind::Bold2D | HistoryKind::Bold3D => "BOLD",
        }
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
    command_query: String,
    connections: Vec<crate::env::backend::ConnectionInfo>,
    conn_sel: Option<usize>,
    conn_doc_h: f32,
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

    // Histories / production graph.
    history_text: String,
    history_comment: String,
    history_start: String,
    history_end: String,
    history_detail: String,
    history_recording: bool,
    history_records: Vec<Json>,
    history_view: Option<HistoryView>,
    history_time_sel: Option<usize>,
    history_item_sel: Option<usize>,
    picker_selected: Vec<bool>,
    bold_scan: usize,
    bold_scale: usize,
    grid_hide_empty: bool,
    grid_scale: f32,
    grid_detail_title: String,
    grid_detail: String,
    graphic_scale: f32,
    graphic_hide_text: bool,
    graphic_hide_grid: bool,
    graph_hide_unused: bool,
    graph_data: Option<ProductionGraph>,
    graph_mode: ProductionGraphMode,
    graph_page: u32,
    graph_detail_title: String,
    graph_detail: String,
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
            command_query: String::new(),
            connections: Vec::new(),
            conn_sel: None,
            conn_doc_h: 0.0,
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
            history_comment: String::new(),
            history_start: String::new(),
            history_end: String::new(),
            history_detail: "low".to_string(),
            history_recording: false,
            history_records: Vec::new(),
            history_view: None,
            history_time_sel: None,
            history_item_sel: None,
            picker_selected: Vec::new(),
            bold_scan: 0,
            bold_scale: 0,
            grid_hide_empty: false,
            grid_scale: 1.0,
            grid_detail_title: String::new(),
            grid_detail: String::new(),
            graphic_scale: 1.0,
            graphic_hide_text: false,
            graphic_hide_grid: false,
            graph_hide_unused: false,
            graph_data: None,
            graph_mode: ProductionGraphMode::All,
            graph_page: 0,
            graph_detail_title: String::new(),
            graph_detail: String::new(),
        }
    }
}

/// Run Until types in the same order as the Tcl stepper
/// (`21-stepper.tcl`): Time, Production, Module. The index doubles as the
/// `run_type` value, so 0 is Time and the default.
const STEPPER_RUN_TYPES: [&str; 3] = ["Time", "Production", "Module"];
const STEPPER_RUN_KINDS: [&str; 3] = ["time", "production", "module"];

fn stepper_kind(run_type: usize) -> &'static str {
    STEPPER_RUN_KINDS[run_type.min(STEPPER_RUN_KINDS.len() - 1)]
}

const BUFFER_HISTORY_TOP_HEIGHT: f32 = 140.0;
const HISTORY_TOP_HEIGHT: f32 = 180.0;
const HISTORY_DETAIL_MIN_HEIGHT: f32 = 120.0;

fn history_top_height(available: f32) -> f32 {
    (available * 0.60).max(HISTORY_TOP_HEIGHT)
}

fn connection_doc_height(text: &str) -> f32 {
    if text.is_empty() {
        0.0
    } else {
        let lines = text.lines().count().max(1) as f32;
        24.0 + (lines * 14.0).clamp(24.0, 140.0)
    }
}

#[derive(Default)]
struct StepperState {
    id: Option<String>,
    handlers_registered: bool,
    step_all: bool,
    tutor: bool,
    next: String,
    last: String,
    items: Vec<String>,
    selected: Option<usize>,
    params: String,
    bindings: String,
    display: String,
    tutorable: Option<String>,
    run_type: usize,
    run_value: String,
    productions: Vec<String>,
    modules: Vec<String>,
}

fn hex_color(text: &str) -> egui::Color32 {
    let text = text.trim().trim_start_matches('#');
    if text.len() != 6 {
        return egui::Color32::GRAY;
    }
    let parse = |i: usize| u8::from_str_radix(&text[i..i + 2], 16).unwrap_or(128);
    egui::Color32::from_rgb(parse(0), parse(2), parse(4))
}

fn palette(i: usize) -> egui::Color32 {
    const COLORS: [egui::Color32; 14] = [
        egui::Color32::from_rgb(0xff, 0x00, 0x00),
        egui::Color32::from_rgb(0x80, 0x80, 0xff),
        egui::Color32::from_rgb(0x00, 0x80, 0x00),
        egui::Color32::from_rgb(0xff, 0x80, 0x00),
        egui::Color32::from_rgb(0x7f, 0x7f, 0x7f),
        egui::Color32::from_rgb(0xff, 0x00, 0xff),
        egui::Color32::from_rgb(0xff, 0xff, 0x00),
        egui::Color32::from_rgb(0xc0, 0x80, 0x00),
        egui::Color32::from_rgb(0xf0, 0xf0, 0xf0),
        egui::Color32::from_rgb(0xc0, 0x00, 0xc0),
        egui::Color32::from_rgb(0x00, 0xc0, 0x50),
        egui::Color32::from_rgb(0xf0, 0xc0, 0xff),
        egui::Color32::from_rgb(0xc0, 0x58, 0x50),
        egui::Color32::from_rgb(0x50, 0xff, 0x25),
    ];
    COLORS[i % COLORS.len()]
}

fn translate_special_key(key: egui::Key) -> Option<&'static str> {
    use egui::Key;
    Some(match key {
        Key::Enter => "return",
        Key::Escape => "esc",
        Key::Backspace => "delete",
        Key::Tab => "tab",
        Key::ArrowUp => "up-arrow",
        Key::ArrowDown => "down-arrow",
        Key::ArrowLeft => "left-arrow",
        Key::ArrowRight => "right-arrow",
        Key::Delete => "delete",
        Key::Home => "home",
        Key::End => "end",
        Key::PageUp => "page-up",
        Key::PageDown => "page-down",
        Key::Insert => "insert",
        Key::F1 => "f1",
        Key::F2 => "f2",
        Key::F3 => "f3",
        Key::F4 => "f4",
        Key::F5 => "f5",
        Key::F6 => "f6",
        Key::F7 => "f7",
        Key::F8 => "f8",
        Key::F9 => "f9",
        Key::F10 => "f10",
        Key::F11 => "f11",
        Key::F12 => "f12",
        _ => return None,
    })
}
/// A simple code editor window (`12-ctrl-panel-open-button.tcl`).
struct EditorWindow {
    id: u64,
    path: PathBuf,
    text: String,
    dirty: bool,
    open: bool,
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
    eyeloc: Option<(f32, f32, egui::Color32)>,
    fingers: Vec<(String, String, f32, f32, bool)>,
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
    Image {
        id: String,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        file: String,
    },
}

impl WinItem {
    fn id(&self) -> &str {
        match self {
            WinItem::Button { id, .. }
            | WinItem::Text { id, .. }
            | WinItem::Line { id, .. }
            | WinItem::Image { id, .. } => id,
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
    /// Model / command / warning / general trace visibility.
    trace_filter: [bool; 4],
    /// Stepper session state (one stepper at a time, as in ACT-R).
    stepper: StepperState,
    /// Reference counts for `record-history` sources shared by viewers.
    recorded_history: HashMap<(Option<String>, String), usize>,
    /// The visible experiment windows the engine is displaying.
    windows: Vec<VisWin>,
    /// Floating tool windows (viewers, Event Queue, Connections, About, …).
    tools: Vec<ToolWindow>,
    next_tool_id: u64,
    /// Open code-editor windows.
    editors: Vec<EditorWindow>,
    next_editor_id: u64,
    /// Editor id awaiting an unsaved-changes decision.
    editor_close: Option<u64>,
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
        let registered = backend.evaluate(
            "add-virtual-window-handler",
            None,
            vec![Json::Str(handler.to_string())],
        );
        env_debug!("add-virtual-window-handler -> {registered:?}");
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
            trace_filter: [true; 4],
            stepper: StepperState::default(),
            recorded_history: HashMap::new(),
            windows: Vec::new(),
            tools: Vec::new(),
            next_tool_id: 1,
            editors: Vec::new(),
            next_editor_id: 1,
            editor_close: None,
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
        let mut tool = ToolWindow::new(id, kind, model);
        if tool.kind.recording_source().is_some() {
            self.set_history_recording(&mut tool, true);
        }
        self.tools.push(tool);
        if kind == ToolKind::Stepper {
            // Match the Tcl Stepper button: start in normal stepping mode,
            // not step-all. Run Until clears Step all when it needs to skip.
            self.stepper.step_all = false;
            self.start_stepper();
        }
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
            ToolKind::TextTrace
            | ToolKind::RetrievalHistory
            | ToolKind::BlendingHistory
            | ToolKind::BufferHistory
            | ToolKind::VisiconHistory
            | ToolKind::AudiconHistory
            | ToolKind::ProductionGrid
            | ToolKind::HorizontalGraphicTrace
            | ToolKind::VerticalGraphicTrace
            | ToolKind::BoldGraph
            | ToolKind::Bold2D
            | ToolKind::Bold3D => self.fetch_history(tool),
            ToolKind::BufferPicker => {
                tool.buffers = self.list_for(model.clone(), "model-buffers", Vec::new());
                if self.settings.sort_lists {
                    tool.buffers.sort();
                }
                let used = self.list_for(model.clone(), "used-production-buffers", Vec::new());
                let first = tool.picker_selected.is_empty();
                if first {
                    tool.picker_selected = tool
                        .buffers
                        .iter()
                        .map(|b| used.iter().any(|u| u.eq_ignore_ascii_case(b)))
                        .collect();
                } else {
                    tool.picker_selected.resize(tool.buffers.len(), false);
                }
                if first {
                    let selected: Vec<String> = tool
                        .buffers
                        .iter()
                        .zip(tool.picker_selected.iter())
                        .filter(|(_, on)| **on)
                        .map(|(b, _)| b.clone())
                        .collect();
                    for buffer in selected {
                        self.eval_for(model.clone(), "record-history", vec![Json::Str(buffer)]);
                    }
                }
                tool.refresh_pending = false;
            }
            ToolKind::ProductionGraph => {
                tool.graph_data = self.fetch_graph(model, tool.graph_mode);
                if let Some(data) = &tool.graph_data {
                    let pages = data.pages.len().max(1) as u32;
                    tool.graph_page = tool.graph_page.min(pages - 1);
                }
            }
            ToolKind::EventQueue => {
                tool.text = self
                    .eval("mp-queue-text", vec![Json::Int(1)])
                    .map_or_else(String::new, |v| {
                        v.first().and_then(|x| x.as_str()).unwrap_or("").to_string()
                    });
            }
            ToolKind::Connections => match self.backend.connections() {
                Ok(c) => {
                    tool.connections = c;
                    if tool.connections.is_empty() {
                        tool.conn_sel = None;
                    } else if tool.conn_sel.map_or(true, |i| i >= tool.connections.len()) {
                        tool.conn_sel = Some(0);
                    }
                }
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
            let mut win = egui::Window::new(&tool.title)
                .id(egui::Id::unique(("tool-window", tool.id)))
                .open(&mut open);
            if tool.kind.history().is_some() {
                win = win.min_size([420.0, 180.0]);
            }
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
            } else if tool.history_recording {
                self.set_history_recording(&mut tool, false);
            } else if tool.kind == ToolKind::Stepper {
                self.stop_stepper();
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
                let mut sel = tool.conn_sel;
                let mut selected_cmd: Option<String> = None;
                let mut doc_requested = false;
                egui::ScrollArea::both()
                    .id_salt(("connections-body", tool.id))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let width = ui.available_width();
                        let height = (ui.available_height() - ui.spacing().item_spacing.y).max(1.0);
                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(width * 0.28, height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.label("Connections");
                                    egui::ScrollArea::both()
                                        .id_salt(("connections-list", tool.id))
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            for (i, c) in tool.connections.iter().enumerate() {
                                                if ui
                                                    .selectable_label(sel == Some(i), &c.name)
                                                    .clicked()
                                                {
                                                    sel = Some(i);
                                                }
                                            }
                                        });
                                },
                            );
                            let right_sel = sel;
                            ui.allocate_ui_with_layout(
                                egui::vec2(width * 0.70, height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    let Some(c) = tool.connections.get(right_sel.unwrap_or(0))
                                    else {
                                        ui.label("No connections.");
                                        return;
                                    };
                                    ui.horizontal(|ui| {
                                        ui.label("Address:");
                                        ui.add(egui::Label::new(c.address.as_str()).wrap());
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Pending:");
                                        ui.label(c.pending_calls.to_string());
                                        ui.separator();
                                        ui.label("Sent:");
                                        ui.label(c.sent.to_string());
                                        ui.separator();
                                        ui.label("Received:");
                                        ui.label(c.received.to_string());
                                    });
                                    ui.separator();
                                    let bottom_h = 48.0 + tool.conn_doc_h;
                                    ui.label("Commands");
                                    let list_h = (ui.available_height()
                                        - bottom_h
                                        - ui.spacing().item_spacing.y)
                                        .max(80.0);
                                    if c.commands.is_empty() {
                                        ui.label("(none)");
                                    } else {
                                        let row_h = ui.text_style_height(&egui::TextStyle::Body)
                                            + ui.spacing().item_spacing.y;
                                        egui::ScrollArea::both()
                                            .id_salt(("connection-commands", tool.id))
                                            .auto_shrink([false, false])
                                            .max_height(list_h)
                                            .show_rows(ui, row_h, c.commands.len(), |ui, range| {
                                                for i in range {
                                                    let cmd = &c.commands[i];
                                                    if ui
                                                        .selectable_label(
                                                            tool.command_query == *cmd,
                                                            cmd,
                                                        )
                                                        .clicked()
                                                    {
                                                        selected_cmd = Some(cmd.clone());
                                                    }
                                                }
                                            });
                                    }
                                    ui.separator();
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label("Command:");
                                        ui.add(
                                            egui::TextEdit::singleline(&mut tool.command_query)
                                                .desired_width(140.0),
                                        );
                                        if ui.button("Documentation").clicked() {
                                            doc_requested = true;
                                        }
                                    });
                                    if !tool.text.is_empty() {
                                        ui.separator();
                                        ui.label("Documentation:");
                                        egui::ScrollArea::both()
                                            .id_salt(("connection-doc", tool.id))
                                            .auto_shrink([false, false])
                                            .show(ui, |ui| {
                                                ui.monospace(&tool.text);
                                            });
                                    }
                                },
                            );
                        });
                    });
                tool.conn_sel = sel;
                if let Some(cmd) = selected_cmd {
                    tool.command_query = cmd.clone();
                    match self.backend.command_doc(&cmd) {
                        Ok(Some(doc)) if !doc.is_empty() => tool.text = doc,
                        Ok(_) => tool.text = "(no documentation)".into(),
                        Err(e) => tool.text = format!("check {cmd}: {e}"),
                    }
                } else if doc_requested {
                    let name = tool.command_query.trim().to_string();
                    tool.text = match self.backend.command_doc(&name) {
                        Ok(Some(doc)) if !doc.is_empty() => doc,
                        Ok(_) => "(no documentation)".into(),
                        Err(e) => format!("check {name}: {e}"),
                    };
                }
                tool.conn_doc_h = connection_doc_height(&tool.text);
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
            ToolKind::TextTrace
            | ToolKind::RetrievalHistory
            | ToolKind::BlendingHistory
            | ToolKind::BufferHistory
            | ToolKind::VisiconHistory
            | ToolKind::AudiconHistory => self.history_panel(tool, ui),
            ToolKind::ProductionGrid => self.production_grid_panel(tool, ui),
            ToolKind::HorizontalGraphicTrace | ToolKind::VerticalGraphicTrace => {
                self.graphic_trace_panel(tool, ui)
            }
            ToolKind::BoldGraph | ToolKind::Bold2D | ToolKind::Bold3D => self.bold_panel(tool, ui),
            ToolKind::BufferPicker => self.buffer_picker_panel(tool, ui),
            ToolKind::ProductionGraph => self.graph_panel(tool, ui),
            ToolKind::Stepper => self.stepper_panel(tool, ui),
        }
    }

    fn fetch_graph(
        &mut self,
        model: Option<String>,
        mode: ProductionGraphMode,
    ) -> Option<ProductionGraph> {
        let target = model.clone();
        let available = self
            .eval_for(
                target.clone(),
                "history-data-available",
                vec![Json::Str("production-history".into())],
            )
            .and_then(|v| v.into_iter().next());
        let any = match available {
            Some(Json::Bool(value)) => value,
            Some(Json::Null) | None => false,
            _ => true,
        };
        if !any {
            return None;
        }
        let id = self
            .eval_for(
                target,
                "start-incremental-history-data",
                vec![Json::Str(mode.command().into()), Json::Int(16000)],
            )
            .and_then(|v| v.into_iter().next());
        let Some(Json::Int(id)) = id else {
            return None;
        };
        let (_, records) = self.collect_history(id);
        graph_parse::from_canvas_items(&records, mode)
    }

    fn graph_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let mut mode = tool.graph_mode;
            for choice in [
                ProductionGraphMode::All,
                ProductionGraphMode::Frequencies,
                ProductionGraphMode::Cycles,
                ProductionGraphMode::UniqueCycles,
                ProductionGraphMode::Runs,
                ProductionGraphMode::UniqueRuns,
                ProductionGraphMode::Utilities,
            ] {
                ui.selectable_value(&mut mode, choice, choice.label());
            }
            if mode != tool.graph_mode {
                tool.graph_mode = mode;
                tool.graph_page = 0;
                tool.graph_data = self.fetch_graph(tool.model.clone(), mode);
            }
            ui.checkbox(&mut tool.graph_hide_unused, "hide unused productions");
        });
        ui.separator();
        let Some(data) = &tool.graph_data else {
            ui.label("No production-history graph yet. Record \"production-history\", run, then Refresh.");
            return;
        };
        let pages = data.pages.len().max(1) as u32;
        tool.graph_page = tool.graph_page.min(pages - 1);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(tool.graph_page > 0, egui::Button::new("←"))
                .clicked()
            {
                tool.graph_page -= 1;
            }
            ui.label(format!("{} / {}", tool.graph_page + 1, pages));
            if ui
                .add_enabled(tool.graph_page + 1 < pages, egui::Button::new("→"))
                .clicked()
            {
                tool.graph_page += 1;
            }
            if let Some(page) = data.pages.get(tool.graph_page as usize) {
                if let (Some(start), Some(end)) = (page.start_ms, page.end_ms) {
                    ui.label(format!(
                        "Time {:.3} - {:.3}",
                        start as f64 / 1000.0,
                        end as f64 / 1000.0
                    ));
                }
            }
        });
        let layout = export::layout_graph_page(data, tool.graph_page, tool.graph_hide_unused);
        ui.horizontal(|ui| {
            if ui.button("Save .eps").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Encapsulated PostScript", &["eps"])
                    .save_file()
                {
                    if let Err(e) = std::fs::write(&path, export::graph_eps(&layout)) {
                        self.error = Some(format!("save {}: {e}", path.display()));
                    }
                }
            }
            if ui.button("Save .dot").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("DOT graph", &["dot"])
                    .save_file()
                {
                    if let Err(e) = std::fs::write(&path, export::graph_dot(&layout)) {
                        self.error = Some(format!("save {}: {e}", path.display()));
                    }
                }
            }
        });
        if layout.nodes.is_empty() {
            ui.label("No nodes on this graph page.");
            return;
        }
        let selected = hex_color(&self.settings.p_selected);
        let mismatched = hex_color(&self.settings.p_mismatched);
        let box_w = export::GRAPH_BOX_W;
        let box_h = export::GRAPH_BOX_H;
        let group_colors = [
            egui::Color32::BLACK,
            egui::Color32::from_rgb(204, 26, 26),
            egui::Color32::from_rgb(26, 77, 217),
            egui::Color32::from_rgb(26, 153, 51),
            egui::Color32::from_rgb(230, 115, 13),
            egui::Color32::from_rgb(140, 26, 166),
            egui::Color32::from_rgb(0, 140, 153),
            egui::Color32::from_rgb(140, 102, 13),
        ];
        egui::ScrollArea::both()
            .id_salt(("prod-graph", tool.id))
            .show(ui, |ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(layout.width, layout.height),
                    egui::Sense::hover(),
                );
                let p = ui.painter_at(rect);
                for edge in &layout.edges {
                    let (Some(&(ax, ay)), Some(&(bx, by))) = (
                        layout.positions.get(&edge.from),
                        layout.positions.get(&edge.to),
                    ) else {
                        continue;
                    };
                    let from = rect.min + egui::vec2(ax + box_w / 2.0, ay + box_h);
                    let to = rect.min + egui::vec2(bx + box_w / 2.0, by);
                    let color = if edge.dashed {
                        egui::Color32::GRAY
                    } else {
                        group_colors[edge.group.unwrap_or(0) as usize % group_colors.len()]
                    };
                    let width = if layout.weighted_edges {
                        1.0 + (edge.count.saturating_sub(1) as f32).min(5.0)
                    } else {
                        1.0
                    };
                    if edge.dashed {
                        for shape in egui::Shape::dashed_line(
                            &[from, to],
                            egui::Stroke::new(1.0, color),
                            4.0,
                            3.0,
                        ) {
                            p.add(shape);
                        }
                    } else if edge.from == edge.to {
                        let r = egui::Rect::from_min_size(
                            rect.min + egui::vec2(ax + box_w - 8.0, ay - 24.0),
                            egui::vec2(32.0, 24.0),
                        );
                        p.rect_stroke(
                            r,
                            6.0,
                            egui::Stroke::new(width, color),
                            egui::StrokeKind::Middle,
                        );
                    } else {
                        p.line_segment([from, to], egui::Stroke::new(width, color));
                    }
                }
                for node in &layout.nodes {
                    let Some(&(x, y)) = layout.positions.get(&node.name) else {
                        continue;
                    };
                    let r = egui::Rect::from_min_size(
                        rect.min + egui::vec2(x, y),
                        egui::vec2(box_w, box_h),
                    );
                    p.rect_filled(
                        r,
                        egui::CornerRadius::ZERO,
                        egui::Color32::from_rgb(230, 230, 230),
                    );
                    let border = if node.start {
                        selected
                    } else if node.end {
                        mismatched
                    } else if !node.used {
                        egui::Color32::GRAY
                    } else {
                        egui::Color32::BLACK
                    };
                    p.rect_stroke(
                        r,
                        egui::CornerRadius::ZERO,
                        egui::Stroke::new(if node.start || node.end { 2.0 } else { 1.0 }, border),
                        egui::StrokeKind::Inside,
                    );
                    p.text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        &node.name,
                        egui::FontId::proportional(12.0),
                        if node.used {
                            egui::Color32::BLACK
                        } else {
                            egui::Color32::GRAY
                        },
                    );
                    if let (Some(value), Some(min), Some(max)) =
                        (node.utility, layout.utility_min, layout.utility_max)
                    {
                        let frac = if max == min {
                            1.0
                        } else {
                            ((value - min) / (max - min)).clamp(0.0, 1.0) as f32
                        };
                        let bar = egui::Rect::from_min_size(
                            r.min + egui::vec2(4.0, box_h + 2.0),
                            egui::vec2((box_w - 8.0) * frac, 4.0),
                        );
                        p.rect_filled(bar, 0.0, egui::Color32::from_rgb(26, 77, 217));
                    }
                    let response = ui.interact(
                        r,
                        ui.make_persistent_id(("graph-node", tool.id, &node.name, layout.page)),
                        egui::Sense::click(),
                    );
                    if !node.text.is_empty() {
                        response.clone().on_hover_text(&node.text);
                    }
                    if response.clicked() {
                        tool.graph_detail_title = node.name.clone();
                        tool.graph_detail = node.text.clone();
                    }
                }
            });
        if !tool.graph_detail.is_empty() {
            let mut open = true;
            egui::Window::new(&tool.graph_detail_title)
                .id(egui::Id::unique(("graph-detail", tool.id)))
                .open(&mut open)
                .default_width(420.0)
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::both()
                        .id_salt(("graph-detail", tool.id))
                        .show(ui, |ui| {
                            ui.monospace(&tool.graph_detail);
                        });
                });
            if !open {
                tool.graph_detail.clear();
                tool.graph_detail_title.clear();
            }
        }
    }

    /// Fetch the recorded `trace-history` via the incremental protocol.
    fn history_target(kind: HistoryKind, model: Option<String>) -> Option<String> {
        if kind == HistoryKind::Trace {
            None
        } else {
            model
        }
    }

    fn history_start_args(&self, tool: &ToolWindow) -> Vec<Json> {
        let kind = tool.kind.history().expect("history tool");
        let mut args = vec![
            Json::Str(kind.processor().into()),
            Json::Int(16000),
            Json::Bool(false),
        ];
        if kind == HistoryKind::Trace {
            args.push(
                parse_seconds(&tool.history_start)
                    .map(Json::Float)
                    .unwrap_or(Json::Null),
            );
            args.push(
                parse_seconds(&tool.history_end)
                    .map(Json::Float)
                    .unwrap_or(Json::Null),
            );
            args.push(Json::Str(history_detail_arg(&tool.history_detail).into()));
        }
        args
    }

    fn collect_history(&mut self, id: i64) -> (Option<String>, Vec<Json>) {
        let mut comment = None;
        let mut records = Vec::new();
        loop {
            let chunk = self
                .eval("get-incremental-history-data", vec![Json::Int(id)])
                .and_then(|v| v.into_iter().next())
                .and_then(|x| x.as_str().map(str::to_string));
            let Some(chunk) = chunk else { break };
            if history_chunk_is_end(&chunk) {
                break;
            }
            match crate::json::parse(&chunk) {
                Ok(Json::Arr(mut a)) => records.append(&mut a),
                Ok(Json::Str(s)) => comment = Some(s),
                Ok(Json::Null) => break,
                _ => {}
            }
        }
        (comment, records)
    }

    fn fetch_history(&mut self, tool: &mut ToolWindow) {
        let kind = tool.kind.history().expect("history tool");
        let target = Self::history_target(kind, tool.model.clone());
        let available_args = if kind == HistoryKind::Trace {
            vec![
                Json::Str(kind.source().into()),
                Json::Null,
                Json::Null,
                Json::Str(history_detail_arg(&tool.history_detail).into()),
            ]
        } else {
            vec![Json::Str(kind.source().into())]
        };
        let available = self
            .eval_for(target.clone(), "history-data-available", available_args)
            .and_then(|v| v.into_iter().next());
        let any = match available {
            Some(Json::Bool(b)) => b,
            Some(Json::Null) | None => false,
            _ => true,
        };
        if !any {
            tool.history_text = "No Data available".into();
            tool.history_comment.clear();
            return;
        }
        let args = self.history_start_args(tool);
        let id = self
            .eval_for(target, "start-incremental-history-data", args)
            .and_then(|v| v.into_iter().next());
        let Some(Json::Int(id)) = id else {
            tool.history_text = "No Data available".into();
            return;
        };
        let (comment, records) = self.collect_history(id);
        if let Some(c) = comment {
            tool.history_comment = c;
        }
        if tool.history_comment.is_empty() {
            tool.history_comment = format!(
                "{} history data for {}",
                kind.label(),
                tool.model.as_deref().unwrap_or("model")
            );
        }
        tool.history_view = Self::parse_history_view(kind, &records);
        tool.history_time_sel = None;
        tool.history_item_sel = None;
        tool.history_text = render_history(kind, &records);
        tool.history_records = records;
    }

    fn parse_history_view(kind: HistoryKind, records: &[Json]) -> Option<HistoryView> {
        match kind {
            HistoryKind::Retrieval => {
                Some(HistoryView::Retrieval(history::parse_retrieval(records)))
            }
            HistoryKind::Blending => Some(HistoryView::Blending(history::parse_blending(records))),
            HistoryKind::Buffer => Some(HistoryView::Buffer(history::parse_buffer(records))),
            HistoryKind::Visicon | HistoryKind::Audicon => {
                Some(HistoryView::Perceptual(history::parse_perceptual(records)))
            }
            _ => None,
        }
    }

    fn save_history(&mut self, tool: &ToolWindow, path: &std::path::Path) {
        let kind = tool.kind.history().expect("history tool");
        let target = Self::history_target(kind, tool.model.clone());
        let mut args = vec![
            Json::Str(kind.processor().into()),
            Json::Str(path.to_string_lossy().into_owned()),
            Json::Str(format!(
                "Data saved from the {} History window",
                kind.label()
            )),
        ];
        if kind == HistoryKind::Trace {
            args.push(
                parse_seconds(&tool.history_start)
                    .map(Json::Float)
                    .unwrap_or(Json::Null),
            );
            args.push(
                parse_seconds(&tool.history_end)
                    .map(Json::Float)
                    .unwrap_or(Json::Null),
            );
            args.push(Json::Str(history_detail_arg(&tool.history_detail).into()));
        }
        self.eval_for(target, "save-history-data", args);
    }

    fn load_history(&mut self, tool: &mut ToolWindow, path: &std::path::Path) {
        let kind = tool.kind.history().expect("history tool");
        let target = Self::history_target(kind, tool.model.clone());
        let args = vec![
            Json::Str(kind.processor().into()),
            Json::Int(16000),
            Json::Str(path.to_string_lossy().into_owned()),
        ];
        let id = self
            .eval_for(target, "start-incremental-history-data", args)
            .and_then(|v| v.into_iter().next());
        let Some(Json::Int(id)) = id else {
            tool.history_text = "No Data available".into();
            return;
        };
        let (comment, records) = self.collect_history(id);
        if let Some(c) = comment {
            tool.history_comment = c;
        }
        tool.history_view = Self::parse_history_view(kind, &records);
        tool.history_time_sel = None;
        tool.history_item_sel = None;
        tool.history_text = render_history(kind, &records);
        tool.history_records = records;
    }

    fn history_recording_target(tool: &ToolWindow) -> Option<String> {
        match tool.kind.history() {
            Some(HistoryKind::Trace) => None,
            _ => tool.model.clone(),
        }
    }

    /// Start recording a source once, even when several viewers need it.
    fn acquire_history(&mut self, model: Option<String>, source: &str) {
        let key = (model.clone(), source.to_string());
        let start = match self.recorded_history.get_mut(&key) {
            Some(count) => {
                *count += 1;
                false
            }
            None => {
                self.recorded_history.insert(key, 1);
                true
            }
        };
        if start {
            self.eval_for(model, "record-history", vec![Json::Str(source.to_string())]);
        }
    }

    /// Stop recording a source when its last viewer closes.
    fn release_history(&mut self, model: Option<String>, source: &str) {
        let key = (model.clone(), source.to_string());
        let stop = match self.recorded_history.get_mut(&key) {
            Some(count) if *count > 1 => {
                *count -= 1;
                false
            }
            Some(_) => {
                self.recorded_history.remove(&key);
                true
            }
            None => false,
        };
        if stop {
            self.eval_for(
                model,
                "stop-recording-history",
                vec![Json::Str(source.to_string())],
            );
        }
    }

    fn set_history_recording(&mut self, tool: &mut ToolWindow, on: bool) {
        let Some(source) = tool.kind.recording_source() else {
            return;
        };
        let target = Self::history_recording_target(tool);
        if on {
            self.acquire_history(target, source);
        } else {
            self.release_history(target, source);
        }
        tool.history_recording = on;
    }
    fn history_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        let kind = tool.kind.history().expect("history tool");
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                tool.refresh_pending = true;
            }
            if ui.button("Save…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("History", &["txt", "log", "json"])
                    .save_file()
                {
                    self.save_history(tool, &path);
                }
            }
            if ui.button("Load…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("History", &["txt", "log", "json"])
                    .pick_file()
                {
                    self.load_history(tool, &path);
                }
            }
            if tool.history_recording {
                ui.colored_label(egui::Color32::from_rgb(0, 128, 0), "recording");
            }
        });
        if kind == HistoryKind::Trace {
            ui.horizontal(|ui| {
                ui.label("Detail:");
                egui::ComboBox::from_id_salt(("history-detail", tool.id))
                    .selected_text(tool.history_detail.clone())
                    .show_ui(ui, |ui| {
                        for level in ["low", "medium", "high", "all"] {
                            if ui
                                .selectable_label(
                                    tool.history_detail.eq_ignore_ascii_case(level),
                                    level,
                                )
                                .clicked()
                            {
                                tool.history_detail = level.to_string();
                            }
                        }
                    });
                ui.label("Start:");
                ui.add(
                    egui::TextEdit::singleline(&mut tool.history_start)
                        .desired_width(60.0)
                        .hint_text("s"),
                );
                ui.label("End:");
                ui.add(
                    egui::TextEdit::singleline(&mut tool.history_end)
                        .desired_width(60.0)
                        .hint_text("s"),
                );
            });
        }
        if !tool.history_comment.is_empty() {
            ui.label(&tool.history_comment);
        }
        ui.separator();
        match tool.history_view.clone() {
            Some(HistoryView::Retrieval(entries)) => {
                egui::ScrollArea::vertical()
                    .id_salt(("history-body", tool.id))
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.retrieval_panes(tool, ui, &entries));
            }
            Some(HistoryView::Blending(entries)) => {
                egui::ScrollArea::vertical()
                    .id_salt(("history-body", tool.id))
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.blending_panes(tool, ui, &entries));
            }
            Some(HistoryView::Buffer(entries)) => {
                egui::ScrollArea::vertical()
                    .id_salt(("history-body", tool.id))
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.buffer_history_panes(tool, ui, &entries));
            }
            Some(HistoryView::Perceptual(entries)) => {
                egui::ScrollArea::vertical()
                    .id_salt(("history-body", tool.id))
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.perceptual_panes(tool, ui, &entries));
            }
            None => {
                egui::ScrollArea::both()
                    .id_salt(("history", tool.id))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.monospace(&tool.history_text);
                    });
            }
        }
    }
    fn retrieval_panes(
        &mut self,
        tool: &mut ToolWindow,
        ui: &mut egui::Ui,
        entries: &[RetrievalEntry],
    ) {
        let mut time_sel = tool.history_time_sel;
        let mut item_sel = tool.history_item_sel;
        let entry = time_sel.and_then(|i| entries.get(i));
        let chunk = entry.and_then(|e| item_sel.and_then(|i| e.chunks.get(i)));
        let width = ui.available_width();
        let height = ui.available_height().max(1.0);
        let top_h = history_top_height(height);
        let rest = (height - top_h).max(1.0);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.12, top_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Times");
                    egui::ScrollArea::vertical()
                        .id_salt("retrieval-times")
                        .show(ui, |ui| {
                            for (i, entry) in entries.iter().enumerate() {
                                if ui
                                    .selectable_label(
                                        time_sel == Some(i),
                                        history::format_time(entry.time_ms),
                                    )
                                    .clicked()
                                {
                                    time_sel = Some(i);
                                    item_sel = None;
                                }
                            }
                        });
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.28, top_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Matching Chunks");
                    egui::ScrollArea::vertical()
                        .id_salt("retrieval-chunks")
                        .show(ui, |ui| {
                            if let Some(entry) = entry {
                                for (i, chunk) in entry.chunks.iter().enumerate() {
                                    if ui
                                        .selectable_label(item_sel == Some(i), &chunk.name)
                                        .clicked()
                                    {
                                        item_sel = Some(i);
                                    }
                                }
                            }
                        });
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.60, top_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Details");
                    egui::ScrollArea::both()
                        .id_salt("retrieval-details")
                        .show(ui, |ui| {
                            if let Some(chunk) = chunk {
                                ui.monospace(&chunk.text);
                                ui.monospace(&chunk.params);
                            }
                        });
                },
            );
        });
        ui.separator();
        ui.label("Request");
        egui::ScrollArea::both()
            .id_salt("retrieval-request")
            .max_height((rest * 0.25).max(60.0))
            .show(ui, |ui| {
                if let Some(entry) = entry {
                    ui.monospace(format!("+retrieval>\n{}", entry.request));
                }
            });
        ui.separator();
        ui.label("Activation");
        egui::ScrollArea::both()
            .id_salt("retrieval-activation")
            .show(ui, |ui| {
                if let Some(chunk) = chunk {
                    ui.monospace(&chunk.trace);
                }
            });
        tool.history_time_sel = time_sel;
        tool.history_item_sel = item_sel;
    }

    fn blending_panes(
        &mut self,
        tool: &mut ToolWindow,
        ui: &mut egui::Ui,
        entries: &[BlendingEntry],
    ) {
        let mut time_sel = tool.history_time_sel;
        let mut item_sel = tool.history_item_sel;
        let entry = time_sel.and_then(|i| entries.get(i));
        let chunk = entry.and_then(|e| item_sel.and_then(|i| e.chunks.get(i)));
        let width = ui.available_width();
        let height = ui.available_height().max(1.0);
        let top_h = history_top_height(height);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.15, top_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Times");
                    egui::ScrollArea::vertical()
                        .id_salt("blending-times")
                        .show(ui, |ui| {
                            for (i, entry) in entries.iter().enumerate() {
                                if ui
                                    .selectable_label(
                                        time_sel == Some(i),
                                        history::format_time(entry.time_ms),
                                    )
                                    .clicked()
                                {
                                    time_sel = Some(i);
                                    item_sel = None;
                                }
                            }
                        });
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.30, top_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Request");
                    egui::ScrollArea::both()
                        .id_salt("blending-request")
                        .max_height(top_h * 0.45)
                        .show(ui, |ui| {
                            if let Some(entry) = entry {
                                ui.monospace(format!("+blending>\n{}", entry.request));
                            }
                        });
                    ui.label("Result");
                    egui::ScrollArea::both()
                        .id_salt("blending-result")
                        .show(ui, |ui| {
                            if let Some(entry) = entry {
                                ui.monospace(&entry.result);
                            }
                        });
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.20, top_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Blended Chunks");
                    egui::ScrollArea::vertical()
                        .id_salt("blending-chunks")
                        .show(ui, |ui| {
                            if let Some(entry) = entry {
                                for (i, chunk) in entry.chunks.iter().enumerate() {
                                    if ui
                                        .selectable_label(item_sel == Some(i), &chunk.name)
                                        .clicked()
                                    {
                                        item_sel = Some(i);
                                    }
                                }
                            }
                        });
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.35, top_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Chunk");
                    egui::ScrollArea::both()
                        .id_salt("blending-chunk-detail")
                        .show(ui, |ui| {
                            if let Some(chunk) = chunk {
                                ui.monospace(format!(
                                    "Activation: {}\n\n{}",
                                    chunk.activation, chunk.printed
                                ));
                            }
                        });
                },
            );
        });
        ui.separator();
        ui.label("Trace");
        egui::ScrollArea::both()
            .id_salt("blending-trace")
            .show(ui, |ui| {
                if let Some(entry) = entry {
                    ui.monospace(&entry.trace);
                }
            });
        tool.history_time_sel = time_sel;
        tool.history_item_sel = item_sel;
    }

    fn buffer_history_panes(
        &mut self,
        tool: &mut ToolWindow,
        ui: &mut egui::Ui,
        entries: &[BufferEntry],
    ) {
        let mut time_sel = tool.history_time_sel;
        let mut item_sel = tool.history_item_sel;
        let entry = time_sel.and_then(|i| entries.get(i));
        let action = entry.and_then(|e| item_sel.and_then(|i| e.actions.get(i)));
        let width = ui.available_width();
        let top_h = BUFFER_HISTORY_TOP_HEIGHT;
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.30, top_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Times");
                    egui::ScrollArea::vertical()
                        .id_salt("buffer-times")
                        .show(ui, |ui| {
                            for (i, entry) in entries.iter().enumerate() {
                                if ui
                                    .selectable_label(
                                        time_sel == Some(i),
                                        history::format_time(entry.time_ms),
                                    )
                                    .clicked()
                                {
                                    time_sel = Some(i);
                                    item_sel = None;
                                }
                            }
                        });
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.70, top_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Buffer actions");
                    egui::ScrollArea::vertical()
                        .id_salt("buffer-actions")
                        .show(ui, |ui| {
                            if let Some(entry) = entry {
                                for (i, action) in entry.actions.iter().enumerate() {
                                    let label = format!("{} {}", action.buffer, action.action);
                                    if ui.selectable_label(item_sel == Some(i), label).clicked() {
                                        item_sel = Some(i);
                                    }
                                }
                            }
                        });
                },
            );
        });
        ui.separator();
        let pane_h = ui.available_height().max(HISTORY_DETAIL_MIN_HEIGHT);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.34, pane_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("After Action");
                    egui::ScrollArea::both()
                        .id_salt("buffer-after")
                        .show(ui, |ui| {
                            if let Some(action) = action {
                                ui.monospace(format!(
                                    "{}\n  {}\n{}",
                                    action.action, action.chunk, action.status_after
                                ));
                            }
                        });
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.33, pane_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Starting");
                    egui::ScrollArea::both()
                        .id_salt("buffer-starting")
                        .show(ui, |ui| {
                            if let Some(action) = action {
                                ui.monospace(format!(
                                    "{}\n{}",
                                    action.previous_chunk, action.previous_status
                                ));
                            }
                        });
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.33, pane_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Ending");
                    egui::ScrollArea::both()
                        .id_salt("buffer-ending")
                        .show(ui, |ui| {
                            if let Some(action) = action {
                                ui.monospace(format!(
                                    "{}\n{}",
                                    action.current_chunk, action.current_status
                                ));
                            }
                        });
                },
            );
        });
        tool.history_time_sel = time_sel;
        tool.history_item_sel = item_sel;
    }

    fn perceptual_panes(
        &mut self,
        tool: &mut ToolWindow,
        ui: &mut egui::Ui,
        entries: &[PerceptualEntry],
    ) {
        let mut time_sel = tool.history_time_sel;
        let entry = time_sel.and_then(|i| entries.get(i));
        let width = ui.available_width();
        let height = ui.available_height().max(1.0);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.10, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Times");
                    egui::ScrollArea::vertical()
                        .id_salt("perceptual-times")
                        .show(ui, |ui| {
                            for (i, entry) in entries.iter().enumerate() {
                                if ui
                                    .selectable_label(
                                        time_sel == Some(i),
                                        history::format_time(entry.time_ms),
                                    )
                                    .clicked()
                                {
                                    time_sel = Some(i);
                                }
                            }
                        });
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.90, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label("Recorded data");
                    egui::ScrollArea::both()
                        .id_salt("perceptual-text")
                        .show(ui, |ui| {
                            if let Some(entry) = entry {
                                ui.monospace(&entry.text);
                            }
                        });
                },
            );
        });
        tool.history_time_sel = time_sel;
    }

    /// Apply one display op from the handler to the window list.
    fn apply_window_op(&mut self, op: Vec<Json>) {
        if super::debug::enabled() {
            env_debug!("window op {}", Json::Arr(op.clone()).write());
        }
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
            "image" => {
                let (title, id) = (s(1), s(2));
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    upsert(
                        &mut w.items,
                        WinItem::Image {
                            id,
                            x: n(3),
                            y: n(4),
                            w: n(6),
                            h: n(7),
                            file: s(5),
                        },
                    );
                }
            }
            "eyeloc" => {
                let title = s(1);
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    w.eyeloc = Some((n(3), n(4), color(&s(6))));
                }
            }
            "cleareyeloc" => {
                let title = s(1);
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    w.eyeloc = None;
                }
            }
            "showfinger" => {
                let title = s(1);
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    let hand = s(3);
                    let finger = s(4);
                    w.fingers.retain(|(h, f, ..)| !(h == &hand && f == &finger));
                    w.fingers.push((hand, finger, n(5), n(6), n(7) as i64 == 1));
                }
            }
            "removefinger" => {
                let title = s(1);
                if let Some(w) = self.windows.iter_mut().find(|w| w.title == title) {
                    let hand = s(3);
                    let finger = s(4);
                    w.fingers.retain(|(h, f, ..)| !(h == &hand && f == &finger));
                }
            }
            _ => {}
        }
        if super::debug::enabled() {
            let titles: Vec<&str> = self.windows.iter().map(|w| w.title.as_str()).collect();
            env_debug!("window state titles={titles:?}");
        }
    }

    /// Draw each visible window and forward clicks back to the engine.
    fn render_windows(&mut self, ctx: &egui::Context) {
        let wins = self.windows.clone();
        let mut click: Option<(String, f64, f64)> = None;
        let mut close: Option<String> = None;
        let mut keys: Vec<String> = Vec::new();
        for w in &wins {
            let mut open = true;
            egui::Window::new(&w.title)
                .default_pos([w.x, w.y])
                .resizable(false)
                .collapsible(false)
                .open(&mut open)
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
                            WinItem::Image {
                                x,
                                y,
                                w: iw,
                                h: ih,
                                file,
                                ..
                            } => {
                                let r = egui::Rect::from_min_size(
                                    rect.min + egui::vec2(*x, *y),
                                    egui::vec2(*iw, *ih),
                                );
                                p.rect_filled(
                                    r,
                                    egui::CornerRadius::ZERO,
                                    egui::Color32::from_gray(220),
                                );
                                p.rect_stroke(
                                    r,
                                    egui::CornerRadius::ZERO,
                                    egui::Stroke::new(1.0, egui::Color32::GRAY),
                                    egui::StrokeKind::Inside,
                                );
                                p.text(
                                    r.center(),
                                    egui::Align2::CENTER_CENTER,
                                    file,
                                    egui::FontId::proportional(11.0),
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
                    if let Some((x, y, c)) = w.eyeloc {
                        p.circle_stroke(
                            rect.min + egui::vec2(x, y),
                            5.0,
                            egui::Stroke::new(3.0, c),
                        );
                    }
                    for (hand, finger, x, y, up) in &w.fingers {
                        let label = format!(
                            "{}{}",
                            hand.chars().next().unwrap_or('?'),
                            finger.chars().next().unwrap_or('?')
                        );
                        let color = if *up {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::BROWN
                        };
                        p.circle_filled(rect.min + egui::vec2(*x, *y), 7.0, color);
                        p.text(
                            rect.min + egui::vec2(*x, *y),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(9.0),
                            if *up {
                                egui::Color32::BROWN
                            } else {
                                egui::Color32::WHITE
                            },
                        );
                    }
                    if resp.clicked() {
                        resp.request_focus();
                        if let Some(q) = resp.interact_pointer_pos() {
                            click = Some((
                                w.title.clone(),
                                (q.x - rect.min.x) as f64,
                                (q.y - rect.min.y) as f64,
                            ));
                        }
                    }
                    if resp.has_focus() {
                        ui.input(|input| {
                            for event in &input.events {
                                match event {
                                    egui::Event::Text(text) => {
                                        if text == " " {
                                            keys.push("space".into());
                                        } else if !text.is_empty() {
                                            keys.push(text.clone());
                                        }
                                    }
                                    egui::Event::Key {
                                        key, pressed: true, ..
                                    } => {
                                        if let Some(name) = translate_special_key(*key) {
                                            keys.push(name.to_string());
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        });
                    }
                });
            if !open {
                close = Some(w.title.clone());
            }
        }
        if let Some(title) = close {
            env_debug!("close window {title:?}");
            let _ = self.eval("close-exp-window", vec![Json::Str(title.clone())]);
            self.windows.retain(|w| w.title != title);
        } else if let Some((title, x, y)) = click {
            env_debug!("click {title:?} at {x},{y}");
            self.eval(
                "visible-virtual-window-mouse-click",
                vec![Json::Str(title), Json::Float(x), Json::Float(y)],
            );
        }
        for key in keys {
            self.eval(
                "output-key",
                vec![Json::Arr(vec![Json::Bool(false), Json::Str(key)])],
            );
        }
    }

    /// Drain backend events: trace, model/run signals, and window ops.
    fn drain_events(&mut self) {
        for event in self.backend.take_events() {
            match event {
                BackendEvent::Trace { stream, line } => {
                    if self.trace_visible(stream) {
                        self.trace.push_str(&line);
                        if !line.ends_with('\n') {
                            self.trace.push('\n');
                        }
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
                    self.recorded_history
                        .retain(|(model, _), _| model.as_deref() != Some(name.as_str()));
                }
                BackendEvent::RunStarted { .. } => self.running = true,
                BackendEvent::RunStopped { .. } => {
                    self.running = false;
                    self.refresh_pending = true;
                }
                BackendEvent::WindowOp(op) => self.apply_window_op(op),
                BackendEvent::Status(_) => {}
                BackendEvent::StepperPause { args, .. } => {
                    self.stepper.next = args
                        .first()
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                }
                BackendEvent::StepperUpdate { args, .. } => {
                    self.stepper.last = args
                        .first()
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    self.stepper.items = match args.get(1) {
                        Some(Json::Arr(items)) => items
                            .iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect(),
                        _ => Vec::new(),
                    };
                    self.stepper.tutorable =
                        args.get(2).and_then(|x| x.as_str()).map(str::to_string);
                    self.stepper.selected = None;
                    self.stepper.params.clear();
                    self.stepper.bindings.clear();
                    self.stepper.display.clear();
                    if self.settings.update_when_stepped {
                        for tool in &mut self.tools {
                            tool.refresh_pending = true;
                        }
                    }
                }
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
            if ui.button("Load code...").clicked() {
                self.load_dialog("load-act-r-code");
            }
            if ui.button("Load model...").clicked() {
                self.load_dialog("load-act-r-model");
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Open File…").clicked() {
                self.open_editor();
            }
            if ui.button("New File…").clicked() {
                self.new_editor();
            }
            if ui.button("Save All").clicked() {
                self.save_all_editors();
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Reload").clicked() {
                if self.settings.save_before_reload {
                    self.save_all_editors();
                }
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
        if ui
            .checkbox(&mut self.settings.real_time, "real time")
            .changed()
        {
            self.backend.set_real_time(self.settings.real_time);
            settings::save(&self.settings);
        }
        if ui.button("Options...").clicked() {
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
            if ui.button("Retrieval").clicked() {
                self.open_tool(ToolKind::RetrievalHistory);
            }
            if ui.button("Blending").clicked() {
                self.open_tool(ToolKind::BlendingHistory);
            }
            if ui.button("Buffer").clicked() {
                self.open_tool(ToolKind::BufferHistory);
            }
            if ui.button("Visicon History").clicked() {
                self.open_tool(ToolKind::VisiconHistory);
            }
            if ui.button("Audicon History").clicked() {
                self.open_tool(ToolKind::AudiconHistory);
            }
            if ui.button("Production Grid").clicked() {
                self.open_tool(ToolKind::ProductionGrid);
            }
            if ui.button("Production Graph").clicked() {
                self.open_tool(ToolKind::ProductionGraph);
            }
            if ui.button("Graphic Trace →").clicked() {
                self.open_tool(ToolKind::HorizontalGraphicTrace);
            }
            if ui.button("Graphic Trace ↓").clicked() {
                self.open_tool(ToolKind::VerticalGraphicTrace);
            }
            if ui.button("BOLD Graph").clicked() {
                self.open_tool(ToolKind::BoldGraph);
            }
            if ui.button("BOLD 2D").clicked() {
                self.open_tool(ToolKind::Bold2D);
            }
            if ui.button("BOLD 3D").clicked() {
                self.open_tool(ToolKind::Bold3D);
            }
            if ui.button("Select Buffers").clicked() {
                self.open_tool(ToolKind::BufferPicker);
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
            if ui.button("Close Exp Windows").clicked() {
                let _ = self.eval("close-all-exp-windows", Vec::new());
                self.windows.clear();
            }
            if ui.button("About").clicked() {
                self.open_tool(ToolKind::About);
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

    fn trace_visible(&self, stream: TraceStream) -> bool {
        let index = match stream {
            TraceStream::Model => 0,
            TraceStream::Command => 1,
            TraceStream::Warning => 2,
            TraceStream::General => 3,
        };
        self.trace_filter[index]
    }

    fn trace_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Trace streams:");
            ui.checkbox(&mut self.trace_filter[0], "model");
            ui.checkbox(&mut self.trace_filter[1], "command");
            ui.checkbox(&mut self.trace_filter[2], "warning");
            ui.checkbox(&mut self.trace_filter[3], "general");
        });
        ui.separator();
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
    fn start_stepper(&mut self) {
        if self.stepper.id.is_some() {
            return;
        }
        if !self.stepper.handlers_registered {
            // The callbacks are client-owned commands and the server keeps
            // them after stop-stepper. Re-adding them makes the Lisp command
            // table reject the duplicate, so register them once per
            // connection, as the Tcl Environment does.
            if let Err(e) = self
                .backend
                .register_stepper_handlers("env-next-event", "env-update-event")
            {
                self.error = Some(format!("stepper handlers: {e}"));
                return;
            }
            self.stepper.handlers_registered = true;
        }
        let args = vec![
            Json::Bool(self.stepper.step_all),
            Json::Bool(self.stepper.tutor),
            Json::Str("env-next-event".into()),
            Json::Str("env-update-event".into()),
        ];
        let Some(result) = self.eval("start-stepper", args) else {
            return;
        };
        match result.as_slice() {
            [Json::Bool(true), Json::Str(id), ..] => {
                self.stepper.id = Some(id.clone());
                self.load_stepper_options();
                self.eval("set-stepper-ready", vec![Json::Str(id.clone())]);
            }
            [_, Json::Str(err), ..] => self.error = Some(err.clone()),
            _ => self.error = Some("start-stepper returned an unexpected value".into()),
        }
    }

    fn stop_stepper(&mut self) {
        if let Some(id) = self.stepper.id.take() {
            self.eval("stop-stepper", vec![Json::Str(id)]);
        }
        self.stepper.next.clear();
        self.stepper.last.clear();
        self.stepper.items.clear();
        self.stepper.selected = None;
        self.stepper.params.clear();
        self.stepper.bindings.clear();
        self.stepper.display.clear();
    }

    fn stepper_command(&mut self, cmd: &str, mut args: Vec<Json>) {
        let Some(id) = self.stepper.id.clone() else {
            return;
        };
        args.insert(0, Json::Str(id));
        if let Some(result) = self.eval(cmd, args) {
            if let Some(Json::Str(err)) = result.get(1) {
                if !err.is_empty() {
                    self.error = Some(err.clone());
                }
            }
        }
    }

    /// Load the names shown in the Run Until production/module menus.
    fn load_stepper_options(&mut self) {
        let model = self
            .selected_model
            .clone()
            .or_else(|| self.backend.current_model())
            .or_else(|| self.models.last().cloned());
        let mut productions = model
            .map(|m| self.list_for(Some(m), "all-productions", Vec::new()))
            .unwrap_or_default();
        if self.settings.sort_lists {
            productions.sort();
        }
        let mut modules = self.list_for(None, "sorted-module-names", Vec::new());
        if self.settings.sort_lists {
            modules.sort();
        }
        self.stepper.productions = productions;
        self.stepper.modules = modules;
    }

    /// Send Run Until. Validates the value and turns Step all off so it cannot
    /// silently override the production/module/time filter.
    fn run_until(&mut self) {
        let value = self.stepper.run_value.trim().to_string();
        let kind = stepper_kind(self.stepper.run_type);
        if value.is_empty() {
            self.error = Some("Enter a value for Run until.".into());
            return;
        }
        match self.stepper.run_type.min(STEPPER_RUN_TYPES.len() - 1) {
            0 => {
                if value.parse::<f64>().is_err() {
                    self.error = Some("Step type Time requires a number in seconds.".into());
                    return;
                }
            }
            1 => {
                if !self.stepper.productions.is_empty()
                    && !self
                        .stepper
                        .productions
                        .iter()
                        .any(|p| p.eq_ignore_ascii_case(&value))
                {
                    self.error = Some("Choose a production from the list.".into());
                    return;
                }
            }
            _ => {
                if !self.stepper.modules.is_empty()
                    && !self
                        .stepper
                        .modules
                        .iter()
                        .any(|m| m.eq_ignore_ascii_case(&value))
                {
                    self.error = Some("Choose a module from the list.".into());
                    return;
                }
            }
        }
        if self.stepper.step_all {
            self.stepper.step_all = false;
            self.stepper_command("set-stepper-step-all", vec![Json::Bool(false)]);
        }
        self.stepper_command(
            "step-stepper",
            vec![Json::Bool(false), Json::Str(kind.into()), Json::Str(value)],
        );
    }

    /// A production/module selector, falling back to a text edit when the
    /// backend did not provide a list.
    fn stepper_value_combo(&mut self, ui: &mut egui::Ui, production: bool) {
        let (id, options, hint) = if production {
            (
                "run-production",
                self.stepper.productions.clone(),
                "production name",
            )
        } else {
            ("run-module", self.stepper.modules.clone(), "module name")
        };
        if options.is_empty() {
            ui.add(
                egui::TextEdit::singleline(&mut self.stepper.run_value)
                    .desired_width(120.0)
                    .hint_text(hint),
            );
            return;
        }
        let mut value = self.stepper.run_value.clone();
        let selected = if value.is_empty() {
            "select".to_string()
        } else {
            value.clone()
        };
        egui::ComboBox::from_id_salt(id)
            .selected_text(selected)
            .width(150.0)
            .show_ui(ui, |ui| {
                for name in &options {
                    if ui
                        .selectable_label(value.eq_ignore_ascii_case(name), name)
                        .clicked()
                    {
                        value = name.clone();
                    }
                }
            });
        self.stepper.run_value = value;
    }

    fn stepper_panel(&mut self, _tool: &mut ToolWindow, ui: &mut egui::Ui) {
        if self.stepper.id.is_none() {
            ui.horizontal(|ui| {
                if ui.button("Start stepper").clicked() {
                    self.start_stepper();
                }
                if ui.button("Pause").clicked() {
                    self.stepper.step_all = true;
                    self.start_stepper();
                }
            });
            ui.label("The stepper pauses events before they execute.");
            ui.label("Pause stops on every event, even when trace is off.");
            return;
        }
        ui.horizontal(|ui| {
            if ui.button("Step").clicked() {
                self.stepper_command("step-stepper", Vec::new());
            }
            if ui.button("Stop").clicked() {
                self.stop_stepper();
            }
            let mut chosen_type = None;
            egui::ComboBox::from_id_salt("step-type")
                .selected_text(STEPPER_RUN_TYPES[self.stepper.run_type.min(2)])
                .show_ui(ui, |ui| {
                    for (i, label) in STEPPER_RUN_TYPES.iter().enumerate() {
                        if ui
                            .selectable_label(self.stepper.run_type == i, *label)
                            .clicked()
                        {
                            chosen_type = Some(i);
                        }
                    }
                });
            if let Some(i) = chosen_type {
                if self.stepper.run_type != i {
                    self.stepper.run_type = i;
                    self.stepper.run_value.clear();
                }
            }
            match self.stepper.run_type.min(2) {
                0 => {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.stepper.run_value)
                            .desired_width(90.0)
                            .hint_text("seconds (absolute)"),
                    );
                }
                1 => self.stepper_value_combo(ui, true),
                _ => self.stepper_value_combo(ui, false),
            }
            if ui.button("Run until").clicked() {
                self.run_until();
            }
        });
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut self.stepper.step_all, "Step all")
                .changed()
            {
                self.stepper_command(
                    "set-stepper-step-all",
                    vec![Json::Bool(self.stepper.step_all)],
                );
            }
            if ui.checkbox(&mut self.stepper.tutor, "Tutor mode").changed() {
                self.stepper_command("set-stepper-tutoring", vec![Json::Bool(self.stepper.tutor)]);
            }
        });
        ui.separator();
        ui.label(format!("Last stepped: {}", self.stepper.last));
        ui.label(format!("Next step: {}", self.stepper.next));
        ui.separator();
        let items = self.stepper.items.clone();
        let mut selected = self.stepper.selected;
        let params = self.stepper.params.clone();
        let bindings = self.stepper.bindings.clone();
        let display = self.stepper.display.clone();
        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt("stepper-items")
                .show(&mut cols[0], |ui| {
                    for (i, item) in items.iter().enumerate() {
                        if ui.selectable_label(selected == Some(i), item).clicked() {
                            selected = Some(i);
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("stepper-panes")
                .show(&mut cols[1], |ui| {
                    ui.monospace(&params);
                    ui.separator();
                    ui.monospace(&bindings);
                    ui.separator();
                    ui.monospace(&display);
                });
        });
        if selected != self.stepper.selected {
            self.stepper.selected = selected;
            if let Some(i) = selected {
                if let Some(result) = self.eval("update-stepper", vec![Json::Str(items[i].clone())])
                {
                    self.stepper.params = result
                        .first()
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    self.stepper.bindings = result
                        .get(1)
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    self.stepper.display = result
                        .get(2)
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                }
            }
        }
        if let Some(t) = &self.stepper.tutorable {
            ui.separator();
            ui.label(format!("Tutor: {t}"));
        }
    }

    fn open_editor(&mut self) {
        let mut dlg = rfd::FileDialog::new().add_filter("ACT-R code", &["lisp", "cl"]);
        if let Some(dir) = &self.start_dir {
            dlg = dlg.set_directory(dir);
        }
        if let Some(path) = dlg.pick_file() {
            self.open_editor_path(path);
        }
    }

    fn new_editor(&mut self) {
        let mut dlg = rfd::FileDialog::new().add_filter("ACT-R code", &["lisp", "cl"]);
        if let Some(dir) = &self.start_dir {
            dlg = dlg.set_directory(dir);
        }
        if let Some(path) = dlg.save_file() {
            let _ = std::fs::write(&path, "");
            self.open_editor_path(path);
        }
    }

    fn open_editor_path(&mut self, path: PathBuf) {
        if let Some(editor) = self.editors.iter_mut().find(|e| e.path == path) {
            editor.open = true;
            return;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let id = self.next_editor_id;
        self.next_editor_id += 1;
        self.editors.push(EditorWindow {
            id,
            path,
            text,
            dirty: false,
            open: true,
        });
    }

    fn save_editor(&mut self, editor: &mut EditorWindow) {
        if self.settings.save_backups && editor.path.exists() {
            let mut n = 0u32;
            loop {
                let mut name = editor.path.as_os_str().to_os_string();
                name.push(format!("-backup-{n}"));
                let backup = PathBuf::from(name);
                if !backup.exists() {
                    let _ = std::fs::copy(&editor.path, &backup);
                    break;
                }
                n += 1;
            }
        }
        match std::fs::write(&editor.path, &editor.text) {
            Ok(()) => editor.dirty = false,
            Err(e) => self.error = Some(format!("save {}: {e}", editor.path.display())),
        }
    }

    fn save_all_editors(&mut self) {
        let mut i = 0;
        while i < self.editors.len() {
            let mut editor = self.editors.remove(i);
            if editor.dirty {
                self.save_editor(&mut editor);
            }
            self.editors.insert(i, editor);
            i += 1;
        }
    }

    fn render_editors(&mut self, ctx: &egui::Context) {
        let mut pending_close = None;
        let mut i = 0;
        while i < self.editors.len() {
            let mut editor = self.editors.remove(i);
            let mut open = editor.open;
            let title = format!(
                "{}{}",
                editor.path.display(),
                if editor.dirty { " *" } else { "" }
            );
            let geo_key = format!("editor:{}", editor.path.display());
            let mut win = egui::Window::new(title)
                .id(egui::Id::unique(("editor", editor.id)))
                .open(&mut open)
                .default_width(640.0)
                .default_height(480.0);
            if let Some([x, y, w, h]) = self.settings.tool_geometry.get(&geo_key).copied() {
                win = win.default_pos([x, y]).default_size([w, h]);
            }
            let mut save = false;
            let shown = win.show(ctx, |ui| {
                if self.settings.edit_warn {
                    ui.label("Basic text editor: files are saved as-is; no Lisp evaluation.");
                }
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        save = true;
                    }
                });
                egui::ScrollArea::both()
                    .id_salt(("editor", editor.id))
                    .show(ui, |ui| {
                        if ui
                            .add(
                                egui::TextEdit::multiline(&mut editor.text)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(30)
                                    .code_editor(),
                            )
                            .changed()
                        {
                            editor.dirty = true;
                        }
                    });
            });
            if let Some(inner) = shown {
                let r = inner.response.rect;
                self.settings
                    .tool_geometry
                    .insert(geo_key, [r.min.x, r.min.y, r.width(), r.height()]);
            }
            if save {
                self.save_editor(&mut editor);
            }
            if !open && editor.dirty {
                editor.open = true;
                pending_close = Some(editor.id);
            } else {
                editor.open = open;
            }
            if editor.open {
                self.editors.insert(i, editor);
                i += 1;
            }
        }
        if pending_close.is_some() {
            self.editor_close = pending_close;
        }
        if let Some(id) = self.editor_close {
            let Some(pos) = self.editors.iter().position(|e| e.id == id) else {
                self.editor_close = None;
                return;
            };
            let path = self.editors[pos].path.clone();
            let mut action = None;
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("Save changes to {}?", path.display()));
                    ui.horizontal(|ui| {
                        if ui.button("Save & Close").clicked() {
                            action = Some(0);
                        }
                        if ui.button("Discard").clicked() {
                            action = Some(1);
                        }
                        if ui.button("Cancel").clicked() {
                            action = Some(2);
                        }
                    });
                });
            match action {
                Some(0) => {
                    let mut editor = self.editors.remove(pos);
                    self.save_editor(&mut editor);
                    self.editor_close = None;
                }
                Some(1) => {
                    self.editors.remove(pos);
                    self.editor_close = None;
                }
                Some(2) => self.editor_close = None,
                _ => {}
            }
        }
    }
    fn buffer_picker_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        let buffers = tool.buffers.clone();
        let mut change = None;
        for (i, name) in buffers.iter().enumerate() {
            let mut on = tool.picker_selected.get(i).copied().unwrap_or(false);
            if ui.checkbox(&mut on, name).changed() {
                if i < tool.picker_selected.len() {
                    tool.picker_selected[i] = on;
                }
                change = Some((name.clone(), on));
            }
        }
        if let Some((name, on)) = change {
            let cmd = if on {
                "record-history"
            } else {
                "stop-recording-history"
            };
            self.eval_for(tool.model.clone(), cmd, vec![Json::Str(name)]);
        }
    }

    fn production_grid_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        let data = export::GridData::parse(&tool.history_records);
        ui.horizontal(|ui| {
            ui.label(format!("{} decisions", data.times.len()));
            ui.checkbox(&mut tool.grid_hide_empty, "hide unselected columns");
            if ui.small_button("−").clicked() {
                tool.grid_scale = (tool.grid_scale * 0.5).max(0.5);
            }
            if ui.small_button("+").clicked() {
                tool.grid_scale = (tool.grid_scale * 2.0).min(8.0);
            }
            if ui.button("Save .eps").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Encapsulated PostScript", &["eps"])
                    .save_file()
                {
                    let text = export::grid_eps(
                        &data,
                        tool.grid_hide_empty,
                        &self.settings.p_selected,
                        &self.settings.p_matched,
                        &self.settings.p_mismatched,
                    );
                    if let Err(e) = std::fs::write(&path, text) {
                        self.error = Some(format!("save {}: {e}", path.display()));
                    }
                }
            }
            if ui.button("Save .ps").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PostScript", &["ps"])
                    .save_file()
                {
                    let text = export::grid_ps(
                        &data,
                        tool.grid_hide_empty,
                        &self.settings.p_selected,
                        &self.settings.p_matched,
                        &self.settings.p_mismatched,
                    );
                    if let Err(e) = std::fs::write(&path, text) {
                        self.error = Some(format!("save {}: {e}", path.display()));
                    }
                }
            }
        });
        ui.separator();
        if data.productions.is_empty() || data.times.is_empty() {
            ui.label("No production-history grid data yet.");
            return;
        }
        let columns = data.visible_columns(tool.grid_hide_empty);
        let selected = hex_color(&self.settings.p_selected);
        let matched = hex_color(&self.settings.p_matched);
        let mismatched = hex_color(&self.settings.p_mismatched);
        let cell_w = 34.0 * tool.grid_scale;
        let notes_h = 24.0;
        let grid_h = (ui.available_height() - notes_h - 8.0).max(1.0);
        let mut hover_note = String::new();
        egui::ScrollArea::vertical()
            .id_salt(("production-grid-body", tool.id))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::ScrollArea::both()
                    .id_salt(("production-grid", tool.id))
                    .auto_shrink([false, false])
                    .max_height(grid_h)
                    .show(ui, |ui| {
                        egui::Grid::new(("production-grid-cells", tool.id))
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("");
                                for column in &columns {
                                    ui.label(data.times.get(*column).cloned().unwrap_or_default());
                                }
                                ui.end_row();
                                for (r, name) in data.productions.iter().enumerate() {
                                    ui.label(name);
                                    for column in &columns {
                                        let cell = data
                                            .rows
                                            .get(r)
                                            .and_then(|row| row.get(*column))
                                            .unwrap_or(&export::GridCell::Blank);
                                        let (color, label): (egui::Color32, String) = match cell {
                                            export::GridCell::Selected(label) => {
                                                (selected, label.clone())
                                            }
                                            export::GridCell::Matched(label) => {
                                                (matched, label.clone())
                                            }
                                            export::GridCell::Mismatched(label) => {
                                                (mismatched, label.clone())
                                            }
                                            export::GridCell::Blank => {
                                                (egui::Color32::TRANSPARENT, String::new())
                                            }
                                        };
                                        let (rect, resp) = ui.allocate_exact_size(
                                            egui::vec2(cell_w, 18.0),
                                            egui::Sense::click(),
                                        );
                                        if color != egui::Color32::TRANSPARENT {
                                            ui.painter().rect_filled(rect, 0.0, color);
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                &label,
                                                egui::FontId::proportional(10.0),
                                                egui::Color32::BLACK,
                                            );
                                        }
                                        if resp.hovered() {
                                            hover_note = data
                                                .notes
                                                .get(r)
                                                .and_then(|row| row.get(*column))
                                                .cloned()
                                                .unwrap_or_default();
                                        }
                                        if resp.clicked() {
                                            tool.grid_detail_title = match cell {
                                                export::GridCell::Mismatched(_) => {
                                                    format!("{name} — why not")
                                                }
                                                _ => name.clone(),
                                            };
                                            tool.grid_detail = match cell {
                                                export::GridCell::Selected(_)
                                                | export::GridCell::Matched(_) => data
                                                    .production_text
                                                    .get(r)
                                                    .cloned()
                                                    .unwrap_or_default(),
                                                export::GridCell::Mismatched(index) => data
                                                    .why_not
                                                    .get(index.parse::<usize>().unwrap_or(0))
                                                    .cloned()
                                                    .unwrap_or_else(|| index.clone()),
                                                export::GridCell::Blank => String::new(),
                                            };
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                ui.separator();
                ui.add_sized(
                    egui::vec2(ui.available_width(), notes_h),
                    egui::Label::new(hover_note),
                );
            });
        if !tool.grid_detail.is_empty() {
            let mut open = true;
            egui::Window::new(&tool.grid_detail_title)
                .id(egui::Id::unique(("grid-detail", tool.id)))
                .open(&mut open)
                .default_width(420.0)
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::both()
                        .id_salt(("grid-detail", tool.id))
                        .show(ui, |ui| {
                            ui.monospace(&tool.grid_detail);
                        });
                });
            if !open {
                tool.grid_detail.clear();
                tool.grid_detail_title.clear();
            }
        }
    }
    fn graphic_trace_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        let vertical = tool.kind == ToolKind::VerticalGraphicTrace;
        let data = export::GraphicTraceData::parse(&tool.history_records, vertical);
        if data.rects.is_empty() {
            ui.label("No graphic-trace data yet. Record buffer-trace, run, then Refresh.");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("−").clicked() {
                tool.graphic_scale = (tool.graphic_scale * 0.5).max(0.05);
            }
            if ui.small_button("+").clicked() {
                tool.graphic_scale = (tool.graphic_scale * 2.0).min(16.0);
            }
            ui.checkbox(&mut tool.graphic_hide_text, "Hide text");
            ui.checkbox(&mut tool.graphic_hide_grid, "Hide grid");
            if ui.button("Refresh").clicked() {
                tool.refresh_pending = true;
            }
            if ui.button("Save .eps").clicked() {
                let layout = export::layout_graphic_trace(&data, vertical, tool.graphic_scale);
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Encapsulated PostScript", &["eps"])
                    .save_file()
                {
                    let text = export::graphic_trace_eps(
                        &layout,
                        tool.graphic_hide_text,
                        tool.graphic_hide_grid,
                    );
                    if let Err(e) = std::fs::write(&path, text) {
                        self.error = Some(format!("save {}: {e}", path.display()));
                    }
                }
            }
            if ui.button("Save .ps").clicked() {
                let layout = export::layout_graphic_trace(&data, vertical, tool.graphic_scale);
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PostScript", &["ps"])
                    .save_file()
                {
                    let text = export::graphic_trace_ps(
                        &layout,
                        tool.graphic_hide_text,
                        tool.graphic_hide_grid,
                    );
                    if let Err(e) = std::fs::write(&path, text) {
                        self.error = Some(format!("save {}: {e}", path.display()));
                    }
                }
            }
        });
        let layout = export::layout_graphic_trace(&data, vertical, tool.graphic_scale);
        let mut hovered = None;
        egui::ScrollArea::both()
            .id_salt(("graphic-trace", tool.id))
            .show(ui, |ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(layout.width, layout.height),
                    egui::Sense::hover(),
                );
                let p = ui.painter_at(rect);
                if !tool.graphic_hide_grid {
                    let mut t = 0.0f64;
                    while t <= layout.max_time {
                        let pos = (t * layout.time_scale as f64) as f32;
                        if vertical {
                            let y = rect.min.y + pos + 40.0;
                            p.line_segment(
                                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                                egui::Stroke::new(0.5, egui::Color32::GRAY),
                            );
                            p.text(
                                egui::pos2(rect.min.x + 6.0, y - 2.0),
                                egui::Align2::LEFT_BOTTOM,
                                format!("{:.3}", t / 1000.0),
                                egui::FontId::monospace(9.0),
                                egui::Color32::DARK_GRAY,
                            );
                        } else {
                            let x = rect.min.x + pos + 150.0;
                            p.line_segment(
                                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                                egui::Stroke::new(0.5, egui::Color32::GRAY),
                            );
                            p.text(
                                egui::pos2(x, rect.max.y - 2.0),
                                egui::Align2::LEFT_BOTTOM,
                                format!("{:.3}", t / 1000.0),
                                egui::FontId::monospace(9.0),
                                egui::Color32::DARK_GRAY,
                            );
                        }
                        t += 50.0;
                    }
                }
                for label in &layout.labels {
                    let (r, g, b) = export::graphic_trace_color(label.index);
                    p.text(
                        rect.min + egui::vec2(label.x, label.y),
                        egui::Align2::LEFT_TOP,
                        &label.name,
                        egui::FontId::monospace(11.0),
                        egui::Color32::from_rgb(r, g, b),
                    );
                }
                for (i, item) in layout.boxes.iter().enumerate() {
                    let (r, g, b) = export::graphic_trace_color(item.index);
                    let color = egui::Color32::from_rgb(r, g, b);
                    let box_rect = egui::Rect::from_min_max(
                        rect.min + egui::vec2(item.x1, item.y1),
                        rect.min + egui::vec2(item.x2, item.y2),
                    );
                    p.rect_filled(box_rect, 0.0, color.gamma_multiply(0.55));
                    p.rect_stroke(
                        box_rect,
                        0.0,
                        egui::Stroke::new(1.0, color),
                        egui::StrokeKind::Inside,
                    );
                    if !tool.graphic_hide_text && !item.text.is_empty() && box_rect.width() > 20.0 {
                        p.text(
                            box_rect.min + egui::vec2(2.0, 2.0),
                            egui::Align2::LEFT_TOP,
                            &item.text,
                            egui::FontId::monospace(10.0),
                            egui::Color32::BLACK,
                        );
                    }
                    let response = ui.interact(
                        box_rect,
                        ui.make_persistent_id(("gt-box", tool.id, i)),
                        egui::Sense::hover(),
                    );
                    if response.hovered() {
                        hovered = Some(i);
                    }
                }
            });
        if let Some(i) = hovered {
            if let Some(item) = layout.boxes.get(i) {
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Request:");
                    ui.monospace(if item.request.is_empty() {
                        "-"
                    } else {
                        &item.request
                    });
                    ui.separator();
                    ui.strong("Chunk:");
                    ui.monospace(if item.chunk.is_empty() {
                        "-"
                    } else {
                        &item.chunk
                    });
                    ui.separator();
                    ui.strong("Notes:");
                    ui.monospace(if item.notes.is_empty() {
                        "-"
                    } else {
                        &item.notes
                    });
                });
                ui.horizontal(|ui| {
                    ui.strong("Time:");
                    ui.monospace(format!(
                        "{:.3} - {:.3}",
                        item.time_start / 1000.0,
                        item.time_end / 1000.0
                    ));
                    ui.separator();
                    ui.strong("Duration:");
                    ui.monospace(format!("{:.3}", (item.time_end - item.time_start) / 1000.0));
                });
            }
        }
    }

    fn bold_panel(&mut self, tool: &mut ToolWindow, ui: &mut egui::Ui) {
        let mut times: Vec<f64> = Vec::new();
        let mut rows: Vec<(String, Vec<f64>)> = Vec::new();
        for record in &tool.history_records {
            let Json::Arr(f) = record else { continue };
            let name = f.first().and_then(|x| x.as_str()).unwrap_or("");
            let values: Vec<f64> = f.iter().skip(1).filter_map(|x| x.as_f64()).collect();
            if name.eq_ignore_ascii_case("time") {
                times = values;
            } else {
                rows.push((name.to_string(), values));
            }
        }
        if times.is_empty() || rows.is_empty() {
            ui.label("No BOLD data yet. Record BOLD buffers or buffer-trace, run, then Refresh.");
            return;
        }
        if tool.kind == ToolKind::BoldGraph {
            ui.horizontal(|ui| {
                ui.label("Scale:");
                egui::ComboBox::from_id_salt(("bold-scale", tool.id))
                    .selected_text(["Raw", "Each", "All"][tool.bold_scale.min(2)])
                    .show_ui(ui, |ui| {
                        for (i, label) in ["Raw", "Each", "All"].iter().enumerate() {
                            if ui.selectable_label(tool.bold_scale == i, *label).clicked() {
                                tool.bold_scale = i;
                            }
                        }
                    });
            });
            self.draw_bold_graph(ui, tool, &times, &rows);
        } else {
            let max_scan = times.len().saturating_sub(1);
            if tool.bold_scan > max_scan {
                tool.bold_scan = max_scan;
            }
            ui.add(egui::Slider::new(&mut tool.bold_scan, 0..=max_scan).text("Scan #"));
            ui.separator();
            self.draw_bold_brain(ui, tool, &times, &rows);
        }
    }

    fn draw_bold_graph(
        &self,
        ui: &mut egui::Ui,
        tool: &ToolWindow,
        times: &[f64],
        rows: &[(String, Vec<f64>)],
    ) {
        let global_max = rows
            .iter()
            .flat_map(|(_, v)| v.iter())
            .fold(0.0f64, |m, v| m.max(v.abs()))
            .max(1e-6);
        let width = (times.len() as f32 * 60.0).max(600.0);
        let height = 280.0;
        egui::ScrollArea::horizontal()
            .id_salt(("bold-graph", tool.id))
            .show(ui, |ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                let p = ui.painter_at(rect);
                let base = rect.center().y;
                p.line_segment(
                    [
                        rect.min + egui::vec2(30.0, base - rect.min.y),
                        rect.min + egui::vec2(rect.width() - 10.0, base - rect.min.y),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::GRAY),
                );
                let x_at = |i: usize| {
                    rect.min.x + 40.0 + i as f32 * (rect.width() - 60.0) / times.len().max(1) as f32
                };
                let y_at = |v: f64, scale: f64| (base - (v / scale) as f32 * (height * 0.4)) as f32;
                for (i, t) in times.iter().enumerate() {
                    if i % 5 == 0 {
                        p.text(
                            egui::pos2(x_at(i), rect.max.y - 16.0),
                            egui::Align2::CENTER_TOP,
                            format!("{t:.1}"),
                            egui::FontId::monospace(9.0),
                            egui::Color32::BLACK,
                        );
                    }
                }
                for (r, (name, values)) in rows.iter().enumerate() {
                    let row_max = values.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-6);
                    let scale = match tool.bold_scale {
                        1 => row_max,
                        2 => global_max,
                        _ => 1.0,
                    };
                    let color = palette(r);
                    let mut last = None;
                    for (i, value) in values.iter().enumerate().take(times.len()) {
                        let point = egui::pos2(x_at(i), y_at(*value, scale));
                        if let Some(prev) = last {
                            p.line_segment([prev, point], egui::Stroke::new(1.5, color));
                        }
                        last = Some(point);
                    }
                    p.text(
                        egui::pos2(rect.min.x + 4.0, rect.min.y + 8.0 + r as f32 * 13.0),
                        egui::Align2::LEFT_TOP,
                        name,
                        egui::FontId::monospace(10.0),
                        color,
                    );
                }
            });
    }

    fn draw_bold_brain(
        &self,
        ui: &mut egui::Ui,
        tool: &ToolWindow,
        _times: &[f64],
        rows: &[(String, Vec<f64>)],
    ) {
        const MODULES: [&str; 8] = [
            "manual",
            "goal",
            "vocal",
            "imaginal",
            "retrieval",
            "procedural",
            "aural",
            "visual",
        ];
        let colors = [
            egui::Color32::RED,
            egui::Color32::GREEN,
            egui::Color32::BLUE,
            egui::Color32::YELLOW,
            egui::Color32::MAGENTA,
            egui::Color32::MAGENTA,
            egui::Color32::GREEN,
            egui::Color32::BLUE,
        ];
        let scan = tool.bold_scan;
        ui.label(if tool.kind == ToolKind::Bold3D {
            "Simplified 3D brain view (scan values as voxel grid)"
        } else {
            "2D brain view"
        });
        egui::Grid::new(("bold-brain", tool.id))
            .num_columns(4)
            .show(ui, |ui| {
                for (i, module) in MODULES.iter().enumerate() {
                    let value = rows
                        .iter()
                        .find(|(n, _)| n.eq_ignore_ascii_case(module))
                        .and_then(|(_, v)| v.get(scan))
                        .copied()
                        .unwrap_or(0.0);
                    let shade = value.clamp(0.0, 1.0);
                    let color = colors[i].gamma_multiply(0.25 + 0.75 * shade as f32);
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(120.0, 54.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, color);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{module}\n{value:.2}"),
                        egui::FontId::proportional(11.0),
                        egui::Color32::BLACK,
                    );
                    if i % 4 == 3 {
                        ui.end_row();
                    }
                }
            });
    }
}
/// The viewer's "all" choice maps to ACT-R's `t` detail level.
fn history_detail_arg(detail: &str) -> &str {
    if detail.eq_ignore_ascii_case("all") {
        "t"
    } else {
        detail
    }
}

/// Lisp returns the JSON text `null` once after the last history segment, then
/// drops the id. Rust's own server returns an empty string instead.
fn history_chunk_is_end(chunk: &str) -> bool {
    let chunk = chunk.trim();
    chunk.is_empty() || chunk.eq_ignore_ascii_case("null")
}

fn parse_seconds(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        text.parse().ok()
    }
}

fn history_ms(value: Option<&Json>) -> String {
    value
        .and_then(|x| x.as_f64())
        .map(|ms| format!("{:.3}", ms / 1000.0))
        .unwrap_or_default()
}

fn history_text(value: Option<&Json>) -> String {
    value
        .map(crate::rpc::session::json_to_text)
        .unwrap_or_default()
}

fn history_items(value: Option<&Json>) -> &[Json] {
    match value {
        Some(Json::Arr(items)) => items,
        _ => &[],
    }
}

fn render_history(kind: HistoryKind, records: &[Json]) -> String {
    let mut out = String::new();
    for record in records {
        let Json::Arr(fields) = record else {
            continue;
        };
        if kind != HistoryKind::Trace {
            out.push_str(&format!("Time {}\n", history_ms(fields.first())));
        }
        match kind {
            HistoryKind::Trace => {
                if let Some(Json::Str(line)) = fields.get(3) {
                    out.push_str(line);
                    if !line.ends_with('\n') {
                        out.push('\n');
                    }
                }
            }
            HistoryKind::Retrieval => {
                out.push_str(&format!("Request: {}\n", history_text(fields.get(1))));
                for chunk in history_items(fields.get(2)) {
                    if let Json::Arr(c) = chunk {
                        out.push_str(&format!(
                            "  {} {}\n",
                            history_text(c.first()),
                            history_text(c.get(1))
                        ));
                        if !history_text(c.get(2)).is_empty() {
                            out.push_str(&format!("    {}\n", history_text(c.get(2))));
                        }
                        if !history_text(c.get(3)).is_empty() {
                            out.push_str(&format!("    {}\n", history_text(c.get(3))));
                        }
                    }
                }
            }
            HistoryKind::Blending => {
                out.push_str(&format!("Request: {}\n", history_text(fields.get(1))));
                out.push_str(&format!("Result: {}\n", history_text(fields.get(2))));
                for chunk in history_items(fields.get(3)) {
                    if let Json::Arr(c) = chunk {
                        out.push_str(&format!(
                            "  {} act={} p={}\n",
                            history_text(c.first()),
                            history_text(c.get(1)),
                            history_text(c.get(2))
                        ));
                        if !history_text(c.get(3)).is_empty() {
                            out.push_str(&format!("    {}\n", history_text(c.get(3))));
                        }
                    }
                }
                let trace = history_text(fields.get(4));
                if !trace.is_empty() {
                    out.push_str(&trace);
                    if !trace.ends_with('\n') {
                        out.push('\n');
                    }
                }
            }
            HistoryKind::Buffer => {
                for action in history_items(fields.get(1)) {
                    if let Json::Arr(a) = action {
                        out.push_str(&format!(
                            "{} {}\n  chunk: {}\n  after: {}\n  before: {} {}\n  after:  {} {}\n",
                            history_text(a.first()),
                            history_text(a.get(1)),
                            history_text(a.get(2)),
                            history_text(a.get(3)),
                            history_text(a.get(4)),
                            history_text(a.get(5)),
                            history_text(a.get(6)),
                            history_text(a.get(7)),
                        ));
                    }
                }
            }
            HistoryKind::Visicon | HistoryKind::Audicon => {
                for value in fields.iter().skip(1) {
                    out.push_str(&history_text(Some(value)));
                    out.push('\n');
                }
            }
            HistoryKind::ProductionGrid
            | HistoryKind::GraphicHorizontal
            | HistoryKind::GraphicVertical
            | HistoryKind::BoldGraph
            | HistoryKind::Bold2D
            | HistoryKind::Bold3D => {
                out.push_str(&history_text(Some(record)));
                out.push('\n');
            }
        }
        if kind != HistoryKind::Trace {
            out.push('\n');
        }
    }
    out
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
        self.render_editors(&ctx);
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

    fn load_stepper_model(app: &mut App) {
        let model = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("stepper")
            .join("stepper-model.lisp")
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
    fn stepper_pauses_and_steps_through_the_gui_state() {
        let mut app = test_app();
        app.backend.set_real_time(false);
        let model = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("stepper")
            .join("stepper-model.lisp")
            .to_string_lossy()
            .into_owned();
        app.eval("load-act-r-model", vec![Json::Str(model)]);
        app.open_tool(ToolKind::Declarative);
        if let Some(tool) = app
            .tools
            .iter_mut()
            .find(|t| t.kind == ToolKind::Declarative)
        {
            tool.refresh_pending = false;
        }
        app.open_tool(ToolKind::Stepper);
        assert!(app.stepper.id.is_some(), "stepper should start");

        app.start_run(false);
        assert!(app.backend.is_running(), "run did not start");
        let mut paused = false;
        for _ in 0..200 {
            if app.backend.is_running() {
                let _ = app.backend.poll_run();
            }
            app.drain_events();
            if !app.stepper.next.is_empty() {
                paused = true;
                break;
            }
        }
        assert!(paused, "stepper never paused: {:?}", app.stepper.next);

        app.stepper_command("step-stepper", Vec::new());
        let mut stepped = false;
        for _ in 0..200 {
            if app.backend.is_running() {
                let _ = app.backend.poll_run();
            }
            app.drain_events();
            if !app.stepper.last.is_empty() {
                stepped = true;
                break;
            }
        }
        assert!(stepped, "stepper never produced an update");
        assert!(
            app.tools
                .iter()
                .filter(|t| t.kind == ToolKind::Declarative)
                .all(|t| t.refresh_pending),
            "update_when_stepped did not refresh inspector tools"
        );
        app.stop_stepper();
    }

    #[test]
    fn stepper_defaults_to_time_and_lists_valid_values() {
        let mut app = test_app();
        app.backend.set_real_time(false);
        load_stepper_model(&mut app);
        app.open_tool(ToolKind::Stepper);
        assert!(app.stepper.id.is_some(), "stepper should start");
        assert_eq!(stepper_kind(app.stepper.run_type), "time");
        assert!(
            !app.stepper.step_all,
            "stepper should not start in step-all mode"
        );
        assert!(
            app.stepper
                .productions
                .iter()
                .any(|p| p.eq_ignore_ascii_case("begin")),
            "productions: {:?}",
            app.stepper.productions
        );
        assert!(
            app.stepper
                .modules
                .iter()
                .any(|m| m.eq_ignore_ascii_case("goal")),
            "modules: {:?}",
            app.stepper.modules
        );
        app.stop_stepper();
    }

    #[test]
    fn history_top_height_grows_with_the_window() {
        assert_eq!(history_top_height(300.0), HISTORY_TOP_HEIGHT);
        assert_eq!(history_top_height(600.0), 360.0);
        assert!(history_top_height(600.0) > history_top_height(300.0));
    }

    #[test]
    fn connection_doc_height_grows_and_caps_long_docs() {
        assert_eq!(connection_doc_height(""), 0.0);
        let short = connection_doc_height("one\ntwo");
        let long = connection_doc_height(&"line\n".repeat(100));
        assert!(short > 0.0);
        assert_eq!(long, 164.0);
        assert!(long > short);
    }

    #[test]
    fn run_until_clears_step_all_and_rejects_invalid_values() {
        let mut app = test_app();
        app.backend.set_real_time(false);
        load_stepper_model(&mut app);
        app.open_tool(ToolKind::Stepper);
        assert!(app.stepper.id.is_some(), "stepper should start");

        app.stepper.step_all = true;
        app.stepper.run_type = 0;
        app.stepper.run_value = "1".into();
        app.run_until();
        assert!(!app.stepper.step_all, "Run until should clear Step all");
        assert!(app.error.is_none(), "unexpected error: {:?}", app.error);

        app.stepper.run_type = 2;
        app.stepper.run_value = "goal".into();
        app.run_until();
        assert!(app.error.is_none(), "unexpected error: {:?}", app.error);

        app.stepper.run_type = 1;
        app.stepper.run_value = "begin".into();
        app.run_until();
        assert!(app.error.is_none(), "unexpected error: {:?}", app.error);

        app.stepper.run_type = 2;
        app.stepper.run_value = "1".into();
        app.run_until();
        assert!(app.error.is_some(), "module mode should reject a number");

        app.stepper.run_type = 1;
        app.stepper.run_value = "no-such-production".into();
        app.run_until();
        assert!(
            app.error.is_some(),
            "production mode should reject an unknown name"
        );

        app.stop_stepper();
    }

    #[test]
    fn editor_saves_with_backups() {
        let mut app = test_app();
        let path = std::env::temp_dir().join(format!(
            "actr-env-editor-{}-{}.lisp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, "(define-model old)").unwrap();
        app.open_editor_path(path.clone());
        assert_eq!(app.editors.len(), 1);
        app.editors[0].text = "(define-model new)".into();
        app.editors[0].dirty = true;
        let mut editor = app.editors.remove(0);
        app.save_editor(&mut editor);
        assert!(!editor.dirty);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "(define-model new)"
        );
        let backup = std::path::PathBuf::from(format!("{}-backup-0", path.display()));
        assert!(backup.exists(), "expected a backup at {}", backup.display());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(backup);
    }

    #[test]
    fn window_overlay_ops_render_state() {
        let mut app = test_app();
        app.apply_window_op(vec![
            Json::Str("open".into()),
            Json::Str("W".into()),
            Json::Int(10),
            Json::Int(20),
            Json::Int(300),
            Json::Int(200),
        ]);
        app.apply_window_op(vec![
            Json::Str("image".into()),
            Json::Str("W".into()),
            Json::Str("logo".into()),
            Json::Int(5),
            Json::Int(6),
            Json::Str("logo.gif".into()),
            Json::Int(40),
            Json::Int(20),
        ]);
        app.apply_window_op(vec![
            Json::Str("eyeloc".into()),
            Json::Str("W".into()),
            Json::Str("M".into()),
            Json::Int(30),
            Json::Int(40),
            Json::Int(0),
            Json::Str("red".into()),
        ]);
        app.apply_window_op(vec![
            Json::Str("showfinger".into()),
            Json::Str("W".into()),
            Json::Str("M".into()),
            Json::Str("left".into()),
            Json::Str("index".into()),
            Json::Int(50),
            Json::Int(60),
            Json::Int(1),
        ]);
        let win = &app.windows[0];
        assert!(win
            .items
            .iter()
            .any(|i| matches!(i, WinItem::Image { file, .. } if file == "logo.gif")));
        assert!(win.eyeloc.is_some());
        assert_eq!(win.fingers.len(), 1);
        app.apply_window_op(vec![
            Json::Str("removefinger".into()),
            Json::Str("W".into()),
            Json::Str("M".into()),
            Json::Str("left".into()),
            Json::Str("index".into()),
        ]);
        assert!(app.windows[0].fingers.is_empty());
        assert_eq!(translate_special_key(egui::Key::Enter), Some("return"));
        assert_eq!(translate_special_key(egui::Key::ArrowUp), Some("up-arrow"));
    }
    #[test]
    fn production_graph_tool_records_history() {
        let mut app = test_app();
        load_count(&mut app);
        app.open_tool(ToolKind::ProductionGraph);
        assert_eq!(app.tools.len(), 1);
        assert!(app.tools[0].history_recording);
        assert!(app
            .recorded_history
            .contains_key(&(Some("COUNT".to_string()), "production-history".to_string())));

        app.acquire_history(Some("COUNT".to_string()), "production-history");
        app.release_history(Some("COUNT".to_string()), "production-history");
        assert!(app
            .recorded_history
            .contains_key(&(Some("COUNT".to_string()), "production-history".to_string())));
        app.release_history(Some("COUNT".to_string()), "production-history");
        assert!(!app
            .recorded_history
            .contains_key(&(Some("COUNT".to_string()), "production-history".to_string())));
    }

    #[test]
    fn command_doc_and_trace_filters() {
        let mut app = test_app();
        assert!(app.backend.command_doc("run").unwrap().is_some());
        assert!(app.backend.command_doc("not-a-command").unwrap().is_none());
        assert!(app.trace_visible(TraceStream::Model));
        app.trace_filter[2] = false;
        assert!(!app.trace_visible(TraceStream::Warning));
    }

    #[test]
    fn production_color_hex_parsing() {
        assert_eq!(
            hex_color("#44da22"),
            egui::Color32::from_rgb(0x44, 0xda, 0x22)
        );
        assert_eq!(
            hex_color("fca31d"),
            egui::Color32::from_rgb(0xfc, 0xa3, 0x1d)
        );
        assert_eq!(hex_color("bad"), egui::Color32::GRAY);
    }
    #[test]
    fn history_renderers_cover_the_tcl_shapes() {
        let trace = Json::Arr(vec![
            Json::Int(100),
            Json::Str("low".into()),
            Json::Null,
            Json::Str("chunk line\n".into()),
        ]);
        assert_eq!(render_history(HistoryKind::Trace, &[trace]), "chunk line\n");

        let retrieval = Json::Arr(vec![
            Json::Int(250),
            Json::Str("+retrieval>".into()),
            Json::Arr(vec![Json::Arr(vec![
                Json::Str("CHUNK0".into()),
                Json::Str("chunk text".into()),
                Json::Str("params".into()),
                Json::Str("activation".into()),
            ])]),
        ]);
        let text = render_history(HistoryKind::Retrieval, &[retrieval]);
        assert!(text.contains("Time 0.250"), "retrieval: {text}");
        assert!(text.contains("chunk text"), "retrieval: {text}");

        let blending = Json::Arr(vec![
            Json::Int(300),
            Json::Str("+blending>".into()),
            Json::Str("result".into()),
            Json::Arr(vec![Json::Arr(vec![
                Json::Str("CHUNK0".into()),
                Json::Float(1.5),
                Json::Float(0.5),
                Json::Str("printed".into()),
            ])]),
            Json::Str("blend trace\n".into()),
        ]);
        let text = render_history(HistoryKind::Blending, &[blending]);
        assert!(text.contains("blend trace"), "blending: {text}");

        let buffer = Json::Arr(vec![
            Json::Int(400),
            Json::Arr(vec![Json::Arr(vec![
                Json::Str("goal".into()),
                Json::Str("set-buffer-chunk".into()),
                Json::Str("chunk".into()),
                Json::Str("free".into()),
                Json::Str("old".into()),
                Json::Str("free".into()),
                Json::Str("new".into()),
                Json::Str("busy".into()),
            ])]),
        ]);
        let text = render_history(HistoryKind::Buffer, &[buffer]);
        assert!(text.contains("set-buffer-chunk"), "buffer: {text}");

        let visicon = Json::Arr(vec![Json::Int(500), Json::Str("TEXT0".into())]);
        assert!(render_history(HistoryKind::Visicon, &[visicon]).contains("TEXT0"));
    }

    #[test]
    fn opening_a_history_tool_starts_recording() {
        let mut app = test_app();
        load_count(&mut app);
        app.open_tool(ToolKind::RetrievalHistory);
        assert_eq!(app.tools.len(), 1);
        assert!(app.tools[0].history_recording);
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

    #[test]
    fn history_stream_ends_on_lisp_null_or_rust_empty_chunk() {
        assert!(history_chunk_is_end("null"));
        assert!(history_chunk_is_end(" NULL \n"));
        assert!(history_chunk_is_end(""));
        assert!(!history_chunk_is_end("[]"));
        assert!(!history_chunk_is_end(r#"[[0,"LOW","COUNT","line"]]"#));
    }

    #[test]
    fn history_detail_all_maps_to_lisp_t() {
        assert_eq!(history_detail_arg("all"), "t");
        assert_eq!(history_detail_arg("ALL"), "t");
        assert_eq!(history_detail_arg("low"), "low");
        assert_eq!(history_detail_arg("medium"), "medium");
        assert_eq!(history_detail_arg("high"), "high");
    }
}

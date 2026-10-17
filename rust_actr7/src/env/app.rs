//! The egui application shell: control panel, trace pane, and the Phase 2
//! read-only inspectors (declarative, buffers, procedural).

use std::path::PathBuf;

use eframe::egui;

use crate::json::Json;

use super::backend::{result_text, EnvBackend};

/// The central-area views.
#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    Trace,
    Declarative,
    Buffers,
    Procedural,
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
}

impl App {
    pub fn new(backend: Box<dyn EnvBackend>, start_dir: Option<PathBuf>) -> App {
        App {
            backend,
            start_dir,
            trace: String::new(),
            command: String::new(),
            result: String::new(),
            run_secs: 10.0,
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

    fn run(&mut self, name: &str) {
        self.eval(name, vec![Json::Float(self.run_secs)]);
        self.drain_trace();
        self.refresh_pending = true;
    }

    fn load_dialog(&mut self) {
        let mut dlg = rfd::FileDialog::new()
            .add_filter("ACT-R model", &["lisp", "cl"])
            .add_filter("All files", &["*"]);
        if let Some(dir) = &self.start_dir {
            dlg = dlg.set_directory(dir);
        }
        if let Some(path) = dlg.pick_file() {
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
                self.buf_sel = None;
                self.buf_detail.clear();
            }
            Tab::Procedural => {
                self.prods = self.list("all-productions", Vec::new());
                self.prod_sel = None;
                self.prod_detail.clear();
            }
        }
    }

    fn refresh_dm_list(&mut self) {
        let filter = self.dm_filter.clone();
        self.dm_chunks = self.list("filter-dm-chunks", vec![Json::Str(filter)]);
        self.dm_sel = None;
        self.dm_detail.clear();
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Controls");
        ui.separator();
        if ui.button("Load model…").clicked() {
            self.load_dialog();
        }
        if ui.button("Reset").clicked() {
            self.eval("reset", Vec::new());
            self.drain_trace();
            self.refresh_pending = true;
        }
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
        if ui.button("Run").clicked() {
            self.run("run");
        }
        if ui.button("Run full time").clicked() {
            self.run("run-full-time");
        }
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

    fn central(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Trace, "Trace");
            ui.selectable_value(&mut self.tab, Tab::Declarative, "Declarative");
            ui.selectable_value(&mut self.tab, Tab::Buffers, "Buffers");
            ui.selectable_value(&mut self.tab, Tab::Procedural, "Procedural");
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
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_trace();

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

        egui::Panel::left("controls")
            .resizable(true)
            .default_size(200.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.controls(ui));
            });

        egui::CentralPanel::default().show(ui, |ui| self.central(ui));
    }
}

//! The egui application shell: control panel, command box, and trace pane.
//!
//! Phase 1 provides the everyday loop — load, reset, run — plus a live trace
//! view. The inspector panels (Phase 2) and the visible experiment window
//! (Phase 3) hang off the same shell.

use std::path::PathBuf;

use eframe::egui;

use crate::json::Json;

use super::backend::{result_text, EnvBackend};

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

    fn run(&mut self, name: &str) {
        self.eval(name, vec![Json::Float(self.run_secs)]);
        self.drain_trace();
    }

    fn load_dialog(&mut self) {
        let mut dlg = rfd::FileDialog::new()
            .add_filter("ACT-R model", &["lisp", "cl", "lisp"])
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
        }
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
                    .desired_width(160.0)
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

        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Trace");
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
        });
    }
}

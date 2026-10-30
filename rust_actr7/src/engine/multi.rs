//! Multi-model meta-process: one independent `Engine` per `define-model`.
use super::*;
use std::cell::Cell;
use std::cmp::Reverse;
use std::rc::Rc;

/// A model-name column is fixed at ten characters by the multi-model trace
/// formatter; `unit-1-together-1-mp` pins it for COUNT/SEMANTIC/ADDITION.
const MODEL_WIDTH: usize = 10;

/// Run several models against one shared clock and emit the interleaved trace.
pub struct MultiEngine {
    engines: Vec<Engine>,
    names: Vec<String>,
    width: usize,
    time_ms: i64,
    lines: Vec<String>,
}

impl MultiEngine {
    pub fn new(
        models: Vec<Model>,
        seed: (u64, u64),
        device: DeviceSetup,
        width: usize,
    ) -> Result<Self, EngineError> {
        let mut engines = Vec::with_capacity(models.len());
        let mut names = Vec::with_capacity(models.len());
        let mut lines = Vec::new();
        let shared_seq = Rc::new(Cell::new(0u64));
        for model in models {
            let name = model.name.clone();
            let mut engine = Engine::new_with_width(model, seed, 0.0, device.clone(), width)?;
            engine.set_trace(true);
            engine.set_module_width(width);
            engine.use_shared_sequence(shared_seq.clone());
            for line in engine.out_since(0) {
                lines.push(with_model_column(&line, &name));
            }
            names.push(name);
            engines.push(engine);
        }
        Ok(Self {
            engines,
            names,
            width,
            time_ms: 0,
            lines,
        })
    }

    pub fn set_trace(&mut self, on: bool) {
        for engine in &mut self.engines {
            engine.set_trace(on);
        }
    }

    pub fn set_module_width(&mut self, width: usize) {
        self.width = width;
        for engine in &mut self.engines {
            engine.set_module_width(width);
        }
    }

    pub fn run_secs(&mut self, secs: f64) -> Result<(), EngineError> {
        let start = self.time_ms;
        let limit = start + (secs * 1000.0).round() as i64;
        for engine in &mut self.engines {
            engine.set_time_ms(start);
        }
        let mut beyond_limit = false;
        loop {
            let mut best: Option<(i64, Reverse<i64>, u64, usize)> = None;
            for (index, engine) in self.engines.iter_mut().enumerate() {
                let Some((time, priority, seq)) = engine.next_event_key() else {
                    continue;
                };
                if time > limit {
                    beyond_limit = true;
                    continue;
                }
                let key = (time, Reverse(priority), seq, index);
                if best
                    .as_ref()
                    .map_or(true, |b| (key.0, key.1, key.2) < (b.0, b.1, b.2))
                {
                    best = Some(key);
                }
            }
            let Some((_, _, _, index)) = best else {
                break;
            };
            let mark = self.engines[index].out_len();
            if !self.engines[index].run_one_event()? {
                break;
            }
            for line in self.engines[index].out_since(mark) {
                self.lines
                    .push(with_model_column(&line, &self.names[index]));
            }
        }
        let stop = if beyond_limit {
            limit
        } else {
            self.engines
                .iter()
                .map(Engine::time_ms)
                .max()
                .unwrap_or(start)
        };
        self.time_ms = stop;
        for engine in &mut self.engines {
            engine.set_time_ms(stop);
        }
        let reason = if beyond_limit {
            "Stopped because time limit reached"
        } else {
            "Stopped because no events left to process"
        };
        let raw = format_line(stop, "------", reason, self.width);
        self.lines.push(with_model_column(&raw, "-"));
        Ok(())
    }

    pub fn output(&self) -> String {
        if self.lines.is_empty() {
            return String::new();
        }
        let mut out = self.lines.join("\n");
        out.push('\n');
        out
    }

    pub fn out_len(&self) -> usize {
        self.lines.len()
    }

    pub fn out_since(&self, start: usize) -> Vec<String> {
        self.lines.get(start..).unwrap_or(&[]).to_vec()
    }
}

fn with_model_column(line: &str, model: &str) -> String {
    // A trace line is `     0.000   MODULE ...`; raw `model-output` lines have
    // no timestamp and stay unmodified.
    if line.len() <= 13 || line.as_bytes().get(6) != Some(&b'.') {
        return line.to_string();
    }
    format!("{}  {:<MODEL_WIDTH$}{}", &line[..10], model, &line[13..])
}

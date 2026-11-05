//! Multi-model meta-process: one independent `Engine` per `define-model`.
use super::*;
use std::cell::Cell;
use std::cmp::Reverse;
use std::rc::Rc;

pub(crate) fn model_column_width(widths: impl IntoIterator<Item = usize>) -> usize {
    widths.into_iter().max().unwrap_or(0) + 2
}

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
        let model_width = model_column_width(models.iter().map(|m| m.name.len()));
        for model in models {
            let name = model.name.clone();
            let mut engine = Engine::new_with_width(model, seed, 0.0, device.clone(), width)?;
            engine.set_trace(true);
            engine.set_module_width(width);
            engine.use_shared_sequence(shared_seq.clone());
            for line in engine.out_since(0) {
                lines.push(with_model_column(&line, &name, model_width));
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
        let model_width = model_column_width(self.names.iter().map(String::len));
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
                    .push(with_model_column(&line, &self.names[index], model_width));
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
        self.lines.push(with_model_column(&raw, "-", model_width));
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

/// Run a set of already-created engines on the shared clock. The RPC session
/// owns the selected engine separately from its inactive models, so it appends
/// to its own global trace buffer instead of constructing a `MultiEngine`.
pub(crate) fn run_models(
    engines: &mut [(&str, &mut Engine)],
    secs: f64,
    width: usize,
    time_ms: &mut i64,
    trace: &mut Vec<String>,
    kinds: &mut Vec<TraceKind>,
    levels: &mut Vec<Out>,
) -> Result<u64, EngineError> {
    let start = *time_ms;
    let limit = start + (secs * 1000.0).round() as i64;
    let model_width = model_column_width(engines.iter().map(|(name, _)| name.len()));
    for (_, engine) in engines.iter_mut() {
        engine.set_time_ms(start);
        engine.set_module_width(width);
    }
    let mut beyond_limit = false;
    let mut events = 0u64;
    loop {
        let mut best: Option<(i64, Reverse<i64>, u64, usize)> = None;
        for (index, (_, engine)) in engines.iter_mut().enumerate() {
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
        let (name, engine) = &mut engines[index];
        let mark = engine.out_len();
        if !engine.run_one_event()? {
            break;
        }
        events += 1;
        let lines = engine.out_since(mark);
        let line_kinds = engine.out_kinds_since(mark);
        let line_levels = engine.out_levels_since(mark);
        for (i, line) in lines.into_iter().enumerate() {
            trace.push(with_model_column(&line, name, model_width));
            kinds.push(line_kinds.get(i).copied().unwrap_or(TraceKind::Model));
            levels.push(line_levels.get(i).copied().unwrap_or(Out::Low));
        }
    }
    let stop = if beyond_limit {
        limit
    } else {
        engines
            .iter()
            .map(|(_, e)| e.time_ms())
            .max()
            .unwrap_or(start)
    };
    *time_ms = stop;
    for (_, engine) in engines.iter_mut() {
        engine.set_time_ms(stop);
    }
    let reason = if beyond_limit {
        "Stopped because time limit reached"
    } else {
        "Stopped because no events left to process"
    };
    let raw = format_line(stop, "------", reason, width);
    trace.push(with_model_column(&raw, "-", model_width));
    kinds.push(TraceKind::Model);
    levels.push(Out::Low);
    Ok(events)
}

pub(crate) fn with_model_column(line: &str, model: &str, width: usize) -> String {
    // A trace line is `     0.000   MODULE ...`; raw `model-output` lines have
    // no timestamp and stay unmodified.
    if line.len() <= 13 || line.as_bytes().get(6) != Some(&b'.') {
        return line.to_string();
    }
    format!("{}  {model:<width$}{}", &line[..10], &line[13..])
}

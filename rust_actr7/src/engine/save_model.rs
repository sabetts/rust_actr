//! `save-model-file` serialization (`extras/save-model`).
//!
//! The Lisp command writes a loadable model definition by temporarily
//! redirecting the command trace to a file and then re-reading it. The engine
//! has no command-trace file, so this module builds the same lines directly
//! from the live model and chunk state.

use super::*;

impl Engine {
    /// Append the `save-model-file` text to the trace. `timestamp` is the
    /// already-formatted `YYYY/MM/DD HH:MM:SS` header value; callers that do
    /// not need a reproducible test can pass `None` for the current wall clock.
    pub fn save_model(&mut self, timestamp: Option<&str>) {
        let stamp = timestamp
            .map(str::to_string)
            .unwrap_or_else(current_timestamp);
        let mut lines = Vec::new();

        lines.push(format!(
            ";;; Saved version of model {} at run time {} on {}",
            self.model.name,
            lisp_s_f32(self.time_ms as f32 / 1000.0),
            stamp
        ));
        lines.push(String::new());
        lines.push("(clear-all)".to_string());
        lines.push(String::new());
        lines.push(format!("(define-model {}-saved", self.model.name));
        lines.push(String::new());
        lines.push("(sgp ".to_string());
        lines.push(")".to_string());
        lines.push(String::new());
        let (seed, draws) = self.rng.borrow().state();
        lines.push(format!(";;; (sgp :seed ({seed} {draws}))"));
        lines.push(String::new());

        lines.push(";PRE-CHUNK-TYPE-HOOK".to_string());
        for ct in &self.model.chunk_types {
            lines.push(format!("(chunk-type {} ", ct.name));
            for slot in &ct.slots {
                lines.push(format!("  {slot}"));
            }
            lines.push(")".to_string());
        }
        lines.push(";PRE-CHUNK-HOOK".to_string());

        let dm_names: Vec<String> = self.dm.iter().filter_map(|c| c.name.clone()).collect();
        let dm_set: HashSet<String> = dm_names.iter().cloned().collect();
        let mut other: Vec<ChunkInst> = self
            .all_named_chunks()
            .into_values()
            .filter(|c| {
                let name = c.name.as_deref().unwrap_or_default();
                !dm_set.contains(name) && !is_default_chunk(name)
            })
            .collect();
        other.sort_by(|a, b| a.name.cmp(&b.name));
        let mut dm: Vec<ChunkInst> = self.dm.clone();
        dm.sort_by(|a, b| a.name.cmp(&b.name));

        lines.push("(define-chunks ".to_string());
        for chunk in other.iter().chain(dm.iter()) {
            push_saved_chunk(&mut lines, chunk, &self.model);
        }
        lines.push(")".to_string());

        let mut dm_sorted = dm_names;
        dm_sorted.sort();
        lines.push("(add-dm-chunks".to_string());
        for name in &dm_sorted {
            lines.push(format!("   {name}"));
        }
        lines.push(")".to_string());
        lines.push(String::new());
        lines.push(String::new());
        lines.push(String::new());
        lines.push(";PRE-PRODUCTION-HOOK".to_string());

        let mark = self.out_lines.len();
        self.print_productions(&[]);
        let mut productions = self.out_lines.split_off(mark);
        lines.append(&mut productions);
        lines.push(String::new());
        lines.push(String::new());

        for (idx, prod) in self.model.productions.iter().enumerate() {
            let at = prod.at.unwrap_or(self.cfg.dat_ms as f32 / 1000.0);
            lines.push(format!(
                "(spp {} :u {} :at {})",
                prod.name,
                lisp_s_f32(self.utilities[idx]),
                lisp_s_f32(at)
            ));
        }
        lines.push(String::new());
        lines.push(String::new());
        if let Some(goal) = self.buffer_read("GOAL") {
            lines.push(format!("(goal-focus {goal})"));
        }
        lines.push(";END-HOOK".to_string());
        lines.push(")".to_string());

        self.out_lines.extend(lines);
    }
}

impl Engine {
    /// Every chunk ACT-R's `(chunks)` would list: `define-chunks`, DM, live
    /// buffer contents, and the visual/copy aliases that only live in the
    /// module maps.
    fn all_named_chunks(&self) -> HashMap<String, ChunkInst> {
        let mut map: HashMap<String, ChunkInst> = HashMap::new();
        for (key, chunk) in &self.named {
            let name = chunk.name.clone().unwrap_or_else(|| key.clone());
            map.insert(name, chunk.clone());
        }
        for chunk in &self.dm {
            if let Some(name) = &chunk.name {
                map.insert(name.clone(), chunk.clone());
            }
        }
        for buffer in self.buffers.values() {
            if let Some(chunk) = &buffer.chunk {
                if let Some(name) = &chunk.name {
                    map.insert(name.clone(), chunk.clone());
                }
            }
        }
        // Reusable buffer chunks survive a clear in ACT-R's chunk table. The
        // engine keeps their name but not their instance, so recover the
        // content from the matching `-0` DM copy.
        for name in self.buffer_reuse.values() {
            if map.contains_key(name) {
                continue;
            }
            if let Some(base) = self
                .dm
                .iter()
                .find(|c| {
                    c.name
                        .as_deref()
                        .is_some_and(|n| n.starts_with(&format!("{name}-")))
                })
                .cloned()
            {
                let mut c = base;
                c.name = Some(name.clone());
                map.insert(name.clone(), c);
            }
        }
        // Copies and merged aliases are not stored as `ChunkInst`s; rebuild
        // them from their source. When the source itself was a transient
        // buffer copy, use the matching DM `-0` chunk for both names.
        for _ in 0..3 {
            let mut pending: Vec<(String, ChunkInst)> = Vec::new();
            for (copy, source) in self
                .copy_source
                .iter()
                .chain(&self.vl_copied_from)
                .chain(&self.chunk_alias)
            {
                if map.contains_key(copy) {
                    continue;
                }
                let source_chunk = map.get(source).cloned().or_else(|| {
                    let prefix = format!("{source}-");
                    let prefix_copy = format!("{copy}-");
                    self.dm
                        .iter()
                        .find(|c| {
                            c.name.as_deref().is_some_and(|n| {
                                n.starts_with(&prefix) || n.starts_with(&prefix_copy)
                            })
                        })
                        .cloned()
                });
                if let Some(src) = source_chunk {
                    if !map.contains_key(source) && !copy.starts_with(source.as_str()) {
                        let mut base = src.clone();
                        base.name = Some(source.clone());
                        pending.push((source.clone(), base));
                    }
                    let mut c = src;
                    c.name = Some(copy.clone());
                    pending.push((copy.clone(), c));
                }
            }
            let before = map.len();
            map.extend(pending);
            if map.len() == before {
                break;
            }
        }
        map
    }
}

fn is_default_chunk(name: &str) -> bool {
    matches!(
        name.to_uppercase().as_str(),
        "BUSY"
            | "CLEAR"
            | "EMPTY"
            | "ERROR"
            | "FAILURE"
            | "FREE"
            | "FULL"
            | "REQUESTED"
            | "UNREQUESTED"
    )
}

fn push_saved_chunk(lines: &mut Vec<String>, chunk: &ChunkInst, model: &crate::model::Model) {
    let name = chunk.name.clone().unwrap_or_default();
    lines.push(format!("  ({name}"));
    lines.push("    ISA CHUNK".to_string());
    let index = model.slot_index();
    let mut slots: Vec<usize> = (0..chunk.slots.len()).collect();
    slots.sort_by_key(|&i| {
        *index
            .get(&chunk.slots[i].0.to_uppercase())
            .unwrap_or(&usize::MAX)
    });
    let mut printed = Vec::new();
    for i in slots {
        let (slot, value) = &chunk.slots[i];
        if slot.eq_ignore_ascii_case("ISA") || *value == Value::Nil {
            continue;
        }
        printed.push(format!("     {slot}  {}", value.show()));
    }
    if let Some(last) = printed.last_mut() {
        last.push(')');
    } else {
        lines.push(")".to_string());
        return;
    }
    lines.append(&mut printed);
}

fn current_timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let day_secs = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = day_secs / 3600;
    let min = (day_secs % 3600) / 60;
    let sec = day_secs % 60;
    format!("{year:04}/{month:02}/{day:02} {hour:02}:{min:02}:{sec:02}")
}

/// Howard Hinnant's civil-from-days algorithm, for a dependency-free UTC
/// fallback when the caller does not supply a timestamp.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + i64::from(m <= 2), m, d)
}

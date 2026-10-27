//! Task drivers (output, goal-focus, zz/al/zbrodoff drivers).
use super::compilation::{show_pat, slot_disp};
use super::dm::{al_history, al_needed_refs, bl_equation, exp_coerced, log_coerced};
use super::rhs::new_chunk;
use super::*;
use crate::sexpr::Sexp;
use crate::{OnehitMode, ZbrodoffMode};

#[derive(Clone)]
struct ZbrodoffTrial {
    block: usize,
    addend: usize,
    text: String,
    answer: String,
}

struct ZbrodoffAnalysis {
    blocks: usize,
    rts: Vec<f64>,
    counts: Vec<u32>,
    totals: [u32; 3],
}

impl Engine {
    /// All trace emitted so far, newline-terminated (empty if none).
    pub fn output(&self) -> String {
        if self.out_lines.is_empty() {
            return String::new();
        }
        let mut s = self.out_lines.join("\n");
        s.push('\n');
        s
    }

    /// Number of trace lines emitted so far (the RPC server streams the tail
    /// to `model-trace` monitors).
    pub fn out_len(&self) -> usize {
        self.out_lines.len()
    }

    /// Replace the trace history (`build_engine` restores it after an
    /// in-place rebuild, which otherwise starts from an empty engine).
    pub fn set_out_lines(&mut self, lines: Vec<String>) {
        self.out_lines = lines;
    }

    /// Drop the trace lines from `len` on (`no-output` binds the output
    /// stream to nothing in ACT-R, so the suppressed command's lines never
    /// exist; this is the equivalent).
    pub fn truncate_out(&mut self, len: usize) {
        self.out_lines.truncate(len);
    }

    /// Take the commands the model fired since the last call, as
    /// `(name, model, key)`. The RPC session dispatches these to the client's
    /// monitors (ACT-R calls `output-key` mid-run, so a task's handler has
    /// run by the time `run` returns).
    pub fn take_fired_commands(&mut self) -> Vec<(String, String, String)> {
        std::mem::take(&mut self.fired_commands)
    }

    /// The trace lines emitted since index `start`.
    pub fn out_since(&self, start: usize) -> Vec<String> {
        self.out_lines.get(start..).unwrap_or(&[]).to_vec()
    }

    /// `(goal-focus <spec>)`: place a chunk in the goal buffer at the
    /// current time (`core-modules/goal.lisp:275`, priority `:max`,
    /// `requested nil`). Any armed conflict-resolution wakes on the buffer
    /// change via [`Engine::changed`], matching the Lisp trace.
    pub fn goal_focus(&mut self, desc: &ChunkDesc) -> Result<(), EngineError> {
        if self.cfg.threads {
            let inst = match desc {
                ChunkDesc::Named(n) => {
                    // Copy the named chunk (`START-GOAL` -> `START-GOAL-0`).
                    let copy = self.copy_chunk(n);
                    self.named
                        .get(&copy)
                        .cloned()
                        .or_else(|| {
                            self.dm
                                .iter()
                                .find(|c| c.name.as_deref() == Some(copy.as_str()))
                                .cloned()
                        })
                        .ok_or_else(|| bad(format!("no chunk named {n}")))?
                }
                ChunkDesc::Spec { .. } => {
                    let nc = new_chunk(desc)?;
                    let mut c = self.instantiate("GOAL", nc)?;
                    // The threads extra defines each goal chunk via
                    // `define-chunks-fct`, which auto-names it by type
                    // (`(isa initial ...)` -> `INITIAL0`).
                    let ctype = c.ctype.clone();
                    let k = *self.name_count.entry(ctype.clone()).or_insert(0);
                    self.name_count.insert(ctype.clone(), k + 1);
                    c.name = Some(format!("{ctype}{k}"));
                    c
                }
            };
            let first = self.goal_set.is_empty();
            self.goal_set.push(inst.clone());
            if first {
                self.now(
                    Prio::Max,
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "GOAL".to_string(),
                        chunk: NewChunk::Instance(inst),
                        requested: false,
                        shown: None,
                    },
                );
            } else {
                self.now(
                    Prio::Max,
                    Out::Low,
                    Event::OverwriteBuffer {
                        buffer: "GOAL".to_string(),
                        chunk: NewChunk::Instance(inst),
                        requested: false,
                        shown: None,
                        module: "GOAL".to_string(),
                    },
                );
            }
            // `goal-focus-fct` also schedules a `clear-delayed-goal`
            // maintenance event (`goal.lisp:284`). It has no state effect here
            // but counts toward `run-n-events`.
            self.now_m(Prio::Max, Out::Never, true, Event::Dummy);
            return Ok(());
        }
        let chunk = new_chunk(desc)?;
        self.now(
            Prio::Max,
            Out::Low,
            Event::SetBuffer {
                buffer: "GOAL".to_string(),
                chunk,
                requested: false,
                shown: None,
            },
        );
        // `goal-focus-fct` also schedules a `clear-delayed-goal` maintenance
        // event (`goal.lisp:284`); it counts toward `run-n-events`.
        self.now_m(Prio::Max, Out::Never, true, Event::Dummy);
        Ok(())
    }

    /// `summarize-tests`/`display-data` (`test-model.lisp:84`): print one
    /// `#####` block per recorded `:test` marker.
    pub(super) fn zz_summarize(&mut self) {
        let data = std::mem::take(&mut self.zz_data);
        let mut i = 0;
        while i < data.len() {
            if data[i].0 == "TEST" {
                let details = data[i].2.clone();
                let mut j = i + 1;
                while j < data.len() && data[j].0 != "TEST" {
                    j += 1;
                }
                let events: Vec<(String, i64, String)> = data[i + 1..j].to_vec();
                self.zz_display_data(&details, &events);
                i = j;
            } else {
                i += 1;
            }
        }
    }

    /// `display-data` (`test-model.lisp:91`): the `#####` summary block.
    pub(super) fn zz_display_data(&mut self, details: &str, events: &[(String, i64, String)]) {
        self.out_lines.push("#####".to_string());
        self.out_lines.push(format!("\"{details}\""));
        self.out_lines.push("#####".to_string());
        for action in ["PRESSED", "RELEASED", "CLICKED", "UNCLICKED"] {
            let mut line = format!(" {action}:");
            let mut last: Option<i64> = None;
            let entries: Vec<(i64, String)> = events
                .iter()
                .filter(|(k, _, _)| k == action)
                .map(|(_, t, key)| (*t, key.clone()))
                .collect();
            for (t, key) in entries {
                let delta = last.map(|l| (t - l) as f64 / 1000.0);
                // Key names are strings (`"r"`); mouse positions are lists
                // (`(10 20 1080)`) printed without quotes.
                let key_str = if action == "CLICKED" || action == "UNCLICKED" {
                    key
                } else {
                    format!("{key:?}")
                };
                line.push_str(&format!(
                    "\n   {:>7.3} {key_str:<10} {}",
                    t as f64 / 1000.0,
                    delta.map(|d| format!("({d:.3})")).unwrap_or_default()
                ));
                last = Some(t);
            }
            self.out_lines.push(line);
        }
        self.out_lines.push(" Held:".to_string());
        let presses: Vec<(i64, String)> = events
            .iter()
            .filter(|(k, _, _)| k == "PRESSED")
            .map(|(_, t, key)| (*t, key.clone()))
            .collect();
        for (t, key) in presses {
            let release = events
                .iter()
                .find(|(k, t2, k2)| k == "RELEASED" && k2 == &key && *t2 >= t);
            let held = release.map(|(_, t2, _)| (*t2 - t) as f64 / 1000.0);
            self.out_lines.push(format!(
                "  {:<10} {}",
                format!("{key:?}"),
                held.map(|h| format!("{h:.3}")).unwrap_or_default()
            ));
        }
        self.out_lines.push(" Clicked:".to_string());
        let clicks: Vec<(i64, String)> = events
            .iter()
            .filter(|(k, _, _)| k == "CLICKED")
            .map(|(_, t, key)| (*t, key.clone()))
            .collect();
        for (t, key) in clicks {
            let release = events
                .iter()
                .find(|(k, t2, _)| k == "UNCLICKED" && *t2 >= t);
            if let Some((_, t2, pos)) = release {
                let delta = (*t2 - t) as f64 / 1000.0;
                self.out_lines.push(format!("  {key} {pos} {delta:.3}"));
            }
        }
    }

    /// `zz-extended-motor-actions` task driver (`test-model.lisp:65`):
    /// reproduce `run-test`/`testN` (the extractor only records the
    /// `run`/`reset` calls).
    pub fn zz_driver(&mut self) -> Result<(), EngineError> {
        self.zz_test1()?;
        self.zz_test2()?;
        self.zz_test3()?;
        self.zz_test4()?;
        self.zz_test5()?;
        self.zz_test6()?;
        self.zz_test7()?;
        self.zz_test8()?;
        self.zz_test9()?;
        Ok(())
    }

    // ------------------------------------------------ associative-learning

    /// `associative-learning` task driver: `run-it`
    /// (`test-models/associative-learning.lisp`) →
    /// `(approach-1-results 1 .75 :prior-p .08 :prior-w 2.5 :show t)`.
    ///
    /// `create-stims` runs before `reset`, so it consumes the stream left by
    /// the model's initial reset; `reset` re-applies the model's
    /// `:starting-parameters (:seed (1 0) …)` (`central-parameters.lisp:131`),
    /// so `initial-presentation` starts from a fresh `(1 0)` stream.
    pub fn al_driver(&mut self) -> Result<(), EngineError> {
        // The model's `define-model` reset applied the `:starting-parameters`
        // seed, so `create-stims` starts from a fresh `(1 0)` stream.
        self.rng.borrow_mut().set_seed(1, 0);
        self.al_create_stims();
        self.reset()?;
        self.rng.borrow_mut().set_seed(1, 0);
        // `(set-parameter-value …)`: :lf .75, :al-show t, :ol t, :al-prior-p
        // .08, :al-prior-w 2.5 (`approach-1-results`).
        self.cfg.lf_ms = 750;
        self.cfg.al_show = true;
        self.cfg.ol = Ol::On;
        self.cfg.al_prior_p = 0.08;
        self.cfg.al_prior_w = 2.5;
        self.al_create_initial_chunks();
        self.al_initial_presentation()?;
        self.al_drop_out()?;
        self.al_drop_out()?;
        // `time-to-retrieve-at-start` + `output-person-location`
        // (`al-fan-study.lisp:382`/`:467`).
        let data = self.al_time_to_retrieve();
        self.al_output_person_location(&data);
        Ok(())
    }

    /// `permute-list` (`random.lisp:562`): each pick is consed onto the front,
    /// so the result is the reverse of the pick order.
    pub(super) fn al_permute<T: Clone>(&mut self, items: &[T]) -> Vec<T> {
        let mut temp: Vec<T> = items.to_vec();
        let mut result: Vec<T> = Vec::new();
        while !temp.is_empty() {
            let i = self.rng.borrow_mut().random_int(temp.len() as u64) as usize;
            let item = temp.remove(i);
            result.insert(0, item);
        }
        result
    }

    /// The stimulus word lists (`al-fan-study.lisp:220`), before permutation.
    pub(super) fn al_people_list() -> [&'static str; 16] {
        [
            "hippie",
            "guard",
            "earl",
            "giant",
            "captain",
            "debutante",
            "fireman",
            "lawyer",
            "wizard",
            "clerk",
            "writer",
            "doctor",
            "teacher",
            "boxer",
            "queen",
            "singer",
        ]
    }

    pub(super) fn al_places_list() -> [&'static str; 16] {
        [
            "beach", "castle", "dungeon", "forest", "park", "church", "cave", "bank", "store",
            "tower", "lake", "stadium", "casino", "school", "diner", "office",
        ]
    }

    /// `create-stims` (`al-fan-study.lisp:245`): permute people/places, build
    /// the sentence/target/foil index lists, then the filler permutes (kept so
    /// the RNG stream stays aligned even though the fillers are unused).
    pub(super) fn al_create_stims(&mut self) {
        let people: Vec<String> = Self::al_people_list()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let places: Vec<String> = Self::al_places_list()
            .iter()
            .map(|s| s.to_string())
            .collect();
        self.al_people = self.al_permute(&people);
        self.al_places = self.al_permute(&places);
        let s_idx: [(usize, usize, i64, i64); 26] = [
            (0, 0, 1, 1),
            (1, 1, 1, 1),
            (2, 2, 1, 1),
            (3, 3, 2, 1),
            (4, 4, 2, 1),
            (5, 5, 2, 1),
            (6, 6, 3, 1),
            (7, 7, 3, 1),
            (8, 8, 3, 1),
            (9, 9, 1, 2),
            (10, 10, 1, 2),
            (11, 11, 1, 2),
            (12, 12, 1, 3),
            (13, 13, 1, 3),
            (14, 14, 1, 3),
            (4, 10, 2, 2),
            (15, 15, 2, 2),
            (6, 9, 3, 2),
            (7, 15, 3, 2),
            (8, 11, 3, 2),
            (3, 12, 2, 3),
            (15, 13, 2, 3),
            (5, 14, 2, 3),
            (6, 12, 3, 3),
            (7, 13, 3, 3),
            (8, 14, 3, 3),
        ];
        let t_idx: [(usize, usize, i64, i64); 9] = [
            (0, 0, 1, 1),
            (9, 9, 1, 2),
            (12, 12, 1, 3),
            (3, 3, 2, 1),
            (4, 10, 2, 2),
            (15, 13, 2, 3),
            (6, 6, 3, 1),
            (7, 15, 3, 2),
            (8, 14, 3, 3),
        ];
        let f_idx: [(usize, usize, i64, i64); 18] = [
            (0, 3, 1, 1),
            (9, 10, 1, 2),
            (12, 13, 1, 3),
            (3, 0, 2, 1),
            (4, 9, 2, 2),
            (15, 12, 2, 3),
            (6, 0, 3, 1),
            (7, 9, 3, 2),
            (8, 12, 3, 3),
            (0, 6, 1, 1),
            (9, 15, 1, 2),
            (12, 14, 1, 3),
            (3, 6, 2, 1),
            (4, 15, 2, 2),
            (15, 14, 2, 3),
            (6, 3, 3, 1),
            (7, 10, 3, 2),
            (8, 13, 3, 3),
        ];
        let build = |idx: &[(usize, usize, i64, i64)],
                     people: &[String],
                     places: &[String]|
         -> Vec<(String, String, i64, i64)> {
            idx.iter()
                .map(|(a, b, pf, lf)| (people[*a].clone(), places[*b].clone(), *pf, *lf))
                .collect()
        };
        self.al_sentences = build(&s_idx, &self.al_people, &self.al_places);
        self.al_targets = build(&t_idx, &self.al_people, &self.al_places);
        self.al_foils = build(&f_idx, &self.al_people, &self.al_places);
        self.al_fillers.clear();
        for _ in 0..6 {
            let op = self.al_permute(&[1usize, 2, 5, 10, 11, 13, 14]);
            let pl = self.al_permute(&[1usize, 2, 4, 5, 7, 8, 11]);
            for (x, y) in op.iter().zip(pl.iter()) {
                self.al_fillers.push((
                    self.al_people[*x].clone(),
                    self.al_places[*y].clone(),
                    -1,
                    -1,
                ));
            }
        }
    }

    /// `create-initial-chunks` (`al-fan-study.lisp:327`): a named `meaning`
    /// chunk for each person/place plus `in`.
    pub(super) fn al_create_initial_chunks(&mut self) {
        let words: Vec<String> = self
            .al_people
            .iter()
            .chain(self.al_places.iter())
            .cloned()
            .chain(std::iter::once("in".to_string()))
            .collect();
        for w in &words {
            self.al_add_meaning(w);
        }
    }

    pub(super) fn al_add_meaning(&mut self, word: &str) {
        let name = word.to_uppercase();
        if self
            .dm
            .iter()
            .any(|c| c.name.as_deref() == Some(name.as_str()))
        {
            return;
        }
        let inst = ChunkInst {
            name: Some(name.clone()),
            ctype: "MEANING".to_string(),
            slots: vec![("WORD".to_string(), Value::Str(word.to_string()))],
            bare: false,
        };
        self.dm.push(inst.clone());
        self.hist.push(History {
            creation_ms: 0,
            refs: vec![0],
            count: 1,
            fixed_base: None,
        });
        self.named.insert(name.clone(), inst);
        self.chunk_names.insert(name.clone());
        *self.fan_out.entry(name).or_insert(0) += 1;
    }

    /// `initial-presentation` (`al-fan-study.lisp:338`): present a random
    /// ordering of the study sentences, 10 s each.
    pub(super) fn al_initial_presentation(&mut self) -> Result<(), EngineError> {
        let sentences = self.al_sentences.clone();
        let presented = self.al_permute(&sentences);
        for (person, place, _pf, _lf) in &presented {
            self.al_imaginal_request(person, place);
            // `(schedule-clear-buffer 'imaginal 1.5)`.
            self.push(
                self.time_ms + 1500,
                Prio::Num(0),
                Out::Low,
                Event::ClearBuffer {
                    buffer: "IMAGINAL".to_string(),
                    module: "NONE".to_string(),
                },
            );
            self.run_secs(10.0)?;
        }
        Ok(())
    }

    /// `(module-request 'imaginal (define-chunk-spec-fct …))` with the
    /// `(relation in arg1 X arg2 Y)` spec.
    pub(super) fn al_imaginal_request(&mut self, arg1: &str, arg2: &str) {
        let spec = NewChunk::Spec {
            ctype: String::new(),
            slots: vec![
                ("RELATION".to_string(), Value::Sym("IN".to_string())),
                ("ARG1".to_string(), Value::Sym(arg1.to_uppercase())),
                ("ARG2".to_string(), Value::Sym(arg2.to_uppercase())),
            ],
            neg: Vec::new(),
            rels: Vec::new(),
            bare: true,
        };
        self.now(
            Prio::Num(0),
            Out::Medium,
            Event::ModuleRequest {
                buffer: "IMAGINAL".to_string(),
                spec,
                var_slots: Vec::new(),
                trace: false,
            },
        );
    }

    /// `(module-request 'imaginal (relation in arg1 X))`: the drop-out probe
    /// (no `arg2`).
    pub(super) fn al_imaginal_probe(&mut self, arg1: &str) {
        let spec = NewChunk::Spec {
            ctype: String::new(),
            slots: vec![
                ("RELATION".to_string(), Value::Sym("IN".to_string())),
                ("ARG1".to_string(), Value::Sym(arg1.to_uppercase())),
            ],
            neg: Vec::new(),
            rels: Vec::new(),
            bare: true,
        };
        self.now(
            Prio::Num(0),
            Out::Medium,
            Event::ModuleRequest {
                buffer: "IMAGINAL".to_string(),
                spec,
                var_slots: Vec::new(),
                trace: false,
            },
        );
    }

    /// `(module-request 'retrieval (relation in arg1 X arg2 Y))`.
    pub(super) fn al_retrieval_request(&mut self, arg1: &str, arg2: &str) {
        let spec = NewChunk::Spec {
            ctype: String::new(),
            slots: vec![
                ("RELATION".to_string(), Value::Sym("IN".to_string())),
                ("ARG1".to_string(), Value::Sym(arg1.to_uppercase())),
                ("ARG2".to_string(), Value::Sym(arg2.to_uppercase())),
            ],
            neg: Vec::new(),
            rels: Vec::new(),
            bare: true,
        };
        self.now(
            Prio::Num(0),
            Out::Medium,
            Event::ModuleRequest {
                buffer: "RETRIEVAL".to_string(),
                spec,
                var_slots: Vec::new(),
                trace: false,
            },
        );
    }

    /// `(run-full-time secs)` (`scheduling.lisp:986`): schedule a
    /// `:min`-priority maintenance dummy at the limit so the run always
    /// reaches it (`Stopped because time limit reached`).
    pub(super) fn al_run_full_time(&mut self, secs: f64) -> Result<(), EngineError> {
        let limit = self.time_ms + (secs * 1000.0).round() as i64;
        self.push_m(limit, Prio::Min, Out::Never, true, Event::Dummy);
        self.run_secs(secs)
    }

    /// `drop-out` (`al-fan-study.lisp:349`): present each probe once, then
    /// retrieve every sentence it appeared in, 20 s each.
    pub(super) fn al_drop_out(&mut self) -> Result<(), EngineError> {
        // `(set-parameter-value :lf 0)` for the study phase; restored after.
        let lf = self.cfg.lf_ms;
        self.cfg.lf_ms = 0;
        // `(permute-list (append (copy-list *people*) (copy-list *places*)))`:
        // the *original* lists, not the `create-stims` permutations.
        let mut all: Vec<String> = Self::al_people_list()
            .iter()
            .map(|s| s.to_string())
            .collect();
        all.extend(Self::al_places_list().iter().map(|s| s.to_string()));
        let order = self.al_permute(&all);
        for x in &order {
            // `(erase-buffer 'imaginal)`: silent, and the removed chunk does
            // not merge into DM.
            self.buffer_mut("IMAGINAL").chunk = None;
            self.al_imaginal_probe(x);
            self.run_secs(0.2)?;
            let matching: Vec<(String, String)> = self
                .al_sentences
                .iter()
                .filter(|(p, pl, _, _)| p == x || pl == x)
                .map(|(p, pl, _, _)| (p.clone(), pl.clone()))
                .collect();
            for (p, pl) in matching {
                self.al_retrieval_request(&p, &pl);
                self.al_run_full_time(20.0)?;
            }
        }
        self.cfg.lf_ms = lf;
        Ok(())
    }

    /// Whether `buffer` is a source buffer for associative learning: a
    /// non-zero spread, not retrieval (`finish-al-reset`,
    /// `associative-learning.lisp:376`).
    pub(super) fn al_is_source_buffer(&self, buffer: &str) -> bool {
        match buffer {
            "GOAL" => self.cfg.ga != 0.0,
            "IMAGINAL" => self.cfg.imaginal_activation != 0.0,
            _ => false,
        }
    }

    /// `source-buffer-chunks` (`associative-learning.lisp:633`).
    pub(super) fn al_source_buffer_chunks(&self) -> Vec<String> {
        let mut r = Vec::new();
        for buffer in ["GOAL", "IMAGINAL"] {
            if !self.al_is_source_buffer(buffer) {
                continue;
            }
            if let Some(chunk) = self.buffers.get(buffer).and_then(|b| b.chunk.as_ref()) {
                for (_, v) in &chunk.slots {
                    if let Some(name) = self.chunk_name_of(v) {
                        r.push(name);
                    }
                }
            }
        }
        r
    }

    /// The chunk-valued slots of the DM chunk named `name` (the `self-sources`
    /// of `al-buffer-cleared`, `associative-learning.lisp:673`).
    pub(super) fn al_chunk_sources(&self, name: &str) -> Vec<String> {
        self.dm
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .map(|c| {
                c.slots
                    .iter()
                    .filter_map(|(_, v)| self.chunk_name_of(v))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `chunk-creation-time` for the DM chunk named `name`.
    pub(super) fn dm_creation_time(&self, name: &str) -> i64 {
        self.dm
            .iter()
            .position(|c| c.name.as_deref() == Some(name))
            .map(|i| self.hist[i].creation_ms)
            .unwrap_or(0)
    }

    pub(super) fn al_ni_cj_count(&self, j: &str, i: &str) -> i64 {
        self.al_chunks
            .get(j)
            .and_then(|c| c.ni_cj_count.iter().find(|(n, _)| n == i))
            .map(|(_, v)| *v)
            .unwrap_or(0)
    }

    pub(super) fn al_ni_cj_refs(&self, j: &str, i: &str) -> Vec<i64> {
        self.al_chunks
            .get(j)
            .and_then(|c| c.ni_cj_refs.iter().find(|(n, _)| n == i))
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }

    /// `(model-output ...)` from the associative-learning module: a raw line
    /// with no timestamp/module prefix.
    pub(super) fn al_out(&mut self, line: String) {
        self.out_lines.push(line);
    }

    /// `update-al-parameters` (`associative-learning.lisp:712`).
    pub(super) fn al_update(&mut self, i: &str, js: &[String]) {
        let ct = self.time_ms;
        let ol = self.cfg.ol.clone();
        self.al_total_count += 1;
        al_needed_refs(&ol, &mut self.al_total_refs, ct);
        {
            let c = self.al_chunks.entry(i.to_string()).or_default();
            c.ni_count += 1;
            al_needed_refs(&ol, &mut c.ni_refs, ct);
        }
        for j in js {
            // `(pushnew j (chunk-associated-js i))`.
            {
                let ci = self.al_chunks.entry(i.to_string()).or_default();
                if !ci.associated_js.iter().any(|x| x == j) {
                    ci.associated_js.insert(0, j.clone());
                }
            }
            let cj = self.al_chunks.entry(j.clone()).or_default();
            cj.cj_count += 1;
            al_needed_refs(&ol, &mut cj.cj_refs, ct);
            match cj.ni_cj_count.iter_mut().find(|(n, _)| n == i) {
                Some(e) => e.1 += 1,
                None => cj.ni_cj_count.insert(0, (i.to_string(), 1)),
            }
            match cj.ni_cj_refs.iter_mut().find(|(n, _)| n == i) {
                Some(e) => al_needed_refs(&ol, &mut e.1, ct),
                None => {
                    let mut r = Vec::new();
                    al_needed_refs(&ol, &mut r, ct);
                    cj.ni_cj_refs.insert(0, (i.to_string(), r));
                }
            }
        }
    }

    /// `compute-learned-sji` (`associative-learning.lisp:288`): the learned Sji
    /// between needed item `i` and context chunk `j`, printing the module's
    /// detailed trace when `:al-show`/`:act` is on.
    pub(super) fn al_compute_learned_sji(&mut self, j: &str, i: &str) -> f32 {
        let Some(eal) = self.cfg.al_eal else {
            return 0.0;
        };
        let p = self.cfg.al_prior_p;
        let w = self.cfg.al_prior_w;
        let decay = eal;
        let scale = 0.001f32.powf(-eal).ln();
        let now = self.time_ms;
        let ol = self.cfg.ol.clone();

        let nicj_count = self.al_ni_cj_count(j, i);
        let nicj_refs = self.al_ni_cj_refs(j, i);
        let (ni_count, ni_refs) = self
            .al_chunks
            .get(i)
            .map(|c| (c.ni_count, c.ni_refs.clone()))
            .unwrap_or((0, Vec::new()));
        let not_ni_refs: Vec<i64> = self
            .al_total_refs
            .iter()
            .filter(|r| !ni_refs.contains(r))
            .cloned()
            .collect();
        let (cj_count, cj_refs) = self
            .al_chunks
            .get(j)
            .map(|c| (c.cj_count, c.cj_refs.clone()))
            .unwrap_or((0, Vec::new()));
        let not_nicj_refs: Vec<i64> = cj_refs
            .iter()
            .filter(|r| !nicj_refs.contains(r))
            .cloned()
            .collect();
        let creation_i = self.dm_creation_time(i);
        let creation_j = self.dm_creation_time(j);
        let total = self.al_total_count;
        let total_creation = self.al_total_creation_time;

        let f_not_ni = if total - ni_count == 0 {
            0.0
        } else {
            let ol2 = if not_ni_refs.is_empty() {
                Ol::On
            } else {
                ol.clone()
            };
            exp_coerced(bl_equation(
                now,
                &not_ni_refs,
                (total - ni_count) as f32,
                total_creation,
                decay,
                &ol2,
                scale,
            ))
        };
        let f_ni = if ni_count == 0 {
            0.0
        } else {
            exp_coerced(bl_equation(
                now,
                &ni_refs,
                ni_count as f32,
                creation_i,
                decay,
                &ol,
                scale,
            ))
        };
        let f_not_nicj = if cj_count - nicj_count == 0 {
            0.0
        } else {
            let ol2 = if not_nicj_refs.is_empty() {
                Ol::On
            } else {
                ol.clone()
            };
            exp_coerced(bl_equation(
                now,
                &not_nicj_refs,
                (cj_count - nicj_count) as f32,
                creation_j,
                decay,
                &ol2,
                scale,
            ))
        };
        let f_nicj = if nicj_count == 0 {
            0.0
        } else {
            exp_coerced(bl_equation(
                now,
                &nicj_refs,
                nicj_count as f32,
                creation_i,
                decay,
                &ol,
                scale,
            ))
        };
        let pcjni = (p * w + f_nicj) / (w + f_ni);
        let pcj_not_ni = (p * w + f_not_nicj) / (w + f_not_ni);
        let mut sji = log_coerced(pcjni / pcj_not_ni);

        if self.cfg.al_show || self.cfg.act.is_some() {
            self.al_out(format!(
                "     Computing Sji from {} to {} with the decay set to {} prior probability {} and prior weight {}",
                j,
                i,
                lisp_f(decay),
                lisp_f(p),
                lisp_f(w)
            ));
            self.al_out(format!("      F(Ni&Cj) = {}", lisp_f(f_nicj)));
            if nicj_count != 0 {
                self.al_out(format!(
                    "      {} was needed while {} was in the context {} times with history {}",
                    i,
                    j,
                    nicj_count,
                    al_history(&nicj_refs)
                ));
            } else {
                self.al_out(format!(
                    "      {} was needed while {} was in the context 0 times with history nil",
                    i, j
                ));
            }
            self.al_out(format!("      F(Ni) = {}", lisp_f(f_ni)));
            self.al_out(format!(
                "        {} was needed {} times with history {}",
                i,
                ni_count,
                al_history(&ni_refs)
            ));
            self.al_out(format!("      F(~Ni&Cj) = {}", lisp_f(f_not_nicj)));
            self.al_out(format!(
                "        Retrieval of something other than {} while {} was in the context {} times with history {}",
                i,
                j,
                cj_count - nicj_count,
                al_history(&not_nicj_refs)
            ));
            self.al_out(format!("      F(~Ni) = {}", lisp_f(f_not_ni)));
            self.al_out(format!(
                "        Retrieval of something other than {} happened {} times with history {}",
                i,
                total - ni_count,
                al_history(&not_ni_refs)
            ));
            self.al_out(format!("      P(Cj|Ni) = {}", lisp_f(pcjni)));
            self.al_out(format!("      P(Cj|~Ni) = {}", lisp_f(pcj_not_ni)));
            self.al_out(format!("      Sji = {}", lisp_f(sji)));
        }

        // `:nsji`: only `t` keeps a negative Sji; `warn`/`nil` clamp at 0 (the
        // associative-learning trace shows no warning).
        if sji < 0.0 && self.cfg.nsji != Nsji::Allow {
            sji = 0.0;
        }
        sji
    }

    /// `time-to-retrieve-at-start` (`al-fan-study.lisp:382`): for each target
    /// and foil, compute the activation of every matching chunk (which prints
    /// the Sji traces) and convert it to a retrieval time; returns the 18
    /// averaged cells.
    pub(super) fn al_time_to_retrieve(&mut self) -> Vec<f64> {
        // `(set-buffer-chunk 'imaginal (car (define-chunks (isa
        // comprehend-sentence relation in))))`. The slot order matches the
        // ref's source order in the Sji trace (arg1 arg2 relation).
        let ctx = ChunkInst {
            name: Some("COMPREHEND-SENTENCE0".to_string()),
            ctype: "COMPREHEND-SENTENCE".to_string(),
            slots: vec![
                ("RELATION".to_string(), Value::Sym("IN".to_string())),
                ("ARG1".to_string(), Value::Nil),
                ("ARG2".to_string(), Value::Nil),
            ],
            bare: false,
        };
        self.buffer_mut("IMAGINAL").chunk = Some(ctx);
        let lf = self.cfg.lf_ms as f64 / 1000.0;
        let mut data: Vec<Vec<Vec<Vec<f64>>>> = vec![vec![vec![Vec::new(); 2]; 3]; 3];
        let mut items: Vec<(String, String, usize, usize, bool)> = Vec::new();
        for (p, pl, pf, lf) in &self.al_targets {
            items.push((p.clone(), pl.clone(), *pf as usize, *lf as usize, true));
        }
        for (p, pl, pf, lf) in &self.al_foils {
            items.push((p.clone(), pl.clone(), *pf as usize, *lf as usize, false));
        }
        for (person, place, pf, lfan, is_target) in &items {
            if let Some(c) = self
                .buffers
                .get_mut("IMAGINAL")
                .and_then(|b| b.chunk.as_mut())
            {
                for (s, v) in c.slots.iter_mut() {
                    if s.as_str() == "ARG1" {
                        *v = Value::Sym(person.to_uppercase());
                    } else if s.as_str() == "ARG2" {
                        *v = Value::Sym(place.to_uppercase());
                    }
                }
            }
            let ap = self.al_sdm_sdp(Some(person), None);
            let al = self.al_sdm_sdp(None, Some(place));
            let act_p = ap.iter().cloned().fold(f64::MIN, f64::max);
            let act_l = al.iter().cloned().fold(f64::MIN, f64::max);
            let idx = if *is_target { 0 } else { 1 };
            data[pf - 1][lfan - 1][idx].push(0.845 + lf * (-act_p).exp());
            data[pf - 1][lfan - 1][idx].push(0.845 + lf * (-act_l).exp());
        }
        let mut d = vec![0.0f64; 18];
        for pf in 0..3 {
            for lf in 0..3 {
                for tf in 0..2 {
                    let v = &data[pf][lf][tf];
                    d[pf + 3 * lf + 9 * tf] = v.iter().sum::<f64>() / v.len() as f64;
                }
            }
        }
        d
    }

    /// `(sdm-fct (list slot value '- other nil))` + `(sdp-fct (list name :name
    /// :activation))` (`dm-commands.lisp:491`/`:667`): the activation of every
    /// chunk with `slot = value` and `other` filled, in DM order.
    pub(super) fn al_sdm_sdp(&mut self, arg1: Option<&str>, arg2: Option<&str>) -> Vec<f64> {
        let a1 = arg1.map(|s| s.to_uppercase());
        let a2 = arg2.map(|s| s.to_uppercase());
        let candidates: Vec<String> = self
            .dm
            .iter()
            .filter(|c| {
                if let Some(v) = &a1 {
                    if c.get("ARG1") != Value::Sym(v.clone()) {
                        return false;
                    }
                    if c.get("ARG2") == Value::Nil {
                        return false;
                    }
                }
                if let Some(v) = &a2 {
                    if c.get("ARG2") != Value::Sym(v.clone()) {
                        return false;
                    }
                    if c.get("ARG1") == Value::Nil {
                        return false;
                    }
                }
                true
            })
            .filter_map(|c| c.name.clone())
            .collect();
        candidates
            .iter()
            .map(|n| self.al_sdp_activation(n))
            .collect()
    }

    /// `compute-activation` for a named DM chunk with an empty request spec
    /// (`dm-commands.lisp:738`): base-level + spreading (no partial matching,
    /// `:ans` is nil).
    pub(super) fn al_sdp_activation(&mut self, name: &str) -> f64 {
        let Some(idx) = self.dm.iter().position(|c| c.name.as_deref() == Some(name)) else {
            return f64::MIN;
        };
        let base = match self.cfg.bll {
            Some(bll) => {
                let (refs, creation, count) = {
                    let h = &self.hist[idx];
                    (h.refs.clone(), h.creation_ms, h.count)
                };
                if count == 0 {
                    -999999.0
                } else {
                    self.cfg.blc
                        + bl_equation(
                            self.time_ms,
                            &refs,
                            count as f32,
                            creation,
                            bll,
                            &self.cfg.ol,
                            self.cfg.act_scale,
                        )
                }
            }
            None => self.hist[idx].fixed_base.unwrap_or(self.cfg.blc),
        };
        let spread = self.spreading_activation(name, self.cfg.mas);
        (base + spread) as f64
    }

    /// `output-person-location` (`al-fan-study.lisp:467`): the correlation and
    /// RMSD against the experimental data plus the TARGETS/FOILS table.
    pub(super) fn al_output_person_location(&mut self, d: &[f64]) {
        let data = [
            1.11, 1.17, 1.22, 1.17, 1.20, 1.22, 1.15, 1.23, 1.36, 1.20, 1.22, 1.26, 1.25, 1.36,
            1.29, 1.26, 1.47, 1.47,
        ];
        let n = d.len() as f64;
        let sum_r: f64 = d.iter().sum();
        let sum_d: f64 = data.iter().sum();
        let sum_r2: f64 = d.iter().map(|x| x * x).sum();
        let sum_d2: f64 = data.iter().map(|x| x * x).sum();
        let sum_rd: f64 = d.iter().zip(data.iter()).map(|(a, b)| a * b).sum();
        let avg_r = sum_r / n;
        let avg_d = sum_d / n;
        let corr = (sum_rd / n - avg_r * avg_d)
            / ((sum_r2 / n - avg_r * avg_r).sqrt() * (sum_d2 / n - avg_d * avg_d).sqrt());
        let md = ((sum_r2 + sum_d2 - 2.0 * sum_rd) / n).sqrt();
        self.out_lines.push(format!("CORRELATION: {corr:6.3}"));
        self.out_lines.push(format!("MEAN DEVIATION: {md:6.3}"));
        self.out_lines.push("TARGETS:".to_string());
        self.out_lines
            .push("                         Person fan".to_string());
        self.out_lines
            .push("  Location      1             2             3".to_string());
        self.out_lines.push("    fan".to_string());
        for i in 0..3 {
            let vals: Vec<String> = (0..3).map(|j| format!("{:8.3}", d[j + i * 3])).collect();
            self.out_lines
                .push(format!("     {}    {}", i + 1, vals.join("      ")));
        }
        self.out_lines.push(String::new());
        self.out_lines.push(String::new());
        self.out_lines.push("FOILS:".to_string());
        for i in 0..3 {
            let vals: Vec<String> = (0..3)
                .map(|j| format!("{:8.3}", d[j + (i + 3) * 3]))
                .collect();
            self.out_lines
                .push(format!("     {}    {}", i + 1, vals.join("      ")));
        }
    }

    /// `zbrodoff` task driver (`tutorial/lisp/zbrodoff.lisp`): present each
    /// alpha-arithmetic trial in a live window, run until the model's key
    /// press, and aggregate correct RTs by block and addend.
    pub fn zbrodoff_driver(&mut self, mode: ZbrodoffMode) -> Result<(), EngineError> {
        match mode {
            ZbrodoffMode::Compare => {
                self.out_lines.push("? (zbrodoff-compare 10)".to_string());
                self.out_lines.push(String::new());
                self.out_lines.push(">>> zbrodoff.compare(10)".to_string());
                self.out_lines.push(String::new());
                self.zbrodoff_reset(100)?;
                let analysis = self.zbrodoff_collect(&Self::zbrodoff_experiment_blocks())?;
                self.zbrodoff_print_statistics(&analysis);
                self.out_lines.push(String::new());
                self.zbrodoff_print_table(&analysis);
            }
            ZbrodoffMode::Experiment => {
                self.out_lines.push("? (zbrodoff-experiment)".to_string());
                self.out_lines.push(String::new());
                self.out_lines.push(">>> zbrodoff.experiment()".to_string());
                self.out_lines.push(String::new());
                self.out_lines.push(String::new());
                self.zbrodoff_reset(100)?;
                let analysis = self.zbrodoff_collect(&Self::zbrodoff_experiment_blocks())?;
                self.zbrodoff_print_table(&analysis);
            }
            ZbrodoffMode::SetAndBlock => {
                self.zbrodoff_reset(100)?;
                self.out_lines.push("? (zbrodoff-set)".to_string());
                self.out_lines.push(String::new());
                let set = self.zbrodoff_collect(&[1])?;
                self.zbrodoff_print_table(&set);
                self.out_lines.push(String::new());
                self.out_lines.push(">>> zbrodoff.block()".to_string());
                self.out_lines.push(String::new());
                let block = self.zbrodoff_collect(&[1; 8])?;
                self.zbrodoff_print_table(&block);
            }
            ZbrodoffMode::StartingResults => {
                self.zbrodoff_reset(201)?;
                let analysis = self.zbrodoff_collect(&Self::zbrodoff_experiment_blocks())?;
                self.zbrodoff_print_statistics(&analysis);
                self.out_lines.push(String::new());
                self.zbrodoff_print_table(&analysis);
            }
        }
        Ok(())
    }

    fn zbrodoff_experiment_blocks() -> Vec<usize> {
        (1..=3)
            .flat_map(|block| std::iter::repeat(block).take(8))
            .collect()
    }

    fn zbrodoff_reset(&mut self, seed: u64) -> Result<(), EngineError> {
        self.reset()?;
        self.set_seed((seed, 0));
        self.set_trace(false);
        self.last_output_key = None;
        Ok(())
    }

    fn zbrodoff_collect(
        &mut self,
        block_numbers: &[usize],
    ) -> Result<ZbrodoffAnalysis, EngineError> {
        let trials = self.zbrodoff_make_trials(block_numbers);
        let results = self.zbrodoff_run_trials(&trials)?;
        Ok(Self::zbrodoff_analyze(&results))
    }

    fn zbrodoff_make_trials(&mut self, block_numbers: &[usize]) -> Vec<ZbrodoffTrial> {
        const DATA: [(&str, &str, &str, &str); 24] = [
            ("a", "2", "c", "k"),
            ("d", "2", "f", "k"),
            ("b", "3", "e", "k"),
            ("e", "3", "h", "k"),
            ("c", "4", "g", "k"),
            ("f", "4", "j", "k"),
            ("a", "2", "d", "d"),
            ("d", "2", "g", "d"),
            ("b", "3", "f", "d"),
            ("e", "3", "i", "d"),
            ("c", "4", "h", "d"),
            ("f", "4", "k", "d"),
            ("a", "2", "c", "k"),
            ("d", "2", "f", "k"),
            ("b", "3", "e", "k"),
            ("e", "3", "h", "k"),
            ("c", "4", "g", "k"),
            ("f", "4", "j", "k"),
            ("a", "2", "d", "d"),
            ("d", "2", "g", "d"),
            ("b", "3", "f", "d"),
            ("e", "3", "i", "d"),
            ("c", "4", "h", "d"),
            ("f", "4", "k", "d"),
        ];
        let mut trials = Vec::new();
        for &block in block_numbers {
            let set: Vec<ZbrodoffTrial> = DATA
                .iter()
                .map(|(a, b, c, answer)| ZbrodoffTrial {
                    block,
                    addend: b.parse().unwrap(),
                    text: format!("{a} + {b} = {c}"),
                    answer: answer.to_string(),
                })
                .collect();
            trials.extend(self.rng.borrow_mut().permute_list(set));
        }
        trials
    }

    fn zbrodoff_run_trials(
        &mut self,
        trials: &[ZbrodoffTrial],
    ) -> Result<Vec<(usize, usize, bool, f64)>, EngineError> {
        self.open_window(300, 300);
        self.pending_finst_transfers.clear();
        let mut results = Vec::with_capacity(trials.len());
        for (i, trial) in trials.iter().enumerate() {
            if i > 0 {
                self.clear_window();
                // This task replaces the entire display between trials; a
                // reused first-letter position must be treated as new.
                self.pending_finst_transfers.clear();
            }
            self.last_output_key = None;
            self.add_window_text(&trial.text, 100, 150, 75, 20, 12);
            let start = self.time_ms;
            let limit = start + 10_000;
            let mut pressed = None;
            while self.time_ms < limit {
                if !self.step_once()? {
                    break;
                }
                if let Some(key) = self.last_output_key.take() {
                    pressed = Some(key);
                    break;
                }
            }
            let Some((key, time)) = pressed else {
                return Err(unimpl(format!(
                    "zbrodoff trial {} ({}) produced no output-key",
                    i + 1,
                    trial.text
                )));
            };
            let correct = key.eq_ignore_ascii_case(&trial.answer);
            results.push((
                trial.block,
                trial.addend,
                correct,
                (time - start) as f64 / 1000.0,
            ));
        }
        Ok(results)
    }

    fn zbrodoff_analyze(results: &[(usize, usize, bool, f64)]) -> ZbrodoffAnalysis {
        let blocks = results.iter().map(|r| r.0).max().unwrap_or(1);
        // ACT-R's trial times are single-floats (`1000.0` in the task source),
        // and the summary format is sensitive to the final half-way value.
        let mut sums = vec![[0.0f32; 3]; blocks];
        let mut counts = vec![[0u32; 3]; blocks];
        let mut all = vec![[0u32; 3]; blocks];
        for &(block, addend, correct, rt) in results {
            let (b, a) = (block - 1, addend - 2);
            all[b][a] += 1;
            if correct {
                sums[b][a] += rt as f32;
                counts[b][a] += 1;
            }
        }
        let mut rts = Vec::with_capacity(blocks * 3);
        let mut out_counts = Vec::with_capacity(blocks * 3);
        let mut totals = [0u32; 3];
        for b in 0..blocks {
            for a in 0..3 {
                let n = counts[b][a];
                rts.push(if n > 0 {
                    (sums[b][a] / n as f32) as f64
                } else {
                    0.0
                });
                out_counts.push(n);
                totals[a] += all[b][a];
            }
        }
        for total in &mut totals {
            *total /= blocks as u32;
        }
        ZbrodoffAnalysis {
            blocks,
            rts,
            counts: out_counts,
            totals,
        }
    }

    fn zbrodoff_print_statistics(&mut self, analysis: &ZbrodoffAnalysis) {
        const DATA: [f64; 9] = [1.84, 2.46, 2.82, 1.21, 1.45, 1.42, 1.14, 1.21, 1.17];
        let n = DATA.len() as f64;
        let sum_r: f64 = analysis.rts.iter().sum();
        let sum_d: f64 = DATA.iter().sum();
        let sum_r2: f64 = analysis.rts.iter().map(|x| x * x).sum();
        let sum_d2: f64 = DATA.iter().map(|x| x * x).sum();
        let sum_rd: f64 = analysis
            .rts
            .iter()
            .zip(DATA.iter())
            .map(|(a, b)| a * b)
            .sum();
        let avg_r = sum_r / n;
        let avg_d = sum_d / n;
        let corr = (sum_rd / n - avg_r * avg_d)
            / ((sum_r2 / n - avg_r * avg_r).sqrt() * (sum_d2 / n - avg_d * avg_d).sqrt());
        let md = ((sum_r2 + sum_d2 - 2.0 * sum_rd) / n).sqrt();
        self.out_lines.push(format!("CORRELATION: {corr:6.3}"));
        self.out_lines.push(format!("MEAN DEVIATION: {md:6.3}"));
    }

    fn zbrodoff_print_table(&mut self, analysis: &ZbrodoffAnalysis) {
        let mut header = String::from("        ");
        for (i, addend) in ["2", "3", "4"].iter().enumerate() {
            header.push_str(&format!(" {addend:>6} ({:>2})", analysis.totals[i]));
        }
        self.out_lines.push(header);
        for block in 0..analysis.blocks {
            let mut line = format!("Block {:>2}", block + 1);
            for addend in 0..3 {
                let i = block * 3 + addend;
                line.push_str(&format!(
                    " {:>6.3} ({:>2})",
                    analysis.rts[i], analysis.counts[i]
                ));
            }
            self.out_lines.push(line);
        }
    }

    /// `onehit.lisp` task driver: regular deck, fixed-threshold opponent, and
    /// the model's key choice for each hand. The model observes the result
    /// before the next deal.
    pub fn onehit_driver(&mut self, mode: OnehitMode) -> Result<(), EngineError> {
        self.set_trace(false);
        self.set_seed((1, 0));
        match mode {
            OnehitMode::Blocks { blocks, hands } => {
                let mut all = Vec::new();
                for _ in 0..blocks {
                    all.push(self.onehit_hands(hands, false)?);
                }
                let shown: Vec<String> = all.iter().map(Self::onehit_score_list).collect();
                self.out_lines.push(format!(
                    "? (onehit-blocks {blocks} {hands})\n({})",
                    shown.join(" ")
                ));
            }
            OnehitMode::Hands { hands } => {
                let scores = self.onehit_hands(hands, false)?;
                self.out_lines.push(format!(
                    "? (onehit-hands {hands})\n{}",
                    Self::onehit_score_list(&scores)
                ));
            }
            OnehitMode::Learning { games } => {
                let mut data = vec![[0u32; 4]; 20];
                for i in 0..games {
                    self.reset()?;
                    self.set_seed((101 + i as u64 + 1, 0));
                    self.set_trace(false);
                    for block in &mut data {
                        let scores = self.onehit_hands(5, false)?;
                        for (total, value) in block.iter_mut().zip(scores) {
                            *total += value;
                        }
                    }
                }
                let n = games as f32;
                let percentages: Vec<f32> =
                    data.iter().map(|block| block[0] as f32 / n / 5.0).collect();
                let averages: Vec<f32> = percentages
                    .chunks(5)
                    .map(|chunk| chunk.iter().sum::<f32>() / 5.0)
                    .collect();
                let list = |values: &[f32]| {
                    format!(
                        "({})",
                        values
                            .iter()
                            .map(|v| super::lisp_s_f32(*v))
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                };
                self.out_lines.push(format!(
                    "? (onehit-learning {games})\n({} {})",
                    list(&averages),
                    list(&percentages)
                ));
            }
            OnehitMode::PythonBlocks {
                seed,
                blocks,
                hands,
            } => {
                self.reset()?;
                self.set_seed(seed);
                self.set_trace(false);
                let all = self.onehit_blocks(blocks, hands, false)?;
                let shown: Vec<String> = all.iter().map(Self::onehit_python_score_list).collect();
                self.out_lines.push(format!(
                    ">>> onehit.blocks({blocks},{hands})\n[{}]",
                    shown.join(", ")
                ));
            }
            OnehitMode::PythonHands { seed, hands } => {
                self.reset()?;
                self.set_seed(seed);
                self.set_trace(false);
                self.out_lines
                    .push(format!(">>> onehit.hands({hands},True)"));
                let scores = self.onehit_hands(hands, true)?;
                self.out_lines.push(Self::onehit_python_score_list(&scores));
            }
            OnehitMode::PythonLearning { first_seed, games } => {
                let mut data = vec![[0u32; 4]; 20];
                for i in 0..games {
                    self.reset()?;
                    self.set_seed((first_seed + i as u64, 0));
                    self.set_trace(false);
                    for block in &mut data {
                        let scores = self.onehit_hands(5, false)?;
                        for (total, value) in block.iter_mut().zip(scores) {
                            *total += value;
                        }
                    }
                }
                let n = games as f64;
                let percentages: Vec<f64> =
                    data.iter().map(|block| block[0] as f64 / n / 5.0).collect();
                let averages: Vec<f64> = percentages
                    .chunks(5)
                    .map(|chunk| chunk.iter().sum::<f64>() / 5.0)
                    .collect();
                self.out_lines.push(format!(
                    ">>> onehit.learning({games})\n[{}, {}]",
                    Self::onehit_python_float_list(&averages),
                    Self::onehit_python_float_list(&percentages)
                ));
            }
        }
        Ok(())
    }

    fn onehit_python_score_list(scores: &[u32; 4]) -> String {
        format!(
            "[{}, {}, {}, {}]",
            scores[0], scores[1], scores[2], scores[3]
        )
    }

    fn onehit_python_float_list(values: &[f64]) -> String {
        format!(
            "[{}]",
            values
                .iter()
                .map(|v| format!("{v}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn onehit_blocks(
        &mut self,
        blocks: u32,
        hands: u32,
        print_game: bool,
    ) -> Result<Vec<[u32; 4]>, EngineError> {
        let mut all = Vec::new();
        for _ in 0..blocks {
            all.push(self.onehit_hands(hands, print_game)?);
        }
        Ok(all)
    }

    fn onehit_score_list(scores: &[u32; 4]) -> String {
        format!("({} {} {} {})", scores[0], scores[1], scores[2], scores[3])
    }

    fn onehit_hands(&mut self, hands: u32, print_game: bool) -> Result<[u32; 4], EngineError> {
        let mut scores = [0u32; 4];
        for _ in 0..hands {
            let mut mcards = vec![self.onehit_deal(), self.onehit_deal(), self.onehit_deal()];
            let mut ocards = vec![self.onehit_deal(), self.onehit_deal(), self.onehit_deal()];
            let mchoice = self.onehit_show_cards(&mcards[..2], ocards[0])?;
            let och = Self::onehit_score(&ocards[..2]) < 15;
            if mchoice != "h" {
                mcards.truncate(2);
            }
            if !och {
                ocards.truncate(2);
            }
            let mres = Self::onehit_outcome(&mcards, &ocards);
            let ores = Self::onehit_outcome(&ocards, &mcards);
            let mtot = Self::onehit_score(&mcards);
            let otot = Self::onehit_score(&ocards);
            self.onehit_show_results(&mcards, &ocards, &mres, &ores)?;
            if print_game {
                let cards = |cards: &[i64]| {
                    cards
                        .iter()
                        .map(|card| format!("{card:>2} "))
                        .collect::<String>()
                };
                self.out_lines.push(format!(
                    "Model: {} -> {:>2} ({mres:<4})   Opponent: {}-> {:>2} ({ores:<4})",
                    cards(&mcards),
                    mtot,
                    cards(&ocards),
                    otot
                ));
            }
            if mres == "win" {
                scores[0] += 1;
            }
            if ores == "win" {
                scores[1] += 1;
            }
            if mres == "bust" && ores == "bust" {
                scores[2] += 1;
            }
            if mtot == otot && mres != "bust" && ores != "bust" {
                scores[3] += 1;
            }
        }
        Ok(scores)
    }

    fn onehit_deal(&mut self) -> i64 {
        (self.rng.borrow_mut().random_int(13) as i64 + 1).min(10)
    }

    fn onehit_score(cards: &[i64]) -> i64 {
        let mut total: i64 = cards.iter().sum();
        for _ in 0..cards.iter().filter(|c| **c == 1).count() {
            if total + 10 <= 21 {
                total += 10;
            }
        }
        total
    }

    fn onehit_outcome(own: &[i64], other: &[i64]) -> &'static str {
        let o = Self::onehit_score(own);
        let t = Self::onehit_score(other);
        if o > 21 {
            "bust"
        } else if t > 21 || o > t {
            "win"
        } else {
            "lose"
        }
    }

    fn onehit_game_slots(
        &self,
        state: &str,
        mcards: &[i64],
        ocards: &[i64],
        mres: Option<&str>,
        ores: Option<&str>,
    ) -> Vec<(String, Sexp)> {
        let card = |c: Option<&i64>| match c {
            Some(v) => Sexp::Int(*v),
            None => Sexp::Sym("NIL".into()),
        };
        let mstart = Self::onehit_score(&mcards[..mcards.len().min(2)]);
        let ostart = Self::onehit_score(&ocards[..ocards.len().min(1)]);
        vec![
            ("MC1".into(), card(mcards.first())),
            ("MC2".into(), card(mcards.get(1))),
            ("MC3".into(), card(mcards.get(2))),
            ("MSTART".into(), Sexp::Int(mstart)),
            (
                "MTOT".into(),
                if mres.is_some() {
                    Sexp::Int(Self::onehit_score(mcards))
                } else {
                    Sexp::Sym("NIL".into())
                },
            ),
            (
                "MRESULT".into(),
                mres.map(|s| Sexp::Sym(s.to_uppercase()))
                    .unwrap_or(Sexp::Sym("NIL".into())),
            ),
            ("OC1".into(), card(ocards.first())),
            ("OC2".into(), card(ocards.get(1))),
            ("OC3".into(), card(ocards.get(2))),
            ("OSTART".into(), Sexp::Int(ostart)),
            (
                "OTOT".into(),
                if ores.is_some() {
                    Sexp::Int(Self::onehit_score(ocards))
                } else {
                    Sexp::Sym("NIL".into())
                },
            ),
            (
                "ORESULT".into(),
                ores.map(|s| Sexp::Sym(s.to_uppercase()))
                    .unwrap_or(Sexp::Sym("NIL".into())),
            ),
            ("STATE".into(), Sexp::Sym(state.to_uppercase())),
        ]
    }

    fn onehit_set_goal(&mut self, slots: &[(String, Sexp)]) -> Result<(), EngineError> {
        if self
            .buffers
            .get("GOAL")
            .and_then(|b| b.chunk.as_ref())
            .is_some()
        {
            self.mod_focus(slots);
        } else {
            let slots = slots
                .iter()
                .map(|(name, value)| {
                    let value = match value {
                        Sexp::Int(i) => crate::model::Pattern::Int(*i),
                        Sexp::Float(x) => crate::model::Pattern::Num(*x),
                        Sexp::Str(s) => crate::model::Pattern::Str(s.clone()),
                        Sexp::Sym(s) => crate::model::Pattern::Sym(s.clone()),
                        Sexp::List(_) => crate::model::Pattern::Nil,
                    };
                    (name.clone(), value)
                })
                .collect();
            self.goal_focus(&ChunkDesc::Spec {
                ctype: "GAME-STATE".into(),
                slots,
                auto_cmd: false,
                neg: Vec::new(),
                rels: Vec::new(),
            })?;
        }
        Ok(())
    }

    fn onehit_show_cards(&mut self, mcards: &[i64], ocard: i64) -> Result<String, EngineError> {
        let slots = self.onehit_game_slots("start", mcards, &[ocard], None, None);
        self.onehit_set_goal(&slots)?;
        self.last_output_key = None;
        let start = self.time_ms;
        let limit = start + 10_000;
        let mut key = None;
        while self.time_ms < limit {
            if !self.step_once()? {
                break;
            }
            if let Some((k, _)) = self.last_output_key.take() {
                key = Some(k);
                break;
            }
        }
        let key = key.ok_or_else(|| unimpl("onehit show-model-cards produced no output-key"))?;
        if self.time_ms < limit {
            self.run_until(limit as f64 / 1000.0)?;
        }
        Ok(key)
    }

    fn onehit_show_results(
        &mut self,
        mcards: &[i64],
        ocards: &[i64],
        mres: &str,
        ores: &str,
    ) -> Result<(), EngineError> {
        let slots = self.onehit_game_slots("results", mcards, ocards, Some(mres), Some(ores));
        self.onehit_set_goal(&slots)?;
        self.run_full_time(10.0)
    }

    /// Start `fan-no-pm`'s `fan-experiment` result recording. The task's
    /// `fan-sentence` reads `(run 30)` and the goal's `state` after each run;
    /// the spec harness replays those runs, so collection happens in
    /// [`Engine::run_secs`].
    pub fn fan_collect_start(&mut self) {
        self.fan_runs.clear();
        self.fan_collect = true;
    }

    /// Stop collecting and emit `fan-no-pm`'s `output-person-location`
    /// (`tutorial/lisp/fan-no-pm.lisp:96`). The first 18 runs are the person
    /// block and the next 18 the location block; `fan-experiment` averages
    /// them pairwise.
    pub fn fan_no_pm_summary(&mut self) {
        self.fan_collect = false;
        const DATA: [f64; 18] = [
            1.11, 1.17, 1.22, 1.17, 1.20, 1.22, 1.15, 1.23, 1.36, 1.20, 1.22, 1.26, 1.25, 1.36,
            1.29, 1.26, 1.47, 1.47,
        ];
        let runs = self.fan_runs.clone();
        if runs.len() < 36 {
            return;
        }
        let d: Vec<(f64, bool)> = (0..18)
            .map(|i| {
                let a = runs[i];
                let b = runs[i + 18];
                ((a.0 + b.0) / 2.0, a.1 && b.1)
            })
            .collect();
        let predicted: Vec<f64> = d.iter().map(|x| x.0).collect();
        let n = DATA.len() as f64;
        let sum_r: f64 = predicted.iter().sum();
        let sum_d: f64 = DATA.iter().sum();
        let sum_r2: f64 = predicted.iter().map(|x| x * x).sum();
        let sum_d2: f64 = DATA.iter().map(|x| x * x).sum();
        let sum_rd: f64 = predicted.iter().zip(DATA.iter()).map(|(a, b)| a * b).sum();
        let avg_r = sum_r / n;
        let avg_d = sum_d / n;
        let corr = (sum_rd / n - avg_r * avg_d)
            / ((sum_r2 / n - avg_r * avg_r).sqrt() * (sum_d2 / n - avg_d * avg_d).sqrt());
        let md = ((sum_r2 + sum_d2 - 2.0 * sum_rd) / n).sqrt();
        self.out_lines.push(format!("CORRELATION: {corr:6.3}"));
        self.out_lines.push(format!("MEAN DEVIATION: {md:6.3}"));
        self.out_lines.push(String::new());
        self.out_lines.push("TARGETS:".to_string());
        self.out_lines
            .push("                         Person fan".to_string());
        self.out_lines
            .push("  Location      1             2             3".to_string());
        self.out_lines.push("    fan".to_string());
        for i in 0..3 {
            self.fan_row(i, &d);
        }
        self.out_lines.push(String::new());
        self.out_lines.push("FOILS:".to_string());
        for i in 3..6 {
            self.fan_row(i, &d);
        }
    }

    fn fan_row(&mut self, row: usize, d: &[(f64, bool)]) {
        // Rows 0..3 are targets (the task expects `"k"`); rows 3..6 are
        // foils, where the recorded answer is correct only when it is not
        // `"k"`.
        let target = row < 3;
        let cells: Vec<String> = (0..3)
            .map(|j| {
                let (rt, answered_yes) = d[j + row * 3];
                let correct = if target { answered_yes } else { !answered_yes };
                let mark = if correct { "T" } else { "NIL" };
                format!("{rt:.3} ({mark:<3})")
            })
            .collect();
        let mut line = format!("     {}    ", row % 3 + 1);
        for cell in &cells {
            line.push_str("   ");
            line.push_str(cell);
        }
        self.out_lines.push(line);
    }

    pub fn paired_collect_start(&mut self) {
        self.paired_runs.clear();
        self.last_output_key = None;
        self.paired_collect = true;
    }

    /// `output-data`/`print-results` from `tutorial/lisp/paired.lisp:167`:
    /// average the per-block accuracy and correct-response latency over the
    /// experiment runs and compare with the original paired-associate data.
    pub fn paired_summary(&mut self) {
        self.paired_collect = false;
        const PAIRS: [(&str, &str); 20] = [
            ("bank", "0"),
            ("card", "1"),
            ("dart", "2"),
            ("face", "3"),
            ("game", "4"),
            ("hand", "5"),
            ("jack", "6"),
            ("king", "7"),
            ("lamb", "8"),
            ("mask", "9"),
            ("neck", "0"),
            ("pipe", "1"),
            ("quip", "2"),
            ("rope", "3"),
            ("sock", "4"),
            ("tent", "5"),
            ("vent", "6"),
            ("wall", "7"),
            ("xray", "8"),
            ("zinc", "9"),
        ];
        const DATA_LATENCY: [f64; 8] = [0.0, 2.158, 1.967, 1.762, 1.680, 1.552, 1.467, 1.402];
        const DATA_ACCURACY: [f64; 8] = [0.0, 0.526, 0.667, 0.798, 0.887, 0.924, 0.958, 0.954];
        let runs = self.paired_runs.clone();
        const PER_SUBJECT: usize = 160;
        let subjects = runs.len() / PER_SUBJECT;
        if subjects == 0 {
            return;
        }
        let mut accuracy = [0.0f64; 8];
        let mut latency = [0.0f64; 8];
        for s in 0..subjects {
            for block in 0..8 {
                let mut score = 0.0f64;
                let mut time = 0.0f64;
                for trial in 0..20 {
                    let run = &runs[s * PER_SUBJECT + block * 20 + trial];
                    let Some((_, answer)) = PAIRS
                        .iter()
                        .find(|(probe, _)| probe.eq_ignore_ascii_case(&run.probe))
                    else {
                        continue;
                    };
                    let Some((key, key_ms)) = &run.key else {
                        continue;
                    };
                    if key.eq_ignore_ascii_case(answer) {
                        score += 1.0;
                        time += (*key_ms - run.start_ms) as f64 / 1000.0;
                    }
                }
                accuracy[block] += score / 20.0;
                if score > 0.0 {
                    latency[block] += time / score;
                }
            }
        }
        let n = subjects as f64;
        let predicted_latency: Vec<f64> = latency.iter().map(|x| x / n).collect();
        let predicted_accuracy: Vec<f64> = accuracy.iter().map(|x| x / n).collect();
        self.out_lines.push(String::new());
        self.print_paired_results("Latency", &predicted_latency, &DATA_LATENCY);
        self.out_lines.push(String::new());
        self.print_paired_results("Accuracy", &predicted_accuracy, &DATA_ACCURACY);
    }

    fn print_paired_results(&mut self, label: &str, predicted: &[f64], data: &[f64]) {
        let n = predicted.len() as f64;
        let sum_r: f64 = predicted.iter().sum();
        let sum_d: f64 = data.iter().sum();
        let sum_r2: f64 = predicted.iter().map(|x| x * x).sum();
        let sum_d2: f64 = data.iter().map(|x| x * x).sum();
        let sum_rd: f64 = predicted.iter().zip(data.iter()).map(|(a, b)| a * b).sum();
        let avg_r = sum_r / n;
        let avg_d = sum_d / n;
        let denom = (sum_r2 / n - avg_r * avg_r).sqrt() * (sum_d2 / n - avg_d * avg_d).sqrt();
        let corr = if denom == 0.0 {
            0.0
        } else {
            (sum_rd / n - avg_r * avg_d) / denom
        };
        let md = ((sum_r2 + sum_d2 - 2.0 * sum_rd) / n).sqrt();
        self.out_lines.push(format!("{label}:"));
        self.out_lines.push(format!("CORRELATION: {corr:6.3}"));
        self.out_lines.push(format!("MEAN DEVIATION: {md:6.3}"));
        self.out_lines
            .push("Trial    1       2       3       4       5       6       7       8".to_string());
        let vals: Vec<String> = predicted.iter().map(|x| format!("{x:.3}")).collect();
        self.out_lines.push(format!("        {}", vals.join("   ")));
    }

    /// `siegler-experiment` (`tutorial/lisp/siegler.lisp`): run the fixed
    /// addition questions, collect the model's spoken answers, and print the
    /// response matrix plus correlation/RMSD against the experiment data.
    #[allow(clippy::too_many_arguments)]
    pub fn siegler_driver(
        &mut self,
        first_seed: u64,
        sets: u32,
        rt: Option<f32>,
        mp: Option<f32>,
        ans: Option<f32>,
        esc: Option<bool>,
        fact_base: Option<f32>,
        all_base: Option<f32>,
    ) -> Result<(), EngineError> {
        const PAIRS: [(i64, i64); 6] = [(1, 1), (1, 2), (1, 3), (2, 2), (2, 3), (3, 3)];
        const WORDS: [&str; 9] = [
            "ZERO", "ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT",
        ];
        let mut counts = [[0u32; 10]; 6];
        let mut seed = first_seed;
        for _ in 0..sets {
            for (row, &(a, b)) in PAIRS.iter().enumerate() {
                self.reset()?;
                self.set_seed((seed, 0));
                seed += 1;
                if let Some(v) = rt {
                    self.apply_sgp(":RT", &Sexp::Float(v as f64))?;
                }
                if let Some(v) = mp {
                    self.apply_sgp(":MP", &Sexp::Float(v as f64))?;
                }
                if let Some(v) = ans {
                    self.apply_sgp(":ANS", &Sexp::Float(v as f64))?;
                }
                if let Some(v) = esc {
                    self.apply_sgp(":ESC", &Sexp::Sym(if v { "T" } else { "NIL" }.to_string()))?;
                }
                if let Some(level) = all_base {
                    self.siegler_set_all_base(level);
                }
                if let Some(level) = fact_base {
                    self.siegler_set_fact_base(level);
                }
                self.trace_on = false;
                self.new_digit_sound(a, self.time_ms);
                self.new_digit_sound(b, self.time_ms + 750);
                self.run_secs(30.0)?;
                let response = self
                    .last_speech
                    .as_ref()
                    .map(|(_, text)| text.to_ascii_uppercase());
                let col = response
                    .as_deref()
                    .and_then(|r| WORDS.iter().position(|w| *w == r))
                    .unwrap_or(9);
                counts[row][col] += 1;
            }
        }
        let n = sets as f64;
        let mut results = [[0.0f64; 10]; 6];
        for (row, out) in results.iter_mut().enumerate() {
            for (col, slot) in out.iter_mut().enumerate() {
                *slot = counts[row][col] as f64 / n;
            }
        }
        self.siegler_results(&results);
        Ok(())
    }

    fn siegler_set_all_base(&mut self, level: f32) {
        let n = level.max(0.0).round() as usize;
        for h in &mut self.hist {
            h.creation_ms = 0;
            h.count = n;
            h.fixed_base = Some(level);
            h.refs.clear();
        }
    }

    fn siegler_set_fact_base(&mut self, level: f32) {
        let n = level.max(0.0).round() as usize;
        for name in [
            "F00", "F01", "F02", "F03", "F04", "F10", "F11", "F12", "F13", "F20", "F21", "F22",
            "F30", "F31", "F40",
        ] {
            if let Some(i) = self.dm.iter().position(|c| c.name.as_deref() == Some(name)) {
                self.hist[i].creation_ms = 0;
                self.hist[i].count = n;
                self.hist[i].fixed_base = Some(level);
                self.hist[i].refs.clear();
            }
        }
    }

    fn siegler_results(&mut self, results: &[[f64; 10]; 6]) {
        let data = [
            [0.0, 0.05, 0.86, 0.0, 0.02, 0.0, 0.02, 0.0, 0.0, 0.06],
            [0.0, 0.04, 0.07, 0.75, 0.04, 0.0, 0.02, 0.0, 0.0, 0.09],
            [0.0, 0.02, 0.0, 0.10, 0.75, 0.05, 0.01, 0.03, 0.0, 0.06],
            [0.02, 0.0, 0.04, 0.05, 0.80, 0.04, 0.0, 0.05, 0.0, 0.0],
            [0.0, 0.0, 0.07, 0.09, 0.25, 0.45, 0.08, 0.01, 0.01, 0.06],
            [0.04, 0.0, 0.0, 0.05, 0.21, 0.09, 0.48, 0.0, 0.02, 0.11],
        ];
        let mut sum_r = 0.0;
        let mut sum_d = 0.0;
        let mut sum_r2 = 0.0;
        let mut sum_d2 = 0.0;
        let mut sum_rd = 0.0;
        let mut count = 0.0;
        for (r, d) in results.iter().zip(data.iter()) {
            for (a, b) in r.iter().zip(d.iter()) {
                sum_r += *a;
                sum_d += *b;
                sum_r2 += *a * *a;
                sum_d2 += *b * *b;
                sum_rd += *a * *b;
                count += 1.0;
            }
        }
        let avg_r = sum_r / count;
        let avg_d = sum_d / count;
        let corr = (sum_rd / count - avg_r * avg_d)
            / ((sum_r2 / count - avg_r * avg_r).sqrt() * (sum_d2 / count - avg_d * avg_d).sqrt());
        let md = ((sum_r2 + sum_d2 - 2.0 * sum_rd) / count).sqrt();
        self.out_lines.push(format!("CORRELATION: {corr:6.3}"));
        self.out_lines.push(format!("MEAN DEVIATION: {md:6.3}"));
        self.out_lines
            .push("       0     1     2     3     4     5     6     7     8   Other".to_string());
        for (i, q) in ["1+1", "1+2", "1+3", "2+2", "2+3", "3+3"]
            .iter()
            .enumerate()
        {
            let mut line = (*q).to_string();
            for v in results[i] {
                line.push_str(&format!("{v:6.2}"));
            }
            self.out_lines.push(line);
        }
    }
    /// `(goal-focus-fct (car (define-chunks-fct '(...))))`: define a named
    /// chunk (`{CTYPE}{n}`) and focus it.
    pub(super) fn zz_goal_focus(
        &mut self,
        ctype: &str,
        slots: Vec<(String, Value)>,
    ) -> Result<(), EngineError> {
        let k = *self.name_count.entry(ctype.to_string()).or_insert(0);
        self.name_count.insert(ctype.to_string(), k + 1);
        let name = format!("{ctype}{k}");
        let inst = ChunkInst {
            name: Some(name.clone()),
            ctype: ctype.to_string(),
            slots,
            bare: false,
        };
        self.named.insert(name.clone(), inst);
        self.goal_focus(&ChunkDesc::Named(name))
    }

    /// `run-test` testing defaults (`test-model.lisp:67`): reset before each
    /// test, then apply the test's overrides.
    pub(super) fn zz_run_test_defaults(&mut self) {
        self.cfg.zz_kct_ms = 10;
        self.cfg.zz_krt_ms = 40;
        self.cfg.zz_psd = None;
        self.cfg.zz_cdd = None;
        self.cfg.zz_two_proc = false;
        self.cfg.zz_two_exec = true;
        self.cfg.zz_query = "STATE".to_string();
        // `run-test` (`test-model.lisp:73`): `*dpd* .075` / `*fpd* .05` /
        // `*spd* .1`.
        self.cfg.zz_dpd_ms = 75;
        self.cfg.zz_fpd_ms = 50;
        self.cfg.zz_spd_ms = 100;
    }

    pub(super) fn zz_test1(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        for hand in ["same", "diff"] {
            for query in ["state", "processor", "preparation"] {
                for processors in [true, false] {
                    for executions in [true, false] {
                        self.cfg.zz_two_proc = processors;
                        self.cfg.zz_two_exec = executions;
                        self.cfg.zz_query = query.to_uppercase();
                        self.reset()?;
                        self.out_lines.push("########################".to_string());
                        self.out_lines.push(format!(
                            "\"{} hand {} processors {} executions query {}\"",
                            hand.to_uppercase(),
                            if processors { 2 } else { 1 },
                            if executions { 2 } else { 1 },
                            query.to_uppercase()
                        ));
                        self.out_lines.push("########################".to_string());
                        self.zz_goal_focus(
                            "GOAL",
                            vec![
                                ("TEST".to_string(), Value::Sym("TEST1".to_string())),
                                ("PARAM".to_string(), Value::Sym(hand.to_uppercase())),
                            ],
                        )?;
                        let d = format!(
                            "{} hand {} processors {} executions query {}",
                            hand.to_uppercase(),
                            if processors { 2 } else { 1 },
                            if executions { 2 } else { 1 },
                            query.to_uppercase()
                        );
                        self.zz_data.push(("TEST".to_string(), 0, d));
                        self.run_secs(10.0)?;
                    }
                }
            }
        }
        self.zz_summarize();
        Ok(())
    }

    pub(super) fn zz_test2(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        for (cdd, disp) in [
            (None, "NIL"),
            (Some(0.8), "T"),
            (Some(0.5), "0.5"),
            (Some(1.0), "1.0"),
        ] {
            self.cfg.zz_cdd = cdd;
            self.reset()?;
            self.zz_start_hand_at_mouse();
            let d = format!("click and hold mouse *cdd* {disp}");
            self.zz_data.push(("TEST".to_string(), 0, d));
            self.zz_goal_focus(
                "GOAL",
                vec![("TEST".to_string(), Value::Sym("TEST2".to_string()))],
            )?;
            self.run_secs(10.0)?;
        }
        self.zz_summarize();
        Ok(())
    }

    /// `(start-hand-at-mouse)` (`mouse.lisp:554`): put the right hand at the
    /// mouse cursor's current position.
    pub(super) fn zz_start_hand_at_mouse(&mut self) {
        let (mx, my) = self.zz_mouse_cursor;
        self.zz_hands[1].loc = (mx, my);
    }

    pub(super) fn zz_test3(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.cfg.zz_kct_ms = 25;
        self.cfg.zz_krt_ms = 25;
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Auto release key & mouse when moving hand".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "GOAL",
            vec![("TEST".to_string(), Value::Sym("TEST3".to_string()))],
        )?;
        self.run_secs(0.5)?;
        self.zz_buffer_status();
        self.run_secs(0.75)?;
        self.zz_buffer_status();
        self.run_secs(5.0)?;
        self.zz_summarize();
        Ok(())
    }

    pub(super) fn zz_test4(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.cfg.zz_psd = Some(0.2);
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Some various key pressing".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "GOAL",
            vec![("TEST".to_string(), Value::Sym("TEST4".to_string()))],
        )?;
        self.zz_breaks = vec!["TEST4-7".to_string()];
        self.run_secs(10.0)?;
        self.zz_buffer_status();
        self.zz_print_finger_locs();
        self.run_secs(10.0)?;
        self.zz_buffer_status();
        self.zz_print_finger_locs();
        self.zz_summarize();
        Ok(())
    }

    /// `test5` (`test-model.lisp:188`): `(do ((time (run 10) (run 10))) ((= time
    /// 0.0)) ...)` — each run stops at a `pbreak`, and the body dumps the
    /// manual buffers and finger locations after every non-empty run.
    pub(super) fn zz_test5(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Move the fingers and hands around".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "GOAL",
            vec![("TEST".to_string(), Value::Sym("TEST5".to_string()))],
        )?;
        self.zz_breaks = vec![
            "TEST5-5".to_string(),
            "TEST5-9".to_string(),
            "TEST5-11".to_string(),
            "TEST5-12".to_string(),
        ];
        loop {
            let ran = self.zz_run_step(10.0)?;
            if !ran {
                break;
            }
            self.zz_buffer_status();
            self.zz_print_finger_locs();
        }
        self.zz_summarize();
        Ok(())
    }

    /// `test6` (`test-model.lisp:202`): a single `(run 10)` exercising
    /// `type-key` from a non-home position, then the buffer/finger dump.
    pub(super) fn zz_test6(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Some type-key actions from non-home position.".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "GOAL",
            vec![("TEST".to_string(), Value::Sym("TEST6".to_string()))],
        )?;
        self.run_secs(10.0)?;
        self.zz_buffer_status();
        self.zz_print_finger_locs();
        self.zz_summarize();
        Ok(())
    }

    /// `test7` (`test-model.lisp:212`): two runs of `delayed-punch` actions,
    /// the second with faster punch-delay parameters.
    pub(super) fn zz_test7(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Default timed delayed-punch actions".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "CHUNK",
            vec![("TEST".to_string(), Value::Sym("TEST7".to_string()))],
        )?;
        self.run_secs(10.0)?;
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Faster timed delayed-punch actions".to_string(),
        ));
        self.cfg.zz_dpd_ms = 50;
        self.cfg.zz_fpd_ms = 25;
        self.cfg.zz_spd_ms = 70;
        self.reset()?;
        self.zz_goal_focus(
            "CHUNK",
            vec![("TEST".to_string(), Value::Sym("TEST7".to_string()))],
        )?;
        self.run_secs(10.0)?;
        self.zz_summarize();
        Ok(())
    }

    /// `test8` (`test-model.lisp:225`): hold several fingers down, then
    /// `all-fingers-to-home`, then dump buffers/finger locations.
    pub(super) fn zz_test8(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "All home with multiple fingers down".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "CHUNK",
            vec![("TEST".to_string(), Value::Sym("TEST8".to_string()))],
        )?;
        self.run_secs(10.0)?;
        self.zz_buffer_status();
        self.zz_print_finger_locs();
        self.zz_summarize();
        Ok(())
    }

    /// `test9` (`test-model.lisp:236`): like test8 but `release-all-fingers`
    /// (including the second, no-op one that prints a warning).
    pub(super) fn zz_test9(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Release-all-fingers with multiple fingers down".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "CHUNK",
            vec![("TEST".to_string(), Value::Sym("TEST9".to_string()))],
        )?;
        self.run_secs(10.0)?;
        self.zz_buffer_status();
        self.zz_print_finger_locs();
        self.zz_summarize();
        Ok(())
    }

    /// `(buffer-status manual-left manual-right)` (`motor-extension.lisp:1306`,
    /// `generic-state-query` at 1337).
    pub(super) fn zz_buffer_status(&mut self) {
        for (name, hand) in [("MANUAL-LEFT", 0usize), ("MANUAL-RIGHT", 1)] {
            let h = &self.zz_hands[hand];
            let proc = self.man_proc[hand];
            let exec = self.man_exec[hand];
            let state_busy = proc || exec;
            self.out_lines.push(format!("{name}:"));
            self.out_lines
                .push("  buffer empty          : T".to_string());
            self.out_lines
                .push("  buffer full           : NIL".to_string());
            self.out_lines
                .push("  buffer failure        : NIL".to_string());
            self.out_lines
                .push("  buffer requested      : NIL".to_string());
            self.out_lines
                .push("  buffer unrequested    : NIL".to_string());
            self.out_lines.push(format!(
                "  state free            : {}",
                if state_busy { "NIL" } else { "T" }
            ));
            self.out_lines.push(format!(
                "  state busy            : {}",
                if state_busy { "T" } else { "NIL" }
            ));
            self.out_lines
                .push("  state error           : NIL".to_string());
            self.out_lines.push(format!(
                "  processor free        : {}",
                if proc { "NIL" } else { "T" }
            ));
            self.out_lines.push(format!(
                "  processor busy        : {}",
                if proc { "T" } else { "NIL" }
            ));
            self.out_lines.push(format!(
                "  execution free        : {}",
                if exec { "NIL" } else { "T" }
            ));
            self.out_lines.push(format!(
                "  execution busy        : {}",
                if exec { "T" } else { "NIL" }
            ));
            let last = self.zz_last_cmd[hand].clone();
            self.out_lines
                .push(format!("  last-command          : {last}"));
            for (i, fname) in ["index", "middle", "ring", "pinkie", "thumb"]
                .iter()
                .enumerate()
            {
                self.out_lines.push(format!(
                    "  {fname:<7}free           : {}",
                    if h.busy[i] { "NIL" } else { "T" }
                ));
                self.out_lines.push(format!(
                    "  {fname:<7}down           : {}",
                    if h.down[i] { "T" } else { "NIL" }
                ));
            }
            self.out_lines.push(String::new());
        }
    }

    /// The test driver's `(finger-loc-m …)` dump (`test-model.lisp:178`).
    pub(super) fn zz_print_finger_locs(&mut self) {
        for (hand, hname) in [(0usize, "LEFT"), (1, "RIGHT")] {
            for (f, fname) in ["INDEX", "MIDDLE", "RING", "PINKIE", "THUMB"]
                .iter()
                .enumerate()
            {
                let (x, y) = self.zz_finger_loc(hand, f);
                self.out_lines
                    .push(format!("  {hname} {fname}: #({x} {y})"));
            }
        }
    }

    /// `print-instantiation` (`procedural.lisp:2441`): the selected
    /// production with its matched bindings, as a `pbreak` prints it.
    pub(super) fn zz_print_instantiation(&mut self, prod: usize, bindings: &Bindings) {
        let p = self.model.productions[prod].clone();
        let mut lines = vec![format!("(P {}", p.name)];
        for item in &p.lhs {
            match item {
                LhsItem::Match { buffer, tests } => {
                    lines.push(format!("   ={buffer}>"));
                    for t in tests {
                        if t.slot.eq_ignore_ascii_case("ISA") {
                            continue;
                        }
                        lines.push(format!(
                            "       {} {}",
                            slot_disp(&t.slot, bindings),
                            show_pat(&t.value, bindings)
                        ));
                    }
                }
                LhsItem::Query { buffer, queries } => {
                    lines.push(format!("   ?{buffer}>"));
                    for (slot, val) in queries {
                        lines.push(format!(
                            "       {} {}",
                            slot_disp(slot, bindings),
                            show_pat(val, bindings)
                        ));
                    }
                }
                _ => {}
            }
        }
        lines.push(" ==>".to_string());
        for item in &p.rhs {
            match item {
                RhsAction::Request { buffer, spec } => {
                    lines.push(format!("   +{buffer}>"));
                    if let ChunkDesc::Spec { ctype, slots, .. } = spec {
                        // `production-statement-text` (`procedural.lisp:2555`)
                        // prints the spec's slots, then an explicit request
                        // type as `CMD`; the compiler's auto-added `CMD` is not
                        // part of the source instantiation.
                        for (slot, val) in
                            slots.iter().filter(|(s, _)| !s.eq_ignore_ascii_case("CMD"))
                        {
                            lines.push(format!(
                                "       {} {}",
                                slot_disp(slot, bindings),
                                show_pat(val, bindings)
                            ));
                        }
                        if !buffer.eq_ignore_ascii_case("RETRIEVAL") {
                            for (slot, val) in
                                slots.iter().filter(|(s, _)| s.eq_ignore_ascii_case("CMD"))
                            {
                                lines.push(format!(
                                    "       {} {}",
                                    slot_disp(slot, bindings),
                                    show_pat(val, bindings)
                                ));
                            }
                            if !ctype.is_empty()
                                && !slots.iter().any(|(s, _)| s.eq_ignore_ascii_case("CMD"))
                            {
                                lines.push(format!("       CMD {ctype}"));
                            }
                        }
                    }
                }
                RhsAction::Clear { buffer } => lines.push(format!("   -{buffer}>")),
                RhsAction::Modify { buffer, slots } => {
                    lines.push(format!("   ={buffer}>"));
                    for (slot, val) in slots {
                        lines.push(format!(
                            "       {} {}",
                            slot_disp(slot, bindings),
                            show_pat(val, bindings)
                        ));
                    }
                }
                _ => {}
            }
        }
        lines.push(")".to_string());
        self.out_lines.extend(lines);
    }
}

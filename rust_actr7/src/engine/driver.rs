//! Task drivers (output, goal-focus, zz/al drivers).
use super::compilation::show_pat;
use super::dm::{al_history, al_needed_refs, bl_equation, exp_coerced, log_coerced};
use super::rhs::new_chunk;
use super::*;
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
                    // `define-chunks-fct` auto-names a spec by its type
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

        // `:nsji`: `nil` (the default) clamps negatives to 0; `t` keeps them.
        if sji < 0.0 && !self.cfg.nsji {
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
                            t.slot,
                            show_pat(&t.value, bindings)
                        ));
                    }
                }
                LhsItem::Query { buffer, queries } => {
                    lines.push(format!("   ?{buffer}>"));
                    for (slot, val) in queries {
                        lines.push(format!("       {slot} {}", show_pat(val, bindings)));
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
                        // prints the spec's slots, then the request type as
                        // `CMD`.
                        for (slot, val) in slots {
                            lines.push(format!("       {slot} {}", show_pat(val, bindings)));
                        }
                        if !ctype.is_empty() {
                            lines.push(format!("       CMD {ctype}"));
                        }
                    }
                }
                RhsAction::Clear { buffer } => lines.push(format!("   -{buffer}>")),
                RhsAction::Modify { buffer, slots } => {
                    lines.push(format!("   ={buffer}>"));
                    for (slot, val) in slots {
                        lines.push(format!("       {slot} {}", show_pat(val, bindings)));
                    }
                }
                _ => {}
            }
        }
        lines.push(")".to_string());
        self.out_lines.extend(lines);
    }
}

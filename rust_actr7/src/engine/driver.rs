//! Task drivers (output, goal-focus, zz/al/zbrodoff drivers).
use super::compilation::{show_pat, slot_disp};
use super::dm::{al_history, al_needed_refs, bl_equation, exp_coerced, log_coerced};
use super::rhs::new_chunk;
use super::*;
use crate::json::Json;
use crate::sexpr::Sexp;
use crate::{BstMode, OnehitMode, PcompGameConfig, ZbrodoffMode};

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

const PAST_WORDS: [(u64, &str, &str, &str); 22] = [
    (12458, "HAVE", "HAD", "BLANK"),
    (16825, "DO", "DID", "BLANK"),
    (19137, "MAKE", "MADE", "BLANK"),
    (20623, "GET", "GOT", "BLANK"),
    (21639, "USE", "USE", "ED"),
    (22549, "LOOK", "LOOK", "ED"),
    (23380, "SEEM", "SEEM", "ED"),
    (24139, "TELL", "TOLD", "BLANK"),
    (24779, "SHOW", "SHOW", "ED"),
    (25410, "WANT", "WANT", "ED"),
    (26037, "CALL", "CALL", "ED"),
    (26649, "ASK", "ASK", "ED"),
    (27215, "TURN", "TURN", "ED"),
    (27755, "FOLLOW", "FOLLOW", "ED"),
    (28251, "WORK", "WORK", "ED"),
    (28723, "LIVE", "LIVE", "ED"),
    (29195, "TRY", "TRY", "ED"),
    (29663, "STAND", "STOOD", "BLANK"),
    (30110, "MOVE", "MOVE", "ED"),
    (30523, "NEED", "NEED", "ED"),
    (30909, "START", "START", "ED"),
    (31183, "LOSE", "LOST", "BLANK"),
];

fn past_sample(rng: &std::cell::RefCell<Mt>) -> (u64, &'static str, &'static str, &'static str) {
    let n = rng.borrow_mut().random_int(31183);
    PAST_WORDS
        .iter()
        .find(|(cum, ..)| n < *cum)
        .copied()
        .unwrap_or(PAST_WORDS[0])
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
                        copy_name: None,
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

    /// One `past-tense-trials` trial for the warning cases. The task samples
    /// two presentation chunks and one target from the tutorial's frequency
    /// table, then runs the standard 200 s trial window.
    pub fn past_tense_warning_trial(&mut self, override_buffers: bool) -> Result<(), EngineError> {
        self.reset()?;
        for _ in 0..2 {
            let (_, verb, stem, suffix) = past_sample(&self.rng);
            let name = format!("PAST-TENSE{}", self.dm.len());
            self.add_runtime_dm(ChunkInst {
                name: Some(name),
                ctype: "PAST-TENSE".into(),
                slots: vec![
                    ("VERB".into(), Value::Sym(verb.into())),
                    ("STEM".into(), Value::Sym(stem.into())),
                    ("SUFFIX".into(), Value::Sym(suffix.into())),
                ],
                bare: false,
            });
        }
        let (_, target, _, _) = past_sample(&self.rng);
        self.set_buffer_chunk_desc(
            "IMAGINAL",
            &ChunkDesc::Spec {
                ctype: "PAST-TENSE".into(),
                slots: vec![("VERB".into(), Pattern::Sym(target.into()))],
                auto_cmd: false,
                neg: Vec::new(),
                rels: Vec::new(),
            },
        )?;
        self.goal_focus(&ChunkDesc::Spec {
            ctype: "GOAL".into(),
            slots: vec![("STATE".into(), Pattern::Sym("START".into()))],
            auto_cmd: false,
            neg: Vec::new(),
            rels: Vec::new(),
        })?;
        if override_buffers {
            self.push(
                1000,
                Prio::Num(0),
                Out::Low,
                Event::ModBuffer {
                    buffer: "GOAL".into(),
                    slots: vec![("STATE".into(), Value::Sym("DONE".into()))],
                    module: "NONE".into(),
                },
            );
            self.push(
                1000,
                Prio::Num(0),
                Out::Low,
                Event::ModBuffer {
                    buffer: "IMAGINAL".into(),
                    slots: vec![
                        ("VERB".into(), Value::Sym("USE".into())),
                        ("STEM".into(), Value::Sym("USE".into())),
                        ("SUFFIX".into(), Value::Sym("ED".into())),
                    ],
                    module: "NONE".into(),
                },
            );
        }
        self.reward_seen = false;
        let old_trace = self.trace_on;
        self.trace_on = false;
        self.run_secs(200.0)?;
        self.trace_on = old_trace;
        let produced = self
            .buffers
            .get("IMAGINAL")
            .and_then(|b| b.chunk.as_ref())
            .map(|c| {
                let val = |name: &str| {
                    c.slots
                        .iter()
                        .find(|(slot, _)| slot == name)
                        .map(|(_, v)| v.show())
                        .unwrap_or_else(|| "NIL".into())
                };
                (val("VERB"), val("STEM"), val("SUFFIX"))
            })
            .unwrap_or_else(|| ("NIL".into(), "NIL".into(), "NIL".into()));
        if produced.0 != target {
            self.out_lines.push(format!(
                "#|Warning: Incorrectly formed verb.  Presented {} and produced VERB {} STEM {} SUFFIX {}. |#",
                target, produced.0, produced.1, produced.2
            ));
        }
        if !self.reward_seen {
            self.out_lines.push(format!(
                "#|Warning: Model did not receive a reward when given {}. |#",
                target
            ));
        }
        Ok(())
    }
    fn clear_test_productions(&mut self) {
        self.out_lines
            .push("#|Warning: Clearing the productions is not recommended |#".to_string());
        self.model.productions.clear();
        self.exec_prods.clear();
        self.utilities.clear();
        self.production_utility.clear();
        self.disabled_prods.clear();
        self.last_conflict_set.clear();
    }

    fn define_child_prod(&mut self, text: &str) -> Result<(), EngineError> {
        self.define_production_text(text)?;
        self.utilities.push(self.cfg.iu);
        self.production_utility.push(None);
        Ok(())
    }

    fn strip_activation_detail(&mut self, start: usize) {
        let lines = self.out_lines.split_off(start);
        self.out_lines.extend(lines.into_iter().filter(|line| {
            let t = line.trim_start();
            !(t.starts_with("Computing activation for chunk")
                || t.starts_with("Computing base-level")
                || t.starts_with("Starting with blc:")
                || t.starts_with("User provided chunk base-level:")
                || t.starts_with("base-level value:")
                || t.starts_with("Total base-level:")
                || t.starts_with("Adding transient noise")
                || t.starts_with("Adding permanent noise")
                || (t.starts_with("Chunk ") && t.contains(" has an activation of:"))
                || t.starts_with("Chunk ")
                    && (t.contains(" has the current best activation")
                        || t.contains(" is now the current best with activation")
                        || t.contains(" matches the current best activation")))
        }));
    }
    fn goal_focus_test_chunk(&mut self, name: &str) -> Result<(), EngineError> {
        let mut inst = self
            .named
            .get(name)
            .or_else(|| self.dm.iter().find(|c| c.name.as_deref() == Some(name)))
            .cloned()
            .ok_or_else(|| bad(format!("no chunk named {name}")))?;
        let reuse = self.reuse_buffer_name("GOAL");
        inst.name = Some(reuse.clone());
        self.copy_source.insert(reuse, name.to_string());
        self.now(
            Prio::Max,
            Out::Low,
            Event::SetBuffer {
                buffer: "GOAL".to_string(),
                chunk: NewChunk::Instance(inst),
                requested: false,
                shown: Some(name.to_string()),
            },
        );
        self.now_m(Prio::Max, Out::Never, true, Event::Dummy);
        Ok(())
    }
    fn define_test_chunk(&mut self, name: &str, count: i64) {
        self.named.insert(
            name.to_string(),
            ChunkInst {
                name: Some(name.to_string()),
                ctype: "TEST".into(),
                slots: vec![("COUNT".into(), Value::Int(count))],
                bare: false,
            },
        );
        self.chunk_names.insert(name.to_string());
    }

    /// `testing-child-slot`'s `run-it`: matching, retrieval, then imaginal
    /// production sets over the parent/child chunk types.
    pub fn testing_child_slot(&mut self) -> Result<(), EngineError> {
        self.set_detail(Detail::Medium);
        self.set_buffer_chunk_desc(
            "IMAGINAL",
            &ChunkDesc::Spec {
                ctype: "TARGET".into(),
                slots: vec![("SLOT1".into(), Pattern::Sym("SLOT1".into()))],
                auto_cmd: false,
                neg: Vec::new(),
                rels: Vec::new(),
            },
        )?;

        self.clear_test_productions();
        self.define_child_prod("(p slot1-empty\n =goal>\n isa parent\n slot1 nil\n ==>)")?;
        self.define_child_prod("(p slot1-filled\n =goal>\n isa parent\n slot1 =val\n ==>)")?;
        self.define_child_prod(
            "(p slot1-filled*\n =goal>\n isa parent\n =slot =val\n =imaginal>\n isa target\n slot1 =slot\n ==>)",
        )?;
        for goal in ["PARENT", "TARGET-NIL", "TARGET-FULL", "OTHER"] {
            self.overwrite_buffer_chunk_named("GOAL", goal)?;
            self.out_lines.push("+++++++++++++++++++++++++".to_string());
            self.print_buffer_chunk(&["GOAL".to_string()]);
            self.print_whynot(&[]);
        }

        self.clear_test_productions();
        self.define_child_prod(
            "(p retrieve0\n =goal>\n isa test\n count 0\n ==>\n -goal>\n +retrieval>\n isa target\n slot1 nil)",
        )?;
        self.define_child_prod(
            "(p retrieve1\n =goal>\n isa test\n count 1\n ==>\n -goal>\n +retrieval>\n isa parent\n slot1 nil)",
        )?;
        self.define_child_prod(
            "(p retrieve2\n =goal>\n isa test\n count 2\n ==>\n -goal>\n +retrieval>\n isa target\n - slot1 nil)",
        )?;
        self.define_child_prod(
            "(p retrieve3\n =goal>\n isa test\n count 3\n ==>\n -goal>\n +retrieval>\n isa parent\n - slot1 nil)",
        )?;
        self.define_child_prod(
            "(p retrieve4\n =goal>\n isa test\n count 4\n ==>\n -goal>\n +retrieval>\n isa target\n slot1 t)",
        )?;
        self.define_child_prod(
            "(p retrieve5\n =goal>\n isa test\n count 5\n ==>\n -goal>\n +retrieval>\n isa parent\n slot1 t)",
        )?;
        self.define_child_prod(
            "(p retrieve6\n =goal>\n isa test\n count 6\n =imaginal>\n isa target\n slot1 =slot\n ==>\n -goal>\n =imaginal>\n +retrieval>\n isa parent\n =slot t)",
        )?;
        self.define_child_prod(
            "(p retrieve7\n =goal>\n isa test\n count 7\n =imaginal>\n isa target\n slot1 =slot\n ==>\n -goal>\n =imaginal>\n +retrieval>\n isa target\n =slot t)",
        )?;
        self.define_child_prod(
            "(p retrieve8\n =goal>\n isa test\n count 8\n =imaginal>\n isa target\n slot1 =slot\n ==>\n -goal>\n =imaginal>\n +retrieval>\n isa other\n =slot t)",
        )?;
        for i in 0..9i64 {
            let name = format!("TEST{i}");
            self.define_test_chunk(&name, i);
            self.goal_focus_test_chunk(&name)?;
            self.print_productions(&[format!("RETRIEVE{i}")]);
            let mark = self.out_len();
            self.run_secs(10.0)?;
            self.strip_activation_detail(mark);
        }

        self.clear_test_productions();
        for (i, body) in [
            "(p imaginal0\n =goal>\n isa test\n count 0\n ==>\n -goal>\n +imaginal>\n isa parent\n slot1 t)",
            "(p imaginal1\n =goal>\n isa test\n count 1\n ==>\n -goal>\n +imaginal>\n isa target\n slot1 t)",
            "(p imaginal2\n =goal>\n isa test\n count 2\n =imaginal>\n isa target\n slot1 =slot\n ==>\n -goal>\n +imaginal>\n isa parent\n =slot t)",
            "(p imaginal3\n =goal>\n isa test\n count 3\n =imaginal>\n isa target\n slot1 =slot\n ==>\n -goal>\n +imaginal>\n isa target\n =slot t)",
            "(p imaginal4\n =goal>\n isa test\n count 4\n =imaginal>\n isa target\n slot1 =slot\n ==>\n -goal>\n +imaginal>\n isa other\n =slot t)",
        ]
        .into_iter()
        .enumerate()
        {
            self.define_child_prod(body)?;
            let _ = i;
        }
        let old_trace = self.trace_on;
        self.trace_on = true;
        self.define_child_prod(
            "(p imaginal-bad\n =goal>\n isa test\n count nil\n ==>\n -goal>\n +imaginal>\n isa other\n slot1 t)",
        )?;
        self.trace_on = old_trace;
        let old_detail = self.cfg.detail;
        self.set_detail(Detail::High);
        for i in 0..5i64 {
            let name = format!("TEST{}", 9 + i);
            self.define_test_chunk(&name, i);
            self.goal_focus_test_chunk(&name)?;
            self.set_buffer_chunk_desc(
                "IMAGINAL",
                &ChunkDesc::Spec {
                    ctype: "TARGET".into(),
                    slots: vec![("SLOT1".into(), Pattern::Sym("SLOT1".into()))],
                    auto_cmd: false,
                    neg: Vec::new(),
                    rels: Vec::new(),
                },
            )?;
            self.print_productions(&[format!("IMAGINAL{i}")]);
            self.run_secs(10.0)?;
        }
        self.set_detail(old_detail);
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
            decays: None,
            fixed_base: None,
            last_retrieval: None,
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
        let base = self.base_level(idx, self.cfg.bll, false);
        let spread = self.spreading_activation(name, self.cfg.mas);
        (base + spread + self.inhibition_offset(&self.hist[idx].refs)) as f64
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

    /// `zbrodoff-problem` (`tutorial/lisp/zbrodoff.lisp:246`): construct and
    /// run one trial, leaving the model's imaginal buffer for a following
    /// `sdm`/`buffer-chunk` dump.
    pub fn zbrodoff_problem(
        &mut self,
        addend1: &str,
        addend2: &str,
        sum: &str,
        answer: &str,
    ) -> Result<(), EngineError> {
        self.zbrodoff_reset(100)?;
        let trial = ZbrodoffTrial {
            block: 1,
            addend: addend2.parse().unwrap_or(2),
            text: format!("{addend1} + {addend2} = {sum}"),
            answer: answer.to_string(),
        };
        self.zbrodoff_run_trials(&[trial])?;
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

    /// Building-sticks experiment (`bst.lisp` / `bst-ppm.lisp`): 100 model
    /// runs over the 15 experimental stick triples.
    pub fn bst_driver(&mut self, mode: BstMode) -> Result<(), EngineError> {
        const STIMS: [(i64, i64, i64, i64); 15] = [
            (15, 250, 55, 125),
            (10, 155, 22, 101),
            (14, 200, 37, 112),
            (22, 200, 32, 114),
            (10, 243, 37, 159),
            (22, 175, 40, 73),
            (15, 250, 49, 137),
            (10, 179, 32, 105),
            (20, 213, 42, 104),
            (14, 237, 51, 116),
            (12, 149, 30, 72),
            (14, 237, 51, 121),
            (22, 200, 32, 114),
            (14, 200, 37, 112),
            (15, 250, 55, 125),
        ];
        const DATA: [f64; 15] = [
            20.0, 67.0, 20.0, 47.0, 87.0, 20.0, 80.0, 93.0, 83.0, 13.0, 29.0, 27.0, 80.0, 73.0,
            53.0,
        ];
        let learn = matches!(mode, BstMode::Learn);
        let names: &[&str] = match mode {
            BstMode::Experiment | BstMode::Learn => {
                &["DECIDE-OVER", "DECIDE-UNDER", "FORCE-OVER", "FORCE-UNDER"]
            }
            BstMode::Ppm => &["DECIDE-OVER", "DECIDE-UNDER"],
        };
        let runs = if learn { 1 } else { 100 };
        let mut counts = [0u32; 15];
        let mut utilities = vec![0.0f64; names.len()];
        for run in 0..runs {
            self.reset()?;
            if learn {
                self.clear_device_script();
            }
            self.set_seed(if learn { (1, 0) } else { (101 + run as u64, 0) });
            self.set_trace(learn);
            for (i, &(a, b, c, goal)) in STIMS.iter().enumerate() {
                let choice = self.bst_trial(a, b, c, goal)?;
                if choice.as_deref() == Some("over") {
                    counts[i] += 1;
                }
            }
            for (total, name) in utilities.iter_mut().zip(names) {
                *total += self.bst_utility(name);
            }
        }
        let n = runs as f64;
        let percentages: Vec<f64> = counts.iter().map(|v| *v as f64 * 100.0 / n).collect();
        let mut sum_r = 0.0;
        let mut sum_d = 0.0;
        let mut sum_r2 = 0.0;
        let mut sum_d2 = 0.0;
        let mut sum_rd = 0.0;
        for (predicted, actual) in percentages.iter().zip(DATA.iter()) {
            sum_r += predicted;
            sum_d += actual;
            sum_r2 += predicted * predicted;
            sum_d2 += actual * actual;
            sum_rd += predicted * actual;
        }
        let m = DATA.len() as f64;
        let avg_r = sum_r / m;
        let avg_d = sum_d / m;
        let corr = (sum_rd / m - avg_r * avg_d)
            / ((sum_r2 / m - avg_r * avg_r).sqrt() * (sum_d2 / m - avg_d * avg_d).sqrt());
        let md = ((sum_r2 + sum_d2 - 2.0 * sum_rd) / m).sqrt();
        self.out_lines.push(format!("CORRELATION: {corr:6.3}"));
        self.out_lines.push(format!("MEAN DEVIATION: {md:6.3}"));
        self.out_lines.push(String::new());
        if learn {
            let header: String = (1..=15).map(|i| format!("{i:<8}")).collect();
            self.out_lines.push(format!("Trial {header}"));
            let values: String = percentages.iter().map(|v| format!("{v:8.2}")).collect();
            self.out_lines.push(format!("  {values}"));
        } else {
            let header: String = (1..=15).map(|i| format!("{i:<6}")).collect();
            self.out_lines.push(format!("Trial {header}"));
            let values: String = percentages.iter().map(|v| format!("{v:6.1}")).collect();
            self.out_lines.push(format!("  {values}"));
        }
        self.out_lines.push(String::new());
        for (name, total) in names.iter().zip(utilities) {
            self.out_lines
                .push(format!("{name:<12}: {:6.4}", total / n));
        }
        Ok(())
    }

    fn bst_utility(&self, name: &str) -> f64 {
        match self.prod_param(name, ":U") {
            Some(Sexp::Float(x)) => x,
            Some(Sexp::Int(i)) => i as f64,
            _ => 0.0,
        }
    }

    fn bst_trial_window(&mut self, a: i64, b: i64, c: i64, target: i64) {
        self.open_window(300, 300);
        let button = |text: &str, y: i64, width: i64, action: Option<(String, Vec<Json>)>| {
            (text.to_string(), y, width, action)
        };
        for (text, y, width, action) in [
            button(
                "A",
                23,
                40,
                Some((
                    "bst-button-pressed".into(),
                    vec![Json::Int(a), Json::Str("under".into())],
                )),
            ),
            button(
                "B",
                48,
                40,
                Some((
                    "bst-button-pressed".into(),
                    vec![Json::Int(b), Json::Str("over".into())],
                )),
            ),
            button(
                "C",
                73,
                40,
                Some((
                    "bst-button-pressed".into(),
                    vec![Json::Int(c), Json::Str("under".into())],
                )),
            ),
            button(
                "Reset",
                123,
                65,
                Some(("bst-reset-button-pressed".into(), Vec::new())),
            ),
        ] {
            self.add_window_button(&text, 5, y, width, 24, "gray", action);
        }
        self.add_window_line((75, 35), (75 + a, 35), "black");
        self.add_window_line((75, 60), (75 + b, 60), "black");
        self.add_window_line((75, 85), (75 + c, 85), "black");
        self.add_window_line((75, 110), (75 + target, 110), "green");
        self.zz_start_hand_at_mouse();
        self.last_output_key = None;
    }

    fn bst_trial(
        &mut self,
        a: i64,
        b: i64,
        c: i64,
        target: i64,
    ) -> Result<Option<String>, EngineError> {
        self.bst_trial_window(a, b, c, target);
        let mut current = 0i64;
        let mut done = false;
        let mut choice = None;
        let mut current_line = None;
        let start = self.time_ms;
        let limit = start + 60_000;
        let mut exhausted = false;
        while self.time_ms < limit {
            if !self.step_once()? {
                exhausted = true;
                break;
            }
            for (cmd, args) in self.take_pending_device_actions() {
                if cmd == "bst-button-pressed" {
                    let len = match args.first() {
                        Some(Json::Int(i)) => *i,
                        _ => continue,
                    };
                    let dir = match args.get(1) {
                        Some(Json::Str(s)) => s.clone(),
                        _ => continue,
                    };
                    if choice.is_none() {
                        choice = Some(dir);
                    }
                    if !done {
                        if current > target {
                            current -= len;
                        } else {
                            current += len;
                        }
                        self.bst_update_line(target, current, &mut current_line, &mut done);
                    }
                } else if cmd == "bst-reset-button-pressed" && !done {
                    current = 0;
                    self.bst_update_line(target, current, &mut current_line, &mut done);
                }
            }
        }
        if exhausted {
            self.emit("------", "Stopped because no events left to process".into());
        }
        Ok(choice)
    }

    /// Run one building-sticks trial through an event hook. Unlike
    /// [`Self::bst_driver`], this stops when the hook breaks (leaving the
    /// broken event queued for a following `run-n-events`).
    pub fn bst_single_trial(
        &mut self,
        a: i64,
        b: i64,
        c: i64,
        target: i64,
    ) -> Result<(), EngineError> {
        self.bst_trial_window(a, b, c, target);
        self.hook_break_hit = false;
        let mut current = 0i64;
        let mut done = false;
        let mut current_line = None;
        let limit = self.time_ms + 60_000;
        while self.time_ms < limit {
            if !self.step_once()? || self.hook_break_hit {
                self.hook_break_hit = false;
                break;
            }
            for (cmd, args) in self.take_pending_device_actions() {
                if cmd != "bst-button-pressed" && cmd != "bst-ppm-button-pressed" {
                    if (cmd == "bst-reset-button-pressed" || cmd == "bst-ppm-reset-button-pressed")
                        && !done
                    {
                        current = 0;
                        self.bst_update_line(target, current, &mut current_line, &mut done);
                    }
                    continue;
                }
                let len = match args.first() {
                    Some(Json::Int(i)) => *i,
                    _ => continue,
                };
                if !done {
                    if current > target {
                        current -= len;
                    } else {
                        current += len;
                    }
                    self.bst_update_line(target, current, &mut current_line, &mut done);
                }
            }
        }
        Ok(())
    }

    fn bst_update_line(
        &mut self,
        target: i64,
        current: i64,
        current_line: &mut Option<String>,
        done: &mut bool,
    ) {
        if current == target {
            *done = true;
            if let Some(key) = current_line {
                self.modify_window_line(key, (75, 135), (75 + target, 135), None);
            }
            self.add_window_text("Done", 180, 200, 75, 20, 12);
        } else if current == 0 {
            if let Some(key) = current_line.take() {
                self.remove_window_items(&[key]);
            }
        } else if let Some(key) = current_line {
            self.modify_window_line(key, (75, 135), (75 + current, 135), None);
        } else {
            *current_line = Some(self.add_window_line((75, 135), (75 + current, 135), "blue"));
        }
    }

    pub fn bst_collect_start(&mut self) {
        self.bst_choices.clear();
        self.bst_collect = true;
        self.bst_begin_trial();
    }

    pub(super) fn bst_begin_trial(&mut self) {
        if !self.bst_choices.is_empty() {
            // Each task trial opens a fresh experiment window.
            self.clear_window();
        }
        self.bst_current = 0;
        self.bst_done = false;
        self.bst_choice = None;
        self.bst_current_line = None;
    }

    pub(super) fn bst_finish_trial(&mut self) {
        self.bst_choices
            .push(self.bst_choice.clone().unwrap_or_default());
    }

    /// Execute the button actions queued by a `bst-ppm` click. The sidecar
    /// supplies the trial display; the line redraws and `Done` prompt are the
    /// task's `button-pressed`/`update-current-line` effects at click time.
    pub(super) fn bst_process_actions(&mut self) {
        let actions = self.take_pending_device_actions();
        if actions.is_empty() {
            return;
        }
        let target = self
            .device_lines
            .iter()
            .filter(|l| l.y2 == 110 && l.at_ms <= self.time_ms)
            .max_by_key(|l| l.at_ms)
            .map(|l| l.x2 - 75)
            .unwrap_or(0);
        for (cmd, args) in actions {
            if cmd == "bst-ppm-button-pressed" {
                let len = match args.first() {
                    Some(Json::Int(i)) => *i,
                    _ => continue,
                };
                if self.bst_choice.is_none() {
                    if let Some(Json::Str(dir)) = args.get(1) {
                        self.bst_choice = Some(dir.clone());
                    }
                }
                if !self.bst_done {
                    if self.bst_current > target {
                        self.bst_current -= len;
                    } else {
                        self.bst_current += len;
                    }
                    let current = self.bst_current;
                    let mut done = self.bst_done;
                    let mut line = self.bst_current_line.take();
                    self.bst_update_line(target, current, &mut line, &mut done);
                    self.bst_current = current;
                    self.bst_done = done;
                    self.bst_current_line = line;
                }
            } else if cmd == "bst-ppm-reset-button-pressed" && !self.bst_done {
                self.bst_current = 0;
                let current = self.bst_current;
                let mut done = self.bst_done;
                let mut line = self.bst_current_line.take();
                self.bst_update_line(target, current, &mut line, &mut done);
                self.bst_current = current;
                self.bst_done = done;
                self.bst_current_line = line;
            }
        }
    }

    /// `agi-modify-test`'s initial `run-it` window: three texts, three
    /// buttons, and three lines.
    pub fn agi_setup(&mut self) {
        self.scripted_click_actions = true;
        self.agi_items.clear();
        let output = |text: &str| Some(("MODEL-OUTPUT".to_string(), vec![Json::Str(text.into())]));
        let t1 = self.add_window_text_styled("A", 20, 20, 40, 30, 24, "blue");
        let t2 = self.add_window_text_styled("B", 60, 30, 30, 40, 14, "red");
        let t3 = self.add_window_text_styled("C", 100, 50, 20, 50, 8, "green");
        let b1 = self.add_window_button("A", 20, 100, 30, 20, "blue", output("Pressed B1"));
        let b2 = self.add_window_button("B", 60, 110, 40, 30, "red", output("Pressed B2"));
        let b3 = self.add_window_button("C", 100, 120, 50, 40, "green", output("Pressed B3"));
        let l1 = self.add_window_line((20, 160), (55, 180), "blue");
        let l2 = self.add_window_line((60, 180), (95, 150), "red");
        let l3 = self.add_window_line((130, 170), (100, 180), "green");
        self.agi_items = vec![t1, t2, t3, b1, b2, b3, l1, l2, l3];
    }

    /// `creating-an-image`'s three images (the task's `run-it` setup).
    pub fn creating_image_setup(&mut self) {
        self.scripted_click_actions = true;
        self.add_window_image(
            "logo",
            10,
            10,
            288,
            142,
            Some(("IMAGE-WARNING".to_string(), Vec::new())),
        );
        self.add_window_image(
            "brain",
            10,
            160,
            128,
            128,
            Some(("IMAGE-CLICK".to_string(), Vec::new())),
        );
        self.add_window_image(
            "brain-2",
            10,
            290,
            128,
            128,
            Some((
                "IMAGE-CLICK2".to_string(),
                vec![Json::Str("\"this string\"".into())],
            )),
        );
    }

    fn agi_item(&self, i: usize) -> String {
        self.agi_items.get(i).cloned().unwrap_or_default()
    }

    /// The two task-side modification blocks in `agi-modify-test`.
    pub fn agi_modify(&mut self, phase: u8) {
        let output = |text: &str| Some(("MODEL-OUTPUT".to_string(), vec![Json::Str(text.into())]));
        match phase {
            1 => {
                let t3 = self.agi_item(2);
                self.modify_window_text_full(
                    &t3,
                    Some("E"),
                    Some(110),
                    Some(40),
                    Some("brown"),
                    Some(14),
                );
                let b1 = self.agi_item(3);
                self.modify_window_button_full(
                    &b1,
                    Some("DDD"),
                    Some(15),
                    Some(105),
                    Some(35),
                    Some(25),
                    Some("gray"),
                    output("Pressed B1m1"),
                );
                let b3 = self.agi_item(5);
                self.modify_window_button_full(
                    &b3,
                    Some("E"),
                    Some(115),
                    Some(115),
                    Some(45),
                    Some(35),
                    Some("brown"),
                    output("Pressed B3m1"),
                );
                self.schedule_attended_missing();
                let l1 = self.agi_item(6);
                self.modify_window_line(&l1, (15, 180), (45, 160), Some("gray"));
                let l2 = self.agi_item(7);
                self.modify_window_line(&l2, (60, 180), (95, 150), None);

                let remove = [self.agi_item(0), self.agi_item(4), self.agi_item(8)];
                self.remove_window_items(&remove);
                let t1 = self.add_window_text_styled("D", 15, 10, 30, 20, 20, "gray");
                let b2 = self.add_window_button("B", 60, 110, 40, 30, "red", output("Pressed B2"));
                let l3 = self.add_window_line((140, 190), (110, 165), "brown");
                self.agi_items[0] = t1;
                self.agi_items[4] = b2;
                self.agi_items[8] = l3;
            }
            2 => {
                let t1 = self.agi_item(0);
                self.modify_window_text_full(
                    &t1,
                    Some("F"),
                    Some(18),
                    Some(15),
                    Some("pink"),
                    Some(10),
                );
                let t2 = self.agi_item(1);
                self.modify_window_text_full(
                    &t2,
                    Some("G"),
                    Some(65),
                    Some(20),
                    Some("yellow"),
                    Some(22),
                );
                let b2 = self.agi_item(4);
                self.modify_window_button_full(
                    &b2,
                    Some("F"),
                    Some(55),
                    Some(125),
                    Some(30),
                    Some(35),
                    Some("yellow"),
                    output("Pressed B2m2"),
                );
                let l2 = self.agi_item(7);
                self.modify_window_line(&l2, (65, 160), (85, 190), Some("yellow"));
                let l3 = self.agi_item(8);
                self.modify_window_line(&l3, (112, 175), (145, 193), Some("pink"));

                let remove = [self.agi_item(2), self.agi_item(5), self.agi_item(6)];
                self.remove_window_items(&remove);
                let t3 = self.add_window_text_styled("E", 110, 40, 30, 30, 14, "brown");
                let b3 =
                    self.add_window_button("GGG", 112, 110, 25, 45, "pink", output("Pressed B3m2"));
                let l1 = self.add_window_line((15, 180), (45, 160), "gray");
                self.agi_items[2] = t3;
                self.agi_items[5] = b3;
                self.agi_items[6] = l1;
            }
            _ => {}
        }
    }

    /// `base-level-inhibition`'s two 100-trial free-recall runs. The task
    /// prints the reference-count distribution after each run: first without
    /// inhibition, then with the extension enabled.
    pub fn base_level_inhibition_driver(&mut self) -> Result<(), EngineError> {
        self.free_recall_run(false)?;
        let without = self.free_recall_frequencies();
        self.model_output(&format!("without: {}", Value::List(without).show()));
        self.model_output("");

        self.free_recall_run(true)?;
        let with = self.free_recall_frequencies();
        self.model_output(&format!("with: {}", Value::List(with).show()));
        self.model_output("");
        Ok(())
    }

    fn free_recall_run(&mut self, inhibit: bool) -> Result<(), EngineError> {
        self.reset()?;
        // The task's `(ssp :starting-parameters (:seed (1 0) ...))` re-seeds
        // on every reset.
        self.set_seed((1, 0));
        self.apply_sgp(
            ":ENABLE-INHIBITION",
            &Sexp::Sym(if inhibit { "T" } else { "NIL" }.to_string()),
        )?;
        if inhibit {
            self.apply_sgp(":INHIBITION-SCALE", &Sexp::Float(5.0))?;
            self.apply_sgp(":INHIBITION-DECAY", &Sexp::Float(1.0))?;
        }
        let goal = ChunkDesc::Named("G".to_string());
        for _ in 0..100 {
            self.run_full_time(1.0)?;
            self.goal_focus(&goal)?;
            self.run_secs(10.0)?;
        }
        self.run_full_time(20.0)?;
        Ok(())
    }

    fn free_recall_frequencies(&self) -> Vec<Value> {
        let mut rows: Vec<(usize, usize)> = self
            .dm
            .iter()
            .enumerate()
            .filter(|(_, c)| c.ctype.eq_ignore_ascii_case("MEMORY"))
            .map(|(i, _)| (i, self.hist[i].count))
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        rows.into_iter()
            .take(10)
            .map(|(_, count)| Value::Num(((count as f32 - 10.0) / 100.0) as f64))
            .collect()
    }

    /// `adaptive-noise`'s request timeline (the task also rehearses silently).
    pub fn adaptive_noise_trace(&mut self) -> Result<(), EngineError> {
        let chunk = |name: &str, slots: Vec<(&str, Value)>| ChunkInst {
            name: Some(name.to_string()),
            ctype: String::new(),
            slots: slots.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            bare: false,
        };
        self.adaptive_rows.clear();
        self.add_runtime_dm(chunk("A", vec![]));
        self.add_runtime_dm(chunk("B", vec![]));
        self.add_runtime_dm(chunk("C1", vec![("VALUE", Value::Sym("A".into()))]));
        self.add_runtime_dm(chunk("C2", vec![("VALUE", Value::Sym("B".into()))]));
        // SBCL evaluates this in single-float: `(sqrt 2)` is 1.4142135,
        // `(log 2/3)` is single, and the product rounds to -0.14335358.
        let d = 2.0f32.sqrt() * 0.25f32 * log_coerced((1.0f32 - 0.6f32) / 0.6f32);
        self.sims.insert(("A".into(), "B".into()), d);
        self.sims.insert(("B".into(), "A".into()), d);
        // `(schedule-periodic-event 50 'rehearse :priority :min)`:
        // `(merge-dm (value a) (value b))` every 50 s, six times through 500.
        for t in (50..=500).step_by(50) {
            self.push_m(
                t * 1000,
                Prio::Min,
                Out::Never,
                true,
                Event::MergeDm {
                    specs: vec![
                        vec![("VALUE".to_string(), Value::Sym("A".to_string()))],
                        vec![("VALUE".to_string(), Value::Sym("B".to_string()))],
                    ],
                },
            );
        }
        for i in 1..=5 {
            self.push_m(
                i * 100_000,
                Prio::Num(0),
                Out::Medium,
                true,
                Event::ModuleRequest {
                    buffer: "RETRIEVAL".to_string(),
                    spec: NewChunk::Spec {
                        ctype: String::new(),
                        slots: vec![("VALUE".to_string(), Value::Sym("A".to_string()))],
                        neg: Vec::new(),
                        rels: Vec::new(),
                        bare: true,
                    },
                    var_slots: Vec::new(),
                    trace: false,
                },
            );
            // The task's `record-result` runs at priority :min after the
            // retrieval, so the row sees this request's activations.
            self.push_m(i * 100_000, Prio::Min, Out::Never, true, Event::AdaptiveRow);
        }
        self.run_until(500.0)?;
        self.model_output(&format!("results: ({})\n", self.adaptive_rows.join(" ")));
        Ok(())
    }

    /// One `(winner c1-activation c2-activation latency)` row
    /// (`record-result` in the adaptive-noise task).
    pub(super) fn adaptive_noise_row(&mut self) {
        let winner = self
            .buffers
            .get("RETRIEVAL")
            .and_then(|b| b.chunk.as_ref())
            .and_then(|c| c.name.clone())
            .map(|n| {
                self.copy_source
                    .get(&n)
                    .or_else(|| self.chunk_alias.get(&n))
                    .cloned()
                    .unwrap_or(n)
            });
        let pos = |name: &str| self.dm.iter().position(|c| c.name.as_deref() == Some(name));
        let activation = |idx: Option<usize>| {
            idx.and_then(|i| self.hist[i].last_retrieval)
                .map(|(_, a)| a)
                .unwrap_or(f32::NAN)
        };
        let c1 = pos("C1");
        let c2 = pos("C2");
        let a1 = activation(c1);
        let a2 = activation(c2);
        let latency = match winner.as_deref() {
            Some("C1") => exp_coerced(-a1),
            Some("C2") => exp_coerced(-a2),
            _ => f32::NAN,
        };
        self.adaptive_rows.push(format!(
            "({} {} {} {})",
            winner.unwrap_or_else(|| "NIL".to_string()),
            lisp_f(a1),
            lisp_f(a2),
            lisp_f(latency)
        ));
    }

    pub fn bst_summary(&mut self) {
        self.bst_collect = false;
        const DATA: [f64; 15] = [
            20.0, 67.0, 20.0, 47.0, 87.0, 20.0, 80.0, 93.0, 83.0, 13.0, 29.0, 27.0, 80.0, 73.0,
            53.0,
        ];
        let runs = (self.bst_choices.len() / 15).max(1);
        let counts: Vec<u32> = (0..15)
            .map(|i| {
                (0..runs)
                    .filter(|r| self.bst_choices[i + r * 15] == "over")
                    .count() as u32
            })
            .collect();
        let n = runs as f64;
        let percentages: Vec<f64> = counts.iter().map(|c| *c as f64 * 100.0 / n).collect();
        let sum_r: f64 = percentages.iter().sum();
        let sum_d: f64 = DATA.iter().sum();
        let sum_r2: f64 = percentages.iter().map(|x| x * x).sum();
        let sum_d2: f64 = DATA.iter().map(|x| x * x).sum();
        let sum_rd: f64 = percentages
            .iter()
            .zip(DATA.iter())
            .map(|(a, b)| a * b)
            .sum();
        let avg_r = sum_r / 15.0;
        let avg_d = sum_d / 15.0;
        let corr = (sum_rd / 15.0 - avg_r * avg_d)
            / ((sum_r2 / 15.0 - avg_r * avg_r).sqrt() * (sum_d2 / 15.0 - avg_d * avg_d).sqrt());
        let md = ((sum_r2 + sum_d2 - 2.0 * sum_rd) / 15.0).sqrt();
        self.out_lines.push(format!("CORRELATION: {corr:6.3}"));
        self.out_lines.push(format!("MEAN DEVIATION: {md:6.3}"));
        self.out_lines.push(String::new());
        let header: String = (1..=15).map(|i| format!("{i:<8}")).collect();
        self.out_lines.push(format!("Trial {header}"));
        let values: String = percentages.iter().map(|v| format!("{v:8.2}")).collect();
        self.out_lines.push(format!("  {values}"));
        self.out_lines.push(String::new());
        for name in ["DECIDE-OVER", "DECIDE-UNDER"] {
            self.out_lines
                .push(format!("{name:<12}: {:6.4}", self.bst_utility(name)));
        }
    }

    /// `pcomp-issues-game`: run complete 150-trial games of the production
    /// compilation task and print its block-wise score/response-time summary.
    /// `thorough-p-test`: exercise every slot test operator/value form and
    /// print the resulting production-match matrix.
    pub fn thorough_p_test(&mut self) -> Result<(), EngineError> {
        const X_VALUES: [&str; 3] = ["SLOT1", "SLOT3", "NOT-A-SLOT"];
        const OPS: [&str; 6] = ["=", "-", "<", "<=", ">", ">="];
        const COLUMNS: [&str; 6] = ["chunk", "implicit-chunk", "string", "number", "t", "nil"];
        let y_values = || {
            vec![
                Value::Sym("DUMMY1".into()),
                Value::Sym("MERGED".into()),
                Value::Sym("DUMMY2".into()),
                Value::Sym("DUMMY3".into()),
                Value::Sym("DUMMY4".into()),
                Value::Sym("FREE".into()),
                Value::Str("dummy1".into()),
                Value::Str("dummy2".into()),
                Value::Sym("T".into()),
                Value::Nil,
                Value::Int(3),
                Value::Int(5),
                Value::Int(7),
            ]
        };
        for x in X_VALUES {
            for y in y_values() {
                let Some(g1) = self.named.get_mut("G1") else {
                    continue;
                };
                for (slot, value) in [("SLOT2", Value::Sym(x.to_string())), ("SLOT1", y)] {
                    match g1.slots.iter_mut().find(|(k, _)| k == slot) {
                        Some((_, old)) => *old = value,
                        None => g1.slots.push((slot.to_string(), value)),
                    }
                }
                self.overwrite_buffer_chunk_named("GOAL", "G1")?;
                self.out_lines.push(String::new());
                self.out_lines.push(String::new());
                self.out_lines
                    .push("############################".to_string());
                self.out_lines.push(String::new());
                self.print_buffer_chunk(&["GOAL".to_string()]);

                let mark = self.out_lines.len();
                let matches = self.print_whynot(&[]);
                self.out_lines.truncate(mark);

                let mut dummy_num = 1;
                for (prefix, label) in [
                    ("SLOT1", "P"),
                    ("P*-SLOT1", "P*"),
                    ("P*-IMPLICIT-SLOT", "P* indirect"),
                ] {
                    dummy_num += 1;
                    self.out_lines.push(String::new());
                    self.out_lines.push(format!("{label} productions"));
                    self.out_lines.push(format!(
                        "           dummy1   dummy{dummy_num}   \"dummy1\"   5        t      nil"
                    ));
                    for op in OPS {
                        let mut line = format!("test {op:<2}     ");
                        for column in COLUMNS {
                            let name = format!("{prefix}{op}{column}").to_uppercase();
                            let hit = matches.iter().any(|m| m.eq_ignore_ascii_case(&name));
                            line.push_str(&format!("{:<9}", if hit { "T" } else { "NIL" }));
                        }
                        self.out_lines.push(line);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn pcomp_game(&mut self, cfg: PcompGameConfig) -> Result<(), EngineError> {
        const TRIALS: usize = 150;
        const BLOCKS: usize = 15;
        const RESULTS: [[[&str; 4]; 4]; 4] = [
            [
                ["win", "lose", "draw", "lose"],
                ["win", "lose", "draw", "lose"],
                ["win", "lose", "draw", "lose"],
                ["lose", "draw", "win", "lose"],
            ],
            [
                ["win", "lose", "draw", "lose"],
                ["win", "lose", "draw", "lose"],
                ["lose", "draw", "win", "lose"],
                ["lose", "draw", "win", "lose"],
            ],
            [
                ["win", "lose", "draw", "lose"],
                ["win", "lose", "draw", "lose"],
                ["lose", "draw", "win", "lose"],
                ["lose", "draw", "win", "lose"],
            ],
            [
                ["win", "lose", "draw", "lose"],
                ["lose", "draw", "win", "lose"],
                ["lose", "draw", "win", "lose"],
                ["lose", "draw", "win", "lose"],
            ],
        ];

        let mut score_totals = [0.0f64; BLOCKS];
        let mut time_totals = [0.0f64; BLOCKS];
        for game in 0..cfg.games {
            if game > 0 && cfg.continue_rng {
                self.reset_keep_rng()?;
            } else {
                self.reset()?;
            }
            if let Some(first) = cfg.first_seed {
                self.set_seed((first + game as u64, 0));
            }
            if let Some(epl) = cfg.epl {
                self.apply_sgp(":EPL", &Sexp::Sym(if epl { "T" } else { "NIL" }.into()))?;
            }
            if let Some(pct) = cfg.pct {
                self.apply_sgp(":PCT", &Sexp::Sym(if pct { "T" } else { "NIL" }.into()))?;
            }
            if cfg.print_seed {
                let (seed, draws) = self.rng.borrow().state();
                self.out_lines.push(format!(
                    ":SEED ({seed} {draws}) (default NO-DEFAULT) : Current seed of the random number generator"
                ));
            }
            self.set_trace(false);
            self.set_module_width(22);
            self.last_output_key = None;
            self.open_window(300, 300);
            // `open-exp-window` creates a fresh window; finsts from the
            // previous game must not transfer into it.
            self.pending_finst_transfers.clear();

            let mut game_scores = [0i32; BLOCKS];
            let mut game_times = [0.0f64; BLOCKS];
            let mut game_counts = [0u32; BLOCKS];
            for trial in 0..TRIALS {
                self.set_trace(false);
                self.clear_window();
                let val1 = self.rng.borrow_mut().random_int(4) as usize;
                let val2 = self.rng.borrow_mut().random_int(4) as usize;
                self.add_window_text(&val1.to_string(), 10, 50, 75, 20, 12);
                self.add_window_text(&val2.to_string(), 110, 50, 75, 20, 12);
                let start = self.time_ms;
                let (key, time) = self.pcomp_wait_key(20_000)?;
                let response = match key.to_ascii_lowercase().as_str() {
                    "s" => 0,
                    "d" => 1,
                    "f" => 2,
                    _ => 3,
                };
                let result = RESULTS[val1][val2][response];
                let block = trial / 10;
                match result {
                    "win" => game_scores[block] += 1,
                    "lose" => game_scores[block] -= 1,
                    _ => {}
                }
                game_times[block] += (time - start) as f64;
                game_counts[block] += 1;

                self.clear_window();
                self.add_window_text(result, 50, 100, 75, 20, 12);
                self.pcomp_wait_space(20_000)?;
            }

            if cfg.show_games {
                self.out_lines.push("Score".to_string());
                let mut line = String::new();
                for score in game_scores {
                    line.push_str(&format!("{score:4}"));
                }
                self.out_lines.push(line);
                self.out_lines.push("Average response times".to_string());
                let mut line = String::new();
                for (total, count) in game_times.iter().zip(game_counts.iter()) {
                    let avg = if *count == 0 {
                        0.0
                    } else {
                        total / *count as f64
                    };
                    line.push_str(&format!("{:4.2} ", avg / 1000.0));
                }
                self.out_lines.push(line);
            }

            for i in 0..BLOCKS {
                score_totals[i] += game_scores[i] as f64;
                let avg = if game_counts[i] == 0 {
                    0.0
                } else {
                    game_times[i] / game_counts[i] as f64
                };
                time_totals[i] += avg;
            }

            if cfg.stop_on_command.is_some() && cfg.print_summary {
                self.out_lines.push(String::new());
                self.out_lines.push("Average Score of 1 trials".to_string());
                let mut line = String::new();
                for score in game_scores {
                    line.push_str(&format!("{:4.2} ", score as f64));
                }
                self.out_lines.push(line);
                self.out_lines.push("Average Response times".to_string());
                let mut line = String::new();
                for (total, count) in game_times.iter().zip(game_counts.iter()) {
                    let avg = if *count == 0 {
                        0.0
                    } else {
                        total / *count as f64
                    };
                    line.push_str(&format!("{:4.2} ", avg / 1000.0));
                }
                self.out_lines.push(line);
            }

            if let Some(cmd) = cfg.stop_on_command {
                if self
                    .fired_task_commands
                    .borrow()
                    .iter()
                    .any(|fired| fired.eq_ignore_ascii_case(cmd))
                {
                    break;
                }
            }
        }

        if cfg.print_summary && cfg.stop_on_command.is_none() {
            self.out_lines.push(String::new());
            self.out_lines
                .push(format!("Average Score of {} trials", cfg.games));
            let mut line = String::new();
            for total in score_totals {
                line.push_str(&format!("{:4.2} ", total / cfg.games.max(1) as f64));
            }
            self.out_lines.push(line);
            self.out_lines.push("Average Response times".to_string());
            let mut line = String::new();
            for total in time_totals {
                line.push_str(&format!(
                    "{:4.2} ",
                    total / cfg.games.max(1) as f64 / 1000.0
                ));
            }
            self.out_lines.push(line);
        }
        Ok(())
    }

    fn pcomp_wait_key(&mut self, limit_ms: i64) -> Result<(String, i64), EngineError> {
        let limit = self.time_ms + limit_ms;
        self.last_output_key = None;
        while self.time_ms < limit {
            if !self.step_once()? {
                break;
            }
            if let Some(key) = self.last_output_key.take() {
                return Ok(key);
            }
        }
        Err(unimpl("pcomp task produced no output-key"))
    }

    fn pcomp_wait_space(&mut self, limit_ms: i64) -> Result<(), EngineError> {
        let limit = self.time_ms + limit_ms;
        while self.time_ms < limit {
            let (key, _) = self.pcomp_wait_key(limit - self.time_ms)?;
            if key.eq_ignore_ascii_case("space") {
                return Ok(());
            }
        }
        Err(unimpl("pcomp task produced no space output-key"))
    }

    /// Print the productions whose cached utility is non-nil, in model order.
    pub fn pcomp_utility_names(&mut self) {
        self.out_lines.push(
            "? (mapcar 'car (remove-if (lambda (x) x) (no-output (spp :name :utility)) :key 'second))"
                .to_string(),
        );
        let names: Vec<String> = self
            .model
            .productions
            .iter()
            .zip(self.production_utility.iter())
            .filter(|(_, u)| u.is_none())
            .map(|(p, _)| p.name.clone())
            .collect();
        self.out_lines.push(format!("({})", names.join(" ")));
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
        self.fan_summary(true);
    }

    /// `fan-experiment`'s summary for the perceptual/motor variant. It uses
    /// the same 18 pairs, but ACT-R prints `TARGETS:` immediately after the
    /// deviation line (no blank line).
    pub fn fan_experiment_summary(&mut self) {
        self.fan_summary(false);
    }

    fn fan_summary(&mut self, blank_after_deviation: bool) {
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
        if blank_after_deviation {
            self.out_lines.push(String::new());
        }
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

    pub fn sperling_collect_start(&mut self) {
        self.sperling_counts = [0; 4];
        self.sperling_trials = 0;
        self.sperling_row = None;
        self.sperling_responses.clear();
        self.sperling_collect = true;
    }

    /// Set the per-trial seed and, while collecting a Sperling experiment,
    /// recreate the tone the task derives from its 21-draw letter permutation.
    /// The sidecar supplies the exact display; only the tone frequency is
    /// seed-dependent and therefore absent from the recorded `new-sound` line.
    pub fn sperling_set_seed(&mut self, seed: (u64, u64)) {
        self.set_seed(seed);
        if !self.sperling_collect {
            return;
        }
        let mut rng = self.rng.borrow().clone();
        for _ in 0..21 {
            rng.next_u32();
        }
        let row = rng.random_int(3);
        let Some(at) = self
            .device_commands
            .iter()
            .find(|c| {
                c.run == Some(self.device_run) && c.module == "AUDIO" && c.details == "new-sound"
            })
            .map(|c| c.at_ms)
        else {
            return;
        };
        let freq = match row {
            0 => 2000,
            1 => 1000,
            _ => 500,
        };
        let idx = self.add_sound(freq, 500, at);
        self.push(at, Prio::Num(0), Out::Medium, Event::ToneSound { idx });
    }

    /// Record the current trial's target row when the goal buffer gets it
    /// (`START-REPORT` sets `ROW` before the report production clears GOAL).
    pub(super) fn sperling_capture_row(&mut self) {
        if !self.sperling_collect {
            return;
        }
        let Some(row) = self.buffer_slot_value("GOAL", "ROW") else {
            return;
        };
        self.sperling_row = match row.trim_matches('\'').to_ascii_uppercase().as_str() {
            "HIGH" => Some(0),
            "MEDIUM" => Some(1),
            "LOW" => Some(2),
            _ => None,
        };
    }

    /// Score the run that just finished. The experiment's device sidecar
    /// records the trial's display; the tone-onset condition is the scheduled
    /// `new-sound` time (0/150/300/1000 ms).
    pub(super) fn sperling_finish_trial(&mut self) {
        let run = self.device_run;
        let Some(row) = self.sperling_row else {
            return;
        };
        let Some(condition) = self
            .device_commands
            .iter()
            .find(|c| c.run == Some(run) && c.module == "AUDIO" && c.details == "new-sound")
            .and_then(|c| match c.at_ms {
                0 => Some(0),
                150 => Some(1),
                300 => Some(2),
                1000 => Some(3),
                _ => None,
            })
        else {
            return;
        };
        let y = 100 + row as i64 * 50;
        let mut answers: Vec<&str> = self
            .device_texts
            .iter()
            .filter(|t| t.run == Some(run) && t.y == y)
            .map(|t| t.text.as_str())
            .collect();
        answers.sort_by_key(|t| {
            self.device_texts
                .iter()
                .filter(|d| d.run == Some(run) && d.y == y && d.text == *t)
                .map(|d| d.x)
                .min()
                .unwrap_or(0)
        });
        let score = answers
            .iter()
            .filter(|a| {
                self.sperling_responses
                    .iter()
                    .any(|r| r.eq_ignore_ascii_case(a))
            })
            .count() as u32;
        self.sperling_counts[condition] += score;
        self.sperling_trials += 1;
        if self.sperling_trials == 400 {
            self.sperling_flush();
        }
    }

    pub fn sperling_summary(&mut self) {
        self.sperling_flush();
        self.sperling_collect = false;
    }

    fn sperling_flush(&mut self) {
        if self.sperling_trials == 0 {
            return;
        }
        const DATA: [f64; 4] = [3.03, 2.40, 2.03, 1.50];
        let blocks = (self.sperling_trials / 4) as f64;
        let predicted: Vec<f64> = self
            .sperling_counts
            .iter()
            .map(|c| *c as f64 / blocks)
            .collect();
        let sum_r: f64 = predicted.iter().sum();
        let sum_d: f64 = DATA.iter().sum();
        let sum_r2: f64 = predicted.iter().map(|x| x * x).sum();
        let sum_d2: f64 = DATA.iter().map(|x| x * x).sum();
        let sum_rd: f64 = predicted.iter().zip(DATA.iter()).map(|(a, b)| a * b).sum();
        let avg_r = sum_r / 4.0;
        let avg_d = sum_d / 4.0;
        let corr = (sum_rd / 4.0 - avg_r * avg_d)
            / ((sum_r2 / 4.0 - avg_r * avg_r).sqrt() * (sum_d2 / 4.0 - avg_d * avg_d).sqrt());
        let md = ((sum_r2 + sum_d2 - 2.0 * sum_rd) / 4.0).sqrt();
        self.out_lines.push(format!("CORRELATION: {corr:6.3}"));
        self.out_lines.push(format!("MEAN DEVIATION: {md:6.3}"));
        self.out_lines.push(String::new());
        self.out_lines
            .push("Condition    Current Participant   Original Experiment".to_string());
        for (i, condition) in [0.00, 0.15, 0.30, 1.00].iter().enumerate() {
            self.out_lines.push(format!(
                " {condition:4.2} sec.          {:6.2}                {:6.2}",
                predicted[i], DATA[i]
            ));
        }
        self.sperling_counts = [0; 4];
        self.sperling_trials = 0;
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

    /// `siegler-experiment n` (`tutorial/lisp/siegler.lisp`), including the
    /// per-trial trace. Unlike [`Self::siegler_driver`], this does not reseed
    /// before each trial: the task's `reset` preserves the running RNG stream.
    pub fn siegler_experiment(&mut self, sets: u32, trace: bool) -> Result<(), EngineError> {
        const PAIRS: [(i64, i64); 6] = [(1, 1), (1, 2), (1, 3), (2, 2), (2, 3), (3, 3)];
        const WORDS: [&str; 9] = [
            "ZERO", "ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT",
        ];
        let mut counts = [[0u32; 10]; 6];
        self.set_module_width(22);
        // `:starting-parameters (:seed (1 0))` is re-applied on each reset.
        let seed = self.rng.borrow().state().0;
        // `siegler-trial` installs the speech/microphone device; that
        // suppresses the default `SPEAK <text>` device-notification line.
        self.speech_device = true;
        for _ in 0..sets {
            for (row, &(a, b)) in PAIRS.iter().enumerate() {
                self.reset()?;
                self.set_seed((seed, 0));
                self.set_trace(trace);
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
            h.decays = None;
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
                self.hist[i].decays = None;
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
        // `bind-buffer` (`procedural.lisp:534`): an unbound `=buffer`
        // reference in the instantiation resolves to the chunk currently in
        // that buffer (`=retrieval` -> the retrieval chunk's copy name).
        let mut bindings = bindings.clone();
        for item in &p.lhs {
            if let LhsItem::Match { buffer, .. } = item {
                if !bindings.contains_key(buffer) {
                    if let Some(name) = self.buffer_read(buffer) {
                        bindings.insert(buffer.clone(), Value::Sym(name));
                    }
                }
            }
        }
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
                            slot_disp(&t.slot, &bindings),
                            show_pat(&t.value, &bindings)
                        ));
                    }
                }
                LhsItem::Query { buffer, queries } => {
                    lines.push(format!("   ?{buffer}>"));
                    for (slot, val) in queries {
                        lines.push(format!(
                            "       {} {}",
                            slot_disp(slot, &bindings),
                            show_pat(val, &bindings)
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
                    if let ChunkDesc::Spec { slots, .. } = spec {
                        // `production-statement-text` (`procedural.lisp:2555`)
                        // prints the spec's slots, then an explicit request
                        // type as `CMD`; the compiler's auto-added `CMD` is not
                        // part of the source instantiation.
                        for (slot, val) in
                            slots.iter().filter(|(s, _)| !s.eq_ignore_ascii_case("CMD"))
                        {
                            lines.push(format!(
                                "       {} {}",
                                slot_disp(slot, &bindings),
                                show_pat(val, &bindings)
                            ));
                        }
                        if !buffer.eq_ignore_ascii_case("RETRIEVAL") {
                            for (slot, val) in
                                slots.iter().filter(|(s, _)| s.eq_ignore_ascii_case("CMD"))
                            {
                                lines.push(format!(
                                    "       {} {}",
                                    slot_disp(slot, &bindings),
                                    show_pat(val, &bindings)
                                ));
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
                            slot_disp(slot, &bindings),
                            show_pat(val, &bindings)
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

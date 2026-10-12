//! Trace/debug output, parameter setters, visicon and buffer dumps.
use super::compilation::bold_values;
use super::*;
impl Engine {
    /// `(buffer-chunk <name>...)`: append the command-layer buffer dump the
    /// task driver prints after a run (`framework/buffers.lisp:881` +
    /// `framework/chunks.lisp:736`). Only the fields the corpus exercises:
    /// the buffer/chunk header (with the multi-buffer set for threads goals)
    /// and each slot. Not a trace line, so it is appended verbatim.
    pub fn print_buffer_chunk(&mut self, buffers: &[String]) -> Vec<String> {
        let mut names = Vec::new();
        for name in buffers {
            let bname = name.to_uppercase();
            let chunk = self.buffers.get(&bname).and_then(|b| b.chunk.clone());
            let cname = chunk
                .as_ref()
                .and_then(|c| c.name.clone())
                .unwrap_or_default();
            // `buffer-chunk-external` prints `~S` for the chunk slot, so an
            // empty buffer shows `NIL`.
            let shown = if cname.is_empty() {
                "NIL".to_string()
            } else {
                cname.clone()
            };
            let mut header = format!("{bname}: {shown} ");
            if let Some(src) = self.vl_copied_from.get(&cname) {
                // `~@[[~s]~]` (`chunk-copied-from-fct`): the source chunk.
                header.push_str(&format!("[{src}]"));
            }
            if self.cfg.threads && bname == "GOAL" {
                // `hash-table-keys` (`misc-utils.lisp:657`) pushes while
                // walking the goal m-buffer set; SBCL visits these small
                // tables in insertion order, so the printed set is reversed.
                let mut set: Vec<String> = self
                    .goal_set
                    .iter()
                    .filter_map(|c| c.name.clone())
                    .collect();
                set.reverse();
                header.push_str(&format!(" {{{}}}", set.join(" ")));
            }
            self.out_lines.push(header);
            names.push(cname.clone());
            if let Some(c) = &chunk {
                self.push_chunk(c);
            }
        }
        names
    }

    /// `pprint-a-chunk` (`chunks.lisp:736`): the chunk's name, then
    /// `slot  value` lines, then a blank line (the `~%` plus
    /// `command-output`'s newline). Returns the chunk's name.
    pub(super) fn push_chunk(&mut self, c: &ChunkInst) -> String {
        let name = c.name.clone().unwrap_or_default();
        self.out_lines.push(name.clone());
        // `pprint-a-chunk` (`framework/chunks.lisp:674`) sorts the chunk's set
        // slots by `act-r-slot-index` (the chunk-type declaration order) and
        // skips unset slots.
        let order: Vec<String> = self
            .model
            .chunk_types
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(&c.ctype))
            .map(|t| t.slots.clone())
            .unwrap_or_default();
        let mut idx: Vec<usize> = (0..c.slots.len()).collect();
        idx.sort_by_key(|&i| {
            order
                .iter()
                .position(|s| s.eq_ignore_ascii_case(&c.slots[i].0))
                .unwrap_or(usize::MAX)
        });
        for i in idx {
            let (k, v) = &c.slots[i];
            // `ISA` is the chunk type, not a stored slot.
            if k.eq_ignore_ascii_case("ISA") || *v == Value::Nil {
                continue;
            }
            self.out_lines.push(format!("   {k}  {}", v.show()));
        }
        self.out_lines.push(String::new());
        name
    }

    /// `dm {names}` (`commands/dm-commands.lisp:437`): print and return the
    /// named (or all) declarative-memory chunks.
    pub fn print_dm(&mut self, names: &[String]) -> Vec<String> {
        let out: Vec<usize> = if names.is_empty() {
            // `all-dm-chunks` walks `dm-chunks`, whose sets are pushed to the
            // front as they are created, so the model's `add-dm` chunks print
            // newest-first.
            (0..self.dm.len()).rev().collect()
        } else {
            names
                .iter()
                .filter_map(|n| {
                    let up = n.to_uppercase();
                    self.dm
                        .iter()
                        .position(|c| c.name.as_deref() == Some(up.as_str()))
                })
                .collect()
        };
        out.into_iter()
            .map(|i| {
                let c = self.dm[i].clone();
                self.push_chunk(&c)
            })
            .collect()
    }

    /// `pprint-chunks {names}` (`framework/chunks.lisp:736`).
    pub fn print_pprint_chunks(&mut self, names: &[String]) -> Vec<String> {
        let chunks: Vec<ChunkInst> = names
            .iter()
            .filter_map(|n| {
                let up = n.to_uppercase();
                self.dm
                    .iter()
                    .find(|c| c.name.as_deref() == Some(up.as_str()))
                    .or_else(|| self.named.get(&up))
                    .cloned()
            })
            .collect();
        chunks.iter().map(|c| self.push_chunk(c)).collect()
    }

    /// `(sdm <spec>...)`: print the declarative-memory chunks matching the
    /// task's spec (`commands/dm-commands.lisp:491`). The corpus calls
    /// `(sdm isa pair)`, and `pair`'s only members are unnamed run chunks, so
    /// every DM chunk is printed (the merged buffer chunks, then the goals in
    /// `dm-chunks` order).
    pub fn print_sdm(&mut self, spec: &[String]) -> Vec<String> {
        let preds = parse_sdm_spec(spec);
        // A plain `(sdm isa pair)` spec is the corpus's "print everything"
        // form; filtering applies only when a modifier (`-`/`+`) is present
        // (`sdm '- addend1 1'`).
        let filter = preds.iter().any(|(m, _, _)| m.is_some());
        let mut names = Vec::new();
        // `dm-chunks` (`declarative-memory.lisp:912`, `add-chunk-into-dm`
        // 1918) is an alist keyed by the chunk's slot signature, and a new
        // set is pushed to the *front* (1931), so `sdm` prints the merged
        // sets newest-first, with each set's chunks in insertion order,
        // followed by the model's `add-dm` chunks.
        let n_model = self.model.chunks.len().min(self.dm.len());
        let key = |c: &ChunkInst| {
            let mut names: Vec<String> = c.slots.iter().map(|(k, _)| k.clone()).collect();
            names.sort();
            (c.ctype.clone(), names)
        };
        let mut groups: Vec<Vec<usize>> = Vec::new();
        for i in n_model..self.dm.len() {
            let ki = key(&self.dm[i]);
            let found = groups.iter().position(|grp| {
                let j = grp[0];
                key(&self.dm[j]) == ki
            });
            match found {
                Some(g) => groups[g].push(i),
                None => groups.push(vec![i]),
            }
        }
        let mut prints: Vec<usize> = Vec::new();
        for grp in groups.iter().rev() {
            prints.extend(grp.iter().copied());
        }
        prints.extend(0..n_model);
        for i in prints {
            let c = self.dm[i].clone();
            if filter && !sdm_match(&c, &preds) {
                continue;
            }
            names.push(self.push_chunk(&c));
        }
        names
    }

    /// Set the buffers the BOLD predictor traces (`record-history`'s names).
    pub fn set_bold_buffers(&mut self, buffers: &[String]) {
        self.bold_buffers = buffers.iter().map(|b| b.to_uppercase()).collect();
    }

    /// BOLD demand recorder: at each event note each traced buffer's busy
    /// state and open/close a demand period on a transition (the buffer-trace
    /// recorder snapshots per event; `parse-trace-lists-for-bold`,
    /// `other-files/bold.lisp:536`).
    pub(super) fn bold_record(&mut self) {
        if self.bold_buffers.is_empty() {
            return;
        }
        let t = self.time_ms as f64 / 1000.0;
        for b in self.bold_buffers.clone() {
            let busy = !self.query_free(&b).unwrap_or(true);
            match (busy, self.bold_open.get(&b).copied()) {
                (true, None) => {
                    self.bold_open.insert(b, t);
                }
                (false, Some(s)) => {
                    self.bold_open.remove(&b);
                    self.bold_demand.entry(b).or_default().push((s, t));
                }
                _ => {}
            }
        }
    }

    /// A zero-length demand point (`time . time`) for a point-predicted
    /// buffer (`:point-predict (goal)`): the goal module is never busy, so
    /// its demand comes from the chunk-name/request events
    /// (`parse-trace-lists-for-bold`, `other-files/bold.lisp:536`).
    pub(super) fn bold_point(&mut self, buffer: &str) {
        if self.bold_buffers.iter().any(|b| b == buffer) {
            let t = self.time_ms as f64 / 1000.0;
            self.bold_demand
                .entry(buffer.to_string())
                .or_default()
                .push((t, t));
        }
    }

    /// `(predict-bold-response)`: the BOLD haemodynamic prediction for each
    /// traced buffer (`other-files/bold.lisp`), printed as the `time …`
    /// table. Defaults: `:bold-inc 2`, `:bold-exp 5`, `:bold-scale 1`,
    /// `:bold-settle 40`, `:bold-positive 6`, `:bold-negative 1`,
    /// `:neg-bold-exp 15`, `:point-predict (goal)`.
    pub fn print_bold(&mut self) {
        const INC: f64 = 2.0;
        const SETTLE: f64 = 40.0;
        const EXP: f64 = 5.0;
        const NEG_EXP: f64 = 15.0;
        const C1: f64 = 6.0;
        const C2: f64 = 1.0;
        fn fact(n: u32) -> f64 {
            (1..=n).fold(1.0, |a, b| a * b as f64)
        }
        let pos_factor = C1 / (C1 - C2) / fact(EXP as u32);
        let neg_factor = C2 / (C1 - C2) / fact(NEG_EXP as u32);
        let start = 0.0f64;
        let end = self.time_ms as f64 / 1000.0;
        let mut buffers: Vec<String> = self.bold_buffers.clone();
        buffers.sort();
        let max_len = 2 + buffers
            .iter()
            .map(|b| b.len())
            .chain(std::iter::once("time".len()))
            .max()
            .unwrap_or(4);
        let mut header = format!("{:>w$}", "time", w = max_len);
        for b in &buffers {
            header.push_str(&format!("{:>w$}", b, w = max_len));
        }
        self.out_lines.push(header);
        let mut time = start + INC / 2.0;
        while time <= end - INC / 2.0 {
            let mut row = format!("{:>w$.3}", time, w = max_len);
            for b in &buffers {
                let demand = self.bold_demand.get(b).cloned().unwrap_or_default();
                let point = b == "GOAL";
                let pos = bold_values(&demand, time, INC, SETTLE, EXP, 1.0, point);
                let neg = bold_values(&demand, time, INC, SETTLE, NEG_EXP, 1.0, point);
                let v = pos * pos_factor - neg * neg_factor;
                row.push_str(&format!("{:>w$.3}", v, w = max_len));
            }
            self.out_lines.push(row);
            time += INC;
        }
        // `command-output` wraps its argument with `~&…~%`
        // (`misc-utils.lisp:845`); the data block already ends in `~%`, so
        // the table is followed by one blank line.
        self.out_lines.push(String::new());
    }

    /// Schedule a no-op event at absolute model time `at_ms` (a real-time
    /// slack hook's `(schedule-event .5 'fn)`, `meta-process.lisp`): hidden at
    /// medium but wakes an armed conflict-resolution.
    pub fn schedule_dummy(&mut self, at_ms: i64) {
        self.push(at_ms, Prio::Num(0), Out::Always, Event::Dummy);
    }

    /// Emit a raw task warning line (`print-warning`), e.g. the act-touch
    /// stub's "Installing a default touch device…" banner.
    pub fn emit_warning(&mut self, msg: &str) {
        self.out_lines.push(format!("#|Warning: {msg} |#"));
    }

    /// `(mod-focus slot value ...)`: schedule a goal modification at the
    /// current time (`goal.lisp:346`, priority `:max`).
    pub fn mod_focus(&mut self, slots: &[(String, Sexp)]) {
        let mut out = Vec::new();
        for (k, v) in slots {
            if let Ok(val) = Value::from_sexp(v) {
                out.push((k.to_uppercase(), val));
            }
        }
        self.now(Prio::Num(0), Out::Medium, Event::ModFocus { slots: out });
    }

    /// `(add-visicon-features (slot value) ...)`: add a visicon feature and
    /// re-run `proc-display` at the current time. The chunk is auto-named
    /// `CHUNK{n}` (`new-name-fct`).
    pub fn add_visicon(&mut self, slots: &[(String, Sexp)]) -> String {
        let n = *self.name_count.entry("CHUNK".to_string()).or_insert(0);
        self.name_count.insert("CHUNK".to_string(), n + 1);
        let mut x = 0i64;
        let mut y = 0i64;
        let mut w = 0i64;
        let mut ht = 0i64;
        let mut extra: Vec<(String, Value)> = Vec::new();
        for (k, v) in slots {
            let ku = k.to_uppercase();
            match (ku.as_str(), v) {
                ("SCREEN-X", Sexp::Int(i)) => x = *i,
                ("SCREEN-Y", Sexp::Int(i)) => y = *i,
                ("WIDTH", Sexp::Int(i)) => w = *i,
                ("HEIGHT", Sexp::Int(i)) => ht = *i,
                _ => {
                    if let Ok(val) = Value::from_sexp(v) {
                        extra.push((ku, val));
                    }
                }
            }
        }
        self.features.push(Feature {
            name: format!("CHUNK{n}"),
            kind: "VISUAL-LOCATION".to_string(),
            value: String::new(),
            x,
            y,
            width: w,
            height: ht,
            extra,
            tstamp_ms: self.time_ms,
            new_p: true,
            finst_ms: None,
        });
        self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
        format!("CHUNK{n}")
    }

    /// `(modify-visicon-features (feature screen-x X screen-y Y))`: move the
    /// named feature at absolute model time `at_ms`.
    pub fn schedule_modify_visicon(&mut self, at_ms: i64, name: &str, x: i64, y: i64) {
        self.push(
            at_ms,
            Prio::Num(0),
            Out::Never,
            Event::ModifyVisicon {
                name: name.to_string(),
                x,
                y,
            },
        );
    }

    /// Trace on/off (`:v`), used by scripted drivers that hide segments.
    pub fn set_trace(&mut self, on: bool) {
        self.trace_on = on;
    }

    /// `(model-output "…")` (`misc-utils.lisp:801`): a raw echoed line (no
    /// timestamp/module formatting), gated by the model trace (`:v`).
    pub fn model_output(&mut self, s: &str) {
        self.out(s.to_string());
    }

    /// Install task-file `defun`s for the bounded `!eval!`/`!bind!` evaluator.
    pub fn set_user_fns(&mut self, fns: HashMap<String, crate::model::UserDefun>) {
        self.user_fns = fns;
    }

    /// Override the trace module-column width (`max-module-name-length`).
    pub fn set_module_width(&mut self, width: usize) {
        self.cfg.module_width = width;
    }

    /// `(sgp :trace-detail X)` mid-driver.
    pub fn set_detail(&mut self, detail: Detail) {
        self.cfg.detail = detail;
    }

    /// `(sgp :key value)` mid-driver. Known parameters update the live
    /// config; unknown ones are accepted (no-op) so specs are not rejected.
    pub fn apply_sgp(&mut self, key: &str, val: &Sexp) -> Result<(), EngineError> {
        let num = |v: &Sexp| -> Option<f32> {
            match v {
                Sexp::Int(i) => Some(*i as f32),
                Sexp::Float(x) => Some(*x as f32),
                _ => None,
            }
        };
        let truthy = !matches!(val, Sexp::Sym(s) if s == "NIL");
        match key.to_uppercase().as_str() {
            ":EMMA" => self.cfg.emma = truthy,
            // `(ssp :starting-parameters (:key val ...))`: apply each nested
            // parameter and remember them so `reset` re-applies them
            // (`central-parameters.lisp:131`). `nil` clears them.
            ":STARTING-PARAMETERS" => {
                self.starting_params.clear();
                if let Sexp::List(items) = val {
                    for kv in items.as_chunks::<2>().0 {
                        if let Some(k) = kv[0].as_sym() {
                            self.starting_params.push((k.to_string(), kv[1].clone()));
                            self.apply_sgp(k, &kv[1])?;
                        }
                    }
                }
            }
            ":PPM" => self.cfg.ppm = num(val),
            ":EGS" => self.cfg.egs = num(val).unwrap_or(0.0),
            // `(sgp :seed (S C))` (`random.lisp:417`): init the stream from `S`
            // and discard `C` draws; `nil` clears it. Applied immediately and
            // remembered so `reset` re-seeds (`engine.rs` reset_impl).
            ":SEED" => match val {
                Sexp::List(items) => {
                    let s = match items.first() {
                        Some(Sexp::Int(i)) => *i as u64,
                        _ => 0,
                    };
                    let c = match items.get(1) {
                        Some(Sexp::Int(i)) => *i as u64,
                        _ => 0,
                    };
                    self.cfg.seed = Some((s, c));
                    self.rng.borrow_mut().set_seed(s, c);
                }
                Sexp::Sym(s) if s == "NIL" => self.cfg.seed = None,
                _ => {}
            },
            ":CIA" => self.cfg.cia = truthy,
            ":RIR" => self.cfg.rir = truthy,
            ":EPL" => self.cfg.epl = truthy,
            ":PCT" => self.cfg.pct = truthy,
            // `(sgp :v nil)` hides a trace segment (`printing.lisp:399`,
            // default `t`); a scripted driver toggles it mid-run
            // (`tutorial-unit5modeling-v7-p4-run`).
            ":V" => self.trace_on = truthy,
            ":LF" => {
                if let Some(x) = num(val) {
                    self.cfg.lf_ms = (x * 1000.0).round() as i64;
                }
            }
            ":ESC" => self.cfg.esc = truthy,
            ":RT" => {
                if let Some(x) = num(val) {
                    self.cfg.rt = x;
                }
            }
            ":ANS" => self.cfg.ans = num(val).filter(|x| *x > 0.0),
            ":MP" => self.cfg.mp = num(val),
            ":MD" => {
                if let Some(x) = num(val) {
                    self.cfg.md = x;
                }
            }
            ":MS" => {
                if let Some(x) = num(val) {
                    self.cfg.ms = x;
                }
            }
            ":MAS" => self.cfg.mas = num(val),
            ":BLC" => {
                if let Some(x) = num(val) {
                    self.cfg.blc = x;
                }
            }
            ":BLL" => {
                self.cfg.bll = num(val).filter(|b| *b > 0.0);
                self.cfg.act_scale = match self.cfg.bll {
                    Some(b) => (0.001f32).powf(-b).ln(),
                    None => 0.0,
                };
            }
            ":UL" => self.cfg.ul = truthy,
            ":ULT" => self.cfg.ult = truthy,
            ":RECORD-TICKS" => self.cfg.record_ticks = truthy,
            ":RANDOMIZE-TIME" => {
                self.cfg.randomize_scale = match val {
                    Sexp::Sym(s) if s == "NIL" => None,
                    Sexp::Sym(_) => Some(3.0),
                    Sexp::Float(x) => Some(*x as f32),
                    Sexp::Int(i) => Some(*i as f32),
                    _ => None,
                }
            }
            ":VPFT" => self.cfg.vpft = truthy,
            ":VIDT" => self.cfg.vidt = truthy,
            ":TOUCH-DRAG-DELAY" => self.cfg.touch_drag_delay = truthy,
            ":LEFT-START" => {
                if let Sexp::List(v) = val {
                    if let (Some(a), Some(b)) = (
                        v.first().and_then(|x| match x {
                            Sexp::Int(i) => Some(*i),
                            _ => None,
                        }),
                        v.get(1).and_then(|x| match x {
                            Sexp::Int(i) => Some(*i),
                            _ => None,
                        }),
                    ) {
                        self.cfg.touch_left_start = (a, b);
                    }
                }
            }
            ":RIGHT-START" => {
                if let Sexp::List(v) = val {
                    if let (Some(a), Some(b)) = (
                        v.first().and_then(|x| match x {
                            Sexp::Int(i) => Some(*i),
                            _ => None,
                        }),
                        v.get(1).and_then(|x| match x {
                            Sexp::Int(i) => Some(*i),
                            _ => None,
                        }),
                    ) {
                        self.cfg.touch_right_start = (a, b);
                    }
                }
            }
            ":IU" => {
                if let Some(x) = num(val) {
                    self.cfg.iu = x;
                }
            }
            ":ALPHA" => {
                if let Some(x) = num(val) {
                    self.cfg.alpha = x;
                }
            }
            ":ER" => self.cfg.er = truthy,
            ":CRT" => self.cfg.crt = truthy,
            ":CST" => self.cfg.cst = truthy,
            ":ACT" => {
                self.cfg.act = match val {
                    Sexp::Sym(s) if s == "NIL" => None,
                    Sexp::Sym(s) => Some(match s.as_str() {
                        "T" => Act::T,
                        "HIGH" => Act::High,
                        "MEDIUM" => Act::Medium,
                        "LOW" => Act::Low,
                        _ => return Err(bad(format!("bad :act {s}"))),
                    }),
                    _ => return Err(bad(format!("bad :act {val:?}"))),
                };
            }
            ":NSJI" => self.cfg.nsji = truthy,
            ":GA" => {
                if let Some(x) = num(val) {
                    self.cfg.ga = x;
                }
            }
            ":DAT" => {
                if let Some(x) = num(val) {
                    self.cfg.dat_ms = (x * 1000.0).round() as i64;
                }
            }
            // `:UTILITY-OFFSETS`, seeds, etc. accepted without an observable
            // effect here (the ppm offset is implicit in `:ppm`).
            _ => {}
        }
        Ok(())
    }

    /// `(sgp :seed (S C))`: init + discard, same as construction.
    pub fn set_seed(&mut self, seed: (u64, u64)) {
        self.rng.borrow_mut().set_seed(seed.0, seed.1);
    }

    /// Consume `n` draws from the random stream without using the values
    /// (a task's setup draws between runs).
    pub fn consume_rng(&mut self, n: u64) {
        for _ in 0..n {
            self.rng.borrow_mut().next_u32();
        }
    }

    /// `pdisable-fct` (`procedural-cmds.lisp:541`): skip the named
    /// productions in conflict resolution; an empty list disables all.
    pub fn pdisable(&mut self, names: &[String]) {
        if names.is_empty() {
            for p in &self.model.productions {
                self.disabled_prods.insert(p.name.to_uppercase());
            }
        } else {
            for n in names {
                self.disabled_prods.insert(n.to_uppercase());
            }
        }
    }

    /// `penable-fct` (`procedural-cmds.lisp:568`): re-enable the named
    /// productions; an empty list enables all.
    pub fn penable(&mut self, names: &[String]) {
        if names.is_empty() {
            self.disabled_prods.clear();
        } else {
            for n in names {
                self.disabled_prods.remove(&n.to_uppercase());
            }
        }
    }
}

impl Engine {
    /// `production-parameter-fct` (`utility-and-reward-1.lisp:660`): the
    /// `Parameters for production` block printed by `spp-fct` after a
    /// composition. `utility` is the cached `production-utility` (nil until a
    /// conflict resolution computes it); `u` is the base `production-u`.
    pub(super) fn print_prod_params(
        &mut self,
        name: &str,
        u: f32,
        utility: Option<f32>,
        reward: Option<f32>,
    ) {
        self.out(format!("Parameters for production {name}:"));
        if self.cfg.esc {
            match utility {
                Some(v) => self.out(format!(" :utility {v:6.3}")),
                None => self.out(" :utility    NIL".to_string()),
            }
        }
        self.out(format!(" :u  {u:6.3}"));
        self.out(format!(" :at {}", fmt_6_3(self.cfg.dat_ms as f32 / 1000.0)));
        if self.cfg.esc && self.cfg.ul {
            // `production-parameter-fct` (`utility-and-reward-1.lisp:686`):
            // the reward lines print only with `:esc` + `:ul`.
            match reward {
                Some(r) => self.out(format!(" :reward {r:6.3}")),
                None => self.out(" :reward    NIL".to_string()),
            }
            self.out(" :fixed-utility    NIL".to_string());
        }
    }
}

/// RPC-facing operations backing the remote command layer
/// (`framework/*.lisp`, `core-modules/*.lisp`). These expose read-only or
/// side-effecting primitives that `evaluate <name> <args>` maps onto.
impl Engine {
    /// `mp-time` (seconds).
    pub fn time_secs(&self) -> f64 {
        self.time_ms as f64 / 1000.0
    }

    /// `mp-time-ms`.
    pub fn time_ms(&self) -> i64 {
        self.time_ms
    }

    /// `buffer-read <buffer>` (`framework/buffers.lisp:...`): the name of the
    /// chunk currently in the buffer, or `None`.
    pub fn buffer_read(&self, buffer: &str) -> Option<String> {
        let b = self.buffers.get(&buffer.to_uppercase())?;
        b.chunk.as_ref().and_then(|c| c.name.clone())
    }

    /// `buffer-status` (`procedural.lisp`): the buffer query state.
    pub fn buffer_status(&self, buffer: &str) -> &'static str {
        match self.buffers.get(&buffer.to_uppercase()) {
            None => "absent",
            Some(b) if b.failed => "error",
            Some(b) if b.busy => "busy",
            Some(b) if b.chunk.is_none() => "empty",
            Some(_) => "free",
        }
    }

    /// `clear-buffer <name>`: empty the buffer.
    pub fn clear_buffer_rpc(&mut self, buffer: &str) {
        let b = self.buffer_mut(&buffer.to_uppercase());
        b.chunk = None;
        b.failed = false;
        b.modified = false;
        self.changed();
    }

    /// `chunk-p`: whether `name` names a known chunk (DM or `define-chunks`).
    pub fn chunk_p(&self, name: &str) -> bool {
        let n = name.to_uppercase();
        self.named.contains_key(&n)
            || self
                .dm
                .iter()
                .any(|c| c.name.as_deref() == Some(n.as_str()))
    }

    /// `chunk-slot-value <chunk> <slot>`: the printed slot value.
    pub fn chunk_slot(&self, name: &str, slot: &str) -> Option<String> {
        self.chunk_name_slot(&name.to_uppercase(), &slot.to_uppercase())
    }

    /// `chunk-copied-from`: the source of a copied chunk, if any.
    pub fn chunk_copied_from(&self, name: &str) -> Option<String> {
        self.copy_source.get(&name.to_uppercase()).cloned()
    }

    /// `act-r-random` for an integer limit (0 when the limit is not positive).
    pub fn random_int_rpc(&mut self, limit: i64) -> i64 {
        if limit <= 0 {
            0
        } else {
            self.rng.borrow_mut().random_int(limit as u64) as i64
        }
    }

    /// `act-r-random` for a float limit.
    pub fn random_float_rpc(&mut self, limit: f64) -> f64 {
        self.rng.borrow_mut().random_float(limit as f32) as f64
    }

    /// `schedule-event-relative`/`schedule-event-now`/`schedule-event`: queue a
    /// task command at an absolute model time (`scheduling.lisp`). `params`
    /// are the command's arguments as AT-R values.
    pub fn schedule_command_rpc(&mut self, at_ms: i64, name: &str, params: &[crate::sexpr::Sexp]) {
        use crate::sexpr::Sexp;
        fn value(s: &Sexp) -> Value {
            match s {
                Sexp::Sym(x) => Value::Sym(x.clone()),
                Sexp::Int(i) => Value::Int(*i),
                Sexp::Float(x) => Value::Num(*x),
                Sexp::Str(x) => Value::Str(x.clone()),
                Sexp::List(items) => Value::List(items.iter().map(value).collect()),
            }
        }
        let params = params.iter().map(value).collect();
        self.push(
            at_ms,
            Prio::Num(0),
            Out::Medium,
            Event::Command {
                name: name.to_string(),
                params,
            },
        );
    }

    /// A scheduled command's task `defun` names (for name resolution).
    pub fn has_user_fn(&self, name: &str) -> bool {
        self.user_fns.contains_key(&name.to_uppercase())
    }

    /// `set-buffer-chunk <buffer> <chunk-name>`: place a copy of a named
    /// chunk (`framework/buffers.lisp:1325`).
    pub fn set_buffer_chunk_named(&mut self, buffer: &str, chunk: &str) -> Result<(), EngineError> {
        let inst = self.instantiate(buffer, NewChunk::Named(chunk.to_string()))?;
        let b = self.buffer_mut(&buffer.to_uppercase());
        b.chunk = Some(inst);
        b.failed = false;
        b.modified = false;
        self.changed();
        Ok(())
    }

    /// `set-buffer-chunk <buffer> <spec>`: build a chunk from a description.
    pub fn set_buffer_chunk_desc(
        &mut self,
        buffer: &str,
        desc: &ChunkDesc,
    ) -> Result<(), EngineError> {
        let inst = match super::rhs::new_chunk(desc)? {
            NewChunk::Instance(c) => c,
            NewChunk::Spec { ctype, slots, .. } => ChunkInst {
                name: None,
                ctype,
                slots,
                bare: false,
            },
            NewChunk::Named(n) => self.instantiate(buffer, NewChunk::Named(n))?,
        };
        let b = self.buffer_mut(&buffer.to_uppercase());
        b.chunk = Some(inst);
        b.failed = false;
        b.modified = false;
        self.changed();
        Ok(())
    }

    /// `(sgp <param>)` / `get-parameter-value`: the printed current value of a
    /// supported model parameter (`framework/printing.lisp`,
    /// `modules/utility-and-reward-1.lisp`). `None` for an unknown name.
    pub fn get_param(&self, key: &str) -> Option<String> {
        let b = |x: bool| if x { "T" } else { "NIL" }.to_string();
        Some(match key.to_uppercase().as_str() {
            ":V" => b(self.trace_on),
            ":ESC" => b(self.cfg.esc),
            ":UL" => b(self.cfg.ul),
            ":ULT" => b(self.cfg.ult),
            ":EPL" => b(self.cfg.epl),
            ":PCT" => b(self.cfg.pct),
            ":CIA" => b(self.cfg.cia),
            ":RIR" => b(self.cfg.rir),
            ":RT" => self.cfg.rt.to_string(),
            ":LF" => format!("{}", self.cfg.lf_ms as f64 / 1000.0),
            ":BLC" => self.cfg.blc.to_string(),
            ":EGS" => self.cfg.egs.to_string(),
            ":ALPHA" => self.cfg.alpha.to_string(),
            ":IU" => self.cfg.iu.to_string(),
            ":MD" => self.cfg.md.to_string(),
            ":MS" => self.cfg.ms.to_string(),
            ":ANS" => self.cfg.ans.map(|x| x.to_string()).unwrap_or("NIL".into()),
            ":BLL" => self.cfg.bll.map(|x| x.to_string()).unwrap_or("NIL".into()),
            ":SJI" => "T".to_string(),
            ":TRACE-DETAIL" => format!("{:?}", self.cfg.detail).to_uppercase(),
            ":SEED" => self
                .cfg
                .seed
                .map(|(a, b)| format!("({a} {b})"))
                .unwrap_or_else(|| "NIL".into()),
            _ => return None,
        })
    }
}

/// Parse an `sdm` specification (`{modifier} slot value …`,
/// `sdm-external`, `commands/dm-commands.lisp:511`) into predicates. A
/// single punctuation token is a modifier (`-` = not equal).
fn parse_sdm_spec(spec: &[String]) -> Vec<(Option<char>, String, String)> {
    let mut preds = Vec::new();
    let mut i = 0;
    while i < spec.len() {
        let tok = &spec[i];
        let modifier = {
            let mut cs = tok.chars();
            match (cs.next(), cs.next()) {
                (Some(c), None) if c.is_ascii_punctuation() => Some(c),
                _ => None,
            }
        };
        let (slot, vi) = match modifier {
            Some(_) => (spec.get(i + 1).cloned().unwrap_or_default(), i + 2),
            None => (tok.clone(), i + 1),
        };
        let val = spec.get(vi).cloned().unwrap_or_default();
        preds.push((modifier, slot.to_uppercase(), val.to_uppercase()));
        i = vi + 1;
    }
    preds
}

/// Whether a chunk satisfies every `sdm` predicate. `ISA` tests the chunk
/// type; other slots compare their printed value.
fn sdm_match(c: &ChunkInst, preds: &[(Option<char>, String, String)]) -> bool {
    preds.iter().all(|(m, slot, val)| {
        let actual = if slot == "ISA" {
            c.ctype.to_uppercase()
        } else {
            c.get(slot).show().to_uppercase()
        };
        match m {
            Some('-') => actual != *val,
            _ => actual == *val,
        }
    })
}

/// `buffer-status` (`framework/buffers.lisp:953`): print the per-buffer query
/// table and return the buffer names. The standard query set, plus the
/// module-specific rows the vision buffer adds while idle.
impl Engine {
    pub fn print_buffer_status(&mut self, buffers: &[String]) -> Vec<String> {
        let t = |x: bool| if x { "T" } else { "NIL" };
        let mut names = Vec::new();
        for name in buffers {
            let bname = name.to_uppercase();
            let (empty, failed, busy) = match self.buffers.get(&bname) {
                None => (true, false, false),
                Some(b) => (b.chunk.is_none(), b.failed, b.busy),
            };
            let mut lines = vec![format!("{bname}:")];
            let mut row = |label: &str, val: &str| {
                lines.push(format!("  {label:<22}: {val}"));
            };
            row("buffer empty", t(empty));
            row("buffer full", "NIL");
            row("buffer failure", t(failed));
            row("buffer requested", "NIL");
            row("buffer unrequested", "NIL");
            row("state free", t(!busy && !failed));
            row("state busy", t(busy));
            row("state error", t(failed));
            if bname == "VISUAL" {
                // `vision-status` (`vision.lisp`): the vision module is idle
                // when nothing is being perceived.
                let idle = !self.vis_busy();
                row("preparation free", t(idle));
                row("preparation busy", t(!idle));
                row("processor free", t(idle));
                row("processor busy", t(!idle));
                row("execution free", t(idle));
                row("execution busy", t(!idle));
                row("last-command", "NONE");
                row("scene-change", "NIL");
                row("scene-change-value", "NIL");
            }
            self.out_lines.extend(lines);
            names.push(bname);
        }
        names
    }

    /// Whether the vision module is currently processing (`vis_busy`
    /// tracked via the visual/visual-location buffers).
    fn vis_busy(&self) -> bool {
        let b = |n: &str| self.buffers.get(n).map(|b| b.busy).unwrap_or(false);
        b("VISUAL") || b("VISUAL-LOCATION")
    }
}

/// `whynot` (`commands/procedural-cmds.lisp:597`): print, for each named
/// production, whether it is in the current conflict set plus its definition
/// and (when not) why it fails. Returns the conflict set, as `whynot-fct` does.
impl Engine {
    pub fn print_whynot(&mut self, names: &[String]) -> Vec<String> {
        let prods: Vec<crate::model::Production> = names
            .iter()
            .filter_map(|n| {
                let up = n.to_uppercase();
                self.model
                    .productions
                    .iter()
                    .find(|p| p.name == up)
                    .cloned()
            })
            .collect();
        for prod in &prods {
            // `command-output "~%Production …"` prints a leading newline.
            self.out_lines.push(String::new());
            if self.last_conflict_set.iter().any(|n| n == &prod.name) {
                self.out_lines
                    .push(format!("Production {} matches:", prod.name));
            } else {
                self.out_lines
                    .push(format!("Production {} does NOT match.", prod.name));
            }
            let text = super::compilation::render_production(prod, &Bindings::new(), &[]);
            for line in text.lines() {
                self.out_lines.push(line.to_string());
            }
            if !self.last_conflict_set.iter().any(|n| n == &prod.name) {
                self.out_lines.push("It fails because: ".to_string());
                let reason = self.why_not_match(prod);
                self.out_lines.push(reason);
            }
        }
        self.last_conflict_set.clone()
    }

    /// The first failing LHS condition's explanation
    /// (`failure-reason-string`, `procedural.lisp:1981`). The compiled `isa`
    /// condition is checked first (its `nil`-valued slots report "has the
    /// slot"), then the remaining slot tests with real binding so `=var`
    /// reuse reports "does not satisfy the constraints".
    fn why_not_match(&self, prod: &crate::model::Production) -> String {
        use crate::model::SlotOp;
        let mut b = Bindings::new();
        for item in &prod.lhs {
            match item {
                LhsItem::Match { buffer, tests } => {
                    let chunk = match self.buffers.get(buffer).and_then(|x| x.chunk.as_ref()) {
                        Some(c) => c,
                        None => return format!("The {buffer} buffer is empty."),
                    };
                    for t in tests {
                        if let Pattern::Nil = t.value {
                            if chunk.get(&t.slot) != Value::Nil {
                                return format!(
                                    "The chunk in the {buffer} buffer has the slot {}.",
                                    t.slot
                                );
                            }
                        }
                    }
                    for t in tests {
                        if t.slot == "ISA" {
                            continue;
                        }
                        let slot = self.resolve_slot_name(&t.slot, &b);
                        let got = chunk.get(&slot);
                        let mut ppm = Vec::new();
                        let ok = match t.op {
                            SlotOp::Gt | SlotOp::Lt | SlotOp::Ge | SlotOp::Le => {
                                self.rel_test(t.op, &t.value, &got)
                            }
                            SlotOp::Neg => self
                                .test_value(&t.value, &got, &mut b, true)
                                .unwrap_or(false),
                            SlotOp::Eq => self
                                .match_eq(buffer, &slot, &t.value, &got, &mut b, &mut ppm)
                                .unwrap_or(false),
                        };
                        if !ok {
                            return match &t.value {
                                Pattern::Nil => {
                                    format!("The chunk in the {buffer} buffer does not have slot {slot}.")
                                }
                                Pattern::Var(_) => format!(
                                    "The value in the {slot} slot of the chunk in the {buffer} buffer does not satisfy the constraints."
                                ),
                                other => format!(
                                    "The {slot} slot of the chunk in the {buffer} buffer does not have the value {}.",
                                    super::compilation::show_pat(other, &Bindings::new())
                                ),
                            };
                        }
                    }
                }
                LhsItem::Query { buffer, queries } => {
                    for (attr, val) in queries {
                        let v = super::compilation::show_pat(val, &Bindings::new());
                        if !self.query_holds(buffer, attr, val) {
                            return format!("The {attr} {v} query of the {buffer} buffer failed.");
                        }
                    }
                }
                _ => {}
            }
        }
        "The production did not match.".to_string()
    }

    /// Whether a `?buffer>` query holds (the subset `crt_failure_reason` uses).
    fn query_holds(&self, buffer: &str, attr: &str, val: &Pattern) -> bool {
        match (attr, val) {
            ("BUFFER", Pattern::Sym(s)) if s == "FAILURE" => {
                self.buffers.get(buffer).map(|b| b.failed).unwrap_or(false)
            }
            ("BUFFER", Pattern::Sym(s)) if s == "EMPTY" => self
                .buffers
                .get(buffer)
                .map(|b| b.chunk.is_none() && !b.failed)
                .unwrap_or(false),
            ("BUFFER", Pattern::Sym(s)) if s == "FULL" => self
                .buffers
                .get(buffer)
                .map(|b| b.chunk.is_some())
                .unwrap_or(false),
            ("STATE", Pattern::Sym(s)) if s == "FREE" => self.query_free(buffer).unwrap_or(false),
            ("STATE", Pattern::Sym(s)) if s == "BUSY" => !self.query_free(buffer).unwrap_or(false),
            ("ERROR", Pattern::Nil) => self.buffer_error(buffer),
            ("ERROR", Pattern::Sym(s)) if s == "T" => !self.buffer_error(buffer),
            _ => true,
        }
    }
}

/// RPC device/vision/motor operations (`commands/*`, `core-modules/vision.lisp`).
impl Engine {
    /// `delete-visicon-features` / `delete-all-visicon-features`.
    pub fn remove_visicon(&mut self, name: &str) {
        self.features.retain(|f| f.name != name);
        self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
    }

    pub fn clear_visicon(&mut self) {
        self.features.clear();
        self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
    }

    /// `print-visicon` (`vision.lisp`): one line per feature with its slots.
    pub fn print_visicon_rpc(&mut self) -> Vec<String> {
        let feats = self.features.clone();
        let mut out = vec![format!("The visicon contains {} features:", feats.len())];
        for f in &feats {
            let mut parts = vec![format!("SCREEN-X {}", f.x), format!("SCREEN-Y {}", f.y)];
            if f.width != 0 {
                parts.push(format!("WIDTH {}", f.width));
            }
            if f.height != 0 {
                parts.push(format!("HEIGHT {}", f.height));
            }
            for (k, v) in &f.extra {
                parts.push(format!("{k} {}", v.show()));
            }
            out.push(format!("({})", parts.join(" ")));
        }
        self.out_lines.extend(out.clone());
        out
    }

    /// `new-tone-sound <freq> <duration> [onset] [time-in-ms]`: schedule a tone.
    pub fn new_tone_sound_rpc(&mut self, freq: i64, duration: i64, onset_ms: i64) {
        let idx = self.add_sound(freq, duration, onset_ms);
        let at = self.time_ms().max(onset_ms);
        self.push(at, Prio::Num(0), Out::Medium, Event::ToneSound { idx });
    }

    /// `(pbreak <name>...)`: halt before firing the named production.
    pub fn pbreak(&mut self, names: &[String]) {
        for n in names {
            let n = n.to_uppercase();
            if !self.zz_breaks.contains(&n) {
                self.zz_breaks.push(n);
            }
        }
    }

    /// `(punbreak <name>...)`.
    pub fn punbreak(&mut self, names: &[String]) {
        for n in names {
            let n = n.to_uppercase();
            self.zz_breaks.retain(|b| b != &n);
        }
    }
}

// --- Live experiment window (`open-exp-window` etc., `devices/virtual`) -----
//
// The scripted device path (`pending_texts`) replays a pre-captured task; the
// RPC path below builds the same visicon features from live `add-*-to-exp-
// window` calls, using the geometry in `devices/virtual/uwi.lisp` +
// `build-string-feats` (`core-modules/vision.lisp:3558`).

/// A `chop-string` token: a word, or a run of spaces (its length).
enum Chop {
    Word(String),
    Spaces(i64),
}

/// `chop-string` (`vision.lisp:3619`): split into words and space-run counts.
/// `:WORD`/`:MISC` are treated alike (the corpus's texts are alphanumeric).
fn chop_string(text: &str) -> Vec<Chop> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut accum: Vec<Chop> = Vec::new();
    let mut wrd = String::new();
    let mut cnt = 0i64;
    let mut old_space = text
        .chars()
        .next()
        .map(char::is_whitespace)
        .unwrap_or(false);
    for ch in text.chars() {
        let space = ch.is_whitespace();
        if !space && !old_space {
            wrd.push(ch);
            cnt = 0;
        } else if !space && old_space {
            accum.push(Chop::Spaces(cnt + 1));
            wrd.clear();
            wrd.push(ch);
        } else if space && !old_space {
            accum.push(Chop::Word(std::mem::take(&mut wrd)));
            cnt = 0;
        } else {
            cnt += 1;
        }
        old_space = space;
    }
    if !wrd.is_empty() {
        accum.push(Chop::Word(wrd));
    }
    if let Some(Chop::Spaces(n)) = accum.first_mut() {
        *n -= 1;
    }
    accum
}

/// Lisp `(round n d)` — round half to even.
fn round_half_even(n: i64, d: i64) -> i64 {
    let q = n.div_euclid(d);
    let r = n.rem_euclid(d);
    let t = 2 * r;
    if t > d {
        q + 1
    } else if t < d || q % 2 == 0 {
        q
    } else {
        q + 1
    }
}

/// Lisp `(round x)` — round half to even, on a float.
fn round_half_even_f(x: f64) -> i64 {
    let fl = x.floor();
    let diff = x - fl;
    if diff > 0.5 {
        fl as i64 + 1
    } else if diff < 0.5 || (fl as i64) % 2 == 0 {
        fl as i64
    } else {
        fl as i64 + 1
    }
}

/// `pts-to-orientation` (`uwi.lisp:503`): degrees from -89..90, 0 vertical.
fn pts_to_orientation(p1: (i64, i64), p2: (i64, i64)) -> i64 {
    let dx = (p2.0 - p1.0) as f64;
    let dy = (p2.1 - p1.1) as f64;
    let theta = dy.atan2(dx);
    let angle = (theta.to_degrees().round() as i64) - 90;
    if angle < -90 {
        angle + 180
    } else if angle > 90 {
        angle - 180
    } else if angle == -90 {
        90
    } else {
        angle
    }
}

impl Engine {
    /// `open-exp-window`: set the window origin and clear live items.
    pub fn open_window(&mut self, x: i64, y: i64) {
        self.window_origin = (x, y);
        self.clear_window();
    }

    /// `clear-exp-window`: drop all live window items and visicon features.
    pub fn clear_window(&mut self) {
        self.window_items.clear();
        self.features.clear();
        self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
    }

    /// `add-text-to-exp-window`: word features (`build-string-feats`,
    /// `optimize-p` t) placed relative to the window origin. Returns the item
    /// key.
    pub fn add_window_text(
        &mut self,
        text: &str,
        x: i64,
        y: i64,
        _width: i64,
        _height: i64,
        font_size: i64,
    ) -> String {
        let key = format!("TEXT-VDI-{}", self.window_item_seq);
        self.window_item_seq += 1;
        let (ox, oy) = self.window_origin;
        // `char-width = (round font-size 12/7)`, `text-height = (round
        // font-size 12/10)` (`uwi.lisp:447`).
        let char_w = round_half_even_f(font_size as f64 * 7.0 / 12.0).max(1);
        let text_h = round_half_even_f(font_size as f64 * 10.0 / 12.0).max(1);
        let line_h = round_half_even_f(text_h as f64 / 0.8).max(text_h + 1);
        let y_screen = y + oy + round_half_even(line_h, 2);
        let mut names = Vec::new();
        let mut curr_x = x + 1;
        for tok in chop_string(text) {
            match tok {
                Chop::Word(w) => {
                    let wd = char_w * w.chars().count() as i64;
                    let name = format!("VISUAL-LOCATION{}", self.feat_seq);
                    self.feat_seq += 1;
                    self.features.push(Feature {
                        name: name.clone(),
                        kind: "TEXT".to_string(),
                        value: w.clone(),
                        x: curr_x + ox + round_half_even(wd, 2),
                        y: y_screen,
                        width: wd,
                        height: text_h,
                        extra: vec![("TEXT".to_string(), Value::Sym("T".to_string()))],
                        tstamp_ms: self.time_ms,
                        new_p: true,
                        finst_ms: None,
                    });
                    names.push(name);
                    curr_x += wd;
                }
                Chop::Spaces(n) => curr_x += n * char_w,
            }
        }
        self.window_items.push((key.clone(), names));
        self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
        key
    }

    /// `add-line-to-exp-window`: a `line` feature (`uwi.lisp:511`).
    pub fn add_window_line(&mut self, start: (i64, i64), end: (i64, i64), color: &str) -> String {
        let key = format!("LINE-VDI-{}", self.window_item_seq);
        self.window_item_seq += 1;
        let (ox, oy) = self.window_origin;
        let (x1, y1) = (ox + start.0, oy + start.1);
        let (x2, y2) = (ox + end.0, oy + end.1);
        let name = format!("VISUAL-LOCATION{}", self.feat_seq);
        self.feat_seq += 1;
        self.features.push(Feature {
            name: name.clone(),
            kind: "LINE".to_string(),
            value: pts_to_orientation((x1, y1), (x2, y2)).to_string(),
            x: ox + (start.0 + end.0).div_euclid(2),
            y: oy + (start.1 + end.1).div_euclid(2),
            width: (start.0 - end.0).abs(),
            height: (start.1 - end.1).abs(),
            extra: vec![
                ("COLOR".to_string(), Value::Sym(color.to_uppercase())),
                ("END1-X".to_string(), Value::Int(x1)),
                ("END1-Y".to_string(), Value::Int(y1)),
                ("END2-X".to_string(), Value::Int(x2)),
                ("END2-Y".to_string(), Value::Int(y2)),
            ],
            tstamp_ms: self.time_ms,
            new_p: true,
            finst_ms: None,
        });
        self.window_items.push((key.clone(), vec![name]));
        self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
        key
    }

    /// `add-button-to-exp-window`: a `button` feature plus its text
    /// (`uwi.lisp:358`); the button itself has the item key as its value.
    pub fn add_window_button(
        &mut self,
        text: &str,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        color: &str,
    ) -> String {
        let key = format!("BUTTON-VDI-{}", self.window_item_seq);
        self.window_item_seq += 1;
        let (ox, oy) = self.window_origin;
        let name = format!("VISUAL-LOCATION{}", self.feat_seq);
        self.feat_seq += 1;
        self.features.push(Feature {
            name: name.clone(),
            kind: "BUTTON".to_string(),
            value: key.clone(),
            x: ox + x + width.div_euclid(2),
            y: oy + y + height.div_euclid(2),
            width,
            height,
            extra: vec![("COLOR".to_string(), Value::Sym(color.to_uppercase()))],
            tstamp_ms: self.time_ms,
            new_p: true,
            finst_ms: None,
        });
        let mut names = vec![name];
        // The button's text is a separate text item (centered on the button).
        let tkey = self.add_window_text(text, x, y, width, height, 12);
        if let Some((_, tnames)) = self.window_items.iter().find(|(k, _)| k == &tkey) {
            names.extend(tnames.iter().cloned());
        }
        self.window_items.push((key.clone(), names));
        key
    }

    /// `remove-items-from-exp-window`: drop the named items' features.
    pub fn remove_window_items(&mut self, keys: &[String]) {
        let mut dropped: Vec<String> = Vec::new();
        self.window_items.retain(|(k, feats)| {
            let hit = keys
                .iter()
                .any(|x| x.eq_ignore_ascii_case(k) || x.to_uppercase().contains(&k.to_uppercase()));
            if hit {
                dropped.extend(feats.iter().cloned());
                false
            } else {
                true
            }
        });
        if !dropped.is_empty() {
            self.features.retain(|f| !dropped.contains(&f.name));
            self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
        }
    }

    /// `modify-text-for-exp-window`: move the text item's features by delta.
    pub fn modify_window_text(&mut self, key: &str, x: Option<i64>, y: Option<i64>) {
        let Some((_, names)) = self.window_items.iter().find(|(k, _)| k == key) else {
            return;
        };
        let names = names.clone();
        for f in self.features.iter_mut().filter(|f| names.contains(&f.name)) {
            if let Some(nx) = x {
                f.x = nx;
            }
            if let Some(ny) = y {
                f.y = ny;
            }
            f.tstamp_ms = self.time_ms;
        }
        self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
    }
}

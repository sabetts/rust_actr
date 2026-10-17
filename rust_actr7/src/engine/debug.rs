//! Trace/debug output, parameter setters, visicon and buffer dumps.
use super::compilation::bold_values;
use super::*;

/// The implicit queries `pp` prints for a production: one `?buffer> state
/// free` per requested buffer (`+` request or `*` modify-request) that has no
/// explicit `?buffer>` condition and is not exempt (`:do-not-query`). ACT-R's
/// `production-statement-text` shows these in the `(P …)` block.
fn pp_implicit_queries(prod: &crate::model::Production, exempt: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for a in &prod.rhs {
        let buffer = match a {
            RhsAction::Request { buffer, .. } | RhsAction::ModifyRequest { buffer, .. } => buffer,
            _ => continue,
        };
        let explicit = prod
            .lhs
            .iter()
            .any(|item| matches!(item, LhsItem::Query { buffer: b, .. } if b == buffer));
        if !exempt.contains(buffer) && !explicit && !out.contains(buffer) {
            out.push(buffer.clone());
        }
    }
    out
}

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
        // slots by `act-r-slot-index` and skips unset slots. That index is
        // model-wide (`chunk-type-fct`, `chunk-types.lisp:434`), not the
        // chunk's own type order: `math-fact`'s `result` sorts first because
        // `simple-value` declared it earlier.
        let slot_index = self.model.slot_index();
        let mut idx: Vec<usize> = (0..c.slots.len()).collect();
        idx.sort_by_key(|&i| {
            *slot_index
                .get(&c.slots[i].0.to_uppercase())
                .unwrap_or(&usize::MAX)
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

    /// `(chunk-type)` with no name (`print-all-chunk-types`,
    /// `framework/chunk-types.lisp:533`): print every chunk type. The
    /// `:include` parents and documentation string are not stored, so the
    /// `<- parents` / doc parts of `+pprint-chunk-type-string+` are omitted.
    pub fn print_chunk_types(&mut self) -> Vec<String> {
        let types = self.model.chunk_types.clone();
        let mut names = Vec::new();
        for ct in &types {
            self.out_lines.push(ct.name.clone());
            for s in &ct.slots {
                match ct.defaults.iter().find(|(k, _)| k.eq_ignore_ascii_case(s)) {
                    Some((_, p)) => self.out_lines.push(format!("   {s} ({})", pattern_show(p))),
                    None => self.out_lines.push(format!("   {s}")),
                }
            }
            self.out_lines.push(String::new());
            names.push(ct.name.clone());
        }
        names
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
        // A run-time chunk whose slot signature matches a model chunk joins
        // that model chunk's `dm-chunks` group (the alist is keyed by
        // signature), so it prints next to it rather than in the run-time
        // block — `tutorial-unit5modeling-v9-p7-8-run`'s `IMAGINAL-CHUNK0-0`
        // follows `OLD-CONTEXT`.
        let mut attached: Vec<Vec<usize>> = vec![Vec::new(); n_model];
        for i in n_model..self.dm.len() {
            let ki = key(&self.dm[i]);
            if let Some(j) = (0..n_model).find(|j| key(&self.dm[*j]) == ki) {
                attached[j].push(i);
                continue;
            }
            let found = groups.iter().position(|grp| key(&self.dm[grp[0]]) == ki);
            match found {
                Some(g) => groups[g].push(i),
                None => groups.push(vec![i]),
            }
        }
        let mut prints: Vec<usize> = Vec::new();
        for grp in groups.iter().rev() {
            prints.extend(grp.iter().copied());
        }
        for i in 0..n_model {
            prints.push(i);
            prints.extend(attached[i].iter().copied());
        }
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
        // The visicon chunk's `new-name "visicon-id"` counter is the number of
        // features created so far (including ones still queued or pending as
        // window items), which is what a later delete/modify references.
        let vid = self.features.len()
            + self
                .visicon_pending
                .iter()
                .filter(|u| matches!(u, VisPending::Add(_)))
                .count()
            + self.pending_added_features(None).len();
        self.visicon_alias
            .insert(format!("VISICON-ID{vid}"), format!("CHUNK{n}"));
        let mut x = 0i64;
        let mut y = 0i64;
        let mut w = 0i64;
        let mut ht = 0i64;
        let mut extra: Vec<(String, Value)> = Vec::new();
        // The rendered slot order is the argument order (`printed-visicon`
        // reverses the construction spec; GOTCHAS #238).
        let mut spec: Vec<(String, Value)> = Vec::new();
        for (k, v) in slots {
            let ku = k.to_uppercase();
            match (ku.as_str(), v) {
                ("SCREEN-X", Sexp::Int(i)) => {
                    x = *i;
                    spec.push((ku.clone(), Value::Int(*i)));
                }
                ("SCREEN-Y", Sexp::Int(i)) => {
                    y = *i;
                    spec.push((ku.clone(), Value::Int(*i)));
                }
                ("WIDTH", Sexp::Int(i)) => {
                    w = *i;
                    spec.push((ku.clone(), Value::Int(*i)));
                }
                ("HEIGHT", Sexp::Int(i)) => {
                    ht = *i;
                    spec.push((ku.clone(), Value::Int(*i)));
                }
                _ => {
                    if let Ok(val) = Value::from_sexp(v) {
                        spec.push((ku.clone(), val.clone()));
                        extra.push((ku, val));
                    }
                }
            }
        }
        self.visicon_pending.push(VisPending::Add(Feature {            name: format!("CHUNK{n}"),
            kind: "VISUAL-LOCATION".to_string(),
            value: String::new(),
            x,
            y,
            width: w,
            height: ht,
            extra,
            spec,
            spec_pre: Vec::new(),
            tstamp_ms: self.time_ms,
            new_p: true,
            finst_ms: None,
        }));
        self.now_display();
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
            ":SIM-HOOK" => {
                self.cfg.sim_hook = match val {
                    Sexp::Str(s) | Sexp::Sym(s) if s != "NIL" => Some(s.clone()),
                    _ => None,
                };
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
            // `:hear-self` (`speech.lisp:210`): `t`/`nil`/`speak`/`subvocalize`.
            ":HEAR-SELF" => {
                self.cfg.hear_self = match val {
                    Sexp::Sym(s) if s == "NIL" => HearSelf::None,
                    Sexp::Sym(s) if s == "SPEAK" => HearSelf::Speak,
                    Sexp::Sym(s) if s == "SUBVOCALIZE" => HearSelf::Subvocalize,
                    _ => HearSelf::All,
                }
            }
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
            // `:nsji` (`declarative-memory.lisp:2261`): `t`/`warn`/`nil`.
            ":NSJI" => {
                self.cfg.nsji = match val {
                    Sexp::Sym(s) if s == "NIL" => Nsji::Clamp,
                    Sexp::Sym(s) if s == "WARN" => Nsji::Warn,
                    _ => Nsji::Allow,
                }
            }
            // `:model-warnings` (`printing.lisp:341`): whether `model-warning`
            // output is shown at all.
            ":MODEL-WARNINGS" => self.cfg.model_warnings = truthy,
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
            // `:overstuff-aural-location` (`audio.lisp:1544`): whether a later
            // sound may overwrite the chunk in `AURAL-LOCATION`
            // (`stuff-sound-buffer`, `audio.lisp:716`).
            ":OVERSTUFF-AURAL-LOCATION" => self.cfg.overstuff_aural = truthy,
            // `:unstuff-aural-location` (`audio.lisp:1534`): nil off, t the
            // module's decay time after the sound's offset, a number a delay in
            // seconds (`safe-seconds->ms`).
            ":UNSTUFF-AURAL-LOCATION" => {
                self.cfg.unstuff_aural = match val {
                    Sexp::Sym(s) if s == "T" => Some(None),
                    Sexp::Int(i) => Some(Some(i * 1000)),
                    Sexp::Float(x) => Some(Some((x * 1000.0).round() as i64)),
                    _ => None,
                }
            }
            ":DECAY-TIME" => {
                if let Some(secs) = num(val) {
                    self.cfg.decay_time_ms = (secs * 1000.0).round() as i64;
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
        reward: Option<Reward>,
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
            // the reward lines print only with `:esc` + `:ul`. A number uses
            // `~6,3F`; any other value `~6@s` (right-aligned in 6 columns).
            match reward {
                Some(Reward::Num(r)) => self.out(format!(" :reward {r:6.3}")),
                Some(Reward::Other(s)) => self.out(format!(" :reward {s:>6}")),
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

    /// `buffer-slot-value` (`buffers.lisp:1030`): the value of `slot` on the
    /// chunk in `buffer`, encoded the way `encode-string`
    /// (`dispatcher.lisp:921`) does for the remote protocol — a string value
    /// is wrapped in single quotes (`"k"` -> `'k'`), which is what the Python
    /// tutorial tasks compare against.
    pub fn buffer_slot_value(&self, buffer: &str, slot: &str) -> Option<String> {
        let b = self.buffers.get(&buffer.to_uppercase())?;
        let c = b.chunk.as_ref()?;
        let v = c.get(&slot.to_uppercase());
        if v == Value::Nil {
            None
        } else {
            Some(match v {
                Value::Str(s) => format!("'{s}'"),
                other => other.show(),
            })
        }
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
    /// `(spp <production> :key …)` read (`utility-and-reward-1.lisp:660`): the
    /// value of one production parameter. `None` for an unknown production or
    /// key. `:at` is the global default action time, as ACT-R reports it.
    pub fn prod_param(&self, name: &str, key: &str) -> Option<Sexp> {
        let idx = self
            .model
            .productions
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(name))?;
        let prod = &self.model.productions[idx];
        Some(match key.to_uppercase().as_str() {
            ":NAME" => Sexp::Sym(prod.name.clone()),
            ":U" => Sexp::Float(self.utilities[idx] as f64),
            ":UTILITY" => match self.production_utility[idx] {
                Some(v) => Sexp::Float(v as f64),
                None => Sexp::Sym("NIL".into()),
            },
            ":AT" => Sexp::Float(self.cfg.dat_ms as f64 / 1000.0),
            ":REWARD" => match &prod.reward {
                Some(Reward::Num(r)) => Sexp::Float(*r as f64),
                Some(Reward::Other(s)) => Sexp::Str(s.clone()),
                None => Sexp::Sym("NIL".into()),
            },
            ":FIXED-UTILITY" => Sexp::Sym("NIL".into()),
            _ => return None,
        })
    }

    /// Every production's name, in load order — `(spp)` with no production
    /// reports them all (`utility-and-reward-1.lisp:804`).
    pub fn production_names(&self) -> Vec<String> {
        self.model.productions.iter().map(|p| p.name.clone()).collect()
    }

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
            ":SIM-HOOK" => self
                .cfg
                .sim_hook
                .clone()
                .unwrap_or_else(|| "NIL".into()),
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
    /// `pp` (`procedural-cmds.lisp`): print the text of the named productions,
    /// or all of them when `names` is empty.
    pub fn print_productions(&mut self, names: &[String]) -> Vec<String> {
        let exempt = self.cfg.do_not_query.clone();
        let selected: Vec<crate::model::Production> = if names.is_empty() {
            self.model.productions.clone()
        } else {
            names
                .iter()
                .filter_map(|n| {
                    let up = n.to_uppercase();
                    self.model
                        .productions
                        .iter()
                        .find(|p| p.name == up)
                        .cloned()
                })
                .collect()
        };
        let mut lines = Vec::new();
        for prod in &selected {
            let implicit = pp_implicit_queries(prod, &exempt);
            let text =
                super::compilation::render_production(prod, &Bindings::new(), &[], &implicit);
            for line in text.lines() {
                lines.push(line.to_string());
            }
        }
        self.out_lines.extend(lines);
        selected.iter().map(|p| p.name.clone()).collect()
    }

    pub fn print_whynot(&mut self, names: &[String]) -> Vec<String> {
        // `whynot-fct` (`procedural-cmds.lisp:597`) first recomputes the
        // conflict set (`pmatches-internal`), then reports each production.
        let conflict = self.pmatches();
        let ppm = self.cfg.ppm.is_some();
        // With no names it reports every production in the model
        // (`all-productions`), in definition order.
        let prods: Vec<crate::model::Production> = if names.is_empty() {
            self.model.productions.clone()
        } else {
            names
                .iter()
                .filter_map(|n| {
                    let up = n.to_uppercase();
                    self.model
                        .productions
                        .iter()
                        .find(|p| p.name == up)
                        .cloned()
                })
                .collect()
        };
        for prod in &prods {
            let hit = conflict.iter().find(|(i, _, _)| {
                self.model.productions[*i].name == prod.name
            });
            // `command-output "~%Production …"` prints a leading newline.
            self.out_lines.push(String::new());
            let (header, partials): (String, &[(String, String, Value, Value, f32)]) = match hit {
                // A ppm partial match renders the mismatched slots inline
                // (`NAME [requested, actual, similarity]`).
                Some((_, _, mism)) if ppm && !mism.is_empty() => (
                    format!(
                        "Production {} partially matches the current state:",
                        prod.name
                    ),
                    mism,
                ),
                Some(_) => (format!("Production {} matches:", prod.name), &[]),
                None => (format!("Production {} does NOT match.", prod.name), &[]),
            };
            self.out_lines.push(header);
            let b = hit.map(|(_, b, _)| b.clone()).unwrap_or_default();
            let text = super::compilation::render_production(prod, &b, partials, &[]);
            for line in text.lines() {
                self.out_lines.push(line.to_string());
            }
            if hit.is_none() {
                self.out_lines.push("It fails because: ".to_string());
                let reason = self.why_not_match(prod);
                self.out_lines.push(reason);
            }
        }
        conflict
            .iter()
            .map(|(i, _, _)| self.model.productions[*i].name.clone())
            .collect()
    }

    /// The first failing LHS condition's explanation
    /// (`failure-reason-string`, `procedural.lisp:1981`).
    ///
    /// ACT-R does not test the conditions in source order: the compiled
    /// lists are walked in category order (`test-production`,
    /// `procedural.lisp:2201`): the buffer-state masks, then *constants*
    /// (isa + literal slot tests), then *binds* (`slot =var` in a fixed
    /// slot, `production-binds`), then *others* (a variable slot name, an
    /// already-bound variable value, a negation). The reason is the first
    /// failure in that order. This is why `=goal> =slot state3` can report
    /// `=slot` resolved to `state`: the later `=imaginal> slot =slot` is a
    /// `bind-slot` and runs before the `test-var-slot`.
    fn why_not_match(&self, prod: &crate::model::Production) -> String {
        use crate::model::SlotOp;

        #[derive(Clone, Copy, PartialEq)]
        enum Cat {
            Constant,
            Bind,
            Other,
        }

        // Classify every test statically. A `slot =var` binding claims the
        // variable for the *first* clause that uses it (source order,
        // `unbound-vars`); later uses become tests.
        let mut bound: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut cats: Vec<Vec<(Cat, usize)>> = Vec::new();
        for item in &prod.lhs {
            let mut row = Vec::new();
            if let LhsItem::Match { tests, .. } = item {
                for (i, t) in tests.iter().enumerate() {
                    let cat = if t.slot == "ISA" {
                        Cat::Constant
                    } else if t.slot.starts_with('=') {
                        Cat::Other
                    } else if let (SlotOp::Eq, Pattern::Var(v)) = (&t.op, &t.value) {
                        if bound.insert(v.clone()) {
                            Cat::Bind
                        } else {
                            Cat::Other
                        }
                    } else {
                        Cat::Constant
                    };
                    row.push((cat, i));
                }
            }
            cats.push(row);
        }

        // `test-buffer-states` (`procedural.lisp:2201`) runs before any
        // condition: a required-but-missing buffer reports empty.
        for item in &prod.lhs {
            if let LhsItem::Match { buffer, .. } = item {
                if self
                    .buffers
                    .get(buffer)
                    .and_then(|x| x.chunk.as_ref())
                    .is_none()
                {
                    return format!("The {buffer} buffer is empty.");
                }
            }
        }

        let mut b = Bindings::new();
        for cat in [Cat::Constant, Cat::Bind, Cat::Other] {
            for (ci, item) in prod.lhs.iter().enumerate() {
                match item {
                    LhsItem::Match { buffer, tests } => {
                        let chunk = match self.buffers.get(buffer).and_then(|x| x.chunk.as_ref()) {
                            Some(c) => c,
                            None => continue,
                        };
                        for (c, i) in &cats[ci] {
                            if *c != cat {
                                continue;
                            }
                            let t = &tests[*i];
                            if t.slot == "ISA" {
                                continue;
                            }
                            // An `isa` `empty`-slot requirement (a `nil`
                            // value) is a constant condition.
                            if let Pattern::Nil = t.value {
                                if chunk.get(&t.slot) != Value::Nil {
                                    return format!(
                                        "The chunk in the {buffer} buffer has the slot {}.",
                                        t.slot
                                    );
                                }
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
                                // `test-var-slot` (`procedural.lisp:2034`): the
                                // slot *name* is a variable (`=slot state3`),
                                // so the reason names the resolved slot and the
                                // variable.
                                if t.slot.starts_with('=') {
                                    return format!(
                                        "The value in the {slot} slot (the value of the {} variable) of the chunk of the {buffer} buffer does not satisfy the constraints.",
                                        t.slot
                                    );
                                }
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
                        if cat != Cat::Constant {
                            continue;
                        }
                        for (attr, val) in queries {
                            let v = super::compilation::show_pat(val, &Bindings::new());
                            if !self.query_holds(buffer, attr, val) {
                                return format!(
                                    "The {attr} {v} query of the {buffer} buffer failed."
                                );
                            }
                        }
                    }
                    _ => {}
                }
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
        let name = self.resolve_visicon_name(name);
        self.visicon_pending.push(VisPending::Delete(name));
        self.now_display();
    }

    /// `add-visicon-features` names its chunks with `new-name "visicon-id"`
    /// (`vision.lisp:3856`), and a later `[delete|modify]-visicon-features`
    /// references *that* name, while the visicon table shows the location
    /// chunk's `CHUNK{n}`. Resolve the alias so both address the same feature.
    fn resolve_visicon_name(&self, name: &str) -> String {
        let key = name.trim_start_matches("#:").to_uppercase();
        self.visicon_alias
            .get(&key)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// `modify-visicon-features`: queue a slot update (`vision.lisp:4039`).
    fn queue_visicon_modify(&mut self, name: &str, mods: Vec<(String, Value)>) {
        let name = self.resolve_visicon_name(name);
        self.visicon_pending.push(VisPending::Update(name, mods));
        self.now_display();
    }

    /// `queue_visicon_modify` for callers outside the engine (the RPC layer),
    /// which hold raw sexps rather than `Value`s.
    pub fn queue_visicon_modify_sexp(&mut self, name: &str, mods: &[(String, Sexp)]) {
        let mods = mods
            .iter()
            .filter_map(|(k, s)| Value::from_sexp(s).ok().map(|v| (k.clone(), v)))
            .collect();
        self.queue_visicon_modify(name, mods);
    }

    pub fn clear_visicon(&mut self) {
        self.visicon_pending.push(VisPending::DeleteAll);
        self.now_display();
    }

    /// `print-visicon` (`vision.lisp:4090`) → `printed-visicon`
    /// (`vision.lisp:4098`): the visicon table, then the pending
    /// `visicon-updates`.
    pub fn print_visicon_rpc(&mut self, show_pending: bool, limit: Option<(i64, u32)>) -> Vec<String> {
        let mut out = self.printed_visicon(show_pending, limit);
        // `command-output` ends its string with `~%`, so the ref has a blank
        // line after each `print-visicon` block.
        out.push(String::new());
        self.out_lines.extend(out.clone());
        out
    }

    /// The `printed-visicon` body, without touching the output log (so the
    /// `printed-visicon` command can return it as a string).
    pub fn printed_visicon(
        &mut self,
        show_pending: bool,
        limit: Option<(i64, u32)>,
    ) -> Vec<String> {
        // The visicon stays ordered by location (`insert-into-visicon`,
        // `vision.lisp:1592`): `(screen-x, screen-y, distance, name)`.
        let mut feats = self.features.clone();
        feats.sort_by(|a, b| (a.x, a.y, a.name.clone()).cmp(&(b.x, b.y, b.name.clone())));
        let loc_slots = ["SCREEN-X", "SCREEN-Y", "DISTANCE"];
        // `(width slot count)` in push order; `stable-sort` by count
        // descending keeps reverse-first-seen order for ties
        // (`vision.lisp:4126`).
        let mut attributes: Vec<(usize, String, usize)> = Vec::new();
        let mut name_w = 4usize;
        let mut loc = [0usize; 3];
        for f in &feats {
            name_w = name_w.max(f.name.chars().count());
            for (slot, val) in f.filled_slots() {
                let w = val.show().chars().count();
                if let Some(i) = loc_slots.iter().position(|s| *s == slot) {
                    loc[i] = loc[i].max(w);
                } else if let Some(a) = attributes.iter_mut().find(|a| a.1 == slot) {
                    if w > a.0 {
                        a.0 = w;
                    }
                    a.2 += 1;
                } else {
                    attributes.push((w.max(slot.chars().count()), slot.clone(), 1));
                }
            }
        }
        attributes.reverse();
        attributes.sort_by(|a, b| b.2.cmp(&a.2));
        let loc_w = loc.iter().sum::<usize>() + 4;
        let mut out = Vec::new();
        // Header and separator (`~v,a` is left-aligned, `~v,,,'-a` pads with
        // dashes).
        let mut header = format!("{:<name_w$}  Att  {:<loc_w$}  ", "Name", "Loc");
        for (w, slot, _) in &attributes {
            header.push_str(&format!("{:<w$}  ", slot));
        }
        out.push(header);
        let mut sep = format!(
            "{:<name_w$}  ---  {:<loc_w$}  ",
            "-".repeat(name_w),
            "-".repeat(loc_w)
        );
        for (w, _, _) in &attributes {
            sep.push_str(&format!("{:<w$}  ", "-".repeat(*w)));
        }
        out.push(sep);

        for f in &feats {
            let filled = f.filled_slots();
            let slot = |n: &str| filled.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
            let att = self.feat_attended(f);
            let num = |n: &str| match slot(n) {
                Some(Value::Int(i)) => i.to_string(),
                Some(v) => v.show(),
                None => "NIL".to_string(),
            };
            let mut row = format!(
                "{:<name_w$}  {:<3}  ({:>w0$} {:>w1$} {:>w2$})  ",
                f.name,
                att,
                num("SCREEN-X"),
                num("SCREEN-Y"),
                num("DISTANCE"),
                w0 = loc[0],
                w1 = loc[1],
                w2 = loc[2],
            );
            for (w, name, _) in &attributes {
                match slot(name) {
                    // `(if v (format "~v,s" v) (format "~v,a" ""))`: a bound
                    // slot prints as `~S`, an unbound one as blanks.
                    Some(v) if !matches!(v, Value::Nil) => {
                        row.push_str(&format!("{:<w$}  ", v.show()));
                    }
                    _ => row.push_str(&format!("{:<w$}  ", "")),
                }
            }
            out.push(row);
        }
        if show_pending {
            let mut adds: Vec<String> = Vec::new();
            let mut deletes: Vec<String> = Vec::new();
            let mut updates: Vec<String> = Vec::new();
            let pending_adds: Vec<Feature> = self.pending_added_features(limit);
            let add_entry = |f: &Feature| {
                let mut parts = vec![f.name.clone()];
                for (k, v) in f.filled_slots() {
                    parts.push(k);
                    parts.push(v.show());
                }
                format!("({})", parts.join(" "))
            };
            for f in &pending_adds {
                adds.push(add_entry(f));
            }
            for u in &self.visicon_pending {
                match u {
                    // `(format "~s" (cons name (slots…)))`: the loc name then
                    // the slot/value pairs in printed order.
                    VisPending::Add(f) => adds.push(add_entry(f)),
                    VisPending::Delete(n) => deletes.push(n.clone()),
                    VisPending::Update(n, mods) => {
                        let mut parts = vec![n.clone()];
                        for (k, v) in mods {
                            parts.push(k.clone());
                            parts.push(v.show());
                        }
                        updates.push(format!("({})", parts.join(" ")));
                    }
                    VisPending::DeleteAll => {}
                }
            }
            for (label, items) in [
                ("Pending additions", &adds),
                ("Pending deletions", &deletes),
                ("Pending modifications", &updates),
            ] {
                if !items.is_empty() {
                    out.push(String::new());
                    out.push(format!("{label}:"));
                    out.extend(items.iter().cloned());
                }
            }
        }
        out
    }

    /// `feat-attended` (`vision.lisp:2269`): a finst is `T`, the in-flight
    /// attention target is `REQ`, a new feature is `NEW`, else `NIL`.
    fn feat_attended(&self, f: &Feature) -> String {
        if f.finst_ms.is_some() {
            return "T".to_string();
        }
        if self.visual_busy {
            if let Some(name) = &self.last_attention {
                // `chunk-visicon-entry`: a location *copy* (`VISUAL-LOCATION0-1`)
                // stands for the feature it was copied from. `vl_copied_from`
                // holds the base *name*, which survives a removal shifting the
                // feature vector (`vl_copies`' index would not).
                let base = self.vl_copied_from.get(name).cloned();
                if base.as_deref() == Some(f.name.as_str()) {
                    return "REQ".to_string();
                }
            }
        }
        if f.new_p {
            "NEW".to_string()
        } else {
            "NIL".to_string()
        }
    }

    /// The features a queued `visicon-updates` `:add` would create, in the
    /// order `process-display` would apply them (`vision.lisp:1634`). Window
    /// items due but not yet proc-displayed are pending adds too: the device
    /// queues its visicon features and `install-device` schedules the wave.
    fn pending_added_features(&self, limit: Option<(i64, u32)>) -> Vec<Feature> {
        // A `print-visicon` the task calls at instant M runs *before* the
        // ops it calls at M but later (`seq`), so those are not pending yet.
        let before = |ms: i64, seq: u32| match limit {
            Some((lms, lseq)) => (ms, seq) < (lms, lseq),
            None => ms <= self.time_ms,
        };
        let mut due: Vec<&DeviceText> = self
            .pending_texts
            .iter()
            .filter(|t| before(t.at_ms, t.seq))
            .collect();
        due.sort_by_key(|t| (t.x, t.y));
        let mut seq = self.feat_seq;
        let mut out = Vec::new();
        for t in due {
            let width = 7 * t.text.chars().count() as i64;
            let sx = t.x + 305 + (width - 7) / 2;
            let sy = t.y + 306;
            out.push(self.text_feature(
                format!("VISUAL-LOCATION{seq}"),
                &t.text,
                sx,
                sy,
                self.time_ms,
            ));
            seq += 1;
        }
        out
    }

    /// `new-tone-sound <freq> <duration> [onset] [time-in-ms]`: schedule a tone.
    pub fn new_tone_sound_rpc(&mut self, freq: i64, duration: i64, onset_ms: i64) {
        // ACT-R stores the *absolute* onset (`new-tone-sound` uses `mp-time-ms`
        // when none is given, `audio.lisp:787`), and the module derives both
        // `detect-at-time` (`onset + tone-detect-delay`) and the sound's
        // `offset` from it. Storing the argument instead made every tone a
        // task sounds between runs land its detection at `0 + delay`.
        let at = self.time_ms().max(onset_ms);
        let idx = self.add_sound(freq, duration, at);
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
/// Set a line feature's geometry from its window-relative endpoints.
/// `build-vis-locs-for` for a `v-liner` (`uwi.lisp:511`): the spec order is
/// `color value screen-x screen-y width height end1-x end1-y end2-x end2-y`,
/// and the isa marker `(visual-location line)` adds `kind` and `line`
/// (GOTCHAS #238). Shared by `add-line-to-exp-window` and
/// `modify-line-for-exp-window` (which keeps the item, so the feature's name
/// and finst are untouched).
pub(super) fn set_line_geometry(
    f: &mut Feature,
    start: (i64, i64),
    end: (i64, i64),
    origin: (i64, i64),
    color: &str,
) {
    let (ox, oy) = origin;
    let (x1, y1) = (ox + start.0, oy + start.1);
    let (x2, y2) = (ox + end.0, oy + end.1);
    let mx = ox + (start.0 + end.0).div_euclid(2);
    let my = oy + (start.1 + end.1).div_euclid(2);
    let orient = pts_to_orientation((x1, y1), (x2, y2));
    let width = (start.0 - end.0).abs();
    let height = (start.1 - end.1).abs();
    let c = Value::Sym(color.to_uppercase());
    f.value = orient.to_string();
    f.x = mx;
    f.y = my;
    f.width = width;
    f.height = height;
    f.extra = vec![
        ("COLOR".to_string(), c.clone()),
        ("END1-X".to_string(), Value::Int(x1)),
        ("END1-Y".to_string(), Value::Int(y1)),
        ("END2-X".to_string(), Value::Int(x2)),
        ("END2-Y".to_string(), Value::Int(y2)),
    ];
    f.spec = vec![
        ("COLOR".to_string(), c),
        ("VALUE".to_string(), Value::Int(orient)),
        ("SCREEN-X".to_string(), Value::Int(mx)),
        ("SCREEN-Y".to_string(), Value::Int(my)),
        ("WIDTH".to_string(), Value::Int(width)),
        ("HEIGHT".to_string(), Value::Int(height)),
        ("END1-X".to_string(), Value::Int(x1)),
        ("END1-Y".to_string(), Value::Int(y1)),
        ("END2-X".to_string(), Value::Int(x2)),
        ("END2-Y".to_string(), Value::Int(y2)),
        ("KIND".to_string(), Value::Sym("LINE".to_string())),
        ("LINE".to_string(), Value::Sym("T".to_string())),
    ];
}

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

    /// Queue a window display wave, **coalescing requests at the same time**.
    ///
    /// A window op is only a visicon change, and ACT-R recomputes the visicon
    /// once per cycle: `bst`'s `build-display` calls `add-button`/
    /// `add-line` nine times before the run and the ref shows **one**
    /// `visicon-update`, not nine. The `add-visicon-features` API and the
    /// scripted driver keep their own per-call push — the device-timeline
    /// specs depend on it — so this is deliberately the window helpers only.
    pub(super) fn now_display(&mut self) {
        if self.display_queued_ms == Some(self.time_ms) {
            return;
        }
        self.display_queued_ms = Some(self.time_ms);
        self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
    }

    /// `clear-exp-window`: drop all live window items and visicon features.
    pub fn clear_window(&mut self) {
        self.window_items.clear();
        self.window_actions.clear();
        // `process-display`'s `:delete-all` collects attended features into
        // `transfer-finsts` (`vision.lisp:1656`), so a rebuilt display can
        // re-attach their finst (`transfer-finst`, `vision.lisp:2306`).
        self.collect_finst_transfers();
        self.features.clear();
        // `update-attended-loc` (`vision.lisp:1870`): the attended object is
        // gone, so the visual module re-encodes it and stays busy for
        // `:visual-attention-latency`. The device timeline reaches that from
        // `proc_display`, which only sees a clear through `pending_clears`
        // (the scripted `CLEAR` ops). A client `clear-exp-window` empties the
        // visicon here, and the `proc_display` that `now_display` queues below
        // then sees the attended location's feature gone and schedules the
        // re-encode itself — scheduling it here too would emit the
        // `encoding-complete`/`no visual-object found` pair twice
        // (`bst-learn`'s per-trial window rebuild).
        self.now_display();
    }

    /// Collect the live features that are currently attended, so a rebuilt
    /// display can move their finst onto the matching new feature
    /// (`transfer-finsts`, `vision.lisp:1656`).
    pub(super) fn collect_finst_transfers(&mut self) {
        for f in &self.features {
            if let Some(t) = f.finst_ms {
                self.pending_finst_transfers.push(FinstTransfer {
                    kind: f.kind.clone(),
                    x: f.x,
                    y: f.y,
                    width: f.width,
                    height: f.height,
                    value: f.value.clone(),
                    finst_ms: t,
                });
            }
        }
    }

    /// Apply the collected finst transfers to the rebuilt display
    /// (`transfer-finst`, `vision.lisp:2306`): the matching new feature keeps
    /// the finst, is stamped with the finst's time, and is left not-`new`.
    /// The list is per-display, so it is drained here.
    pub(super) fn apply_finst_transfers(&mut self) {
        if self.pending_finst_transfers.is_empty() {
            return;
        }
        let transfers = std::mem::take(&mut self.pending_finst_transfers);
        for f in self.features.iter_mut() {
            if let Some(t) = transfers.iter().find(|t| t.matches(f)) {
                f.finst_ms = Some(t.finst_ms);
                f.new_p = false;
                f.tstamp_ms = t.finst_ms;
            }
        }
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
        // `static-text-vdi` (`uwi.lisp:459`): `start-x = (1+ (x-pos self))`,
        // `y-pos = y-pos + round(line-height, 2)`, and no `x-fct`, so
        // `build-string-feats` starts each line at `start-x`.
        let y_screen = y + oy + round_half_even(line_h, 2);
        self.push_text_item(&key, text, x + 1 + ox, y_screen, char_w, text_h);
        self.now_display();
        key
    }

    /// Push a text item's word features (`build-string-feats`,
    /// `vision.lisp:3558`). Each word's `screen-x` is
    /// `start + round(word-width, 2)` (`vision.lisp:3586`), `screen-y` is
    /// fixed for the item, and a space run advances the cursor. `start` is
    /// already in screen coordinates (the caller's `x-fct`/`x-off` folded in).
    fn push_text_item(
        &mut self,
        key: &str,
        text: &str,
        start_screen_x: i64,
        screen_y: i64,
        char_w: i64,
        text_h: i64,
    ) {
        let mut names = Vec::new();
        let mut curr_x = start_screen_x;
        for tok in chop_string(text) {
            match tok {
                Chop::Word(w) => {
                    let wd = char_w * w.chars().count() as i64;
                    let name = format!("VISUAL-LOCATION{}", self.feat_seq);
                    self.feat_seq += 1;
                    let sx = curr_x + round_half_even(wd, 2);
                    self.features.push(Feature {
                        name: name.clone(),
                        kind: "TEXT".to_string(),
                        value: w.clone(),
                        x: sx,
                        y: screen_y,
                        width: wd,
                        height: text_h,
                        extra: vec![("TEXT".to_string(), Value::Sym("T".to_string()))],
                        // `build-string-feats` (`vision.lisp:3558`) builds
                        // `(isa (visual-location text) <y-slot> … height …
                        // value … width … <x-slot> …)`, and the device appends
                        // `color`, `kind` and `text` (GOTCHAS #238)."
                        spec: vec![
                            ("SCREEN-Y".to_string(), Value::Int(screen_y)),
                            ("HEIGHT".to_string(), Value::Int(text_h)),
                            ("VALUE".to_string(), Value::Str(w.clone())),
                            ("WIDTH".to_string(), Value::Int(wd)),
                            ("SCREEN-X".to_string(), Value::Int(sx)),
                            ("COLOR".to_string(), Value::Sym("BLACK".to_string())),
                            ("KIND".to_string(), Value::Sym("TEXT".to_string())),
                            ("TEXT".to_string(), Value::Sym("T".to_string())),
                        ],
                        spec_pre: Vec::new(),
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
        self.window_items.push((key.to_string(), names));
    }

    /// `add-line-to-exp-window`: a `line` feature (`uwi.lisp:511`).
    pub fn add_window_line(&mut self, start: (i64, i64), end: (i64, i64), color: &str) -> String {
        let key = format!("LINE-VDI-{}", self.window_item_seq);
        self.window_item_seq += 1;
        let name = format!("VISUAL-LOCATION{}", self.feat_seq);
        self.feat_seq += 1;
        let mut f = Feature {
            name: name.clone(),
            kind: "LINE".to_string(),
            value: String::new(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            extra: Vec::new(),
            spec: Vec::new(),
            spec_pre: Vec::new(),
            tstamp_ms: self.time_ms,
            new_p: true,
            finst_ms: None,
        };
        set_line_geometry(&mut f, start, end, self.window_origin, color);
        self.features.push(f);
        self.window_items.push((key.clone(), vec![name]));
        self.now_display();
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
        action: Option<(String, Vec<Json>)>,
    ) -> String {
        let key = format!("BUTTON-VDI-{}", self.window_item_seq);
        self.window_item_seq += 1;
        let (ox, oy) = self.window_origin;
        let name = format!("VISUAL-LOCATION{}", self.feat_seq);
        self.feat_seq += 1;
        // `add-button-to-exp-window`'s visicon feature (`uwi.lisp:401`):
        // `(isa (visual-location oval) … value (oval <text>) …)` — the button is
        // an **oval** whose value is its text, which is how the tutorial models
        // ask for it (`+visual-location> kind oval, value "b"`). It was built
        // as `kind BUTTON` with the window-item key as its value, so no such
        // request could ever match (GOTCHAS #229).
        self.features.push(Feature {
            name: name.clone(),
            kind: "OVAL".to_string(),
            value: text.to_string(),
            x: ox + x + width.div_euclid(2),
            y: oy + y + height.div_euclid(2),
            width,
            height,
            extra: vec![("COLOR".to_string(), Value::Sym(color.to_uppercase()))],
            // `build-vis-locs-for` for a `button-vdi` (`uwi.lisp:358`): the
            // oval's spec order is `screen-x screen-y width height value
            // color`, and the isa marker adds `kind` and `oval` (GOTCHAS #238)."
            spec: vec![
                ("SCREEN-X".to_string(), Value::Int(ox + x + width.div_euclid(2))),
                ("SCREEN-Y".to_string(), Value::Int(oy + y + height.div_euclid(2))),
                ("WIDTH".to_string(), Value::Int(width)),
                ("HEIGHT".to_string(), Value::Int(height)),
                ("VALUE".to_string(), Value::Str(text.to_string())),
                ("COLOR".to_string(), Value::Sym(color.to_uppercase())),
                ("KIND".to_string(), Value::Sym("OVAL".to_string())),
                ("OVAL".to_string(), Value::Sym("T".to_string())),
            ],
            spec_pre: Vec::new(),
            tstamp_ms: self.time_ms,
            new_p: true,
            finst_ms: None,
        });
        let mut names = vec![name];
        // The button's text is a separate `text` feature at the **same**
        // screen position as the oval. `button-vdi` (`uwi.lisp:366`) builds it
        // with `start-x = px(view-loc)`, `y-pos = py(view-loc)` and
        // `x-fct = (lambda (s startx obj) (- startx (round (str-width obj) 2)))`,
        // so `build-string-feats`' `(+ curr-x x-off (round curr-width 2))`
        // collapses to `px(top-xy) + px(view-loc)` — exactly the oval's own
        // `screen-x`/`screen-y`. (A label one pixel off lands in a column of
        // its own and changes `newest_feature`'s pick: bst-learn's ref places
        // VISUAL-LOCATION19, a label, at a trial start.)
        let char_w = round_half_even_f(12.0 * 7.0 / 12.0).max(1);
        let text_h = round_half_even_f(12.0 * 10.0 / 12.0).max(1);
        let total_w = char_w * text.chars().count() as i64;
        let oval_x = ox + x + width.div_euclid(2);
        let oval_y = oy + y + height.div_euclid(2);
        let tkey = format!("TEXT-VDI-{}", self.window_item_seq);
        self.window_item_seq += 1;
        self.push_text_item(
            &tkey,
            text,
            oval_x - round_half_even(total_w, 2),
            oval_y,
            char_w,
            text_h,
        );
        if let Some((_, tnames)) = self.window_items.iter().find(|(k, _)| k == &tkey) {
            names.extend(tnames.iter().cloned());
        }
        self.window_items.push((key.clone(), names));
        if let Some(action) = action {
            self.window_actions.insert(key.clone(), action);
        }
        self.now_display();
        key
    }

    /// Queue the `:action` of the window button under the cursor, if any
    /// (`uwi.lisp`'s button press). ACT-R tests the buttons topmost-first, and
    /// `window_items` is in add order, so scan in reverse. The click lands on
    /// the button's oval (`build-vis-locs-for`'s `oval` feature).
    pub(super) fn record_click_action(&mut self) {
        let (mx, my, _) = self.zz_mouse_pos;
        let hit = self
            .window_items
            .iter()
            .rev()
            .find_map(|(key, names)| {
                if !self.window_actions.contains_key(key) {
                    return None;
                }
                let under = names.iter().any(|n| {
                    self.features.iter().any(|f| {
                        &f.name == n
                            && f.kind == "OVAL"
                            && (mx - f.x).abs() <= f.width / 2
                            && (my - f.y).abs() <= f.height / 2
                    })
                });
                under.then(|| key.clone())
            });
        if let Some(key) = hit {
            if let Some((cmd, args)) = self.window_actions.get(&key).cloned() {
                self.pending_device_actions.push((cmd, args));
            }
        }
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
            self.now_display();
        }
    }

    /// `modify-line-for-exp-window` (`uwi.lisp:490`): redraw the line item.
    /// ACT-R mutates the `v-liner` in place and `internal-modify-visicon-
    /// features` queues an `:update`, so the feature keeps its identity (and
    /// finst); `color` is only changed when supplied (`colorp`).
    pub fn modify_window_line(
        &mut self,
        key: &str,
        start: (i64, i64),
        end: (i64, i64),
        color: Option<&str>,
    ) {
        let Some((_, names)) = self.window_items.iter().find(|(k, _)| k == key) else {
            return;
        };
        let names = names.clone();
        let origin = self.window_origin;
        for f in self.features.iter_mut().filter(|f| names.contains(&f.name)) {
            let color = color
                .map(str::to_string)
                .or_else(|| {
                    f.extra
                        .iter()
                        .find(|(k, _)| k == "COLOR")
                        .and_then(|(_, v)| match v {
                            Value::Sym(s) => Some(s.clone()),
                            _ => None,
                        })
                })
                .unwrap_or_else(|| "BLACK".to_string());
            set_line_geometry(f, start, end, origin, &color);
            f.tstamp_ms = self.time_ms;
        }
        self.now_display();
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
        self.now_display();
    }
}

/// Introspection for the Environment viewers (`environment/environment-cmds.lisp`
/// plus the `printed-*`/`all-productions`/`buffers`/`chunks` families). These
/// return data rather than pushing to the output stream.
impl Engine {
    /// `act-r-version`: the architecture version string.
    pub fn act_r_version(&self) -> String {
        "7.0".to_string()
    }

    /// `all-productions`: every production name, in definition order.
    pub fn all_production_names(&self) -> Vec<String> {
        self.model
            .productions
            .iter()
            .map(|p| p.name.clone())
            .collect()
    }

    /// `buffers` / `model-buffers`: every model buffer name, sorted.
    pub fn buffer_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.buffers.keys().cloned().collect();
        v.sort();
        v
    }

    /// `chunks`: the declarative-memory chunk names.
    pub fn dm_chunk_names(&self) -> Vec<String> {
        self.dm.iter().filter_map(|c| c.name.clone()).collect()
    }

    /// `chunk-filled-slots-list`: the filled (non-`ISA`, non-nil) slot names.
    pub fn chunk_filled_slots(&self, name: &str) -> Vec<String> {
        match self.find_chunk(&name.to_uppercase()) {
            Some(c) => c
                .slots
                .iter()
                .filter(|(k, v)| !k.eq_ignore_ascii_case("ISA") && *v != Value::Nil)
                .map(|(k, _)| k.clone())
                .collect(),
            None => Vec::new(),
        }
    }

    /// `printed-chunk`: the `pprint-a-chunk` text of a chunk (name + slots).
    pub fn printed_chunk(&self, name: &str) -> Option<String> {
        self.find_chunk(&name.to_uppercase())
            .map(|c| self.chunk_text(c))
    }

    /// `pprint-a-chunk` (`chunks.lisp:674`) as a string, mirroring
    /// [`Self::push_chunk`] without emitting: name, the filled slots sorted by
    /// the model-wide slot index, and a trailing blank line.
    fn chunk_text(&self, c: &ChunkInst) -> String {
        let mut out = String::new();
        out.push_str(c.name.as_deref().unwrap_or(""));
        out.push('\n');
        let slot_index = self.model.slot_index();
        let mut idx: Vec<usize> = (0..c.slots.len()).collect();
        idx.sort_by_key(|&i| {
            *slot_index
                .get(&c.slots[i].0.to_uppercase())
                .unwrap_or(&usize::MAX)
        });
        for i in idx {
            let (k, v) = &c.slots[i];
            if k.eq_ignore_ascii_case("ISA") || *v == Value::Nil {
                continue;
            }
            out.push_str(&format!("   {k}  {}\n", v.show()));
        }
        out.push('\n');
        out
    }

    /// `dm-slot-filters`: the distinct filled-slot lists among DM chunks (the
    /// Environment's filter dropdown), each rendered as a space-joined string
    /// with the `_none_` sentinel first.
    pub fn dm_slot_filters(&self) -> Vec<String> {
        let mut lists: Vec<String> = Vec::new();
        for c in &self.dm {
            let mut names: Vec<String> = c
                .slots
                .iter()
                .filter(|(k, v)| !k.eq_ignore_ascii_case("ISA") && *v != Value::Nil)
                .map(|(k, _)| k.clone())
                .collect();
            names.sort();
            names.dedup();
            if !names.is_empty() {
                let joined = names.join(" ");
                if !lists.contains(&joined) {
                    lists.push(joined);
                }
            }
        }
        lists.sort();
        lists.insert(0, "_none_".to_string());
        lists
    }

    /// `filter-dm-chunks`: DM chunks that have every slot in `filter` filled.
    /// An empty filter (or the `_none_` sentinel) returns every DM chunk.
    pub fn filter_dm_chunks(&self, filter: &[String]) -> Vec<String> {
        let want: Vec<String> = filter
            .iter()
            .map(|s| s.to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
        self.dm
            .iter()
            .filter_map(|c| {
                let has = |slot: &String| {
                    c.slots
                        .iter()
                        .any(|(k, v)| k.eq_ignore_ascii_case(slot) && *v != Value::Nil)
                };
                if want.iter().all(has) {
                    c.name.clone()
                } else {
                    None
                }
            })
            .collect()
    }

    /// `used-production-buffers`: the buffers any production reads or writes.
    pub fn used_production_buffers(&self) -> Vec<String> {
        let mut set: Vec<String> = Vec::new();
        let mut add = |b: &String| {
            if !set.contains(b) {
                set.push(b.clone());
            }
        };
        for p in &self.model.productions {
            for item in &p.lhs {
                match item {
                    LhsItem::Match { buffer, .. } | LhsItem::Query { buffer, .. } => add(buffer),
                    _ => {}
                }
            }
            for a in &p.rhs {
                match a {
                    RhsAction::Modify { buffer, .. }
                    | RhsAction::ModifyRequest { buffer, .. }
                    | RhsAction::Request { buffer, .. }
                    | RhsAction::Clear { buffer }
                    | RhsAction::DirectSet { buffer, .. }
                    | RhsAction::DirectErase { buffer } => add(buffer),
                    _ => {}
                }
            }
        }
        set.sort();
        set
    }

    /// `printed-production` / `pp`: the `(P …)` text of a production.
    pub fn printed_production(&self, name: &str) -> Option<String> {
        let up = name.to_uppercase();
        let prod = self.model.productions.iter().find(|p| p.name == up)?;
        let exempt = self.cfg.do_not_query.clone();
        Some(super::compilation::render_production(
            prod,
            &Bindings::new(),
            &[],
            &pp_implicit_queries(prod, &exempt),
        ))
    }

    /// The output text pushed since `mark` (ACT-R's `capture-command-output`).
    pub fn out_text_since(&self, mark: usize) -> String {
        self.out_lines[mark.min(self.out_lines.len())..].join("\n")
    }

    /// `printed-buffer-chunk`: the buffer dump as a string (suppressed from
    /// the trace), for the Environment buffer viewer.
    pub fn printed_buffer_chunk(&mut self, name: &str) -> String {
        let mark = self.out_lines.len();
        self.print_buffer_chunk(&[name.to_string()]);
        let text = self.out_lines[mark..].join("\n");
        self.out_lines.truncate(mark);
        text
    }

    /// `printed-buffer-status`: the buffer status block as a string.
    pub fn printed_buffer_status(&mut self, name: &str) -> String {
        let mark = self.out_lines.len();
        self.print_buffer_status(&[name.to_string()]);
        let text = self.out_lines[mark..].join("\n");
        self.out_lines.truncate(mark);
        text
    }
}

/// A chunk-type default value as Lisp `~S` would print it
/// (`(chunk-type … (slot default))`).
fn pattern_show(p: &crate::model::Pattern) -> String {
    use crate::model::Pattern;
    match p {
        Pattern::Nil => "NIL".to_string(),
        Pattern::Var(v) => format!("={v}"),
        Pattern::Sym(s) => s.clone(),
        Pattern::Int(i) => i.to_string(),
        Pattern::Num(x) => x.to_string(),
        Pattern::Str(s) => format!("\"{s}\""),
    }
}

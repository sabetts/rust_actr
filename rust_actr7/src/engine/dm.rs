//! Declarative memory, finsts, and blending.
use super::compilation::{failure_wait, latency};
use super::eval::{f32_of, is_number, set_result, show_values};
use super::*;
impl Engine {
    /// Filled-slot names of a chunk (`chunk-slots-vector` mask, as names):
    /// non-nil slots only; `isa` is the chunk type, not a slot.
    pub(super) fn filled_slots(&self, c: &ChunkInst) -> Vec<String> {
        c.slots
            .iter()
            .filter(|(_, v)| *v != Value::Nil)
            .map(|(s, _)| s.clone())
            .collect()
    }

    pub(super) fn find_chunk(&self, name: &str) -> Option<&ChunkInst> {
        self.dm
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .or_else(|| self.named.get(name))
    }

    /// The base-level-inhibition activation offset, when the extra is
    /// enabled (`bl-inhibition-term`).
    pub(super) fn inhibition_offset(&self, refs: &[i64]) -> f32 {
        if self.cfg.inhibition {
            inhibition_term(
                self.time_ms,
                refs,
                self.cfg.inhibition_scale,
                self.cfg.inhibition_decay,
            )
        } else {
            0.0
        }
    }

    /// DM chunk order as `all-dm-chunks` (`declarative-memory.lisp:2747`):
    /// `dm-chunks` is an alist of signature -> chunks, built by pushing new
    /// signature groups and new chunks to the front, then flattened. So
    /// newest signature group first, newest chunk first within a group.
    pub(super) fn dm_alist_order(&self) -> Vec<usize> {
        let mut groups: Vec<(Vec<String>, Vec<usize>)> = Vec::new();
        for (i, c) in self.dm.iter().enumerate() {
            let mut sig = self.filled_slots(c);
            sig.sort();
            if let Some(g) = groups.iter_mut().find(|(s, _)| *s == sig) {
                g.1.insert(0, i);
            } else {
                groups.insert(0, (sig, vec![i]));
            }
        }
        groups.into_iter().flat_map(|(_, v)| v).collect()
    }

    /// Global slot index order from chunk-type declarations (first
    /// appearance wins), mirroring `slot-name->index`.
    pub(super) fn slot_index_map(&self) -> HashMap<String, usize> {
        let mut m: HashMap<String, usize> = HashMap::new();
        let mut n = 0usize;
        for ct in &self.model.chunk_types {
            for s in &ct.slots {
                if !m.contains_key(s) {
                    m.insert(s.clone(), n);
                    n += 1;
                }
            }
        }
        m
    }

    /// `slot-mask->names`: the given names in slot-index order.
    pub(super) fn names_by_index(
        names: &HashSet<String>,
        idx: &HashMap<String, usize>,
    ) -> Vec<String> {
        let mut v: Vec<String> = names.iter().cloned().collect();
        v.sort_by_key(|s| idx.get(s).copied().unwrap_or(usize::MAX));
        v
    }

    /// Numeric `> < >= <=` against runtime values.
    pub(super) fn rel_test_v(&self, op: SlotOp, rhs: &Value, got: &Value) -> bool {
        let rhs = match rhs {
            Value::Int(i) => *i as f64,
            Value::Num(x) => *x,
            _ => return false,
        };
        let lhs = match got {
            Value::Int(i) => *i as f64,
            Value::Num(x) => *x,
            _ => return false,
        };
        match op {
            SlotOp::Gt => lhs > rhs,
            SlotOp::Lt => lhs < rhs,
            SlotOp::Ge => lhs >= rhs,
            SlotOp::Le => lhs <= rhs,
            _ => false,
        }
    }

    /// Full `find-matching-chunks` test of a chunk against a request spec
    /// (`=`, `-` and the relative slots). Nil values are signature-only.
    pub(super) fn chunk_matches_req(
        &self,
        c: &ChunkInst,
        slots: &[(String, Value)],
        neg: &[(String, Value)],
        rels: &[(String, Value, SlotOp)],
    ) -> bool {
        for (s, v) in slots {
            if s.starts_with(':') || *v == Value::Nil {
                continue;
            }
            if !self.value_eq(&c.get(s), v) {
                return false;
            }
        }
        for (s, v) in neg {
            if s.starts_with(':') {
                continue;
            }
            if *v == Value::Nil {
                if c.get(s) == Value::Nil {
                    return false;
                }
            } else if self.value_eq(&c.get(s), v) {
                return false;
            }
        }
        for (s, v, op) in rels {
            if s.starts_with(':') {
                continue;
            }
            if !self.rel_test_v(*op, v, &c.get(s)) {
                return false;
            }
        }
        true
    }

    /// `compute-activation` for a blending candidate: base + spreading +
    /// partial matching + one logistic noise draw. `:min-bl` suppression is
    /// applied by the caller (but the draw still happens, as in Lisp).
    pub(super) fn compute_blending_activation(
        &mut self,
        idx: usize,
        want_slots: &[(String, Value)],
        want_neg: &[(String, Value)],
    ) -> (f32, f32) {
        let base = self.base_level(idx, self.cfg.bll, false);
        let name = self.dm[idx].name.clone().unwrap_or_default();
        let spread = if self.dm[idx].name.is_some() {
            self.spreading_activation(&name, self.cfg.mas)
        } else {
            0.0
        };
        let mut a = base + spread + self.inhibition_offset(&self.hist[idx].refs);
        if let Some(mp) = self.cfg.mp {
            for (s, v) in want_slots {
                if s.starts_with(':') || *v == Value::Nil {
                    continue;
                }
                a += mp * self.similarity(v, &self.dm[idx].get(s));
            }
            for (s, v) in want_neg {
                if s.starts_with(':') || *v == Value::Nil {
                    continue;
                }
                if self.similarity(v, &self.dm[idx].get(s)) == self.cfg.ms {
                    a += mp * self.cfg.md;
                }
            }
        }
        let noise = if self.cfg.uan {
            0.0
        } else {
            match self.cfg.ans {
                Some(s) => self.rng.borrow_mut().noise(s),
                None => 0.0,
            }
        };
        (a + noise, base)
    }

    /// `compute-spacing-effect-activation`
    /// (`extras/spacing-effect/spacing-effect.lisp`): base level from the
    /// chunk's per-reference decays. The cached list is used when it still
    /// matches `refs`; otherwise it is recomputed with the module's fallback
    /// (oldest-to-newest accumulation, first decay = intercept).
    pub(super) fn spacing_activation(&mut self, idx: usize) -> f32 {
        let refs = self.hist[idx].refs.clone();
        if refs.is_empty() {
            return -999999.0;
        }
        let cached = self.hist[idx].decays.clone();
        let decays = match cached {
            Some(d)
                if d.len() == refs.len() && d.iter().map(|(t, _)| *t).eq(refs.iter().copied()) =>
            {
                d
            }
            _ => self.recompute_decays(idx, &refs),
        };
        let now = self.time_ms;
        let mut value = 0.0f32;
        for (r, (_, decay)) in refs.iter().zip(decays.iter()) {
            let t = ((now - r) as f32 / 1000.0).max(0.05);
            // `expt-coerced` on a single base: SBCL computes in double and
            // rounds (see `bl_equation`'s `:ol t` branch).
            value += ((-*decay as f64) * (t as f64).ln()).exp() as f32;
        }
        log_coerced(value)
    }

    /// The spacing-effect recompute branch: walk the references oldest to
    /// newest, deriving each decay from the activation just before that
    /// reference. Stores and returns the newest-first `(ref, decay)` list.
    fn recompute_decays(&mut self, idx: usize, refs: &[i64]) -> Vec<(i64, f32)> {
        let n = refs.len();
        let mut acc_refs: Vec<i64> = vec![refs[n - 1]];
        let mut acc_decays: Vec<f32> = vec![self.cfg.se_intercept];
        for k in (0..n - 1).rev() {
            let reference = refs[k];
            let mut value = 0.0f32;
            for (r, decay) in acc_refs.iter().zip(acc_decays.iter()) {
                let t = ((reference - r) as f32 / 1000.0).max(0.05);
                value += ((-*decay as f64) * (t as f64).ln()).exp() as f32;
            }
            let act = log_coerced(value);
            let decay = exp_coerced(act) * self.cfg.se_scale + self.cfg.se_intercept;
            acc_decays.push(decay);
            acc_refs.push(reference);
        }
        acc_decays.reverse();
        let decays: Vec<(i64, f32)> = refs.iter().copied().zip(acc_decays).collect();
        self.hist[idx].decays = Some(decays.clone());
        decays
    }

    /// `an-logistic-noise` (`extras/adaptive-noise/adaptive-noise.lisp`):
    /// `exp(act-r-noise(:ans))`, or 0 without `:ans`. Consumes one draw.
    fn adaptive_noise(&mut self) -> f32 {
        match self.cfg.ans {
            Some(s) => exp_coerced(self.rng.borrow_mut().noise(s)),
            None => 0.0,
        }
    }

    /// `compute-adaptive-noise-references`: the `:ol nil` / numeric / `t`
    /// formulas with the logistic noise inside the log.
    fn adaptive_references(
        &self,
        refs: &[i64],
        count: usize,
        creation: i64,
        bll: f32,
        noise: f32,
    ) -> f32 {
        let now = self.time_ms;
        let minus_decay = -bll;
        let mut value = 0.0f32;
        let mut last = 0i64;
        for r in refs {
            let lag = (now - r).max(50) as f32;
            value += lag.powf(minus_decay);
            last = *r;
        }
        let scale = (0.001f32).powf(-bll);
        match &self.cfg.ol {
            Ol::Off => log_coerced(scale * value + noise),
            Ol::On => {
                let t = ((now - creation).max(0) as f32 / 1000.0).max(0.05);
                let denom = 1.0f32 + minus_decay;
                let p = (minus_decay as f64 * (t as f64).ln()).exp() as f32;
                let result = count as f32 * p / denom;
                log_coerced(result + noise)
            }
            Ol::Num(n) => {
                let denom = 1.0f32 + minus_decay;
                if count as f64 > *n {
                    let a = (now - creation).max(0) as f32;
                    let b = (now - last).max(0) as f32;
                    value += ((count as f32 - *n as f32) * (a.powf(denom) - b.powf(denom)))
                        / ((last - creation).max(50) as f32 * denom);
                }
                log_coerced(scale * value + noise)
            }
        }
    }

    /// `adaptive-noise-base-level-activation`
    /// (`extras/adaptive-noise/adaptive-noise.lisp:198`): the `:bll` base
    /// level with the noise inside the log, or a fixed base plus `n1` when
    /// `:bll` is nil.
    fn adaptive_base_level(&mut self, idx: usize, bll: Option<f32>, trace: bool) -> f32 {
        let show = trace && self.act_medium();
        let (refs, creation, count, fixed_base) = {
            let h = &self.hist[idx];
            (h.refs.clone(), h.creation_ms, h.count, h.fixed_base)
        };
        match bll {
            Some(bll) => {
                if show {
                    self.out(format!("Starting with blc: {}", lisp_f(self.cfg.blc)));
                }
                if count == 0 {
                    return -999999.0;
                }
                if show {
                    let refs_str: Vec<String> = refs.iter().map(|r| fmt_secs(*r)).collect();
                    self.out(format!(
                        "Computing base-level from {count} references ({})",
                        refs_str.join(" ")
                    ));
                    self.out(format!(
                        "  creation time: {} decay: {}  Optimized-learning: {}",
                        fmt_secs(creation),
                        lisp_f(bll),
                        ol_str(&self.cfg.ol)
                    ));
                }
                let noise = self.adaptive_noise();
                if show {
                    self.out(format!("Adding adaptive noise: {}", lisp_f(noise)));
                }
                self.cfg.blc + self.adaptive_references(&refs, count, creation, bll, noise)
            }
            None => {
                let base = match fixed_base {
                    Some(b) => {
                        if show {
                            self.out(format!("User provided chunk base-level: {}", lisp_f(b)));
                        }
                        b
                    }
                    None => {
                        if show {
                            self.out(format!("Starting with blc: {}", lisp_f(self.cfg.blc)));
                        }
                        self.cfg.blc
                    }
                };
                base + self.adaptive_noise()
            }
        }
    }

    /// The `base-level-activation` equation (`declarative-memory.lisp:2372`):
    /// the spacing-effect/adaptive-noise hooks when enabled, else
    /// `:bll`/fixed base/`:blc`. `trace` emits the medium-detail narration the
    /// retrieval path needs; `sdp`/blending/simulated retrieval pass `false`.
    pub(super) fn base_level(&mut self, idx: usize, bll: Option<f32>, trace: bool) -> f32 {
        let show = trace && self.act_medium();
        if self.cfg.eblse {
            let value = self.spacing_activation(idx);
            if show {
                self.out(format!("base-level hook returns: {}", lisp_f(value)));
            }
            return value;
        }
        if self.cfg.uan {
            let value = self.adaptive_base_level(idx, bll, trace);
            if show {
                self.out(format!("base-level hook returns: {}", lisp_f(value)));
            }
            return value;
        }
        let (refs, creation, count, fixed_base) = {
            let h = &self.hist[idx];
            (h.refs.clone(), h.creation_ms, h.count, h.fixed_base)
        };
        let base = match bll {
            Some(bll) => {
                if count == 0 {
                    // Mirrors the Lisp warning path; unreachable via
                    // merge/add (both start count at 1).
                    -999999.0
                } else {
                    if show {
                        self.out(format!("Starting with blc: {}", lisp_f(self.cfg.blc)));
                        let refs_str: Vec<String> = refs.iter().map(|r| fmt_secs(*r)).collect();
                        self.out(format!(
                            "Computing base-level from {count} references ({})",
                            refs_str.join(" ")
                        ));
                        self.out(format!(
                            "  creation time: {} decay: {}  Optimized-learning: {}",
                            fmt_secs(creation),
                            lisp_f(bll),
                            ol_str(&self.cfg.ol)
                        ));
                    }
                    let value = bl_equation(
                        self.time_ms,
                        &refs,
                        count as f32,
                        creation,
                        bll,
                        &self.cfg.ol,
                        self.cfg.act_scale,
                    );
                    if show {
                        self.out(format!("base-level value: {}", lisp_f(value)));
                    }
                    self.cfg.blc + value
                }
            }
            None => match fixed_base {
                Some(b) => {
                    if show {
                        self.out(format!("User provided chunk base-level: {}", lisp_f(b)));
                    }
                    b
                }
                None => {
                    if show {
                        self.out(format!("Starting with blc: {}", lisp_f(self.cfg.blc)));
                    }
                    self.cfg.blc
                }
            },
        };
        if show {
            self.out(format!("Total base-level: {}", lisp_f(base)));
        }
        base
    }

    /// `start-blending` (`extras/blending/blending.lisp:474`). Operates on
    /// the concretized request spec; emits the `:blt` trace and schedules
    /// either the completion (with the blended chunk) or the failure.
    pub(super) fn start_blending(&mut self, spec: &NewChunk) -> Result<(), EngineError> {
        let (slots, neg, rels) = match spec {
            NewChunk::Spec {
                slots, neg, rels, ..
            } => (slots.clone(), neg.clone(), rels.clone()),
            _ => return Err(bad("blending request must be a chunk spec".to_string())),
        };
        let idx_map = self.slot_index_map();

        // Request signature (`add-slot-spec-to-chunk-spec`): `= V` (V
        // non-nil) and `- nil` are filled; `= nil` is empty.
        let mut req_filled: HashSet<String> = HashSet::new();
        let mut req_empty: HashSet<String> = HashSet::new();
        for (s, v) in &slots {
            if s.starts_with(':') {
                continue;
            }
            if *v == Value::Nil {
                req_empty.insert(s.clone());
            } else {
                req_filled.insert(s.clone());
            }
        }
        for (s, v) in &neg {
            if s.starts_with(':') {
                continue;
            }
            if *v == Value::Nil {
                req_filled.insert(s.clone());
            }
        }
        for (s, _, _) in &rels {
            if !s.starts_with(':') {
                req_filled.insert(s.clone());
            }
        }
        let filled_names = Self::names_by_index(&req_filled, &idx_map);
        let empty_names = Self::names_by_index(&req_empty, &idx_map);

        // Candidates: DM order filtered by the request signature.
        let chunk_list: Vec<usize> = self
            .dm_alist_order()
            .into_iter()
            .filter(|i| {
                let fs = self.filled_slots(&self.dm[*i]);
                req_filled.iter().all(|f| fs.contains(f))
                    && req_empty.iter().all(|e| !fs.contains(e))
            })
            .collect();

        // `find-matching-chunks`: full spec unless (esc && mp), in which
        // case only the relative slots filter; the pushing loop reverses.
        let partial = self.cfg.esc && self.cfg.mp.is_some();
        let mut matched: Vec<usize> = Vec::new();
        for i in &chunk_list {
            let c = &self.dm[*i];
            let ok = if partial {
                rels.iter()
                    .all(|(s, v, op)| s.starts_with(':') || self.rel_test_v(*op, v, &c.get(s)))
            } else {
                self.chunk_matches_req(c, &slots, &neg, &rels)
            };
            if ok {
                matched.push(*i);
            }
        }
        matched.reverse();

        // Blended slots: union of matching chunks' filled slots minus the
        // fixed (`=`) slots unless `:blend-all-slots`.
        let mut all_slots: HashSet<String> = HashSet::new();
        for i in &matched {
            for s in self.filled_slots(&self.dm[*i]) {
                all_slots.insert(s);
            }
        }
        let fixed: HashSet<String> = slots
            .iter()
            .filter(|(s, _)| !s.starts_with(':'))
            .map(|(s, _)| s.clone())
            .collect();
        let mut blended_set: HashSet<String> = if self.cfg.blend_all_slots {
            all_slots
        } else {
            all_slots.difference(&fixed).cloned().collect()
        };
        blended_set.retain(|s| !s.starts_with(':'));
        let blended_slots = Self::names_by_index(&blended_set, &idx_map);

        // Temperature.
        let (temperature, temp_line) = if let Some(t) = self.cfg.tmp_ {
            (t, format!("Blending temperature is: {}", lisp_s_f32(t)))
        } else if let Some(a) = self.cfg.ans {
            let t = std::f32::consts::SQRT_2 * a;
            (
                t,
                format!(
                    "Blending temperature defaults to (* (sqrt 2) :ans): {}",
                    lisp_s_f32(t)
                ),
            )
        } else {
            (
                1.0,
                "Blending temperature defaults to 1 when :ans and :tmp are both nil".to_string(),
            )
        };

        // Activations (one noise draw per matching chunk, in match order).
        let mut act_list: Vec<(f32, usize, String)> = Vec::new();
        for i in &matched {
            let (a, base) = self.compute_blending_activation(*i, &slots, &neg);
            // `:min-bl`: a chunk whose base level is below the minimum is
            // suppressed (its activation and noise draw already happened).
            if let Some(minb) = self.cfg.min_bl {
                if base < minb {
                    continue;
                }
            }
            let name = self.dm[*i].name.clone().unwrap_or_default();
            act_list.push((a, *i, name));
        }

        if self.cfg.blt {
            let mut line = "Blending request for chunks".to_string();
            if !filled_names.is_empty() {
                line.push_str(&format!(" with slots ({})", filled_names.join(" ")));
            }
            line.push(' ');
            if !empty_names.is_empty() {
                line.push_str(&format!("without slots ({})", empty_names.join(" ")));
            }
            self.out(line);
            self.out(temp_line);
        }

        if act_list.is_empty() {
            if self.cfg.blt {
                self.out("No matching chunks found.".to_string());
                self.out("Blending request fails.".to_string());
            }
            let wait = failure_wait(&self.cfg, self.cfg.rt);
            self.push(
                self.time_ms + wait,
                Prio::Num(0),
                Out::Low,
                Event::BlendingFailure,
            );
            return Ok(());
        }

        // Softmax probabilities over the activations.
        let max_ai = act_list
            .iter()
            .map(|(a, _, _)| *a)
            .fold(f32::NEG_INFINITY, f32::max);
        let mut probs: Vec<f32> = act_list
            .iter()
            .map(|(a, _, _)| ((a - max_ai) / temperature).exp())
            .collect();
        let sum: f32 = probs.iter().sum();
        for p in probs.iter_mut() {
            *p /= sum;
        }

        if self.cfg.blt {
            for ((a, _, name), p) in act_list.iter().zip(probs.iter()) {
                self.out(format!(
                    "Chunk {name} matches blending request\n  Activation {}\n  Probability of recall {}\n",
                    lisp_s_f32(*a),
                    lisp_s_f32(*p)
                ));
            }
            self.out(format!(
                "\nSlots to be blended: ({})",
                blended_slots.join(" ")
            ));
        }

        // Per-slot blending. `results` keeps blended-slots order.
        let mut results: Vec<(String, Option<Value>)> =
            blended_slots.iter().map(|s| (s.clone(), None)).collect();

        for slot in &blended_slots {
            if self.cfg.blt {
                self.out(format!("Finding blended value for slot: {slot}"));
            }
            // possible-values: (name, value, prob, magnitude) for each
            // activation chunk with a non-nil slot value; magnitude is
            // `:value->mag` (default identity).
            let possible: Vec<(String, Value, f32, Value)> = act_list
                .iter()
                .zip(probs.iter())
                .filter_map(|((_, i, name), p)| {
                    let v = self.dm[*i].get(slot);
                    if v == Value::Nil {
                        None
                    } else {
                        Some((name.clone(), v.clone(), *p, v))
                    }
                })
                .collect();
            let slot_vals: Vec<Value> = possible.iter().map(|(_, v, _, _)| v.clone()).collect();
            let mags: Vec<Value> = possible.iter().map(|(_, _, _, m)| m.clone()).collect();
            if self.cfg.blt {
                self.out(format!(
                    "Matched chunks' slots contain: {}",
                    show_values(&slot_vals)
                ));
                self.out(format!(
                    "Magnitude values for those items: {}",
                    show_values(&mags)
                ));
            }

            let true_mags: Vec<Value> =
                mags.iter().filter(|m| **m != Value::Nil).cloned().collect();

            if true_mags.is_empty() {
                if self.cfg.blt {
                    self.out(
                        "When all magnitudes are nil there's nothing to blend and the slot is ignored"
                            .to_string(),
                    );
                }
                results.retain(|(s, _)| s != slot);
                continue;
            }

            if true_mags.iter().all(is_number) {
                if self.cfg.blt {
                    self.out("With numeric magnitudes blending by weighted average".to_string());
                }
                let mut sum = 0.0f32;
                for (name, _v, p, m) in &possible {
                    if *m == Value::Nil {
                        continue;
                    }
                    let mag = f32_of(m).unwrap_or(0.0);
                    let inc = p * mag;
                    sum += inc;
                    if self.cfg.blt {
                        self.out(format!(
                            " Chunk {name} with probability {} times magnitude {} = {} cumulative result: {}",
                            lisp_s_f32(*p),
                            lisp_s_f32(mag),
                            lisp_s_f32(inc),
                            lisp_s_f32(sum)
                        ));
                    }
                }
                set_result(&mut results, slot, Value::Num(sum as f64));
                if self.cfg.blt {
                    self.out(format!(" Final result: {}", lisp_s_f32(sum)));
                }
                continue;
            }

            // Chunk (or mixed) magnitudes: pick the candidate value whose
            // similarity-weighted probability mass is smallest.
            let which_chunks = true_mags.iter().all(|m| self.chunk_name_of(m).is_some());
            if self.cfg.blt {
                if which_chunks {
                    self.out(
                        "When all magnitudes are chunks blending based on similarities to all related chunks"
                            .to_string(),
                    );
                } else {
                    self.out(format!(
                        "When not all magnitudes are numbers or chunks blending based on similarities using those unique values: {}",
                        show_values(&true_mags)
                    ));
                }
            }
            let chunks: Vec<Value> = if which_chunks {
                // Intersection of the magnitudes' filled-slot sets.
                let mut inter: Option<HashSet<String>> = None;
                for m in &true_mags {
                    if let Some(n) = self.chunk_name_of(m) {
                        if let Some(c) = self.find_chunk(&n) {
                            let s: HashSet<String> = self.filled_slots(c).into_iter().collect();
                            inter = Some(match inter {
                                None => s,
                                Some(x) => x.intersection(&s).cloned().collect(),
                            });
                        }
                    }
                }
                let inter = inter.unwrap_or_default();
                if inter.is_empty() {
                    if self.cfg.blt {
                        self.out(
                            "No intersecting slots found all chunks will be tested".to_string(),
                        );
                    }
                    self.dm_alist_order()
                        .into_iter()
                        .map(|i| Value::Sym(self.dm[i].name.clone().unwrap_or_default()))
                        .collect()
                } else {
                    if self.cfg.blt {
                        self.out(format!(
                            "Intersection of slots for values is: ({})",
                            Self::names_by_index(&inter, &idx_map).join(" ")
                        ));
                    }
                    let list: Vec<Value> = self
                        .dm_alist_order()
                        .into_iter()
                        .filter(|i| {
                            let fs = self.filled_slots(&self.dm[*i]);
                            inter.iter().all(|f| fs.contains(f))
                        })
                        .map(|i| Value::Sym(self.dm[i].name.clone().unwrap_or_default()))
                        .collect();
                    if self.cfg.blt {
                        self.out(format!(
                            "Chunks from DM with those slots: {}",
                            show_values(&list)
                        ));
                    }
                    list
                }
            } else {
                let mut uniq: Vec<Value> = Vec::new();
                for m in &true_mags {
                    if !uniq.iter().any(|u| self.value_eq(u, m)) {
                        uniq.push(m.clone());
                    }
                }
                uniq
            };

            let best = if chunks.len() == 1 {
                if self.cfg.blt {
                    self.out(format!(" Only one possible value {}", chunks[0].show()));
                }
                chunks[0].clone()
            } else {
                let mut best_val: Option<Value> = None;
                let mut best_mag: Option<f32> = None;
                for val in &chunks {
                    if self.cfg.blt {
                        self.out(format!(" Comparing value {}", val.show()));
                    }
                    let mut sum = 0.0f32;
                    for (name, v, p, m) in &possible {
                        let sim = self.similarity(val, m);
                        sum += p * (sim * sim);
                        if self.cfg.blt {
                            self.out(format!(
                                "  Chunk {name} with probability {} slot value {} similarity: {} cumulative result: {}",
                                lisp_s_f32(*p),
                                v.show(),
                                lisp_s_f32(sim),
                                lisp_s_f32(sum)
                            ));
                        }
                    }
                    if best_mag.is_none() || sum < best_mag.unwrap() {
                        best_mag = Some(sum);
                        best_val = Some(val.clone());
                    }
                }
                best_val.unwrap_or(Value::Nil)
            };
            if self.cfg.blt {
                self.out(format!(" Final result: {}", best.show()));
            }
            set_result(&mut results, slot, best);
        }

        // Build the blended chunk def: fixed values (unless blend-all)
        // followed by the blended values, in blended-slots order.
        let mut new_slots: Vec<(String, Value)> = Vec::new();
        if !self.cfg.blend_all_slots {
            for (s, v) in &slots {
                if s.starts_with(':') {
                    continue;
                }
                new_slots.push((s.clone(), v.clone()));
            }
        }
        for (s, opt) in &results {
            if let Some(v) = opt {
                new_slots.push((s.clone(), v.clone()));
            }
        }

        if self.cfg.blt {
            let mut flat: Vec<String> = Vec::new();
            for (s, v) in &new_slots {
                flat.push(s.clone());
                flat.push(v.show());
            }
            self.out(format!(
                "This is the definition of the blended chunk:\n({})",
                flat.join(" ")
            ));
            self.out("\nComputing activation and latency for the blended chunk".to_string());
        }

        let mut act = 0.0f32;
        for (a, _, name) in &act_list {
            act += a.exp();
            if self.cfg.blt {
                self.out(format!(" Activation of chunk {name} is {}", lisp_s_f32(*a)));
            }
        }
        act = act.ln();
        if self.cfg.blt {
            self.out(format!(
                "Activation for blended chunk is: {}",
                lisp_s_f32(act)
            ));
        }

        let blended = NewChunk::Spec {
            ctype: String::new(),
            slots: new_slots,
            neg: Vec::new(),
            rels: Vec::new(),
            bare: true,
        };

        if act >= self.cfg.rt {
            let wait = latency(self.cfg.lf_ms, self.cfg.le, act);
            self.push(
                self.time_ms + wait,
                Prio::Num(0),
                Out::Medium,
                Event::BlendingComplete { spec: blended },
            );
        } else {
            if self.cfg.blt {
                self.out("Not above threshold so blending failed".to_string());
            }
            let wait = failure_wait(&self.cfg, self.cfg.rt);
            self.push(
                self.time_ms + wait,
                Prio::Num(0),
                Out::Low,
                Event::BlendingFailure,
            );
        }
        Ok(())
    }
}

/// Base-level-inhibition activation offset
/// (`bl-inhibition.lisp:105`): `-log(1 + (Tn/ts)^-ds)`, where `Tn` is the
/// seconds since the most recent reference, clamped to 0.05.
pub(super) fn inhibition_term(now_ms: i64, refs: &[i64], scale: f32, decay: f32) -> f32 {
    let Some(reference) = refs.first() else {
        return 0.0;
    };
    let t = ((now_ms - reference) as f64 / 1000.0).max(0.05) as f32;
    let ratio = (t / scale) as f64;
    let pow = ((-decay as f64) * ratio.ln()).exp() as f32;
    -(((1.0f32 + pow) as f64).ln()) as f32
}

/// `~s` of an `:ol` value in the base-level trace.
pub(super) fn ol_str(ol: &Ol) -> String {
    match ol {
        Ol::On => "T".to_string(),
        Ol::Off => "NIL".to_string(),
        // `:ol` takes an integer; the trace prints it with `~s`, not as a
        // float (the corpus has `:ol 1`/`:ol 3`).
        Ol::Num(n) if n.fract() == 0.0 => format!("{}", *n as i64),
        Ol::Num(n) => lisp_f(*n as f32),
    }
}

/// Generalized base-level decay (`compute-references`,
/// declarative-memory.lisp:2695), all single-float. Pure: takes the
/// clock, reference/creation times in ms, decay, OL flag and the
/// `:bll`-derived act-scale. Chunks with zero references never reach
/// here (the caller returns -999999.0 with a warning instead).
pub(super) fn bl_equation(
    now_ms: i64,
    refs: &[i64],
    count: f32,
    creation_ms: i64,
    decay: f32,
    ol: &Ol,
    act_scale: f32,
) -> f32 {
    let mut value = 0.0f32;
    let mut last_ref = 0i64;
    for r in refs {
        let lag = (now_ms - r).max(50);
        value += (lag as f32).powf(-decay);
        last_ref = *r;
    }
    match ol {
        Ol::Off => value.ln() + act_scale,
        Ol::Num(ol) => {
            if count as f64 > *ol {
                // (+ 1.0 minus-decay)
                let denom = 1.0f32 - decay;
                let a = (now_ms - creation_ms).max(0) as f32;
                let b = (now_ms - last_ref).max(0) as f32;
                value += ((count - *ol as f32) * (a.powf(denom) - b.powf(denom)))
                    / ((last_ref - creation_ms).max(50) as f32 * denom);
            }
            value.ln() + act_scale
        }
        Ol::On => {
            // NOTE: unlike the other branches, this one adds NO
            // act-scale (declarative-memory.lisp:2725). The units table
            // caught an early draft that added it. Uses the reference
            // *count* `n` (not the list length — under `:ol t` merges
            // clear the list but keep counting).
            let t = ((now_ms - creation_ms).max(0) as f32 / 1000.0).max(0.05);
            let denom = 1.0f32 - decay;
            // `expt-coerced` (`misc-utils.lisp:515`): SBCL's `expt` on singles
            // goes through the double `(exp (* y (log x)))` and coerces, which
            // differs from Rust's `f32::powf` by an ulp (the refs adjudicate).
            let p = ((-decay as f64) * (t as f64).ln()).exp() as f32;
            (((count * p / denom) as f64).ln()) as f32
        }
    }
}

/// `exp-coerced` (`misc-utils.lisp:554`): SBCL's `exp` on a single-float
/// computes in double and rounds, which differs from Rust's `f32::exp` by an
/// ulp (the associative-learning refs adjudicate).
pub(super) fn exp_coerced(x: f32) -> f32 {
    ((x as f64).exp()) as f32
}

/// `log-coerced` (`misc-utils.lisp:580`): SBCL's `log` on a single-float
/// computes in double and rounds.
pub(super) fn log_coerced(x: f32) -> f32 {
    ((x as f64).ln()) as f32
}

/// `needed-times` (`associative-learning.lisp:704`): with `:ol t` the history
/// lists are never populated (they stay `nil`), which is why every
/// `… with history NIL` line is nil.
pub(super) fn al_needed_refs(ol: &Ol, refs: &mut Vec<i64>, ct: i64) {
    if matches!(ol, Ol::On) {
        return;
    }
    refs.insert(0, ct);
    if let Ol::Num(n) = ol {
        let keep = (*n as usize).min(refs.len());
        refs.truncate(keep);
    }
}

/// `~s` of a reference history list (`NIL` when empty).
pub(super) fn al_history(refs: &[i64]) -> String {
    if refs.is_empty() {
        "NIL".to_string()
    } else {
        format!(
            "({})",
            refs.iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

impl Engine {
    /// Whether `name` was retrieved within the current finst span.
    pub(super) fn is_recent(&self, name: &str) -> bool {
        let now = self.time_ms;
        self.finsts
            .iter()
            .any(|(n, t)| n == name && now - t <= self.cfg.finst_span_ms)
    }

    /// Record a retrieval finst (newest last, capped at
    /// `:declarative-num-finsts`).
    pub(super) fn add_finst(&mut self, name: &str) {
        self.finsts.retain(|(n, _)| n != name);
        self.finsts.push((name.to_string(), self.time_ms));
        let cap = self.cfg.num_finsts;
        if self.finsts.len() > cap {
            let drop = self.finsts.len() - cap;
            self.finsts.drain(0..drop);
        }
    }

    /// Resolve a chunk name through merge aliases (`merging-test`).
    pub(super) fn resolve_alias(&self, n: &str) -> String {
        let mut cur = n.to_string();
        for _ in 0..8 {
            match self.copy_source.get(&cur) {
                Some(next) if *next != cur => {
                    cur = next.clone();
                    continue;
                }
                _ => {}
            }
            match self.chunk_alias.get(&cur) {
                Some(next) if *next != cur => cur = next.clone(),
                _ => break,
            }
        }
        cur
    }

    /// Value equality with chunk-name aliases applied to `Sym`s.
    pub(super) fn value_eq(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Sym(x), Value::Sym(y)) => self.resolve_alias(x) == self.resolve_alias(y),
            // `chunk-slot-equal` compares strings with `string-equal`
            // (`chunks.lisp:1854`), i.e. case-insensitively: `bst`'s
            // `pick-another-strategy` tests `value "reset"` against the
            // Reset button's encoded `"Reset"`.
            (Value::Str(x), Value::Str(y)) => x.eq_ignore_ascii_case(y),
            _ => a == b,
        }
    }

    /// Numeric inequality test (`> < >= <=`) for LHS/request slots.
    pub(super) fn rel_test(&self, op: SlotOp, pat: &Pattern, got: &Value) -> bool {
        let rhs = match pat {
            Pattern::Int(i) => *i as f64,
            Pattern::Num(x) => *x,
            _ => return false,
        };
        let lhs = match got {
            Value::Int(i) => *i as f64,
            Value::Num(x) => *x,
            _ => return false,
        };
        match op {
            SlotOp::Gt => lhs > rhs,
            SlotOp::Lt => lhs < rhs,
            SlotOp::Ge => lhs >= rhs,
            SlotOp::Le => lhs <= rhs,
            _ => false,
        }
    }

    /// Match one slot test; returns whether it passes.
    pub(super) fn test_value(
        &self,
        pat: &Pattern,
        got: &Value,
        b: &mut Bindings,
        negated: bool,
    ) -> Result<bool, EngineError> {
        let ok = match pat {
            Pattern::Nil => *got == Value::Nil,
            Pattern::Var(v) => match b.get(v) {
                Some(prev) => self.value_eq(prev, got),
                // A negative test must not bind its variable: the variable is
                // bound by the corresponding positive condition, which may be
                // matched after this buffer (`- target =let` before
                // `=imaginal> result =let`). Leave it unbound for that
                // condition rather than binding it to this slot's value.
                None if negated => return Ok(true),
                None => {
                    // A LHS `slot =var` requires the slot to be filled
                    // (`add-slot-spec-to-chunk-spec` sets the signature's
                    // filled bit); an empty slot does not bind.
                    if *got == Value::Nil {
                        false
                    } else {
                        b.insert(v.clone(), got.clone());
                        true
                    }
                }
            },
            Pattern::Sym(s) => self.value_eq(got, &Value::Sym(s.clone())),
            Pattern::Int(i) => *got == Value::Int(*i),
            Pattern::Num(x) => *got == Value::Num(*x),
            Pattern::Str(s) => self.value_eq(got, &Value::Str(s.clone())),
            Pattern::List(ps) => {
                let vals = ps
                    .iter()
                    .map(Value::from_pattern)
                    .collect::<Result<Vec<_>, _>>()?;
                self.value_eq(got, &Value::List(vals))
            }
        };
        Ok(if negated { !ok } else { ok })
    }
}

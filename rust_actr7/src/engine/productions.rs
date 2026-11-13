//! Procedural matching, conflict resolution, firing, compilation orchestration.
use super::compilation::{
    basic_vars, bound_text, buffer_usage, canon_composed, canon_prod, compute_retain_buffers,
    constant_value, failure_wait, goal_reason, imaginal_reason, indirect_action_buffer_name,
    indirect_action_vars, indirect_safe_bind_reason, instantiate_slot_names, is_buffer_name,
    latency, lhs_buffers, motor_reason, perceptual_reason, prod_lhs_slots, prod_rhs_actions,
    production_bind_vars, r_b_c1, r_b_c2, remove_dup_keep_last, render_compiled, render_production,
    retrieval_reason, rhs_buffers, sexp_lisp, sort_token,
};
use super::rhs::{explicit_out, implicit_clears, implicit_queries};
use super::*;

/// Resolve an RHS slot name: a dynamic slot (`=target`) becomes the value bound
/// to that variable (`instantiate-chunk-spec`), a plain slot is unchanged.
fn dynamic_slot(slot: &str, b: &Bindings) -> String {
    match slot.strip_prefix('=') {
        Some(var) => b
            .get(var)
            .map(|v| v.show())
            .unwrap_or_else(|| slot.to_string()),
        None => slot.to_string(),
    }
}

/// `replace-variables` over an RHS `!`-action argument list.
fn rename_sexp(s: &Sexp, ren: &HashMap<String, String>) -> Sexp {
    match s {
        Sexp::List(xs) => Sexp::List(xs.iter().map(|x| rename_sexp(x, ren)).collect()),
        Sexp::Sym(t) if t.starts_with('=') && t.len() > 1 => match ren.get(&t[1..]) {
            Some(v) => Sexp::Sym(format!("={v}")),
            None => s.clone(),
        },
        _ => s.clone(),
    }
}

/// ACT-R's default `buffer-index` order (`framework/buffers.lisp`); the
/// composition buffer loop sorts by it.
fn buffer_sort_index(name: &str) -> u32 {
    match name {
        "RETRIEVAL" => 0,
        "GOAL" => 1,
        "VISUAL" => 3,
        "VISUAL-LOCATION" => 4,
        "MANUAL" | "MANUAL-LEFT" | "MANUAL-RIGHT" => 5,
        "AURAL" => 6,
        "AURAL-LOCATION" => 7,
        "VOCAL" | "SPEECH" => 8,
        "BLENDING" => 9,
        "IMAGINAL" => 10,
        "TEMPORAL" => 11,
        _ => 100,
    }
}

/// Whether an LHS match/query item's dynamic slot-name variables are all bound,
/// so it can be matched now (ACT-R orders conditions by dependency).
/// `isa` matching including the vision chunk-type hierarchy: `line`, `oval`
/// and `text` are subtypes of `visual-object` (`(chunk-type line
/// (visual-object))`, `vision.lisp`), so `isa visual-object` matches them.
fn isa_matches(ctype: &str, want: &str) -> bool {
    ctype == want || (want == "VISUAL-OBJECT" && matches!(ctype, "LINE" | "OVAL" | "TEXT"))
}

fn lhs_dynamic_ready(item: &LhsItem, b: &Bindings) -> bool {
    match item {
        LhsItem::Match { tests, .. } => tests.iter().all(|t| match t.slot.strip_prefix('=') {
            Some(var) => b.contains_key(var),
            None => true,
        }),
        _ => true,
    }
}

impl Engine {
    pub(super) fn conflict_resolution(&mut self) -> Result<(), EngineError> {
        // Utilities are production U plus optional `:egs` noise, drawn
        // per candidate in definition order (`compute-utility` runs
        // inside the conflict-set loop). Ties go to the LAST production
        // when `:er` is nil, per `conflict-resolution` source.
        let mut best_u: Option<f32> = None;
        let mut tied: Vec<(usize, Bindings)> = Vec::new();
        let crt = self.cfg.crt;
        let mut matched: Vec<(usize, Bindings, f32, Vec<PpmMismatch>)> = Vec::new();
        for ep in self.exec_prods.clone() {
            let pname = self.model.productions[ep.index].name.clone();
            // `conflict-resolution` skips disabled productions
            // (`procedural.lisp:2195`).
            if self.disabled_prods.contains(&pname.to_uppercase()) {
                continue;
            }
            if crt {
                self.out(format!("Trying production: {pname}"));
            }
            let prod = self.model.productions[ep.index].clone();
            let mut mism: Vec<PpmMismatch> = Vec::new();
            if let Some(b) = self.match_prod(&prod, &ep.implicit_queries, &mut mism)? {
                // `warn-vision` (`vision.lisp:3224`): the conflict code of a
                // *matched* production with a `+visual>` RHS request calls the
                // vision module's `warn` (`production-parsing.lisp:1439`),
                // locking proc-display until a `+visual>` request unlocks it.
                if !self.visual_lock
                    && prod.rhs.iter().any(
                        |a| matches!(a, RhsAction::Request { buffer, .. } if buffer == "VISUAL"),
                    )
                {
                    self.visual_lock = true;
                }
                let mut u = self.utilities[ep.index];
                if self.cfg.egs != 0.0 {
                    u += self.rng.borrow_mut().noise(self.cfg.egs);
                }
                // `ppm-offset` (procedural.lisp:2373): the hook may override
                // the default `:ppm * Σ similarity` offset.
                let hook_offset = if self.cfg.ppm.is_some() && !mism.is_empty() {
                    self.ppm_hook_offset(&pname, &mism)
                } else {
                    None
                };
                if let Some(p) = self.cfg.ppm {
                    u += hook_offset.unwrap_or_else(|| p * mism.iter().map(|m| m.4).sum::<f32>());
                }
                // `compute-utility` with `save` (`utility-and-reward-1.lisp:357`,
                // called at `procedural.lisp:2230`) caches the value with the
                // ppm/search offsets folded in; `spp` prints it.
                self.production_utility[ep.index] = Some(u);
                matched.push((ep.index, b.clone(), u, mism));
                match best_u {
                    None => {
                        best_u = Some(u);
                        tied = vec![(ep.index, b)];
                    }
                    Some(cur) if u > cur => {
                        best_u = Some(u);
                        tied = vec![(ep.index, b)];
                    }
                    Some(cur) if u == cur => tied.push((ep.index, b)),
                    _ => {}
                }
            } else if crt {
                self.out("Fails because: ".to_string());
                let reason = self.crt_failure_reason(&prod);
                self.out(reason);
            }
        }
        self.last_conflict_set = matched
            .iter()
            .map(|(idx, _, _, _)| self.model.productions[*idx].name.clone())
            .collect();
        if crt {
            for (idx, _, _, _) in &matched {
                let name = &self.model.productions[*idx].name;
                self.out(format!("Production {name} matches"));
            }
        }
        // `:cst t`: print the conflict set's production instantiations and
        // their parameters (procedural.lisp:2283; `print-instantiation`).
        if self.cfg.cst && self.trace_on {
            let mut lines: Vec<String> = Vec::new();
            for (idx, b, u, mism) in &matched {
                let prod = &self.model.productions[*idx];
                // `print-instantiation` emits the definition one line at a
                // time, so split the render.
                lines.extend(
                    render_production(prod, b, mism, &[], &self.model.chunk_types)
                        .lines()
                        .map(str::to_string),
                );
                lines.push(format!("Parameters for production {}:", prod.name));
                // `spp-fct (list name :utility :u)` uses the `parameters`
                // branch (`utility-and-reward-1.lisp:666`), which prints every
                // requested keyword with `~S` (upper-case), unconditionally.
                lines.push(format!(" :UTILITY {u:6.3}"));
                // `:u` is the base production-u, not the adjusted value.
                let base = self.utilities[*idx];
                lines.push(format!(" :U {base:6.3}"));
            }
            for l in lines {
                self.out(l);
            }
        }
        let winner = match tied.len() {
            0 => None,
            // `conflict-resolution` permutes the whole best list when `:er` is
            // on (`procedural.lisp:2261`) — even for a single candidate — and
            // takes its head, so the draw count follows the candidate count
            // (`permute-list`, `framework/random.lisp:562`).
            _ if self.cfg.er => {
                let permuted = self.rng.borrow_mut().permute_list(tied);
                Some(permuted.into_iter().next().unwrap())
            }
            _ => Some(tied.into_iter().last().unwrap()),
        };
        // `production-history-recorder` (a conflict-set hook).
        self.record_production_history(&matched, winner.as_ref().map(|(i, _)| *i));
        match winner {
            Some((prod, bindings)) => {
                self.cr_armed = false;
                self.now(
                    Prio::Max,
                    Out::Always,
                    Event::ProductionSelected { prod, bindings },
                );
            }
            None => {
                self.cr_armed = true;
            }
        }
        Ok(())
    }

    /// The current conflict set with each match's partial-matched slots
    /// (`pmatches-internal`, `procedural-cmds.lisp:679`). Read-only: no trace
    /// output, no utility/noise draw, no scheduling, so `whynot` can report
    /// matches and partial matches without disturbing a run.
    pub(super) fn pmatches(&self) -> Vec<(usize, Bindings, Vec<PpmMismatch>)> {
        let mut out = Vec::new();
        for ep in self.exec_prods.clone() {
            let pname = self.model.productions[ep.index].name.clone();
            if self.disabled_prods.contains(&pname.to_uppercase()) {
                continue;
            }
            let prod = self.model.productions[ep.index].clone();
            let mut mism: Vec<PpmMismatch> = Vec::new();
            if let Ok(Some(b)) = self.match_prod(&prod, &ep.implicit_queries, &mut mism) {
                out.push((ep.index, b, mism));
            }
        }
        out
    }

    /// First failing condition reason for `:crt` (`failure-reason-string`,
    /// procedural.lisp:1981). Covers the buffer/state query cases the
    /// corpus exercises; other condition kinds fall back to a generic line.
    pub(super) fn crt_failure_reason(&self, prod: &Production) -> String {
        for item in &prod.lhs {
            if let LhsItem::Query { buffer, queries } = item {
                for (attr, val) in queries {
                    let ok = match (attr.as_str(), val) {
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
                            .and_then(|b| b.chunk.as_ref())
                            .is_some(),
                        ("STATE", Pattern::Sym(s)) if s == "FREE" => {
                            self.query_free(buffer).unwrap_or(false)
                        }
                        ("STATE", Pattern::Sym(s)) if s == "BUSY" => {
                            !self.query_free(buffer).unwrap_or(false)
                        }
                        ("ERROR", Pattern::Nil) => !self.buffer_error(buffer),
                        ("ERROR", Pattern::Sym(s)) if s == "T" => self.buffer_error(buffer),
                        _ => true,
                    };
                    if !ok {
                        let v = match val {
                            Pattern::Sym(s) => s.clone(),
                            other => format!("{other:?}"),
                        };
                        return format!(
                            "The {} {} query of the {} buffer failed.",
                            attr, v, buffer
                        );
                    }
                }
            }
        }
        "The production did not match.".to_string()
    }

    pub(super) fn match_prod(
        &self,
        prod: &Production,
        implicit_queries: &[String],
        ppm: &mut Vec<PpmMismatch>,
    ) -> Result<Option<Bindings>, EngineError> {
        let mut b = Bindings::new();
        // Buffer matches/queries run first (they bind the variables that
        // LHS `!eval!`/`!bind!` conditions may use; ACT-R orders these by
        // dependency, `production-parsing.lisp`). A dynamic slot
        // (`=imaginal> =target =x`) needs its *slot-name* variable bound,
        // which a later clause may supply (`retrieve-associate` binds
        // `=target` from the retrieval clause), so retry passes until every
        // such variable is bound.
        let mut pending: Vec<&LhsItem> = prod
            .lhs
            .iter()
            .filter(|i| matches!(i, LhsItem::Match { .. } | LhsItem::Query { .. }))
            .collect();
        loop {
            if pending.is_empty() {
                break;
            }
            let mut deferred: Vec<&LhsItem> = Vec::new();
            let mut progressed = false;
            for item in pending.drain(..) {
                if !lhs_dynamic_ready(item, &b) {
                    deferred.push(item);
                    continue;
                }
                progressed = true;
                if self.match_item(item, &mut b, ppm)?.is_none() {
                    return Ok(None);
                }
            }
            if !progressed {
                // No further progress possible: process the rest in source
                // order (an unbound dynamic slot resolves to itself).
                for item in deferred {
                    if self.match_item(item, &mut b, ppm)?.is_none() {
                        return Ok(None);
                    }
                }
                break;
            }
            pending = deferred;
        }
        for item in &prod.lhs {
            if matches!(
                item,
                LhsItem::Eval { .. } | LhsItem::Bind { .. } | LhsItem::MvBind { .. }
            ) && self.match_item(item, &mut b, ppm)?.is_none()
            {
                return Ok(None);
            }
        }
        // Negative slot tests may name a variable bound by a later buffer
        // (`- target =let` before `=imaginal> result =let`). Re-check them
        // once all positive conditions have supplied the bindings.
        for item in &prod.lhs {
            let LhsItem::Match { buffer, tests } = item else {
                continue;
            };
            if buffer == "GOAL" && self.cfg.threads {
                continue;
            }
            let Some(chunk) = self
                .buffers
                .get(buffer)
                .and_then(|buffer| buffer.chunk.as_ref())
            else {
                continue;
            };
            for test in tests {
                if test.op != SlotOp::Neg {
                    continue;
                }
                let Pattern::Var(var) = &test.value else {
                    continue;
                };
                let Some(prev) = b.get(var) else {
                    continue;
                };
                let slot = self.resolve_slot_name(&test.slot, &b);
                if self.value_eq(prev, &chunk.get(&slot)) {
                    return Ok(None);
                }
            }
        }
        for buffer in implicit_queries {
            if !self.query_free(buffer)? {
                return Ok(None);
            }
        }
        Ok(Some(b))
    }

    /// ACT-R per-chunk LHS tests (`test-var-slot`, slot ops).
    pub(super) fn match_chunk_tests(
        &self,
        chunk: &ChunkInst,
        buffer: &str,
        tests: &[crate::model::SlotTest],
        only_isa: bool,
        b: &mut Bindings,
        ppm: &mut Vec<PpmMismatch>,
    ) -> Result<bool, EngineError> {
        for t in tests {
            if t.slot == "ISA" {
                if only_isa {
                    if let Pattern::Sym(want) = &t.value {
                        if want != "CHUNK" && !isa_matches(&chunk.ctype, want) {
                            return Ok(false);
                        }
                    }
                }
                continue;
            }
            let slot = self.resolve_slot_name(&t.slot, b);
            let got = chunk.get(&slot);
            let ok = match t.op {
                SlotOp::Gt | SlotOp::Lt | SlotOp::Ge | SlotOp::Le => {
                    // A relational test's right-hand side may be a variable
                    // bound by an earlier condition (`<= width =difference`).
                    match &t.value {
                        Pattern::Var(v) => match b.get(v) {
                            Some(rhs) => self.rel_test_v(t.op, rhs, &got),
                            None => false,
                        },
                        _ => self.rel_test(t.op, &t.value, &got),
                    }
                }
                SlotOp::Neg => self.test_value(&t.value, &got, &mut *b, true)?,
                SlotOp::Eq => self.match_eq(buffer, &slot, &t.value, &got, b, ppm)?,
            };
            if !ok {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Threads extra: the goal chunk a production matches (the first in the
    /// goal set satisfying its `=goal>` conditions), for the matched-chunk
    /// clear.
    pub(super) fn threaded_goal_match(&self, prod: &Production) -> Option<String> {
        for item in &prod.lhs {
            if let LhsItem::Match { buffer, tests } = item {
                if buffer == "GOAL" {
                    let only_isa = tests.iter().all(|t| t.slot == "ISA");
                    for c in &self.goal_set {
                        let mut b = Bindings::new();
                        let mut ppm = Vec::new();
                        if self
                            .match_chunk_tests(c, buffer, tests, only_isa, &mut b, &mut ppm)
                            .unwrap_or(false)
                        {
                            return c.name.clone();
                        }
                    }
                }
            }
        }
        None
    }

    pub(super) fn match_item(
        &self,
        item: &LhsItem,
        b: &mut Bindings,
        ppm: &mut Vec<PpmMismatch>,
    ) -> Result<Option<()>, EngineError> {
        match item {
            LhsItem::Match { buffer, tests } => {
                // A LHS `ISA TYPE` as the *only* test constrains the buffer
                // chunk's type: a request-built `(ISA CHUNK)` chunk must not
                // satisfy `ISA GOAL` (bind-test). Retrieval/imaginal
                // conditions are the exception: ACT-R warns that such an
                // `ISA` provides no tests and matches any chunk there.
                let only_isa = tests.iter().all(|t| t.slot == "ISA")
                    && !matches!(buffer.as_str(), "RETRIEVAL" | "IMAGINAL");
                // Threaded goal: search the goal set in order for the first
                // chunk satisfying the tests.
                if buffer == "GOAL" && self.cfg.threads {
                    for chunk in &self.goal_set {
                        let mut b2 = b.clone();
                        let mut ppm2 = ppm.clone();
                        if self
                            .match_chunk_tests(chunk, buffer, tests, only_isa, &mut b2, &mut ppm2)?
                        {
                            *b = b2;
                            *ppm = ppm2;
                            if let Some(name) = &chunk.name {
                                b.insert(format!("={buffer}"), Value::Sym(name.clone()));
                            }
                            return Ok(Some(()));
                        }
                    }
                    return Ok(None);
                }
                let buf = self
                    .buffers
                    .get(buffer)
                    .ok_or_else(|| bad(format!("unknown buffer ={buffer}>")))?;
                let chunk = match &buf.chunk {
                    Some(c) => c,
                    None => return Ok(None),
                };
                if !self.match_chunk_tests(chunk, buffer, tests, only_isa, b, ppm)? {
                    return Ok(None);
                }
                // `buffer-name->variable` (`support/production-parsing.lisp:481`):
                // a `=buffer>` match binds `=BUFFER` to the chunk it held, which
                // production compilation uses for drop-out naming.
                if let Some(name) = &chunk.name {
                    b.insert(format!("={buffer}"), Value::Sym(name.clone()));
                }
            }
            LhsItem::Query { buffer, queries } => {
                for (attr, val) in queries {
                    let ok = match (attr.as_str(), val) {
                        ("BUFFER", Pattern::Sym(s)) if s == "FAILURE" => {
                            self.buffers
                                .get(buffer)
                                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?
                                .failed
                        }
                        ("BUFFER", Pattern::Sym(s)) if s == "EMPTY" => {
                            let b = self
                                .buffers
                                .get(buffer)
                                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?;
                            b.chunk.is_none() && !b.failed
                        }
                        ("BUFFER", Pattern::Sym(s)) if s == "FULL" => self
                            .buffers
                            .get(buffer)
                            .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?
                            .chunk
                            .is_some(),
                        // `?buffer> buffer requested/unrequested`
                        // (`buffers.lisp:1113`): a chunk (or failure flag) in
                        // the buffer whose `act-r-buffer-requested` matches.
                        ("BUFFER", Pattern::Sym(s)) if s == "REQUESTED" => {
                            let b = self
                                .buffers
                                .get(buffer)
                                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?;
                            (b.chunk.is_some() || b.failed) && b.requested
                        }
                        ("BUFFER", Pattern::Sym(s)) if s == "UNREQUESTED" => {
                            let b = self
                                .buffers
                                .get(buffer)
                                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?;
                            (b.chunk.is_some() || b.failed) && !b.requested
                        }
                        ("STATE", Pattern::Sym(s)) if s == "FREE" => self.query_free(buffer)?,
                        ("STATE", Pattern::Sym(s)) if s == "BUSY" => !self.query_free(buffer)?,
                        // `?visual-location> state error` (`vision.lisp:3206`):
                        // the vision module's `loc-failure` flag (visual
                        // location requests are otherwise always free).
                        ("STATE", Pattern::Sym(s))
                            if s == "ERROR" && buffer == "VISUAL-LOCATION" =>
                        {
                            self.loc_failure
                        }
                        // `?retrieval> state error`: the declarative module's
                        // failure flag (`query-module`, declarative-memory.lisp).
                        ("STATE", Pattern::Sym(s)) if s == "ERROR" && buffer == "RETRIEVAL" => {
                            self.buffers
                                .get(buffer)
                                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?
                                .failed
                        }
                        // extended-motor-actions adds per-stage queries.
                        ("PROCESSOR", Pattern::Sym(s)) if s == "FREE" => {
                            self.query_processor_free(buffer)?
                        }
                        ("PROCESSOR", Pattern::Sym(s)) if s == "BUSY" => {
                            !self.query_processor_free(buffer)?
                        }
                        ("PREPARATION", Pattern::Sym(s)) if s == "FREE" => {
                            self.query_prep_free(buffer)?
                        }
                        ("PREPARATION", Pattern::Sym(s)) if s == "BUSY" => {
                            !self.query_prep_free(buffer)?
                        }
                        ("EXECUTION", Pattern::Sym(s)) if s == "FREE" => {
                            self.query_execution_free(buffer)?
                        }
                        ("EXECUTION", Pattern::Sym(s)) if s == "BUSY" => {
                            !self.query_execution_free(buffer)?
                        }
                        ("LAST-COMMAND", v)
                            if matches!(
                                buffer.as_str(),
                                "MANUAL" | "MANUAL-LEFT" | "MANUAL-RIGHT"
                            ) =>
                        {
                            self.query_last_command(buffer, v)
                        }
                        // `?manual-left> index free/down` etc.
                        (attr, Pattern::Sym(s))
                            if matches!(attr, "INDEX" | "MIDDLE" | "RING" | "PINKIE" | "THUMB") =>
                        {
                            match s.as_str() {
                                "FREE" => self.query_finger_free(buffer, attr)?,
                                "BUSY" => !self.query_finger_free(buffer, attr)?,
                                "DOWN" => self.query_finger_down(buffer, attr)?,
                                "UP" => !self.query_finger_down(buffer, attr)?,
                                _ => {
                                    return Err(unimpl(format!("query ?{buffer}> {attr} {val:?}")))
                                }
                            }
                        }
                        // `?buffer> error t/nil`: only blending carries an
                        // error flag (`query-module`, modules.lisp:798).
                        ("ERROR", Pattern::Nil) => !self.buffer_error(buffer),
                        ("ERROR", Pattern::Sym(s)) if s == "T" => self.buffer_error(buffer),
                        (attr, value) if self.module_index_for_buffer(buffer).is_some() => {
                            let value = Value::from_pattern(value)?;
                            self.module_query(buffer, attr, &value).unwrap_or(false)
                        }
                        _ => {
                            return Err(unimpl(format!("query ?{buffer}> {attr} {val:?}")));
                        }
                    };
                    if !ok {
                        return Ok(None);
                    }
                }
            }
            LhsItem::Eval { value, safe } => match self.eval_expr(value, b) {
                Ok(v) => {
                    if v == Value::Nil {
                        return Ok(None);
                    }
                }
                Err(e) => {
                    if !*safe {
                        return Err(unimpl(format!("!EVAL! {e}")));
                    }
                    return Ok(None);
                }
            },
            LhsItem::Bind { var, value, safe } => match self.eval_expr(value, b) {
                Ok(v) => {
                    b.insert(var.clone(), v);
                }
                Err(e) => {
                    if !*safe {
                        return Err(unimpl(format!("!BIND! {e}")));
                    }
                    b.insert(var.clone(), Value::Nil);
                }
            },
            LhsItem::MvBind { vars, value } => {
                let vals = self
                    .eval_values(value, b)
                    .map_err(|e| unimpl(format!("!MV-BIND! {e}")))?;
                for (i, var) in vars.iter().enumerate() {
                    b.insert(var.clone(), vals.get(i).cloned().unwrap_or(Value::Nil));
                }
            }
        }
        Ok(Some(()))
    }

    /// `?buffer> state free`: module busy flags for perceptual-motor
    /// buffers, buffer flag otherwise. (Motor free = neither preparing
    /// nor executing; visual free = no attention shift in flight.)
    pub(super) fn query_free(&self, buffer: &str) -> Result<bool, EngineError> {
        // Lisp routes `state free` through the owning module's `:query`
        // callback (`query-module`, modules.lisp:794). Custom modules get the
        // first word; built-in modules keep their hand-coded flags.
        if self.module_index_for_buffer(buffer).is_some() {
            if let Some(answer) = self.module_query(buffer, "STATE", &Value::Sym("FREE".into())) {
                return Ok(answer);
            }
        }
        match buffer {
            "VISUAL" => Ok(!self.visual_busy && !self.visual_prep_busy),
            "MANUAL" => {
                if self.cfg.zz_ext {
                    Ok(!self.man_prep_busy
                        && !self.man_proc[0]
                        && !self.man_proc[1]
                        && !self.man_exec[0]
                        && !self.man_exec[1])
                } else {
                    Ok(!self.man_prep_busy && !self.man_exec_busy)
                }
            }
            "MANUAL-LEFT" => Ok(!self.man_proc[0] && !self.man_exec[0]),
            "MANUAL-RIGHT" => Ok(!self.man_proc[1] && !self.man_exec[1]),
            "IMAGINAL" => Ok(!self.imag_busy),
            // The imaginal-action buffer is the imaginal module's second
            // buffer; `?imaginal-action> state free` tests the same busy flag.
            "IMAGINAL-ACTION" => Ok(!self.imag_busy),
            "TRACKER" => Ok(true),
            "VOCAL" => Ok(!self.vocal_busy),
            "BLENDING" => Ok(!self.blending_busy),
            _ => Ok(!self
                .buffers
                .get(buffer)
                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?
                .busy),
        }
    }

    /// `?manual> processor free/busy` (extended-motor-actions).
    pub(super) fn query_processor_free(&self, buffer: &str) -> Result<bool, EngineError> {
        match buffer {
            "MANUAL" => Ok(!self.man_proc[0] && !self.man_proc[1]),
            "MANUAL-LEFT" => Ok(!self.man_proc[0]),
            "MANUAL-RIGHT" => Ok(!self.man_proc[1]),
            _ => self.query_free(buffer),
        }
    }

    /// `?manual> preparation free/busy` (extended-motor-actions).
    pub(super) fn query_prep_free(&self, buffer: &str) -> Result<bool, EngineError> {
        match buffer {
            "MANUAL" | "MANUAL-LEFT" | "MANUAL-RIGHT" => Ok(!self.man_prep_busy),
            "VOCAL" => Ok(!self.vocal_prep_busy),
            _ => self.query_free(buffer),
        }
    }

    /// `?manual> execution free/busy` (extended-motor-actions).
    pub(super) fn query_execution_free(&self, buffer: &str) -> Result<bool, EngineError> {
        match buffer {
            "MANUAL" => Ok(!self.man_exec[0] && !self.man_exec[1]),
            "MANUAL-LEFT" => Ok(!self.man_exec[0]),
            "MANUAL-RIGHT" => Ok(!self.man_exec[1]),
            _ => self.query_free(buffer),
        }
    }

    /// `?manual-left> last-command X` (`motor-extension.lisp:1392`).
    fn query_last_command(&self, buffer: &str, value: &Pattern) -> bool {
        let hand = match buffer {
            "MANUAL-LEFT" => Some(0),
            "MANUAL-RIGHT" => Some(1),
            _ => None,
        };
        let want = match value {
            Pattern::Sym(s) => s.as_str(),
            Pattern::Str(s) => s.as_str(),
            Pattern::Nil => return false,
            _ => return false,
        };
        match hand {
            Some(h) => self.zz_last_cmd[h].eq_ignore_ascii_case(want),
            None => self
                .zz_last_cmd
                .iter()
                .any(|cmd| cmd.eq_ignore_ascii_case(want)),
        }
    }

    /// `?manual-left> index free` (`motor-extension.lisp:1306`): whether a
    /// hand's finger is free (not busy with a movement).
    pub(super) fn query_finger_free(
        &self,
        buffer: &str,
        finger: &str,
    ) -> Result<bool, EngineError> {
        let hand = match buffer {
            "MANUAL-LEFT" => 0usize,
            "MANUAL-RIGHT" => 1,
            _ => return self.query_free(buffer),
        };
        let f = Self::touch_finger_idx(finger)
            .ok_or_else(|| bad(format!("unknown finger {finger}")))?;
        Ok(!self.zz_hands[hand].busy[f])
    }

    /// `?manual-left> index down`: whether a hand's finger is held down.
    pub(super) fn query_finger_down(
        &self,
        buffer: &str,
        finger: &str,
    ) -> Result<bool, EngineError> {
        let hand = match buffer {
            "MANUAL-LEFT" => 0usize,
            "MANUAL-RIGHT" => 1,
            _ => return self.query_free(buffer),
        };
        let f = Self::touch_finger_idx(finger)
            .ok_or_else(|| bad(format!("unknown finger {finger}")))?;
        Ok(self.zz_hands[hand].down[f])
    }

    /// `?buffer> error t/nil`: blending owns a module-level flag; other
    /// buffers use their `failed` state (e.g. an imaginal-action `nil`).
    pub(super) fn buffer_error(&self, buffer: &str) -> bool {
        if self.module_index_for_buffer(buffer).is_some() {
            if let Some(answer) = self.module_query(buffer, "STATE", &Value::Sym("ERROR".into())) {
                return answer;
            }
        }
        if buffer == "BLENDING" {
            self.blending_error
        } else {
            self.buffers.get(buffer).is_some_and(|b| b.failed)
        }
    }

    /// `instantiate-chunk-spec` (`chunk-spec.lisp:777`): a `=buffer>`/
    /// `*buffer>` mod-request whose *variable-bound* slot (a `=var` slot name,
    /// resolved at fire time) is not a possible slot extends the chunk, and
    /// the production action schedules an `EXTEND-BUFFER-CHUNK <buffer>` line
    /// before the modification (`production-parsing.lisp:1096`). A literal
    /// slot was already extended when the production was parsed, so it never
    /// emits the line (`simple-dynamic-model`'s `=target` vs `act-touch`'s
    /// `loc`). The extended slot stays possible, so a later mod of the same
    /// slot does not extend again.
    fn extend_buffer_chunk(&mut self, buffer: &str, dynamic: &[String], prio: i64) {
        let new: Vec<String> = dynamic
            .iter()
            .map(|s| s.to_uppercase())
            // `ISA` is the chunk's type, stored outside the slot list
            // (`chunk-spec.lisp`), and a `:keyword` is a request parameter —
            // neither is a possible-slot check (`keywordp` short-circuits in
            // `instantiate-chunk-spec`).
            .filter(|s| s != "ISA" && !s.starts_with(':'))
            .filter(|s| !self.valid_slots.contains(s))
            .collect();
        if new.is_empty() {
            return;
        }
        for s in new {
            self.valid_slots.insert(s);
        }
        self.now(
            Prio::Num(prio),
            explicit_out(self.cfg.rhst),
            Event::Emit {
                module: "PROCEDURAL".to_string(),
                line: format!("EXTEND-BUFFER-CHUNK {buffer}"),
            },
        );
    }

    pub(super) fn fire(&mut self, prod_idx: usize, bindings: &Bindings) -> Result<(), EngineError> {
        let prod = self.model.productions[prod_idx].clone();
        // Copies of chunk-valued `=buffer` refs are deduped within one
        // firing (PORTING-NOTES #41): a modify and a request referencing the same
        // buffer share `NAME-0`.
        let mut copied: HashMap<String, String> = HashMap::new();
        // RHS `!bind!`/`!mv-bind!` run first (they may be referenced by
        // later actions), then `!eval!`, then `!output!`. ACT-R reorders
        // these by variable dependencies (`production-parsing.lisp:1879`).
        let mut b = bindings.clone();
        for action in &prod.rhs {
            if let RhsAction::Bind { var, value, safe } = action {
                match self.eval_expr(value, &b) {
                    Ok(v) => {
                        b.insert(var.clone(), v);
                    }
                    Err(e) => {
                        if !*safe {
                            return Err(unimpl(format!("!BIND! {e}")));
                        }
                        b.insert(var.clone(), Value::Nil);
                    }
                }
            }
        }
        for action in &prod.rhs {
            if let RhsAction::MvBind { vars, value } = action {
                let vals = self
                    .eval_values(value, &b)
                    .map_err(|e| unimpl(format!("!MV-BIND! {e}")))?;
                for (i, var) in vars.iter().enumerate() {
                    b.insert(var.clone(), vals.get(i).cloned().unwrap_or(Value::Nil));
                }
            }
        }
        for action in &prod.rhs {
            if let RhsAction::Eval { value, safe } = action {
                // `(buffer-chunk <buffer> …)` runs in source order (after
                // `!output!`); see the action loop below.
                if let Sexp::List(items) = value {
                    if items.first().and_then(|x| x.as_sym()) == Some("BUFFER-CHUNK") {
                        continue;
                    }
                }
                if let Err(e) = self.m_eval(value, &b) {
                    if !*safe {
                        return Err(unimpl(format!("!EVAL! {e}")));
                    }
                }
            }
        }
        for action in &prod.rhs {
            match action {
                RhsAction::Output { args } => {
                    let mut text = String::new();
                    for a in args {
                        text.push_str(&self.show_output_arg(a, &b)?);
                        text.push(' ');
                    }
                    // `!output!` expands to `model-output`, which ACT-R gates
                    // on the printing module's `:v` stream
                    // (`framework/misc-utils.lisp:801`): with `(:v nil)`
                    // there is no stream, so nothing is printed at all.
                    self.out(text);
                    // `print-production-output` (`production-parsing.lisp:1826`)
                    // also calls `add-buffer-trace-notes`, which schedules an
                    // invisible maintenance event ACT-R counts.
                    self.now_m(Prio::Max, Out::Never, true, Event::Dummy);
                }
                RhsAction::Other { head, .. } => {
                    // `!stop!` schedules a priority-`:min` break event
                    // (`schedule-break-relative 0 :priority :min`); implicit
                    // clears and the other same-time actions still run first.
                    if head == "!STOP!" {
                        self.now_m(
                            Prio::Min,
                            Out::Medium,
                            true,
                            Event::Break {
                                details: "Stopped by !stop!".to_string(),
                            },
                        );
                    } else {
                        return Err(unimpl(format!("RHS {head} needs a Lisp evaluator")));
                    }
                }
                _ => {}
            }
        }
        // Whole-chunk requests resolved this fire (`+retrieval> =goal`),
        // used by production compilation for indirect actions.
        let mut reqs: Vec<(String, String)> = Vec::new();
        // Explicit actions in RHS order, then implicit harvest clears.
        for action in &prod.rhs {
            match action {
                RhsAction::Modify { buffer, slots } => {
                    // An empty `=buffer>` (no slots) is a no-op: ACT-R
                    // schedules no MOD event for it (e.g. `merging-test` P2).
                    if slots.is_empty() {
                        continue;
                    }
                    // A `=buffer>` modify stores a chunk-valued `=buffer`
                    // ref by *reference* (the bound chunk name), not a copy
                    // (`bind-buffer`; `merging-test`'s `=goal> value
                    // =imaginal` keeps `CHUNK0` so the merge alias to `B`
                    // applies). Only request specs copy (`instantiate-chunk-spec`).
                    let copy = false;
                    let mut sub = Vec::new();
                    let mut dyn_slots = Vec::new();
                    for (s, p) in slots {
                        // A dynamic slot (`=target =x`) is set under the name
                        // bound to `=target` (`instantiate-chunk-spec`).
                        if s.starts_with('=') {
                            dyn_slots.push(dynamic_slot(s, &b));
                        }
                        sub.push((
                            dynamic_slot(s, &b),
                            self.rhs_value(p, &b, copy, &mut copied)?,
                        ));
                    }
                    self.extend_buffer_chunk(buffer, &dyn_slots, 101);
                    self.now(
                        Prio::Num(100),
                        explicit_out(self.cfg.rhst),
                        Event::ModBuffer {
                            buffer: buffer.clone(),
                            slots: sub,
                            module: "PROCEDURAL".to_string(),
                        },
                    );
                }
                RhsAction::ModifyRequest { buffer, slots } => {
                    // `*buffer>` (`buffer-mod-request`): request a change from
                    // the buffer's module, which goes busy and applies it
                    // after its delay (the imaginal module's 200 ms).
                    if slots.is_empty() {
                        continue;
                    }
                    let copy = false;
                    let mut sub = Vec::new();
                    let mut dyn_slots = Vec::new();
                    for (s, p) in slots {
                        if s.starts_with('=') {
                            dyn_slots.push(dynamic_slot(s, &b));
                        }
                        sub.push((
                            dynamic_slot(s, &b),
                            self.rhs_value(p, &b, copy, &mut copied)?,
                        ));
                    }
                    self.extend_buffer_chunk(buffer, &dyn_slots, 61);
                    self.now(
                        Prio::Num(60),
                        explicit_out(self.cfg.rhst),
                        Event::ModuleModRequest {
                            buffer: buffer.clone(),
                            slots: sub,
                        },
                    );
                }
                RhsAction::Request { buffer, spec } => {
                    // `=imaginal>` is priority 100 and the request is
                    // priority 50, so a `+retrieval> =imaginal` in the same
                    // fire sees the modified chunk even though the visible
                    // MOD event is still queued.
                    self.apply_pending_imaginal_mod_peek();
                    let copy = matches!(buffer.as_str(), "GOAL" | "IMAGINAL");
                    let concrete = self.concretize(spec, &b, copy, &mut copied)?;
                    if let NewChunk::Named(n) = &concrete {
                        reqs.push((buffer.clone(), n.clone()));
                    }
                    let var_slots = match spec {
                        ChunkDesc::Spec { slots, .. } => slots
                            .iter()
                            .filter(|(_, p)| matches!(p, Pattern::Var(_)))
                            .map(|(s, _)| s.clone())
                            .collect(),
                        _ => Vec::new(),
                    };
                    self.now(
                        Prio::Num(50),
                        explicit_out(self.cfg.rhst),
                        Event::ModuleRequest {
                            buffer: buffer.clone(),
                            spec: concrete,
                            var_slots,
                            trace: true,
                        },
                    );
                }
                RhsAction::Clear { buffer } => {
                    self.now(
                        Prio::Num(10),
                        Out::Medium,
                        Event::ClearBuffer {
                            buffer: buffer.clone(),
                            module: "PROCEDURAL".to_string(),
                        },
                    );
                }
                RhsAction::DirectSet { buffer, spec } => {
                    // `@buffer> chunk` (`production-parsing.lisp:1147`):
                    // overwrite the buffer now (priority 90).
                    let copy = matches!(buffer.as_str(), "GOAL" | "IMAGINAL");
                    let concrete = self.concretize(spec, &b, copy, &mut copied)?;
                    self.now(
                        Prio::Num(90),
                        explicit_out(self.cfg.rhst),
                        Event::OverwriteBuffer {
                            buffer: buffer.clone(),
                            chunk: concrete,
                            requested: false,
                            shown: None,
                            module: "PROCEDURAL".to_string(),
                            copy_name: None,
                        },
                    );
                }
                RhsAction::DirectErase { buffer } => {
                    // `@buffer>` (`production-parsing.lisp:1140`): erase-buffer.
                    self.now(
                        Prio::Num(90),
                        explicit_out(self.cfg.rhst),
                        Event::EraseBuffer {
                            buffer: buffer.clone(),
                            module: "PROCEDURAL".to_string(),
                        },
                    );
                }
                // `(buffer-chunk <buffer> …)`: a model-body `!eval!` prints
                // the command-layer buffer dump (`tracking-test`'s `report`
                // production) in source order; buffer names are symbols.
                RhsAction::Eval { value, .. } => {
                    if let Sexp::List(items) = value {
                        if items.first().and_then(|x| x.as_sym()) == Some("BUFFER-CHUNK") {
                            let names: Vec<String> = items[1..]
                                .iter()
                                .filter_map(|a| a.as_sym().map(|s| s.to_string()))
                                .collect();
                            self.print_buffer_chunk(&names);
                        }
                    }
                }
                RhsAction::Output { .. }
                | RhsAction::Other { .. }
                | RhsAction::Bind { .. }
                | RhsAction::MvBind { .. } => {}
            }
        }
        for buffer in &self.exec_prods[prod_idx].implicit_clears.clone() {
            if buffer == "GOAL" && self.cfg.threads {
                if let Some(name) = self.threaded_goal_match(&prod) {
                    self.now(Prio::Num(10), Out::Medium, Event::ClearGoalThread { name });
                    continue;
                }
            }
            self.now(
                Prio::Num(10),
                Out::Medium,
                Event::ClearBuffer {
                    buffer: buffer.clone(),
                    module: "PROCEDURAL".to_string(),
                },
            );
        }
        self.now(Prio::Min, Out::Medium, Event::ConflictResolution);
        // `production-fired` runs actions, then compiles
        // (`production-compilation.lisp`, called from `production-fired`).
        self.compile_productions(prod_idx, &b, &reqs);
        Ok(())
    }

    /// First reason a production is invalid for compilation
    /// (`invalid-compilation-reason`, production-compilation.lisp:1243).
    /// `None` means valid. With `:cia` off, any indirect action invalidates.
    pub(super) fn invalid_compilation_reason(&self, idx: usize) -> Option<String> {
        let prod = &self.model.productions[idx];
        if prod
            .lhs
            .iter()
            .any(|x| matches!(x, LhsItem::Eval { safe: false, .. }))
            || prod
                .rhs
                .iter()
                .any(|x| matches!(x, RhsAction::Eval { safe: false, .. }))
        {
            return Some("it contains one or more !eval! operators".to_string());
        }
        if prod.lhs.iter().any(|x| {
            matches!(
                x,
                LhsItem::Match { tests, .. }
                    if tests.iter().any(|t| t.op != crate::model::SlotOp::Eq)
            )
        }) {
            return Some("it has conditions with modifiers on slot tests".to_string());
        }
        if let Some(buf) = indirect_action_buffer_name(prod) {
            if !self.cfg.cia {
                return Some(format!("it has an indirect action with the {buf} buffer"));
            }
            // With :cia, an indirect action whose variable is bound by a
            // `!bind!`/`!safe-bind!` is invalid (`:rir` ignores the retrieval
            // style).
            let ivars = indirect_action_vars(prod, self.cfg.rir);
            let bvars = production_bind_vars(prod);
            if ivars.iter().any(|v| bvars.contains(v)) {
                return Some(
                    "it has an indirect action using a variable bound with a !safe-bind!"
                        .to_string(),
                );
            }
        }
        None
    }

    /// `valid-1st-p` (`production-compilation.lisp:1315`): a production can
    /// become the "previous" only if it has no `!stop!` and no duplicate
    /// buffer action of the same kind.
    pub(super) fn valid_first_production(&self, idx: usize) -> bool {
        let prod = &self.model.productions[idx];
        for a in &prod.rhs {
            match a {
                RhsAction::Other { head, .. } if head == "!STOP!" => return false,
                _ => {}
            }
        }
        let mut seen: Vec<(char, &str)> = Vec::new();
        for a in &prod.rhs {
            let key = match a {
                RhsAction::Modify { buffer, .. } => Some(('=', buffer.as_str())),
                RhsAction::ModifyRequest { buffer, .. } => Some(('=', buffer.as_str())),
                RhsAction::Request { buffer, .. } => Some(('+', buffer.as_str())),
                _ => None,
            };
            if let Some(k) = key {
                if seen.contains(&k) {
                    return false;
                }
                seen.push(k);
            }
        }
        true
    }

    /// `invalid-1st-p-reason` (`production-compilation.lisp:1326`).
    pub(super) fn invalid_first_reason(&self, idx: usize) -> String {
        let prod = &self.model.productions[idx];
        for a in &prod.rhs {
            if let RhsAction::Other { head, .. } = a {
                if head == "!STOP!" {
                    return "it has a !stop! action".to_string();
                }
            }
        }
        let mut seen: Vec<(char, String)> = Vec::new();
        for a in &prod.rhs {
            let kv = match a {
                RhsAction::Modify { buffer, .. } | RhsAction::ModifyRequest { buffer, .. } => {
                    Some(('=', buffer.clone()))
                }
                RhsAction::Request { buffer, .. } => Some(('+', buffer.clone())),
                _ => None,
            };
            if let Some(k) = kv {
                if seen.contains(&k) {
                    return match k.0 {
                        '=' => format!(
                            "it has multiple modification actions for the {} buffer",
                            k.1
                        ),
                        _ => format!("it has multiple requests to the {} buffer", k.1),
                    };
                }
                seen.push(k);
            }
        }
        "of unknown reason. (This should not happen. Please report this issue.)".to_string()
    }

    /// A buffer both productions make a `+` request to: composition is
    /// refused (`Buffer X prevents composition … both make requests`).
    /// `determine-composable` (`production-compilation.lisp:1375`): the first
    /// buffer, in `remove-duplicates` order (the last occurrence wins), whose
    /// `(p1-usage, p2-usage)` pair is not in its compilation type's table,
    /// with the type's `whynot-reason` string.
    pub(super) fn determine_composable(&self, i1: usize, i2: usize) -> Option<(String, String)> {
        let p1 = &self.model.productions[i1];
        let p2 = &self.model.productions[i2];
        let mut all: Vec<String> = lhs_buffers(p1);
        all.extend(lhs_buffers(p2));
        all.extend(rhs_buffers(p1));
        all.extend(rhs_buffers(p2));
        let buffers: Vec<String> = all
            .iter()
            .enumerate()
            .filter(|(i, b)| !all[i + 1..].contains(b))
            .map(|(_, b)| b.clone())
            .collect();
        for buffer in &buffers {
            let u1 = buffer_usage(p1, buffer);
            let u2 = buffer_usage(p2, buffer);
            if buffer == "RETRIEVAL" {
                match RETRIEVAL_COMPOSE
                    .iter()
                    .find(|(a, b, _)| *a == u1 && *b == u2)
                    .map(|(_, _, c)| *c)
                {
                    None => return Some((buffer.clone(), retrieval_reason(u1, u2))),
                    Some(CompCheck::T) => {}
                    Some(CompCheck::RbC1) => {
                        if !r_b_c1(p2, buffer) {
                            return Some((
                                buffer.clone(),
                                "when the first production makes a request and the second does not harvest it the second can only query for state busy or buffer empty.".to_string(),
                            ));
                        }
                    }
                    Some(CompCheck::RbC2) => {
                        if !r_b_c2(p1, p2, buffer) {
                            return Some((
                                buffer.clone(),
                                "the queries in both productions must be the same.".to_string(),
                            ));
                        }
                    }
                }
                continue;
            }
            let (table, reason): (&[(u32, u32)], fn(u32, u32) -> String) = match buffer.as_str() {
                "GOAL" => (GOAL_COMPOSE, goal_reason),
                "VISUAL-LOCATION" | "VISUAL" | "AURAL-LOCATION" | "AURAL" | "TEMPORAL" => {
                    (PERCEPTUAL_COMPOSE, perceptual_reason)
                }
                "IMAGINAL" => (IMAGINAL_COMPOSE, imaginal_reason),
                _ => (MOTOR_COMPOSE, motor_reason),
            };
            if !table.contains(&(u1, u2)) {
                return Some((buffer.clone(), reason(u1, u2)));
            }
        }
        None
    }

    /// Install a composed production into the runtime set (parse its text
    /// and add it with utility 0). Compiled productions with contradictory
    /// conditions simply never match.
    pub(super) fn install_compiled(
        &mut self,
        name: &str,
        doc: &str,
        lhs: &[(String, Vec<(String, SlotVal)>)],
        rhs: &[(String, Vec<(String, String)>)],
    ) {
        let mut text = format!("(P {name}");
        if !doc.is_empty() {
            text.push_str(&format!(" \"{doc}\""));
        }
        for (token, slots) in lhs {
            text.push_str(&format!(" {token}"));
            for (n, v) in slots {
                if n.is_empty() {
                    text.push_str(&format!(" {}", v.render()));
                } else {
                    text.push_str(&format!(" {n} {}", v.render()));
                }
            }
        }
        text.push_str(" ==>");
        for (token, slots) in rhs {
            text.push_str(&format!(" {token}"));
            for (n, v) in slots {
                text.push_str(&format!(" {n} {v}"));
            }
        }
        text.push(')');
        if let Ok(prod) = crate::model::parse_production_text(&text) {
            self.model.productions.push(prod);
            self.utilities.push(0.0);
            self.production_utility.push(None);
            let idx = self.model.productions.len() - 1;
            let p = self.model.productions[idx].clone();
            self.exec_prods.push(ExecProd {
                index: idx,
                implicit_clears: implicit_clears(&p, &self.cfg.do_not_harvest, self.cfg.threads),
                implicit_queries: implicit_queries(&p, &self.cfg.do_not_query),
            });
        }
    }

    /// `compile-productions` (`production-compilation.lisp:1129`): compose
    /// the previous fired production with the current one and emit the
    /// `:epl`/`:pct` trace. This port covers the goal-style test+modify
    /// composition exercised by `ppm-compilation`; compiled productions are
    /// not (yet) added to the runtime set.
    pub(super) fn compile_productions(
        &mut self,
        idx: usize,
        bindings: &Bindings,
        reqs: &[(String, String)],
    ) {
        if !self.cfg.epl {
            return;
        }
        let trace = self.cfg.pct && self.trace_on;
        let p2name = self.model.productions[idx].name.clone();
        if trace {
            self.out(format!(
                "Production Compilation process started for {p2name}"
            ));
        }
        // `valid-compilation-production` (`production-compilation.lisp:1188`).
        if let Some(reason) = self.invalid_compilation_reason(idx) {
            if trace {
                self.out(format!(
                    "  Production {p2name} is not valid for compilation"
                ));
                self.out(format!("   because {reason}"));
            }
            self.prev_prod = None;
            return;
        }
        if let Some((p1_idx, p1_bindings, p1_reqs)) = self.prev_prod.clone() {
            let p1name = self.model.productions[p1_idx].name.clone();
            if self.time_ms - self.prev_prod_ms > self.cfg.tt_ms {
                if trace {
                    self.out(format!(
                        "  Cannot compile {p1name} and {p2name} because the time between them exceeds the threshold time."
                    ));
                }
            } else if let Some((buf, reason)) = self.determine_composable(p1_idx, idx) {
                if trace {
                    self.out(format!(
                        "  Buffer {buf} prevents composition of these productions"
                    ));
                    self.out(format!("   because {reason}"));
                    self.out(format!(
                        "  Production {p1name} and {p2name} cannot be composed."
                    ));
                }
            } else {
                let (retain1, retain2) = if self.cfg.cia && self.cfg.rir {
                    compute_retain_buffers(
                        &self.model.productions[p1_idx],
                        &self.model.productions[idx],
                    )
                } else {
                    (Vec::new(), Vec::new())
                };
                let reason = indirect_safe_bind_reason(&self.model.productions[p1_idx], &retain1)
                    .or_else(|| indirect_safe_bind_reason(&self.model.productions[idx], &retain2));
                if let Some(reason) = reason {
                    if trace {
                        self.out(format!(
                            "  Production {p1name} and {p2name} cannot be composed"
                        ));
                        self.out(format!("   because {reason}"));
                    }
                } else {
                    let (lhs, rhs, _compose_dropped) = self.compose_pair(
                        p1_idx,
                        &p1_bindings,
                        &p1_reqs,
                        idx,
                        bindings,
                        &retain1,
                        &retain2,
                    );
                    let cname = format!("PRODUCTION{}", self.compiled_count);
                    // `new-production-name` (`production-compilation.lisp:1874`)
                    // consumes a name for every composition that reaches this
                    // point, duplicate or not (a recreated name is released but
                    // the counter never rewinds).
                    self.compiled_count += 1;
                    // `check-for-duplicate-productions`
                    // (`production-compilation.lisp:1915`): a composed production
                    // equivalent to an existing one is not installed.
                    let dup = {
                        let crep = canon_composed(&lhs, &rhs);
                        self.model
                            .productions
                            .iter()
                            .enumerate()
                            .find_map(|(i, p)| {
                                if canon_prod(p) == crep {
                                    Some((i, p.name.clone(), i >= self.base_prods))
                                } else {
                                    None
                                }
                            })
                    };
                    // `initialize-utility-for-compiled-production`
                    // (`utility-and-reward-1.lisp:477`): the composed production's
                    // reward is inherited from its parents (the max when both are
                    // numeric).
                    let reward: Option<Reward> = {
                        let r1 = self.model.productions[p1_idx].reward.clone();
                        let r2 = self.model.productions[idx].reward.clone();
                        match (&r1, &r2) {
                            (Some(Reward::Num(a)), Some(Reward::Num(b))) => {
                                Some(Reward::Num(a.max(*b)))
                            }
                            (Some(Reward::Num(a)), _) => Some(Reward::Num(*a)),
                            (_, Some(Reward::Num(b))) => Some(Reward::Num(*b)),
                            _ => r1.or(r2),
                        }
                    };
                    // `update-utility-for-compiled-production`
                    // (`utility-and-reward-1.lisp:497`) runs even while the
                    // trace is suppressed. Only its printed block is gated.
                    if let Some((didx, _, compiled)) = &dup {
                        if *compiled && self.cfg.esc && self.cfg.ul {
                            let alpha = self.cfg.alpha;
                            let parent = self.utilities[p1_idx];
                            let old = self.utilities[*didx];
                            self.utilities[*didx] = old + alpha * (parent - old);
                        }
                    }
                    let install_doc = {
                        let mut dropped: Vec<(String, String)> = Vec::new();
                        for x in lhs_buffers(&self.model.productions[idx]) {
                            if x != "RETRIEVAL" {
                                continue;
                            }
                            if buffer_usage(&self.model.productions[p1_idx], &x) & 4 == 0 {
                                continue;
                            }
                            if let Some(v) = bindings.get(&format!("={x}")) {
                                let name = v.show();
                                let name = self.copy_source.get(&name).cloned().unwrap_or(name);
                                dropped.push((x, name));
                            }
                        }
                        dropped.sort_by(|a, b| a.0.cmp(&b.0));
                        if dropped.is_empty() {
                            format!("{p1name} & {p2name}")
                        } else {
                            let vals: Vec<String> =
                                dropped.iter().map(|(_, n)| n.clone()).collect();
                            format!("{p1name} & {p2name} - {}", vals.join(" - "))
                        }
                    };
                    if trace {
                        self.out(format!(
                            "  Production {p1name} and {p2name} are being composed."
                        ));
                        if let Some((didx, dname, compiled)) = &dup {
                            if *compiled {
                                self.out(format!("  Recreating production {dname}"));
                                let u = self.utilities[*didx];
                                let utility = self.production_utility[*didx];
                                let reward = self.model.productions[*didx].reward.clone();
                                self.print_prod_params(dname, u, utility, reward);
                            } else {
                                self.out(format!(
                                    "  Result is production {dname}\n  No actions taken."
                                ));
                            }
                        } else {
                            self.out("  New production:\n".to_string());
                            // `dropped-chunks` (`production-compilation.lisp:1723`):
                            // for each `=` buffer of p2 of a drop-out type that p1
                            // requested, the chunk p2's buffer variable held (the
                            // copy's source if it was one).
                            let mut dropped: Vec<(String, String)> = Vec::new();
                            for x in lhs_buffers(&self.model.productions[idx]) {
                                if x != "RETRIEVAL" {
                                    continue;
                                }
                                if buffer_usage(&self.model.productions[p1_idx], &x) & 4 == 0 {
                                    continue;
                                }
                                if let Some(v) = bindings.get(&format!("={x}")) {
                                    let name = v.show();
                                    let name = self.copy_source.get(&name).cloned().unwrap_or(name);
                                    dropped.push((x, name));
                                }
                            }
                            dropped.sort_by(|a, b| a.0.cmp(&b.0));
                            let doc = if dropped.is_empty() {
                                format!("{p1name} & {p2name}")
                            } else {
                                let vals: Vec<String> =
                                    dropped.iter().map(|(_, n)| n.clone()).collect();
                                format!("{p1name} & {p2name} - {}", vals.join(" - "))
                            };
                            // `sort-conditions` (`production-compilation.lisp:1903`)
                            // plus the assembly at 1741: `!`-conditions are collected
                            // and prepended (in reverse `push` order), then the rest
                            // sorted by token.
                            let mut bangs: Vec<(String, Vec<(String, SlotVal)>)> = lhs
                                .iter()
                                .filter(|(t, _)| t.starts_with('!'))
                                .cloned()
                                .collect();
                            bangs.sort_by(|a, b| sort_token(&a.0).cmp(&sort_token(&b.0)));
                            bangs.reverse();
                            let mut rest: Vec<(String, Vec<(String, SlotVal)>)> = lhs
                                .iter()
                                .filter(|(t, _)| !t.starts_with('!'))
                                .cloned()
                                .collect();
                            rest.sort_by(|a, b| sort_token(&a.0).cmp(&sort_token(&b.0)));
                            let sorted: Vec<(String, Vec<(String, SlotVal)>)> =
                                bangs.into_iter().chain(rest).collect();
                            // Names are already substituted from the
                            // composition `mappings`; the instantiation
                            // bindings remain as the fallback for anything the
                            // mappings leave alone (and for `:ppm`).
                            self.out(render_compiled(&cname, &doc, &sorted, &rhs, bindings));
                            // A freshly compiled production has no cached
                            // `production-utility` yet and `u = :nu` (0).
                            self.print_prod_params(&cname, 0.0, None, reward.clone());
                        }
                    }
                    if dup.is_none() {
                        self.install_compiled(&cname, &install_doc, &lhs, &rhs);
                        if let Some(last) = self.model.productions.last_mut() {
                            last.reward = reward;
                        }
                    }
                }
            }
        } else if trace {
            self.out("  No previous production to compose with.".to_string());
        }
        // `handle-check-valid-1st-p` (`production-compilation.lisp:1338`).
        if self.valid_first_production(idx) {
            self.prev_prod = Some((idx, bindings.clone(), reqs.to_vec()));
            self.prev_prod_ms = self.time_ms;
            if trace {
                let name = self.model.productions[idx].name.clone();
                self.out(format!("  Setting previous production to {name}."));
            }
        } else {
            if trace {
                self.out(format!(
                    "  Production {p2name} is not valid as a first production for compilation"
                ));
                self.out(format!("   because {}", self.invalid_first_reason(idx)));
            }
            self.prev_prod = None;
        }
    }

    /// ACT-R's cross-production `mappings` (`compose-productions`,
    /// `modules/production-compilation.lisp:1556`): the union of each used
    /// buffer's `MAP-*` function, sorted by `constant-value-p` key `0`/`1`/`2`
    /// (the order matters — a mapping whose key also appears later rewrites
    /// the later entry's car, which is how `=OTHER` resolves to `NUMBER`).
    ///
    /// Only the two branches the corpus's `:pct` traces exercise are ported:
    /// `MAP-RETRIEVAL-BUFFER` (`tools/retrieval-compilation.lisp:71`) and the
    /// `MAP-GOAL-BUFFER` RHS `=` to LHS `=` case
    /// (`tools/goal-compilation.lisp:216`).
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn compose_mappings(
        &self,
        p1: &Production,
        p2: &Production,
        p1_lhs: &[(String, Vec<(String, SlotVal)>)],
        p1_acts: &[(char, String, Vec<(String, SlotVal)>, Option<String>)],
        p2_lhs: &[(String, Vec<(String, SlotVal)>)],
        p2_acts: &[(char, String, Vec<(String, SlotVal)>, Option<String>)],
        b1: &Bindings,
        b2: &Bindings,
    ) -> Vec<(SlotVal, SlotVal)> {
        let lhs_of =
            |lhs: &[(String, Vec<(String, SlotVal)>)], buf: &str| -> Vec<(String, SlotVal)> {
                lhs.iter()
                    .find(|(b, _)| b == buf)
                    .map(|(_, s)| s.clone())
                    .unwrap_or_default()
            };
        let act_of = |acts: &[(char, String, Vec<(String, SlotVal)>, Option<String>)],
                      buf: &str,
                      ops: &[char]|
         -> Vec<(String, SlotVal)> {
            acts.iter()
                .find(|(op, b, _, _)| b == buf && ops.contains(op))
                .map(|(_, _, s, _)| s.clone())
                .unwrap_or_default()
        };
        // Slots whose *name* is a variable, as `(variable, bound text)`.
        let var_slots = |s: &[(String, SlotVal)], b: &Bindings| -> Vec<(String, String)> {
            s.iter()
                .filter_map(|(n, _)| {
                    n.strip_prefix('=')
                        .and_then(|v| bound_text(n, b).map(|t| (v.to_string(), t)))
                })
                .collect()
        };
        let has_slot = |s: &[(String, SlotVal)], n: &str| s.iter().any(|(x, _)| x == n);

        let mut m: Vec<(SlotVal, SlotVal)> = Vec::new();

        // --- MAP-RETRIEVAL-BUFFER ---------------------------------------
        let u1r = buffer_usage(p1, "RETRIEVAL");
        let u2r = buffer_usage(p2, "RETRIEVAL");
        if matches!(u1r, 4 | 12 | 20 | 28) && matches!(u2r, 8 | 12 | 24 | 28) {
            if let Some(chunk) = b2.get("=RETRIEVAL").map(|v| v.show()) {
                m.push((
                    SlotVal::Var("RETRIEVAL".to_string()),
                    SlotVal::Lit(chunk.clone()),
                ));
                // p1's `+retrieval>` request: a variable slot *name* and a
                // variable slot *value* both instantiate from p1's firing.
                for (op, buf, slots, _) in p1_acts {
                    if buf != "RETRIEVAL" || !matches!(*op, '+' | 'r') {
                        continue;
                    }
                    for (name, v) in slots {
                        if name.starts_with('=') {
                            if let Some(t) = bound_text(name, b1) {
                                m.push((SlotVal::Var(name[1..].to_string()), SlotVal::Lit(t)));
                            }
                        }
                        // `MAP-RETRIEVAL-BUFFER` only instantiates a
                        // variable *value* for an `=` request slot. Negative
                        // and relative slots keep their variables (`- key
                        // =key` must not become `- key "f"`).
                        let value_eq = !name.starts_with('-')
                            && !name.starts_with('>')
                            && !name.starts_with('<');
                        if value_eq {
                            if let SlotVal::Var(n) = v {
                                if let Some(t) = bound_text(n, b1) {
                                    m.push((SlotVal::Var(n.clone()), SlotVal::Lit(t)));
                                }
                            }
                        }
                    }
                }
                // p2's `=retrieval>` condition: a variable slot name comes
                // from p2's instantiation, and a variable value is read from
                // the retrieved chunk at the *instantiated* slot name.
                for (name, v) in &lhs_of(p2_lhs, "RETRIEVAL") {
                    let resolved = bound_text(name, b2);
                    if name.starts_with('=') {
                        if let Some(t) = &resolved {
                            m.push((SlotVal::Var(name[1..].to_string()), SlotVal::Lit(t.clone())));
                        }
                    }
                    if let SlotVal::Var(n) = v {
                        let slot = if name.starts_with('=') {
                            resolved.clone().unwrap_or_else(|| name.clone())
                        } else {
                            name.clone()
                        };
                        if let Some(cv) = self.chunk_name_slot(&chunk, &slot) {
                            m.push((SlotVal::Var(n.clone()), SlotVal::Lit(cv)));
                        }
                    }
                }
            }
        }

        // --- MAP-IMAGINAL-BUFFER, RHS `=`/`*` to LHS `=` -----------------
        let u1i = buffer_usage(p1, "IMAGINAL");
        let u2i = buffer_usage(p2, "IMAGINAL");
        if matches!(u1i, 9 | 25 | 56) && matches!(u2i, 8 | 9 | 12 | 13 | 24 | 25 | 28 | 29 | 56) {
            let p1a = lhs_of(p1_lhs, "IMAGINAL");
            let p1b = act_of(p1_acts, "IMAGINAL", &['=', '*']);
            let p2a = lhs_of(p2_lhs, "IMAGINAL");
            let p2b = act_of(p2_acts, "IMAGINAL", &['=', '*']);
            let slotsa = instantiate_slot_names(&p1a, b1);
            let slotsb = instantiate_slot_names(&p1b, b1);
            let slots2 = instantiate_slot_names(&p2a, b2);
            let slots2b = instantiate_slot_names(&p2b, b2);
            let vsa = var_slots(&p1a, b1);
            let vsb = var_slots(&p1b, b1);
            let vs2 = var_slots(&p2a, b2);
            let vs2b = var_slots(&p2b, b2);
            let p1slots: Vec<(String, SlotVal)> = slotsa
                .iter()
                .filter(|(n, _)| !slotsb.iter().any(|(x, _)| x == n))
                .cloned()
                .chain(slotsb.iter().cloned())
                .collect();
            let mut interesting: Vec<String> = Vec::new();
            for src in [&p1slots, &slots2, &slots2b] {
                for (n, _) in src {
                    if !interesting.contains(n) {
                        interesting.push(n.clone());
                    }
                }
            }
            for slot in &interesting {
                let p1v: Vec<&String> = vsa
                    .iter()
                    .chain(vsb.iter())
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                let p1vb: Vec<&String> = vsb
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                let p2v: Vec<&String> = vs2
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                let p2vb: Vec<&String> = vs2b
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                if !p1v.is_empty() && !p2v.is_empty() {
                    for a in &p1v {
                        for c in &p2v {
                            m.push((SlotVal::Var((*a).clone()), SlotVal::Var((*c).clone())));
                        }
                    }
                }
                if !p1v.is_empty() && !p2vb.is_empty() {
                    for a in &p1v {
                        for c in &p2vb {
                            m.push((SlotVal::Var((*a).clone()), SlotVal::Var((*c).clone())));
                        }
                    }
                }
                if !p1v.is_empty() && has_slot(&p2a, slot) {
                    for a in &p1v {
                        m.push((SlotVal::Var((*a).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p1vb.is_empty() && has_slot(&p2b, slot) {
                    for a in &p1vb {
                        m.push((SlotVal::Var((*a).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p2v.is_empty() && (has_slot(&p1a, slot) || has_slot(&p1b, slot)) {
                    for c in &p2v {
                        m.push((SlotVal::Var((*c).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p2vb.is_empty() && has_slot(&p1b, slot) {
                    for c in &p2vb {
                        m.push((SlotVal::Var((*c).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                for (_, v1) in p1slots.iter().filter(|(n, _)| n == slot) {
                    for (_, v2) in slots2.iter().filter(|(n, _)| n == slot) {
                        if constant_value(v2) {
                            m.push((v1.clone(), v2.clone()));
                        } else {
                            m.push((v2.clone(), v1.clone()));
                        }
                    }
                }
            }
        }

        // --- MAP-GOAL-BUFFER, RHS `+` request to LHS `=` ----------------
        if matches!(buffer_usage(p1, "GOAL"), 4 | 12 | 13 | 44)
            && matches!(buffer_usage(p2, "GOAL"), 8 | 9 | 12 | 13 | 40 | 44)
        {
            let p1b = act_of(p1_acts, "GOAL", &['+']);
            let p2a = lhs_of(p2_lhs, "GOAL");
            let p2b = act_of(p2_acts, "GOAL", &['=', '*']);
            let slotsb = instantiate_slot_names(&p1b, b1);
            let slots2 = instantiate_slot_names(&p2a, b2);
            let slots2b = instantiate_slot_names(&p2b, b2);
            let vsb = var_slots(&p1b, b1);
            let vs2 = var_slots(&p2a, b2);
            let vs2b = var_slots(&p2b, b2);
            let mut interesting: Vec<String> = Vec::new();
            for src in [&slotsb, &slots2, &slots2b] {
                for (n, _) in src {
                    if !interesting.contains(n) {
                        interesting.push(n.clone());
                    }
                }
            }
            for slot in &interesting {
                let p1v: Vec<&String> = vsb
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                let p2v: Vec<&String> = vs2
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                let p2vb: Vec<&String> = vs2b
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                if !p1v.is_empty() && !p2v.is_empty() {
                    for a in &p1v {
                        for c in &p2v {
                            m.push((SlotVal::Var((*a).clone()), SlotVal::Var((*c).clone())));
                        }
                    }
                }
                if !p1v.is_empty() && !p2vb.is_empty() {
                    for a in &p1v {
                        for c in &p2vb {
                            m.push((SlotVal::Var((*a).clone()), SlotVal::Var((*c).clone())));
                        }
                    }
                }
                if !p1v.is_empty() && has_slot(&p2a, slot) {
                    for a in &p1v {
                        m.push((SlotVal::Var((*a).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p1v.is_empty() && has_slot(&p2b, slot) {
                    for a in &p1v {
                        m.push((SlotVal::Var((*a).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p2v.is_empty() && has_slot(&p1b, slot) {
                    for c in &p2v {
                        m.push((SlotVal::Var((*c).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p2vb.is_empty() && has_slot(&p1b, slot) {
                    for c in &p2vb {
                        m.push((SlotVal::Var((*c).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                for (_, v1) in slotsb.iter().filter(|(n, _)| n == slot) {
                    for (_, v2) in slots2.iter().filter(|(n, _)| n == slot) {
                        if constant_value(v2) {
                            m.push((v1.clone(), v2.clone()));
                        } else {
                            m.push((v2.clone(), v1.clone()));
                        }
                    }
                }
            }
        }
        // --- MAP-IMAGINAL-BUFFER, RHS `+` request to LHS `=` ------------
        if matches!(u1i, 4 | 12 | 13 | 20 | 28 | 29)
            && matches!(u2i, 8 | 9 | 12 | 13 | 24 | 25 | 28 | 29 | 40 | 56)
        {
            let p1b = act_of(p1_acts, "IMAGINAL", &['+']);
            let p2a = lhs_of(p2_lhs, "IMAGINAL");
            let p2b = act_of(p2_acts, "IMAGINAL", &['=', '*']);
            let slotsb = instantiate_slot_names(&p1b, b1);
            let slots2 = instantiate_slot_names(&p2a, b2);
            let slots2b = instantiate_slot_names(&p2b, b2);
            let vsb = var_slots(&p1b, b1);
            let vs2 = var_slots(&p2a, b2);
            let vs2b = var_slots(&p2b, b2);
            let mut interesting: Vec<String> = Vec::new();
            for src in [&slotsb, &slots2, &slots2b] {
                for (n, _) in src {
                    if !interesting.contains(n) {
                        interesting.push(n.clone());
                    }
                }
            }
            for slot in &interesting {
                let p1v: Vec<&String> = vsb
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                let p2v: Vec<&String> = vs2
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                let p2vb: Vec<&String> = vs2b
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                if !p1v.is_empty() && !p2v.is_empty() {
                    for a in &p1v {
                        for c in &p2v {
                            m.push((SlotVal::Var((*a).clone()), SlotVal::Var((*c).clone())));
                        }
                    }
                }
                if !p1v.is_empty() && !p2vb.is_empty() {
                    for a in &p1v {
                        for c in &p2vb {
                            m.push((SlotVal::Var((*a).clone()), SlotVal::Var((*c).clone())));
                        }
                    }
                }
                if !p1v.is_empty() && has_slot(&p2a, slot) {
                    for a in &p1v {
                        m.push((SlotVal::Var((*a).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p1v.is_empty() && has_slot(&p2b, slot) {
                    for a in &p1v {
                        m.push((SlotVal::Var((*a).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p2v.is_empty() && has_slot(&p1b, slot) {
                    for c in &p2v {
                        m.push((SlotVal::Var((*c).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p2vb.is_empty() && has_slot(&p1b, slot) {
                    for c in &p2vb {
                        m.push((SlotVal::Var((*c).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                for (_, v1) in slotsb.iter().filter(|(n, _)| n == slot) {
                    for (_, v2) in slots2.iter().filter(|(n, _)| n == slot) {
                        if constant_value(v2) {
                            m.push((v1.clone(), v2.clone()));
                        } else {
                            m.push((v2.clone(), v1.clone()));
                        }
                    }
                }
            }
        } // --- MAP-IMAGINAL-BUFFER, LHS `=` with null RHS to LHS `=` ------
        if matches!(u1i, 8 | 24) && matches!(u2i, 8 | 9 | 12 | 13 | 24 | 25 | 28 | 29 | 40 | 56) {
            let p1a = lhs_of(p1_lhs, "IMAGINAL");
            let p2a = lhs_of(p2_lhs, "IMAGINAL");
            let p1slots = instantiate_slot_names(&p1a, b1);
            let p2slots = instantiate_slot_names(&p2a, b2);
            let mut interesting: Vec<String> = Vec::new();
            for (n, _) in &p1slots {
                if p2slots.iter().any(|(n2, _)| n2 == n) && !interesting.contains(n) {
                    interesting.push(n.clone());
                }
            }
            for slot in &interesting {
                for (_, v1) in p1slots.iter().filter(|(n, _)| n == slot) {
                    for (_, v2) in p2slots.iter().filter(|(n, _)| n == slot) {
                        if constant_value(v2) {
                            if self.cfg.ppm.is_some() && !constant_value(v1) {
                                if let SlotVal::Var(n) = v1 {
                                    let bound = b1
                                        .get(n)
                                        .or_else(|| b1.get(&format!("={n}")))
                                        .or_else(|| b2.get(n))
                                        .or_else(|| b2.get(&format!("={n}")));
                                    if let Some(val) = bound {
                                        m.push((v1.clone(), SlotVal::Lit(val.show())));
                                    }
                                }
                            } else {
                                m.push((v1.clone(), v2.clone()));
                            }
                        } else {
                            m.push((v2.clone(), v1.clone()));
                        }
                    }
                }
            }
        }
        // --- MAP-PERCEPTUAL-BUFFER, LHS `=` to LHS `=` -------------------
        for buf in [
            "VISUAL-LOCATION",
            "VISUAL",
            "AURAL-LOCATION",
            "AURAL",
            "TEMPORAL",
        ] {
            let u1p = buffer_usage(p1, buf);
            let u2p = buffer_usage(p2, buf);
            if !matches!(u1p, 8 | 24) || !matches!(u2p, 8 | 12 | 24 | 28) {
                continue;
            }
            let p1a = lhs_of(p1_lhs, buf);
            let p2a = lhs_of(p2_lhs, buf);
            let p1slots = instantiate_slot_names(&p1a, b1);
            let p2slots = instantiate_slot_names(&p2a, b2);
            let mut interesting: Vec<String> = Vec::new();
            for (n, _) in &p1slots {
                if p2slots.iter().any(|(n2, _)| n2 == n) && !interesting.contains(n) {
                    interesting.push(n.clone());
                }
            }
            for slot in &interesting {
                for (_, v1) in p1slots.iter().filter(|(n, _)| n == slot) {
                    for (_, v2) in p2slots.iter().filter(|(n, _)| n == slot) {
                        if constant_value(v2) {
                            if self.cfg.ppm.is_some() && !constant_value(v1) {
                                if let SlotVal::Var(n) = v1 {
                                    let bound = b1
                                        .get(n)
                                        .or_else(|| b1.get(&format!("={n}")))
                                        .or_else(|| b2.get(n))
                                        .or_else(|| b2.get(&format!("={n}")));
                                    if let Some(val) = bound {
                                        m.push((v1.clone(), SlotVal::Lit(val.show())));
                                    }
                                }
                            } else {
                                m.push((v1.clone(), v2.clone()));
                            }
                        } else {
                            m.push((v2.clone(), v1.clone()));
                        }
                    }
                }
            }
        }
        // --- MAP-GOAL-BUFFER, RHS `=` to LHS `=` ------------------------
        let u1g = buffer_usage(p1, "GOAL");
        let u2g = buffer_usage(p2, "GOAL");
        if (u1g == 9 && matches!(u2g, 8 | 9 | 12 | 13))
            || (u1g == 40 && matches!(u2g, 8 | 12 | 40 | 44))
        {
            let p1a = lhs_of(p1_lhs, "GOAL");
            let p1b = act_of(p1_acts, "GOAL", if u1g == 40 { &['*'] } else { &['='] });
            let p2a = lhs_of(p2_lhs, "GOAL");
            let p2b = act_of(p2_acts, "GOAL", &['=', '*']);
            let slotsa = instantiate_slot_names(&p1a, b1);
            let slotsb = instantiate_slot_names(&p1b, b1);
            let slots2 = instantiate_slot_names(&p2a, b2);
            let slots2b = instantiate_slot_names(&p2b, b2);
            let vsa = var_slots(&p1a, b1);
            let vsb = var_slots(&p1b, b1);
            let vs2 = var_slots(&p2a, b2);
            let vs2b = var_slots(&p2b, b2);
            let mut vs = vsa.clone();
            for e in &vsb {
                if !vs.contains(e) {
                    vs.push(e.clone());
                }
            }
            // p1's slots: the LHS minus the names its RHS supersedes, then the
            // RHS (`(append (remove-if … p1-slotsa) p1-slotsb)`).
            let mut p1slots: Vec<(String, SlotVal)> = slotsa
                .iter()
                .filter(|(n, _)| !slotsb.iter().any(|(x, _)| x == n))
                .cloned()
                .collect();
            p1slots.extend(slotsb.clone());
            let mut interesting: Vec<String> = Vec::new();
            for src in [&p1slots, &slots2, &slots2b] {
                for (n, _) in src.iter() {
                    if !interesting.contains(n) {
                        interesting.push(n.clone());
                    }
                }
            }
            for slot in &interesting {
                let p1v: Vec<&String> = vs
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                let p1vb: Vec<&String> = vsb
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                let p2v: Vec<&String> = vs2
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                let p2vb: Vec<&String> = vs2b
                    .iter()
                    .filter(|(_, t)| t == slot)
                    .map(|(v, _)| v)
                    .collect();
                if !p1v.is_empty() && !p2v.is_empty() {
                    for a in &p1v {
                        for c in &p2v {
                            m.push((SlotVal::Var((*a).clone()), SlotVal::Var((*c).clone())));
                        }
                    }
                }
                if !p1vb.is_empty() && !p2vb.is_empty() {
                    for a in &p1vb {
                        for c in &p2vb {
                            m.push((SlotVal::Var((*a).clone()), SlotVal::Var((*c).clone())));
                        }
                    }
                }
                if !p1v.is_empty() && has_slot(&p2a, slot) {
                    for a in &p1v {
                        m.push((SlotVal::Var((*a).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p1vb.is_empty() && has_slot(&p2b, slot) {
                    for a in &p1vb {
                        m.push((SlotVal::Var((*a).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p2v.is_empty() && (has_slot(&p1a, slot) || has_slot(&p1b, slot)) {
                    for c in &p2v {
                        m.push((SlotVal::Var((*c).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                if !p2vb.is_empty() && has_slot(&p1b, slot) {
                    for c in &p2vb {
                        m.push((SlotVal::Var((*c).clone()), SlotVal::Lit(slot.clone())));
                    }
                }
                for (_, v1) in p1slots.iter().filter(|(n, _)| n == slot) {
                    for (_, v2) in slots2.iter().filter(|(n, _)| n == slot) {
                        if constant_value(v2) {
                            // With `:ppm` on and a p1 *variable* facing a p2
                            // constant, the variable is instantiated from its
                            // own firing instead of mapping to p2's constant
                            // (`(push (find (spec-slot-value p1slots)
                            // bindings :key 'car))`).
                            if self.cfg.ppm.is_some() && !constant_value(v1) {
                                if let SlotVal::Var(n) = v1 {
                                    let bound = b1
                                        .get(n)
                                        .or_else(|| b1.get(&format!("={n}")))
                                        .or_else(|| b2.get(n))
                                        .or_else(|| b2.get(&format!("={n}")));
                                    if let Some(val) = bound {
                                        m.push((v1.clone(), SlotVal::Lit(val.show())));
                                    }
                                }
                            } else {
                                m.push((v1.clone(), v2.clone()));
                            }
                        } else {
                            m.push((v2.clone(), v1.clone()));
                        }
                    }
                }
            }
        }

        // --- MAP-GOAL-BUFFER, LHS `=` with null RHS to LHS `=` ----------
        if u1g == 8 && matches!(u2g, 8 | 9 | 12 | 13 | 40 | 44) {
            let p1a = lhs_of(p1_lhs, "GOAL");
            let p2a = lhs_of(p2_lhs, "GOAL");
            let p1slots = instantiate_slot_names(&p1a, b1);
            let p2slots = instantiate_slot_names(&p2a, b2);
            let mut interesting: Vec<String> = Vec::new();
            for (n, _) in &p1slots {
                if p2slots.iter().any(|(n2, _)| n2 == n) && !interesting.contains(n) {
                    interesting.push(n.clone());
                }
            }
            for slot in &interesting {
                for (_, v1) in p1slots.iter().filter(|(n, _)| n == slot) {
                    for (_, v2) in p2slots.iter().filter(|(n, _)| n == slot) {
                        if constant_value(v2) {
                            if self.cfg.ppm.is_some() && !constant_value(v1) {
                                if let SlotVal::Var(n) = v1 {
                                    let bound = b1
                                        .get(n)
                                        .or_else(|| b1.get(&format!("={n}")))
                                        .or_else(|| b2.get(n))
                                        .or_else(|| b2.get(&format!("={n}")));
                                    if let Some(val) = bound {
                                        m.push((v1.clone(), SlotVal::Lit(val.show())));
                                    }
                                }
                            } else {
                                m.push((v1.clone(), v2.clone()));
                            }
                        } else {
                            m.push((v2.clone(), v1.clone()));
                        }
                    }
                }
            }
        }

        // `(sort mappings #'< :key …)` — `sort` is stable for the ties that
        // matter here.
        m.sort_by_key(|(k, v)| match (constant_value(v), constant_value(k)) {
            (false, _) => 0,
            (true, true) => 1,
            (true, false) => 2,
        });
        m
    }

    /// Compose two productions per buffer (`buffer-condition-union`,
    /// production-compilation.lisp:2320): the first's conditions, then the
    /// second's conditions minus the slots the first modifies, with
    /// `remove-duplicates` keeping the later copy; the actions are the
    /// first's untouched slots followed by the second's.
    #[allow(clippy::type_complexity)]
    #[allow(clippy::type_complexity)]
    pub(super) fn compose_pair(
        &mut self,
        i1: usize,
        b1: &Bindings,
        reqs1: &[(String, String)],
        i2: usize,
        b2: &Bindings,
        retain1: &[String],
        retain2: &[String],
    ) -> (
        Vec<(String, Vec<(String, SlotVal)>)>,
        Vec<(String, Vec<(String, String)>)>,
        Vec<String>,
    ) {
        let p1_owned = self.model.productions[i1].clone();
        let p2_owned = self.model.productions[i2].clone();
        let p1 = &p1_owned;
        let p2 = &p2_owned;
        // `rename-variables` (`production-compilation.lisp:1945`): every basic
        // variable shared by the two productions is renamed in the second.
        // ACT-R does not exempt variables that bind the same buffer slot, and
        // it applies `replace-variables` to the whole representation, so we
        // generate fresh names even when the current bindings agree.
        let ren: HashMap<String, String> = {
            let v1 = basic_vars(p1);
            let v2 = basic_vars(p2);
            let mut used: HashSet<String> = v1.union(&v2).cloned().collect();
            let mut m = HashMap::new();
            let mut shared: Vec<String> = v1.intersection(&v2).cloned().collect();
            shared.sort();
            for v in shared {
                let fresh = loop {
                    let n = self.rename_counts.entry(v.clone()).or_insert(0);
                    let c = format!("{v}{n}");
                    *n += 1;
                    if !used.contains(&c) {
                        break c;
                    }
                };
                used.insert(fresh.clone());
                m.insert(v, fresh);
            }
            m
        };
        let rename = |n: &str| -> String { ren.get(n).cloned().unwrap_or_else(|| n.to_string()) };
        let b2r: Bindings = {
            let mut m = b2.clone();
            for (k, v) in &ren {
                if let Some(val) = b2.get(k) {
                    m.insert(v.clone(), val.clone());
                }
            }
            m
        };
        let b2: &Bindings = &b2r;
        let rename_slots = |slots: Vec<(String, SlotVal)>| -> Vec<(String, SlotVal)> {
            slots
                .into_iter()
                .map(|(n, v)| {
                    let n = match n.strip_prefix('=') {
                        Some(x) => format!("={}", rename(x)),
                        None => n,
                    };
                    let v = match v {
                        SlotVal::Var(x) => SlotVal::Var(rename(&x)),
                        other => other,
                    };
                    (n, v)
                })
                .collect()
        };
        // The first production's `!safe-eval!` keeps its variables: they are
        // still bound by the composed LHS. The second's are instantiated from
        // its firing (e.g. `=result` -> `(QUOTE LOSE)`).
        let p1_lhs = prod_lhs_slots(p1, &Bindings::new(), &self.model.chunk_types);
        let p2_lhs: Vec<(String, Vec<(String, SlotVal)>)> =
            prod_lhs_slots(p2, b2, &self.model.chunk_types)
                .into_iter()
                .map(|(b, s)| (b, rename_slots(s)))
                .collect();
        let p1_acts = prod_rhs_actions(p1, b1, &self.model.chunk_types);
        let p2_acts: Vec<(char, String, Vec<(String, SlotVal)>, Option<String>)> =
            prod_rhs_actions(p2, b2, &self.model.chunk_types)
                .into_iter()
                .map(|(op, buf, slots, var)| (op, buf, rename_slots(slots), var))
                .collect();
        let p2_outputs: Vec<Vec<Sexp>> = p2
            .rhs
            .iter()
            .filter_map(|a| match a {
                RhsAction::Output { args } => Some(
                    args.iter()
                        .map(|s| rename_sexp(s, &ren))
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .collect();

        // ACT-R's cross-production `mappings` (`compose-productions`,
        // `modules/production-compilation.lisp:1556`): applied to both
        // productions so the composed form — and the installed production the
        // engine parses back from the rendered text — substitutes exactly what
        // ACT-R does. A variable bound but never mapped stays `=VAR`.
        //
        // Only the `:ppm`-off path is ported: with `:ppm` on, the map
        // functions' slot sets are built from partially-matched specs with
        // extra instantiation (`buffer-condition-union`'s `bindings`), which
        // is not modelled here — the existing `bindings` fallback keeps those
        // traces as they were.
        let mappings = self.compose_mappings(p1, p2, &p1_lhs, &p1_acts, &p2_lhs, &p2_acts, b1, b2);
        // `compose-productions` removes duplicate LHS safe-binds: the second
        // production`s bind is kept and the first`s variable is rewritten to
        // it.  This is applied after the normal mappings, so it must bypass
        // the bind-variable guard below.
        let mut duplicate_bindings: Vec<(SlotVal, SlotVal)> = Vec::new();
        for (b1t, s1) in &p1_lhs {
            if b1t != "!SAFE-BIND!" {
                continue;
            }
            let Some((n1, v1)) = s1.first() else { continue };
            let Some(v1) = (match v1 {
                SlotVal::Lit(v) => Some(v),
                SlotVal::Var(_) => None,
            }) else {
                continue;
            };
            for (b2t, s2) in &p2_lhs {
                if b2t != "!SAFE-BIND!" {
                    continue;
                }
                let Some((n2, SlotVal::Lit(v2))) = s2.first() else {
                    continue;
                };
                if v1 == v2 {
                    if let (Some(v1), Some(v2)) = (n1.strip_prefix("="), n2.strip_prefix("=")) {
                        duplicate_bindings
                            .push((SlotVal::Var(v1.to_string()), SlotVal::Var(v2.to_string())));
                    }
                }
            }
        }
        // ACT-R applies the mappings one at a time, and after applying one it
        // rewrites every still-queued entry whose car equals the applied
        // entry's car to that entry's cdr (`(dolist (x mappings) (when (eq (car
        // x) (car mapping)) (replace … (cons (cdr mapping) (cdr x)))))`). That
        // is how `(=DEST . NUMBER)` becomes `(=OTHER . NUMBER)` after
        // `(=DEST . =OTHER)` is applied, so `=OTHER` resolves to `NUMBER`.
        let bind_vars: HashSet<String> = production_bind_vars(p1)
            .into_iter()
            .chain(production_bind_vars(p2))
            .collect();
        let mut action_bind_vars = bind_vars.clone();
        for (orig, fresh) in &ren {
            if bind_vars.contains(&orig.to_uppercase()) {
                action_bind_vars.insert(fresh.to_uppercase());
            }
        }
        let mut queue = mappings.clone();
        let mut applied: Vec<(SlotVal, SlotVal)> = Vec::new();
        for i in 0..queue.len() {
            let (k, mut v) = queue[i].clone();
            let SlotVal::Var(kv) = &k else {
                continue;
            };
            if bind_vars.contains(&kv.to_uppercase()) {
                continue;
            }
            // With `:ppm`, a mapping whose cdr is constant is replaced by the
            // variable's actual firing binding (`(awhen (find (car mapping)
            // bindings :key 'car) (setf mapping it))`). This is why a
            // partially matched p2 variable does not simply take p1's constant.
            if self.cfg.ppm.is_some() && constant_value(&v) {
                let bound = b1
                    .get(kv)
                    .or_else(|| b1.get(&format!("={kv}")))
                    .or_else(|| b2.get(kv))
                    .or_else(|| b2.get(&format!("={kv}")));
                if let Some(val) = bound {
                    v = SlotVal::Lit(val.show());
                }
            }
            applied.push((k.clone(), v.clone()));
            for later in queue.iter_mut().skip(i + 1) {
                if later.0 == k {
                    later.0 = v.clone();
                }
            }
        }
        applied.extend(duplicate_bindings.clone());
        // `replace-variables-special` applies the mappings to the reps one at
        // a time.  Replaying them for a single variable preserves the
        // composition order, including pairs that deliberately round-trip
        // (`=S0 -> =S` then `=S -> =S0`).
        let resolve = |var: &str| -> SlotVal {
            let mut cur = SlotVal::Var(var.to_string());
            for (k, v) in &applied {
                if &cur == k {
                    cur = v.clone();
                }
            }
            cur
        };
        // Substitute a slot *name* through the mappings (`replace-variables`
        // rewrites names and values alike).
        let sub_name = |n: &str| -> String {
            match n.strip_prefix('=') {
                Some(var) => match resolve(var) {
                    SlotVal::Var(w) => format!("={w}"),
                    SlotVal::Lit(t) => t,
                },
                None => n.to_string(),
            }
        };

        // Only variables referenced by an *indirect* request (`+buffer> =c`,
        // not `+goal> =retrieval`) are resolved from the firings; other
        // variables stay symbolic (compiled productions are not otherwise
        // specialized — cf. `ppm-compilation`). With `:rir`, a retained
        // retrieval request keeps its variable.
        let collect = |acts: &[(char, String, Vec<(String, SlotVal)>, Option<String>)],
                       retain: &[String]| {
            let mut s: HashSet<String> = HashSet::new();
            for (op, buf, _slots, var) in acts {
                if *op == 'r' {
                    if let Some(v) = var {
                        if !is_buffer_name(v) && !retain.contains(buf) {
                            s.insert(v.clone());
                        }
                    }
                }
            }
            s
        };
        let indirect_vars = {
            let mut s = collect(&p1_acts, retain1);
            s.extend(collect(&p2_acts, retain2));
            s
        };
        // `retrieval-buffers-map` (`production-compilation.lisp:297`): the
        // drop-out buffer variable (`=RETRIEVAL`) is instantiated to the
        // chunk it held when the first production requested that buffer;
        // other buffer variables (`=VISUAL-LOCATION`, ...) stay symbolic.
        let drop_retrieval = p1_acts
            .iter()
            .any(|(op, buf, _, _)| (*op == '+' || *op == 'r') && buf == "RETRIEVAL");
        // A variable not unified across the pair, but resolved by an
        // indirect action, is instantiated from the firing bindings.
        let subst = |v: &SlotVal, b: &Bindings, ivars: &HashSet<String>| -> SlotVal {
            match v {
                SlotVal::Var(n) => {
                    let resolved = resolve(n);
                    let resolved_name = match &resolved {
                        SlotVal::Var(w) => Some(w),
                        SlotVal::Lit(_) => None,
                    };
                    if ivars.contains(n) && b.contains_key(n) {
                        SlotVal::Lit(b[n].show())
                    } else if let Some(w) = resolved_name {
                        if ivars.contains(w) && b.contains_key(w) {
                            SlotVal::Lit(b[w].show())
                        } else {
                            resolved
                        }
                    } else if resolved != SlotVal::Var(n.clone()) {
                        resolved
                    } else if drop_retrieval && n == "RETRIEVAL" {
                        self.buffers
                            .get(n)
                            .and_then(|buf| buf.chunk.as_ref())
                            .and_then(|c| c.name.clone())
                            .map(SlotVal::Lit)
                            .unwrap_or_else(|| SlotVal::Var(n.clone()))
                    } else {
                        SlotVal::Var(n.clone())
                    }
                }
                SlotVal::Lit(s) => SlotVal::Lit(s.clone()),
            }
        };
        let apply_lhs =
            |lhs: &[(String, Vec<(String, SlotVal)>)], b: &Bindings, ivars: &HashSet<String>| {
                lhs.iter()
                    .map(|(buf, s)| {
                        (
                            buf.clone(),
                            s.iter()
                                .map(|(n, v)| (n.clone(), subst(v, b, ivars)))
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
        // `replace-variables` rewrites a variable slot *name* as well as a
        // value; a `!`-condition uses `replace-variables-for-eval` instead, so
        // its slot names are left alone.
        let sub_lhs = |v: Vec<(String, Vec<(String, SlotVal)>)>| {
            v.into_iter()
                .map(|(b, s): (String, Vec<(String, SlotVal)>)| {
                    let s = if b.starts_with('!') {
                        s
                    } else {
                        s.into_iter().map(|(n, val)| (sub_name(&n), val)).collect()
                    };
                    (b, s)
                })
                .collect::<Vec<_>>()
        };
        let sub_acts = |v: Vec<(char, String, Vec<(String, SlotVal)>, Option<String>)>| {
            v.into_iter()
                .map(
                    |(op, b, s, var): (char, String, Vec<(String, SlotVal)>, Option<String>)| {
                        let s = s
                            .into_iter()
                            .map(|(n, val)| {
                                let keep = n
                                    .strip_prefix('=')
                                    .is_some_and(|v| action_bind_vars.contains(&v.to_uppercase()));
                                (if keep { n } else { sub_name(&n) }, val)
                            })
                            .collect();
                        (op, b, s, var)
                    },
                )
                .collect::<Vec<_>>()
        };
        let p1_lhs = sub_lhs(apply_lhs(&p1_lhs, b1, &indirect_vars));
        let p2_lhs = sub_lhs(apply_lhs(&p2_lhs, b2, &indirect_vars));
        let inst_acts = |acts: &[(char, String, Vec<(String, SlotVal)>, Option<String>)],
                         b: &Bindings,
                         ivars: &HashSet<String>| {
            acts.iter()
                .map(|(op, buf, slots, var)| {
                    (
                        *op,
                        buf.clone(),
                        slots
                            .iter()
                            .map(|(n, v)| (n.clone(), subst(v, b, ivars)))
                            .collect::<Vec<_>>(),
                        var.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let p1_request_vars: HashSet<String> = p1_acts
            .iter()
            .filter(|(op, _, _, _)| matches!(*op, '+' | 'r'))
            .flat_map(|(_, _, slots, _)| slots.iter())
            .filter_map(|(_, v)| match v {
                SlotVal::Var(n) => Some(n.clone()),
                _ => None,
            })
            .collect();
        let p1_acts = sub_acts(inst_acts(&p1_acts, b1, &indirect_vars));
        let p2_acts_inst: Vec<(char, String, Vec<(String, SlotVal)>, Option<String>)> = p2_acts
            .iter()
            .map(|(op, buf, slots, var)| {
                (
                    *op,
                    buf.clone(),
                    slots
                        .iter()
                        .map(|(n, v)| {
                            let v = match v {
                                SlotVal::Var(name)
                                    if p1_request_vars.contains(name) && b2.contains_key(name) =>
                                {
                                    SlotVal::Lit(b2[name].show())
                                }
                                _ => subst(v, b2, &indirect_vars),
                            };
                            (n.clone(), v)
                        })
                        .collect(),
                    var.clone(),
                )
            })
            .collect();
        let p2_acts = sub_acts(p2_acts_inst);

        let mut buffers: Vec<String> = Vec::new();
        let add_buf = |buffers: &mut Vec<String>, b: &str| {
            if !buffers.iter().any(|x| x == b) {
                buffers.push(b.to_string());
            }
        };
        for (b, _) in p1_lhs.iter().chain(&p2_lhs) {
            if !b.starts_with('?') && !b.starts_with('!') {
                add_buf(&mut buffers, b);
            }
        }
        for (_, b, _, _) in p1_acts.iter().chain(&p2_acts) {
            add_buf(&mut buffers, b);
        }
        buffers.sort();

        let lhs_for = |v: &[(String, Vec<(String, SlotVal)>)], b: &str| -> Vec<(String, SlotVal)> {
            v.iter()
                .find(|(x, _)| x == b)
                .map(|(_, s)| s.clone())
                .unwrap_or_default()
        };
        let acts_for = |v: &[(char, String, Vec<(String, SlotVal)>, Option<String>)],
                        b: &str,
                        ops: &[char]|
         -> Vec<(Vec<(String, SlotVal)>, Option<String>)> {
            v.iter()
                .filter(|(op, bb, _, _)| bb == b && ops.contains(op))
                .map(|(_, _, s, var)| (s.clone(), var.clone()))
                .collect()
        };

        let mut lhs = Vec::new();
        for buf in &buffers {
            let c1 = lhs_for(&p1_lhs, buf);
            let c2 = lhs_for(&p2_lhs, buf);
            let mut modified: Vec<String> = Vec::new();
            for (s, _) in acts_for(&p1_acts, buf, &['=', '*']) {
                for (n, _) in instantiate_slot_names(&s, b1) {
                    modified.push(n);
                }
            }
            let p1_req = p1_acts
                .iter()
                .any(|(op, bb, _, _)| bb == buf && (*op == '+' || *op == 'r'));
            if p1_req {
                // The request supplies the buffer; drop the second's tests,
                // but keep p1's own condition even if it has no slots
                // (`=retrieval>`).
                if p1_lhs.iter().any(|(b, _)| b == buf) {
                    lhs.push((format!("={buf}>"), remove_dup_keep_last(c1)));
                }
            } else {
                let mut slots = c1;
                for (n, v) in c2 {
                    let resolved = if n.starts_with('=') {
                        bound_text(&n, b2).unwrap_or_else(|| n.clone())
                    } else {
                        n.clone()
                    };
                    if !modified.contains(&resolved) && !modified.contains(&n) {
                        slots.push((n, v));
                    }
                }
                let had =
                    p1_lhs.iter().any(|(b, _)| b == buf) || p2_lhs.iter().any(|(b, _)| b == buf);
                if had {
                    lhs.push((format!("={buf}>"), remove_dup_keep_last(slots)));
                }
            }
        }
        // Query conditions (`?buffer> state free`) follow the matches, in
        // ACT-R buffer-index order.
        let mut queries: Vec<(String, Vec<(String, SlotVal)>)> = Vec::new();
        for (b, s) in p1_lhs.iter().chain(&p2_lhs) {
            if b.starts_with('?')
                && !lhs.iter().any(|(t, _)| t == b)
                && !queries.iter().any(|(t, _)| t == b)
            {
                queries.push((b.clone(), remove_dup_keep_last(s.clone())));
            }
        }
        queries.sort_by_key(|(b, _)| {
            buffer_sort_index(b.trim_start_matches('?').trim_end_matches('>'))
        });
        lhs.extend(queries);
        // `!bind!`/`!safe-bind!` conditions are prepended
        // (`post-conditions`, production-compilation.lisp:1742).
        let duplicate_bind_vars: HashSet<String> = duplicate_bindings
            .iter()
            .filter_map(|(k, _)| match k {
                SlotVal::Var(v) => Some(v.clone()),
                SlotVal::Lit(_) => None,
            })
            .collect();
        let mut binds: Vec<(String, Vec<(String, SlotVal)>)> = Vec::new();
        // ACT-R builds `p3-s` with p2`s binds first, then p1`s non-duplicate
        // binds.  The trace printer pushes and therefore reverses them.
        for (b, s) in &p2_lhs {
            if b.starts_with('!') {
                binds.push((b.clone(), s.clone()));
            }
        }
        for (b, s) in &p1_lhs {
            let duplicate = b == "!SAFE-BIND!"
                && s.first()
                    .and_then(|(n, _)| n.strip_prefix('='))
                    .is_some_and(|v| duplicate_bind_vars.contains(v));
            if b.starts_with('!') && !duplicate {
                binds.push((b.clone(), s.clone()));
            }
        }
        // `compose-productions` keeps the second copy of a duplicate bind.
        // The later entry is also the renamed/p2-backed form.
        let mut binds = remove_dup_keep_last(binds);
        if !binds.is_empty() {
            binds.extend(lhs);
            lhs = binds;
        }

        let mut rhs: Vec<(String, Vec<(String, String)>)> = Vec::new();
        let mut dropped: Vec<String> = Vec::new();
        for buf in &buffers {
            let p2_tests = p2_lhs.iter().any(|(b, _)| b == buf);
            let sv = |s: Vec<(String, SlotVal)>| -> Vec<(String, String)> {
                s.iter().map(|(n, v)| (n.clone(), v.render())).collect()
            };
            // `COMPOSE-*-BUFFER` request-plus-modify case (`(4 12 13 44)`):
            // p1 requests the buffer and p2 modifies it, so the composed
            // action is p1's `+` with p2's `=`/`*` slots merged in
            // (`buffer+-union`: p1's non-matching slots then p2's, sorted).
            let p1_req = acts_for(&p1_acts, buf, &['+', 'r']);
            let p2_eq = acts_for(&p2_acts, buf, &['=', '*']);
            if !p1_req.is_empty()
                && !p2_eq.is_empty()
                && acts_for(&p2_acts, buf, &['+', 'r']).is_empty()
            {
                // `a1=`/`a1*`: p1's own `=`/`*` modify is emitted first,
                // then the `+` union.
                let p1_eq: Vec<(String, SlotVal)> = acts_for(&p1_acts, buf, &['='])
                    .into_iter()
                    .flat_map(|(s, _)| s)
                    .collect();
                let p1_star: Vec<(String, SlotVal)> = acts_for(&p1_acts, buf, &['*'])
                    .into_iter()
                    .flat_map(|(s, _)| s)
                    .collect();
                if !p1_eq.is_empty() {
                    rhs.push((format!("={buf}>"), sv(p1_eq)));
                } else if !p1_star.is_empty() {
                    rhs.push((format!("*{buf}>"), sv(p1_star)));
                }
                let mut slots: Vec<(String, SlotVal)> =
                    p1_req.into_iter().flat_map(|(s, _)| s).collect();
                let mut eq: Vec<(String, SlotVal)> =
                    p2_eq.into_iter().flat_map(|(s, _)| s).collect();
                eq.sort_by(|a, b| a.0.cmp(&b.0));
                let names2: Vec<String> = eq.iter().map(|(n, _)| n.clone()).collect();
                let p2_effective: Vec<String> = eq
                    .iter()
                    .map(|(n, _)| {
                        if n.starts_with('=') {
                            bound_text(n, b2).unwrap_or_else(|| n.clone())
                        } else {
                            n.clone()
                        }
                    })
                    .collect();
                slots.retain(|(n, _)| {
                    !names2.contains(n)
                        && !p2_effective.contains(n)
                        && !(n.starts_with('=')
                            && bound_text(n, b1).is_some_and(|t| p2_effective.contains(&t)))
                });
                slots.extend(eq);
                rhs.push((format!("+{buf}>"), sv(slots)));
                continue;
            }
            // `=` modify union (second overrides). A null `=buffer>` modify
            // is kept (`COMPOSE-GOAL-BUFFER`'s `(when a2= (list a2=))`), so a
            // self-composition like `G2 & G2` matches `G2`.
            let has_eq1 = !acts_for(&p1_acts, buf, &['=']).is_empty();
            let has_eq2 = !acts_for(&p2_acts, buf, &['=']).is_empty();
            // The composed slots come from each production's *standard rep*,
            // whose slots are sorted (`sort-production-slots`,
            // `production-compilation.lisp:1976`), so sort each side before
            // concatenating (`buffer+-union`'s `(append a1-remain a2)`).
            let eq1: Vec<_> = acts_for(&p1_acts, buf, &['='])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            let eq2: Vec<_> = acts_for(&p2_acts, buf, &['='])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            let st1: Vec<_> = acts_for(&p1_acts, buf, &['*'])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            let st2: Vec<_> = acts_for(&p2_acts, buf, &['*'])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            // `buffer=-union` when the first modifies-by-request and the second
            // modifies: the union is a single `*` (`COMPOSE-IMAGINAL-BUFFER`
            // `(and a1* a2=)`), not a separate `=` plus `*`.
            if !st1.is_empty() && has_eq2 && st2.is_empty() {
                let names2: Vec<String> = eq2.iter().map(|(n, _)| n.clone()).collect();
                let mut slots: Vec<(String, SlotVal)> = st1
                    .into_iter()
                    .filter(|(n, _)| !names2.contains(n))
                    .collect();
                slots.extend(eq2);
                rhs.push((format!("*{buf}>"), sv(slots)));
                continue;
            }
            let effective_names = |slots: &[(String, SlotVal)], b: &Bindings| -> Vec<String> {
                slots
                    .iter()
                    .map(|(n, _)| {
                        if n.starts_with('=') {
                            bound_text(n, b).unwrap_or_else(|| n.clone())
                        } else {
                            n.clone()
                        }
                    })
                    .collect()
            };
            if has_eq1 || has_eq2 {
                let names2: Vec<String> = eq2.iter().map(|(n, _)| n.clone()).collect();
                let p2_effective = effective_names(&eq2, b2);
                let mut slots: Vec<(String, SlotVal)> = eq1
                    .into_iter()
                    .filter(|(n, _)| {
                        !names2.contains(n)
                            && !p2_effective.contains(n)
                            && !(n.starts_with('=')
                                && bound_text(n, b1).is_some_and(|t| p2_effective.contains(&t)))
                    })
                    .collect();
                slots.extend(eq2);
                rhs.push((format!("={buf}>"), sv(slots)));
            }
            // `*` modify-request union.
            if !st1.is_empty() || !st2.is_empty() {
                let names2: Vec<String> = st2.iter().map(|(n, _)| n.clone()).collect();
                let p2_effective = effective_names(&st2, b2);
                let mut slots: Vec<(String, SlotVal)> = st1
                    .into_iter()
                    .filter(|(n, _)| {
                        !names2.contains(n)
                            && !p2_effective.contains(n)
                            && !(n.starts_with('=')
                                && bound_text(n, b1).is_some_and(|t| p2_effective.contains(&t)))
                    })
                    .collect();
                slots.extend(st2);
                rhs.push((format!("*{buf}>"), sv(slots)));
            }
            // `+` requests: p1's is dropped when p2 tests the buffer (and
            // the requested chunk is noted); p2's indirect ones are
            // resolved from p1's requests and expanded to slots.
            // p1 requests: kept unless p2 tests the buffer; an indirect
            // request (`+retrieval> =c`) expands to the referenced chunk's
            // spec.
            let p2_reqs_buf = p2_acts
                .iter()
                .any(|(op, bb, _, _)| bb == buf && (*op == '+' || *op == 'r'));
            for (s, var) in acts_for(&p1_acts, buf, &['+', 'r']) {
                if p2_tests {
                    if reqs1.iter().any(|(bb, _)| bb == buf) {
                        // The dropped name is the chunk p1's request actually
                        // produced (e.g. the retrieved `G`, not the requested
                        // `red`), i.e. the buffer's current content.
                        let name = self
                            .buffers
                            .get(buf)
                            .and_then(|b| b.chunk.as_ref())
                            .and_then(|c| c.name.clone())
                            .map(|n| self.copy_source.get(&n).cloned().unwrap_or(n))
                            .or_else(|| {
                                reqs1
                                    .iter()
                                    .find(|(bb, _)| bb == buf)
                                    .map(|(_, n)| n.clone())
                            });
                        if let Some(n) = name {
                            dropped.push(n);
                        }
                    }
                } else if p2_reqs_buf {
                    // Both productions request the buffer; p1's request is
                    // superseded by p2's (no dropped-chunk note).
                } else if retain1.contains(buf) {
                    // `:rir`: keep the indirect retrieval request as its
                    // variable (`+RETRIEVAL> =C`), or its instantiated value
                    // when the variable is also being resolved.
                    if let Some(v) = &var {
                        let shown = if indirect_vars.contains(v) && b1.contains_key(v) {
                            b1[v].show()
                        } else {
                            format!("={v}")
                        };
                        rhs.push((format!("+{buf}> {shown}"), Vec::new()));
                    }
                } else {
                    let slots = match &var {
                        Some(v) => self
                            .binding_chunk_name(v, b1)
                            .map(|n| self.chunk_spec_slots(&n))
                            .unwrap_or_default(),
                        None => sv(s),
                    };
                    rhs.push((format!("+{buf}>"), slots));
                }
            }
            // p2 requests.
            for (s, var) in acts_for(&p2_acts, buf, &['+', 'r']) {
                if retain2.contains(buf) {
                    if let Some(v) = &var {
                        let shown = if indirect_vars.contains(v) && b2.contains_key(v) {
                            b2[v].show()
                        } else {
                            format!("={v}")
                        };
                        rhs.push((format!("+{buf}> {shown}"), Vec::new()));
                        continue;
                    }
                }
                if let Some(v) = var {
                    let slots = if let Some(chunk) = reqs1
                        .iter()
                        .find(|(bb, _)| bb == &v || bb == buf)
                        .map(|(_, c)| c.clone())
                    {
                        self.chunk_spec_slots(&chunk)
                    } else {
                        self.binding_chunk_name(&v, b2)
                            .map(|n| self.chunk_spec_slots(&n))
                            .unwrap_or_default()
                    };
                    rhs.push((format!("+{buf}>"), slots));
                } else {
                    rhs.push((format!("+{buf}>"), sv(s)));
                }
            }
        }
        // Composed `!output!` actions use the mappings by replaying them over
        // the referenced variables; non-variable arguments keep their printed
        // Lisp form.
        let mut outputs: Vec<Vec<Sexp>> = Vec::new();
        for a in &p1.rhs {
            if let RhsAction::Output { args } = a {
                outputs.push(args.clone());
            }
        }
        outputs.extend(p2_outputs);
        for args in outputs {
            let text = args
                .iter()
                .map(|a| match a {
                    Sexp::Sym(t) if t.starts_with('=') && t.len() > 1 => match resolve(&t[1..]) {
                        SlotVal::Lit(s) => s,
                        SlotVal::Var(v) => format!("={v}"),
                    },
                    other => sexp_lisp(other),
                })
                .collect::<Vec<_>>()
                .join(" ");
            rhs.push(("!OUTPUT!".to_string(), vec![(String::new(), text)]));
        }
        // `sort-actions`: descending by token (op+buffer), stable.
        rhs.sort_by(|a, b| b.0.cmp(&a.0));
        (lhs, rhs, dropped)
    }

    pub(super) fn similarity(&self, a: &Value, b: &Value) -> f32 {
        // An embedder hook (the Python `:sim-hook`) gets the printed values.
        if let Some(hook) = &self.sim_hook_fn {
            if let Some(v) = hook(
                &crate::engine::HookValue::from_value(a),
                &crate::engine::HookValue::from_value(b),
            ) {
                return v;
            }
        }
        // `:sim-hook` (`declarative-memory.lisp:2218`): a task function's
        // numeric result overrides the similarity table and `:ms`/`:md`
        // (`chunks-similarity`, `declarative-memory.lisp:2628`). A non-number
        // (or an unknown function) falls through to the table.
        if let Some(hook) = &self.cfg.sim_hook {
            if let Some(v) = self.call_user_fn(hook, &[a.clone(), b.clone()]) {
                if let Some(n) = match &v {
                    Value::Int(i) => Some(*i as f64),
                    Value::Num(x) => Some(*x as f64),
                    _ => None,
                } {
                    return n as f32;
                }
            }
        }
        let a_name = match a {
            Value::Sym(s) => Some(self.resolve_alias(s)),
            _ => None,
        };
        let b_name = match b {
            Value::Sym(s) => Some(self.resolve_alias(s)),
            _ => None,
        };
        // Explicit similarities are keyed by the (symbol) values, not by DM
        // membership: ACT-R registers slot values as undefined chunks, so
        // `set-similarities (pink red -.4)` applies to plain symbols. Buffer
        // copies (`ONE-0`) compare as their source chunk (`ONE`).
        if let (Some(x), Some(y)) = (a_name, b_name) {
            if let Some(v) = self
                .sims
                .get(&(x.clone(), y.clone()))
                .or_else(|| self.sims.get(&(y, x)))
            {
                return *v;
            }
        }
        if self.value_eq(a, b) {
            self.cfg.ms
        } else {
            self.cfg.md
        }
    }

    /// The chunk name bound to a variable (for an indirect request).
    pub(super) fn binding_chunk_name(&self, var: &str, b: &Bindings) -> Option<String> {
        match b.get(var) {
            Some(Value::Sym(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// `chunk-name-to-chunk-spec`: a chunk's slots; for an undefined chunk
    /// (a bare-symbol slot value like `red`), the slot(s) that reference it
    /// (`add-dm`'s `get-true-slot-value-name` gives the chunk that slot).
    pub(super) fn chunk_spec_slots(&self, name: &str) -> Vec<(String, String)> {
        if let Some(c) = self
            .dm
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .or_else(|| self.named.get(name))
        {
            return c.slots.iter().map(|(k, v)| (k.clone(), v.show())).collect();
        }
        let mut out = Vec::new();
        for c in self.dm.iter() {
            for (k, v) in &c.slots {
                if let Value::Sym(s) = v {
                    if s == name && !out.contains(&(k.clone(), name.to_string())) {
                        out.push((k.clone(), name.to_string()));
                    }
                }
            }
        }
        out
    }

    /// For a whole-chunk request (`+goal> =retrieval`), the trace shows the
    /// referenced chunk's slots (`(COLOR RED VALUE 3)`), sorted by name.
    pub(super) fn named_chunk_shown(&self, spec: &NewChunk) -> Option<String> {
        if let NewChunk::Named(n) = spec {
            if let Some(c) = self
                .dm
                .iter()
                .find(|c| c.name.as_deref() == Some(n.as_str()))
                .or_else(|| self.named.get(n.as_str()))
            {
                let mut slots: Vec<(String, String)> =
                    c.slots.iter().map(|(k, v)| (k.clone(), v.show())).collect();
                slots.sort_by(|a, b| a.0.cmp(&b.0));
                let mut parts = Vec::new();
                for (k, v) in slots {
                    parts.push(k);
                    parts.push(v);
                }
                return Some(format!("({})", parts.join(" ")));
            }
        }
        None
    }

    /// `get-chunk`: a chunk by name from DM, the model's named chunks, or a
    /// buffer (a `reuse?` copy like `GOAL-CHUNK0`, `buffers.lisp:1349`).
    pub(super) fn find_chunk_by_name(&self, name: &str) -> Option<&ChunkInst> {
        self.dm
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .or_else(|| self.named.get(name))
            .or_else(|| {
                self.buffers
                    .values()
                    .filter_map(|b| b.chunk.as_ref())
                    .find(|c| c.name.as_deref() == Some(name))
            })
    }

    pub(super) fn start_retrieval(
        &mut self,
        spec: NewChunk,
        var_slots: &[String],
    ) -> Result<(), EngineError> {
        // A whole-chunk request (`+retrieval> =goal`) expands the referenced
        // chunk to its slot spec (`chunk-name-to-chunk-spec`) and retrieves
        // by matching; a name that is not a chunk (e.g. `=c` bound to a
        // bare-symbol slot value like `red`) expands to an unconstrained
        // request matching any chunk.
        if let NewChunk::Named(n) = &spec {
            let chunk = self.find_chunk_by_name(n);
            let expanded = match chunk {
                Some(c) => NewChunk::Spec {
                    ctype: String::new(),
                    slots: c.slots.clone(),
                    neg: Vec::new(),
                    rels: Vec::new(),
                    bare: true,
                },
                None => NewChunk::Spec {
                    ctype: String::new(),
                    slots: Vec::new(),
                    neg: Vec::new(),
                    rels: Vec::new(),
                    bare: true,
                },
            };
            return self.start_retrieval(expanded, var_slots);
        }
        // A spec whose "type" names a DM chunk (e.g. `(b :mp-value 2)`)
        // expands to that chunk's slots plus the request parameters
        // (`define-chunk-spec-fct` with a chunk name, chunk-spec.lisp:437).
        if let NewChunk::Spec {
            ctype,
            slots,
            neg,
            rels,
            ..
        } = &spec
        {
            if !ctype.is_empty()
                && !self
                    .model
                    .chunk_types
                    .iter()
                    .any(|t| t.name.eq_ignore_ascii_case(ctype))
            {
                if let Some(chunk) = self
                    .dm
                    .iter()
                    .find(|c| c.name.as_deref() == Some(ctype.as_str()))
                {
                    let mut merged = chunk.slots.clone();
                    merged.extend(slots.clone());
                    let expanded = NewChunk::Spec {
                        ctype: String::new(),
                        slots: merged,
                        neg: neg.clone(),
                        rels: rels.clone(),
                        bare: true,
                    };
                    return self.start_retrieval(expanded, var_slots);
                }
            }
        }
        let (mut want_slots, want_neg) = match &spec {
            NewChunk::Named(_) => unreachable!(),
            NewChunk::Instance(_) => {
                return Err(bad(
                    "retrieval request cannot be a buffer instance".to_string()
                ));
            }
            NewChunk::Spec { slots, neg, .. } => (slots.clone(), neg.clone()),
        };
        if std::env::var_os("RETR_DEBUG").is_some() {
            eprintln!(
                "REQ ctype={:?} slots={want_slots:?} neg={want_neg:?}",
                match &spec {
                    NewChunk::Spec { ctype, .. } => ctype.clone(),
                    _ => String::new(),
                }
            );
        }
        // `isa TYPE` (`define-chunk-spec-fct`, chunk-spec.lisp:441): the
        // chunk-type's default slot tests are added for any slot not already
        // tested (`process-slots-specs`, chunk-spec.lisp:597) — e.g.
        // `isa response` on `(chunk-type response key (is-response t))`
        // requests `is-response = t`.
        if let NewChunk::Spec { ctype, .. } = &spec {
            if !ctype.is_empty() {
                let defaults = self
                    .model
                    .chunk_types
                    .iter()
                    .find(|t| t.name.eq_ignore_ascii_case(ctype))
                    .map(|t| t.defaults.clone())
                    .unwrap_or_default();
                for (slot, pat) in defaults {
                    if !want_slots
                        .iter()
                        .any(|(s, _)| s.eq_ignore_ascii_case(&slot))
                    {
                        want_slots.push((slot.to_uppercase(), Value::from_pattern(&pat)?));
                    }
                }
            }
        }
        // `:mp-value` request parameter (`+retrieval> isa test :mp-value 3`)
        // overrides `:mp` for this request; `nil` disables partial matching.
        let mp_req = want_slots
            .iter()
            .find(|(s, _)| s == ":MP-VALUE")
            .map(|(_, v)| v.clone());
        let mp = match &mp_req {
            Some(Value::Nil) => None,
            Some(Value::Num(x)) => Some(*x as f32),
            Some(Value::Int(i)) => Some(*i as f32),
            _ => self.cfg.mp,
        };
        // Validate the `:mp-value` parameter (`declarative-memory.lisp`):
        // an invalid value or a repeat aborts the request with a warning.
        if want_slots.iter().filter(|(s, _)| s == ":MP-VALUE").count() > 1 {
            self.emit_warning("Invalid retrieval request.");
            self.emit_warning(":mp-value parameter used more than once.");
            self.dm_busy = false;
            self.buffers.get_mut("RETRIEVAL").unwrap().busy = false;
            return Ok(());
        }
        if let Some(v) = &mp_req {
            if !matches!(v, Value::Nil | Value::Num(_) | Value::Int(_)) {
                self.emit_warning("Invalid retrieval request.");
                self.emit_warning(":mp-value parameter's value can only be nil or a number.");
                self.dm_busy = false;
                self.buffers.get_mut("RETRIEVAL").unwrap().busy = false;
                return Ok(());
            }
        }
        // `:rt-value` request parameter (`declarative-memory.lisp:1292`)
        // overrides `:rt` for this request; the value must be a number.
        let rt = match want_slots
            .iter()
            .find(|(s, _)| s == ":RT-VALUE")
            .map(|(_, v)| v.clone())
        {
            Some(Value::Num(x)) => x as f32,
            Some(Value::Int(i)) => i as f32,
            _ => self.cfg.rt,
        };
        // `:bll-value`/`:mas-value`/`:ans-value` (`declarative-memory.lisp:1243`):
        // nil disables that term for this request.
        let bll = match want_slots
            .iter()
            .find(|(s, _)| s == ":BLL-VALUE")
            .map(|(_, v)| v.clone())
        {
            Some(Value::Nil) => None,
            Some(Value::Num(x)) => Some(x as f32),
            Some(Value::Int(i)) => Some(i as f32),
            _ => self.cfg.bll,
        };
        let mas = match want_slots
            .iter()
            .find(|(s, _)| s == ":MAS-VALUE")
            .map(|(_, v)| v.clone())
        {
            Some(Value::Nil) => None,
            Some(Value::Num(x)) => Some(x as f32),
            Some(Value::Int(i)) => Some(i as f32),
            _ => self.cfg.mas,
        };
        let ans = match want_slots
            .iter()
            .find(|(s, _)| s == ":ANS-VALUE")
            .map(|(_, v)| v.clone())
        {
            Some(Value::Nil) => None,
            Some(Value::Num(x)) => Some(x as f32),
            Some(Value::Int(i)) => Some(i as f32),
            _ => self.cfg.ans,
        };
        // Partial matching (mp set, esc on): candidacy is the request's
        // slot signature — required-filled and required-empty slot names —
        // NOT the chunk type (`slots-vector-match-signature`,
        // framework/chunk-spec.lisp:895). `=`/`-` values score activation
        // instead of filtering. Otherwise exact matching as before.
        let partial = self.cfg.esc && mp.is_some();
        // Signature masks from `add-slot-spec-to-chunk-spec`
        // (chunk-spec.lisp:758): `= V` (V non-nil) -> filled; `= nil` ->
        // empty; `- nil` -> filled; `- V` (V non-nil) -> no constraint.
        let mut req_filled: Vec<&str> = Vec::new();
        let mut req_empty: Vec<&str> = Vec::new();
        for (s, v) in &want_slots {
            if s.starts_with(':') {
                continue;
            }
            if *v == Value::Nil {
                req_empty.push(s);
            } else {
                req_filled.push(s);
            }
        }
        for (s, v) in &want_neg {
            if s.starts_with(':') {
                continue;
            }
            if *v == Value::Nil {
                req_filled.push(s);
            }
        }
        let recent_req = want_slots
            .iter()
            .find(|(s, _)| s == ":RECENTLY-RETRIEVED")
            .map(|(_, v)| v.clone());
        // `:recently-retrieved nil` removes the marked candidates
        // (`declarative-memory.lisp:1192`), tracing the header followed by
        // each (DM order, newest chunk first).
        let mut recent_removed: Vec<String> = Vec::new();
        if let Some(rv) = &recent_req {
            if matches!(rv, Value::Nil) {
                for idx in self.dm_alist_order() {
                    let c = &self.dm[idx];
                    let cname = c.name.clone().unwrap_or_default();
                    let sig_ok = req_filled.iter().all(|s| c.get(s) != Value::Nil)
                        && req_empty.iter().all(|s| c.get(s) == Value::Nil);
                    if sig_ok && self.is_recent(&cname) {
                        recent_removed.push(cname);
                    }
                }
                if self.act_high() {
                    self.out("Removing recently retrieved chunks:".to_string());
                    for n in &recent_removed {
                        self.out(n.clone());
                    }
                }
            }
        }
        // `retrieval-history` (`other-files/retrieval-history.lisp`): capture
        // the printed request and the per-candidate activations.
        let hist_request = self
            .record_retrieval_history
            .then(|| render_request_spec(&spec));
        let request_lines = pprint_request_lines(&spec);
        let mut non_matches: Vec<String> = Vec::new();
        // `(dm index, activation, base-level)` for each candidate.
        let mut acts: Vec<(usize, f32, f32)> = Vec::new();
        let mut best_a: Option<f32> = None;
        let mut tied: Vec<ChunkInst> = Vec::new();
        // `chunk-list` is `(dm-chunks)` (`declarative-memory.lisp:1125`):
        // newest signature group first, newest chunk first within a group.
        let order = self.dm_alist_order();
        // Pass 1 — candidacy: ACT-R prints every match/no-match line before
        // any activation line (`declarative-memory.lisp:1314`).
        let mut candidates: Vec<usize> = Vec::new();
        for idx in order {
            let c = self.dm[idx].clone();
            let cname = c.name.clone().unwrap_or_default();
            // The signature filter applies in exact and partial modes
            // (`chunk-list` at declarative-memory.lisp:1125).
            let sig_ok = req_filled.iter().all(|s| c.get(s) != Value::Nil)
                && req_empty.iter().all(|s| c.get(s) == Value::Nil);
            if !sig_ok {
                continue;
            }
            // `:recently-retrieved nil` excludes chunks retrieved within the
            // finst span; `t` requires one (declarative finsts).
            if let Some(rv) = &recent_req {
                let want_recent = !matches!(rv, Value::Nil);
                if want_recent != self.is_recent(&cname) {
                    continue;
                }
            }
            if !partial {
                // ACT-R retrieval does not test the request's `isa`; the
                // signature + slot tests alone decide (`match-chunk-spec-p`
                // only checks slots, chunk-spec.lisp:940).
                let mut matched = true;
                for (s, v) in &want_slots {
                    if s.starts_with(':') {
                        continue;
                    }
                    if !self.value_eq(&c.get(s), v) {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    for (s, v) in &want_neg {
                        if s.starts_with(':') {
                            continue;
                        }
                        if self.value_eq(&c.get(s), v) {
                            matched = false;
                            break;
                        }
                    }
                }
                if !matched {
                    if self.act_high() {
                        self.out(format!("Chunk {cname} does not match"));
                    }
                    non_matches.push(cname);
                    continue;
                }
                if self.act_medium() {
                    self.out(format!("Chunk {cname} matches"));
                }
            }
            candidates.push(idx);
        }
        // Pass 2 — activation. Exact matching keeps `chunk-list` order;
        // partial matching uses `(nreverse chunk-list)`
        // (`declarative-memory.lisp:1349`).
        let act_order: Vec<usize> = if partial {
            candidates.iter().rev().cloned().collect()
        } else {
            candidates.clone()
        };
        let sact = self.cfg.sact.is_some();
        self.sact_capture = sact;
        let mut sact_chunks: Vec<(String, String)> = Vec::new();
        for idx in act_order {
            let c = self.dm[idx].clone();
            let cname = c.name.clone().unwrap_or_default();
            let sact_mark = sact.then(|| self.out_len());
            let sact_trace = self.trace_on;
            let sact_act = self.cfg.act;
            if sact {
                self.trace_on = true;
                self.cfg.act = Some(Act::T);
            }
            if self.act_medium() {
                self.out(format!("Computing activation for chunk {cname}"));
                self.out("Computing base-level".to_string());
            }
            // A = base-level + spreading + partial matching + activation
            // noise. Base-level from the decay/spacing equation when `:bll`
            // or `:eblse` is set, else blc. Noise drawn per candidate in DM
            // order.
            let base = self.base_level(idx, bll, true);
            let spread = if c.name.is_some() {
                self.spreading_activation(&cname, mas)
            } else {
                0.0
            };
            let mut a = base + spread + self.inhibition_offset(&self.hist[idx].refs);
            // Partial matching: `=` slots add `mp * similarity`; `-` slots
            // add `mp * md` only when the chunk's value matches the
            // negated value (similarity == ms), per
            // `partial-matching` (`declarative-memory.lisp:2596`). Slots
            // with a nil request value are signature-only (no testable
            // slot), so they do not score.
            if let Some(mp) = mp {
                let show = self.act_medium();
                if show {
                    self.out("Computing partial matching component".to_string());
                }
                let mut total = 0.0f32;
                for (s, v) in &want_neg {
                    // `partial-matching` (`declarative-memory.lisp:2583`) walks
                    // every slot spec in request order. Negatives and positives
                    // are stored separately here, so replay the negative tests
                    // first for the requests whose source order starts with `-`.
                    if s.starts_with(':') {
                        continue;
                    }
                    let cv = c.get(s);
                    let sim = self.similarity(v, &cv);
                    if show {
                        self.out(format!("  comparing slot {s}"));
                        self.out(format!(
                            "  Requested: - {}  Chunk's slot value: {}",
                            v.show(),
                            cv.show()
                        ));
                        self.out(format!("  similarity: {}", lisp_f(sim)));
                    }
                    let simd = if sim == self.cfg.ms {
                        mp * self.cfg.md
                    } else {
                        if show && !self.sact_capture {
                            self.out(
                                "  negation test with similarity not ms has no effect".to_string(),
                            );
                        }
                        0.0
                    };
                    if show {
                        self.out(format!("  effective similarity value is {}", lisp_f(simd)));
                    }
                    total += simd;
                }
                for (s, v) in &want_slots {
                    if s.starts_with(':') || *v == Value::Nil {
                        continue;
                    }
                    let cv = c.get(s);
                    let sim = self.similarity(v, &cv);
                    let simd = mp * sim;
                    if show {
                        self.out(format!("  comparing slot {s}"));
                        self.out(format!(
                            "  Requested: = {}  Chunk's slot value: {}",
                            v.show(),
                            cv.show()
                        ));
                        self.out(format!("  similarity: {}", lisp_f(sim)));
                        self.out(format!("  effective similarity value is {}", lisp_f(simd)));
                    }
                    total += simd;
                }
                if show {
                    self.out(format!("Total similarity score {}", lisp_f(total)));
                }
                // Lisp adds the partial-matching component to the activation
                // once (`partial-matching` returns `total-sim`); adding each
                // slot's score individually changes the last float bit.
                a += total;
            }
            // `:uan` moved the transient noise inside the base-level log and
            // replaced the noise hook with `only-permanent-noise`.
            let noise = if self.cfg.uan {
                0.0
            } else {
                match ans {
                    Some(s) => self.rng.borrow_mut().noise(s),
                    None => 0.0,
                }
            };
            if self.act_medium() {
                if self.cfg.uan {
                    self.out(format!("Adding permanent noise {}", lisp_f(0.0)));
                    self.out(format!("noise hook returns: {}", lisp_f(0.0)));
                } else {
                    self.out(format!("Adding transient noise {}", lisp_f(noise)));
                    self.out(format!("Adding permanent noise {}", lisp_f(0.0)));
                }
            }
            a += noise;
            self.hist[idx].last_retrieval = Some((self.time_ms, a));
            acts.push((idx, a, base));
            if self.act_low() {
                self.out(format!("Chunk {cname} has an activation of: {}", lisp_f(a)));
            }
            if let Some(mark) = sact_mark {
                let detail = self.out_since(mark).join("\n");
                self.truncate_out(mark);
                sact_chunks.push((cname.clone(), detail));
                self.trace_on = sact_trace;
                self.cfg.act = sact_act;
            }
            match best_a {
                None => {
                    if self.act_medium() {
                        self.out(format!(
                            "Chunk {cname} has the current best activation {}",
                            lisp_f(a)
                        ));
                    }
                    best_a = Some(a);
                    tied = vec![c.clone()];
                }
                Some(cur) if a > cur => {
                    if self.act_medium() {
                        self.out(format!(
                            "Chunk {cname} is now the current best with activation {}",
                            lisp_f(a)
                        ));
                    }
                    best_a = Some(a);
                    tied = vec![c.clone()];
                }
                Some(cur) if a == cur => {
                    if self.act_medium() {
                        self.out(format!(
                            "Chunk {cname} matches the current best activation {}",
                            lisp_f(a)
                        ));
                    }
                    tied.push(c.clone());
                }
                _ => {}
            }
        }
        self.sact_capture = false;
        // A unique best and a tie-break narrate differently
        // (`declarative-memory.lisp:1488` vs `:1515`), so the tie set's size has
        // to survive the selection below.
        let ntied = tied.len();
        let tied_names: Vec<String> = tied
            .iter()
            .map(|c| c.name.clone().unwrap_or_default())
            .collect();
        let best = match ntied {
            0 => None,
            1 => Some(tied.into_iter().next().unwrap()),
            _ if self.cfg.er => {
                let i = self.rng.borrow_mut().random_int(tied.len() as u64) as usize;
                Some(tied.into_iter().nth(i).unwrap())
            }
            // Deterministic tie-break: lowest chunk name first
            // (`(sort best 'string<)` then car).
            _ => {
                tied.sort_by(|a, b| {
                    a.name
                        .as_deref()
                        .unwrap_or("")
                        .cmp(b.name.as_deref().unwrap_or(""))
                });
                Some(tied.into_iter().next().unwrap())
            }
        };
        let act = best_a.unwrap_or(0.0);
        let below = self.cfg.esc && act < rt;
        let matches: Vec<String> = candidates
            .iter()
            .map(|i| self.dm[*i].name.clone().unwrap_or_default())
            .collect();
        let activations: HashMap<String, f32> = acts
            .iter()
            .map(|(i, a, _)| (self.dm[*i].name.clone().unwrap_or_default(), *a))
            .collect();
        self.last_request = Some(LastRequest {
            time_ms: self.time_ms,
            request_lines: request_lines.clone(),
            matches: matches.clone(),
            best: tied_names,
            activations,
            rt,
        });
        let sact_details: HashMap<String, String> = sact_chunks.iter().cloned().collect();
        if sact {
            let result = match &best {
                None => "No matching chunk found retrieval failure".to_string(),
                Some(_) if below => {
                    format!("No chunk above the retrieval threshold: {}", lisp_f(rt))
                }
                Some(w) if ntied > 1 => {
                    let wname = w.name.clone().unwrap_or_default();
                    format!(
                        "Chunk {wname} chosen among the chunks with activation {}",
                        lisp_f(act)
                    )
                }
                Some(w) => {
                    let wname = w.name.clone().unwrap_or_default();
                    format!("Chunk {wname} with activation {} is the best", lisp_f(act))
                }
            };
            self.sact_traces.insert(
                self.time_ms,
                SactTrace {
                    matches: matches.clone(),
                    no_matches: non_matches.clone(),
                    chunks: sact_chunks,
                    result: Some(result),
                },
            );
        }
        if let Some(request) = hist_request {
            let failure = |name: &str| RetrievalChunk {
                failure: true,
                name: name.to_string(),
                text: String::new(),
                params: String::new(),
                activation_trace: String::new(),
            };
            let mut chunks: Vec<RetrievalChunk> = Vec::new();
            if acts.is_empty() {
                chunks.push(failure(":retrieval-failure"));
            } else {
                if below {
                    chunks.push(failure(":retrieval-failure"));
                }
                // The set is the best chunk first, then the remaining
                // candidates in `chunk-set` order (`cons (car best)
                // (remove (car best) chunk-set)`).
                let winner = best
                    .as_ref()
                    .and_then(|b| b.name.as_deref())
                    .and_then(|bn| {
                        acts.iter()
                            .position(|(i, _, _)| self.dm[*i].name.as_deref() == Some(bn))
                    });
                let mut order: Vec<usize> = Vec::new();
                if let Some(w) = winner {
                    order.push(w);
                }
                for (j, _) in acts.iter().enumerate() {
                    if Some(j) != winner {
                        order.push(j);
                    }
                }
                for j in order {
                    let (cidx, cact, cbase) = acts[j];
                    let name = self.dm[cidx].name.clone().unwrap_or_default();
                    let text = self.printed_chunk(&name).unwrap_or_default();
                    let params = self.sdp_chunk_block(cidx, cact, cbase);
                    let activation_trace = sact_details.get(&name).cloned().unwrap_or_default();
                    chunks.push(RetrievalChunk {
                        failure: false,
                        name,
                        text,
                        params,
                        activation_trace,
                    });
                }
            }
            self.retrieval_history.push(RetrievalRecord {
                time_ms: self.time_ms,
                request,
                chunks,
            });
        }
        if self.act_low() {
            match &best {
                Some(_) if below => {
                    self.out(format!(
                        "No chunk above the retrieval threshold: {}",
                        lisp_f(rt)
                    ));
                }
                Some(w) => {
                    let wname = w.name.clone().unwrap_or_default();
                    if ntied > 1 {
                        self.out(format!(
                            "Chunk {wname} chosen among the chunks with activation {}",
                            lisp_f(act)
                        ));
                    } else {
                        self.out(format!(
                            "Chunk {wname} with activation {} is the best",
                            lisp_f(act)
                        ));
                    }
                }
                None => {
                    self.out("No matching chunk found retrieval failure".to_string());
                }
            }
        }
        let gen = self.dm_gen;
        match best {
            None => {
                let wait = failure_wait(&self.cfg, rt);
                let at = self.time_ms + wait;
                self.push(at, Prio::Num(0), Out::Low, Event::RetrievalFailure { gen });
            }
            Some(_) if self.cfg.esc && act < rt => {
                let wait = failure_wait(&self.cfg, rt);
                let at = self.time_ms + wait;
                self.push(at, Prio::Num(0), Out::Low, Event::RetrievalFailure { gen });
            }
            Some(winner) => {
                let name = winner.name.clone().unwrap_or_default();
                let wait = if self.cfg.esc {
                    latency(self.cfg.lf_ms, self.cfg.le, act)
                } else {
                    0
                };
                let at = self.time_ms + wait;
                self.push(
                    at,
                    Prio::Num(0),
                    Out::Medium,
                    Event::RetrievedChunk { name, gen },
                );
            }
        }
        Ok(())
    }
}

/// `pprint-chunk-spec` (`framework/chunk-spec.lisp:1164`): command-output
/// lines for the request's slot tests. The type is not printed.
pub(super) fn pprint_request_lines(spec: &NewChunk) -> Vec<String> {
    let NewChunk::Spec {
        slots, neg, rels, ..
    } = spec
    else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for (s, v) in neg {
        if !s.starts_with(':') {
            lines.push(format!(" -  {s} {}", v.show()));
        }
    }
    for (s, v) in slots {
        if !s.starts_with(':') {
            lines.push(format!("    {s} {}", v.show()));
        }
    }
    for (s, v, op) in rels {
        if !s.starts_with(':') {
            let op = match op {
                crate::model::SlotOp::Gt => ">",
                crate::model::SlotOp::Lt => "<",
                crate::model::SlotOp::Ge => ">=",
                crate::model::SlotOp::Le => "<=",
                _ => "=",
            };
            lines.push(format!(" {op} {s} {}", v.show()));
        }
    }
    lines
}

/// `printed-chunk-spec`: the retrieval request as written -- `isa TYPE` then
/// `slot value` and `- slot value` lines -- for the retrieval history's
/// request column.
pub(super) fn render_request_spec(spec: &NewChunk) -> String {
    match spec {
        NewChunk::Named(n) => n.clone(),
        NewChunk::Instance(c) => c.name.clone().unwrap_or_default(),
        NewChunk::Spec {
            ctype, slots, neg, ..
        } => {
            let mut parts: Vec<String> = Vec::new();
            if !ctype.is_empty() {
                parts.push(format!("isa {ctype}"));
            }
            for (s, v) in slots {
                parts.push(format!("{s} {}", v.show()));
            }
            for (s, v) in neg {
                parts.push(format!("- {s} {}", v.show()));
            }
            parts.join("\n")
        }
    }
}

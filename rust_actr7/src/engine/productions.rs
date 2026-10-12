//! Procedural matching, conflict resolution, firing, compilation orchestration.
use super::compilation::{
    basic_vars, buffer_module, buffer_usage, canon_composed, canon_prod, compute_retain_buffers,
    failure_wait, goal_reason, imaginal_reason, indirect_action_buffer_name, indirect_action_vars,
    indirect_safe_bind_reason, instantiate_var, is_buffer_name, latency, lhs_buffers, motor_reason,
    perceptual_reason, prod_lhs_slots, prod_rhs_actions, production_bind_vars, r_b_c1, r_b_c2,
    remove_dup_keep_last, render_compiled, render_production, retrieval_reason, rhs_buffers,
    sort_token, var_buffers,
};
use super::dm::{bl_equation, ol_str};
use super::rhs::{explicit_out, implicit_clears, implicit_queries};
use super::*;
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
                let mut u = self.utilities[ep.index];
                if self.cfg.egs != 0.0 {
                    u += self.rng.borrow_mut().noise(self.cfg.egs);
                }
                // `compute-utility` with `save` (`utility-and-reward-1.lisp:357`,
                // called at `procedural.lisp:2230`) caches the value *before*
                // the ppm search offsets are folded in; `spp` prints it.
                self.production_utility[ep.index] = Some(u);
                // `ppm-offset` (procedural.lisp:2373): penalty is
                // `:ppm * Σ similarity` over the mismatched slots (the
                // similarities are negative, so this lowers utility).
                if let Some(p) = self.cfg.ppm {
                    u += p * mism.iter().map(|m| m.4).sum::<f32>();
                }
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
                lines.push(render_production(prod, b, mism));
                lines.push(format!("Parameters for production {}:", prod.name));
                if self.cfg.esc {
                    lines.push(format!(" :utility {}", fmt_6_3(*u)));
                }
                lines.push(format!(" :u {}", fmt_6_3(0.0)));
            }
            for l in lines {
                self.out(l);
            }
        }
        let winner = match tied.len() {
            0 => None,
            1 => Some(tied.into_iter().next().unwrap()),
            _ if self.cfg.er => {
                let i = self.rng.borrow_mut().random_int(tied.len() as u64) as usize;
                Some(tied.into_iter().nth(i).unwrap())
            }
            _ => Some(tied.into_iter().last().unwrap()),
        };
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
                        ("ERROR", Pattern::Nil) => self.buffer_error(buffer),
                        ("ERROR", Pattern::Sym(s)) if s == "T" => !self.buffer_error(buffer),
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
        // dependency, `production-parsing.lisp`).
        for item in &prod.lhs {
            if matches!(item, LhsItem::Match { .. } | LhsItem::Query { .. })
                && self.match_item(item, &mut b, ppm)?.is_none()
            {
                return Ok(None);
            }
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
                        if want != "CHUNK" && &chunk.ctype != want {
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
                    self.rel_test(t.op, &t.value, &got)
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
                // satisfy `ISA GOAL` (bind-test). With other slot tests the
                // type is carried for slot indices only, so the tests
                // themselves decide (blending/addition).
                let only_isa = tests.iter().all(|t| t.slot == "ISA");
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
                        ("STATE", Pattern::Sym(s)) if s == "FREE" => self.query_free(buffer)?,
                        ("STATE", Pattern::Sym(s)) if s == "BUSY" => !self.query_free(buffer)?,
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
                        // `?manual-left> index free/down` etc.
                        (attr, Pattern::Sym(s))
                            if matches!(attr, "INDEX" | "MIDDLE" | "RING" | "PINKIE" | "THUMB") =>
                        {
                            match s.as_str() {
                                "FREE" => self.query_finger_free(buffer, attr)?,
                                "BUSY" => !self.query_finger_free(buffer, attr)?,
                                "DOWN" => self.query_finger_down(buffer, attr)?,
                                _ => {
                                    return Err(unimpl(format!("query ?{buffer}> {attr} {val:?}")))
                                }
                            }
                        }
                        // `?buffer> error t/nil`: only blending carries an
                        // error flag (`query-module`, modules.lisp:798).
                        ("ERROR", Pattern::Nil) => !self.buffer_error(buffer),
                        ("ERROR", Pattern::Sym(s)) if s == "T" => self.buffer_error(buffer),
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
            "IMAGINAL" => Ok(!self.imag_busy),
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
            _ => self.query_free(buffer),
        }
    }

    /// `?manual> preparation free/busy` (extended-motor-actions).
    pub(super) fn query_prep_free(&self, buffer: &str) -> Result<bool, EngineError> {
        match buffer {
            "MANUAL" => Ok(!self.man_prep_busy),
            _ => self.query_free(buffer),
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

    /// `?buffer> error t/nil`: only the blending module tracks an error
    /// flag; every other buffer reports no error.
    pub(super) fn buffer_error(&self, buffer: &str) -> bool {
        buffer == "BLENDING" && self.blending_error
    }

    pub(super) fn fire(&mut self, prod_idx: usize, bindings: &Bindings) -> Result<(), EngineError> {
        let prod = self.model.productions[prod_idx].clone();
        // Copies of chunk-valued `=buffer` refs are deduped within one
        // firing (GOTCHA #41): a modify and a request referencing the same
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
                    self.out_lines.push(text);
                }
                RhsAction::Other { head, .. } => {
                    // `!stop!` ends the run with a BREAK-EVENT.
                    if head == "!STOP!" {
                        self.stopped = true;
                    } else {
                        return Err(unimpl(format!("RHS {head} needs a Lisp evaluator")));
                    }
                }
                _ => {}
            }
        }
        if self.stopped {
            return Ok(());
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
                    // A `=buffer>` modify stores chunk-valued refs by
                    // reference unless the ref is an unnamed spec chunk
                    // (`paired-2x2`), matching `instantiate-chunk-spec`.
                    let copy = matches!(buffer.as_str(), "GOAL" | "IMAGINAL");
                    let mut sub = Vec::new();
                    for (s, p) in slots {
                        sub.push((s.clone(), self.rhs_value(p, &b, copy, &mut copied)?));
                    }
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
                    let copy = matches!(buffer.as_str(), "GOAL" | "IMAGINAL");
                    let mut sub = Vec::new();
                    for (s, p) in slots {
                        sub.push((s.clone(), self.rhs_value(p, &b, copy, &mut copied)?));
                    }
                    if buffer == "IMAGINAL" {
                        self.imag_busy = true;
                        let delay = if self.cfg.vidt {
                            self.randomize_ms(self.cfg.imaginal_delay_ms)
                        } else {
                            self.cfg.imaginal_delay_ms
                        };
                        self.push(
                            self.time_ms + delay,
                            Prio::Num(0),
                            Out::Low,
                            Event::ModBuffer {
                                buffer: buffer.clone(),
                                slots: sub,
                                module: buffer_module(buffer),
                            },
                        );
                    } else {
                        self.now(
                            Prio::Num(100),
                            Out::Medium,
                            Event::ModBuffer {
                                buffer: buffer.clone(),
                                slots: sub,
                                module: buffer_module(buffer),
                            },
                        );
                    }
                }
                RhsAction::Request { buffer, spec } => {
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
        lhs: &[(String, Vec<(String, SlotVal)>)],
        rhs: &[(String, Vec<(String, String)>)],
    ) {
        let mut text = format!("(P {name}");
        for (token, slots) in lhs {
            text.push_str(&format!(" {token}"));
            for (n, v) in slots {
                text.push_str(&format!(" {n} {}", v.render()));
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
                    let reward: Option<f32> = {
                        let r1 = self.model.productions[p1_idx].reward;
                        let r2 = self.model.productions[idx].reward;
                        match (r1, r2) {
                            (Some(a), Some(b)) => Some(a.max(b)),
                            (Some(a), None) => Some(a),
                            (None, Some(b)) => Some(b),
                            (None, None) => None,
                        }
                    };
                    if trace {
                        self.out(format!(
                            "  Production {p1name} and {p2name} are being composed."
                        ));
                        if let Some((didx, dname, compiled)) = &dup {
                            if *compiled {
                                self.out(format!("  Recreating production {dname}"));
                                // `update-utility-for-compiled-production`
                                // (`utility-and-reward-1.lisp:497`): the existing
                                // compiled production's `u` moves toward the first
                                // parent's via `linear-update-utility`, then the
                                // updated parameters are printed.
                                if self.cfg.esc && self.cfg.ul {
                                    let alpha = self.cfg.alpha;
                                    let parent = self.utilities[p1_idx];
                                    let old = self.utilities[*didx];
                                    self.utilities[*didx] = old + alpha * (parent - old);
                                }
                                let u = self.utilities[*didx];
                                let utility = self.production_utility[*didx];
                                let reward = self.model.productions[*didx].reward;
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
                            self.out(render_compiled(&cname, &doc, &sorted, &rhs));
                            // A freshly compiled production has no cached
                            // `production-utility` yet and `u = :nu` (0).
                            self.print_prod_params(&cname, 0.0, None, reward);
                        }
                    }
                    if dup.is_none() {
                        self.install_compiled(&cname, &lhs, &rhs);
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

    /// Compose two productions per buffer (`buffer-condition-union`,
    /// production-compilation.lisp:2320): the first's conditions, then the
    /// second's conditions minus the slots the first modifies, with
    /// `remove-duplicates` keeping the later copy; the actions are the
    /// first's untouched slots followed by the second's.
    #[allow(clippy::type_complexity)]
    #[allow(clippy::type_complexity)]
    pub(super) fn compose_pair(
        &self,
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
        let p1 = &self.model.productions[i1];
        let p2 = &self.model.productions[i2];
        // `rename-variables` (`production-compilation.lisp:1945`): a basic
        // variable that occurs in both productions is renamed in the second
        // so the two no longer unify (e.g. a goal state variable and a
        // retrieved-slot variable that happen to share a name).
        let ren: HashMap<String, String> = {
            let v1 = basic_vars(p1);
            let v2 = basic_vars(p2);
            let vb1 = var_buffers(p1);
            let vb2 = var_buffers(p2);
            let mut used: HashSet<String> = v1.union(&v2).cloned().collect();
            let mut m = HashMap::new();
            for v in &v2 {
                // `rename-variables` (`production-compilation.lisp:1533`): a
                // shared basic variable is renamed in the second production
                // when it binds a different (buffer, slot) there — e.g. a goal
                // state variable and a retrieved-slot variable sharing a name.
                // The same slot in the same buffer stays unified.
                if v1.contains(v) && vb1.get(v) != vb2.get(v) {
                    let mut n = 1;
                    let fresh = loop {
                        let c = format!("{v}{n}");
                        if !used.contains(&c) {
                            break c;
                        }
                        n += 1;
                    };
                    used.insert(fresh.clone());
                    m.insert(v.clone(), fresh);
                }
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
                    let v = match v {
                        SlotVal::Var(x) => SlotVal::Var(rename(&x)),
                        other => other,
                    };
                    (n, v)
                })
                .collect()
        };
        let p1_lhs = prod_lhs_slots(p1, b1, &self.model.chunk_types);
        let p2_lhs: Vec<(String, Vec<(String, SlotVal)>)> =
            prod_lhs_slots(p2, b2, &self.model.chunk_types)
                .into_iter()
                .map(|(b, s)| (b, rename_slots(s)))
                .collect();
        let p1_acts = prod_rhs_actions(p1, b1);
        let p2_acts: Vec<(char, String, Vec<(String, SlotVal)>, Option<String>)> =
            prod_rhs_actions(p2, b2)
                .into_iter()
                .map(|(op, buf, slots, var)| (op, buf, rename_slots(slots), var))
                .collect();

        // Cross-production variable mappings for same-slot `=` pairs
        // (`MAP-GOAL-BUFFER`, goal-compilation.lisp:320-330): two variables
        // unify onto the first's; a variable facing a constant is
        // instantiated from its own bindings.
        let mut map: HashMap<String, SlotVal> = HashMap::new();
        for (buf, slots1) in &p1_lhs {
            // `MAP-GOAL-BUFFER` (`goal-compilation.lisp:320`) also maps p1's
            // RHS goal slots — from a `+` request, `=`/`*` modify — onto p2's
            // LHS `=` slots, so a variable renamed by `rename-variables`
            // re-unifies with p1's.
            let mut p1_slots: Vec<(String, SlotVal)> = slots1.clone();
            if buf == "GOAL" {
                for (op, b, s, _) in &p1_acts {
                    if b == buf && matches!(op, '=' | '*' | '+' | 'r') {
                        for (n, v) in s {
                            if !p1_slots.iter().any(|(x, _)| x == n) {
                                p1_slots.push((n.clone(), v.clone()));
                            }
                        }
                    }
                }
            }
            for (name1, v1) in &p1_slots {
                if let Some((_, slots2)) = p2_lhs.iter().find(|(b, _)| b == buf) {
                    if let Some((_, v2)) = slots2.iter().find(|(n, _)| n == name1) {
                        match (v1, v2) {
                            (SlotVal::Var(a), SlotVal::Var(c)) => {
                                map.insert(c.clone(), SlotVal::Var(a.clone()));
                            }
                            (SlotVal::Lit(_), SlotVal::Var(c)) => {
                                if let Some(val) = instantiate_var(c, b2) {
                                    map.insert(c.clone(), SlotVal::Lit(val));
                                }
                            }
                            (SlotVal::Var(a), SlotVal::Lit(_)) => {
                                if let Some(val) = instantiate_var(a, b1) {
                                    map.insert(a.clone(), SlotVal::Lit(val));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        // `MAP-RETRIEVAL-BUFFER` (`retrieval-compilation.lisp:71`): p1
        // requests the buffer (4/12/20/28) and p2 matches it (8/12/24/28).
        // The retrieved chunk's slots instantiate p1's request-action
        // variables and p2's condition variables, and the buffer variable
        // maps to the chunk itself.
        let u1r = buffer_usage(p1, "RETRIEVAL");
        let u2r = buffer_usage(p2, "RETRIEVAL");
        if matches!(u1r, 4 | 12 | 20 | 28) && matches!(u2r, 8 | 12 | 24 | 28) {
            if let Some(chunk) = b2.get("=RETRIEVAL").map(|v| v.show()) {
                map.insert("RETRIEVAL".to_string(), SlotVal::Lit(chunk.clone()));
                for (op, buf, slots, _) in &p1_acts {
                    if buf == "RETRIEVAL" && (*op == '+' || *op == 'r') {
                        for (_, v) in slots {
                            if let SlotVal::Var(n) = v {
                                if let Some(val) = instantiate_var(n, b1) {
                                    map.insert(n.clone(), SlotVal::Lit(val));
                                }
                            }
                        }
                    }
                }
                if let Some((_, slots2)) = p2_lhs.iter().find(|(b, _)| b == "RETRIEVAL") {
                    for (name, v) in slots2 {
                        if let SlotVal::Var(n) = v {
                            if let Some(cv) = self.chunk_name_slot(&chunk, name) {
                                // A variable already instantiated from p1's
                                // own request (`pre =state`) keeps p1's value;
                                // p2's condition only fills the rest.
                                map.entry(n.clone()).or_insert(SlotVal::Lit(cv));
                            }
                        }
                    }
                }
            }
        }
        // `MAP-IMAGINAL-BUFFER`'s RHS `=` to LHS `=` case
        // (`imaginal-compilation.lisp:212`): p1 modified the imaginal buffer
        // and p2 matches it. Slots common to p1's (LHS ∪ RHS) and p2's LHS
        // map by value — a p2 constant wins, otherwise p2's variable maps to
        // p1's value.
        let u1i = buffer_usage(p1, "IMAGINAL");
        let u2i = buffer_usage(p2, "IMAGINAL");
        if matches!(u1i, 9 | 25 | 56) && matches!(u2i, 8 | 9 | 12 | 13 | 24 | 25 | 28 | 29 | 56) {
            let p1_lhs_i = p1_lhs
                .iter()
                .find(|(b, _)| b == "IMAGINAL")
                .map(|(_, s)| s.clone())
                .unwrap_or_default();
            let p1_rhs_i: Vec<(String, SlotVal)> = p1_acts
                .iter()
                .filter(|(op, b, _, _)| matches!(*op, '=' | '*') && b == "IMAGINAL")
                .flat_map(|(_, _, s, _)| s.clone())
                .collect();
            let rhs_names: Vec<String> = p1_rhs_i.iter().map(|(n, _)| n.clone()).collect();
            let mut p1_slots: Vec<(String, SlotVal)> = p1_lhs_i
                .into_iter()
                .filter(|(n, _)| !rhs_names.contains(n))
                .collect();
            p1_slots.extend(p1_rhs_i);
            if let Some((_, p2_slots)) = p2_lhs.iter().find(|(b, _)| b == "IMAGINAL") {
                for (n1, v1) in &p1_slots {
                    if let Some((_, v2)) = p2_slots.iter().find(|(n, _)| n == n1) {
                        match (v1, v2) {
                            (SlotVal::Var(a), SlotVal::Lit(_)) => {
                                if let Some(val) = instantiate_var(a, b1) {
                                    map.insert(a.clone(), SlotVal::Lit(val));
                                }
                            }
                            (_, SlotVal::Var(c)) => {
                                map.insert(c.clone(), v1.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
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
                    if ivars.contains(n) && b.contains_key(n) {
                        SlotVal::Lit(b[n].show())
                    } else if let Some(m) = map.get(n) {
                        m.clone()
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
        let p1_lhs = apply_lhs(&p1_lhs, b1, &indirect_vars);
        let p2_lhs = apply_lhs(&p2_lhs, b2, &indirect_vars);
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
        let p1_acts = inst_acts(&p1_acts, b1, &indirect_vars);
        let p2_acts = inst_acts(&p2_acts, b2, &indirect_vars);

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
                for (n, _) in s {
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
                    if !modified.contains(&n) {
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
        // Query conditions (`?buffer> state free`) follow the matches.
        for (b, s) in p1_lhs.iter().chain(&p2_lhs) {
            if b.starts_with('?') && !lhs.iter().any(|(t, _)| t == b) {
                lhs.push((b.clone(), remove_dup_keep_last(s.clone())));
            }
        }
        // `!bind!`/`!safe-bind!` conditions are prepended
        // (`post-conditions`, production-compilation.lisp:1742).
        let mut binds: Vec<(String, Vec<(String, SlotVal)>)> = Vec::new();
        for (b, s) in p1_lhs.iter().chain(&p2_lhs) {
            if b.starts_with('!') {
                binds.push((b.clone(), s.clone()));
            }
        }
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
            // `COMPOSE-GOAL-BUFFER`'s `(4 12 13 44)` case
            // (`goal-compilation.lisp:544`): p1 requests the buffer and p2
            // modifies it (without requesting), so the composed action is
            // p1's `+` with p2's `=`/`*` slots merged in (`buffer+-union`:
            // p1's non-matching slots then p2's, sorted).
            let p1_goal_req = acts_for(&p1_acts, buf, &['+', 'r']);
            let p2_goal_eq = acts_for(&p2_acts, buf, &['=', '*']);
            if buf == "GOAL"
                && !p1_goal_req.is_empty()
                && !p2_goal_eq.is_empty()
                && acts_for(&p2_acts, buf, &['+', 'r']).is_empty()
            {
                // `a1=`/`a1*`: p1's own `=`/`*` modify is emitted first
                // (`goal-compilation.lisp:546`), then the `+` union.
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
                    p1_goal_req.into_iter().flat_map(|(s, _)| s).collect();
                let mut eq: Vec<(String, SlotVal)> =
                    p2_goal_eq.into_iter().flat_map(|(s, _)| s).collect();
                eq.sort_by(|a, b| a.0.cmp(&b.0));
                let names2: Vec<String> = eq.iter().map(|(n, _)| n.clone()).collect();
                slots.retain(|(n, _)| !names2.contains(n));
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
            let mut eq1: Vec<_> = acts_for(&p1_acts, buf, &['='])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            eq1.sort_by(|a, b| a.0.cmp(&b.0));
            let mut eq2: Vec<_> = acts_for(&p2_acts, buf, &['='])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            eq2.sort_by(|a, b| a.0.cmp(&b.0));
            let mut st1: Vec<_> = acts_for(&p1_acts, buf, &['*'])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            st1.sort_by(|a, b| a.0.cmp(&b.0));
            let mut st2: Vec<_> = acts_for(&p2_acts, buf, &['*'])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            st2.sort_by(|a, b| a.0.cmp(&b.0));
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
            if has_eq1 || has_eq2 {
                let names2: Vec<String> = eq2.iter().map(|(n, _)| n.clone()).collect();
                let mut slots: Vec<(String, SlotVal)> = eq1
                    .into_iter()
                    .filter(|(n, _)| !names2.contains(n))
                    .collect();
                slots.extend(eq2);
                rhs.push((format!("={buf}>"), sv(slots)));
            }
            // `*` modify-request union.
            if !st1.is_empty() || !st2.is_empty() {
                let names2: Vec<String> = st2.iter().map(|(n, _)| n.clone()).collect();
                let mut slots: Vec<(String, SlotVal)> = st1
                    .into_iter()
                    .filter(|(n, _)| !names2.contains(n))
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
        // `sort-actions`: descending by token (op+buffer), stable.
        rhs.sort_by(|a, b| b.0.cmp(&a.0));
        (lhs, rhs, dropped)
    }

    pub(super) fn similarity(&self, a: &Value, b: &Value) -> f32 {
        let a_name = match a {
            Value::Sym(s) => Some(s.as_str()),
            _ => None,
        };
        let b_name = match b {
            Value::Sym(s) => Some(s.as_str()),
            _ => None,
        };
        // Explicit similarities are keyed by the (symbol) values, not by DM
        // membership: ACT-R registers slot values as undefined chunks, so
        // `set-similarities (pink red -.4)` applies to plain symbols.
        if let (Some(x), Some(y)) = (a_name, b_name) {
            if let Some(v) = self
                .sims
                .get(&(x.to_string(), y.to_string()))
                .or_else(|| self.sims.get(&(y.to_string(), x.to_string())))
            {
                return *v;
            }
        }
        if a == b {
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
            if !ctype.is_empty() {
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
                for idx in (0..self.dm.len()).rev() {
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
        let mut best_a: Option<f32> = None;
        let mut tied: Vec<ChunkInst> = Vec::new();
        // `chunk-list` (the DM hash-table order) is reverse-insertion.
        let order: Vec<usize> = (0..self.dm.len()).rev().collect();
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
                    if c.get(s) != *v {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    for (s, v) in &want_neg {
                        if s.starts_with(':') {
                            continue;
                        }
                        if c.get(s) == *v {
                            matched = false;
                            break;
                        }
                    }
                }
                if !matched {
                    if self.act_high() {
                        self.out(format!("Chunk {cname} does not match"));
                    }
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
        for idx in act_order {
            let c = self.dm[idx].clone();
            let cname = c.name.clone().unwrap_or_default();
            if self.act_medium() {
                self.out(format!("Computing activation for chunk {cname}"));
                self.out("Computing base-level".to_string());
            }
            // A = base-level + spreading + partial matching + activation
            // noise. Base-level from the decay equation when :bll is set,
            // else blc. Noise drawn per candidate in DM order.
            let base = match bll {
                Some(bll) => {
                    let (refs, creation, count) = {
                        let h = &self.hist[idx];
                        (h.refs.clone(), h.creation_ms, h.count)
                    };
                    if count == 0 {
                        // Mirrors the Lisp warning path; unreachable via
                        // merge/add (both start count at 1).
                        -999999.0
                    } else {
                        if self.act_medium() {
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
                        if self.act_medium() {
                            self.out(format!("base-level value: {}", lisp_f(value)));
                        }
                        self.cfg.blc + value
                    }
                }
                None => match self.hist[idx].fixed_base {
                    Some(b) => {
                        if self.act_medium() {
                            self.out(format!("User provided chunk base-level: {}", lisp_f(b)));
                        }
                        b
                    }
                    None => {
                        if self.act_medium() {
                            self.out(format!("Starting with blc: {}", lisp_f(self.cfg.blc)));
                        }
                        self.cfg.blc
                    }
                },
            };
            if self.act_medium() {
                self.out(format!("Total base-level: {}", lisp_f(base)));
            }
            let spread = if c.name.is_some() {
                self.spreading_activation(&cname, mas)
            } else {
                0.0
            };
            let mut a = base + spread;
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
                    a += simd;
                }
                for (s, v) in &want_neg {
                    if s.starts_with(':') || *v == Value::Nil {
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
                    if sim == self.cfg.ms {
                        let simd = mp * self.cfg.md;
                        if show {
                            self.out(format!("  effective similarity value is {}", lisp_f(simd)));
                        }
                        total += simd;
                        a += simd;
                    } else if show {
                        self.out(
                            "  negation test with similarity not ms has no effect".to_string(),
                        );
                    }
                }
                if show {
                    self.out(format!("Total similarity score {}", lisp_f(total)));
                }
            }
            let noise = match ans {
                Some(s) => self.rng.borrow_mut().noise(s),
                None => 0.0,
            };
            if self.act_medium() {
                self.out(format!("Adding transient noise {}", lisp_f(noise)));
                self.out(format!("Adding permanent noise {}", lisp_f(0.0)));
            }
            a += noise;
            if self.act_low() {
                self.out(format!("Chunk {cname} has an activation of: {}", lisp_f(a)));
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
        let best = match tied.len() {
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
                    self.out(format!(
                        "Chunk {wname} with activation {} is the best",
                        lisp_f(act)
                    ));
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

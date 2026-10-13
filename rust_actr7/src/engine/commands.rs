//! Task-command layer and module request routing.
use super::eval::as_num;
use super::motor::key_table;
use super::*;
impl Engine {
    /// Run a scheduled task command: bind its `:params` to the `defun`'s
    /// parameters and execute the body (side-effecting forms handled by
    /// [`Engine::m_eval`]).
    pub(super) fn run_command(
        &mut self,
        name: &str,
        params: Vec<Value>,
    ) -> Result<(), EngineError> {
        let key = name.to_uppercase();
        let def = self
            .user_fns
            .get(&key)
            .cloned()
            .ok_or_else(|| unimpl(format!("scheduled command has no task defun: {name}")))?;
        let mut b: Bindings = HashMap::new();
        for (p, v) in def.params.iter().zip(params) {
            b.insert(p.to_uppercase(), v);
        }
        for e in &def.body {
            self.m_eval(e, &b).map_err(bad)?;
        }
        Ok(())
    }

    /// Mutable evaluator for command bodies: handles the side-effecting forms
    /// (`goal-focus`, `set-buffer-chunk`, `schedule-event[-relative]`,
    /// `act-r-random`) and the special forms `if`/`list`, delegating
    /// everything else to the pure evaluator.
    pub(super) fn m_eval(&mut self, e: &Sexp, b: &Bindings) -> Result<Value, String> {
        if let Sexp::List(items) = e {
            if let Some(h) = items.first().and_then(|x| x.as_sym()) {
                match h.to_uppercase().as_str() {
                    "IF" => {
                        let c = self.m_eval(items.get(1).ok_or("if needs a test")?, b)?;
                        return if c == Value::Nil {
                            match items.get(3) {
                                Some(a) => self.m_eval(a, b),
                                None => Ok(Value::Nil),
                            }
                        } else {
                            self.m_eval(items.get(2).ok_or("if needs a then")?, b)
                        };
                    }
                    "LIST" => {
                        let mut out = Vec::new();
                        for a in &items[1..] {
                            out.push(self.m_eval(a, b)?);
                        }
                        return Ok(Value::List(out));
                    }
                    "ACT-R-RANDOM" => {
                        let lim =
                            self.m_eval(items.get(1).ok_or("act-r-random needs a limit")?, b)?;
                        return match lim {
                            Value::Int(n) if n > 0 => {
                                Ok(Value::Int(self.rng.borrow_mut().random_int(n as u64) as i64))
                            }
                            Value::Int(_) => Ok(Value::Int(0)),
                            Value::Num(x) => Ok(Value::Num(
                                self.rng.borrow_mut().random_float(x as f32) as f64,
                            )),
                            other => {
                                Err(format!("act-r-random needs a number, got {}", other.show()))
                            }
                        };
                    }
                    "GOAL-FOCUS" | "GOAL-FOCUS-FCT" => {
                        let desc =
                            self.m_chunk_desc(items.get(1).ok_or("goal-focus needs an argument")?)?;
                        self.goal_focus(&desc).map_err(|e| e.to_string())?;
                        return Ok(Value::Nil);
                    }
                    "SET-BUFFER-CHUNK" => {
                        let buffer =
                            self.m_eval(items.get(1).ok_or("set-buffer-chunk needs a buffer")?, b)?;
                        let chunk =
                            self.m_eval(items.get(2).ok_or("set-buffer-chunk needs a chunk")?, b)?;
                        self.set_buffer_chunk_cmd(&buffer, &chunk)?;
                        return Ok(Value::Nil);
                    }
                    // The task's `move-it` is scheduled by the model's
                    // `!eval! (schedule-event-relative … 'move-it)`. Its
                    // effect on the window is replayed from the captured
                    // device op (`MODIFY-TEXT`), so the body is a no-op here
                    // (the arguments reference task globals the engine does
                    // not model).
                    "INCF" | "MODIFY-TEXT-FOR-EXP-WINDOW" => {
                        return Ok(Value::Nil);
                    }
                    // `(buffer-chunk <buffer> …)`: a model-body call prints
                    // the command-layer buffer dump (`tracking-test`'s
                    // `report` production). Buffer names are symbols.
                    "BUFFER-CHUNK" => {
                        let names: Vec<String> = items[1..]
                            .iter()
                            .filter_map(|a| a.as_sym().map(|s| s.to_string()))
                            .collect();
                        self.print_buffer_chunk(&names);
                        return Ok(Value::Nil);
                    }
                    "SCHEDULE-EVENT-RELATIVE" | "SCHEDULE-EVENT" => {
                        let relative = h.eq_ignore_ascii_case("schedule-event-relative");
                        let at =
                            self.m_eval(items.get(1).ok_or("schedule-event needs a time")?, b)?;
                        let at_secs = as_num(&at)
                            .ok_or_else(|| "schedule-event time is not a number".to_string())?;
                        let name_v =
                            self.m_eval(items.get(2).ok_or("schedule-event needs a command")?, b)?;
                        let name = match name_v {
                            Value::Str(s) | Value::Sym(s) => s,
                            other => {
                                return Err(format!("bad command name {}", other.show()));
                            }
                        };
                        let mut params = Vec::new();
                        let mut i = 3;
                        while i + 1 < items.len() {
                            if items[i].as_sym() == Some(":PARAMS") {
                                if let Value::List(vs) = self.m_eval(&items[i + 1], b)? {
                                    params = vs;
                                }
                            }
                            i += 2;
                        }
                        let base = if relative { self.time_ms } else { 0 };
                        let at_ms = base + (at_secs * 1000.0).round() as i64;
                        self.push(
                            at_ms,
                            Prio::Num(0),
                            Out::Medium,
                            Event::Command { name, params },
                        );
                        return Ok(Value::Nil);
                    }
                    _ => {}
                }
            }
        }
        self.eval_expr(e, b)
    }

    /// A `goal-focus` argument: a chunk spec list, a symbol, or a string.
    pub(super) fn m_chunk_desc(&self, e: &Sexp) -> Result<ChunkDesc, String> {
        match e {
            Sexp::List(spec) => crate::model::parse_chunk_desc(spec).map_err(|err| err.to_string()),
            Sexp::Sym(n) | Sexp::Str(n) => Ok(ChunkDesc::Named(n.clone())),
            other => Err(format!("bad goal-focus argument {other:?}")),
        }
    }

    /// `set-buffer-chunk <buffer> <(slot value ...)>`: replace the buffer's
    /// chunk silently (no module trace at the tested detail), using the
    /// buffer's `declare-buffer-usage` type for the chunk.
    pub(super) fn set_buffer_chunk_cmd(
        &mut self,
        buffer: &Value,
        chunk: &Value,
    ) -> Result<(), String> {
        let bname = match buffer {
            Value::Sym(s) | Value::Str(s) => s.to_uppercase(),
            other => return Err(format!("set-buffer-chunk buffer {}", other.show())),
        };
        let items = match chunk {
            Value::List(v) => v,
            other => return Err(format!("set-buffer-chunk content {}", other.show())),
        };
        let mut slots: Vec<(String, Value)> = Vec::new();
        for pair in items.chunks(2) {
            let name = match &pair[0] {
                Value::Sym(s) | Value::Str(s) => s.to_uppercase(),
                other => return Err(format!("set-buffer-chunk slot {}", other.show())),
            };
            slots.push((name, pair.get(1).cloned().unwrap_or(Value::Nil)));
        }
        let ctype = self
            .model
            .buffer_decls
            .iter()
            .find(|(b, _)| *b == bname)
            .map(|(_, t)| t.clone())
            .unwrap_or_else(|| "CHUNK".to_string());
        let inst = ChunkInst {
            name: None,
            ctype,
            slots,
            bare: false,
        };
        let buf = self
            .buffers
            .get_mut(&bname)
            .ok_or_else(|| format!("unknown buffer {bname}"))?;
        buf.chunk = Some(inst);
        buf.busy = false;
        buf.failed = false;
        Ok(())
    }

    /// Speech module `+vocal>` request (`core-modules/speech.lisp`):
    /// `isa speak string X` (or `cmd speak`) emits `SPEAK TEXT X` and holds
    /// the vocal buffer busy for the articulation time.
    pub(super) fn vocal_request(&mut self, spec: NewChunk) -> Result<(), EngineError> {
        let (ctype, slots) = match &spec {
            NewChunk::Spec { ctype, slots, .. } => (ctype.clone(), slots.clone()),
            _ => (String::new(), Vec::new()),
        };
        let text = slots
            .iter()
            .find(|(s, _)| s == "STRING")
            .and_then(|(_, v)| match v {
                Value::Str(s) | Value::Sym(s) => Some(s.clone()),
                _ => None,
            });
        // `pm-module-last-cmd-name` (`speech.lisp:341`): the style is the
        // request's `isa` type, or the `cmd` slot's value.
        let style = if ctype.eq_ignore_ascii_case("SPEAK")
            || slots.iter().any(|(s, v)| {
                s == "CMD" && matches!(v, Value::Sym(x) if x.eq_ignore_ascii_case("SPEAK"))
            }) {
            Some(SpeechStyle::Speak)
        } else if ctype.eq_ignore_ascii_case("SUBVOCALIZE")
            || slots.iter().any(|(s, v)| {
                s == "CMD" && matches!(v, Value::Sym(x) if x.eq_ignore_ascii_case("SUBVOCALIZE"))
            })
        {
            Some(SpeechStyle::Subvocalize)
        } else {
            None
        };
        if let (Some(style), Some(t)) = (style, text) {
            // The speech action is scheduled at default priority so the
            // request's implicit clear-buffer precedes it.
            self.now(Prio::Num(0), Out::Medium, Event::Speak { style, text: t });
        }
        Ok(())
    }

    /// `+temporal>` request (`modules/temporal.lisp:186`): `isa clear`
    /// stops the timer; `isa time` (re)starts it, placing `(isa time
    /// ticks 0)` and arming the first tick.
    pub(super) fn temporal_request(&mut self, spec: NewChunk) -> Result<(), EngineError> {
        let (ctype, slots) = match &spec {
            NewChunk::Spec { ctype, slots, .. } => (ctype.clone(), slots.clone()),
            _ => return Err(unimpl("non-spec temporal request")),
        };
        // Any request cancels the pending tick (`delete-event`).
        self.temporal_gen += 1;
        self.queue
            .retain(|q| !matches!(q.event, Event::TemporalTick { .. }));
        let gen = self.temporal_gen;
        let is_clear = ctype.eq_ignore_ascii_case("CLEAR")
            || slots.iter().any(|(s, v)| {
                (s == "CLEAR" && *v != Value::Nil)
                    || (s == "CMD" && matches!(v, Value::Sym(x) if x == "CLEAR"))
            });
        if is_clear {
            self.now(Prio::Num(0), Out::Medium, Event::TemporalClear);
            return Ok(());
        }
        // `schedule-set-buffer-chunk 'temporal (isa time) 0 :priority -1000`
        // — lands after the production's implicit clear.
        self.now(
            Prio::Num(-1000),
            Out::Low,
            Event::SetBuffer {
                buffer: "TEMPORAL".to_string(),
                chunk: NewChunk::Spec {
                    ctype: "TIME".to_string(),
                    slots: vec![("TICKS".to_string(), Value::Int(0))],
                    neg: Vec::new(),
                    rels: Vec::new(),
                    bare: true,
                },
                requested: true,
                shown: None,
            },
        );
        // The start increment is drawn only on the first request
        // (`(when (null time-start-increment) ...)`).
        let start = match self.temporal_start {
            Some(s) => s,
            None => {
                let master = self.cfg.time_start_ms as f64;
                let s = (master
                    + self
                        .rng
                        .borrow_mut()
                        .noise((master * 5.0 * self.cfg.time_noise as f64) as f32)
                        as f64)
                    .max(1.0) as f32;
                self.temporal_start = Some(s);
                s
            }
        };
        let tick = (start as f64 + self.rng.borrow_mut().noise(self.cfg.time_noise * start) as f64)
            .max(1.0) as f32;
        self.temporal_tick = tick;
        self.temporal_ticks = 0;
        let at = self.time_ms + tick.round() as i64;
        self.push(
            at,
            Prio::Min,
            Out::Medium,
            Event::TemporalTick {
                gen,
                details: "Incrementing time ticks to 1".to_string(),
            },
        );
        Ok(())
    }

    /// One temporal tick (`next-time-tick`, `modules/temporal.lisp:231`):
    /// bump the count, grow and jitter the interval, update the buffer, and
    /// arm the next tick.
    pub(super) fn temporal_tick(&mut self, show: bool) -> Result<(), EngineError> {
        let has_ticks = self
            .buffers
            .get("TEMPORAL")
            .and_then(|b| b.chunk.as_ref())
            .map(|c| matches!(c.get("TICKS"), Value::Int(_) | Value::Num(_)))
            .unwrap_or(false);
        if !has_ticks {
            return Ok(());
        }
        self.temporal_ticks += 1;
        let ticks = self.temporal_ticks;
        let mut t = self.temporal_tick * self.cfg.time_mult;
        t = (t + self.rng.borrow_mut().noise(self.cfg.time_noise * t)).max(1.0);
        self.temporal_tick = t;
        if self.cfg.record_ticks && show {
            self.emit("TEMPORAL", "MODULE-MOD-REQUEST TEMPORAL".into());
        }
        if let Some(b) = self.buffers.get_mut("TEMPORAL") {
            if let Some(c) = b.chunk.as_mut() {
                match c.slots.iter_mut().find(|(k, _)| k == "TICKS") {
                    Some((_, old)) => *old = Value::Int(ticks as i64),
                    None => c
                        .slots
                        .push(("TICKS".to_string(), Value::Int(ticks as i64))),
                }
            }
        }
        if show {
            self.emit("TEMPORAL", "MOD-BUFFER-CHUNK TEMPORAL".into());
        }
        let at = self.time_ms + t.round() as i64;
        self.push(
            at,
            Prio::Min,
            Out::Medium,
            Event::TemporalTick {
                gen: self.temporal_gen,
                details: format!("Incrementing time ticks to {}", ticks + 1),
            },
        );
        Ok(())
    }

    /// Seconds from integer ms, printed as Lisp `princ` shows it.
    pub(super) fn secs_str(req_ms: i64) -> String {
        lisp_s_f32(req_ms as f32 / 1000.0)
    }

    /// Virtual keyboard geometry for press-key movements (hand/finger/r/theta
    /// per key name as requested; output uses the table's lowercase name).
    /// M5a seeds Q; digits join with device-task goldens.
    pub(super) fn key_spec(key: &str) -> Result<KeySpec, EngineError> {
        key_table()
            .get(&key.to_uppercase())
            .cloned()
            .ok_or_else(|| unimpl(format!("press-key for unknown key {key}")))
    }

    pub(super) fn instantiate(
        &self,
        _buffer: &str,
        chunk: NewChunk,
    ) -> Result<ChunkInst, EngineError> {
        match chunk {
            NewChunk::Named(name) => self
                .dm
                .iter()
                .find(|c| c.name.as_deref() == Some(name.as_str()))
                .or_else(|| self.named.get(&name))
                .cloned()
                .ok_or_else(|| bad(format!("no chunk named {name}"))),
            NewChunk::Spec {
                ctype, slots, bare, ..
            } => Ok(ChunkInst {
                name: None,
                ctype,
                slots,
                bare,
            }),
            NewChunk::Instance(inst) => Ok(inst),
        }
    }
}

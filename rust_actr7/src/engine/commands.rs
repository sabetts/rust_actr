//! Task-command layer and module request routing.
use super::eval::as_num;
use super::motor::key_table;
use super::*;

/// `add-dm`/`merge-dm` driver steps carry named chunks only. Literal slot
/// definitions use the model loader or the command layer.
fn named_dm_chunk(desc: &ChunkDesc, cmd: &str) -> Result<ChunkInst, EngineError> {
    match desc {
        ChunkDesc::Named(name) => Ok(ChunkInst {
            name: Some(name.clone()),
            ctype: String::new(),
            slots: Vec::new(),
            bare: false,
        }),
        ChunkDesc::Spec { .. } => Err(EngineError::Model(format!(
            "{cmd} step requires a named chunk descriptor"
        ))),
    }
}

/// Minimal `format` for task hooks: `~%`, `~~`, `~S`, and `~A`.
fn format_lisp_simple(fmt: &str, args: &[Value]) -> String {
    let mut out = String::new();
    let mut chars = fmt.chars().peekable();
    let mut ai = 0usize;
    while let Some(c) = chars.next() {
        if c != '~' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => out.push('\n'),
            Some('~') => out.push('~'),
            Some('S') | Some('A') => {
                if let Some(v) = args.get(ai) {
                    out.push_str(&v.show());
                    ai += 1;
                }
            }
            Some(other) => {
                out.push('~');
                out.push(other);
            }
            None => out.push('~'),
        }
    }
    out
}

impl Engine {
    /// Run a scheduled task command: bind its `:params` to the `defun`'s
    /// parameters and execute the body (side-effecting forms handled by
    /// [`Engine::m_eval`]).
    /// Run the model-body `call-act-r-command` forms. ACT-R re-evaluates the
    /// model body on load and on every reset; command errors are warnings,
    /// not model-load failures (`handle-evaluate-results`).
    pub fn apply_model_commands(&mut self) -> Result<(), EngineError> {
        let commands = self.model.call_commands.clone();
        for (name, args) in commands {
            if self.user_fns.is_empty() {
                continue;
            }
            let mut values = Vec::new();
            let empty = Bindings::new();
            for arg in &args {
                values.push(self.m_eval(arg, &empty).map_err(bad)?);
            }
            if !self.user_fns.contains_key(&name.to_uppercase()) {
                self.emit_warning(&format!(
                    "No command named {name} for model-body call-act-r-command."
                ));
                continue;
            }
            if let Err(e) = self.run_command(&name, values) {
                self.emit_warning(&format!("Error executing command {name}: {e}"));
            }
        }
        Ok(())
    }

    /// `ppm-offset`'s `:ppm-hook` call: task function receives the production
    /// name and `(buffer slot expected actual similarity)` rows. A numeric
    /// result overrides the normal PPM offset.
    pub(super) fn ppm_hook_offset(
        &mut self,
        production: &str,
        mismatches: &[PpmMismatch],
    ) -> Option<f32> {
        let hook = self.cfg.ppm_hook.clone()?;
        let def = self.user_fns.get(&hook.to_uppercase())?.clone();
        let p = Value::Sym(production.to_uppercase());
        let rows = Value::List(
            mismatches
                .iter()
                .map(|(buffer, slot, want, got, sim)| {
                    Value::List(vec![
                        Value::Sym(buffer.to_uppercase()),
                        Value::Sym(slot.to_uppercase()),
                        want.clone(),
                        got.clone(),
                        Value::Num(*sim as f64),
                    ])
                })
                .collect(),
        );
        let mut b: Bindings = HashMap::new();
        for (param, value) in def.params.iter().zip([p, rows]) {
            b.insert(param.to_uppercase(), value);
        }
        let mut result = Value::Nil;
        for e in &def.body {
            result = self.m_eval(e, &b).ok()?;
        }
        match result {
            Value::Num(x) => Some(x as f32),
            Value::Int(i) => Some(i as f32),
            _ => None,
        }
    }

    pub(super) fn run_command(
        &mut self,
        name: &str,
        params: Vec<Value>,
    ) -> Result<(), EngineError> {
        let key = name.to_uppercase();
        if key == "SET-IMAGINAL-FREE" {
            self.imag_busy = false;
            self.buffer_mut("IMAGINAL").busy = false;
            return Ok(());
        }
        if key == "SET-IMAGINAL-ERROR" {
            self.imag_busy = false;
            let b = self.buffer_mut("IMAGINAL");
            b.busy = false;
            b.failed = true;
            self.changed();
            return Ok(());
        }
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
                    "CAR" | "FIRST" => {
                        let v = self.m_eval(items.get(1).ok_or("car needs a list")?, b)?;
                        return Ok(match v {
                            Value::List(v) => v.into_iter().next().unwrap_or(Value::Nil),
                            Value::Nil => Value::Nil,
                            other => return Err(format!("car of {}", other.show())),
                        });
                    }
                    "FIFTH" => {
                        let v = self.m_eval(items.get(1).ok_or("fifth needs a list")?, b)?;
                        return Ok(match v {
                            Value::List(v) => v.into_iter().nth(4).unwrap_or(Value::Nil),
                            Value::Nil => Value::Nil,
                            other => return Err(format!("fifth of {}", other.show())),
                        });
                    }
                    "MODEL-OUTPUT" | "COMMAND-OUTPUT" => {
                        let fmt =
                            self.m_eval(items.get(1).ok_or("model-output needs a format")?, b)?;
                        let fmt = match fmt {
                            Value::Str(s) | Value::Sym(s) => s,
                            other => other.show(),
                        };
                        let mut args = Vec::new();
                        for a in &items[2..] {
                            args.push(self.m_eval(a, b)?);
                        }
                        let text = format_lisp_simple(&fmt, &args);
                        self.model_output(&text);
                        return Ok(Value::Nil);
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
                    "QUASIQUOTE" => {
                        let body = items.get(1).ok_or("quasiquote needs a body")?;
                        return self.eval_quasi(body, b);
                    }
                    "DOLIST" => {
                        let spec = match items.get(1) {
                            Some(Sexp::List(v)) => v,
                            _ => return Err("dolist needs a binding list".to_string()),
                        };
                        let var = spec
                            .first()
                            .and_then(|x| x.as_sym())
                            .ok_or_else(|| "dolist needs a variable".to_string())?;
                        let list_expr = spec.get(1).ok_or("dolist needs a list")?;
                        let list = self.m_eval(list_expr, b)?;
                        let values = match list {
                            Value::List(v) => v,
                            Value::Nil => Vec::new(),
                            other => {
                                return Err(format!("dolist list is {}", other.show()));
                            }
                        };
                        for value in values {
                            let mut local = b.clone();
                            local.insert(var.to_uppercase(), value);
                            for form in &items[2..] {
                                self.m_eval(form, &local)?;
                            }
                        }
                        return match spec.get(2) {
                            Some(result) => self.m_eval(result, b),
                            None => Ok(Value::Nil),
                        };
                    }
                    "MAPCAN" => {
                        let func =
                            self.m_eval(items.get(1).ok_or("mapcan needs a function")?, b)?;
                        let mut lists = Vec::new();
                        for form in &items[2..] {
                            match self.m_eval(form, b)? {
                                Value::List(v) => lists.push(v),
                                Value::Nil => lists.push(Vec::new()),
                                other => return Err(format!("mapcan list is {}", other.show())),
                            }
                        }
                        let n = lists.iter().map(Vec::len).min().unwrap_or(0);
                        let mut out = Vec::new();
                        for i in 0..n {
                            let args: Vec<Value> = lists.iter().map(|l| l[i].clone()).collect();
                            let value = match &func {
                                Value::Sym(name) if name.eq_ignore_ascii_case("LIST") => {
                                    Value::List(args)
                                }
                                Value::Sym(name) => self
                                    .call_user_fn(name, &args)
                                    .ok_or_else(|| format!("mapcan unknown function {name}"))?,
                                other => {
                                    return Err(format!("mapcan function {}", other.show()));
                                }
                            };
                            match value {
                                Value::List(v) => out.extend(v),
                                Value::Nil => {}
                                other => out.push(other),
                            }
                        }
                        return Ok(Value::List(out));
                    }
                    "EXTEND-POSSIBLE-SLOTS" => {
                        let value = self
                            .m_eval(items.get(1).ok_or("extend-possible-slots needs a slot")?, b)?;
                        let name = match value {
                            Value::Sym(s) | Value::Str(s) => s.to_uppercase(),
                            other => {
                                return Err(format!("extend-possible-slots slot {}", other.show()));
                            }
                        };
                        self.valid_slots.insert(name.clone());
                        return Ok(Value::Sym(name));
                    }
                    "DEFINE-CHUNKS" | "DEFINE-CHUNKS-FCT" | "ADD-DM-FCT" => {
                        // `define-chunks` is a macro: its argument is a raw
                        // chunk-definition form. The `-fct` variants evaluate
                        // their argument first.
                        let defs = if h.eq_ignore_ascii_case("DEFINE-CHUNKS") {
                            Value::from_sexp(items.get(1).ok_or("chunk definition list expected")?)
                                .map_err(|e| e.to_string())?
                        } else {
                            self.m_eval(items.get(1).ok_or("chunk definition list expected")?, b)?
                        };
                        let defs = match defs {
                            Value::List(v) => {
                                // One anonymous spec `(isa type ...)` is a
                                // single definition; a list whose first item is
                                // a list is already the definition list.
                                let anon = v.first().is_some_and(
                                    |x| matches!(x, Value::Sym(s) if s.eq_ignore_ascii_case("ISA")),
                                );
                                if anon {
                                    vec![Value::List(v)]
                                } else {
                                    v
                                }
                            }
                            Value::Nil => Vec::new(),
                            other => return Err(format!("chunk definitions {}", other.show())),
                        };
                        let add_dm = h.eq_ignore_ascii_case("ADD-DM-FCT");
                        let names = self.define_chunks_values(&defs, add_dm)?;
                        return Ok(Value::List(names.into_iter().map(Value::Sym).collect()));
                    }
                    "SETF" | "SETQ" => {
                        let mut i = 1;
                        while i + 1 < items.len() {
                            let name = items[i]
                                .as_sym()
                                .ok_or_else(|| "setf needs a variable".to_string())?;
                            let value = self.m_eval(&items[i + 1], b)?;
                            self.user_vars.insert(name.to_uppercase(), value);
                            i += 2;
                        }
                        return Ok(Value::Nil);
                    }
                    "INCF" => {
                        let name = items
                            .get(1)
                            .and_then(|x| x.as_sym())
                            .ok_or_else(|| "incf needs a variable".to_string())?;
                        let Some(old) = self.user_vars.get(&name.to_uppercase()).cloned() else {
                            return Ok(Value::Nil);
                        };
                        let value = match old {
                            Value::Int(i) => Value::Int(i + 1),
                            Value::Num(x) => Value::Num(x + 1.0),
                            _ => return Ok(Value::Nil),
                        };
                        self.user_vars.insert(name.to_uppercase(), value.clone());
                        return Ok(value);
                    }
                    // The task's `move-it` is scheduled by the model's
                    // `!eval! (schedule-event-relative … 'move-it)`. Its
                    // effect on the window is replayed from the captured
                    // device op (`MODIFY-TEXT`), so the body is a no-op here.
                    "MODIFY-TEXT-FOR-EXP-WINDOW" => {
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
                    "SCHEDULE-MOD-BUFFER-CHUNK" => {
                        let bv = self.m_eval(
                            items
                                .get(1)
                                .ok_or("schedule-mod-buffer-chunk needs a buffer")?,
                            b,
                        )?;
                        let buffer = match bv {
                            Value::Sym(s) | Value::Str(s) => s.to_uppercase(),
                            other => {
                                return Err(format!("bad buffer name {}", other.show()));
                            }
                        };
                        let mods = match self.m_eval(
                            items
                                .get(2)
                                .ok_or("schedule-mod-buffer-chunk needs slots")?,
                            b,
                        )? {
                            Value::List(v) => v,
                            Value::Nil => Vec::new(),
                            other => return Err(format!("bad mod slots {}", other.show())),
                        };
                        let mut slots = Vec::new();
                        for pair in mods.chunks(2) {
                            let name = match &pair[0] {
                                Value::Sym(s) | Value::Str(s) => s.to_uppercase(),
                                other => return Err(format!("bad mod slot {}", other.show())),
                            };
                            slots.push((name, pair.get(1).cloned().unwrap_or(Value::Nil)));
                        }
                        let delay = self.m_eval(
                            items
                                .get(3)
                                .ok_or("schedule-mod-buffer-chunk needs a delay")?,
                            b,
                        )?;
                        let delay = as_num(&delay)
                            .ok_or_else(|| "mod delay is not a number".to_string())?;
                        let module = buffer.clone();
                        self.push(
                            self.time_ms + (delay * 1000.0).round() as i64,
                            Prio::Num(0),
                            Out::Medium,
                            Event::ModBuffer {
                                buffer,
                                slots,
                                module,
                            },
                        );
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
                        // The imaginal module's state-maintenance commands are
                        // scheduled with `:output nil` in ACT-R.
                        let out = if name.eq_ignore_ascii_case("SET-IMAGINAL-FREE")
                            || name.eq_ignore_ascii_case("SET-IMAGINAL-ERROR")
                        {
                            Out::Never
                        } else {
                            Out::Medium
                        };
                        self.push(at_ms, Prio::Num(0), out, Event::Command { name, params });
                        return Ok(Value::Nil);
                    }
                    // `(let* ((var expr) ...) body ...)`: sequential local
                    // bindings, for a task action body (`compute-difference`).
                    "LET" | "LET*" => {
                        let binds = match items.get(1) {
                            Some(Sexp::List(l)) => l,
                            other => {
                                return Err(format!("let needs a binding list, got {other:?}"))
                            }
                        };
                        let mut local = b.clone();
                        for bind in binds {
                            match bind {
                                Sexp::List(pair) if pair.len() == 2 => {
                                    let n = pair[0]
                                        .as_sym()
                                        .ok_or_else(|| "let binding needs a name".to_string())?;
                                    let v = self.m_eval(&pair[1], &local)?;
                                    local.insert(n.to_uppercase(), v);
                                }
                                Sexp::Sym(n) => {
                                    local.insert(n.to_uppercase(), Value::Nil);
                                }
                                other => {
                                    return Err(format!("bad let binding {other:?}"));
                                }
                            }
                        }
                        let mut out = Value::Nil;
                        for e in &items[2..] {
                            out = self.m_eval(e, &local)?;
                        }
                        return Ok(out);
                    }
                    // `copy-chunk-fct` (`chunks.lisp:860`): a named copy
                    // `{name}-{k}` (`copy_chunk`).
                    "COPY-CHUNK-FCT" => {
                        let v =
                            self.m_eval(items.get(1).ok_or("copy-chunk-fct needs a chunk")?, b)?;
                        let name = match v {
                            Value::Sym(s) | Value::Str(s) => s,
                            other => return Err(format!("copy-chunk-fct chunk {}", other.show())),
                        };
                        return Ok(Value::Str(self.copy_chunk(&name)));
                    }
                    // `pprint-chunks-fct` (`chunks.lisp:736`): print the
                    // named chunks into the trace.
                    "PPRINT-CHUNKS-FCT" => {
                        let v =
                            self.m_eval(items.get(1).ok_or("pprint-chunks-fct needs names")?, b)?;
                        let names: Vec<String> = match v {
                            Value::List(v) => v
                                .into_iter()
                                .filter_map(|x| match x {
                                    Value::Sym(s) | Value::Str(s) => Some(s),
                                    _ => None,
                                })
                                .collect(),
                            Value::Sym(s) | Value::Str(s) => vec![s],
                            _ => Vec::new(),
                        };
                        self.print_pprint_chunks(&names);
                        return Ok(Value::Nil);
                    }
                    // `mod-chunk-fct` (`chunks.lisp:1054`): set slots on a
                    // named chunk, returning its name.
                    "MOD-CHUNK-FCT" => {
                        let v =
                            self.m_eval(items.get(1).ok_or("mod-chunk-fct needs a chunk")?, b)?;
                        let name = match v {
                            Value::Sym(s) | Value::Str(s) => s.to_uppercase(),
                            other => return Err(format!("mod-chunk-fct chunk {}", other.show())),
                        };
                        let slots = match self
                            .m_eval(items.get(2).ok_or("mod-chunk-fct needs slots")?, b)?
                        {
                            Value::List(l) => l,
                            other => return Err(format!("mod-chunk-fct slots {}", other.show())),
                        };
                        self.mod_named_chunk(&name, &slots);
                        return Ok(Value::Str(name));
                    }
                    _ => {}
                }
            }
        }
        self.eval_expr(e, b)
    }

    fn eval_quasi(&mut self, e: &Sexp, b: &Bindings) -> Result<Value, String> {
        if let Sexp::List(items) = e {
            if items.first().and_then(|x| x.as_sym()) == Some("UNQUOTE") {
                return self.m_eval(items.get(1).ok_or("unquote needs a body")?, b);
            }
            let mut out = Vec::new();
            for item in items {
                if let Sexp::List(parts) = item {
                    if parts.first().and_then(|x| x.as_sym()) == Some("UNQUOTE-SPLICING") {
                        let value =
                            self.m_eval(parts.get(1).ok_or("unquote-splicing needs a body")?, b)?;
                        match value {
                            Value::List(v) => out.extend(v),
                            Value::Nil => {}
                            other => {
                                return Err(format!("unquote-splicing produced {}", other.show()));
                            }
                        }
                        continue;
                    }
                }
                out.push(self.eval_quasi(item, b)?);
            }
            return Ok(Value::List(out));
        }
        Ok(Value::from_sexp(e).map_err(|e| e.to_string())?)
    }

    fn new_name(&mut self, prefix: &str) -> String {
        let key = prefix.to_uppercase();
        let n = *self.name_count.entry(key.clone()).or_insert(0);
        self.name_count.insert(key.clone(), n + 1);
        format!("{key}{n}")
    }

    fn chunk_pair(&self, items: &[Value]) -> Result<Vec<(String, Value)>, String> {
        if items.len() % 2 != 0 {
            return Err("chunk definition needs slot/value pairs".to_string());
        }
        let mut slots = Vec::new();
        for pair in items.chunks(2) {
            let name = match &pair[0] {
                Value::Sym(s) | Value::Str(s) => s.to_uppercase(),
                other => return Err(format!("chunk slot name {}", other.show())),
            };
            slots.push((name, pair.get(1).cloned().unwrap_or(Value::Nil)));
        }
        Ok(slots)
    }

    fn define_chunks_values(
        &mut self,
        defs: &[Value],
        add_dm: bool,
    ) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        for def in defs {
            let (name, ctype, mut slots, explicit) = match def {
                Value::Sym(n) => (n.clone(), None, Vec::new(), true),
                Value::List(items) => {
                    let first = items.first().and_then(|v| match v {
                        Value::Sym(s) | Value::Str(s) => Some(s.clone()),
                        _ => None,
                    });
                    if first.as_deref() == Some("ISA") {
                        let ctype = items
                            .get(1)
                            .and_then(|v| match v {
                                Value::Sym(s) | Value::Str(s) => Some(s.to_uppercase()),
                                _ => None,
                            })
                            .ok_or_else(|| "chunk definition needs a type".to_string())?;
                        (
                            String::new(),
                            Some(ctype),
                            self.chunk_pair(&items[2..])?,
                            false,
                        )
                    } else if items.get(1).and_then(|v| match v {
                        Value::Sym(s) | Value::Str(s) => Some(s.eq_ignore_ascii_case("ISA")),
                        _ => None,
                    }) == Some(true)
                    {
                        let name =
                            first.ok_or_else(|| "chunk definition needs a name".to_string())?;
                        let ctype = items
                            .get(2)
                            .and_then(|v| match v {
                                Value::Sym(s) | Value::Str(s) => Some(s.to_uppercase()),
                                _ => None,
                            })
                            .ok_or_else(|| "chunk definition needs a type".to_string())?;
                        (name, Some(ctype), self.chunk_pair(&items[3..])?, true)
                    } else if let Some(name) = first {
                        (name, None, self.chunk_pair(&items[1..])?, true)
                    } else {
                        return Err(format!("bad chunk definition {}", def.show()));
                    }
                }
                other => return Err(format!("bad chunk definition {}", other.show())),
            };
            if let Some(ct) = ctype.as_deref().and_then(|t| {
                self.model
                    .chunk_types
                    .iter()
                    .find(|ct| ct.name.eq_ignore_ascii_case(t))
            }) {
                for (slot, default) in &ct.defaults {
                    if !slots.iter().any(|(s, _)| s == slot) {
                        slots.push((
                            slot.clone(),
                            Value::from_pattern(default).map_err(|e| e.to_string())?,
                        ));
                    }
                }
            }
            let name = if explicit && !name.is_empty() {
                name
            } else {
                self.new_name(ctype.as_deref().unwrap_or("CHUNK"))
            };
            let inst = ChunkInst {
                name: Some(name.clone()),
                ctype: ctype.unwrap_or_default(),
                slots,
                bare: false,
            };
            if add_dm {
                self.add_runtime_dm(inst);
            } else {
                self.named.insert(name.clone(), inst);
                self.chunk_names.insert(name.clone());
            }
            names.push(name);
        }
        Ok(names)
    }

    /// `(add-dm (name ...))` for a driver step: add a named empty chunk.
    pub fn add_dm_desc(&mut self, desc: &ChunkDesc) -> Result<(), EngineError> {
        self.add_runtime_dm(named_dm_chunk(desc, "add-dm")?);
        Ok(())
    }

    /// `(merge-dm (name ...))` for a driver step: merge a named empty chunk
    /// into DM by content, adding a reference on an existing match.
    pub fn merge_dm_desc(&mut self, desc: &ChunkDesc) -> Result<(), EngineError> {
        self.merge_into_dm(named_dm_chunk(desc, "merge-dm")?, "MERGE-DM");
        Ok(())
    }

    /// `(module-request '<buffer> (define-chunk-spec))` for a driver step.
    /// The empty spec matches every chunk in the module.
    pub fn empty_module_request(&mut self, buffer: &str) -> Result<(), EngineError> {
        if buffer.eq_ignore_ascii_case("RETRIEVAL") {
            if self.retrieval_pending {
                self.model_warning("A retrieval event has been aborted by a new request");
            }
            self.retrieval_pending = true;
        }
        let spec = NewChunk::Spec {
            ctype: String::new(),
            slots: Vec::new(),
            neg: Vec::new(),
            rels: Vec::new(),
            bare: true,
        };
        self.now(
            Prio::Num(0),
            Out::Medium,
            Event::ModuleRequest {
                buffer: buffer.to_uppercase(),
                spec,
                var_slots: Vec::new(),
                trace: false,
            },
        );
        Ok(())
    }

    pub(super) fn add_runtime_dm(&mut self, inst: ChunkInst) {
        let name = inst.name.clone().unwrap_or_default();
        if self
            .dm
            .iter()
            .any(|c| c.name.as_deref() == Some(name.as_str()))
        {
            return;
        }
        self.chunk_names.insert(name.clone());
        self.dm.push(inst);
        self.hist.push(History {
            creation_ms: self.time_ms,
            refs: vec![self.time_ms],
            count: 1,
            decays: None,
            fixed_base: None,
            last_retrieval: None,
        });
        *self.fan_out.entry(name.clone()).or_insert(0) += 1;
        let mut counts: HashMap<String, u32> = HashMap::new();
        let slots = self.dm.last().map(|c| c.slots.clone()).unwrap_or_default();
        for (_, value) in &slots {
            if let Some(source) = self.chunk_name_of(value) {
                *self.fan_out.entry(source.clone()).or_insert(0) += 1;
                *counts.entry(source).or_insert(0) += 1;
            }
        }
        *counts.entry(name.clone()).or_insert(0) += 1;
        self.fan_in.insert(name, counts);
    }

    /// Apply a same-time `=imaginal>` modification queued by the production
    /// that also requested an imaginal action. ACT-R's action sees the updated
    /// slots; the modification is invisible in the trace (the module-request
    /// line is the next visible event), so consume the queued event silently.
    pub(super) fn apply_pending_imaginal_mod(&mut self) {
        let now = self.time_ms;
        let Some(slots) = self.queue.iter().find_map(|q| match &q.event {
            Event::ModBuffer { buffer, slots, .. } if buffer == "IMAGINAL" && q.time_ms == now => {
                Some(slots.clone())
            }
            _ => None,
        }) else {
            return;
        };
        self.queue.retain(|q| {
            !matches!(&q.event, Event::ModBuffer { buffer, .. }
                if buffer == "IMAGINAL" && q.time_ms == now)
        });
        if let Some(chunk) = self
            .buffers
            .get_mut("IMAGINAL")
            .and_then(|b| b.chunk.as_mut())
        {
            for (slot, value) in slots {
                match chunk.slots.iter_mut().find(|(k, _)| *k == slot) {
                    Some((_, old)) => *old = value,
                    None => chunk.slots.push((slot, value)),
                }
            }
        }
        self.buffer_mut("IMAGINAL").modified = true;
    }

    /// `imaginal-action` request (`imaginal.lisp:343`): a production's
    /// `+imaginal-action>` asks a task function to compute the imaginal
    /// buffer's next chunk. A `simple` action clears the imaginal buffer,
    /// then places the function's result (a chunk name or a slot list) after
    /// `:imaginal-delay`; a generic action just runs the function, which
    /// schedules its own work and frees the module.
    pub(super) fn imaginal_action_request(&mut self, spec: NewChunk) -> Result<(), EngineError> {
        if self.imag_busy {
            return Ok(());
        }
        let (ctype, slots) = match &spec {
            NewChunk::Spec { ctype, slots, .. } => (ctype.clone(), slots.clone()),
            _ => return Ok(()),
        };
        let get = |k: &str| slots.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        let Some(action) = get("ACTION") else {
            return Ok(());
        };
        let name = match action {
            Value::Str(s) | Value::Sym(s) => s,
            _ => return Ok(()),
        };
        let extra: Vec<Value> = match get("SLOTS") {
            Some(Value::List(v)) => v,
            _ => Vec::new(),
        };
        // `simple-action` declares `(simple t)` as a chunk-type default
        // (`imaginal.lisp:267`); core-module types are not part of the
        // model's declared chunk types, so apply that default here.
        let simple = get("SIMPLE")
            .map(|v| v != Value::Nil)
            .unwrap_or_else(|| ctype.eq_ignore_ascii_case("SIMPLE-ACTION"));
        // The action runs at request time, with the module marked busy.
        self.imag_busy = true;
        let result = self.call_action(&name, &extra);
        if !simple {
            return Ok(());
        }
        // simple action: clear the imaginal buffer now, place the result (or
        // the error state) after the delay.
        self.now(
            Prio::Num(0),
            Out::Medium,
            Event::ClearBuffer {
                buffer: "IMAGINAL".to_string(),
                module: "IMAGINAL".to_string(),
            },
        );
        let delay = if self.cfg.vidt {
            self.randomize_ms(self.cfg.imaginal_delay_ms)
        } else {
            self.cfg.imaginal_delay_ms
        };
        let at = self.time_ms + delay;
        match result {
            Some(chunk) => {
                self.push(
                    at,
                    Prio::Num(-1000),
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "IMAGINAL".to_string(),
                        chunk,
                        requested: true,
                        shown: None,
                    },
                );
                self.push_m(at, Prio::Num(-1001), Out::Never, true, Event::ImaginalFree);
            }
            None => self.push(at, Prio::Num(0), Out::Never, Event::ImaginalError),
        }
        Ok(())
    }

    /// Run an imaginal-action function with already-evaluated arguments and
    /// turn its return value into the chunk to place: a chunk name, a
    /// `(slot value ...)` list, or `nil` (no chunk).
    fn call_action(&mut self, name: &str, args: &[Value]) -> Option<NewChunk> {
        let def = self.user_fns.get(&name.to_uppercase()).cloned()?;
        let mut b: Bindings = HashMap::new();
        for (p, v) in def.params.iter().zip(args) {
            b.insert(p.to_uppercase(), v.clone());
        }
        let mut out = Value::Nil;
        for e in &def.body {
            out = self.m_eval(e, &b).ok()?;
        }
        match out {
            Value::Nil => None,
            Value::Str(s) | Value::Sym(s) => Some(NewChunk::Named(s)),
            Value::List(v) => {
                let mut slots = Vec::new();
                for pair in v.chunks(2) {
                    let k = match &pair[0] {
                        Value::Sym(s) | Value::Str(s) => s.to_uppercase(),
                        _ => return None,
                    };
                    slots.push((k, pair.get(1).cloned().unwrap_or(Value::Nil)));
                }
                Some(NewChunk::Spec {
                    ctype: "CHUNK".to_string(),
                    slots,
                    neg: Vec::new(),
                    rels: Vec::new(),
                    bare: true,
                })
            }
            _ => None,
        }
    }

    /// `mod-chunk-fct` on a named chunk: overwrite or add each `slot value`.
    pub(super) fn mod_named_chunk(&mut self, name: &str, slots: &[Value]) {
        let Some(c) = self.named.get_mut(name) else {
            return;
        };
        for pair in slots.chunks(2) {
            let k = match &pair[0] {
                Value::Sym(s) | Value::Str(s) => s.to_uppercase(),
                _ => continue,
            };
            let v = pair.get(1).cloned().unwrap_or(Value::Nil);
            match c.slots.iter_mut().find(|(n, _)| n == &k) {
                Some(s) => s.1 = v,
                None => c.slots.push((k, v)),
            }
        }
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
            // Preparation can begin while the previous utterance is still
            // executing; `SpeechPrep` queues the execution behind
            // `vocal_end_ms`.
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
    /// Q is seeded; digits join with device-task traces.
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

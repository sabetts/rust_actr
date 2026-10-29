//! Bounded `!eval!`/`!bind!` evaluator.
use super::*;
impl Engine {
    /// LHS/RHS `!eval!`/`!bind!` expressions, evaluated against bindings.
    pub(super) fn eval_expr(&self, e: &Sexp, b: &Bindings) -> Result<Value, String> {
        match e {
            Sexp::Int(i) => Ok(Value::Int(*i)),
            Sexp::Float(x) => Ok(Value::Num(*x)),
            Sexp::Str(s) => Ok(Value::Str(s.clone())),
            Sexp::Sym(s) => self.eval_symbol(s, b),
            Sexp::List(items) => {
                if items.is_empty() {
                    return Err("empty ()".to_string());
                }
                if let Some(h) = items[0].as_sym() {
                    if h.eq_ignore_ascii_case("quote") {
                        return self.quote_value(items.get(1));
                    }
                }
                self.eval_call(&items[0], &items[1..], b)
            }
        }
    }

    pub(super) fn eval_symbol(&self, s: &str, b: &Bindings) -> Result<Value, String> {
        if s == "T" {
            return Ok(Value::Sym("T".to_string()));
        }
        if s == "NIL" {
            return Ok(Value::Nil);
        }
        if let Some(var) = s.strip_prefix('=') {
            if let Some(v) = b.get(var) {
                return Ok(v.clone());
            }
            if self.buffers.contains_key(var) {
                // A buffer reference (`=goal`) denotes the buffer's chunk;
                // keep it as a marker resolved by `chunk-slot-value-fct`.
                return Ok(Value::Sym(format!("={var}")));
            }
            return Err(format!("unbound variable {s}"));
        }
        // A bare symbol bound as a task-file `defun` parameter; otherwise
        // a task-file global, else literal data (e.g. a quoted symbol).
        if let Some(v) = b.get(s) {
            return Ok(v.clone());
        }
        if let Some(v) = self.user_vars.get(&s.to_uppercase()) {
            return Ok(v.clone());
        }
        Ok(Value::Sym(s.to_string()))
    }

    pub(super) fn quote_value(&self, e: Option<&Sexp>) -> Result<Value, String> {
        match e {
            None => Ok(Value::Nil),
            Some(Sexp::Sym(s)) => Ok(Value::Sym(s.clone())),
            Some(Sexp::Int(i)) => Ok(Value::Int(*i)),
            Some(Sexp::Float(x)) => Ok(Value::Num(*x)),
            Some(Sexp::Str(s)) => Ok(Value::Str(s.clone())),
            Some(Sexp::List(_)) => Err("quoted lists are not supported".to_string()),
        }
    }

    /// Evaluate an expression that may yield multiple values (`values`).
    pub(super) fn eval_values(&self, e: &Sexp, b: &Bindings) -> Result<Vec<Value>, String> {
        if let Sexp::List(items) = e {
            if let Some(head) = items.first() {
                if head
                    .as_sym()
                    .map(|s| s.eq_ignore_ascii_case("values"))
                    .unwrap_or(false)
                {
                    let mut out = Vec::new();
                    for a in &items[1..] {
                        out.push(self.eval_expr(a, b)?);
                    }
                    return Ok(out);
                }
                if let Some(def) = self.user_fn_for_head(head) {
                    return self.user_fn_values(def, &items[1..], b);
                }
            }
        }
        Ok(vec![self.eval_expr(e, b)?])
    }

    /// Look up a task-file `defun` by a symbol or string head.
    pub(super) fn user_fn_for_head(&self, head: &Sexp) -> Option<&crate::model::UserDefun> {
        let name = match head {
            Sexp::Sym(s) | Sexp::Str(s) => s,
            _ => return None,
        };
        self.user_fns.get(&name.to_uppercase())
    }

    /// Call a task-file `defun`: evaluate args in the caller's scope, bind
    /// parameters, and evaluate the body (last expression may be `values`).
    pub(super) fn user_fn_values(
        &self,
        def: &crate::model::UserDefun,
        args: &[Sexp],
        b: &Bindings,
    ) -> Result<Vec<Value>, String> {
        let mut local = b.clone();
        for (i, p) in def.params.iter().enumerate() {
            let v = match args.get(i) {
                Some(a) => self.eval_expr(a, b)?,
                None => Value::Nil,
            };
            local.insert(p.to_uppercase(), v);
        }
        let mut out = vec![Value::Nil];
        for (i, e) in def.body.iter().enumerate() {
            if i + 1 == def.body.len() {
                out = self.eval_values(e, &local)?;
            } else {
                self.eval_expr(e, &local)?;
            }
        }
        Ok(out)
    }

    /// Call a task `defun` by name with already-evaluated arguments,
    /// returning its first value. Used by `:sim-hook`'s similarity function
    /// (`bst-ppm`'s `number-sims`). `None` when the function is unknown or
    /// its body errors.
    pub(super) fn call_user_fn(&self, name: &str, args: &[Value]) -> Option<Value> {
        let def = self.user_fns.get(&name.to_uppercase())?;
        let mut b: Bindings = HashMap::new();
        for (p, v) in def.params.iter().zip(args) {
            b.insert(p.to_uppercase(), v.clone());
        }
        let mut out = Value::Nil;
        for (i, e) in def.body.iter().enumerate() {
            if i + 1 == def.body.len() {
                out = self
                    .eval_values(e, &b)
                    .ok()?
                    .into_iter()
                    .next()
                    .unwrap_or(Value::Nil);
            } else {
                self.eval_expr(e, &b).ok()?;
            }
        }
        Some(out)
    }

    pub(super) fn eval_call(
        &self,
        head: &Sexp,
        args: &[Sexp],
        b: &Bindings,
    ) -> Result<Value, String> {
        if let Some(def) = self.user_fn_for_head(head) {
            let vals = self.user_fn_values(def, args, b)?;
            return Ok(vals.into_iter().next().unwrap_or(Value::Nil));
        }
        if let Sexp::Str(name) = head {
            return self.eval_command(name, args, b);
        }
        let head = head.as_sym().ok_or("expression head is not a symbol")?;
        match head.to_uppercase().as_str() {
            "+" | "-" | "*" | "/" => {
                let nums = self.eval_nums(args, b)?;
                if nums.is_empty() {
                    return Err(format!("{head} needs arguments"));
                }
                let r = match head {
                    "+" => nums.iter().sum::<f64>(),
                    "*" => nums.iter().product::<f64>(),
                    "-" => {
                        if nums.len() == 1 {
                            -nums[0]
                        } else {
                            nums[1..].iter().fold(nums[0], |a, x| a - x)
                        }
                    }
                    _ => {
                        if nums.len() == 1 {
                            1.0 / nums[0]
                        } else {
                            nums[1..].iter().fold(nums[0], |a, x| a / x)
                        }
                    }
                };
                Ok(self.num_value(r))
            }
            "1+" => Ok(self.num_value(self.eval_num(args, b)? + 1.0)),
            "1-" => Ok(self.num_value(self.eval_num(args, b)? - 1.0)),
            "ABS" => Ok(self.num_value(self.eval_num(args, b)?.abs())),
            "MAX" | "MIN" => {
                let nums = self.eval_nums(args, b)?;
                let first = *nums.first().ok_or("max/min needs arguments")?;
                Ok(self.num_value(if head.eq_ignore_ascii_case("MAX") {
                    nums.into_iter().fold(first, f64::max)
                } else {
                    nums.into_iter().fold(first, f64::min)
                }))
            }
            ">" | "<" | ">=" | "<=" | "=" | "/=" => {
                let nums = self.eval_nums(args, b)?;
                if nums.len() < 2 {
                    return Err(format!("{head} needs two numbers"));
                }
                let ok = match head {
                    ">" => nums[0] > nums[1],
                    "<" => nums[0] < nums[1],
                    ">=" => nums[0] >= nums[1],
                    "<=" => nums[0] <= nums[1],
                    "=" => nums[0] == nums[1],
                    _ => nums[0] != nums[1],
                };
                Ok(bool_value(ok))
            }
            "EQ" | "EQL" | "EQUAL" | "EQUALP" => {
                if args.len() < 2 {
                    return Err(format!("{head} needs two arguments"));
                }
                let a = self.eval_expr(&args[0], b)?;
                let c = self.eval_expr(&args[1], b)?;
                Ok(bool_value(a == c))
            }
            "NOT" | "NULL" => {
                let v = self.eval_expr(args.first().ok_or("not needs an argument")?, b)?;
                Ok(bool_value(v == Value::Nil))
            }
            "NUMBERP" => {
                let v = self.eval_expr(args.first().ok_or("numberp needs an argument")?, b)?;
                Ok(bool_value(matches!(v, Value::Int(_) | Value::Num(_))))
            }
            "SYMBOLP" => {
                let v = self.eval_expr(args.first().ok_or("symbolp needs an argument")?, b)?;
                Ok(bool_value(matches!(v, Value::Sym(_))))
            }
            "STRINGP" => {
                let v = self.eval_expr(args.first().ok_or("stringp needs an argument")?, b)?;
                Ok(bool_value(matches!(v, Value::Str(_))))
            }
            // `when`/`unless`: evaluate the body only if the test passes,
            // returning the last body value (`nil` otherwise).
            "WHEN" | "UNLESS" => {
                let test = self.eval_expr(args.first().ok_or("when needs a test")?, b)?;
                let pass = if head.eq_ignore_ascii_case("WHEN") {
                    test != Value::Nil
                } else {
                    test == Value::Nil
                };
                if !pass {
                    return Ok(Value::Nil);
                }
                let mut out = Value::Nil;
                for e in &args[1..] {
                    out = self.eval_expr(e, b)?;
                }
                Ok(out)
            }
            "AND" => {
                for a in args {
                    if self.eval_expr(a, b)? == Value::Nil {
                        return Ok(Value::Nil);
                    }
                }
                Ok(Value::Sym("T".to_string()))
            }
            "OR" => {
                for a in args {
                    let v = self.eval_expr(a, b)?;
                    if v != Value::Nil {
                        return Ok(v);
                    }
                }
                Ok(Value::Nil)
            }
            "MP-TIME" => Ok(Value::Num(self.time_ms as f64 / 1000.0)),
            "PUSH" => {
                // Task bookkeeping (`record-response` pushes onto a task
                // variable); the state is not part of the trace.
                let mut out = Value::Nil;
                for (i, a) in args.iter().enumerate() {
                    let v = self.eval_expr(a, b)?;
                    if i == 0 {
                        out = v;
                    }
                }
                Ok(out)
            }
            "ACT-R-RANDOM" => {
                let lim = self.eval_expr(args.first().ok_or("act-r-random needs a limit")?, b)?;
                match lim {
                    Value::Int(n) if n > 0 => {
                        Ok(Value::Int(self.rng.borrow_mut().random_int(n as u64) as i64))
                    }
                    Value::Int(_) => Ok(Value::Int(0)),
                    Value::Num(x) => Ok(Value::Num(
                        self.rng.borrow_mut().random_float(x as f32) as f64
                    )),
                    other => Err(format!("act-r-random needs a number, got {}", other.show())),
                }
            }
            "CHUNK-SLOT-VALUE-FCT" => {
                if args.len() < 2 {
                    return Err("chunk-slot-value-fct needs a chunk and a slot".to_string());
                }
                let chunk = self.eval_expr(&args[0], b)?;
                let slot = self.eval_expr(&args[1], b)?;
                self.chunk_slot_value(&chunk, &slot)
            }
            "BUFFER-READ" => {
                // `buffer-read` (`buffers.lisp:1010`): the chunk in a buffer.
                let bv = self.eval_expr(args.first().ok_or("buffer-read needs a buffer")?, b)?;
                let name = match bv {
                    Value::Sym(s) | Value::Str(s) => s.to_uppercase(),
                    other => return Err(format!("buffer-read buffer {}", other.show())),
                };
                Ok(self
                    .buffer_read(&name)
                    .map(Value::Sym)
                    .unwrap_or(Value::Nil))
            }
            other => Err(format!("unsupported eval function {other}")),
        }
    }

    /// Remote-command call by name (a string head). No commands are
    /// registered yet, so these fail loudly rather than silently.
    pub(super) fn eval_command(
        &self,
        name: &str,
        _args: &[Sexp],
        _b: &Bindings,
    ) -> Result<Value, String> {
        // A registered task command (`add-act-r-command`): its side effects
        // (e.g. recording a response in the task's own variable) are not part
        // of the trace. A task driver may install the command's return value
        // (`nearest-vis-loc`'s `which-one`); otherwise it is a no-op.
        if let Some(value) = self.task_command_values.get(&name.to_uppercase()) {
            self.fired_task_commands
                .borrow_mut()
                .push(name.to_uppercase());
            return Ok(value.clone());
        }
        if self.task_commands.contains(&name.to_uppercase())
            || self.user_fns.contains_key(&name.to_uppercase())
        {
            self.fired_task_commands
                .borrow_mut()
                .push(name.to_uppercase());
            return Ok(Value::Nil);
        }
        Err(format!("unsupported eval command \"{name}\""))
    }

    pub(super) fn eval_num(&self, args: &[Sexp], b: &Bindings) -> Result<f64, String> {
        let e = args.first().ok_or("number argument expected")?;
        let v = self.eval_expr(e, b)?;
        as_num(&v).ok_or_else(|| format!("not a number: {}", v.show()))
    }

    pub(super) fn eval_nums(&self, args: &[Sexp], b: &Bindings) -> Result<Vec<f64>, String> {
        let mut out = Vec::new();
        for e in args {
            let v = self.eval_expr(e, b)?;
            out.push(as_num(&v).ok_or_else(|| format!("not a number: {}", v.show()))?);
        }
        Ok(out)
    }

    /// Preserve integer-ness when a numeric result is integral.
    pub(super) fn num_value(&self, x: f64) -> Value {
        if x.fract() == 0.0 && x.abs() < 9e15 {
            Value::Int(x as i64)
        } else {
            Value::Num(x)
        }
    }

    /// `randomize-time-ms` (`framework/random.lisp:541`): with
    /// `:randomize-time` enabled, `time - round(time/scale) + act-r-random(1
    /// + 2·round(time/scale))`.
    pub(super) fn randomize_ms(&mut self, time_ms: i64) -> i64 {
        match self.cfg.randomize_scale {
            None => time_ms,
            Some(_) if time_ms == 0 => time_ms,
            Some(scale) => {
                let step = lisp_round(time_ms as f32 / scale);
                time_ms - step + self.rng.borrow_mut().random_int((1 + 2 * step) as u64) as i64
            }
        }
    }

    /// `randomize-time` (`framework/random.lisp:519`): with `:randomize-time`
    /// enabled, `(time/scale)·(scale - 1 + act-r-random(2.0))`.
    pub(super) fn randomize_secs(&mut self, secs: f32) -> f32 {
        match self.cfg.randomize_scale {
            None => secs,
            Some(_) if secs == 0.0 => secs,
            Some(scale) => (secs / scale) * (scale - 1.0 + self.rng.borrow_mut().random_float(2.0)),
        }
    }

    pub(super) fn chunk_slot_value(&self, chunk: &Value, slot: &Value) -> Result<Value, String> {
        let slot = match slot {
            Value::Sym(s) => s.clone(),
            _ => return Err("chunk-slot-value-fct needs a slot symbol".to_string()),
        };
        let c = match chunk {
            // A buffer reference marker (`=goal`).
            Value::Sym(s) if s.starts_with('=') => {
                self.buffers.get(&s[1..]).and_then(|b| b.chunk.as_ref())
            }
            Value::Sym(name) => self
                .dm
                .iter()
                .find(|c| c.name.as_deref() == Some(name.as_str()))
                .or_else(|| self.named.get(name))
                // A buffer's chunk may not be registered by name (an
                // imaginal request's unnamed chunk); find it live.
                .or_else(|| {
                    self.buffers.values().find_map(|b| {
                        b.chunk
                            .as_ref()
                            .filter(|c| c.name.as_deref() == Some(name.as_str()))
                    })
                }),
            Value::Nil => return Ok(Value::Nil),
            _ => return Err("chunk-slot-value-fct needs a chunk".to_string()),
        };
        Ok(c.map(|c| c.get(&slot)).unwrap_or(Value::Nil))
    }

    /// The printed value of `slot` on the chunk named `chunk` (following a
    /// copy back to its source), or `None` when the chunk or slot is absent.
    /// Used by `MAP-RETRIEVAL-BUFFER` to instantiate p2's condition
    /// variables from the retrieved chunk.
    pub(super) fn chunk_name_slot(&self, chunk: &str, slot: &str) -> Option<String> {
        let name = self
            .copy_source
            .get(chunk)
            .map(|s| s.as_str())
            .unwrap_or(chunk);
        let c = self
            .dm
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .or_else(|| self.named.get(name))?;
        let v = c.get(slot);
        if v == Value::Nil {
            None
        } else {
            Some(v.show())
        }
    }
    /// An exact miss may still pass when the value similarity is strictly
    /// greater than `:md`; the mismatch is recorded for the utility penalty
    /// (`test-slot`, procedural.lisp:1774).
    pub(super) fn match_eq(
        &self,
        buffer: &str,
        slot: &str,
        pat: &Pattern,
        got: &Value,
        b: &mut Bindings,
        ppm: &mut Vec<PpmMismatch>,
    ) -> Result<bool, EngineError> {
        match pat {
            Pattern::Var(v) => match b.get(v) {
                Some(prev) => {
                    if self.value_eq(prev, got) {
                        Ok(true)
                    } else {
                        let prev = prev.clone();
                        Ok(self.ppm_fallback(buffer, slot, &prev, got, ppm))
                    }
                }
                None => {
                    // A fresh `slot =var` requires the slot to be filled.
                    if *got == Value::Nil {
                        Ok(false)
                    } else {
                        b.insert(v.clone(), got.clone());
                        Ok(true)
                    }
                }
            },
            other => {
                let want = Value::from_pattern(other)?;
                if self.value_eq(&want, got) {
                    Ok(true)
                } else {
                    Ok(self.ppm_fallback(buffer, slot, &want, got, ppm))
                }
            }
        }
    }

    /// Record a `:ppm` partial match when `similarity(want, got) > :md`.
    pub(super) fn ppm_fallback(
        &self,
        buffer: &str,
        slot: &str,
        want: &Value,
        got: &Value,
        ppm: &mut Vec<PpmMismatch>,
    ) -> bool {
        if let Some(_p) = self.cfg.ppm {
            let sim = self.similarity(want, got);
            if sim > self.cfg.md {
                ppm.push((
                    buffer.to_string(),
                    slot.to_string(),
                    want.clone(),
                    got.clone(),
                    sim,
                ));
                return true;
            }
        }
        false
    }
}

/// Numeric view of a value for the `!eval!`/`!bind!` evaluator.
pub(super) fn as_num(v: &Value) -> Option<f64> {
    match v {
        Value::Int(i) => Some(*i as f64),
        Value::Num(x) => Some(*x),
        _ => None,
    }
}

/// CL truth: `T` or `NIL`.
pub(super) fn bool_value(b: bool) -> Value {
    if b {
        Value::Sym("T".to_string())
    } else {
        Value::Nil
    }
}

/// `(A B C)` from a slice of values (`~S` of a list).
pub(super) fn show_values(vals: &[Value]) -> String {
    format!(
        "({})",
        vals.iter().map(|v| v.show()).collect::<Vec<_>>().join(" ")
    )
}

pub(super) fn is_number(v: &Value) -> bool {
    matches!(v, Value::Int(_) | Value::Num(_))
}

pub(super) fn f32_of(v: &Value) -> Option<f32> {
    match v {
        Value::Int(i) => Some(*i as f32),
        Value::Num(x) => Some(*x as f32),
        _ => None,
    }
}

pub(super) fn set_result(results: &mut [(String, Option<Value>)], slot: &str, v: Value) {
    if let Some(e) = results.iter_mut().find(|(s, _)| s == slot) {
        e.1 = Some(v);
    }
}

/// SBCL `~S`/`~A` for a single-float: shortest round-tripping decimal,
/// rendered fixed for decimal exponents in `-3..=6` and in `m.mmmmmmeN`
/// form otherwise (e.g. `3.7122225e-6`, `1.0e7`). The blending trace prints
/// its floats with this form (the refs are the arbiter).
/// A symbol/string value's name (act-touch slot helpers).
pub(super) fn name_of(v: &Value) -> String {
    match v {
        Value::Sym(s) | Value::Str(s) => s.clone(),
        _ => String::new(),
    }
}

/// A numeric value as `f64`, if it is one.
pub(super) fn num_of(v: &Value) -> Option<f64> {
    match v {
        Value::Int(i) => Some(*i as f64),
        Value::Num(x) => Some(*x),
        _ => None,
    }
}

/// A numeric value as Lisp `~a` would print it: an integer stays an integer
/// (`r 1` -> `1`) while a float gets the single-float spelling (`1.0`).
pub(super) fn lisp_num_str(v: &Value) -> String {
    match v {
        Value::Int(i) => i.to_string(),
        Value::Num(x) => lisp_s_f32(*x as f32),
        _ => String::new(),
    }
}

/// A numeric value, as an integer when integral (ACT-R preserves the
/// `generate-tracker-choices` integer choices).
pub(super) fn num_value(x: f64) -> Value {
    if x.fract() == 0.0 && x.abs() < 9e15 {
        Value::Int(x as i64)
    } else {
        Value::Num(x)
    }
}

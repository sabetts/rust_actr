//! RHS concretization and chunk construction.
use super::*;
impl ChunkInst {
    /// Missing slots read as NIL (matches `count nil`-style tests).
    pub(super) fn get(&self, slot: &str) -> Value {
        self.slots
            .iter()
            .find(|(s, _)| s == slot)
            .map(|(_, v)| v.clone())
            .unwrap_or(Value::Nil)
    }
}

impl Engine {
    pub(super) fn show_output_arg(
        &self,
        a: &crate::sexpr::Sexp,
        b: &Bindings,
    ) -> Result<String, EngineError> {
        use crate::sexpr::Sexp::*;
        match a {
            // `!output! (=num1)`: one-element lists hold the value.
            List(items) if items.len() == 1 => self.show_output_arg(&items[0], b),
            List(items) => {
                let mut s = String::new();
                for it in items {
                    s.push_str(&self.show_output_arg(it, b)?);
                    s.push(' ');
                }
                Ok(s.trim_end().to_string())
            }
            Sym(v) if v.starts_with('=') => self
                .subst(&Pattern::Var(v[1..].to_string()), b)
                .map(|val| val.show())
                .map_err(|_| bad(format!("unbound output variable {v}"))),
            Sym(v) => Ok(v.clone()),
            Int(i) => Ok(i.to_string()),
            Float(x) => Ok(Value::Num(*x).show()),
            Str(s) => Ok(format!("{s:?}")),
        }
    }
}

/// Build a chunk instance from a parsed `add-dm`/`define-chunks` entry.
pub(super) fn chunk_inst(c: &Chunk) -> Result<ChunkInst, EngineError> {
    let mut slots = Vec::new();
    for (s, p) in &c.slots {
        slots.push((s.clone(), Value::from_pattern(p)?));
    }
    Ok(ChunkInst {
        name: Some(c.name.clone()),
        ctype: c.ctype.clone().unwrap_or_default(),
        slots,
        bare: false,
    })
}

pub(super) fn new_chunk(desc: &ChunkDesc) -> Result<NewChunk, EngineError> {
    match desc {
        ChunkDesc::Named(n) => Ok(NewChunk::Named(n.clone())),
        ChunkDesc::Spec {
            ctype,
            slots,
            neg,
            rels,
        } => {
            let mut out = Vec::new();
            for (s, p) in slots {
                out.push((s.clone(), Value::from_pattern(p)?));
            }
            let mut negv = Vec::new();
            for (s, p) in neg {
                negv.push((s.clone(), Value::from_pattern(p)?));
            }
            let mut relv = Vec::new();
            for (s, p, op) in rels {
                relv.push((s.clone(), Value::from_pattern(p)?, *op));
            }
            Ok(NewChunk::Spec {
                ctype: ctype.clone(),
                slots: out,
                neg: negv,
                rels: relv,
                bare: false,
            })
        }
    }
}

impl Engine {
    /// Slot names may themselves be LHS variables (`- =attr nil`), resolved
    /// against the current bindings when a request is concretized.
    pub(super) fn resolve_slot_name(&self, s: &str, b: &Bindings) -> String {
        if let Some(var) = s.strip_prefix('=') {
            if let Some(Value::Sym(name)) = b.get(var) {
                return name.clone();
            }
        }
        s.to_string()
    }

    /// Copy a chunk to `{name}-{k}` (GOTCHA #41): the copy is a named,
    /// non-DM chunk whose copy fan-out is the source's fan-out at copy
    /// time (`copy-from-chunk-function chunk-fan-out`).
    pub(super) fn copy_chunk(&mut self, name: &str) -> String {
        let n = *self.copy_count.entry(name.to_string()).or_insert(0);
        self.copy_count.insert(name.to_string(), n + 1);
        let copy = format!("{name}-{n}");
        self.copy_source.insert(copy.clone(), name.to_string());
        if let Some(src) = self
            .dm
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .cloned()
            .or_else(|| self.named.get(name).cloned())
        {
            let mut inst = src;
            inst.name = Some(copy.clone());
            self.named.insert(copy.clone(), inst);
        }
        self.chunk_names.insert(copy.clone());
        let fo = self.fan_out.get(name).copied().unwrap_or(0);
        self.c_fan_out.insert(copy.clone(), fo);
        self.fan_out.entry(copy.clone()).or_insert(0);
        copy
    }

    /// RHS value substitution. When `copy` is set (goal/imaginal specs),
    /// an unbound `=buffer` reference resolves to a fresh `-k` copy of the
    /// buffer's chunk (GOTCHA #41), deduped within the firing via `copied`.
    pub(super) fn rhs_value(
        &mut self,
        p: &Pattern,
        b: &Bindings,
        copy: bool,
        copied: &mut HashMap<String, String>,
    ) -> Result<Value, EngineError> {
        if copy {
            if let Pattern::Var(v) = p {
                if !b.contains_key(v) {
                    let named = self
                        .buffers
                        .get(v)
                        .and_then(|buf| buf.chunk.as_ref())
                        .and_then(|c| c.name.clone());
                    if let Some(name) = named {
                        // The retrieval buffer already holds a copy (`G-k`
                        // from `set-buffer-chunk`), so a reference to it is
                        // used verbatim rather than copied again.
                        if v == "RETRIEVAL" {
                            return Ok(Value::Sym(name));
                        }
                        // A visual-location chunk is already a per-request
                        // copy (`VISUAL-LOCATION0-1`), so `=goal> … loc
                        // =visual-location` references it verbatim.
                        if v == "VISUAL-LOCATION" {
                            return Ok(Value::Sym(name));
                        }
                        let c = match copied.get(&name) {
                            Some(x) => x.clone(),
                            None => {
                                let x = self.copy_chunk(&name);
                                copied.insert(name.clone(), x.clone());
                                x
                            }
                        };
                        return Ok(Value::Sym(c));
                    }
                    // Unnamed spec-created buffer chunk: give it its reuse
                    // name (`BUFFER-CHUNK0`) so the reference is stable and
                    // can be aliased when it later merges into DM.
                    let has_chunk = self
                        .buffers
                        .get(v)
                        .map(|buf| buf.chunk.is_some())
                        .unwrap_or(false);
                    if has_chunk {
                        let reuse = format!("{v}-CHUNK0");
                        if let Some(buf) = self.buffers.get_mut(v) {
                            if let Some(c) = buf.chunk.as_mut() {
                                if c.name.is_none() {
                                    c.name = Some(reuse.clone());
                                }
                            }
                        }
                        return Ok(Value::Sym(reuse));
                    }
                }
            }
        }
        // A non-copy `=buffer` reference to an unnamed spec chunk: the
        // module names it (`CHUNK{n}`, `new-name-fct`) so it can be
        // referenced (`=imaginal` in act-touch).
        if !copy {
            if let Pattern::Var(v) = p {
                if !b.contains_key(v) {
                    let unnamed = self
                        .buffers
                        .get(v)
                        .and_then(|buf| buf.chunk.as_ref())
                        .map(|c| c.name.is_none())
                        .unwrap_or(false);
                    if unnamed {
                        let n = *self.name_count.entry("CHUNK".to_string()).or_insert(0);
                        self.name_count.insert("CHUNK".to_string(), n + 1);
                        let name = format!("CHUNK{n}");
                        if let Some(buf) = self.buffers.get_mut(v) {
                            if let Some(c) = buf.chunk.as_mut() {
                                c.name = Some(name.clone());
                            }
                        }
                        return Ok(Value::Sym(name));
                    }
                }
            }
        }
        self.subst(p, b)
    }

    pub(super) fn concretize(
        &mut self,
        spec: &ChunkDesc,
        b: &Bindings,
        copy: bool,
        copied: &mut HashMap<String, String>,
    ) -> Result<NewChunk, EngineError> {
        match spec {
            ChunkDesc::Named(n) => {
                // A named request item that is actually a variable
                // (`+retrieval> =goal`) resolves to the referenced chunk.
                if let Some(var) = n.strip_prefix('=') {
                    let v = self.subst(&Pattern::Var(var.to_string()), b)?;
                    match v {
                        Value::Sym(s) => Ok(NewChunk::Named(s)),
                        other => Err(bad(format!(
                            "request chunk reference {} is not a chunk name: {}",
                            n,
                            other.show()
                        ))),
                    }
                } else {
                    Ok(NewChunk::Named(n.clone()))
                }
            }
            ChunkDesc::Spec {
                ctype,
                slots,
                neg,
                rels,
            } => {
                let mut out = Vec::new();
                for (s, p) in slots {
                    out.push((
                        self.resolve_slot_name(s, b),
                        self.rhs_value(p, b, copy, copied)?,
                    ));
                }
                let mut negv = Vec::new();
                for (s, p) in neg {
                    negv.push((
                        self.resolve_slot_name(s, b),
                        self.rhs_value(p, b, copy, copied)?,
                    ));
                }
                let mut relv = Vec::new();
                for (s, p, op) in rels {
                    relv.push((
                        self.resolve_slot_name(s, b),
                        self.rhs_value(p, b, copy, copied)?,
                        *op,
                    ));
                }
                Ok(NewChunk::Spec {
                    ctype: ctype.clone(),
                    slots: out,
                    neg: negv,
                    rels: relv,
                    bare: true,
                })
            }
        }
    }
}

/// Strict harvesting + request rule
/// (`support/production-parsing.lisp:1323`): buffers tested on the LHS
/// that no RHS action targets, plus requested buffers without an
/// explicit clear, are cleared when the production fires.
pub(super) fn implicit_clears(
    prod: &Production,
    exempt: &[String],
    allow_dup: bool,
) -> Vec<String> {
    let mut tested: Vec<String> = Vec::new();
    for item in &prod.lhs {
        match item {
            LhsItem::Match { buffer, .. } => {
                if !tested.contains(buffer) {
                    tested.push(buffer.clone());
                }
            }
            LhsItem::Query { buffer, queries } => {
                if queries
                    .iter()
                    .any(|(a, v)| a == "BUFFER" && *v == Pattern::Sym("FAILURE".into()))
                    && !tested.contains(buffer)
                {
                    tested.push(buffer.clone());
                }
            }
            LhsItem::Eval { .. } | LhsItem::Bind { .. } | LhsItem::MvBind { .. } => {}
        }
    }
    let mut targeted: Vec<(char, String)> = Vec::new();
    for a in &prod.rhs {
        match a {
            RhsAction::Modify { buffer, .. } => targeted.push(('=', buffer.clone())),
            RhsAction::ModifyRequest { buffer, .. } => targeted.push(('=', buffer.clone())),
            RhsAction::Request { buffer, .. } => targeted.push(('+', buffer.clone())),
            RhsAction::Clear { buffer } => targeted.push(('-', buffer.clone())),
            // A direct `@buffer>` set/erase is targeted, so no implicit clear.
            RhsAction::DirectSet { buffer, .. } | RhsAction::DirectErase { buffer } => {
                targeted.push(('=', buffer.clone()))
            }
            RhsAction::Output { .. }
            | RhsAction::Other { .. }
            | RhsAction::Eval { .. }
            | RhsAction::Bind { .. }
            | RhsAction::MvBind { .. } => {}
        }
    }
    let mut clears: Vec<String> = Vec::new();
    for t in &tested {
        // Harvest rule skips do-not-harvest buffers; the request rule
        // below does not.
        if exempt.contains(t) {
            continue;
        }
        if !targeted.iter().any(|(_, b)| b == t) && !clears.contains(t) {
            clears.push(t.clone());
        }
    }
    for (op, t) in &targeted {
        if *op == '+'
            && !targeted.iter().any(|(o, b)| *o == '-' && b == t)
            && (allow_dup || !clears.contains(t))
        {
            clears.push(t.clone());
        }
    }
    clears
}

/// Buffers gaining an implicit `?buffer> state free` test: RHS `+`
/// (or `*`) requests whose target is not exempt and has no explicit
/// `?` test on the LHS (`production-parsing.lisp:1287`). Appended to
/// the LHS, matched, and emitted as query-buffer selection events.
pub(super) fn implicit_queries(prod: &Production, exempt: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for a in &prod.rhs {
        if let RhsAction::Request { buffer, .. } = a {
            let explicit = prod.lhs.iter().any(|item| match item {
                LhsItem::Query { buffer: b, .. } => b == buffer,
                _ => false,
            });
            if !exempt.contains(buffer) && !explicit && !out.contains(buffer) {
                out.push(buffer.clone());
            }
        }
    }
    out
}

/// Explicit modify/request output: `(procedural-rhst prod)` is `t` by
/// default, and `t` shows only at high detail. With `:rhst nil` the
/// event is hidden entirely.
pub(super) fn explicit_out(rhst: bool) -> Out {
    if rhst {
        Out::Always
    } else {
        Out::Never
    }
}

impl ChunkInst {
    pub(super) fn show(&self) -> String {
        match &self.name {
            Some(n) => n.clone(),
            None => {
                // `chunk-spec-to-chunk-def` (chunk-spec.lisp:1207): an empty
                // request spec collapses to the default `(ISA CHUNK)`.
                if self.bare && self.slots.is_empty() {
                    return "(ISA CHUNK)".to_string();
                }
                let mut parts: Vec<String> = Vec::new();
                if !self.bare {
                    parts.push("ISA".to_string());
                    parts.push(self.ctype.clone());
                }
                for (k, v) in &self.slots {
                    parts.push(k.clone());
                    parts.push(v.show());
                }
                format!("({})", parts.join(" "))
            }
        }
    }
}

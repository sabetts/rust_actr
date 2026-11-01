//! Trace/debug output, parameter setters, visicon and buffer dumps.
use super::compilation::bold_values;
use super::*;

fn apply_chunk_mods(chunk: &mut ChunkInst, mods: &[(String, Sexp)]) -> Result<(), EngineError> {
    for (name, value) in mods {
        let value = Value::from_sexp(value)?;
        let name = name.to_uppercase();
        match chunk.slots.iter_mut().find(|(k, _)| k == &name) {
            Some((_, old)) => *old = value,
            None => chunk.slots.push((name, value)),
        }
    }
    Ok(())
}

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
    /// Requests that lose their implicit `?buffer> state free` condition
    /// because `:do-not-query` exempts the buffer. ACT-R's style pass sees no
    /// query then, so the request is reported (`procedural.lisp:1425`). This
    /// must run after reset re-applies `:starting-parameters`.
    pub(super) fn implicit_query_style_warnings(&self) -> Vec<String> {
        if !crate::model::style_warnings_enabled(&self.model) {
            return Vec::new();
        }
        let mut out = Vec::new();
        for p in &self.model.productions {
            let queried: Vec<&str> = p
                .lhs
                .iter()
                .filter_map(|item| match item {
                    LhsItem::Query { buffer, .. } => Some(buffer.as_str()),
                    _ => None,
                })
                .collect();
            for action in &p.rhs {
                let RhsAction::Request { buffer, .. } = action else {
                    continue;
                };
                let safe = matches!(
                    buffer.as_str(),
                    "GOAL"
                        | "RETRIEVAL"
                        | "VISUAL"
                        | "VISUAL-LOCATION"
                        | "AURAL"
                        | "AURAL-LOCATION"
                        | "MANUAL"
                        | "MANUAL-LEFT"
                        | "MANUAL-RIGHT"
                        | "VOCAL"
                        | "SPEECH"
                        | "TEMPORAL"
                        | "MOTOR"
                );
                if !safe
                    && self.cfg.do_not_query.iter().any(|b| b == buffer)
                    && !queried.iter().any(|b| *b == buffer)
                {
                    out.push(format!(
                        "Production {} makes a request to buffer {} without a query in the conditions.",
                        p.name, buffer
                    ));
                }
            }
        }
        out
    }
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
            if let Some(src) = self
                .copy_source
                .get(&cname)
                .or_else(|| self.vl_copied_from.get(&cname))
            {
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

    /// `printed-buffer-chunk` (`framework/buffers.lisp:915`): the same
    /// header/chunk text as [`Self::print_buffer_chunk`], but consecutive
    /// buffers have no blank line between them; one blank line closes the
    /// returned string.
    pub fn print_printed_buffer_chunk(&mut self, buffers: &[String]) -> Vec<String> {
        let mut names = Vec::new();
        for (i, name) in buffers.iter().enumerate() {
            let bname = name.to_uppercase();
            let chunk = self.buffers.get(&bname).and_then(|b| b.chunk.clone());
            let cname = chunk
                .as_ref()
                .and_then(|c| c.name.clone())
                .unwrap_or_default();
            let shown = if cname.is_empty() {
                "NIL".to_string()
            } else {
                cname.clone()
            };
            let mut header = format!("{bname}: {shown} ");
            if let Some(src) = self
                .copy_source
                .get(&cname)
                .or_else(|| self.vl_copied_from.get(&cname))
            {
                header.push_str(&format!("[{src}]"));
            }
            if self.cfg.threads && bname == "GOAL" {
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
                if i + 1 < buffers.len() {
                    self.out_lines.pop();
                }
            }
        }
        if self.out_lines.last().is_some_and(|s| !s.is_empty()) {
            self.out_lines.push(String::new());
        }
        names
    }

    /// `(buffer-chunk)` with no buffer names: print only each buffer's
    /// `NAME: CHUNK ` header (the ref does not include the chunk slot lines).
    pub fn print_buffer_headers(&mut self, buffers: &[String]) -> Vec<String> {
        let mut names = Vec::new();
        for name in buffers {
            let bname = name.to_uppercase();
            let cname = self
                .buffers
                .get(&bname)
                .and_then(|b| b.chunk.as_ref())
                .and_then(|c| c.name.clone())
                .unwrap_or_default();
            let shown = if cname.is_empty() {
                "NIL".to_string()
            } else {
                cname.clone()
            };
            let mut header = format!("{bname}: {shown} ");
            if let Some(src) = self
                .copy_source
                .get(&cname)
                .or_else(|| self.vl_copied_from.get(&cname))
            {
                header.push_str(&format!("[{src}]"));
            }
            self.out_lines.push(header);
            names.push(cname);
        }
        names
    }

    /// `pprint-a-chunk` (`chunks.lisp:736`): the chunk's name, then
    /// `slot  value` lines, then a blank line (the `~%` plus
    /// `command-output`'s newline). Returns the chunk's name.
    pub(super) fn push_chunk(&mut self, c: &ChunkInst) -> String {
        let name = c.name.clone().unwrap_or_default();
        self.push_chunk_named(c, &name, false);
        name
    }

    /// `pprint-a-chunk` when the requested name is an alias for a DM chunk:
    /// print the requested name followed by the chunk's canonical name in
    /// parentheses (`CHUNK0 (G1)`).
    fn push_chunk_named(&mut self, c: &ChunkInst, display: &str, show_source: bool) {
        let name = c.name.clone().unwrap_or_default();
        if display.eq_ignore_ascii_case(&name) {
            if show_source {
                if let Some(source) = self.copy_source.get(&name.to_uppercase()) {
                    self.out_lines.push(format!("{display} ({source})"));
                } else {
                    self.out_lines.push(name);
                }
            } else {
                self.out_lines.push(name);
            }
        } else {
            self.out_lines.push(format!("{display} ({name})"));
        }
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
        let chunks: Vec<(ChunkInst, String)> = names
            .iter()
            .filter_map(|n| {
                let up = n.trim_start_matches("#:").to_uppercase();
                if let Some(c) = self
                    .dm
                    .iter()
                    .find(|c| c.name.as_deref() == Some(up.as_str()))
                    .or_else(|| self.named.get(&up))
                    .cloned()
                {
                    return Some((c, up));
                }
                if up == "VISUAL-CHUNK0" {
                    if let Some(c) = self
                        .buffers
                        .get("VISUAL")
                        .and_then(|b| b.chunk.as_ref())
                        .cloned()
                    {
                        let display = self
                            .vl_copied_from
                            .get(&up)
                            .map(|s| format!("{s}-0"))
                            .unwrap_or_else(|| up.clone());
                        let mut c = c;
                        c.name = Some(display.clone());
                        return Some((c, display));
                    }
                }
                if self.audio_copied_from.contains_key(&up) {
                    for buffer in ["AURAL-LOCATION", "AURAL"] {
                        if let Some(c) = self
                            .buffers
                            .get(buffer)
                            .and_then(|b| b.chunk.as_ref())
                            .filter(|c| c.name.as_deref() == Some(up.as_str()))
                            .cloned()
                        {
                            return Some((c, up.clone()));
                        }
                    }
                }
                let target = self.resolve_alias(&up);
                if target != up {
                    if let Some(c) = self.find_chunk(&target).cloned() {
                        return Some((c, up));
                    }
                }
                let feature_name = self
                    .vl_copied_from
                    .get(&up)
                    .cloned()
                    .unwrap_or_else(|| up.clone());
                self.features
                    .iter()
                    .find(|f| f.name == feature_name)
                    .map(|f| {
                        let mut slots = f.filled_slots();
                        slots.retain(|(k, _)| {
                            k != "TEXT"
                                && k != "OVAL"
                                && k != "LINE"
                                && k != "IMAGE"
                                && !k.starts_with("END1")
                                && !k.starts_with("END2")
                        });
                        if f.kind == "TEXT" || f.kind == "OVAL" || f.kind == "IMAGE" {
                            if let Some((_, v)) = slots.iter_mut().find(|(k, _)| k == "VALUE") {
                                *v = Value::Sym(f.kind.clone());
                            }
                        }
                        (
                            ChunkInst {
                                name: Some(up.clone()),
                                ctype: "VISUAL-LOCATION".to_string(),
                                slots,
                                bare: false,
                            },
                            up.clone(),
                        )
                    })
            })
            .collect();
        chunks
            .iter()
            .map(|(c, display)| {
                self.push_chunk_named(c, display, true);
                display.clone()
            })
            .collect()
    }

    /// `(sdm <spec>...)`: print the declarative-memory chunks matching the
    /// task's spec (`commands/dm-commands.lisp:491`). The corpus calls
    /// `(sdm isa pair)`, and `pair`'s only members are unnamed run chunks, so
    /// every DM chunk is printed (the merged buffer chunks, then the goals in
    /// `dm-chunks` order).
    pub fn print_sdm(&mut self, spec: &[String]) -> Vec<String> {
        let preds = parse_sdm_spec(spec);
        // A plain `(sdm isa pair)` spec is the corpus's "print everything"
        // form. Any other spec filters, with or without a `-`/`+` modifier.
        let filter = !spec.is_empty()
            && !(preds.len() == 1
                && preds[0].0.is_none()
                && preds[0].1 == "ISA"
                && preds[0].2 == "PAIR");
        let mut names = Vec::new();
        // `dm-chunks` (`declarative-memory.lisp:912`, `add-chunk-into-dm`
        // 1918) is an alist keyed by the chunk's slot signature. A chunk with
        // an existing signature joins that group; a new signature pushes a
        // group to the *front* (1931). `sdm` therefore prints groups
        // newest-first, each group in insertion order. This applies to the
        // model's `add-dm` chunks as well: `semantic`'s `property` group is
        // newer than the bare-word group and prints first.
        let key = |c: &ChunkInst| {
            let mut slots: Vec<String> = c.slots.iter().map(|(k, _)| k.clone()).collect();
            slots.sort();
            (c.ctype.clone(), slots)
        };
        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut signatures: Vec<(String, Vec<String>)> = Vec::new();
        for i in 0..self.dm.len() {
            let ki = key(&self.dm[i]);
            match signatures.iter().position(|k| *k == ki) {
                Some(g) => groups[g].push(i),
                None => {
                    signatures.insert(0, ki);
                    groups.insert(0, vec![i]);
                }
            }
        }
        for i in groups.into_iter().flatten() {
            let c = self.dm[i].clone();
            if filter && !sdm_match(&c, &preds) {
                continue;
            }
            names.push(self.push_chunk(&c));
        }
        names
    }

    /// `sdp` with chunk names (`commands/dm-commands.lisp:667`). Prints the
    /// full declarative parameter block for each named DM chunk.
    pub fn print_sdp_chunks(&mut self, names: &[String]) -> Vec<String> {
        let idxs: Vec<usize> = if names.is_empty() {
            self.dm_alist_order()
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
        for idx in idxs {
            self.push_sdp_chunk(idx);
        }
        names.to_vec()
    }

    /// `compute-activation` (`commands/dm-commands.lisp:738`) for a named DM
    /// chunk with an empty request spec: prints the activation narration for
    /// `:act` and returns nothing. Unlike retrieval it does not record
    /// `:last-retrieval-*`.
    pub fn print_compute_activation(&mut self, name: &str) {
        let up = name.to_uppercase();
        let Some(idx) = self
            .dm
            .iter()
            .position(|c| c.name.as_deref() == Some(up.as_str()))
        else {
            return;
        };
        let cname = self.dm[idx].name.clone().unwrap_or_default();
        if self.act_medium() {
            self.out(format!("Computing activation for chunk {cname}"));
            self.out("Computing base-level".to_string());
        }
        let base = self.base_level(idx, self.cfg.bll, true);
        let spread = self.spreading_activation(&cname, self.cfg.mas);
        let noise = if self.cfg.uan {
            0.0
        } else {
            match self.cfg.ans {
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
        let activation = base + spread + self.inhibition_offset(&self.hist[idx].refs) + noise;
        if self.act_low() {
            self.out(format!(
                "Chunk {cname} has an activation of: {}",
                lisp_f(activation)
            ));
        }
    }

    /// `(sdp <chunk> :references (...))`: replace the chunk's reference list
    /// with the given times (seconds, newest first) and invalidate the
    /// spacing-effect decay cache.
    pub fn set_chunk_references(&mut self, name: &str, refs: &[f64]) {
        let up = name.to_uppercase();
        let Some(idx) = self
            .dm
            .iter()
            .position(|c| c.name.as_deref() == Some(up.as_str()))
        else {
            return;
        };
        let refs_ms: Vec<i64> = refs.iter().map(|s| (s * 1000.0).round() as i64).collect();
        let h = &mut self.hist[idx];
        h.count = refs_ms.len();
        h.refs = match &self.cfg.ol {
            Ol::On => Vec::new(),
            Ol::Num(ol) => refs_ms.iter().take(*ol as usize).cloned().collect(),
            Ol::Off => refs_ms,
        };
        h.decays = None;
    }

    /// The spacing-effect decay list for a chunk, populating the cache through
    /// the same recompute path activation uses (`chunk-decays`).
    fn spacing_decays(&mut self, idx: usize) -> Vec<f32> {
        if self.cfg.eblse {
            self.spacing_activation(idx);
        }
        self.hist[idx]
            .decays
            .as_ref()
            .map(|d| d.iter().map(|(_, x)| *x).collect())
            .unwrap_or_default()
    }

    fn decays_line(&self, decays: &[f32]) -> String {
        format!(
            "({})",
            decays
                .iter()
                .map(|x| lisp_f(*x))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }

    /// `(model-output "~s" (mapcar 'second (chunk-decays 'g)))`: one decay
    /// list line.
    pub fn print_chunk_decays(&mut self, name: &str) {
        let up = name.to_uppercase();
        let Some(idx) = self
            .dm
            .iter()
            .position(|c| c.name.as_deref() == Some(up.as_str()))
        else {
            return;
        };
        let decays = self.spacing_decays(idx);
        let line = self.decays_line(&decays);
        self.model_output(&line);
    }

    /// `(model-output "chunk a: ~s~%…")`: one labelled decay line per chunk.
    pub fn print_named_decay_lines(&mut self, names: &[String]) {
        for name in names {
            let up = name.to_uppercase();
            let Some(idx) = self
                .dm
                .iter()
                .position(|c| c.name.as_deref() == Some(up.as_str()))
            else {
                continue;
            };
            let decays = self.spacing_decays(idx);
            let line = format!(
                "chunk {}: {}",
                name.to_lowercase(),
                self.decays_line(&decays)
            );
            self.model_output(&line);
        }
        // The task's single `(model-output "…~%…~%")` ends with a newline,
        // which `command-output` turns into one trailing blank line.
        self.model_output("");
    }

    /// `(clear-buffer <name>)` from a driver: clears without a trace line but
    /// still notifies the declarative module, so the chunk merges into DM and
    /// gains a reference (`clear-buffer-process`, `framework/buffers.lisp:1225`).
    pub fn clear_buffer_driver(&mut self, buffer: &str) {
        let bname = buffer.to_uppercase();
        let cleared = self.buffer_mut(&bname).chunk.take();
        self.buffer_mut(&bname).modified = false;
        self.buffer_mut(&bname).failed = false;
        if let Some(chunk) = cleared {
            self.merge_into_dm(chunk, &bname);
        }
        // No `changed()`: like the direct command, a task-level clear does not
        // re-arm conflict resolution.
    }

    /// `simulate-retrieval-request` (`commands/dm-commands.lisp:1484`):
    /// report the activation a request would produce without scheduling a
    /// retrieval. The RNG is restored afterwards.
    pub fn simulate_retrieval(
        &mut self,
        desc: &crate::model::ChunkDesc,
    ) -> Result<(), EngineError> {
        let spec = super::rhs::new_chunk(desc)?;
        let NewChunk::Spec {
            slots, neg, rels, ..
        } = &spec
        else {
            return Ok(());
        };
        if !self.cfg.esc {
            return Ok(());
        }
        let req_filled: Vec<&str> = slots
            .iter()
            .filter(|(s, v)| !s.starts_with(':') && *v != Value::Nil)
            .map(|(s, _)| s.as_str())
            .chain(
                neg.iter()
                    .filter(|(s, v)| !s.starts_with(':') && *v == Value::Nil)
                    .map(|(s, _)| s.as_str()),
            )
            .collect();
        let req_empty: Vec<&str> = slots
            .iter()
            .filter(|(s, v)| !s.starts_with(':') && *v == Value::Nil)
            .map(|(s, _)| s.as_str())
            .collect();
        let saved = self.rng.borrow().clone();
        let mut best: Option<(String, f32)> = None;
        let mut tied: Vec<(String, f32)> = Vec::new();
        for idx in 0..self.dm.len() {
            let c = self.dm[idx].clone();
            let cname = c.name.clone().unwrap_or_default();
            if !req_filled.iter().all(|s| c.get(s) != Value::Nil)
                || !req_empty.iter().all(|s| c.get(s) == Value::Nil)
            {
                continue;
            }
            let a = self.simulate_activation(idx, slots, neg, rels, self.cfg.mp);
            if best.as_ref().is_none_or(|(_, b)| a > *b) {
                if best.is_some() {
                    tied.clear();
                }
                self.out_lines.push(format!(
                    "Chunk {cname} has the current best activation {}",
                    lisp_f(a)
                ));
                best = Some((cname.clone(), a));
                tied.push((cname, a));
            } else if best.as_ref().is_some_and(|(_, b)| a == *b) {
                self.out_lines.push(format!(
                    "Chunk {cname} matches the current best activation {}",
                    lisp_f(a)
                ));
                tied.push((cname, a));
            } else {
                self.out_lines
                    .push(format!("Chunk {cname} has activation {}", lisp_f(a)));
            }
        }
        if let Some((name, activation)) = best {
            if tied.len() > 1 {
                self.out_lines.push(format!(
                    "Chunk {name} chosen among the chunks with activation {}",
                    lisp_f(activation)
                ));
            } else {
                self.out_lines.push(format!(
                    "Chunk {name} with activation {} is the best",
                    lisp_f(activation)
                ));
            }
        } else {
            self.out_lines
                .push("No matching chunk found retrieval failure".to_string());
        }
        *self.rng.borrow_mut() = saved;
        Ok(())
    }

    fn simulate_activation(
        &mut self,
        idx: usize,
        slots: &[(String, Value)],
        neg: &[(String, Value)],
        _rels: &[(String, Value, crate::model::SlotOp)],
        mp: Option<f32>,
    ) -> f32 {
        let refs = self.hist[idx].refs.clone();
        let base = self.base_level(idx, self.cfg.bll, false);
        let name = self.dm[idx].name.clone().unwrap_or_default();
        let old_act = self.cfg.act;
        self.cfg.act = None;
        let spread = if name.is_empty() {
            0.0
        } else {
            self.spreading_activation(&name, self.cfg.mas)
        };
        self.cfg.act = old_act;
        let mut a = base + spread + self.inhibition_offset(&refs);
        if let Some(mp) = mp {
            for (s, v) in neg {
                if s.starts_with(':') {
                    continue;
                }
                let sim = self.similarity(v, &self.dm[idx].get(s));
                if sim == self.cfg.ms {
                    a += mp * self.cfg.md;
                }
            }
            for (s, v) in slots {
                if s.starts_with(':') || *v == Value::Nil {
                    continue;
                }
                let sim = self.similarity(v, &self.dm[idx].get(s));
                a += mp * sim;
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
        a + noise
    }

    /// `print-chunk-activation-trace` (`dm-commands.lisp:1200`): replay the
    /// saved `:sact` detail for one chunk at a request time.
    pub fn print_chunk_activation_trace(&mut self, name: &str, time_ms: i64) {
        let text = self.chunk_activation_trace_text(name, time_ms);
        self.out_lines.extend(text.lines().map(str::to_string));
    }

    /// The string returned by `printed-chunk-activation-trace`
    /// (`dm-commands.lisp:1335`), which the training task filters by line.
    fn chunk_activation_trace_text(&self, name: &str, time_ms: i64) -> String {
        let name = name.to_uppercase();
        let Some(trace) = self.sact_traces.get(&time_ms) else {
            return format!(
                "#|Warning: No activation trace information available for time {} |#",
                fmt_secs(time_ms)
            );
        };
        if trace.no_matches.iter().any(|n| n == &name) {
            return format!("Chunk {name} did not match the request.\n");
        }
        if let Some((_, detail)) = trace.chunks.iter().find(|(n, _)| n == &name) {
            return detail.clone();
        }
        format!("Chunk {name} was not considered.\n")
    }

    /// The task-side loop in
    /// `tutorial-unit5modeling-v9-spreading-activation-only.lisp`: get the
    /// chunks returned by `sdm`, then print each chunk's saved activation
    /// trace, collapsing unmatched detail lines to a single `...`.
    pub fn print_sdm_activation_traces(&mut self, spec: &[String], time_ms: i64) {
        let mark = self.out_len();
        let names = self.print_sdm(spec);
        self.truncate_out(mark);
        for name in names {
            let text = self.chunk_activation_trace_text(&name, time_ms);
            let mut dots = false;
            for line in text.lines() {
                let lower = line.to_ascii_lowercase();
                let keep = line.contains("Computing activation")
                    || lower.contains("spreading")
                    || lower.contains("sources")
                    || lower.contains("transient")
                    || lower.contains("has an activation");
                if keep {
                    dots = false;
                    self.out_lines.push(line.to_string());
                } else if !dots {
                    dots = true;
                    self.out_lines.push("...".to_string());
                }
            }
            // The task splits the returned string on its trailing newline, so
            // the final empty segment becomes another `...`.
            if !dots {
                self.out_lines.push("...".to_string());
            }
        }
    }

    /// `print-activation-trace` (`dm-commands.lisp:1042`): replay the saved
    /// `:sact` match lists and activation details for a request time.
    pub fn print_activation_trace(&mut self, time_ms: i64) {
        let Some(trace) = self.sact_traces.get(&time_ms).cloned() else {
            self.out_lines.push(format!(
                "No activation trace information available for time {}",
                fmt_secs(time_ms)
            ));
            return;
        };
        // The Lisp `dm-chunks` set does not expose the engine's transient
        // visual/imaginal buffer copies. Keep the saved no-match list to the
        // model's declarative chunks, as the reference traces do.
        let model_chunks: HashSet<String> = self
            .model
            .chunks
            .iter()
            .chain(self.model.defined_chunks.iter())
            .map(|c| c.name.clone())
            .collect();
        for name in &trace.matches {
            self.out_lines.push(format!("Chunk {name} matches"));
        }
        for name in &trace.no_matches {
            if model_chunks.contains(name) {
                self.out_lines.push(format!("Chunk {name} does not match"));
            }
        }
        for (_, detail) in &trace.chunks {
            self.out_lines.extend(detail.lines().map(str::to_string));
        }
        if let Some(result) = trace.result {
            self.out_lines.push(result);
        }
    }

    /// `print-conflict-tree` (`commands/conflict-tree.lisp:222`): print the
    /// decision tree over the model's constant LHS tests. The upstream builder
    /// chooses splits by information gain; this tree uses the first condition
    /// that separates the current production classes, which is sufficient for
    /// the COUNT corpus and keeps the command available for larger models.
    pub fn print_conflict_tree(&mut self) {
        let mut classes: Vec<CtClass> = Vec::new();
        for prod in &self.model.productions {
            let mut conds: Vec<CtCond> = Vec::new();
            for item in &prod.lhs {
                match item {
                    LhsItem::Match { buffer, tests } => {
                        for t in tests {
                            if t.slot.eq_ignore_ascii_case("ISA") {
                                continue;
                            }
                            // `production-implicit`: every tested slot gets a
                            // safe-chunk-slot-equal condition. A `nil` literal
                            // is true (the slot must be empty); a variable is
                            // false (no constant constraint).
                            let cond = CtCond {
                                kind: CtKind::TestSlot,
                                buffer: buffer.to_uppercase(),
                                slot: t.slot.to_uppercase(),
                                test: "SAFE-CHUNK-SLOT-EQUAL".to_string(),
                                value: match t.value {
                                    crate::model::Pattern::Nil | crate::model::Pattern::Var(_) => {
                                        "NIL".to_string()
                                    }
                                    _ => pattern_show(&t.value),
                                },
                                result: !matches!(t.value, crate::model::Pattern::Var(_)),
                            };
                            if let Some(e) = conds.iter_mut().find(|c| c.key() == cond.key()) {
                                e.result |= cond.result;
                            } else {
                                conds.push(cond);
                            }
                        }
                    }
                    _ => {}
                }
            }
            conds.sort_by_key(CtCond::key);
            if let Some(c) = classes.iter_mut().find(|c| c.conds == conds) {
                c.names.push(prod.name.clone());
            } else {
                classes.push(CtClass {
                    names: vec![prod.name.clone()],
                    conds,
                });
            }
        }
        let all: Vec<String> = self
            .model
            .productions
            .iter()
            .map(|p| p.name.clone())
            .collect();
        let node = build_ct_node(&classes);
        self.out_lines
            .push(format!("---->[root ({})]", all.join(" ")));
        if let Some(n) = node {
            self.push_ct_node(&n, true, 2);
        }
    }

    fn push_ct_node(&mut self, node: &CtNode, branch: bool, depth: usize) {
        let indent = " ".repeat(depth);
        let branch = if branch { "T" } else { "NIL" };
        match node {
            CtNode::Leaf(names) => {
                // `production-ordering` uses the model's production order.
                let mut names = names.clone();
                names.sort_by_key(|n| {
                    self.model
                        .productions
                        .iter()
                        .position(|p| &p.name == n)
                        .unwrap_or(usize::MAX)
                });
                self.out_lines.push(format!(
                    "{indent}-{}- {branch} ->[leaf ({})]",
                    depth / 2,
                    names.join(" ")
                ));
            }
            CtNode::Test { cond, yes, no } => {
                self.out_lines.push(format!(
                    "{indent}-{}- {branch} ->[{}]",
                    depth / 2,
                    cond.display()
                ));
                self.push_ct_node(yes, true, depth + 2);
                self.push_ct_node(no, false, depth + 2);
            }
        }
    }

    /// `whynot-dm` (`commands/dm-commands.lisp:1383`): report the last
    /// retrieval request, then each named chunk's declarative parameters and
    /// why it was or was not retrieved.
    pub fn print_whynot_dm(&mut self, names: &[String]) {
        let Some(last) = self.last_request.clone() else {
            self.out_lines
                .push("No retrieval request has been made.".to_string());
            return;
        };
        self.out_lines.push(format!(
            "Retrieval request made at time {}:",
            fmt_secs(last.time_ms)
        ));
        self.out_lines.extend(last.request_lines);
        let wanted: Vec<String> = if names.is_empty() {
            self.dm_alist_order()
                .into_iter()
                .filter_map(|i| self.dm[i].name.clone())
                .collect()
        } else {
            names.iter().map(|n| n.to_uppercase()).collect()
        };
        for name in wanted {
            self.out_lines.push(String::new());
            let Some(idx) = self
                .dm
                .iter()
                .position(|c| c.name.as_deref() == Some(name.as_str()))
            else {
                self.out_lines.push(format!(
                    "Chunk {name} is not in the model's declarative memory."
                ));
                continue;
            };
            let c = self.dm[idx].clone();
            self.push_chunk(&c);
            self.push_sdp_chunk(idx);
            self.out_lines.push(String::new());
            if last.matches.iter().any(|m| m == &name) {
                self.out_lines.push(format!("{name} matched the request"));
                if self.cfg.esc {
                    let activation = last.activations.get(&name).copied().unwrap_or(0.0);
                    if activation >= last.rt {
                        if last.best.iter().any(|b| b == &name) {
                            self.out_lines
                                .push(format!("{name} was the chunk chosen to be retrieved"));
                        } else {
                            self.out_lines
                                .push(format!("{name} did not have the highest activation"));
                        }
                    } else {
                        self.out_lines.push(format!(
                            "{name} was below the retrieval threshold {}",
                            lisp_f(last.rt)
                        ));
                    }
                } else if last.best.iter().any(|b| b == &name) {
                    self.out_lines
                        .push(format!("{name} was the chunk chosen to be retrieved"));
                } else {
                    self.out_lines.push(format!(
                        "{name} was not chosen as the chunk to be retrieved"
                    ));
                }
            } else {
                self.out_lines
                    .push(format!("{name} did not match the request"));
            }
        }
    }

    /// The activation/base/spread triple for `sdp`'s `isa chunk` request.
    /// `compute-activation` draws one activation-noise sample and uses no
    /// partial matching because the request has no slot tests.
    fn sdp_activation(&mut self, idx: usize) -> (f32, f32, f32) {
        let refs = self.hist[idx].refs.clone();
        let base = self.base_level(idx, self.cfg.bll, false);
        let name = self.dm[idx].name.clone().unwrap_or_default();
        let old_act = self.cfg.act;
        self.cfg.act = None;
        let spread = if name.is_empty() {
            0.0
        } else {
            self.spreading_activation(&name, self.cfg.mas)
        };
        self.cfg.act = old_act;
        let noise = if self.cfg.uan {
            0.0
        } else {
            match self.cfg.ans {
                Some(s) => self.rng.borrow_mut().noise(s),
                None => 0.0,
            }
        };
        (
            base,
            spread,
            base + spread + self.inhibition_offset(&refs) + noise,
        )
    }

    fn sdp_sjis(&mut self, idx: usize) -> Vec<(String, f32)> {
        let c = self.dm[idx].clone();
        let name = c.name.clone().unwrap_or_default();
        let mas = self.cfg.mas.unwrap_or(0.0);
        let mut names = vec![name.clone()];
        for (_, v) in &c.slots {
            if let Some(j) = self.chunk_name_of(v) {
                if !names.contains(&j) {
                    names.push(j);
                }
            }
        }
        names
            .into_iter()
            .map(|j| {
                let sji = self.compute_sji(&j, &name, mas);
                (j, sji)
            })
            .collect()
    }

    fn sdp_similarities(&self, name: &str) -> Vec<(String, f32)> {
        let mut sims: Vec<(String, f32)> = self
            .sims
            .iter()
            .filter(|((a, _), _)| a == name)
            .map(|((_, b), v)| (b.clone(), *v))
            .collect();
        sims.sort_by(|a, b| a.0.cmp(&b.0));
        if !sims.iter().any(|(n, _)| n == name) {
            sims.push((name.to_string(), self.cfg.ms));
        }
        sims
    }

    fn push_sdp_chunk(&mut self, idx: usize) {
        let name = self.dm[idx].name.clone().unwrap_or_default();
        let (base, spread, activation) = self.sdp_activation(idx);
        self.out_lines
            .push(format!("Declarative parameters for chunk {name}:"));
        if !self.cfg.esc {
            return;
        }
        self.out_lines
            .push(format!(" :Activation {activation:>6.3}"));
        self.out_lines.push(" :Permanent-Noise  0.000".to_string());
        self.out_lines.push(format!(" :Base-Level {base:>6.3}"));
        if self.cfg.bll.is_some() {
            let h = self.hist[idx].clone();
            self.out_lines
                .push(format!(" :Creation-Time {}", fmt_secs(h.creation_ms)));
            if !matches!(self.cfg.ol, Ol::Off) {
                self.out_lines
                    .push(format!(" :Reference-Count  {}", h.count));
            }
            if !matches!(self.cfg.ol, Ol::On) {
                let refs: Vec<String> = h.refs.iter().map(|r| fmt_secs(*r)).collect();
                self.out_lines
                    .push(format!(" :Reference-List ({})", refs.join(" ")));
            }
        }
        if self.cfg.mas.is_some() {
            self.out_lines
                .push(format!(" :Source-Spread {spread:>6.3}"));
            let sjis = self.sdp_sjis(idx);
            let body = sjis
                .iter()
                .map(|(n, v)| format!("({n} {})", lisp_f(*v)))
                .collect::<Vec<_>>()
                .join(" ");
            self.out_lines.push(format!(" :Sjis ({body})"));
        }
        if self.cfg.mp.is_some() {
            let sims = self.sdp_similarities(&name);
            let body = sims
                .iter()
                .map(|(n, v)| format!("({n} {})", lisp_f(*v)))
                .collect::<Vec<_>>()
                .join(" ");
            self.out_lines.push(format!(" :Similarities ({body})"));
        }
        if let Some((time, activation)) = self.hist[idx].last_retrieval {
            self.out_lines
                .push(format!(" :Last-Retrieval-Activation {activation:>6.3}"));
            // `~6/print-time-in-seconds/` left-pads the printed time to six
            // characters (`misc-utils.lisp:1518`), so `0.050` keeps two
            // spaces while a four-digit second value has one.
            let t = fmt_secs(time);
            let pad = 6usize.saturating_sub(t.len());
            self.out_lines
                .push(format!(" :Last-Retrieval-Time {}{t}", " ".repeat(pad)));
        }
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

    /// A task's `(module-request 'visual ... move-attention ...)`: queue the
    /// attention shift now (`:priority 0`), before the armed
    /// conflict-resolution.
    pub fn schedule_move_attention(&mut self, loc: &str) {
        self.now(
            Prio::Num(0),
            Out::Medium,
            Event::MoveAttention {
                loc: loc.to_string(),
            },
        );
    }

    /// A task window change moved the attended item: schedule the
    /// `update-attended-loc` re-encode that finds no object.
    pub fn schedule_attended_missing(&mut self) {
        if let Some(loc) = self.last_attention.clone() {
            self.visual_busy = true;
            self.push(
                self.time_ms + self.cfg.attention_ms,
                Prio::Num(0),
                Out::Medium,
                Event::EncodingMissing { loc },
            );
            self.changed();
        }
    }

    /// Emit a raw task warning line (`print-warning`), e.g. the act-touch
    /// stub's "Installing a default touch device���?���" banner.
    pub fn emit_warning(&mut self, msg: &str) {
        if self.trace_on {
            self.out_lines.push(format!("#|Warning: {msg} |#"));
        }
    }

    /// `model-warning`: like `emit_warning`, but suppressed unless the model
    /// has `:model-warnings` on.
    pub(super) fn model_warning(&mut self, msg: &str) {
        if self.cfg.model_warnings {
            self.emit_warning(msg);
        }
    }

    /// Runtime `(p name ...)`: parse a production and emit ACT-R's typed-slot
    /// warnings. The test-model drivers use this to exercise runtime
    /// production definition without a full command-layer macro expander.
    pub fn define_production_text(&mut self, text: &str) -> Result<(), EngineError> {
        let p = crate::model::parse_production_text(text)
            .map_err(|e| EngineError::Model(e.to_string()))?;
        let (spec_warnings, prod_warnings) =
            crate::model::production_type_warnings(&p, &self.model);
        for w in spec_warnings.iter().chain(prod_warnings.iter()) {
            self.emit_warning(w);
        }
        if self.model.productions.iter().any(|q| q.name == p.name) {
            // `(p name ...)` for an existing production is a redefinition;
            // ACT-R keeps the last one and warns (`redefine-production`).
            self.model_warning(&format!(
                "Production {} already exists and it is being redefined.",
                p.name
            ));
        }
        self.model.productions.retain(|q| q.name != p.name);
        self.model.productions.push(p);
        self.rebuild_exec_prods();
        self.changed();
        Ok(())
    }

    /// `delete-production` (`procedural-cmds.lisp`): remove a named
    /// production, keeping the exec table and utility vectors aligned.
    pub fn delete_production(&mut self, name: &str) {
        let name = name.to_uppercase();
        self.model.productions.retain(|p| p.name != name);
        self.utilities = self
            .model
            .productions
            .iter()
            .map(|p| p.utility.unwrap_or(self.cfg.iu))
            .collect();
        self.production_utility = vec![None; self.model.productions.len()];
        self.rebuild_exec_prods();
        self.changed();
    }

    fn rebuild_exec_prods(&mut self) {
        self.exec_prods = self
            .model
            .productions
            .iter()
            .enumerate()
            .map(|(index, prod)| ExecProd {
                index,
                implicit_clears: implicit_clears(prod, &self.cfg.do_not_harvest, self.cfg.threads),
                implicit_queries: implicit_queries(prod, &self.cfg.do_not_query),
            })
            .collect();
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
        // reverses the construction spec; PORTING-NOTES #238).
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
        self.visicon_pending.push(VisPending::Add(Feature {
            name: format!("CHUNK{n}"),
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
        // `model-output` writes to the command stream, independently of the
        // model-trace toggle (`:v`): ACT-R captures it whenever the output
        // stream is live, so a driver that turns `:v` off and then does a
        // `(model-output "? …")` echo still prints it.
        self.out_lines.push(s.to_string());
    }

    /// Install task-file `defun`s for the bounded `!eval!`/`!bind!` evaluator.
    pub fn set_user_fns(&mut self, fns: HashMap<String, crate::model::UserDefun>) {
        self.user_fns = fns;
    }

    /// Register a task command (`add-act-r-command`). Its side effects are
    /// outside the model trace, so the bounded `!eval!` evaluator can treat it
    /// as a successful no-op.
    pub fn register_task_command(&mut self, name: &str) {
        self.task_commands.insert(name.to_uppercase());
    }

    /// Install the value a scripted task command returns to `!bind!`. The real
    /// task owns the value (e.g. `nearest-vis-loc`'s `*which*`); this is the
    /// task-driver bridge, not an engine special case.
    fn set_task_command_value(&mut self, name: &str, value: Value) {
        self.task_command_values.insert(name.to_uppercase(), value);
    }

    /// Sexp form used by the scripted [`crate::Step`] driver.
    pub fn set_task_command_value_sexp(
        &mut self,
        name: &str,
        value: &Sexp,
    ) -> Result<(), EngineError> {
        let value = Value::from_sexp(value)?;
        self.set_task_command_value(name, value);
        Ok(())
    }

    /// Remove a task command (`remove-act-r-command`).
    pub fn remove_task_command(&mut self, name: &str) {
        self.task_commands.remove(&name.to_uppercase());
        self.task_command_values.remove(&name.to_uppercase());
    }

    /// `set-visual-center-point` (`vision.lisp:2031`): the reference point
    /// used by `:nearest clockwise`/`counterclockwise`.
    pub fn set_visual_center_point(&mut self, x: f32, y: f32) {
        self.visual_center = (x, y);
    }

    /// Install task-file globals for the bounded evaluator.
    pub fn set_user_vars(&mut self, vars: HashMap<String, crate::sexpr::Sexp>) {
        self.user_vars = vars
            .into_iter()
            .filter_map(|(k, v)| Value::from_sexp(&v).ok().map(|v| (k.to_uppercase(), v)))
            .collect();
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
            ":OL" => {
                // Spacing effect requires optimized learning off; the module
                // refuses to turn it back on while enabled.
                self.cfg.ol = if self.cfg.eblse {
                    Ol::Off
                } else {
                    match val {
                        Sexp::Sym(s) if s == "NIL" => Ol::Off,
                        Sexp::Sym(s) if s == "T" => Ol::On,
                        Sexp::Int(i) => Ol::Num(*i as f64),
                        Sexp::Float(x) => Ol::Num(*x),
                        other => return Err(bad(format!("bad :ol {other:?}"))),
                    }
                };
            }
            // `update-chunks-on-the-fly` is the conjunction of `:ncnar` and
            // `:dcnn`; both may be changed at runtime
            // (`framework/naming-module.lisp:238`).
            ":NCNAR" => {
                self.cfg.ncnar = truthy;
                self.cfg.chunk_update = self.cfg.ncnar && self.cfg.dcnn;
            }
            ":DCNN" => {
                self.cfg.dcnn = truthy;
                self.cfg.chunk_update = self.cfg.ncnar && self.cfg.dcnn;
            }
            ":AUDIO-MS-TIMES" => self.cfg.audio_ms_times = truthy,
            ":SYLLABLE-RATE" => {
                if let Some(x) = num(val) {
                    self.cfg.syllable_rate_ms = (x * 1000.0).round() as i64;
                }
            }
            ":CHAR-PER-SYLLABLE" => {
                if let Some(x) = num(val).filter(|x| *x > 0.0) {
                    self.cfg.char_per_syllable = x;
                }
            }
            ":USE-SYLLABLE-TABLE" => self.cfg.use_syllable_table = truthy,
            ":SPLIT-VOCAL-STRINGS" => self.cfg.split_vocal_strings = truthy,
            ":ENABLE-INHIBITION" => self.cfg.inhibition = truthy,
            ":INHIBITION-SCALE" => {
                if let Some(x) = num(val) {
                    self.cfg.inhibition_scale = x;
                }
            }
            ":INHIBITION-DECAY" => {
                if let Some(x) = num(val) {
                    self.cfg.inhibition_decay = x;
                }
            }
            ":PPM" => self.cfg.ppm = num(val),
            ":PPM-HOOK" => {
                self.cfg.ppm_hook = match val {
                    Sexp::Sym(s) | Sexp::Str(s) if s != "NIL" => Some(s.clone()),
                    _ => None,
                }
            }
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
                // The spacing-effect module owns `:bll` while `:eblse` is on
                // (it is both the switch and the sentinel value).
                self.cfg.bll = if self.cfg.eblse {
                    Some(12.34)
                } else {
                    num(val).filter(|b| *b > 0.0)
                };
                self.cfg.act_scale = match self.cfg.bll {
                    Some(b) => (0.001f32).powf(-b).ln(),
                    None => 0.0,
                };
            }
            // `extras/spacing-effect/spacing-effect.lisp`: `:eblse` replaces
            // the decay equation with the per-reference spacing one. Enabling
            // forces `:ol nil` and the `:bll` sentinel 12.34.
            ":EBLSE" => {
                self.cfg.eblse = truthy;
                if truthy {
                    self.cfg.ol = Ol::Off;
                    self.cfg.bll = Some(12.34);
                    self.cfg.act_scale = (0.001f32).powf(-12.34).ln();
                }
            }
            ":SE-INTERCEPT" => {
                if let Some(x) = num(val) {
                    self.cfg.se_intercept = x;
                }
            }
            ":SE-SCALE" => {
                if let Some(x) = num(val) {
                    self.cfg.se_scale = x;
                }
            }
            ":UAN" => self.cfg.uan = truthy,
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
            ":DO-NOT-QUERY" => apply_buffer_list(&mut self.cfg.do_not_query, val)?,
            ":DO-NOT-HARVEST" => apply_buffer_list(&mut self.cfg.do_not_harvest, val)?,
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
            ":TRACE-DETAIL" => {
                self.cfg.detail = match val {
                    Sexp::Sym(s) => match s.as_str() {
                        "HIGH" => crate::trace::Detail::High,
                        "MEDIUM" => crate::trace::Detail::Medium,
                        "LOW" => crate::trace::Detail::Low,
                        _ => return Err(bad(format!("bad :trace-detail {s}"))),
                    },
                    Sexp::List(v) if v.len() == 2 && v[0].as_sym() == Some("QUOTE") => {
                        match v[1].as_sym() {
                            Some("HIGH") => crate::trace::Detail::High,
                            Some("MEDIUM") => crate::trace::Detail::Medium,
                            Some("LOW") => crate::trace::Detail::Low,
                            got => return Err(bad(format!("bad :trace-detail {got:?}"))),
                        }
                    }
                    got => return Err(bad(format!("bad :trace-detail {got:?}"))),
                };
            }
            ":SACT" => {
                self.cfg.sact = match val {
                    Sexp::Sym(s) if s == "NIL" => None,
                    Sexp::Sym(s) => Some(match s.as_str() {
                        "T" => Act::T,
                        "HIGH" => Act::High,
                        "MEDIUM" => Act::Medium,
                        "LOW" => Act::Low,
                        _ => return Err(bad(format!("bad :sact {s}"))),
                    }),
                    _ => return Err(bad(format!("bad :sact {val:?}"))),
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
            // Visual-location stuffing parameters (`vision.lisp:3499`).
            // `:unstuff-visual-location` is nil, t (the module's `new-span`),
            // or a delay in seconds; the test changes it between runs.
            ":UNSTUFF-VISUAL-LOCATION" => {
                self.cfg.unstuff_vl = match val {
                    Sexp::Sym(s) if s == "NIL" => UnstuffVl::Off,
                    Sexp::Sym(_) => UnstuffVl::NewSpan,
                    Sexp::Int(i) => UnstuffVl::Delay(i * 1000),
                    Sexp::Float(x) => UnstuffVl::Delay((x * 1000.0).round() as i64),
                    _ => UnstuffVl::NewSpan,
                }
            }
            ":OVERSTUFF-VISUAL-LOCATION" => self.cfg.overstuff_visual = truthy,
            ":VISUAL-ONSET-SPAN" => {
                if let Some(secs) = num(val) {
                    self.cfg.onset_span_ms = (secs * 1000.0).round() as i64;
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
            // Classify anything without an arm above. `Ignored` entries are
            // accepted no-ops (cosmetic flags and documented parity gaps);
            // `ModelOnly` has no runtime setter yet. A `Both`/`RuntimeOnly`
            // key reaching here means its arm was removed without updating
            // `params`; a missing key is a real error.
            other => match params::sgp_support(other) {
                Some(params::SgpSupport::Ignored(_)) | Some(params::SgpSupport::ModelOnly) => {}
                Some(params::SgpSupport::Both) | Some(params::SgpSupport::RuntimeOnly) => {
                    return Err(bad(format!("missing runtime arm for sgp {other}")));
                }
                None => return Err(bad(format!("unknown sgp parameter {other}"))),
            },
        }
        Ok(())
    }

    /// `(sgp :seed (S C))`: init + discard, same as construction.
    pub fn set_seed(&mut self, seed: (u64, u64)) {
        self.rng.borrow_mut().set_seed(seed.0, seed.1);
    }

    /// Start recording RNG calls (draw-sequence differential tests).
    pub fn start_rng_log(&mut self) {
        self.rng.borrow_mut().start_log();
    }

    /// Stop recording and return the RNG calls since `start_rng_log`.
    pub fn take_rng_log(&mut self) -> Vec<crate::rand::RngDraw> {
        self.rng.borrow_mut().take_log()
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
        // `command-output` writes to the command stream independently of the
        // model trace (`:v`), so this is not gated on `trace_on`.
        self.out_lines.push(format!(
            "Parameters for production {}:",
            name.to_uppercase()
        ));
        if self.cfg.esc {
            match utility {
                Some(v) => self.out_lines.push(format!(" :utility {v:6.3}")),
                None => self.out_lines.push(" :utility    NIL".to_string()),
            }
        }
        self.out_lines.push(format!(" :u  {u:6.3}"));
        self.out_lines
            .push(format!(" :at {}", fmt_6_3(self.cfg.dat_ms as f32 / 1000.0)));
        if self.cfg.esc && self.cfg.ul {
            // `production-parameter-fct` (`utility-and-reward-1.lisp:686`):
            // the reward lines print only with `:esc` + `:ul`. A number uses
            // `~6,3F`; any other value `~6@s` (right-aligned in 6 columns).
            match reward {
                Some(Reward::Num(r)) => self.out_lines.push(format!(" :reward {r:6.3}")),
                Some(Reward::Other(s)) => self.out_lines.push(format!(" :reward {s:>6}")),
                None => self.out_lines.push(" :reward    NIL".to_string()),
            }
            self.out_lines.push(" :fixed-utility    NIL".to_string());
        }
    }

    /// `(spp <name>…)` getter: resolve each production's parameters and print
    /// its `Parameters for production …` block, like ACT-R's
    /// `production-parameter-fct` (`utility-and-reward-1.lisp:678`); unknown
    /// names are skipped.
    pub fn print_prod_params_by_name(&mut self, name: &str) {
        let Some(idx) = self
            .model
            .productions
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(name))
        else {
            return;
        };
        let pname = self.model.productions[idx].name.clone();
        let u = self.utilities[idx];
        let utility = self.production_utility[idx];
        let reward = self.model.productions[idx].reward.clone();
        self.print_prod_params(&pname, u, utility, reward);
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

    /// `chunk-slot-value` for the RPC/facade path: prefer the live buffer
    /// chunk so a task callback sees a just-applied buffer modification.
    pub fn chunk_slot_live_value(&self, name: &str, slot: &str) -> Option<String> {
        self.chunk_slot_live(name, slot)
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
        let params: Vec<Value> = params.iter().map(value).collect();
        // The task schedules the built-in `end-ongoing-sound`; unlike a task
        // defun it has no `NONE` command trace line.
        if name.eq_ignore_ascii_case("end-ongoing-sound") {
            if let Some(Value::Int(id)) = params.first() {
                self.push(
                    at_ms,
                    Prio::Num(0),
                    Out::Medium,
                    Event::EndOngoingSound { id: *id as u64 },
                );
            }
            return;
        }
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

    /// `overwrite-buffer-chunk <buffer> <chunk-name>`: replace the reusable
    /// buffer chunk with a copy of the named source, recording the
    /// `[SOURCE]` alias printed by `buffer-chunk`.
    pub fn overwrite_buffer_chunk_named(
        &mut self,
        buffer: &str,
        source: &str,
    ) -> Result<(), EngineError> {
        let bname = buffer.to_uppercase();
        let mut inst = self.instantiate(&bname, NewChunk::Named(source.to_string()))?;
        let reuse = self
            .buffer_reuse
            .get(&bname)
            .cloned()
            .unwrap_or_else(|| format!("{bname}-CHUNK0"));
        inst.name = Some(reuse.clone());
        self.copy_source.insert(reuse, source.to_uppercase());
        let b = self.buffer_mut(&bname);
        b.chunk = Some(inst);
        b.failed = false;
        b.modified = false;
        self.changed();
        Ok(())
    }

    /// `schedule-mod-buffer-chunk <buffer> <mods> <delay>`: queue a task-side
    /// buffer modification. Unlike a module's `*buffer>` request, the default
    /// module is `:none`, so the trace column is `NONE` and the imaginal
    /// module's busy state is untouched.
    pub fn schedule_mod_buffer_chunk_rpc(
        &mut self,
        buffer: &str,
        mods: &[(String, Sexp)],
        delay_ms: i64,
        module: &str,
        priority: i64,
    ) {
        let bname = buffer.to_uppercase();
        let slots = mods
            .iter()
            .map(|(name, value)| {
                (
                    name.to_uppercase(),
                    Value::from_sexp(value).unwrap_or(Value::Nil),
                )
            })
            .collect();
        self.push(
            self.time_ms + delay_ms,
            Prio::Num(priority),
            Out::Low,
            Event::ModBuffer {
                buffer: bname,
                slots,
                module: module.to_uppercase(),
            },
        );
    }

    /// `trigger-reward` (`modules/utility-and-reward-1.lisp`): propagate a
    /// reward now, updating every production fired since the last reward.
    pub fn trigger_reward(&mut self, value: f32) {
        self.push(
            self.time_ms,
            Prio::Max,
            Out::Medium,
            Event::PropagateReward {
                value: crate::model::Reward::Num(value),
            },
        );
    }

    /// `extend-possible-slots <slot>`: make a slot name usable on any chunk.
    pub fn extend_possible_slots(&mut self, slot: &str) {
        self.valid_slots.insert(slot.to_uppercase());
    }

    /// `mod-buffer-chunk <buffer> (slot value ...)`: modify the chunk already
    /// in a buffer (the RPC form of the task command).
    pub fn mod_buffer_chunk_rpc(
        &mut self,
        buffer: &str,
        mods: &[(String, Sexp)],
    ) -> Result<(), EngineError> {
        let bname = buffer.to_uppercase();
        let Some(chunk) = self.buffers.get_mut(&bname).and_then(|b| b.chunk.as_mut()) else {
            return Err(bad(format!("mod-buffer-chunk with empty {bname}")));
        };
        for (name, value) in mods {
            let value = Value::from_sexp(value)?;
            let name = name.to_uppercase();
            match chunk.slots.iter_mut().find(|(k, _)| k == &name) {
                Some((_, old)) => *old = value,
                None => chunk.slots.push((name, value)),
            }
        }
        self.buffer_mut(&bname).modified = true;
        self.changed();
        Ok(())
    }

    /// `copy-chunk <chunk>`: a named copy `{name}-{k}` in the chunk table.
    pub fn copy_chunk_rpc(&mut self, name: &str) -> String {
        self.copy_chunk(&name.to_uppercase())
    }

    /// `copy-chunk` for the task/RPC path: copy the live buffer chunk when the
    /// name refers to one, so modifications made by the current production are
    /// included (the named-table entry can be the pre-modification source).
    pub fn copy_chunk_live_rpc(&mut self, name: &str) -> String {
        let target = name.to_uppercase();
        let live = self.buffers.values().find_map(|b| {
            b.chunk
                .as_ref()
                .filter(|c| c.name.as_deref() == Some(target.as_str()))
                .cloned()
        });
        if let Some(chunk) = live {
            let previous = self.named.insert(target.clone(), chunk);
            let copy = self.copy_chunk(&target);
            match previous {
                Some(chunk) => {
                    self.named.insert(target, chunk);
                }
                None => {
                    self.named.remove(&target);
                }
            }
            copy
        } else {
            self.copy_chunk(&target)
        }
    }

    /// `mod-chunk <chunk> (slot value ...)`: modify a named chunk in the
    /// chunk table or declarative memory (`commands/chunks.lisp`).
    pub fn mod_chunk_rpc(
        &mut self,
        name: &str,
        mods: &[(String, Sexp)],
    ) -> Result<(), EngineError> {
        let target = name.to_uppercase();
        let mut found = false;
        if let Some(chunk) = self.named.get_mut(&target) {
            apply_chunk_mods(chunk, mods)?;
            found = true;
        }
        for chunk in self
            .dm
            .iter_mut()
            .filter(|c| c.name.as_deref() == Some(target.as_str()))
        {
            apply_chunk_mods(chunk, mods)?;
            found = true;
        }
        if !found {
            return Err(bad(format!("mod-chunk: no chunk named {name}")));
        }
        Ok(())
    }

    /// `set-buffer-chunk <buffer> <spec>`: build a chunk from a description.
    pub fn set_buffer_chunk_desc(
        &mut self,
        buffer: &str,
        desc: &ChunkDesc,
    ) -> Result<(), EngineError> {
        self.set_buffer_chunk_desc_impl(buffer, desc, true)
    }

    /// The same direct set without re-arming conflict resolution. A task-level
    /// `(set-buffer-chunk …)` is not a production action, so ACT-R prints no
    /// follow-up `CONFLICT-RESOLUTION` for it (the spacing-effect ref's fake
    /// retrievals).
    pub fn set_buffer_chunk_quiet(
        &mut self,
        buffer: &str,
        desc: &ChunkDesc,
    ) -> Result<(), EngineError> {
        self.set_buffer_chunk_desc_impl(buffer, desc, false)
    }

    fn set_buffer_chunk_desc_impl(
        &mut self,
        buffer: &str,
        desc: &ChunkDesc,
        notify: bool,
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
        if notify {
            self.changed();
        }
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
        self.model
            .productions
            .iter()
            .map(|p| p.name.clone())
            .collect()
    }

    /// Print the `key value (default ...) : documentation` line ACT-R's
    /// `sgp-fct` reports for a parameter getter. Only the parameters whose
    /// descriptions are exercised by the corpus are listed.
    pub fn print_sgp_get(&mut self, key: &str) {
        let Some(value) = self.get_param(key) else {
            return;
        };
        match key.to_uppercase().as_str() {
            ":TRACE-DETAIL" => self.out_lines.push(format!(
                ":TRACE-DETAIL {value} (default MEDIUM) : Determines which events show in the trace"
            )),
            ":LF" => self
                .out_lines
                .push(format!(":LF {value} (default 1.0) : Latency Factor")),
            _ => {}
        }
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
            ":SIM-HOOK" => self.cfg.sim_hook.clone().unwrap_or_else(|| "NIL".into()),
            ":SYLLABLE-RATE" => format!("{}", self.cfg.syllable_rate_ms as f64 / 1000.0),
            ":CHAR-PER-SYLLABLE" => self.cfg.char_per_syllable.to_string(),
            ":USE-SYLLABLE-TABLE" => b(self.cfg.use_syllable_table),
            ":SPLIT-VOCAL-STRINGS" => b(self.cfg.split_vocal_strings),
            ":ANS" => self.cfg.ans.map(|x| x.to_string()).unwrap_or("NIL".into()),
            ":BLL" => self.cfg.bll.map(|x| x.to_string()).unwrap_or("NIL".into()),
            ":EBLSE" => b(self.cfg.eblse),
            ":SE-INTERCEPT" => self.cfg.se_intercept.to_string(),
            ":SE-SCALE" => self.cfg.se_scale.to_string(),
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
            } else if bname == "MANUAL" {
                // `motor-status` (`motor.lisp`): the manual module's
                // preparation/processor/execution state.
                let prep = self.man_prep_busy;
                let proc = self.man_proc[0] || self.man_proc[1];
                let exec = self.man_exec_busy || self.man_exec[0] || self.man_exec[1];
                row("preparation free", t(!prep));
                row("preparation busy", t(prep));
                row("processor free", t(!proc));
                row("processor busy", t(proc));
                row("execution free", t(!exec));
                row("execution busy", t(exec));
                row("last-command", "NONE");
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
            let text = super::compilation::render_production(
                prod,
                &Bindings::new(),
                &[],
                &implicit,
                &self.model.chunk_types,
            );
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
            let hit = conflict
                .iter()
                .find(|(i, _, _)| self.model.productions[*i].name == prod.name);
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
            let text = super::compilation::render_production(
                prod,
                &b,
                partials,
                &[],
                &self.model.chunk_types,
            );
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
    pub(super) fn why_not_match(&self, prod: &crate::model::Production) -> String {
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
                            if !t.slot.starts_with('=')
                                && !chunk.slots.iter().any(|(name, _)| name == &slot)
                                && matches!((&t.op, &t.value), (SlotOp::Eq, Pattern::Var(_)))
                            {
                                return format!(
                                    "The chunk in the {buffer} buffer does not have slot {slot}."
                                );
                            }
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
                                    if !chunk.slots.iter().any(|(name, _)| name == &slot) {
                                        return format!(
                                            "The chunk in the {buffer} buffer does not have a slot named {slot} (the value of the {} variable).",
                                            t.slot
                                        );
                                    }
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
    pub fn print_visicon_rpc(
        &mut self,
        show_pending: bool,
        limit: Option<(i64, u32)>,
    ) -> Vec<String> {
        let mut out = self.printed_visicon(show_pending, limit);
        // `command-output` ends its string with `~%`, so the ref has a blank
        // line after each `print-visicon` block.
        out.push(String::new());
        self.out_lines.extend(out.clone());
        out
    }

    /// `print-audicon` (`audio.lisp:1572`): the command-output form, which
    /// leaves a blank line after the table (`command-output` ends `~%`).
    pub fn print_audicon_rpc(&mut self) -> Vec<String> {
        let mut out = self.printed_audicon();
        out.push(String::new());
        self.out_lines.extend(out.clone());
        out
    }

    /// The `printed-audicon` body, without touching the output log.
    pub fn printed_audicon(&self) -> Vec<String> {
        let mut live: Vec<&Sound> = self
            .sounds
            .iter()
            .filter(|s| {
                s.onset <= self.time_ms
                    && s.offset
                        .map_or(true, |o| o + self.cfg.decay_time_ms >= self.time_ms)
            })
            .collect();
        live.sort_by_key(|s| s.onset);
        let mut out = vec![
            "Sound event     Att  Detectable  Kind           Content           Location      Onset     Offset    Sound ID  ONLY-SOUND  ONLY-EVT  BOTH".to_string(),
            "--------------  ---  ----------  -------------  ----------------  ------------  --------  --------  --------  ----------  --------  ----".to_string(),
        ];
        for s in live {
            let num = |ms: i64| {
                if self.cfg.audio_ms_times {
                    ms.to_string()
                } else {
                    lisp_s_f32((ms as f64 / 1000.0) as f32)
                }
            };
            let content = s.content.show();
            let mut row = format!(
                "{:<16}{:<5}{:<12}{:<15}{:<18}{:<14}{:>8}",
                s.ename,
                s.attended.table(),
                if self.time_ms >= s.onset + s.delay {
                    "T"
                } else {
                    "NIL"
                },
                s.kind,
                content,
                s.location,
                num(s.onset),
            );
            match s.offset {
                Some(offset) => {
                    row.push_str(&format!("  {:>8}  ", num(offset)));
                }
                None => row.push_str("   ongoing  "),
            }
            row.push_str(&format!("{:<10}", s.sname));
            let feature = |scope: AudioFeatureScope| {
                s.other_features
                    .iter()
                    .find(|f| f.scope == scope)
                    .map(|f| match &f.value {
                        Value::Str(x) => x.clone(),
                        v => v.show(),
                    })
                    .unwrap_or_default()
            };
            row.push_str(&format!(
                "{:<12}{:<10}{:<4}",
                feature(AudioFeatureScope::Sound),
                feature(AudioFeatureScope::Evt),
                feature(AudioFeatureScope::Both)
            ));
            out.push(row);
        }
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
        if self.feature_is_new(f) {
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

    /// `new-digit-sound <digit> [onset] [time-in-ms]`: schedule a digit sound.
    pub fn new_digit_sound_rpc(&mut self, digit: i64, onset_ms: i64) {
        let at = self.time_ms().max(onset_ms);
        self.new_digit_sound(digit, at);
    }

    /// `new-word-sound <word> [onset] [location] [time-in-ms]`: schedule a
    /// word sound whose duration is the word's articulation time.
    pub fn new_word_sound_rpc(&mut self, word: String, onset_ms: i64, location: String) {
        let at = self.time_ms().max(onset_ms);
        self.new_word_sound(word, at, location);
    }

    /// `(pbreak <name>...)`: halt before firing the named production.
    pub fn pbreak(&mut self, names: &[String]) {
        self.zz_break_quiet = false;
        for n in names {
            let n = n.to_uppercase();
            if !self.zz_breaks.contains(&n) {
                self.zz_breaks.push(n);
            }
        }
    }

    /// Quiet `pbreak` (the task wraps the run in `no-output`).
    pub fn pbreak_quiet(&mut self, names: &[String]) {
        self.pbreak(names);
        self.zz_break_quiet = true;
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
/// (PORTING-NOTES #238). Shared by `add-line-to-exp-window` and
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

    /// Drop the armed scripted device timeline. The BST learning driver
    /// builds each trial's window itself, while the extracted sidecar also
    /// contains the same buttons/lines; only one may drive the visicon.
    pub fn clear_device_script(&mut self) {
        self.pending_texts.clear();
        self.pending_rng.clear();
        self.pending_buttons.clear();
        self.pending_lines.clear();
        self.pending_line_modifies.clear();
        self.pending_line_removes.clear();
        self.pending_modifies.clear();
        self.pending_clears.clear();
        self.pending_removes.clear();
        self.pending_prints.clear();
        self.pending_late_prints.clear();
        self.queue
            .retain(|q| !matches!(q.event, Event::ProcDisplay));
        self.display_queued_ms = None;
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
        self.agi_items.clear();
        // `process-display` runs `check-finsts` before `:delete-all`
        // (`vision.lisp:1796` / `:1656`): an expired finst must not transfer
        // to the rebuilt display.
        self.check_visual_finsts();
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
        width: i64,
        height: i64,
        font_size: i64,
    ) -> String {
        self.add_window_text_styled(text, x, y, width, height, font_size, "black")
    }

    /// Styled `add-text-to-exp-window` (the AGI task passes a color).
    pub fn add_window_text_styled(
        &mut self,
        text: &str,
        x: i64,
        y: i64,
        _width: i64,
        _height: i64,
        font_size: i64,
        color: &str,
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
        self.push_text_item(&key, text, x + 1 + ox, y_screen, char_w, text_h, color);
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
        color: &str,
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
                        // `color`, `kind` and `text` (PORTING-NOTES #238)."
                        spec: vec![
                            ("SCREEN-Y".to_string(), Value::Int(screen_y)),
                            ("HEIGHT".to_string(), Value::Int(text_h)),
                            ("VALUE".to_string(), Value::Str(w.clone())),
                            ("WIDTH".to_string(), Value::Int(wd)),
                            ("SCREEN-X".to_string(), Value::Int(sx)),
                            ("COLOR".to_string(), Value::Sym(color.to_uppercase())),
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
        // request could ever match (PORTING-NOTES #229).
        self.features.push(Feature {
            name: name.clone(),
            kind: "OVAL".to_string(),
            value: text.to_string(),
            x: ox + x + round_half_even_f(width as f64 / 2.0) as i64,
            y: oy + y + round_half_even_f(height as f64 / 2.0) as i64,
            width,
            height,
            extra: vec![("COLOR".to_string(), Value::Sym(color.to_uppercase()))],
            // `build-vis-locs-for` for a `button-vdi` (`uwi.lisp:358`): the
            // oval's spec order is `screen-x screen-y width height value
            // color`, and the isa marker adds `kind` and `oval` (PORTING-NOTES #238)."
            spec: vec![
                (
                    "SCREEN-X".to_string(),
                    Value::Int(ox + x + round_half_even_f(width as f64 / 2.0) as i64),
                ),
                (
                    "SCREEN-Y".to_string(),
                    Value::Int(oy + y + round_half_even_f(height as f64 / 2.0) as i64),
                ),
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
        let oval_x = ox + x + round_half_even_f(width as f64 / 2.0) as i64;
        let oval_y = oy + y + round_half_even_f(height as f64 / 2.0) as i64;
        let tkey = format!("TEXT-VDI-{}", self.window_item_seq);
        self.window_item_seq += 1;
        self.push_text_item(
            &tkey,
            text,
            oval_x - round_half_even(total_w, 2),
            oval_y,
            char_w,
            text_h,
            "black",
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

    /// Queue the `:action` of the window button/image under the cursor
    /// (`uwi.lisp`'s button press and `image-feature.lisp`'s image click).
    /// ACT-R tests items topmost-first, and `window_items` is in add order, so
    /// scan in reverse.
    pub(super) fn record_click_action(&mut self) {
        let (mx, my, _) = self.zz_mouse_pos;
        let hit = self.window_items.iter().rev().find_map(|(key, names)| {
            if !self.window_actions.contains_key(key) {
                return None;
            }
            names.iter().find_map(|n| {
                self.features
                    .iter()
                    .find(|f| {
                        &f.name == n
                            && (f.kind == "OVAL" || f.kind == "IMAGE")
                            && (mx - f.x).abs() <= f.width / 2
                            && (my - f.y).abs() <= f.height / 2
                    })
                    .map(|f| (key.clone(), f.clone()))
            })
        });
        if let Some((key, feat)) = hit {
            if let Some((cmd, args)) = self.window_actions.get(&key).cloned() {
                let action = if self.scripted_click_actions && feat.kind == "IMAGE" {
                    let x = mx - (feat.x - feat.width / 2);
                    let y = my - (feat.y - feat.height / 2);
                    let line = match cmd.as_str() {
                        "IMAGE-WARNING" => format!(
                            "#|Warning: Image {:?} with no valid action clicked at position {},{} at time {}. |#",
                            feat.value,
                            x,
                            y,
                            lisp_s_f32(self.time_ms as f32 / 1000.0)
                        ),
                        "IMAGE-CLICK" => {
                            format!("Clicked image {:?} at {} {}", feat.value, x, y)
                        }
                        "IMAGE-CLICK2" => format!(
                            "Clicked the second brain image given value {}",
                            args.first().map(|a| a.as_str().unwrap_or("")).unwrap_or("")
                        ),
                        _ => String::new(),
                    };
                    ("MODEL-OUTPUT".to_string(), vec![Json::Str(line)])
                } else {
                    (cmd, args)
                };
                if self.scripted_click_actions {
                    self.pending_scripted_click = Some(action);
                    self.run_scripted_click();
                } else {
                    self.pending_device_actions.push(action);
                }
            }
        }
    }

    /// `add-image-to-exp-window` (the AGI image extension): an image feature
    /// with `value` = the image name.
    pub fn add_window_image(
        &mut self,
        text: &str,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        action: Option<(String, Vec<Json>)>,
    ) -> String {
        let key = format!("IMAGE-VDI-{}", self.window_item_seq);
        self.window_item_seq += 1;
        let (ox, oy) = self.window_origin;
        let name = format!("VISUAL-LOCATION{}", self.feat_seq);
        self.feat_seq += 1;
        let sx = ox + x + round_half_even_f(width as f64 / 2.0) as i64;
        let sy = oy + y + round_half_even_f(height as f64 / 2.0) as i64;
        // `image-vdi`'s `build-vis-locs-for` (`tools/image-feature.lisp:292`).
        // The printed-table order is the reverse of the construction order.
        let spec = vec![
            ("VALUE".to_string(), Value::Str(text.to_string())),
            ("WIDTH".to_string(), Value::Int(width)),
            ("HEIGHT".to_string(), Value::Int(height)),
            ("KIND".to_string(), Value::Sym("IMAGE".to_string())),
            ("IMAGE".to_string(), Value::Sym("T".to_string())),
            ("SCREEN-X".to_string(), Value::Int(sx)),
            ("SCREEN-Y".to_string(), Value::Int(sy)),
        ];
        self.features.push(Feature {
            name: name.clone(),
            kind: "IMAGE".to_string(),
            value: text.to_string(),
            x: sx,
            y: sy,
            width,
            height,
            extra: Vec::new(),
            spec,
            spec_pre: Vec::new(),
            tstamp_ms: self.time_ms,
            new_p: true,
            finst_ms: None,
        });
        self.window_items.push((key.clone(), vec![name]));
        if let Some(action) = action {
            self.window_actions.insert(key.clone(), action);
        }
        self.now_display();
        key
    }

    /// Fire a scripted click's held action at the mouse release, the point
    /// where the GUI invokes a button item's callback.
    pub(super) fn run_scripted_click(&mut self) {
        let Some((cmd, args)) = self.pending_scripted_click.take() else {
            return;
        };
        if cmd.eq_ignore_ascii_case("MODEL-OUTPUT") {
            if let Some(Json::Str(s)) = args.first() {
                self.model_output(s);
            }
        }
    }

    /// A human mouse click at global `(x, y)` in the experiment window
    /// (`visible-virtual-window-mouse-click`): move the virtual cursor there
    /// and queue any button action under it.
    pub fn click_window(&mut self, x: i64, y: i64) {
        self.zz_mouse_pos = (x, y, self.zz_mouse_pos.2);
        self.record_click_action();
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
            // A removed attended item can be re-added at the same place; keep
            // its finst so the rebuilt feature stays `T` (AGI's button B).
            for f in &self.features {
                if dropped.contains(&f.name) {
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
        }
        self.now_display();
    }

    /// Full `modify-text-for-exp-window` update used by the AGI task. The
    /// item keeps its identity but the visicon feature is rebuilt in place.
    pub fn modify_window_text_full(
        &mut self,
        key: &str,
        text: Option<&str>,
        x: Option<i64>,
        y: Option<i64>,
        color: Option<&str>,
        font_size: Option<i64>,
    ) {
        let Some((_, names)) = self.window_items.iter().find(|(k, _)| k == key) else {
            return;
        };
        let names = names.clone();
        let (ox, oy) = self.window_origin;
        let font = font_size.unwrap_or(12);
        let char_w = round_half_even_f(font as f64 * 7.0 / 12.0).max(1);
        let text_h = round_half_even_f(font as f64 * 10.0 / 12.0).max(1);
        let line_h = round_half_even_f(text_h as f64 / 0.8).max(text_h + 1);
        for f in self.features.iter_mut().filter(|f| names.contains(&f.name)) {
            if let Some(t) = text {
                f.value = t.to_string();
                f.set_slot("VALUE", Value::Str(t.to_string()));
                if t.is_ascii() {
                    let wd = char_w * t.chars().count() as i64;
                    f.width = wd;
                    f.set_slot("WIDTH", Value::Int(wd));
                    if let Some(nx) = x {
                        f.x = nx + 1 + ox + round_half_even(wd, 2);
                        f.set_slot("SCREEN-X", Value::Int(f.x));
                    }
                }
            }
            if let Some(nx) = x {
                if text.is_none() {
                    f.x = nx + 1 + ox;
                    f.set_slot("SCREEN-X", Value::Int(f.x));
                }
            }
            if let Some(ny) = y {
                f.y = ny + oy + round_half_even(line_h, 2);
                f.set_slot("SCREEN-Y", Value::Int(f.y));
            }
            if let Some(c) = color {
                f.set_slot("COLOR", Value::Sym(c.to_uppercase()));
            }
            f.height = text_h;
            f.set_slot("HEIGHT", Value::Int(text_h));
        }
        self.now_display();
    }

    /// Full `modify-button-for-exp-window` update: the button's oval and its
    /// label keep their feature names while their slots change.
    pub fn modify_window_button_full(
        &mut self,
        key: &str,
        text: Option<&str>,
        x: Option<i64>,
        y: Option<i64>,
        width: Option<i64>,
        height: Option<i64>,
        color: Option<&str>,
        action: Option<(String, Vec<Json>)>,
    ) {
        let Some((_, names)) = self.window_items.iter().find(|(k, _)| k == key) else {
            return;
        };
        let names = names.clone();
        let (ox, oy) = self.window_origin;
        let mut oval_x = 0;
        let mut oval_y = 0;
        for f in self.features.iter_mut().filter(|f| names.contains(&f.name)) {
            if f.kind == "OVAL" {
                if let Some(t) = text {
                    f.value = t.to_string();
                    f.set_slot("VALUE", Value::Str(t.to_string()));
                }
                if let Some(w) = width {
                    f.width = w;
                    f.set_slot("WIDTH", Value::Int(w));
                    if let Some(nx) = x {
                        f.x = ox + nx + round_half_even_f(w as f64 / 2.0) as i64;
                        f.set_slot("SCREEN-X", Value::Int(f.x));
                    }
                } else if let Some(nx) = x {
                    f.x = ox + nx + round_half_even_f(f.width as f64 / 2.0) as i64;
                    f.set_slot("SCREEN-X", Value::Int(f.x));
                }
                if let Some(h) = height {
                    f.height = h;
                    f.set_slot("HEIGHT", Value::Int(h));
                    if let Some(ny) = y {
                        f.y = oy + ny + round_half_even_f(h as f64 / 2.0) as i64;
                        f.set_slot("SCREEN-Y", Value::Int(f.y));
                    }
                } else if let Some(ny) = y {
                    f.y = oy + ny + round_half_even_f(f.height as f64 / 2.0) as i64;
                    f.set_slot("SCREEN-Y", Value::Int(f.y));
                }
                if let Some(c) = color {
                    f.set_slot("COLOR", Value::Sym(c.to_uppercase()));
                }
                oval_x = f.x;
                oval_y = f.y;
            }
        }
        if let Some(t) = text {
            let char_w = 7;
            let text_h = 10;
            let total_w = char_w * t.chars().count() as i64;
            for f in self.features.iter_mut().filter(|f| names.contains(&f.name)) {
                if f.kind == "TEXT" {
                    f.value = t.to_string();
                    f.x = oval_x;
                    f.y = oval_y;
                    f.width = total_w;
                    f.height = text_h;
                    f.set_slot("VALUE", Value::Str(t.to_string()));
                    f.set_slot("SCREEN-X", Value::Int(f.x));
                    f.set_slot("SCREEN-Y", Value::Int(f.y));
                    f.set_slot("WIDTH", Value::Int(total_w));
                    f.set_slot("HEIGHT", Value::Int(text_h));
                }
            }
        }
        if let Some(action) = action {
            self.window_actions.insert(key.to_string(), action);
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

    /// Enable/disable `retrieval-history` recording (`record-history`).
    pub fn set_retrieval_history(&mut self, on: bool) {
        self.record_retrieval_history = on;
    }

    /// `record-history` for the names a hook passes. `retrieval-history`
    /// enables `:sact` as Lisp's `enable-retrieval-history` does, so the
    /// recorded chunks carry their activation narration.
    pub fn record_history_names(&mut self, names: &[String]) {
        self.set_bold_buffers(names);
        for n in names {
            if n.eq_ignore_ascii_case("retrieval-history") {
                self.set_retrieval_history(true);
                self.cfg.sact = Some(Act::T);
            } else if n.eq_ignore_ascii_case("buffer-history") {
                self.set_buffer_history(true);
            } else if n.eq_ignore_ascii_case("visicon-history") {
                self.set_visicon_history(true);
            } else if n.eq_ignore_ascii_case("buffer-trace") {
                self.set_buffer_trace(true);
            } else if n.eq_ignore_ascii_case("production-history") {
                self.set_production_history(true);
            } else if self.is_buffer(n) {
                self.trace_buffer(n, true);
            }
        }
    }

    /// `stop-recording-history` counterpart for hook command effects.
    pub fn stop_recording_history_names(&mut self, names: &[String]) {
        for n in names {
            if n.eq_ignore_ascii_case("retrieval-history") {
                self.set_retrieval_history(false);
            } else if n.eq_ignore_ascii_case("buffer-history") {
                self.set_buffer_history(false);
            } else if n.eq_ignore_ascii_case("visicon-history") {
                self.set_visicon_history(false);
            } else if n.eq_ignore_ascii_case("buffer-trace") {
                self.set_buffer_trace(false);
            } else if n.eq_ignore_ascii_case("production-history") {
                self.set_production_history(false);
            } else if self.is_buffer(n) {
                self.trace_buffer(n, false);
            }
        }
    }

    /// The `sdp` chunk block (`commands/dm-commands.lisp:784`): the esc
    /// sub-symbolic parameter lines for one DM chunk, for the retrieval
    /// history's params field. `activation` and `base` are the values computed
    /// for the request.
    pub(super) fn sdp_chunk_block(&self, idx: usize, activation: f32, base: f32) -> String {
        if !self.cfg.esc {
            return String::new();
        }
        let mut s = String::new();
        s.push_str(&format!(" :Activation {:>6.3}\n", activation));
        s.push_str(&format!(" :Permanent-Noise {:>6.3}\n", 0.0));
        s.push_str(&format!(" :Base-Level {:>6.3}\n", base));
        if self.cfg.bll.is_some() {
            let h = &self.hist[idx];
            s.push_str(&format!(" :Creation-Time {}\n", fmt_secs(h.creation_ms)));
            if !matches!(self.cfg.ol, Ol::Off) {
                s.push_str(&format!(" :Reference-Count  {}\n", h.count));
            }
            if !matches!(self.cfg.ol, Ol::On) {
                let refs: Vec<String> = h.refs.iter().map(|r| fmt_secs(*r)).collect();
                s.push_str(&format!(" :Reference-List ({})\n", refs.join(" ")));
            }
        }
        s
    }

    /// The recorded `retrieval-history` records, in request order.
    pub fn retrieval_history(&self) -> &[RetrievalRecord] {
        &self.retrieval_history
    }

    /// Print the activation traces saved in a retrieval-history record:
    /// grouped's `(get-history-data "retrieval-history" nil <time>)` followed
    /// by `find`/`nth` on the recorded chunk set.
    pub fn print_retrieval_history_trace(
        &mut self,
        at_ms: i64,
        select: &crate::HistoryTraceSelect,
    ) {
        let Some(record) = self
            .retrieval_history
            .iter()
            .find(|r| r.time_ms == at_ms)
            .cloned()
        else {
            self.out_lines.push(format!(
                "No retrieval history data available for time {}",
                fmt_secs(at_ms)
            ));
            return;
        };
        let traces: Vec<String> = match select {
            crate::HistoryTraceSelect::Names(names) => names
                .iter()
                .filter_map(|name| {
                    record
                        .chunks
                        .iter()
                        .find(|c| c.name.eq_ignore_ascii_case(name))
                        .map(|c| c.activation_trace.clone())
                })
                .collect(),
            crate::HistoryTraceSelect::Index(index) => record
                .chunks
                .get(*index)
                .map(|c| c.activation_trace.clone())
                .into_iter()
                .collect(),
        };
        let mut printed = false;
        for trace in traces.into_iter().filter(|t| !t.is_empty()) {
            if printed {
                self.out_lines.push(String::new());
            }
            self.out_lines.extend(trace.lines().map(str::to_string));
            printed = true;
        }
        if printed {
            // `model-output` of the history string leaves its trailing newline.
            self.out_lines.push(String::new());
        }
    }

    /// The parameter groups the Environment's viewer shows, keyed by module
    /// name. ACT-R owns each parameter through its module table; the Rust
    /// engine keeps them in `Config`, so this groups the keys `get_param` can
    /// read. Only queryable keys are listed so the viewer never shows a
    /// placeholder.
    pub fn module_parameters(&self) -> Vec<(&'static str, Vec<&'static str>)> {
        vec![
            ("central", vec![":V", ":TRACE-DETAIL", ":SEED"]),
            (
                "declarative",
                vec![
                    ":BLC",
                    ":BLL",
                    ":ANS",
                    ":RT",
                    ":LF",
                    ":MD",
                    ":MS",
                    ":SJI",
                    ":EGS",
                    ":ALPHA",
                    ":SIM-HOOK",
                    ":ESC",
                ],
            ),
            (
                "procedural",
                vec![":EPL", ":PCT", ":IU", ":UL", ":ULT", ":CIA", ":RIR"],
            ),
        ]
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
            &self.model.chunk_types,
        ))
    }

    /// Capture `(spp <name>)` output without leaving it in the trace.
    fn capture_spp_text(&mut self, name: &str) -> String {
        let mark = self.out_lines.len();
        self.print_prod_params_by_name(name);
        let text = self.out_lines[mark..].join("\n");
        self.out_lines.truncate(mark);
        text
    }

    /// Capture `(sdp <name>)` output without leaving it in the trace.
    fn capture_sdp_text(&mut self, name: &str) -> String {
        let mark = self.out_lines.len();
        self.print_sdp_chunks(&[name.to_string()]);
        let text = self.out_lines[mark..].join("\n");
        self.out_lines.truncate(mark);
        text
    }

    /// Capture one production's `whynot` report without leaving it in the
    /// trace.
    fn capture_whynot_text(&mut self, name: &str) -> String {
        let mark = self.out_lines.len();
        self.print_whynot(&[name.to_string()]);
        let text = self.out_lines[mark..].join("\n");
        self.out_lines.truncate(mark);
        text
    }

    /// Build the stepper's production detail row:
    /// `(name spp-text binding-text printed printed-instantiation rows)`.
    fn stepper_production_item(
        &mut self,
        name: &str,
        bindings: &Bindings,
        mism: &[PpmMismatch],
    ) -> StepperItem {
        let params = self.capture_spp_text(name);
        let printed = self.printed_production(name).unwrap_or_default();
        let idx = self
            .model
            .productions
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(name));
        let instantiation = match idx {
            Some(idx) => {
                let prod = &self.model.productions[idx];
                let implicit = pp_implicit_queries(prod, &self.cfg.do_not_query);
                super::compilation::render_production(
                    prod,
                    bindings,
                    mism,
                    &implicit,
                    &self.model.chunk_types,
                )
            }
            None => printed.clone(),
        };
        let mut keys: Vec<String> = bindings.keys().cloned().collect();
        keys.sort();
        let binding_text: String = keys
            .iter()
            .map(|k| format!("{k}: {}\n", bindings[k].show()))
            .collect();
        let binding_details: Vec<(String, String, bool)> = keys
            .iter()
            .map(|k| {
                let encoded = match &bindings[k] {
                    Value::Str(v) => format!("'{v}'"),
                    v => v.show(),
                };
                let var = k.trim_start_matches('=');
                let is_buffer = idx.is_some_and(|idx| {
                    self.model.productions[idx]
                        .lhs
                        .iter()
                        .any(|item| match item {
                            LhsItem::Match { buffer, .. } | LhsItem::Query { buffer, .. } => {
                                buffer.eq_ignore_ascii_case(var)
                            }
                            _ => false,
                        })
                });
                (k.clone(), encoded, is_buffer)
            })
            .collect();
        StepperItem {
            name: name.to_string(),
            params,
            bindings: binding_text,
            printed,
            instantiation,
            binding_details,
        }
    }

    /// The detail data behind `update-stepper` for the event the stepper just
    /// stopped on.
    pub fn stepper_snapshot(&mut self, ev: &EventView) -> StepperSnapshot {
        let production_labels = [
            "Possible Productions".to_string(),
            "Production Parameters".to_string(),
            "Bindings".to_string(),
            "Production".to_string(),
        ];
        let chunk_labels = [
            "Possible Chunks".to_string(),
            "Chunk Parameters".to_string(),
            "Retrieval Request".to_string(),
            "Chunk".to_string(),
        ];
        match ev.action.as_str() {
            "production-selected" => {
                let matches: Vec<(String, Bindings, Vec<PpmMismatch>)> = self
                    .pmatches()
                    .into_iter()
                    .map(|(idx, bindings, mism)| {
                        (self.model.productions[idx].name.clone(), bindings, mism)
                    })
                    .collect();
                let matches = if matches.is_empty() {
                    ev.production
                        .clone()
                        .map(|name| {
                            let bindings = self
                                .last_event_bindings
                                .as_ref()
                                .filter(|(n, _)| n.eq_ignore_ascii_case(&name))
                                .map(|(_, b)| b.clone())
                                .unwrap_or_default();
                            vec![(name, bindings, Vec::new())]
                        })
                        .unwrap_or_default()
                } else {
                    matches
                };
                let mut details = Vec::new();
                let mut items = Vec::new();
                for (name, bindings, mism) in &matches {
                    items.push(name.clone());
                    details.push(self.stepper_production_item(name, bindings, mism));
                }
                StepperSnapshot {
                    items,
                    tutorable: Some("production-selected".into()),
                    labels: production_labels,
                    details,
                    tutoring: Json::Null,
                }
            }
            "production-fired" => {
                let name = ev
                    .production
                    .clone()
                    .or_else(|| self.last_event_bindings.as_ref().map(|(n, _)| n.clone()))
                    .unwrap_or_default();
                let bindings = self
                    .last_event_bindings
                    .as_ref()
                    .filter(|(n, _)| n.eq_ignore_ascii_case(&name))
                    .map(|(_, b)| b.clone())
                    .unwrap_or_default();
                let item = self.stepper_production_item(&name, &bindings, &[]);
                StepperSnapshot {
                    items: if name.is_empty() {
                        Vec::new()
                    } else {
                        vec![name]
                    },
                    tutorable: None,
                    labels: production_labels,
                    details: vec![item],
                    tutoring: Json::Null,
                }
            }
            "retrieved-chunk" | "retrieval-failure" => {
                let (request, names) = match &self.last_request {
                    Some(last) => (last.request_lines.join("\n"), last.matches.clone()),
                    None => (String::new(), Vec::new()),
                };
                let mut details = Vec::new();
                for name in &names {
                    details.push(StepperItem {
                        name: name.clone(),
                        params: self.capture_sdp_text(name),
                        bindings: request.clone(),
                        printed: self.printed_chunk(name).unwrap_or_default(),
                        instantiation: String::new(),
                        binding_details: Vec::new(),
                    });
                }
                StepperSnapshot {
                    items: names,
                    tutorable: None,
                    labels: chunk_labels,
                    details,
                    tutoring: Json::Null,
                }
            }
            "conflict-resolution" => {
                let names: Vec<String> = self
                    .pmatches()
                    .into_iter()
                    .map(|(idx, _, _)| self.model.productions[idx].name.clone())
                    .collect();
                let mut rows = Vec::new();
                for name in names {
                    let printed = self.printed_production(&name).unwrap_or_default();
                    let why = self.capture_whynot_text(&name);
                    rows.push(Json::Arr(vec![Json::Str(printed), Json::Str(why)]));
                }
                StepperSnapshot {
                    items: Vec::new(),
                    tutorable: Some("conflict-resolution".into()),
                    labels: std::array::from_fn(|_| String::new()),
                    details: Vec::new(),
                    tutoring: Json::Arr(rows),
                }
            }
            "start-retrieval" => {
                let (request, matching) = match &self.last_request {
                    Some(last) => (last.request_lines.join("\n"), last.matches.clone()),
                    None => (String::new(), Vec::new()),
                };
                let mut rows = Vec::new();
                for name in self.dm_chunk_names() {
                    let printed = self.printed_chunk(&name).unwrap_or_default();
                    let text = printed.lines().skip(1).collect::<Vec<_>>().join("\n");
                    let matched = matching.iter().any(|n| n == &name);
                    rows.push(Json::Arr(vec![
                        Json::Str(text),
                        if matched {
                            Json::Bool(true)
                        } else {
                            Json::Null
                        },
                    ]));
                }
                StepperSnapshot {
                    items: Vec::new(),
                    tutorable: Some("start-retrieval".into()),
                    labels: std::array::from_fn(|_| String::new()),
                    details: Vec::new(),
                    tutoring: Json::Arr(vec![Json::Str(request), Json::Arr(rows)]),
                }
            }
            _ => StepperSnapshot {
                labels: std::array::from_fn(|_| String::new()),
                tutoring: Json::Null,
                ..StepperSnapshot::default()
            },
        }
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

    /// `printed-buffer-chunk` for one or more buffers, captured without
    /// appending to the trace. Used by [`super::HookEffect`].
    pub fn capture_printed_buffer_chunk(&mut self, buffers: &[String]) -> String {
        let mark = self.out_lines.len();
        self.print_printed_buffer_chunk(buffers);
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

    pub fn print_dm_finsts(&mut self) {
        self.finsts
            .retain(|(_, t)| self.time_ms - *t <= self.cfg.finst_span_ms);
        let width = self
            .finsts
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(11)
            .max(11);
        self.out_lines.push(String::new());
        self.out_lines
            .push(format!("{:<width$}    Time Stamp", "Chunk name"));
        self.out_lines.push("-".repeat(width + 14));
        for (name, t) in &self.finsts {
            self.out_lines
                .push(format!("{:<width$}    {:>8.3}", name, *t as f64 / 1000.0));
        }
    }

    pub fn print_queue(&mut self) {
        let mut entries = Vec::new();
        while let Some(q) = self.queue.pop() {
            entries.push(q);
        }
        let mut lines = vec!["Events in the queue:".to_string()];
        let next_cr = self
            .cr_armed
            .then(|| entries.iter().map(|q| q.time_ms).min())
            .flatten();
        let cr_after = next_cr.and_then(|time| {
            entries
                .iter()
                .rposition(|q| q.time_ms == time)
                .map(|i| i + 1)
        });
        for (i, q) in entries.iter().enumerate() {
            if Some(i) == cr_after {
                lines.push(format_line(
                    next_cr.unwrap(),
                    "PROCEDURAL",
                    "CONFLICT-RESOLUTION",
                    self.cfg.module_width,
                ));
            }
            let (module, details) = self.queue_event(&q.event);
            lines.push(format_line(
                q.time_ms,
                &module,
                &details,
                self.cfg.module_width,
            ));
        }
        if cr_after == Some(entries.len()) {
            if let Some(time) = next_cr {
                lines.push(format_line(
                    time,
                    "PROCEDURAL",
                    "CONFLICT-RESOLUTION",
                    self.cfg.module_width,
                ));
            }
        }
        self.out_lines.extend(lines);
        for q in entries {
            self.queue.push(q);
        }
    }

    pub(super) fn queue_event(&self, event: &Event) -> (String, String) {
        match event {
            Event::SetBuffer { buffer, .. } => (
                Self::module_of_buffer(buffer).to_string(),
                format!("SET-BUFFER-CHUNK {buffer}"),
            ),
            Event::ClearBuffer { buffer, module } => {
                (module.clone(), format!("CLEAR-BUFFER {buffer}"))
            }
            Event::ModBuffer { buffer, module, .. } => {
                (module.clone(), format!("MOD-BUFFER-CHUNK {buffer}"))
            }
            Event::ModuleModRequest { buffer, .. } => (
                "PROCEDURAL".to_string(),
                format!("MODULE-MOD-REQUEST {buffer}"),
            ),
            Event::ModuleRequest { buffer, trace, .. } => (
                "PROCEDURAL".to_string(),
                if *trace {
                    format!("MODULE-REQUEST {buffer}")
                } else {
                    String::new()
                },
            ),
            Event::StartRetrieval { .. } => {
                ("DECLARATIVE".to_string(), "start-retrieval".to_string())
            }
            Event::RetrievedChunk { name, .. } => {
                ("DECLARATIVE".to_string(), format!("RETRIEVED-CHUNK {name}"))
            }
            Event::RetrievalFailure { .. } => {
                ("DECLARATIVE".to_string(), "RETRIEVAL-FAILURE".to_string())
            }
            Event::StartBlending { .. } => ("BLENDING".to_string(), "start-blending".to_string()),
            Event::BlendingComplete { .. } => {
                ("BLENDING".to_string(), "BLENDING-COMPLETE".to_string())
            }
            Event::BlendingFailure => ("BLENDING".to_string(), "BLENDING-FAILURE".to_string()),
            Event::ConflictResolution => {
                ("PROCEDURAL".to_string(), "CONFLICT-RESOLUTION".to_string())
            }
            Event::ProductionSelected { prod, .. } => (
                "PROCEDURAL".to_string(),
                format!("PRODUCTION-SELECTED {}", self.model.productions[*prod].name),
            ),
            Event::ProductionFired { prod, .. } => (
                "PROCEDURAL".to_string(),
                format!("PRODUCTION-FIRED {}", self.model.productions[*prod].name),
            ),
            Event::Dummy => ("NONE".to_string(), String::new()),
            Event::Emit { module, line } => (module.clone(), line.clone()),
            Event::Command { name, .. } => ("NONE".to_string(), name.clone()),
            _ => {
                let view = self.event_view(event, Out::Low, true);
                (view.module, view.action.to_uppercase())
            }
        }
    }
}

/// `buffer-history` recording (`other-files/buffer-history.lisp`).
impl Engine {
    /// Enable/disable `buffer-history` recording.
    pub fn set_buffer_history(&mut self, on: bool) {
        self.buffer_history.enabled = on;
    }

    /// The recorded `buffer-history` segments `(time_ms, rows)`, where each row
    /// is `[buffer, action, what, status, prev-chunk, prev-status, next-chunk,
    /// next-status]`.
    pub fn buffer_history(&self) -> Vec<(i64, Vec<Vec<String>>)> {
        let bh = &self.buffer_history;
        if bh.current.is_empty() {
            return bh.recorded.clone();
        }
        let mut results: Vec<Vec<String>> = Vec::new();
        for x in &bh.current {
            let mut row = x.clone();
            push_snapshot(&mut row, &bh.start, &x[0]);
            row.push(String::new());
            row.push(String::new());
            results.push(row);
        }
        let mut all = bh.recorded.clone();
        all.push((bh.last_time, results));
        all
    }

    /// A snapshot of every buffer: `(name, printed chunk or "buffer empty",
    /// printed status)`.
    fn bh_snapshot(&mut self) -> Vec<(String, String, String)> {
        let names = self.buffer_names();
        names
            .into_iter()
            .map(|b| {
                let chunk = match self.buffer_read(&b) {
                    Some(n) if !n.is_empty() => self.printed_chunk(&n).unwrap_or(n),
                    _ => "buffer empty".to_string(),
                };
                let status = self.printed_buffer_status(&b);
                (b, chunk, status)
            })
            .collect()
    }

    /// The pre-event hook (`buffer-start-time-recorder`): close out the
    /// previous time segment when the model clock advances.
    pub(super) fn bh_pre_hook(&mut self) {
        if !self.buffer_history.enabled {
            return;
        }
        let t = self.time_ms;
        if t == self.buffer_history.last_time {
            return;
        }
        let snap = self.bh_snapshot();
        if self.buffer_history.last_time != -1 {
            let prev = self.buffer_history.start.clone();
            let mut results: Vec<Vec<String>> = Vec::new();
            for x in self.buffer_history.current.clone() {
                let mut row = x.clone();
                let buf = x[0].clone();
                push_snapshot(&mut row, &prev, &buf);
                push_snapshot(&mut row, &snap, &buf);
                results.push(row);
            }
            let time = self.buffer_history.last_time;
            self.buffer_history.recorded.push((time, results));
        }
        self.buffer_history.current.clear();
        self.buffer_history.start = snap;
        self.buffer_history.last_time = t;
    }

    /// The `(buffer, action, what)` of a buffer event when recording (the post
    /// hook's input, captured before `exec` consumes the event).
    pub(super) fn bh_event_info_take(&self, ev: &Event) -> Option<(String, String, String)> {
        if !self.buffer_history.enabled {
            return None;
        }
        self.bh_event_info(ev)
    }

    /// The post-event hook (`buffer-event-recorder`): record a buffer change
    /// (status read after the event has run).
    pub(super) fn bh_record(&mut self, info: Option<(String, String, String)>) {
        let Some((buffer, action, what)) = info else {
            return;
        };
        let status = self.printed_buffer_status(&buffer);
        self.buffer_history
            .current
            .push(vec![buffer, action, what, status]);
    }

    /// `(buffer, action, what)` for a buffer-changing event, or `None`.
    fn bh_event_info(&self, ev: &Event) -> Option<(String, String, String)> {
        match ev {
            Event::SetBuffer { buffer, chunk, .. } => {
                let what = match chunk {
                    NewChunk::Named(n) => self.printed_chunk(n).unwrap_or_else(|| n.clone()),
                    _ => super::productions::render_request_spec(chunk),
                };
                Some((buffer.clone(), "set-buffer-chunk".to_string(), what))
            }
            Event::ClearBuffer { buffer, .. } => {
                Some((buffer.clone(), "clear-buffer".to_string(), String::new()))
            }
            Event::ModBuffer {
                buffer,
                slots,
                module,
            } => {
                let action = if module == "PROCEDURAL" {
                    "mod-buffer-chunk"
                } else {
                    "module-mod-request"
                };
                Some((buffer.clone(), action.to_string(), render_slots_spec(slots)))
            }
            Event::ModuleModRequest { buffer, slots } => Some((
                buffer.clone(),
                "module-mod-request".to_string(),
                render_slots_spec(slots),
            )),
            Event::ModuleRequest { buffer, spec, .. } => Some((
                buffer.clone(),
                "module-request".to_string(),
                super::productions::render_request_spec(spec),
            )),
            _ => None,
        }
    }
}

/// `visicon-history` recording (`other-files/perceptual-history.lisp`).
impl Engine {
    /// Enable/disable `visicon-history` recording.
    pub fn set_visicon_history(&mut self, on: bool) {
        self.visicon_history.enabled = on;
    }

    /// The recorded `visicon-history` entries `(time_ms, snapshots)`.
    pub fn visicon_history(&self) -> &[(i64, Vec<String>)] {
        &self.visicon_history.data
    }

    /// `perceptual-history-update`: record the printed visicon when it differs
    /// from the last snapshot, appending to the entry at the same time.
    pub(super) fn record_visicon_snapshot(&mut self) {
        if !self.visicon_history.enabled {
            return;
        }
        let current = self.printed_visicon(false, None).join("\n");
        if current == self.visicon_history.last_item {
            return;
        }
        self.visicon_history.last_item = current.clone();
        let t = self.time_ms;
        if let Some((lt, items)) = self.visicon_history.data.last_mut() {
            if *lt == t {
                items.push(current);
                return;
            }
        }
        self.visicon_history.data.push((t, vec![current]));
    }
}

/// `buffer-trace` recording (`tools/buffer-trace.lisp`).
impl Engine {
    /// Enable/disable the `buffer-trace` history.
    pub fn set_buffer_trace(&mut self, on: bool) {
        self.buffer_trace.enabled = on;
    }

    /// Whether `name` is a model buffer (`record-history <buffer>`).
    pub fn is_buffer(&self, name: &str) -> bool {
        self.buffers.contains_key(&name.to_uppercase())
    }

    /// Add/remove a traced buffer (`record-history <buffer>` constituent).
    pub fn trace_buffer(&mut self, name: &str, on: bool) {
        let up = name.to_uppercase();
        if on {
            if !self.buffer_trace.traced.contains(&up) {
                self.buffer_trace.traced.push(up);
            }
        } else {
            self.buffer_trace.traced.retain(|x| x != &up);
        }
    }

    /// The recorded `buffer-trace` records `(time_ms, summaries)`.
    pub fn buffer_trace(&self) -> Vec<(i64, Vec<BufSummary>)> {
        let mut out = self.buffer_trace.saved.clone();
        if let Some(c) = &self.buffer_trace.current {
            out.push(c.clone());
        }
        out
    }

    /// The pre-event boundary for `buffer-trace`: start a new record when the
    /// clock advances (`buffer-trace-event-recorder`).
    pub(super) fn bt_pre_hook(&mut self) {
        if !self.buffer_trace.enabled {
            return;
        }
        let t = self.time_ms;
        let new = match &self.buffer_trace.current {
            None => true,
            Some((ct, _)) => *ct != t,
        };
        if new {
            if let Some(c) = self.buffer_trace.current.take() {
                self.buffer_trace.saved.push(c);
            }
            let rows = self
                .buffer_trace
                .traced
                .iter()
                .map(|n| BufSummary {
                    name: n.clone(),
                    ..Default::default()
                })
                .collect();
            self.buffer_trace.current = Some((t, rows));
        }
    }

    /// The post-event update for `buffer-trace`: apply the event, then refresh
    /// each traced buffer's busy/error/full flags.
    pub(super) fn bt_post(&mut self, info: Option<(String, BufKind)>) {
        if !self.buffer_trace.enabled {
            return;
        }
        if let Some((buffer, kind)) = info {
            let up = buffer.to_uppercase();
            if let Some((_, rows)) = self.buffer_trace.current.as_mut() {
                if let Some(s) = rows.iter_mut().find(|s| s.name == up) {
                    match kind {
                        BufKind::SetChunk(t) => s.chunk_name = Some(t),
                        BufKind::Mod => s.modified = true,
                        BufKind::Clear => s.cleared = true,
                        BufKind::Request(t) => s.request = Some(format!("+{up}> {t}")),
                        BufKind::ModRequest(t) => s.request = Some(format!("*{up}> {t}")),
                    }
                }
            }
        }
        // Read the buffer states before borrowing the record mutably.
        let states: Vec<(bool, bool, bool)> = self
            .buffer_trace
            .traced
            .iter()
            .map(|n| match self.buffers.get(n) {
                Some(b) => (b.busy, b.failed, b.chunk.is_some()),
                None => (false, false, false),
            })
            .collect();
        if let Some((_, rows)) = self.buffer_trace.current.as_mut() {
            for (s, (busy, failed, full)) in rows.iter_mut().zip(states) {
                if busy {
                    s.busy = true;
                } else if s.busy {
                    s.busy_free = true;
                }
                if failed {
                    s.error = true;
                } else if s.error {
                    s.error_clear = true;
                }
                if full {
                    s.full = true;
                }
            }
        }
    }

    /// The `(buffer, kind)` of a buffer event for `buffer-trace`, captured
    /// before `exec` consumes the event.
    pub(super) fn bt_event_info_take(&self, ev: &Event) -> Option<(String, BufKind)> {
        if !self.buffer_trace.enabled {
            return None;
        }
        match ev {
            Event::SetBuffer { buffer, chunk, .. } => {
                let what = match chunk {
                    NewChunk::Named(n) => n.clone(),
                    _ => super::productions::render_request_spec(chunk),
                };
                Some((buffer.clone(), BufKind::SetChunk(what)))
            }
            Event::ClearBuffer { buffer, .. } => Some((buffer.clone(), BufKind::Clear)),
            Event::ModBuffer {
                buffer,
                slots,
                module,
            } => {
                let kind = if module == "PROCEDURAL" {
                    BufKind::Mod
                } else {
                    BufKind::ModRequest(render_slots_spec(slots))
                };
                Some((buffer.clone(), kind))
            }
            Event::ModuleRequest { buffer, spec, .. } => Some((
                buffer.clone(),
                BufKind::Request(super::productions::render_request_spec(spec)),
            )),
            _ => None,
        }
    }
}

/// A `buffer-trace`-relevant buffer event.
pub(super) enum BufKind {
    SetChunk(String),
    Mod,
    Clear,
    Request(String),
    ModRequest(String),
}

/// A buffer's BOLD demand state (`parse-trace-lists-for-bold`).
struct BoldDemand {
    name: String,
    current: Option<f64>,
    intervals: Vec<(f64, f64)>,
}

/// BOLD prediction processors (`other-files/bold.lisp`).
impl Engine {
    /// `bold-prediction-with-time` (`-scaled`): the BOLD prediction per
    /// buffer-trace buffer, sampled over time. The result is the time header
    /// followed by one `[buffer, value…]` row per buffer, sorted by name.
    pub fn bold_prediction_with_times(&self, scaled: bool) -> Vec<Json> {
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

        // `parse-trace-lists-for-bold`: per-buffer busy demand intervals.
        let mut demands: Vec<BoldDemand> = Vec::new();
        for (t_ms, rows) in &self.buffer_trace() {
            let t = *t_ms as f64 / 1000.0;
            for s in rows {
                let di = match demands.iter().position(|d| d.name == s.name) {
                    Some(i) => i,
                    None => {
                        demands.push(BoldDemand {
                            name: s.name.clone(),
                            current: None,
                            intervals: Vec::new(),
                        });
                        demands.len() - 1
                    }
                };
                let req = s.request.as_deref().map(|v| !v.is_empty()).unwrap_or(false);
                let chunk = s
                    .chunk_name
                    .as_deref()
                    .map(|v| !v.is_empty())
                    .unwrap_or(false);
                let d = &mut demands[di];
                if d.current.is_some() {
                    if !s.busy || s.busy_free || req {
                        d.intervals.push((d.current.unwrap(), t));
                        d.current = if req { Some(t) } else { None };
                    }
                } else if s.busy {
                    if req && (chunk || (s.error && !s.error_clear) || s.busy_free) {
                        d.intervals.push((t, t));
                    } else {
                        d.current = Some(t);
                    }
                } else if req || chunk {
                    d.intervals.push((t, t));
                }
            }
        }
        let end = self.time_ms as f64 / 1000.0;
        for d in &mut demands {
            if let Some(c) = d.current {
                d.intervals.push((c, end));
            }
        }
        let mut names: Vec<String> = demands.iter().map(|d| d.name.clone()).collect();
        names.sort();

        // Sample times: `start + inc/2`, step `inc`, up to `end - inc/2`.
        let mut times: Vec<Json> = vec![Json::Str("time".into())];
        let mut t = INC / 2.0;
        while t <= end - INC / 2.0 {
            times.push(Json::Float(t));
            t += INC;
        }
        let mut out = vec![Json::Arr(times)];
        for name in &names {
            let d = demands.iter().find(|d| &d.name == name).unwrap();
            let point = name == "GOAL";
            let bv = |demand: &[(f64, f64)], t: f64, a: f64| {
                super::compilation::bold_values(demand, t, INC, SETTLE, a, 1.0, point)
            };
            let mut vals: Vec<f64> = Vec::new();
            let mut t = INC / 2.0;
            while t <= end - INC / 2.0 {
                let pos = bv(&d.intervals, t, EXP);
                let neg = bv(&d.intervals, t, NEG_EXP);
                vals.push(pos * pos_factor - neg * neg_factor);
                t += INC;
            }
            if scaled {
                let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
                let delta = if min == max { 1.0 } else { max - min };
                vals = vals.into_iter().map(|x| (x - min) / delta).collect();
            }
            let mut row = vec![Json::Str(name.clone())];
            row.extend(vals.into_iter().map(Json::Float));
            out.push(Json::Arr(row));
        }
        out
    }
}

/// `production-history` recording (`other-files/production-history.lisp`).
impl Engine {
    /// Enable/disable `production-history` recording.
    pub fn set_production_history(&mut self, on: bool) {
        self.production_history.enabled = on;
    }

    /// The recorded `production-history` records, oldest first.
    pub fn production_history(&self) -> Vec<PHistRecord> {
        let mut r = self.production_history.records.clone();
        r.reverse();
        r
    }

    /// The distinct production failure reasons (`why-not-list`).
    pub fn production_history_why_not(&self) -> Vec<String> {
        self.production_history.why_not.clone()
    }

    /// The production transition graph from the recorded `production-history`:
    /// every production as a node, and an edge (with a count) for each
    /// consecutive selected pair. A pragmatic stand-in for
    /// `create-production-graph-coords`' full layout.
    pub fn production_graph(&self) -> (Vec<String>, Vec<(String, String, u32)>) {
        let nodes = self.all_production_names();
        let mut counts: std::collections::BTreeMap<(String, String), u32> = Default::default();
        let mut prev: Option<String> = None;
        for r in self.production_history() {
            match r.tag {
                PHistTag::Start => prev = None,
                PHistTag::Conflict => {
                    if let Some(sel) = &r.selected {
                        if let Some(p) = &prev {
                            *counts.entry((p.clone(), sel.clone())).or_insert(0) += 1;
                        }
                        prev = Some(sel.clone());
                    }
                }
                _ => {}
            }
        }
        let edges = counts.into_iter().map(|((a, b), c)| (a, b, c)).collect();
        (nodes, edges)
    }

    /// Per-production `[name, utility, u]` rows for the stop/reward markers.
    pub(super) fn p_hist_utility_info(&self) -> Vec<Vec<Json>> {
        self.model
            .productions
            .iter()
            .enumerate()
            .map(|(i, p)| {
                vec![
                    Json::Str(p.name.clone()),
                    self.production_utility[i]
                        .map(|u| Json::Float(u as f64))
                        .unwrap_or(Json::Null),
                    Json::Float(self.utilities[i] as f64),
                ]
            })
            .collect()
    }

    /// Record a run-start marker (`:run-start`).
    pub(super) fn p_hist_run_start(&mut self) {
        if self.production_history.enabled && !self.production_history.running {
            self.production_history.running = true;
            self.production_history.records.push(PHistRecord {
                tag: PHistTag::Start,
                time_ms: self.time_ms,
                selected: None,
                matched: Vec::new(),
                mismatched: Vec::new(),
                info: Vec::new(),
                reward: None,
            });
        }
    }

    /// Record a run-stop marker (`:run-end`).
    pub(super) fn p_hist_run_stop(&mut self) {
        if self.production_history.enabled && self.production_history.running {
            self.production_history.running = false;
            let info = self.p_hist_utility_info();
            let time_ms = self.time_ms;
            self.production_history.records.push(PHistRecord {
                tag: PHistTag::Stop,
                time_ms,
                selected: None,
                matched: Vec::new(),
                mismatched: Vec::new(),
                info,
                reward: None,
            });
        }
    }

    /// Record a reward marker (`production-history-reward-markers`).
    pub(super) fn p_hist_reward(&mut self, reward: Option<f32>) {
        if !self.production_history.enabled {
            return;
        }
        let info = self.p_hist_utility_info();
        let time_ms = self.time_ms;
        self.production_history.records.push(PHistRecord {
            tag: PHistTag::Reward,
            time_ms,
            selected: None,
            matched: Vec::new(),
            mismatched: Vec::new(),
            info,
            reward,
        });
    }

    /// Record a conflict-resolution block (`production-history-recorder`).
    pub(super) fn record_production_history(
        &mut self,
        cs: &[(usize, Bindings, f32, Vec<PpmMismatch>)],
        selected: Option<usize>,
    ) {
        if !self.production_history.enabled {
            return;
        }
        let matched: Vec<String> = cs
            .iter()
            .map(|(i, _, _, _)| self.model.productions[*i].name.clone())
            .collect();
        let mismatched: Vec<String> = self
            .model
            .productions
            .iter()
            .map(|p| p.name.clone())
            .filter(|n| !matched.contains(n))
            .collect();
        let mut info: Vec<Vec<Json>> = Vec::new();
        for (i, _, _, _) in cs {
            let name = self.model.productions[*i].name.clone();
            info.push(vec![
                Json::Str(name),
                self.production_utility[*i]
                    .map(|u| Json::Float(u as f64))
                    .unwrap_or(Json::Null),
                Json::Float(self.utilities[*i] as f64),
            ]);
        }
        for name in &mismatched {
            let Some(idx) = self.model.productions.iter().position(|p| &p.name == name) else {
                continue;
            };
            let prod = self.model.productions[idx].clone();
            let reason = self.why_not_match(&prod);
            let wi = match self
                .production_history
                .why_not
                .iter()
                .position(|r| r.eq_ignore_ascii_case(&reason))
            {
                Some(i) => i,
                None => {
                    self.production_history.why_not.push(reason);
                    self.production_history.why_not.len() - 1
                }
            };
            info.push(vec![
                Json::Str(name.clone()),
                Json::Int(wi as i64),
                Json::Float(self.utilities[idx] as f64),
            ]);
        }
        let time_ms = self.time_ms;
        self.production_history.records.push(PHistRecord {
            tag: PHistTag::Conflict,
            time_ms,
            selected: selected.map(|i| self.model.productions[i].name.clone()),
            matched,
            mismatched,
            info,
            reward: None,
        });
    }
}

/// Append a snapshot's `(chunk-text, status)` for `buf` to a history row,
/// empty when the buffer is absent from the snapshot.
fn push_snapshot(row: &mut Vec<String>, snap: &[(String, String, String)], buf: &str) {
    match snap.iter().find(|e| e.0 == buf) {
        Some(s) => {
            row.push(s.1.clone());
            row.push(s.2.clone());
        }
        None => {
            row.push(String::new());
            row.push(String::new());
        }
    }
}

/// `slot value` lines for a chunk spec (the mod/request `what` column).
fn render_slots_spec(slots: &[(String, Value)]) -> String {
    slots
        .iter()
        .map(|(s, v)| format!("{s} {}", v.show()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CtKind {
    TestSlot,
}

#[derive(Clone, PartialEq, Eq)]
struct CtCond {
    kind: CtKind,
    buffer: String,
    slot: String,
    test: String,
    value: String,
    /// `cr-condition-result` for this production class: whether the test is
    /// expected to be true (a `nil` slot test) or false.
    result: bool,
}

impl CtCond {
    fn key(&self) -> String {
        format!(
            "{:?}|{}|{}|{}|{}",
            self.kind as u8, self.buffer, self.slot, self.test, self.value
        )
    }

    fn display(&self) -> String {
        match self.kind {
            CtKind::TestSlot => format!(
                "TEST-SLOT {} {} {} {}",
                self.buffer, self.slot, self.test, self.value
            ),
        }
    }
}

#[derive(Clone)]
struct CtClass {
    names: Vec<String>,
    conds: Vec<CtCond>,
}

enum CtNode {
    Leaf(Vec<String>),
    Test {
        cond: CtCond,
        yes: Box<CtNode>,
        no: Box<CtNode>,
    },
}

fn build_ct_node(classes: &[CtClass]) -> Option<CtNode> {
    if classes.is_empty() {
        return None;
    }
    if classes.len() == 1 {
        return Some(CtNode::Leaf(classes[0].names.clone()));
    }
    let mut candidates: Vec<CtCond> = Vec::new();
    for c in classes {
        for cond in &c.conds {
            if !candidates.iter().any(|x| x.key() == cond.key()) {
                candidates.push(cond.clone());
            }
        }
    }
    // `build-tree-from-productions` chooses the highest information-gain
    // condition. The condition vector ends up in reverse discovery order for
    // equal-gain ties in this corpus (`fan-w-tree`'s ARG2 vs ARG1).
    let mut best: Option<(f64, CtCond, Vec<CtClass>, Vec<CtClass>)> = None;
    for cond in candidates.into_iter().rev() {
        let (gain, yes, no) = ct_split(classes, &cond);
        if yes.is_empty() || no.is_empty() {
            continue;
        }
        if best.as_ref().is_none_or(|(b, _, _, _)| gain > *b) {
            best = Some((gain, cond, yes, no));
        }
    }
    if let Some((gain, cond, yes, no)) = best {
        if gain > 0.0 {
            let yes = build_ct_node(&yes)?;
            let no = build_ct_node(&no)?;
            return Some(CtNode::Test {
                cond,
                yes: Box::new(yes),
                no: Box::new(no),
            });
        }
    }
    let mut names = Vec::new();
    for c in classes {
        names.extend(c.names.clone());
    }
    Some(CtNode::Leaf(names))
}

/// `tree-entropy` (`commands/conflict-tree.lisp:575`): entropy over the
/// production counts of the condition classes.
fn ct_entropy(classes: &[CtClass]) -> f64 {
    let n: usize = classes.iter().map(|c| c.names.len()).sum();
    if n == 0 {
        return 0.0;
    }
    classes
        .iter()
        .filter(|c| !c.names.is_empty())
        .map(|c| {
            let p = c.names.len() as f64 / n as f64;
            -p * p.log2()
        })
        .sum()
}

fn ct_group(classes: &[CtClass]) -> Vec<CtClass> {
    let mut out: Vec<CtClass> = Vec::new();
    for c in classes {
        if let Some(e) = out.iter_mut().find(|e| e.conds == c.conds) {
            e.names.extend(c.names.clone());
        } else {
            out.push(CtClass {
                names: c.names.clone(),
                conds: c.conds.clone(),
            });
        }
    }
    out
}

/// `split-productions-with-condition` (`commands/conflict-tree.lisp:602`) for
/// a binary test-slot condition. Productions that do not test the condition
/// are present in *both* branches.
fn ct_split(classes: &[CtClass], cond: &CtCond) -> (f64, Vec<CtClass>, Vec<CtClass>) {
    let haves: Vec<CtClass> = classes
        .iter()
        .filter(|c| c.conds.iter().any(|x| x.key() == cond.key()))
        .cloned()
        .collect();
    let have_nots: Vec<CtClass> = classes
        .iter()
        .filter(|c| !c.conds.iter().any(|x| x.key() == cond.key()))
        .cloned()
        .collect();
    if haves.is_empty() {
        return (0.0, Vec::new(), have_nots);
    }
    let base = ct_entropy(&haves);
    let mut yes_without: Vec<CtClass> = Vec::new();
    let mut no_without: Vec<CtClass> = Vec::new();
    for c in &haves {
        let result = c
            .conds
            .iter()
            .find(|x| x.key() == cond.key())
            .map(|x| x.result)
            .unwrap_or(false);
        let without = CtClass {
            names: c.names.clone(),
            conds: c
                .conds
                .iter()
                .filter(|x| x.key() != cond.key())
                .cloned()
                .collect(),
        };
        if result {
            yes_without.push(without);
        } else {
            no_without.push(without);
        }
    }
    let yes_grouped = ct_group(&yes_without);
    let no_grouped = ct_group(&no_without);
    let total: f64 = yes_grouped
        .iter()
        .chain(no_grouped.iter())
        .map(|c| c.names.len() as f64)
        .sum();
    let gain = if total == 0.0 {
        0.0
    } else {
        let weighted = (yes_grouped
            .iter()
            .map(|c| c.names.len() as f64)
            .sum::<f64>()
            / total)
            * ct_entropy(&yes_grouped)
            + (no_grouped.iter().map(|c| c.names.len() as f64).sum::<f64>() / total)
                * ct_entropy(&no_grouped);
        base - weighted
    };
    let mut yes = yes_grouped;
    let mut no = no_grouped;
    if !have_nots.is_empty() {
        yes.extend(have_nots.clone());
        no.extend(have_nots);
        yes = ct_group(&yes);
        no = ct_group(&no);
    }
    (gain, yes, no)
}

/// A chunk-type default value as Lisp `~S` would print it
/// (`(chunk-type �?� (slot default))`).
fn pattern_show(p: &crate::model::Pattern) -> String {
    use crate::model::Pattern;
    match p {
        Pattern::Nil => "NIL".to_string(),
        Pattern::Var(v) => format!("={v}"),
        Pattern::Sym(s) => s.clone(),
        Pattern::Int(i) => i.to_string(),
        Pattern::Num(x) => x.to_string(),
        Pattern::Str(s) => format!("\"{s}\""),
        Pattern::List(ps) => format!(
            "({})",
            ps.iter().map(pattern_show).collect::<Vec<_>>().join(" ")
        ),
    }
}

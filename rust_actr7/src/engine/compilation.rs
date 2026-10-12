//! Production compilation: composition, rendering, utility/activation helpers.
use super::*;
impl SlotVal {
    pub(super) fn render(&self) -> String {
        match self {
            SlotVal::Var(v) => format!("={v}"),
            SlotVal::Lit(s) => s.clone(),
        }
    }
}

/// Instantiated LHS match slots per buffer, sorted by slot name
/// (`sort-production-slots`). `isa` is the chunk type, not a slot. Values
/// stay symbolic (`=var`) so cross-production mappings can apply first.
/// The variables of a production minus its buffer-name variables
/// (`basic-variables`, `production-compilation.lisp:1964`): the ones
/// `rename-variables` disambiguates across a composition.
pub(super) fn basic_vars(p: &Production) -> HashSet<String> {
    let mut s: HashSet<String> = HashSet::new();
    for item in &p.lhs {
        match item {
            LhsItem::Match { tests, .. } => {
                for t in tests {
                    if let Pattern::Var(x) = &t.value {
                        s.insert(x.clone());
                    }
                }
            }
            LhsItem::Query { queries, .. } => {
                for (_, v) in queries {
                    if let Pattern::Var(x) = v {
                        s.insert(x.clone());
                    }
                }
            }
            _ => {}
        }
    }
    for a in &p.rhs {
        match a {
            RhsAction::Modify { slots, .. } | RhsAction::ModifyRequest { slots, .. } => {
                for (_, v) in slots {
                    if let Pattern::Var(x) = v {
                        s.insert(x.clone());
                    }
                }
            }
            RhsAction::Request { spec, .. } | RhsAction::DirectSet { spec, .. } => {
                if let ChunkDesc::Spec {
                    slots, neg, rels, ..
                } = spec
                {
                    for (_, v) in slots.iter().chain(neg.iter()) {
                        if let Pattern::Var(x) = v {
                            s.insert(x.clone());
                        }
                    }
                    for (_, v, _) in rels {
                        if let Pattern::Var(x) = v {
                            s.insert(x.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    s.retain(|v| !is_buffer_name(&v.to_uppercase()));
    s
}

/// Defaults for ACT-R's built-in module chunk-types that the engine doesn't
/// load from the model file. `vision.lisp:3067`: `(chunk-type
/// (text (:include visual-object)) (text t))`, etc. Only the defaults that
/// show up in composed conditions matter.
pub(super) fn builtin_type_defaults(name: &str) -> &'static [(&'static str, &'static str)] {
    match name.to_uppercase().as_str() {
        "TEXT" => &[("TEXT", "T")],
        "OVAL" => &[("OVAL", "T")],
        "LINE" => &[("LINE", "T")],
        _ => &[],
    }
}

/// The LHS buffers whose slot values mention each variable (used to decide
/// whether a shared variable really binds to the same place in both
/// productions).
pub(super) fn var_buffers(p: &Production) -> HashMap<String, HashSet<(String, String)>> {
    let mut m: HashMap<String, HashSet<(String, String)>> = HashMap::new();
    for item in &p.lhs {
        if let LhsItem::Match { buffer, tests } = item {
            for t in tests {
                if let Pattern::Var(x) = &t.value {
                    m.entry(x.clone())
                        .or_default()
                        .insert((buffer.clone(), t.slot.clone()));
                }
            }
        }
    }
    m
}

pub(super) fn prod_lhs_slots(
    p: &Production,
    _b: &Bindings,
    types: &[crate::model::ChunkType],
) -> Vec<(String, Vec<(String, SlotVal)>)> {
    let mut out = Vec::new();
    for item in &p.lhs {
        if let LhsItem::Match { buffer, tests } = item {
            let mut slots: Vec<(String, SlotVal)> = tests
                .iter()
                .filter(|t| t.slot != "ISA")
                .map(|t| {
                    let v = match &t.value {
                        Pattern::Var(v) => SlotVal::Var(v.clone()),
                        other => SlotVal::Lit(
                            Value::from_pattern(other)
                                .map(|x| x.show())
                                .unwrap_or_default(),
                        ),
                    };
                    (t.slot.clone(), v)
                })
                .collect();
            // `isa TYPE` adds the chunk-type's default slot tests
            // (`process-slots-specs`, chunk-spec.lisp:597), so the composed
            // condition prints them too (`isa response` -> `is-response t`).
            if let Some(ct) = tests
                .iter()
                .find(|t| t.slot == "ISA")
                .and_then(|t| match &t.value {
                    Pattern::Sym(s) => Some(s.clone()),
                    _ => None,
                })
            {
                let mut defaults: Vec<(String, String)> = Vec::new();
                if let Some(t) = types.iter().find(|t| t.name.eq_ignore_ascii_case(&ct)) {
                    defaults.extend(t.defaults.iter().map(|(slot, pat)| {
                        (
                            slot.to_uppercase(),
                            Value::from_pattern(pat)
                                .map(|x| x.show())
                                .unwrap_or_default(),
                        )
                    }));
                }
                // ACT-R's built-in module chunk-types aren't in the model
                // file (`vision.lisp:3067`): `isa text` also adds `text t`.
                for (slot, val) in builtin_type_defaults(&ct) {
                    if !defaults.iter().any(|(s, _)| s.eq_ignore_ascii_case(slot)) {
                        defaults.push((slot.to_string(), val.to_string()));
                    }
                }
                for (slot, val) in defaults {
                    if !slots.iter().any(|(s, _)| s.eq_ignore_ascii_case(&slot)) {
                        slots.push((slot, SlotVal::Lit(val)));
                    }
                }
            }
            slots.sort_by(|a, b| a.0.cmp(&b.0));
            out.push((buffer.clone(), slots));
        } else if let LhsItem::Bind { var, value, safe } = item {
            // A `!safe-bind!`/`!bind!` condition joins the composed LHS.
            let token = if *safe { "!SAFE-BIND!" } else { "!BIND!" };
            out.push((
                token.to_string(),
                vec![(format!("={var}"), SlotVal::Lit(sexp_lisp(value)))],
            ));
        } else if let LhsItem::Query { buffer, queries } = item {
            // State/failure queries (`?imaginal> state free`).
            let mut slots: Vec<(String, SlotVal)> = queries
                .iter()
                .filter(|(s, _)| s != "ISA")
                .map(|(s, v)| {
                    let sv = match v {
                        Pattern::Var(vv) => SlotVal::Var(vv.clone()),
                        other => SlotVal::Lit(
                            Value::from_pattern(other)
                                .map(|x| x.show())
                                .unwrap_or_default(),
                        ),
                    };
                    (s.clone(), sv)
                })
                .collect();
            slots.sort_by(|a, b| a.0.cmp(&b.0));
            out.push((format!("?{buffer}>"), slots));
        }
    }
    out
}

/// The printed value bound to an LHS variable, if bound.
pub(super) fn instantiate_var(name: &str, b: &Bindings) -> Option<String> {
    b.get(name).map(|v| v.show())
}

/// A canonical production comparison rep (`production-comparison-rep`,
/// `production-compilation.lisp:2044`): LHS/RHS tokens with their slots,
/// tokens and slots sorted. Null `=buffer>` modifies are dropped (ACT-R's
/// `COMPOSE-GOAL-BUFFER` may keep them; the composed side here does not).
#[allow(clippy::type_complexity)]
pub(super) fn canon_prod(
    prod: &Production,
) -> (
    Vec<(String, Vec<(String, String)>)>,
    Vec<(String, Vec<(String, String)>)>,
) {
    let mut lhs: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for item in &prod.lhs {
        match item {
            LhsItem::Match { buffer, tests } => {
                let mut slots: Vec<(String, String)> = tests
                    .iter()
                    .filter(|t| t.slot != "ISA")
                    .map(|t| (t.slot.clone(), canon_pattern(&t.value)))
                    .collect();
                slots.sort();
                lhs.push((format!("={buffer}>"), slots));
            }
            LhsItem::Query { buffer, queries } => {
                let mut slots: Vec<(String, String)> = queries
                    .iter()
                    .map(|(s, v)| (s.clone(), canon_pattern(v)))
                    .collect();
                slots.sort();
                lhs.push((format!("?{buffer}>"), slots));
            }
            _ => {}
        }
    }
    lhs.sort_by(|a, b| a.0.cmp(&b.0));
    let mut rhs: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for a in &prod.rhs {
        if let Some((tok, mut slots)) = canon_action(a) {
            slots.sort();
            rhs.push((tok, slots));
        }
    }
    rhs.sort_by(|a, b| a.0.cmp(&b.0));
    (lhs, rhs)
}

/// The canonical token/slots of one RHS action (`None` for non-buffer ones).
pub(super) fn canon_action(a: &RhsAction) -> Option<(String, Vec<(String, String)>)> {
    let pair = |buffer: &str, slots: Vec<(String, String)>| Some((format!("{buffer}>"), slots));
    match a {
        RhsAction::Modify { buffer, slots } => pair(
            &format!("={buffer}"),
            slots
                .iter()
                .map(|(n, v)| (n.clone(), canon_pattern(v)))
                .collect(),
        ),
        RhsAction::ModifyRequest { buffer, slots } => pair(
            &format!("*{buffer}"),
            slots
                .iter()
                .map(|(n, v)| (n.clone(), canon_pattern(v)))
                .collect(),
        ),
        RhsAction::Request { buffer, spec } => match spec {
            ChunkDesc::Spec { slots, .. } => pair(
                &format!("+{buffer}"),
                slots
                    .iter()
                    .map(|(n, v)| (n.clone(), canon_pattern(v)))
                    .collect(),
            ),
            ChunkDesc::Named(n) => pair(&format!("+{buffer}"), vec![("=".into(), format!("={n}"))]),
        },
        RhsAction::Clear { buffer } => Some((format!("-{buffer}>"), Vec::new())),
        _ => None,
    }
}

pub(super) fn canon_pattern(p: &Pattern) -> String {
    match p {
        Pattern::Var(v) => format!("={v}"),
        Pattern::Nil => "NIL".to_string(),
        other => Value::from_pattern(other)
            .map(|x| x.show())
            .unwrap_or_default(),
    }
}

/// The canonical rep of the composed (lhs, rhs) pair, matching [`canon_prod`].
#[allow(clippy::type_complexity)]
pub(super) fn canon_composed(
    lhs: &[(String, Vec<(String, SlotVal)>)],
    rhs: &[(String, Vec<(String, String)>)],
) -> (
    Vec<(String, Vec<(String, String)>)>,
    Vec<(String, Vec<(String, String)>)>,
) {
    let mut cl: Vec<(String, Vec<(String, String)>)> = lhs
        .iter()
        .map(|(tok, slots)| {
            let mut s: Vec<(String, String)> =
                slots.iter().map(|(n, v)| (n.clone(), v.render())).collect();
            s.sort();
            (tok.clone(), s)
        })
        .collect();
    cl.sort_by(|a, b| a.0.cmp(&b.0));
    let mut cr: Vec<(String, Vec<(String, String)>)> = rhs
        .iter()
        .map(|(tok, slots)| {
            let mut s = slots.clone();
            s.sort();
            (tok.clone(), s)
        })
        .collect();
    cr.sort_by(|a, b| a.0.cmp(&b.0));
    (cl, cr)
}

/// Instantiated RHS actions per buffer as `(op, buffer, slots, var)`:
/// `=` modify, `*` modify-request, `+` request (var is the referenced
/// variable for a whole-chunk request), `-` clear.
pub(super) fn prod_rhs_actions(
    p: &Production,
    b: &Bindings,
) -> Vec<(char, String, Vec<(String, SlotVal)>, Option<String>)> {
    let mut out = Vec::new();
    for a in &p.rhs {
        match a {
            RhsAction::Modify { buffer, slots } => out.push((
                '=',
                buffer.clone(),
                slots
                    .iter()
                    .map(|(n, v)| {
                        (
                            n.clone(),
                            match v {
                                Pattern::Var(v) => SlotVal::Var(v.clone()),
                                other => SlotVal::Lit(
                                    Value::from_pattern(other)
                                        .map(|x| x.show())
                                        .unwrap_or_default(),
                                ),
                            },
                        )
                    })
                    .collect(),
                None,
            )),
            RhsAction::ModifyRequest { buffer, slots } => out.push((
                '*',
                buffer.clone(),
                slots
                    .iter()
                    .map(|(n, v)| {
                        (
                            n.clone(),
                            match v {
                                Pattern::Var(v) => SlotVal::Var(v.clone()),
                                other => SlotVal::Lit(
                                    Value::from_pattern(other)
                                        .map(|x| x.show())
                                        .unwrap_or_default(),
                                ),
                            },
                        )
                    })
                    .collect(),
                None,
            )),
            RhsAction::Request { buffer, spec } => match spec {
                ChunkDesc::Named(n) if n.starts_with('=') => {
                    out.push(('r', buffer.clone(), Vec::new(), Some(n[1..].to_string())))
                }
                ChunkDesc::Spec { slots, rels, .. } => {
                    let mut s: Vec<(String, SlotVal)> = slots
                        .iter()
                        .map(|(n, v)| {
                            (
                                n.clone(),
                                match v {
                                    Pattern::Var(v) => SlotVal::Var(v.clone()),
                                    other => SlotVal::Lit(
                                        Value::from_pattern(other)
                                            .map(|x| x.show())
                                            .unwrap_or_default(),
                                    ),
                                },
                            )
                        })
                        .collect();
                    // Relative tests (`> screen-x current`) are carried as
                    // `"<op> <slot>"` pseudo-slots so both the trace
                    // (`render_compiled`) and the installed production
                    // (`install_compiled`) keep them.
                    for (name, v, op) in rels {
                        s.push((
                            format!("{} {}", slot_op_str(*op), name),
                            SlotVal::Lit(
                                Value::from_pattern(v).map(|x| x.show()).unwrap_or_default(),
                            ),
                        ));
                    }
                    out.push(('+', buffer.clone(), s, None));
                }
                _ => {}
            },
            RhsAction::Clear { buffer } => out.push(('-', buffer.clone(), Vec::new(), None)),
            _ => {}
        }
    }
    let _ = b;
    out
}

/// CL `remove-duplicates` ordering: drop earlier duplicates, keeping the
/// later copy, preserving the original order of the kept elements.
pub(super) fn remove_dup_keep_last<T: PartialEq + Clone>(v: Vec<T>) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    for i in 0..v.len() {
        if !v[i + 1..].iter().any(|x| x == &v[i]) {
            out.push(v[i].clone());
        }
    }
    out
}

/// Render a compiled production the way `print-production-text` does (the
/// `:pct` trace block): `(P NAME`, the `"A & B"` documentation, one
/// `=BUFFER>` group per condition/action, then `)`.
pub(super) fn render_compiled(
    name: &str,
    doc: &str,
    lhs: &[(String, Vec<(String, SlotVal)>)],
    rhs: &[(String, Vec<(String, String)>)],
    b: &Bindings,
) -> String {
    let mut s = format!("(P {name}\n  \"{doc}\"\n");
    for (token, slots) in lhs {
        if token.starts_with('!') {
            let args: Vec<String> = slots
                .iter()
                .map(|(n, v)| format!("{n} {}", v.render()))
                .collect();
            s.push_str(&format!("   {token} {}\n", args.join(" ")));
            continue;
        }
        s.push_str(&format!("   {token}\n"));
        for (n, v) in slots {
            // A dynamic slot shows its bound name (the canonical form keeps
            // `=var`, so substitute only for the trace).
            s.push_str(&format!("       {} {}\n", slot_disp(n, b), v.render()));
        }
    }
    s.push_str(" ==>\n");
    for (token, slots) in rhs {
        s.push_str(&format!("   {token}\n"));
        for (n, v) in slots {
            s.push_str(&render_rhs_slot(&slot_disp(n, b), v));
        }
    }
    s.push(')');
    s
}

/// One composed-action slot line: a relative pseudo-slot (`"<op> <slot>"`,
/// from a request's `> slot value`) prints the way `render_production` does
/// (`    >  slot value`); everything else is `       slot value`.
pub(super) fn render_rhs_slot(n: &str, v: &str) -> String {
    for op in [">=", "<=", ">", "<"] {
        if let Some(rest) = n.strip_prefix(op) {
            if let Some(slot) = rest.strip_prefix(' ') {
                return format!("    {op:<2} {slot} {v}\n");
            }
        }
    }
    format!("       {n} {v}\n")
}

/// `sort-conditions`'s key (`production-compilation.lisp:1905`): the
/// condition token uppercased, or `"z"` so `!`-conditions sort last.
pub(super) fn sort_token(token: &str) -> String {
    if token.starts_with('!') {
        "z".to_string()
    } else {
        token.to_uppercase()
    }
}

/// `get-buffer-index` (`production-compilation.lisp:1441`): bitmask of how a
/// production uses a buffer — LHS match +8, LHS query +16, RHS `=` +1,
/// RHS `-` +2, RHS `+` +4, RHS `*` +32.
pub(super) fn buffer_usage(p: &crate::model::Production, buffer: &str) -> u32 {
    let mut u = 0;
    if p.lhs
        .iter()
        .any(|i| matches!(i, LhsItem::Match { buffer: b, .. } if b == buffer))
    {
        u += 8;
    }
    if p.lhs
        .iter()
        .any(|i| matches!(i, LhsItem::Query { buffer: b, .. } if b == buffer))
    {
        u += 16;
    }
    for a in &p.rhs {
        match a {
            RhsAction::Modify { buffer: b, .. } if b == buffer => u += 1,
            RhsAction::Clear { buffer: b } if b == buffer => u += 2,
            RhsAction::Request { buffer: b, .. } if b == buffer => u += 4,
            RhsAction::ModifyRequest { buffer: b, .. } if b == buffer => u += 32,
            _ => {}
        }
    }
    u
}

/// `production-lhs-buffers` (`support/production-parsing.lisp:435`): the
/// `=`-statement targets of the LHS, in source order (query buffers are not
/// included).
pub(super) fn lhs_buffers(p: &crate::model::Production) -> Vec<String> {
    p.lhs
        .iter()
        .filter_map(|i| match i {
            LhsItem::Match { buffer, .. } => Some(buffer.clone()),
            _ => None,
        })
        .collect()
}

/// `production-rhs-buffers` (`support/production-parsing.lisp:441`): the
/// RHS statement targets, in source order, deduplicated.
pub(super) fn rhs_buffers(p: &crate::model::Production) -> Vec<String> {
    let mut out = Vec::new();
    for a in &p.rhs {
        if let Some(b) = rhs_buffer(a) {
            if !out.contains(&b) {
                out.push(b);
            }
        }
    }
    out
}

/// `R-B-C1` (`retrieval-compilation.lisp:223`): when the first production
/// requests the buffer and the second does not harvest it, the second may
/// only query `state busy` or `buffer empty`.
pub(super) fn r_b_c1(p2: &crate::model::Production, buffer: &str) -> bool {
    match p2.lhs.iter().find_map(|i| match i {
        LhsItem::Query { buffer: b, queries } if b == buffer => Some(queries),
        _ => None,
    }) {
        None => true,
        Some(qs) => qs.iter().all(|(s, v)| {
            (s.eq_ignore_ascii_case("STATE")
                && matches!(v, Pattern::Sym(x) if x.eq_ignore_ascii_case("busy")))
                || (s.eq_ignore_ascii_case("BUFFER")
                    && matches!(v, Pattern::Sym(x) if x.eq_ignore_ascii_case("empty")))
        }),
    }
}

/// `R-B-C2` (`retrieval-compilation.lisp:237`): the queries in both
/// productions must be the same.
pub(super) fn r_b_c2(
    p1: &crate::model::Production,
    p2: &crate::model::Production,
    buffer: &str,
) -> bool {
    let q = |p: &crate::model::Production| -> Vec<(String, Pattern)> {
        p.lhs
            .iter()
            .find_map(|i| match i {
                LhsItem::Query { buffer: b, queries } if b == buffer => Some(queries.clone()),
                _ => None,
            })
            .unwrap_or_default()
    };
    let q1 = q(p1);
    let q2 = q(p2);
    let mut all: Vec<&(String, Pattern)> = q1.iter().chain(q2.iter()).collect();
    all.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    all.dedup();
    q1.len() == q2.len() && q1.len() == all.len()
}

/// `retrieval-reason` (`retrieval-compilation.lisp:255`), the
/// `failed-function = nil` branch (checks are not modelled).
pub(super) fn retrieval_reason(u1: u32, u2: u32) -> String {
    let s = if u1 > 30 {
        "buffer modification actions in first production are not allowed."
    } else if u2 > 30 {
        "buffer modification actions in second production are not allowed."
    } else if matches!(u1, 9 | 11 | 13 | 15 | 25 | 27 | 29) {
        "buffer modification actions in first production are not allowed."
    } else if matches!(u1, 2 | 6 | 10 | 14 | 18 | 22 | 26 | 30) {
        "the buffer is explicitly cleared in the first production."
    } else if matches!(u2, 2 | 6 | 10 | 14 | 18 | 22 | 26 | 30) {
        "the buffer is explicitly cleared in the second production."
    } else if matches!(u2, 9 | 11 | 13 | 15 | 25 | 27 | 29) {
        "buffer modification actions in second production are not allowed."
    } else {
        "strict harvesting should have prevented the buffer condition from matching in the second production."
    };
    s.to_string()
}

/// `perceptual-reason` (`perceptual-compilation.lisp:259`), `nil` branch.
pub(super) fn perceptual_reason(u1: u32, u2: u32) -> String {
    let s = if u1 > 30 {
        "buffer modification actions in first production are not allowed."
    } else if u2 > 30 {
        "buffer modification actions in second production are not allowed."
    } else if matches!(u1, 9 | 11 | 13 | 15 | 25 | 27 | 29) {
        "buffer modification actions in first production are not allowed."
    } else if matches!(u1, 2 | 6 | 10 | 14 | 18 | 22 | 26 | 30) {
        "the buffer is explicitly cleared in the first production."
    } else if matches!(u2, 2 | 6 | 10 | 14 | 18 | 22 | 26 | 30) {
        "the buffer is explicitly cleared in the second production."
    } else if matches!(u2, 9 | 11 | 13 | 15 | 25 | 27 | 29) {
        "buffer modification actions in second production are not allowed."
    } else if matches!(u2, 4 | 20) {
        "both productions make a request."
    } else if matches!(u1, 4 | 12 | 20 | 28) {
        "the first production makes a request and the second production harvests the chunk."
    } else {
        "the second production harvests a chunk which is there because of buffer stuffing or because strict harvesting has been disabled."
    };
    s.to_string()
}

/// `goal-reason` (`goal-compilation.lisp:670`), `nil` branch.
pub(super) fn goal_reason(u1: u32, u2: u32) -> String {
    const CLEARED: &[u32] = &[2, 6, 10, 11, 14, 15, 42, 43, 46, 47];
    const QUERIES: &[u32] = &[
        16, 18, 20, 22, 24, 25, 26, 27, 28, 29, 30, 31, 56, 57, 58, 60, 59, 61, 62, 63,
    ];
    let s = if CLEARED.contains(&u1) {
        "the buffer is explicitly cleared in the first production"
    } else if QUERIES.contains(&u1) {
        "there are queries of the buffer in the first production"
    } else if matches!(u1, 41 | 45) {
        "the first production makes both a modification and a modification request"
    } else if CLEARED.contains(&u2) {
        "the buffer is explicitly cleared in the second production"
    } else if QUERIES.contains(&u2) {
        "there are queries of the buffer in the second production"
    } else if matches!(u2, 41 | 45) {
        "the second production makes both a modification and a modification request"
    } else if matches!(u1, 4 | 12 | 13 | 44) {
        "both productions make requests"
    } else {
        "one production makes a modification and the other makes a modification request"
    };
    s.to_string()
}

/// `imaginal-reason` (`imaginal-compilation.lisp:753`), `nil` branch.
pub(super) fn imaginal_reason(u1: u32, u2: u32) -> String {
    const CLEARED: &[u32] = &[
        2, 6, 10, 11, 14, 15, 42, 43, 46, 47, 18, 22, 26, 27, 30, 31, 58, 59, 63, 62,
    ];
    let s = if CLEARED.contains(&u1) {
        "the buffer is explicitly cleared in the first production."
    } else if matches!(u1, 44 | 45 | 60 | 61) {
        "the first production makes both a request and a modification request."
    } else if matches!(u1, 41 | 57) {
        "the first production makes both a modification and a modification request."
    } else if CLEARED.contains(&u2) {
        "the buffer is explicitly cleared in the second production."
    } else if matches!(u2, 44 | 45 | 60 | 61) {
        "the second production makes both a request and a modification request."
    } else if matches!(u2, 41 | 57) {
        "the second production makes both a modification and a modification request."
    } else if matches!(u2, 40 | 56) {
        "the first production makes a modification and the second makes a modification request."
    } else if matches!(u1, 4 | 12 | 13 | 20 | 28 | 29) {
        "both productions make requests."
    } else if matches!(u2, 4 | 12 | 13 | 20 | 28 | 29) {
        "the first production makes a modification request and the second makes a request."
    } else {
        "the first production makes a modification request and the other makes a modification."
    };
    s.to_string()
}

/// `motor-reason` (`motor-compilation.lisp:125`), `nil` branch (the default
/// type's reason).
pub(super) fn motor_reason(u1: u32, u2: u32) -> String {
    const TESTS: &[u32] = &[
        8, 9, 10, 11, 12, 13, 14, 15, 24, 25, 26, 27, 28, 29, 30, 31, 40, 41, 42, 44, 43, 45, 46,
        47, 56, 57, 58, 60, 59, 61, 62, 63,
    ];
    let s = if TESTS.contains(&u1) {
        "buffer tests in first production are not allowed."
    } else if matches!(u1, 2 | 6 | 18 | 22) {
        "the buffer is explicitly cleared in the first production."
    } else if TESTS.contains(&u2) {
        "buffer tests in second production are not allowed."
    } else if matches!(u2, 2 | 6 | 18 | 22) {
        "the buffer is explicitly cleared in the second production."
    } else {
        "both productions make requests."
    };
    s.to_string()
}

/// The buffers an action targets.
pub(super) fn rhs_buffer(a: &RhsAction) -> Option<String> {
    match a {
        RhsAction::Request { buffer, .. }
        | RhsAction::Modify { buffer, .. }
        | RhsAction::ModifyRequest { buffer, .. }
        | RhsAction::Clear { buffer } => Some(buffer.clone()),
        _ => None,
    }
}

/// `compute-retain-buffers` (`production-compilation.lisp:2170`): with
/// `:cia`+`:rir`, which productions keep their indirect retrieval request
/// (rendered `+RETRIEVAL> =var`) rather than expanding it.
pub(super) fn compute_retain_buffers(
    p1: &Production,
    p2: &Production,
) -> (Vec<String>, Vec<String>) {
    let mut retain1 = Vec::new();
    let mut retain2 = Vec::new();
    let mut buffers: Vec<String> = Vec::new();
    for a in p1.rhs.iter().chain(p2.rhs.iter()) {
        if let Some(b) = rhs_buffer(a) {
            if !buffers.contains(&b) {
                buffers.push(b);
            }
        }
    }
    for x in &buffers {
        // Only retrieval-style buffers.
        if x != "RETRIEVAL" {
            continue;
        }
        let u1 = buffer_usage(p1, x);
        let u2 = buffer_usage(p2, x);
        match u2 {
            4 | 12 | 20 | 28 => retain2.push(x.clone()),
            8 | 24 => {}
            _ => {
                if matches!(u1, 4 | 12 | 20 | 28) {
                    retain1.push(x.clone());
                }
            }
        }
    }
    (retain1, retain2)
}

/// Render a raw `Sexp` the way Lisp prints it (for `!bind!` definitions).
pub(super) fn sexp_lisp(e: &Sexp) -> String {
    match e {
        Sexp::Sym(s) => s.clone(),
        Sexp::Str(s) => format!("{s:?}"),
        Sexp::Int(i) => i.to_string(),
        Sexp::Float(x) => lisp_s_f32(*x as f32),
        Sexp::List(v) => format!(
            "({})",
            v.iter().map(sexp_lisp).collect::<Vec<_>>().join(" ")
        ),
    }
}

/// `produce-composition-representation`'s early return
/// (`production-compilation.lisp:2258`): a non-retained indirect request
/// whose variable is bound by a LHS `!safe-bind!` blocks composition.
pub(super) fn indirect_safe_bind_reason(p: &Production, retain: &[String]) -> Option<String> {
    let bind_vars = production_bind_vars(p);
    for a in &p.rhs {
        if let RhsAction::Request {
            buffer,
            spec: ChunkDesc::Named(n),
        } = a
        {
            if let Some(v) = n.strip_prefix('=') {
                if !retain.contains(buffer) && bind_vars.contains(&v.to_uppercase()) {
                    return Some(format!(
                        "an indirect request with buffer {buffer} uses a variable from a LHS !safe-bind!"
                    ));
                }
            }
        }
    }
    None
}

/// True for a buffer name (an RHS `=BUFFER` reference rather than a slot
/// variable).
pub(super) fn is_buffer_name(s: &str) -> bool {
    matches!(
        s,
        "GOAL"
            | "RETRIEVAL"
            | "IMAGINAL"
            | "VISUAL"
            | "VISUAL-LOCATION"
            | "MANUAL"
            | "BLENDING"
            | "TEMPORAL"
    )
}

/// The module a buffer's events are traced under (`:module` of a scheduled
/// buffer event).
pub(super) fn buffer_module(buffer: &str) -> String {
    match buffer {
        "RETRIEVAL" => "DECLARATIVE".to_string(),
        "VISUAL" | "VISUAL-LOCATION" => "VISION".to_string(),
        "MANUAL" => "MOTOR".to_string(),
        other => other.to_string(),
    }
}

/// The buffer of an indirect action: an RHS `=`/`*`/`+` statement whose
/// definition is a single chunk-spec variable (`+retrieval> =goal`).
/// (`indirect-action-buffer-name`, production-compilation.lisp:1280`.)
pub(super) fn indirect_action_buffer_name(prod: &Production) -> Option<String> {
    for a in &prod.rhs {
        if let RhsAction::Request {
            buffer,
            spec: ChunkDesc::Named(n),
        } = a
        {
            if n.starts_with('=') {
                return Some(buffer.clone());
            }
        }
    }
    None
}

/// `indirect-action-variables` (`production-compilation.lisp:1218`): the
/// variables named by single-variable requests. With `:rir` the retrieval
/// style is ignored.
pub(super) fn indirect_action_vars(prod: &Production, ignore_retrieval: bool) -> Vec<String> {
    let mut out = Vec::new();
    for a in &prod.rhs {
        if let RhsAction::Request {
            buffer,
            spec: ChunkDesc::Named(n),
        } = a
        {
            if let Some(v) = n.strip_prefix('=') {
                if ignore_retrieval && buffer == "RETRIEVAL" {
                    continue;
                }
                out.push(v.to_uppercase());
            }
        }
    }
    out
}

/// `production-bind-variables` (`production-compilation.lisp:1229`): the
/// variables bound by `!bind!`/`!safe-bind!`/`!mv-bind!`.
pub(super) fn production_bind_vars(prod: &Production) -> Vec<String> {
    let mut out = Vec::new();
    for item in &prod.lhs {
        if let LhsItem::Bind { var, .. } = item {
            out.push(var.to_uppercase());
        }
    }
    for a in &prod.rhs {
        match a {
            RhsAction::Bind { var, .. } => out.push(var.to_uppercase()),
            RhsAction::MvBind { vars, .. } => out.extend(vars.iter().map(|v| v.to_uppercase())),
            _ => {}
        }
    }
    out
}

/// Instantiate a pattern for `print-production-text`: bound variables show
/// their value, unbound ones keep `=NAME`.
pub(super) fn show_pat(p: &Pattern, b: &Bindings) -> String {
    match p {
        Pattern::Var(v) => match b.get(v) {
            Some(val) => val.show(),
            None => format!("={v}"),
        },
        other => Value::from_pattern(other)
            .map(|v| v.show())
            .unwrap_or_default(),
    }
}

/// A dynamic LHS slot (`=context =x`) shows its bound slot name; a plain slot
/// is shown verbatim.
fn slot_disp(slot: &str, b: &Bindings) -> String {
    match slot.strip_prefix('=') {
        Some(var) => b
            .get(var)
            .map(|v| v.show())
            .unwrap_or_else(|| slot.to_string()),
        None => slot.to_string(),
    }
}

/// Instantiate a raw RHS `!output!`/`!action!` argument.
pub(super) fn show_sexp_arg(a: &Sexp, b: &Bindings) -> String {
    match a {
        Sexp::Sym(s) => {
            if let Some(var) = s.strip_prefix('=') {
                match b.get(var) {
                    Some(val) => val.show(),
                    None => s.clone(),
                }
            } else {
                s.clone()
            }
        }
        Sexp::Int(i) => i.to_string(),
        Sexp::Float(x) => Value::Num(*x).show(),
        Sexp::Str(s) => format!("{s:?}"),
        Sexp::List(items) => format!(
            "({})",
            items
                .iter()
                .map(|i| show_sexp_arg(i, b))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    }
}

pub(super) fn slot_op_str(op: SlotOp) -> &'static str {
    match op {
        SlotOp::Eq => "=",
        SlotOp::Neg => "-",
        SlotOp::Gt => ">",
        SlotOp::Lt => "<",
        SlotOp::Ge => ">=",
        SlotOp::Le => "<=",
    }
}

/// `print-production-text` with instantiations (`procedural.lisp:2441`):
/// the `(P NAME …)` block shown by `:cst`. `isa` is the chunk type, not a
/// slot, so it is not printed among the conditions.
pub(super) fn render_production(
    prod: &Production,
    b: &Bindings,
    partials: &[PpmMismatch],
) -> String {
    let mut s = format!("(P {}\n", prod.name);
    for item in &prod.lhs {
        match item {
            LhsItem::Match { buffer, tests } => {
                s.push_str(&format!("   ={buffer}>\n"));
                for t in tests {
                    if t.slot == "ISA" {
                        continue;
                    }
                    // A dynamic slot (`=context =x`) shows its bound name
                    // (`production-statement-text` substitutes the bindings).
                    let slot = slot_disp(&t.slot, b);
                    let val = show_pat(&t.value, b);
                    if t.op == SlotOp::Eq {
                        // `production-statement-text` annotates a partial
                        // match as `NAME [requested, actual, similarity]`.
                        let ann = partials.iter().find(|(pb, ps, req, _, _)| {
                            pb == buffer && ps == &t.slot && req.show() == val
                        });
                        if let Some((_, _, _, actual, sim)) = ann {
                            s.push_str(&format!(
                                "       {} [{}, {}, {}]\n",
                                slot,
                                val,
                                actual.show(),
                                lisp_s_f32(*sim)
                            ));
                        } else {
                            s.push_str(&format!("       {slot} {val}\n"));
                        }
                    } else {
                        s.push_str(&format!("    {:<2} {slot} {val}\n", slot_op_str(t.op)));
                    }
                }
            }
            LhsItem::Query { buffer, queries } => {
                s.push_str(&format!("   ?{buffer}>\n"));
                for (attr, val) in queries {
                    s.push_str(&format!("       {attr} {}\n", show_pat(val, b)));
                }
            }
            LhsItem::Eval { value, safe } => {
                s.push_str(&format!(
                    "   !{}! {}\n",
                    if *safe { "SAFE-EVAL" } else { "EVAL" },
                    show_sexp_arg(value, b)
                ));
            }
            LhsItem::Bind { var, value, safe } => {
                s.push_str(&format!(
                    "   !{}! ={var} {}\n",
                    if *safe { "SAFE-BIND" } else { "BIND" },
                    show_sexp_arg(value, b)
                ));
            }
            LhsItem::MvBind { vars, value } => {
                let vs: Vec<String> = vars.iter().map(|v| format!("={v}")).collect();
                s.push_str(&format!(
                    "   !MV-BIND! ({}) {}\n",
                    vs.join(" "),
                    show_sexp_arg(value, b)
                ));
            }
        }
    }
    s.push_str(" ==>\n");
    for a in &prod.rhs {
        match a {
            RhsAction::Modify { buffer, slots } => {
                s.push_str(&format!("   ={buffer}>\n"));
                for (name, val) in slots {
                    if name == "ISA" {
                        continue;
                    }
                    s.push_str(&format!(
                        "       {} {}\n",
                        slot_disp(name, b),
                        show_pat(val, b)
                    ));
                }
            }
            RhsAction::ModifyRequest { buffer, slots } => {
                s.push_str(&format!("   *{buffer}>\n"));
                for (name, val) in slots {
                    if name == "ISA" {
                        continue;
                    }
                    s.push_str(&format!(
                        "       {} {}\n",
                        slot_disp(name, b),
                        show_pat(val, b)
                    ));
                }
            }
            RhsAction::Request { buffer, spec } => match spec {
                ChunkDesc::Named(n) => s.push_str(&format!("   +{buffer}> {n}\n")),
                ChunkDesc::Spec {
                    slots, neg, rels, ..
                } => {
                    s.push_str(&format!("   +{buffer}>\n"));
                    for (name, val) in slots {
                        if name == "ISA" {
                            continue;
                        }
                        s.push_str(&format!(
                            "       {} {}\n",
                            slot_disp(name, b),
                            show_pat(val, b)
                        ));
                    }
                    for (name, val) in neg {
                        s.push_str(&format!("    -  {name} {}\n", show_pat(val, b)));
                    }
                    for (name, val, op) in rels {
                        s.push_str(&format!(
                            "    {:<2} {name} {}\n",
                            slot_op_str(*op),
                            show_pat(val, b)
                        ));
                    }
                }
            },
            RhsAction::Clear { buffer } => s.push_str(&format!("   -{buffer}>\n")),
            RhsAction::DirectSet { buffer, spec } => match spec {
                ChunkDesc::Named(n) => s.push_str(&format!("   @{buffer}> {n}\n")),
                ChunkDesc::Spec { slots, .. } => {
                    s.push_str(&format!("   @{buffer}>\n"));
                    for (name, val) in slots {
                        if name == "ISA" {
                            continue;
                        }
                        s.push_str(&format!(
                            "       {} {}\n",
                            slot_disp(name, b),
                            show_pat(val, b)
                        ));
                    }
                }
            },
            RhsAction::DirectErase { buffer } => s.push_str(&format!("   @{buffer}>\n")),
            RhsAction::Output { args } => {
                let body: Vec<String> = args.iter().map(|a| show_sexp_arg(a, b)).collect();
                s.push_str(&format!("   !OUTPUT! {}\n", body.join(" ")));
            }
            RhsAction::Other { head, args } => {
                let name = head.trim_matches('!');
                let body: Vec<String> = args.iter().map(|a| show_sexp_arg(a, b)).collect();
                s.push_str(&format!("   !{name}! {}\n", body.join(" ")));
            }
            RhsAction::Eval { value, safe } => {
                s.push_str(&format!(
                    "   !{}! {}\n",
                    if *safe { "SAFE-EVAL" } else { "EVAL" },
                    show_sexp_arg(value, b)
                ));
            }
            RhsAction::Bind { var, value, safe } => {
                s.push_str(&format!(
                    "   !{}! ={var} {}\n",
                    if *safe { "SAFE-BIND" } else { "BIND" },
                    show_sexp_arg(value, b)
                ));
            }
            RhsAction::MvBind { vars, value } => {
                let vs: Vec<String> = vars.iter().map(|v| format!("={v}")).collect();
                s.push_str(&format!(
                    "   !MV-BIND! ({}) {}\n",
                    vs.join(" "),
                    show_sexp_arg(value, b)
                ));
            }
        }
    }
    s.push(')');
    s
}

/// Failure latency uses the threshold: `round(lf * e^(-le*rt))`.
pub(super) fn failure_wait(cfg: &Config, rt: f32) -> i64 {
    if cfg.esc {
        latency(cfg.lf_ms, cfg.le, rt)
    } else {
        0
    }
}

/// `round(lf_ms * e^(-le*A))`, all single-float like the Lisp
/// (`compute-activation-latency`; `:lf` is stored as integer ms).
/// Rounds halves to even, as CL `round` does (units table ROUND rows).
pub(super) fn latency(lf_ms: i64, le: f32, act: f32) -> i64 {
    lisp_round((lf_ms as f32) * (-le * act).exp())
}

/// `bold_partial_gammapdf` (`other-files/bold.lisp:464`): `(t/b)^a · e^(−t/b)`,
/// 0 for a non-positive `t` (a demand that starts in the future).
pub(super) fn bold_pdf(t: f64, a: f64, b: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    let scale = t / b;
    scale.powf(a) * (-scale).exp()
}

/// `area-under-bold-subsegment` (`other-files/bold.lisp:475`): the trapezoidal
/// area under the gamma PDF over `[t1, t2]`.
pub(super) fn area_under_subsegment(time: f64, t1: f64, t2: f64, a: f64, b: f64) -> f64 {
    (bold_pdf(time - t1, a, b)
        + 2.0 * bold_pdf(time - (t1 + t2) / 2.0, a, b)
        + bold_pdf(time - t2, a, b))
        * (t2 - t1)
        * 0.25
}

/// `area-under-bold-curve` (`other-files/bold.lisp:482`): split `[t1, t2]`
/// into blocks no wider than `inc` and sum their trapezoids.
pub(super) fn area_under_curve(time: f64, t1: f64, t2: f64, inc: f64, a: f64, b: f64) -> f64 {
    let t2 = t2.min(time);
    if t1 == t2 {
        return 0.0;
    }
    let width = t2 - t1;
    let steps = (width / inc).ceil() as i64;
    let step = width / steps as f64;
    let mut sum = 0.0;
    let mut x = t2 - step;
    let mut y = t2;
    for _ in 1..steps {
        sum += area_under_subsegment(time, x, y, a, b);
        x -= step;
        y -= step;
    }
    sum + area_under_subsegment(time, t1, y, a, b)
}

/// `compute-bold-values`/`sum-bold-demands` (`other-files/bold.lisp:487`): the
/// summed demand for `time`, dropping demand intervals older than
/// `time - settle`.
pub(super) fn bold_values(
    demand: &[(f64, f64)],
    time: f64,
    inc: f64,
    settle: f64,
    a: f64,
    b: f64,
    point: bool,
) -> f64 {
    let mut sum = 0.0;
    for &(t1, t2) in demand {
        if t1 > time {
            break;
        }
        if t2 <= time - settle {
            continue;
        }
        sum += if point {
            bold_pdf(time - t1, a, b)
        } else {
            area_under_curve(time, t1, t2, inc, a, b)
        };
    }
    sum
}

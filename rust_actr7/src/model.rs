//! Loader: `define-model` files → [`Model`] AST.
//!
//! Handles the forms appearing in tutorial models: `clear-all`,
//! `define-model`, `sgp`, `chunk-type`, `add-dm`, `goal-focus`, and `p`
//! productions with `=buffer>` match / `?buffer?`... precisely:
//! `=x>` match patterns, `?x>` state queries, `+x>` requests,
//! `-x>` clears, `==>` separator, `- slot value` negation, `!output!`.
//! Anything outside that errors structurally — strict now, while the
//! executor's needs are still unknown, beats silent mis-parses.
//!
//! Symbol case: the reader folds to UPPERCASE, so `isa`/`ISA`,
//! `p`/`P` compare equal here.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::sexpr::{parse_all, Sexp};

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// `=foo` variable reference (stored without the `=`).
    Var(String),
    Sym(String),
    /// Integer literal. Kept distinct from [`Pattern::Num`] because Lisp
    /// prints `3` for an integer and `3.0` for a float (`~S`), and traces
    /// compare byte-for-byte (GOTCHAS #13).
    Int(i64),
    Num(f64),
    Str(String),
    Nil,
}

impl Pattern {
    fn from_sexp(s: &Sexp) -> Result<Pattern, LoadError> {
        match s {
            Sexp::Sym(v) if v == "NIL" => Ok(Pattern::Nil),
            Sexp::Sym(v) if v.starts_with('=') => Ok(Pattern::Var(v[1..].to_string())),
            Sexp::Sym(v) => Ok(Pattern::Sym(v.clone())),
            Sexp::Int(i) => Ok(Pattern::Int(*i)),
            Sexp::Float(x) => Ok(Pattern::Num(*x)),
            Sexp::Str(v) => Ok(Pattern::Str(v.clone())),
            Sexp::List(_) => Err(LoadError::Structural(format!(
                "list where atomic pattern expected: {s:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlotTest {
    pub slot: String,
    pub value: Pattern,
    pub op: SlotOp,
}

/// Slot test operator: `=`, `-`, or a numeric inequality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotOp {
    Eq,
    Neg,
    Gt,
    Lt,
    Ge,
    Le,
}

impl SlotOp {
    pub fn negated(self) -> bool {
        self == SlotOp::Neg
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LhsItem {
    /// `=goal> ISA count-from start =num1 ...`
    Match {
        buffer: String,
        tests: Vec<SlotTest>,
    },
    /// `?retrieval> buffer failure`
    Query {
        buffer: String,
        queries: Vec<(String, Pattern)>,
    },
    /// `!eval!`/`!safe-eval!` — a predicate evaluated against bindings.
    Eval { value: Sexp, safe: bool },
    /// `!bind!`/`!safe-bind!` — evaluate and bind one variable.
    Bind {
        var: String,
        value: Sexp,
        safe: bool,
    },
    /// `!mv-bind!` — evaluate to multiple values and bind several vars.
    MvBind { vars: Vec<String>, value: Sexp },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChunkDesc {
    /// `ISA <type> <slot value>...`, or bare pairs (empty type).
    Spec {
        ctype: String,
        slots: Vec<(String, Pattern)>,
        /// `- slot value` exclusions from a request spec (empty for
        /// construction contexts).
        neg: Vec<(String, Pattern)>,
        /// `> < >= <= slot value` numeric inequalities.
        rels: Vec<(String, Pattern, SlotOp)>,
    },
    /// A bare chunk name.
    Named(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RhsAction {
    /// `=goal> <slot value>...`
    Modify {
        buffer: String,
        slots: Vec<(String, Pattern)>,
    },
    /// `*goal> <slot value>...` — buffer modification request
    /// (`buffer-mod-request`); same change as `=`, but traced under the
    /// buffer's module and usable on untested buffers.
    ModifyRequest {
        buffer: String,
        slots: Vec<(String, Pattern)>,
    },
    /// `+retrieval> ISA number number =num2`
    Request { buffer: String, spec: ChunkDesc },
    /// `-goal>`
    Clear { buffer: String },
    /// `@goal> <chunk>` — a direct set (`overwrite-buffer-chunk`,
    /// `production-parsing.lisp:1147`): overwrite the buffer now, not a
    /// request.
    DirectSet { buffer: String, spec: ChunkDesc },
    /// `@goal>` (no chunk) — direct `erase-buffer`
    /// (`production-parsing.lisp:1140`).
    DirectErase { buffer: String },
    /// `!output! (...)` — args kept raw for the M2c evaluator.
    Output { args: Vec<Sexp> },
    /// `!eval!`/`!safe-eval!` on the RHS.
    Eval { value: Sexp, safe: bool },
    /// `!bind!`/`!safe-bind!` on the RHS.
    Bind {
        var: String,
        value: Sexp,
        safe: bool,
    },
    /// `!mv-bind!` on the RHS.
    MvBind { vars: Vec<String>, value: Sexp },
    /// Any other `!head! ...` — preserved, executor decides later.
    Other { head: String, args: Vec<Sexp> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Production {
    pub name: String,
    pub lhs: Vec<LhsItem>,
    pub rhs: Vec<RhsAction>,
    /// `(spp <name> :reward R)`: propagated as a reward when the production
    /// is selected (`commands/procedural-cmds.lisp`).
    pub reward: Option<f32>,
    /// `(spp <name> :at S)`: production action time override in seconds.
    pub at: Option<f32>,
    /// `(spp <name> :u U)`: initial utility override (else `:iu`).
    pub utility: Option<f32>,
}

/// Collect `add-act-r-command` name→function aliases from a task file
/// (`(add-act-r-command "cmd-name" 'fn ...)`) so scheduled commands resolve
/// to their `defun`.
pub fn parse_commands(src: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(forms) = parse_all(src) else {
        return map;
    };
    fn walk(f: &Sexp, map: &mut HashMap<String, String>) {
        let Sexp::List(items) = f else {
            return;
        };
        if items.first().and_then(|s| s.as_sym()) == Some("ADD-ACT-R-COMMAND") {
            let name = items.get(1).and_then(|s| match s {
                Sexp::Str(s) | Sexp::Sym(s) => Some(s.clone()),
                _ => None,
            });
            let fname = items.get(2).and_then(|s| match s {
                Sexp::Sym(s) => Some(s.clone()),
                Sexp::List(q) => match q.as_slice() {
                    [Sexp::Sym(q), Sexp::Sym(s)] if q == "QUOTE" => Some(s.clone()),
                    _ => None,
                },
                _ => None,
            });
            if let (Some(name), Some(fname)) = (name, fname) {
                map.insert(name.to_uppercase(), fname.to_uppercase());
            }
        }
        for x in items {
            walk(x, map);
        }
    }
    for f in &forms {
        walk(f, &mut map);
    }
    map
}

/// A task-file `defun` captured for the bounded evaluator (simple pure
/// helpers like `(defun single-bind (val) (1+ val))`).
#[derive(Debug, Clone, PartialEq)]
pub struct UserDefun {
    pub params: Vec<String>,
    pub body: Vec<Sexp>,
}

/// Collect `defun`s from a task file (other top-level forms are ignored).
/// Parse failures yield an empty map.
pub fn parse_defuns(src: &str) -> HashMap<String, UserDefun> {
    let mut map = HashMap::new();
    let Ok(forms) = parse_all(src) else {
        return map;
    };
    for f in forms {
        if let Sexp::List(items) = &f {
            if items.first().and_then(|s| s.as_sym()) != Some("DEFUN") {
                continue;
            }
            let Some(name) = items.get(1).and_then(|s| s.as_sym()) else {
                continue;
            };
            let Sexp::List(params) = &items[2] else {
                continue;
            };
            let params = params
                .iter()
                .filter_map(|p| p.as_sym().map(str::to_string))
                .collect();
            map.insert(
                name.to_uppercase(),
                UserDefun {
                    params,
                    body: items[3..].to_vec(),
                },
            );
        }
    }
    map
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkType {
    pub name: String,
    pub slots: Vec<String>,
    /// `(slot default)` declarations: slots with a default value applied
    /// when a chunk of this type is created.
    pub defaults: Vec<(String, Pattern)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub name: String,
    pub ctype: Option<String>,
    /// Bare `(shark)` entries have no type and no slots.
    pub slots: Vec<(String, Pattern)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub name: String,
    /// `(sgp k v ...)` pairs, values raw.
    pub params: Vec<(String, Sexp)>,
    pub chunk_types: Vec<ChunkType>,
    /// Declarative memory from `add-dm`.
    pub chunks: Vec<Chunk>,
    /// Named chunks from `define-chunks` that are NOT in declarative memory
    /// (values/references only; resolved for goal-focus and `=buffer>`
    /// copies). `add-dm` puts its chunks in `chunks` instead.
    pub defined_chunks: Vec<Chunk>,
    /// Model-body `(set-similarities (a b v) ...)`.
    pub similarities: Vec<(String, String, f32)>,
    /// Model-body `(set-base-levels (name level [creation-seconds]) ...)`.
    pub base_levels: Vec<(String, f32, Option<f64>)>,
    /// Model-body `(set-all-base-levels <level> [creation-seconds])`: a fixed
    /// base level for every DM chunk (`commands/dm-commands.lisp:600`).
    pub set_all_base_levels: Option<(f32, Option<f64>)>,
    /// Extras requested via top-level `(require-extra "name")`.
    pub required_extras: Vec<String>,
    pub initial_goal: Option<ChunkDesc>,
    /// All model-body `(goal-focus ...)` calls in order (the threads extra
    /// applies every one).
    pub goal_focuses: Vec<ChunkDesc>,
    pub productions: Vec<Production>,
    /// Model-body `(schedule-event <secs> "command" :params (...) ...)`:
    /// scheduled relative to model time 0 on each reset.
    pub scheduled_events: Vec<(i64, String, Vec<Sexp>)>,
    /// `(declare-buffer-usage <buffer> <chunk-type> <slots...>)`: the default
    /// chunk type per buffer, used by `set-buffer-chunk`.
    pub buffer_decls: Vec<(String, String)>,
    /// Model-body `(set-visloc-default slot value ...)`: default constraints
    /// for an unconstrained visual-location request (`vision.lisp:2107`).
    pub visloc_default: Vec<(String, Sexp)>,
    /// Model-body `(p-fct \`(... ,*var* ...))` raw production lists: ACT-R
    /// re-evaluates the model body on `reset` (`model.lisp:983`), so
    /// `,*query*` picks up the current `defparameter` value.
    pub pfct: Vec<Vec<Sexp>>,
    /// Model-body `(buffer-requires-copies '<buffer>)`: the named buffers do
    /// not reuse a single chunk (`buffers.lisp:1920`); each request makes a
    /// fresh chunk. Stored upper-cased.
    pub no_reuse_buffers: Vec<String>,
    /// Model-body `(pdisable name...)`: productions disabled on every reset
    /// (ACT-R re-evaluates the body, `procedural-cmds.lisp:541`). Upper-cased.
    pub disabled_prods: Vec<String>,
    /// Model-body `(set-buffer-chunk '<buffer> '<chunk>)`: initial buffer
    /// contents applied silently on every reset (the model body is
    /// re-evaluated). `(buffer, chunk-name)`.
    pub initial_buffer_sets: Vec<(String, String)>,
    /// Warning blocks from productions rejected by the validator. ACT-R
    /// re-evaluates the model body on `reset`, so these are re-emitted then
    /// (not at load, where the warning monitor isn't installed yet).
    pub reset_warnings: Vec<String>,
}

impl Model {
    /// Re-parse the `p-fct` productions with `query` substituted for
    /// `,*query*` (ACT-R re-evaluates the model body on `reset`).
    pub fn reparse_pfct(&mut self, query: &str) {
        for raw in &self.pfct {
            let mut list = raw.clone();
            for e in list.iter_mut() {
                if let Sexp::Sym(s) = e {
                    if s.trim_start_matches(',').eq_ignore_ascii_case("*QUERY*") {
                        *e = Sexp::Sym(query.to_uppercase());
                    }
                }
            }
            let mut form = vec![Sexp::Sym("P".to_string())];
            form.extend(list);
            if let Ok(p) = parse_production(&Sexp::List(form), &[]) {
                if let Some(existing) = self
                    .productions
                    .iter_mut()
                    .find(|x| x.name.eq_ignore_ascii_case(&p.name))
                {
                    *existing = p;
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Parse(crate::sexpr::ParseError),
    Structural(String),
    /// A production rejected by ACT-R's validator. The warning block is
    /// re-emitted on every model-body re-evaluation (`reset`), not at load
    /// (`model-tester.cl:351` installs its warning monitor after `load`).
    RejectedProduction(Vec<String>),
}

/// Lisp `~s`-ish printing of a read form (symbols are already upcased).
pub fn sexp_lisp(s: &Sexp) -> String {
    match s {
        Sexp::Sym(t) => t.clone(),
        Sexp::Int(i) => i.to_string(),
        Sexp::Float(x) => format!("{x}"),
        Sexp::Str(x) => format!("{x:?}"),
        Sexp::List(v) => format!(
            "({})",
            v.iter().map(sexp_lisp).collect::<Vec<_>>().join(" ")
        ),
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "io: {e}"),
            LoadError::Parse(e) => write!(f, "{e}"),
            LoadError::Structural(m) => write!(f, "structural: {m}"),
            LoadError::RejectedProduction(w) => write!(f, "rejected production: {}", w.join(" ")),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        LoadError::Io(e)
    }
}

impl From<crate::sexpr::ParseError> for LoadError {
    fn from(e: crate::sexpr::ParseError) -> Self {
        LoadError::Parse(e)
    }
}

fn structural<T>(msg: impl Into<String>) -> Result<T, LoadError> {
    Err(LoadError::Structural(msg.into()))
}

/// `=goal>` / `+retrieval>` / `?x>` / `-x>`, but not `==>` or `=var`.
fn buffer_ref(s: &str) -> Option<(char, &str)> {
    let mut c = s.chars();
    let op = c.next()?;
    if !"=+-?*@".contains(op) || !s.ends_with('>') || s == "==>" {
        return None;
    }
    let inner = &s[op.len_utf8()..s.len() - 1];
    if inner.is_empty() {
        return None;
    }
    Some((op, inner))
}

fn is_action_head(s: &Sexp) -> bool {
    match s {
        Sexp::Sym(t) => buffer_ref(t).is_some() || (t.starts_with('!') && t.ends_with('!')),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Production validation (`production-parsing.lisp` / `chunk-spec.lisp`).
//
// ACT-R never hard-errors on a malformed production: it warns and skips it
// (`create-production`, production-parsing.lisp:393). The warning block is
// replayed on every model-body re-evaluation, so the engine collects it at
// load and re-emits it from `Engine::reset`.
// ---------------------------------------------------------------------------

/// ACT-R `!cmd!` names (`command-symbol-p`, production-parsing.lisp:1879).
const BANG_COMMANDS: &[&str] = &[
    "EVAL",
    "SAFE-EVAL",
    "BIND",
    "SAFE-BIND",
    "MV-BIND",
    "STOP",
    "OUTPUT",
];

/// Buffers that exist in every model: the module defaults plus the ones the
/// extras define (`define-buffer`, and the tracker/threads extras).
fn is_known_buffer(name: &str, extra: &[String]) -> bool {
    matches!(
        name,
        "GOAL"
            | "RETRIEVAL"
            | "IMAGINAL"
            | "VISUAL"
            | "VISUAL-LOCATION"
            | "MANUAL"
            | "MANUAL-LEFT"
            | "MANUAL-RIGHT"
            | "VOCAL"
            | "AURAL"
            | "AURAL-LOCATION"
            | "TEMPORAL"
            | "BLENDING"
            | "TRACKER"
            | "IMAGINAL-ACTION"
            | "MOTOR"
            | "SPEECH"
            | "EMMA"
    ) || extra.iter().any(|b| b == name)
}

/// `command-symbol-p` (production-parsing.lisp:1877): a buffer head whose
/// buffer exists, or a `!cmd!` whose command is known.
fn is_command_symbol(s: &Sexp, extra: &[String]) -> bool {
    let Some(t) = s.as_sym() else { return false };
    let n = t.len();
    if n <= 2 {
        return false;
    }
    let first = t.as_bytes()[0] as char;
    let last = t.as_bytes()[n - 1] as char;
    if first == '!' && last == '!' {
        return BANG_COMMANDS.contains(&t[1..n - 1].to_uppercase().as_str());
    }
    if last == '>' && "=+-?*@".contains(first) {
        return is_known_buffer(&t[1..n - 1].to_uppercase(), extra);
    }
    false
}

/// `segment-production` (production-parsing.lisp:1865): split `forms` at each
/// command head. `Some(vec![])` for an empty body; `None` when the first form
/// is not a command (`:error`).
fn segment_production(forms: &[Sexp], extra: &[String]) -> Option<Vec<Vec<Sexp>>> {
    if forms.is_empty() {
        return Some(Vec::new());
    }
    let starts: Vec<usize> = forms
        .iter()
        .enumerate()
        .filter(|(_, s)| is_command_symbol(s, extra))
        .map(|(i, _)| i)
        .collect();
    if starts.first().copied() != Some(0) {
        return None;
    }
    let mut segs = Vec::new();
    for k in 0..starts.len() {
        let end = starts.get(k + 1).copied().unwrap_or(forms.len());
        segs.push(forms[starts[k]..end].to_vec());
    }
    Some(segs)
}

fn is_var_sym(t: &str) -> bool {
    t.len() > 1 && t.starts_with('=')
}

/// `chunk-spec-variable-p` (misc-utils.lisp:1480).
fn is_var(s: &Sexp) -> bool {
    s.as_sym().map(is_var_sym).unwrap_or(false)
}

/// `find-variables` (production-parsing.lisp:1831).
fn find_variables(s: &Sexp) -> Vec<String> {
    match s {
        Sexp::List(v) => {
            let mut out: Vec<String> = Vec::new();
            for e in v {
                for x in find_variables(e) {
                    if !out.contains(&x) {
                        out.push(x);
                    }
                }
            }
            out
        }
        Sexp::Sym(t) if is_var_sym(t) => vec![t.clone()],
        _ => Vec::new(),
    }
}

/// Render a segment the way `~s` prints it: `(=GOAL> ISA TEST ...)`.
fn seg_text(seg: &[Sexp]) -> String {
    format!(
        "({})",
        seg.iter().map(sexp_lisp).collect::<Vec<_>>().join(" ")
    )
}

/// `process-slots-specs` (chunk-spec.lisp:600): the warning that makes a
/// chunk-spec invalid, or `None` when it is well-formed.
fn chunk_spec_warning(specs: &[Sexp]) -> Option<String> {
    let mut i = 0;
    while i < specs.len() {
        let mut modifier = "=";
        if let Some(Sexp::Sym(t)) = specs.get(i) {
            if matches!(t.as_str(), "=" | "-" | ">" | "<" | ">=" | "<=") {
                modifier = t.as_str();
                i += 1;
            }
        }
        if i >= specs.len() {
            return Some(
                "Invalid specs in call to define-chunk-spec - not enough arguments".into(),
            );
        }
        match &specs[i] {
            Sexp::Sym(name) => {
                if name == "NIL" {
                    return Some("Cannot use nil as a slot name.".into());
                }
                if name.starts_with(':') && modifier != "=" {
                    return Some("Request parameters may not use a modifier other than =.".into());
                }
                let valid = is_var_sym(name)
                    || name.starts_with(':')
                    || name
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_alphanumeric())
                        .unwrap_or(false);
                if !valid {
                    return Some(format!(
                        "Invalid slot-name {} in call to define-chunk-spec.",
                        name
                    ));
                }
            }
            other => {
                return Some(format!(
                    "Invalid slot-name {} in call to define-chunk-spec.",
                    sexp_lisp(other)
                ));
            }
        }
        i += 1;
        if i >= specs.len() {
            return Some(
                "Invalid specs in call to define-chunk-spec - not enough arguments".into(),
            );
        }
        i += 1;
    }
    None
}

/// `define-chunk-spec-fct` (chunk-spec.lisp:434): strip a leading `isa TYPE`.
fn define_chunk_spec_warning(definition: &[Sexp]) -> Option<String> {
    match definition {
        [Sexp::Sym(isa), _, rest @ ..] if isa == "ISA" => chunk_spec_warning(rest),
        _ => chunk_spec_warning(definition),
    }
}

/// `parse-conditions` (production-parsing.lisp:1491): the rejection block, or
/// `None` when the LHS is well-formed.
fn validate_lhs(lhs: &[Sexp], extra: &[String]) -> Option<Vec<String>> {
    let segs = match segment_production(lhs, extra) {
        Some(s) => s,
        None => return Some(vec!["First item on LHS is not a valid command".into()]),
    };
    for seg in &segs {
        let head = seg[0].as_sym().unwrap_or_default();
        let op = head.chars().next().unwrap_or(' ');
        let definition = &seg[1..];
        match op {
            '=' => {
                if definition.len() == 1 {
                    return Some(vec![format!(
                        "Insufficient conditions specified in {}",
                        seg_text(seg)
                    )]);
                }
                if definition.is_empty() {
                    continue;
                }
                if let Some(w) = define_chunk_spec_warning(definition) {
                    return Some(vec![
                        format!("Invalid syntax in {} condition.", seg_text(seg)),
                        w,
                    ]);
                }
            }
            '?' => {
                if definition.len() < 2 {
                    return Some(vec![format!(
                        "Insufficient conditions specified in {}",
                        seg_text(seg)
                    )]);
                }
            }
            _ => {}
        }
    }
    None
}

/// `parse-actions` (production-parsing.lisp:1602): the rejection block, or
/// `None` when the RHS is well-formed.
fn validate_rhs(rhs: &[Sexp], lhs: &[Sexp], extra: &[String]) -> Option<Vec<String>> {
    let segs = match segment_production(rhs, extra) {
        Some(s) => s,
        None => return Some(vec!["First item on RHS is not a valid command".into()]),
    };
    // Buffers tested with `=` on the LHS (`parse-actions`, :1671).
    let tested: Vec<String> = segment_production(lhs, extra)
        .unwrap_or_default()
        .iter()
        .filter_map(|seg| {
            let h = seg[0].as_sym()?;
            if h.starts_with('=') {
                Some(h[1..h.len() - 1].to_string())
            } else {
                None
            }
        })
        .collect();
    for seg in &segs {
        let head = seg[0].as_sym().unwrap_or_default();
        let op = head.chars().next().unwrap_or(' ');
        let target = &head[1..head.len().saturating_sub(1)];
        let definition = &seg[1..];
        match op {
            '=' | '*' => {
                // A modification needs the buffer tested on the LHS.
                if !definition.is_empty() && !tested.iter().any(|t| t == target) {
                    return Some(vec![format!(
                        "Buffer modification action for untested buffer {}.",
                        seg_text(seg)
                    )]);
                }
                if definition.len() >= 2 {
                    if let Some(w) = define_chunk_spec_warning(definition) {
                        return Some(vec![
                            format!("Invalid buffer modification {}.", seg_text(seg)),
                            w,
                        ]);
                    }
                }
            }
            '+' => {
                if definition.is_empty() {
                    continue;
                }
                if definition.len() == 1 {
                    let chunk = &definition[0];
                    if is_var(chunk) {
                        continue;
                    }
                    if let Sexp::List(l) = chunk {
                        if l.len() > 1 {
                            if let Some(w) = chunk_spec_warning(&l[1..]) {
                                return Some(vec![
                                    format!("Invalid indirect request list: {}.  Must be a list with a chunk or variable and a valid request parameters settings.", seg_text(seg)),
                                    w,
                                ]);
                            }
                        }
                        continue;
                    }
                    // A bare chunk name: valid when it names a chunk, which
                    // the loader cannot check here, so accept.
                    continue;
                }
                if let Some(w) = define_chunk_spec_warning(definition) {
                    return Some(vec![
                        format!("Invalid buffer request {}.", seg_text(seg)),
                        w,
                    ]);
                }
            }
            _ => {}
        }
    }
    None
}

/// `circular-references` (misc-utils.lisp:1465) over the RHS bind
/// dependencies (production-parsing.lisp:661).
fn circular_rhs(rhs: &[Sexp], extra: &[String]) -> bool {
    let Some(segs) = segment_production(rhs, extra) else {
        return false;
    };
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for seg in &segs {
        let head = seg[0].as_sym().unwrap_or_default();
        if !head.starts_with('!') {
            continue;
        }
        let target = head[1..head.len() - 1].to_uppercase();
        let definition = &seg[1..];
        match target.as_str() {
            "BIND" | "SAFE-BIND" => {
                if let Some(v) = definition.first().and_then(|s| s.as_sym()) {
                    let d = definition.get(1).map(find_variables).unwrap_or_default();
                    adj.insert(v.to_string(), d);
                }
            }
            "MV-BIND" => {
                if let Some(Sexp::List(bv)) = definition.first() {
                    let d = definition.get(1).map(find_variables).unwrap_or_default();
                    for v in bv {
                        if let Some(n) = v.as_sym() {
                            adj.insert(n.to_string(), d.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    fn dfs(node: &str, adj: &HashMap<String, Vec<String>>, visiting: &mut Vec<String>) -> bool {
        if visiting.iter().any(|x| x == node) {
            return true;
        }
        visiting.push(node.to_string());
        if let Some(ns) = adj.get(node) {
            for n in ns {
                if dfs(n, adj, visiting) {
                    return true;
                }
            }
        }
        visiting.pop();
        false
    }
    let nodes: Vec<String> = adj.keys().cloned().collect();
    nodes.iter().any(|n| dfs(n, &adj, &mut Vec::new()))
}

/// Build the rejection block (`bad-production-exit`, production-parsing.lisp:397).
fn reject_production(prod_text: &str, name: &str, msgs: &[String]) -> LoadError {
    let mut block = vec![format!("No production defined for {prod_text}.")];
    block.extend(msgs.iter().cloned());
    block.push(format!(
        "--- end of warnings for undefined production {name} ---"
    ));
    LoadError::RejectedProduction(block)
}

/// The value forms of a `slot value` list (skipping modifiers and a leading
/// `isa TYPE`).
fn slot_values(forms: &[Sexp]) -> Vec<&Sexp> {
    let mut i = 0;
    if matches!(forms.first(), Some(Sexp::Sym(s)) if s == "ISA") {
        i = 2;
    }
    let mut out = Vec::new();
    while i < forms.len() {
        if let Some(Sexp::Sym(t)) = forms.get(i) {
            if matches!(t.as_str(), "=" | "-" | ">" | "<" | ">=" | "<=") {
                i += 1;
            }
        }
        if i + 1 < forms.len() {
            out.push(&forms[i + 1]);
        }
        i += 2;
    }
    out
}

/// A production slot value that names no chunk (`create-undefined-chunk`,
/// chunks.lisp:850). `nil` is skipped — the ref shows no `NIL` warning even
/// though `eq t` is the only literal guard (production-parsing.lisp:1380).
fn is_undefined_chunk_value(t: &str, known: &[String]) -> bool {
    if t == "NIL" || t == "T" {
        return false;
    }
    if t.starts_with(':') || t.starts_with('=') {
        return false;
    }
    if t.chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        return false;
    }
    if MODULE_DEFAULT_CHUNKS.contains(&t) {
        return false;
    }
    !known.iter().any(|k| k == t)
}

/// Chunks every model starts with (`create-model-default-chunks`,
/// model.lisp:755): the module-defined command/colour/state names, plus the
/// touch/motor action names the extras add. Verified against SBCL — a
/// production value naming one of these does not warn.
const MODULE_DEFAULT_CHUNKS: &[&str] = &[
    "ASSIGN-FINST",
    "BLACK",
    "BLUE",
    "BOX",
    "BROWN",
    "BUSY",
    "CHAR-PRIMITIVE",
    "CLEAR",
    "CLEAR-ALL-FINSTS",
    "CLEAR-SCENE-CHANGE",
    "CLICK-MOUSE",
    "CLOCKWISE",
    "COUNTERCLOCKWISE",
    "CURRENT",
    "CURRENT-DISTANCE",
    "CURRENT-X",
    "CURRENT-Y",
    "CYAN",
    "DARK-BLUE",
    "DARK-CYAN",
    "DARK-GRAY",
    "DARK-GREEN",
    "DARK-MAGENTA",
    "DARK-RED",
    "DARK-YELLOW",
    "DIGIT",
    "EMPTY",
    "ERROR",
    "EXECUTE",
    "EXTERNAL",
    "FAILURE",
    "FIND-LOCATION",
    "FIND-SOUND",
    "FREE",
    "FULL",
    "GRAY",
    "GREEN",
    "HAND-TO-HOME",
    "HAND-TO-JOYSTICK1",
    "HAND-TO-JOYSTICK2",
    "HAND-TO-KEYPAD",
    "HAND-TO-MOUSE",
    "HAND-TO-TOUCH",
    "HIGH",
    "HIGHEST",
    "HIT-KEY",
    "HOLD-KEY",
    "HOLD-MOUSE",
    "HOLD-PECK",
    "HOLD-PUNCH",
    "INDEX",
    "INDEX-THUMB",
    "INTERNAL",
    "LEFT",
    "LIGHT-BLUE",
    "LIGHT-GRAY",
    "LINE",
    "LOW",
    "LOWEST",
    "MAGENTA",
    "MIDDLE",
    "MOVE-ATTENTION",
    "MOVE-CURSOR",
    "MOVE-FINGER-TO-HOME",
    "MOVE-HAND-TOUCH",
    "MOVE-TO-KEY",
    "NEW",
    "OVAL",
    "PECK",
    "PECK-RECOIL",
    "PINCH",
    "PINK",
    "PINKIE",
    "POINT-FINGER",
    "POINT-HAND-AT-KEY",
    "PREPARE",
    "PRESS-KEY",
    "PUNCH",
    "PURPLE",
    "RED",
    "RELEASE",
    "RELEASE-ALL-FINGERS",
    "RELEASE-KEY",
    "RELEASE-KEY-TO-HOME",
    "RELEASE-MOUSE",
    "RELEASE-RECOIL",
    "REQUESTED",
    "RIGHT",
    "RING",
    "ROTATE",
    "SELF",
    "SET-AUDLOC-DEFAULT",
    "SET-VISLOC-DEFAULT",
    "SOUND",
    "SPEAK",
    "SPEECH",
    "START-TRACKING",
    "SUBVOCALIZE",
    "SWIPE",
    "TAP",
    "TAP-HOLD",
    "TAP-RELEASE",
    "TEXT",
    "THUMB",
    "TONE",
    "TYPE-KEY",
    "UNREQUESTED",
    "WHITE",
    "WORD",
];

/// The `create-undefined-chunk` warnings for one production's scans
/// (production-parsing.lisp:1369/1389). `known` accumulates the created names.
fn undefined_chunk_warnings(
    lhs: &[Sexp],
    rhs: &[Sexp],
    known: &mut Vec<String>,
    extra: &[String],
) -> Vec<String> {
    let mut out = Vec::new();
    let mut check = |v: &Sexp, known: &mut Vec<String>| {
        if let Sexp::Sym(t) = v {
            if is_undefined_chunk_value(t, known) {
                out.push(format!("Creating chunk {} with no slots", t));
                known.push(t.clone());
            }
        }
    };
    if let Some(segs) = segment_production(lhs, extra) {
        for seg in &segs {
            let head = seg[0].as_sym().unwrap_or_default();
            let op = head.chars().next().unwrap_or(' ');
            if op != '=' && op != '?' {
                continue;
            }
            for v in slot_values(&seg[1..]) {
                check(v, known);
            }
        }
    }
    if let Some(segs) = segment_production(rhs, extra) {
        for seg in &segs {
            let head = seg[0].as_sym().unwrap_or_default();
            let op = head.chars().next().unwrap_or(' ');
            let definition = &seg[1..];
            match op {
                '=' | '@' => {
                    for v in slot_values(definition) {
                        check(v, known);
                    }
                }
                '+' | '*' => {
                    if definition.len() == 1 {
                        match &definition[0] {
                            Sexp::Sym(v) => check(&Sexp::Sym(v.clone()), known),
                            Sexp::List(l) => {
                                if let Some(v) = l.first().and_then(|s| s.as_sym()) {
                                    check(&Sexp::Sym(v.to_string()), known);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// `sort-for-bind-order` (production-parsing.lisp:1810): the reorder warning
/// block when the RHS `!bind!`/`!eval!`/`!output!` order changes.
fn reorder_warning(name: &str, rhs: &[Sexp], extra: &[String]) -> Vec<String> {
    let Some(segs) = segment_production(rhs, extra) else {
        return Vec::new();
    };
    let mut items: Vec<(Vec<String>, Vec<String>)> = Vec::new();
    for seg in &segs {
        let head = seg[0].as_sym().unwrap_or_default();
        if !head.starts_with('!') {
            continue;
        }
        let target = head[1..head.len() - 1].to_uppercase();
        let definition = &seg[1..];
        match target.as_str() {
            "BIND" | "SAFE-BIND" => {
                let v = definition
                    .first()
                    .and_then(|s| s.as_sym())
                    .map(str::to_string);
                let d = definition.get(1).map(find_variables).unwrap_or_default();
                items.push((v.into_iter().collect(), d));
            }
            "MV-BIND" => {
                let names: Vec<String> = match definition.first() {
                    Some(Sexp::List(bv)) => bv
                        .iter()
                        .filter_map(|s| s.as_sym().map(str::to_string))
                        .collect(),
                    _ => Vec::new(),
                };
                let d = definition.get(1).map(find_variables).unwrap_or_default();
                items.push((names, d));
            }
            "EVAL" | "SAFE-EVAL" | "OUTPUT" => {
                let d = definition.first().map(find_variables).unwrap_or_default();
                items.push((Vec::new(), d));
            }
            _ => {}
        }
    }
    let original = items.clone();
    let mut result: Vec<(Vec<String>, Vec<String>)> = Vec::new();
    for x in items {
        let pos = result.iter().position(|y| {
            if x.0.len() == 1 {
                y.1.iter().any(|d| d == &x.0[0])
            } else if x.0.len() > 1 {
                x.0.iter().any(|v| y.1.contains(v))
            } else {
                false
            }
        });
        match pos {
            Some(i) => result.insert(i, x),
            None => result.push(x),
        }
    }
    if result != original {
        vec![
            format!(
                "RHS !bind!, !eval!, and/or !output! actions of production {name} had to be reordered because of interdependencies of variables."
            ),
            "If those actions have side effects check the actions carefully to ensure proper operation.".to_string(),
        ]
    } else {
        Vec::new()
    }
}

/// Parse `slot value` pairs with optional `-`/`>`/`<`/`>=`/`<=` operators.
fn parse_pairs(forms: &[Sexp]) -> Result<Vec<SlotTest>, LoadError> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < forms.len() {
        let op = match &forms[i] {
            Sexp::Sym(t) if t == "-" => SlotOp::Neg,
            Sexp::Sym(t) if t == ">" => SlotOp::Gt,
            Sexp::Sym(t) if t == "<" => SlotOp::Lt,
            Sexp::Sym(t) if t == ">=" => SlotOp::Ge,
            Sexp::Sym(t) if t == "<=" => SlotOp::Le,
            _ => SlotOp::Eq,
        };
        if op != SlotOp::Eq {
            i += 1;
        }
        let slot = match forms.get(i) {
            Some(Sexp::Sym(t)) => t.clone(),
            got => return structural(format!("slot name expected, got {got:?}")),
        };
        let value = match forms.get(i + 1) {
            Some(v) => Pattern::from_sexp(v)?,
            None => return structural(format!("value missing for slot {slot}")),
        };
        out.push(SlotTest { slot, value, op });
        i += 2;
    }
    Ok(out)
}

fn parse_pairs_plain(forms: &[Sexp]) -> Result<Vec<(String, Pattern)>, LoadError> {
    let mut out = Vec::new();
    for t in parse_pairs(forms)? {
        if t.op != SlotOp::Eq {
            return structural(format!(
                "operator on slot {} only valid in a request/test",
                t.slot
            ));
        }
        out.push((t.slot, t.value));
    }
    Ok(out)
}

/// `ISA <type> pairs...`, bare pairs (empty type), or a lone chunk name.
pub fn parse_chunk_desc(forms: &[Sexp]) -> Result<ChunkDesc, LoadError> {
    if forms.len() == 1 {
        if let Some(name) = forms[0].as_sym() {
            if name != "ISA" {
                return Ok(ChunkDesc::Named(name.to_string()));
            }
        }
        // A single list `(head slot value ...)` whose head is not `isa`: a
        // chunk-spec naming a chunk (or chunk-type) in the head
        // (`define-chunk-spec-fct`, chunk-spec.lisp:434). Request parameters
        // like `:mp-value` ride along as `:`-slots (skipped by matching).
        if let Sexp::List(items) = &forms[0] {
            if let Some(head) = items.first().and_then(|x| x.as_sym()) {
                if head != "ISA" {
                    let mut slots = Vec::new();
                    let mut neg = Vec::new();
                    let mut rels = Vec::new();
                    for t in parse_pairs(&items[1..])? {
                        match t.op {
                            SlotOp::Eq => slots.push((t.slot, t.value)),
                            SlotOp::Neg => neg.push((t.slot, t.value)),
                            other => rels.push((t.slot, t.value, other)),
                        }
                    }
                    return Ok(ChunkDesc::Spec {
                        ctype: head.to_string(),
                        slots,
                        neg,
                        rels,
                    });
                }
            }
        }
    }
    let (ctype, rest) = match forms {
        [Sexp::Sym(isa), Sexp::Sym(t), rest @ ..] if isa == "ISA" => (t.clone(), rest),
        _ => (String::new(), forms),
    };
    // Operators are separated out: `-` exclusions and numeric inequalities
    // are applied during candidacy/scoring; `=` slots are exact/score tests.
    let mut slots = Vec::new();
    let mut neg = Vec::new();
    let mut rels = Vec::new();
    for t in parse_pairs(rest)? {
        match t.op {
            SlotOp::Eq => slots.push((t.slot, t.value)),
            SlotOp::Neg => neg.push((t.slot, t.value)),
            other => rels.push((t.slot, t.value, other)),
        }
    }
    Ok(ChunkDesc::Spec {
        ctype,
        slots,
        neg,
        rels,
    })
}

fn parse_lhs(forms: &[Sexp]) -> Result<Vec<LhsItem>, LoadError> {
    let mut items = Vec::new();
    let mut i = 0;
    while i < forms.len() {
        let head = match &forms[i] {
            Sexp::Sym(t) => t.clone(),
            got => return structural(format!("LHS: buffer ref expected, got {got:?}")),
        };
        i += 1;
        let mut j = i;
        while j < forms.len() && !is_action_head(&forms[j]) {
            j += 1;
        }
        let body = &forms[i..j];
        if let Some((op, buffer)) = buffer_ref(&head) {
            match op {
                '=' | '+' => {
                    // `+` on the LHS never occurs in M0 models; accept like `=`
                    // so the loader stays total over match-shaped groups.
                    let tests = parse_pairs(body)?;
                    items.push(LhsItem::Match {
                        buffer: buffer.to_string(),
                        tests,
                    });
                }
                '?' => {
                    let qs = parse_pairs(body)?;
                    for q in &qs {
                        if q.op != SlotOp::Eq {
                            return structural("`-`/inequality not allowed in `?` queries");
                        }
                    }
                    items.push(LhsItem::Query {
                        buffer: buffer.to_string(),
                        queries: qs.into_iter().map(|q| (q.slot, q.value)).collect(),
                    });
                }
                _ => return structural(format!("LHS: unexpected `{op}` buffer ref")),
            }
        } else if head.starts_with('!') && head.ends_with('!') {
            parse_lhs_bang(&head, body, &mut items)?;
        } else {
            return structural(format!("LHS: buffer ref expected, got {head}"));
        }
        i = j;
    }
    Ok(items)
}

/// Split a `!name!` head into its (lower-case) name and whether it is the
/// `safe-` variant (`!safe-eval!` / `!safe-bind!`).
fn bang_head(head: &str) -> (String, bool) {
    let inner = head.trim_matches('!').to_lowercase();
    match inner.strip_prefix("safe-") {
        Some(rest) => (rest.to_string(), true),
        None => (inner, false),
    }
}

fn parse_lhs_bang(head: &str, body: &[Sexp], items: &mut Vec<LhsItem>) -> Result<(), LoadError> {
    let (name, safe) = bang_head(head);
    match name.as_str() {
        "eval" => {
            let value = body
                .first()
                .cloned()
                .ok_or_else(|| LoadError::Structural("!eval! needs an expression".into()))?;
            items.push(LhsItem::Eval { value, safe });
        }
        "bind" => {
            let var = body
                .first()
                .and_then(|s| s.as_sym())
                .and_then(|v| v.strip_prefix('='))
                .ok_or_else(|| LoadError::Structural("!bind! needs a =variable".into()))?
                .to_string();
            let value = body
                .get(1)
                .cloned()
                .ok_or_else(|| LoadError::Structural("!bind! needs an expression".into()))?;
            items.push(LhsItem::Bind { var, value, safe });
        }
        "mv-bind" => {
            let vars = match body.first() {
                Some(Sexp::List(v)) => v
                    .iter()
                    .filter_map(|s| {
                        s.as_sym()
                            .and_then(|x| x.strip_prefix('='))
                            .map(str::to_string)
                    })
                    .collect(),
                _ => return structural("!mv-bind! needs a (=a =b) list"),
            };
            let value = body
                .get(1)
                .cloned()
                .ok_or_else(|| LoadError::Structural("!mv-bind! needs an expression".into()))?;
            items.push(LhsItem::MvBind { vars, value });
        }
        other => return structural(format!("LHS: unsupported !{other}! condition")),
    }
    Ok(())
}

fn parse_rhs(forms: &[Sexp]) -> Result<Vec<RhsAction>, LoadError> {
    let mut actions = Vec::new();
    let mut i = 0;
    while i < forms.len() {
        let head = match &forms[i] {
            Sexp::Sym(t) => t.clone(),
            got => return structural(format!("RHS: action head expected, got {got:?}")),
        };
        i += 1;
        let mut j = i;
        while j < forms.len() && !is_action_head(&forms[j]) {
            j += 1;
        }
        let body = &forms[i..j];
        if let Some((op, buffer)) = buffer_ref(&head) {
            let buffer = buffer.to_string();
            match op {
                '=' => actions.push(RhsAction::Modify {
                    buffer,
                    slots: parse_pairs_plain(body)?,
                }),
                '*' => actions.push(RhsAction::ModifyRequest {
                    buffer,
                    slots: parse_pairs_plain(body)?,
                }),
                '+' => actions.push(RhsAction::Request {
                    buffer,
                    spec: parse_chunk_desc(body)?,
                }),
                '-' => {
                    if !body.is_empty() {
                        return structural(format!("clear `-{buffer}>` takes no args"));
                    }
                    actions.push(RhsAction::Clear { buffer });
                }
                '@' => {
                    if body.is_empty() {
                        actions.push(RhsAction::DirectErase { buffer });
                    } else {
                        actions.push(RhsAction::DirectSet {
                            buffer,
                            spec: parse_chunk_desc(body)?,
                        });
                    }
                }
                _ => return structural(format!("RHS: unexpected `{op}` buffer ref")),
            }
        } else if head.starts_with('!') && head.ends_with('!') {
            let (name, safe) = bang_head(&head);
            match name.as_str() {
                "output" => actions.push(RhsAction::Output {
                    args: body.to_vec(),
                }),
                "eval" => {
                    let value = body.first().cloned().ok_or_else(|| {
                        LoadError::Structural("!eval! needs an expression".into())
                    })?;
                    actions.push(RhsAction::Eval { value, safe });
                }
                "bind" => {
                    let var = body
                        .first()
                        .and_then(|s| s.as_sym())
                        .and_then(|v| v.strip_prefix('='))
                        .ok_or_else(|| LoadError::Structural("!bind! needs a =variable".into()))?
                        .to_string();
                    let value = body.get(1).cloned().ok_or_else(|| {
                        LoadError::Structural("!bind! needs an expression".into())
                    })?;
                    actions.push(RhsAction::Bind { var, value, safe });
                }
                "mv-bind" => {
                    let vars = match body.first() {
                        Some(Sexp::List(v)) => v
                            .iter()
                            .filter_map(|s| {
                                s.as_sym()
                                    .and_then(|x| x.strip_prefix('='))
                                    .map(str::to_string)
                            })
                            .collect(),
                        _ => return structural("!mv-bind! needs a (=a =b) list"),
                    };
                    let value = body.get(1).cloned().ok_or_else(|| {
                        LoadError::Structural("!mv-bind! needs an expression".into())
                    })?;
                    actions.push(RhsAction::MvBind { vars, value });
                }
                _ => actions.push(RhsAction::Other {
                    head,
                    args: body.to_vec(),
                }),
            }
        } else {
            return structural(format!("RHS: action head expected, got {head}"));
        }
        i = j;
    }
    Ok(actions)
}

/// Parse a production from text (used for runtime-compiled productions).
pub fn parse_production_text(text: &str) -> Result<Production, LoadError> {
    let forms = crate::sexpr::parse_all(text).map_err(|e| LoadError::Structural(e.to_string()))?;
    match forms.first() {
        Some(f) => parse_production(f, &[]),
        None => structural("empty production text"),
    }
}

fn parse_production(form: &Sexp, extra_buffers: &[String]) -> Result<Production, LoadError> {
    let items = match form {
        Sexp::List(items) => items,
        _ => return structural("production must be a list"),
    };
    let (head, name, body) = match items.as_slice() {
        [Sexp::Sym(h), Sexp::Sym(n), rest @ ..] if h == "P" => (h, n, rest),
        got => return structural(format!("`(p name ...)` expected, got {got:?}")),
    };
    let _ = head;
    let prod_text = format!(
        "({})",
        items[1..]
            .iter()
            .map(sexp_lisp)
            .collect::<Vec<_>>()
            .join(" ")
    );
    let Some(split) = body
        .iter()
        .position(|s| matches!(s, Sexp::Sym(t) if t == "==>"))
    else {
        return Err(reject_production(
            &prod_text,
            name,
            &["Production is missing the ==> separator.".to_string()],
        ));
    };
    let lhs_forms = &body[..split];
    let rhs_forms = &body[split + 1..];
    if let Some(w) = validate_lhs(lhs_forms, extra_buffers) {
        return Err(reject_production(&prod_text, name, &w));
    }
    if let Some(w) = validate_rhs(rhs_forms, lhs_forms, extra_buffers) {
        return Err(reject_production(&prod_text, name, &w));
    }
    let lhs = parse_lhs(lhs_forms)?;
    let rhs = parse_rhs(rhs_forms)?;
    if circular_rhs(rhs_forms, extra_buffers) {
        return Err(reject_production(
            &prod_text,
            name,
            &["Because there are circular variable bindings on the RHS.".to_string()],
        ));
    }
    Ok(Production {
        name: name.clone(),
        lhs,
        rhs,
        reward: None,
        at: None,
        utility: None,
    })
}

fn parse_chunk_type(form: &Sexp) -> Result<ChunkType, LoadError> {
    match form {
        Sexp::List(items) => match items.as_slice() {
            [Sexp::Sym(h), rest @ ..] if h == "CHUNK-TYPE" => parse_chunk_type_spec(rest),
            got => structural(format!("bad chunk-type form: {got:?}")),
        },
        got => structural(format!("bad chunk-type form: {got:?}")),
    }
}

/// Parse a chunk type's `name-and-slots` list (`chunk-type-fct`,
/// `framework/chunk-types.lisp:340`): `name` or `(name (:include parent)*)`,
/// an optional documentation string, then `slot` or `(slot default)` entries.
/// Also the RPC `chunk-type` command's payload. `:include` parents are
/// accepted and ignored: the engine only needs a type's own slots/defaults,
/// and the module types the parents name (`visual-object`, `audio-event`, …)
/// are not in the model.
pub fn parse_chunk_type_spec(items: &[Sexp]) -> Result<ChunkType, LoadError> {
    let mut it = items.iter();
    let name_desc = match it.next() {
        Some(x) => x,
        None => return structural("chunk-type with no name".to_string()),
    };
    let name = match name_desc {
        Sexp::Sym(n) => n.clone(),
        // `(name (:include parent) …)`: the modifiers are the rest.
        Sexp::List(parts) => match parts.first() {
            Some(Sexp::Sym(n)) => n.clone(),
            got => return structural(format!("bad chunk-type name: {got:?}")),
        },
        got => return structural(format!("bad chunk-type name: {got:?}")),
    };
    let mut rest: Vec<&Sexp> = it.collect();
    // An optional documentation string follows the name.
    if matches!(rest.first(), Some(Sexp::Str(_))) {
        rest.remove(0);
    }
    let mut out = Vec::new();
    let mut defaults = Vec::new();
    for s in rest {
        match s {
            Sexp::Sym(t) => out.push(t.clone()),
            // `(slot default)` — slot with a default value.
            Sexp::List(pair) => match pair.as_slice() {
                [Sexp::Sym(t), default] => {
                    out.push(t.clone());
                    defaults.push((t.clone(), Pattern::from_sexp(default)?));
                }
                got => return structural(format!("bad chunk-type slot: {got:?}")),
            },
            got => return structural(format!("chunk-type slot must be a symbol: {got:?}")),
        }
    }
    Ok(ChunkType {
        name,
        slots: out,
        defaults,
    })
}

/// Numeric S-expression (`Int` or `Float`) as f64.
fn sexp_f64(e: &Sexp) -> Option<f64> {
    match e {
        Sexp::Int(i) => Some(*i as f64),
        Sexp::Float(x) => Some(*x),
        _ => None,
    }
}

pub fn parse_dm_entry(form: &Sexp) -> Result<Chunk, LoadError> {
    match form {
        Sexp::List(items) => match items.as_slice() {
            [Sexp::Sym(name)] => Ok(Chunk {
                name: name.clone(),
                ctype: None,
                slots: Vec::new(),
            }),
            [Sexp::Sym(name), Sexp::Sym(isa), Sexp::Sym(t), rest @ ..] if isa == "ISA" => {
                Ok(Chunk {
                    name: name.clone(),
                    ctype: Some(t.clone()),
                    slots: parse_pairs_plain(rest)?,
                })
            }
            // Named chunk without `isa` (`(g value 3 color red)`): an odd
            // number of forms is a name followed by slot/value pairs.
            [Sexp::Sym(name), rest @ ..] if !rest.is_empty() && rest.len() % 2 == 0 => Ok(Chunk {
                name: name.clone(),
                ctype: None,
                slots: parse_pairs_plain(rest)?,
            }),
            got => structural(format!("bad add-dm entry: {got:?}")),
        },
        got => structural(format!("bad add-dm entry: {got:?}")),
    }
}

/// Define one `define-chunks` entry in the model's chunk table and return
/// the chunk name. Anonymous `(isa TYPE slots...)` entries are named
/// `TYPE{index}` (`TYPE0`, `TYPE1`, ...) like ACT-R.
pub fn define_chunk_table_entry(e: &Sexp, model: &mut Model) -> Result<String, LoadError> {
    match e {
        Sexp::Sym(n) => {
            model.defined_chunks.push(Chunk {
                name: n.clone(),
                ctype: None,
                slots: Vec::new(),
            });
            Ok(n.clone())
        }
        Sexp::List(p) => {
            if matches!(p.as_slice(), [Sexp::Sym(isa), Sexp::Sym(_), ..] if isa == "ISA") {
                let ctype = p[1].as_sym().unwrap_or_default().to_string();
                let slots = parse_pairs_plain(&p[2..])?;
                let idx = model
                    .defined_chunks
                    .iter()
                    .filter(|c| c.ctype.as_deref() == Some(ctype.as_str()))
                    .count();
                let name = format!("{ctype}{idx}");
                model.defined_chunks.push(Chunk {
                    name: name.clone(),
                    ctype: Some(ctype),
                    slots,
                });
                Ok(name)
            } else {
                let c = parse_dm_entry(e)?;
                let name = c.name.clone();
                model.defined_chunks.push(c);
                Ok(name)
            }
        }
        got => structural(format!("bad define-chunks entry: {got:?}")),
    }
}

/// Recognize `(car (define-chunks DEF...))` (the `goal-focus-fct` idiom):
/// define the chunks and return the first name. `Some(None)` means the form
/// matched but no chunk could be defined.
fn car_define_chunks(arg: &Sexp, model: &mut Model) -> Result<Option<Option<String>>, LoadError> {
    let items = match arg {
        Sexp::List(v) => v,
        _ => return Ok(None),
    };
    if items.len() != 2 || items[0].as_sym() != Some("CAR") {
        return Ok(None);
    }
    let dc = match &items[1] {
        Sexp::List(v) => v,
        _ => return Ok(None),
    };
    if dc.first().and_then(|s| s.as_sym()) != Some("DEFINE-CHUNKS") {
        return Ok(None);
    }
    let mut first = None;
    for e in &dc[1..] {
        if let Ok(name) = define_chunk_table_entry(e, model) {
            if first.is_none() {
                first = Some(name);
            }
        }
    }
    Ok(Some(first))
}

/// Substitute backquote-comma symbols (`,*VAR*`) with model-file
/// `defparameter` values (unwrapping a leading `QUOTE`).
fn substitute_commas(list: &[Sexp], defs: &std::collections::HashMap<String, Sexp>) -> Vec<Sexp> {
    list.iter().map(|e| subst_commas(e, defs)).collect()
}

fn subst_commas(e: &Sexp, defs: &std::collections::HashMap<String, Sexp>) -> Sexp {
    match e {
        Sexp::Sym(s) if s.starts_with(',') => {
            let name = s.trim_start_matches(',').to_uppercase();
            match defs.get(&name) {
                Some(Sexp::List(v)) if v.first().and_then(|x| x.as_sym()) == Some("QUOTE") => {
                    v.get(1).cloned().unwrap_or(Sexp::Sym("NIL".to_string()))
                }
                Some(v) => v.clone(),
                None => Sexp::Sym("NIL".to_string()),
            }
        }
        Sexp::List(v) => Sexp::List(v.iter().map(|x| subst_commas(x, defs)).collect()),
        other => other.clone(),
    }
}

/// Substitute a bare symbol that names a model-file `defparameter`
/// (`sgp-fct`'s `(list :k *var* ...)`).
fn subst_bare(e: &Sexp, defs: &std::collections::HashMap<String, Sexp>) -> Sexp {
    match e {
        Sexp::Sym(s) => match defs.get(&s.to_uppercase()) {
            Some(Sexp::List(v)) if v.first().and_then(|x| x.as_sym()) == Some("QUOTE") => {
                v.get(1).cloned().unwrap_or(Sexp::Sym("NIL".to_string()))
            }
            Some(v) => v.clone(),
            None => e.clone(),
        },
        other => other.clone(),
    }
}

fn parse_model(
    form: &Sexp,
    defs: &std::collections::HashMap<String, Sexp>,
) -> Result<Model, LoadError> {
    let body = match form {
        Sexp::List(items) => match items.as_slice() {
            [Sexp::Sym(h), Sexp::Sym(name), rest @ ..] if h == "DEFINE-MODEL" => {
                (name.clone(), rest)
            }
            got => return structural(format!("define-model expected, got {got:?}")),
        },
        got => return structural(format!("define-model expected, got {got:?}")),
    };
    let (name, forms) = body;
    let mut model = Model {
        name,
        params: Vec::new(),
        chunk_types: Vec::new(),
        chunks: Vec::new(),
        defined_chunks: Vec::new(),
        similarities: Vec::new(),
        base_levels: Vec::new(),
        set_all_base_levels: None,
        required_extras: Vec::new(),
        initial_goal: None,
        goal_focuses: Vec::new(),
        productions: Vec::new(),
        scheduled_events: Vec::new(),
        buffer_decls: Vec::new(),
        visloc_default: Vec::new(),
        pfct: Vec::new(),
        no_reuse_buffers: Vec::new(),
        disabled_prods: Vec::new(),
        initial_buffer_sets: Vec::new(),
        reset_warnings: Vec::new(),
    };
    // `(spp <production> :key value ...)` calls, applied after the body so
    // they may appear before their production.
    let mut spp_calls: Vec<(String, Vec<(String, Sexp)>)> = Vec::new();
    // Chunks ACT-R would have created via `create-undefined-chunk` while
    // scanning productions (`chunks.lisp:850`); later productions see them.
    let mut created_chunks: Vec<String> = Vec::new();
    for f in forms {
        let items = match f {
            Sexp::List(items) => items,
            got => return structural(format!("model body form must be a list: {got:?}")),
        };
        let head = match items.first() {
            Some(Sexp::Sym(h)) => h.as_str(),
            got => return structural(format!("model body head must be a symbol: {got:?}")),
        };
        match head {
            "SGP" => {
                let rest = &items[1..];
                if rest.len() % 2 != 0 {
                    return structural("sgp needs even key/value count");
                }
                for kv in rest.as_chunks::<2>().0 {
                    let key = match &kv[0] {
                        Sexp::Sym(k) => k.clone(),
                        got => return structural(format!("sgp key must be a symbol: {got:?}")),
                    };
                    model.params.push((key, kv[1].clone()));
                }
            }
            // `(sgp-fct (list :key value ...))`: same as `sgp`.
            "SGP-FCT" => {
                if let Some(Sexp::List(args)) = items.get(1) {
                    if args.first().and_then(|x| x.as_sym()) == Some("LIST") {
                        let rest = &args[1..];
                        let mut i = 0;
                        while i + 1 < rest.len() {
                            if let Some(k) = rest[i].as_sym() {
                                model
                                    .params
                                    .push((k.to_string(), subst_bare(&rest[i + 1], defs)));
                            }
                            i += 2;
                        }
                    }
                }
            }
            "CHUNK-TYPE" => model.chunk_types.push(parse_chunk_type(f)?),
            "ADD-DM" => {
                let mut anon = 0u64;
                for e in &items[1..] {
                    let named_or_typed = match e {
                        Sexp::List(p) => {
                            matches!(p.as_slice(), [Sexp::Sym(_)])
                                || matches!(
                                    p.as_slice(),
                                    [Sexp::Sym(_), Sexp::Sym(isa), Sexp::Sym(_), ..] if isa == "ISA"
                                )
                                || (p.len() % 2 == 1 && matches!(p.as_slice(), [Sexp::Sym(_), ..]))
                        }
                        _ => false,
                    };
                    if named_or_typed {
                        model.chunks.push(parse_dm_entry(e)?);
                    } else if let Sexp::List(p) = e {
                        // Anonymous slot-value list (`(add-dm (value t))`):
                        // a default-CHUNK chunk auto-named `CHUNK{k}`.
                        model.chunks.push(Chunk {
                            name: format!("CHUNK{anon}"),
                            ctype: Some("CHUNK".to_string()),
                            slots: parse_pairs_plain(p)?,
                        });
                        anon += 1;
                    } else {
                        return structural(format!("bad add-dm entry: {e:?}"));
                    }
                }
            }
            "DEFINE-CHUNKS" => {
                // Named chunks that live in the model's chunk table but not
                // in declarative memory (`framework/chunks.lisp:1129`).
                // Anonymous `(isa TYPE ...)` entries are auto-named
                // `TYPE{index}`, as ACT-R does.
                for e in &items[1..] {
                    define_chunk_table_entry(e, &mut model)?;
                }
            }
            "GOAL-FOCUS" | "GOAL-FOCUS-FCT" => {
                if items.len() != 2 {
                    return structural("goal-focus takes exactly one argument");
                }
                // `(goal-focus-fct (car (define-chunks DEF)))`: define the
                // chunk(s), then focus the first. Errors in the definition
                // leave the goal unset (ACT-R warns and `(car nil)` is nil).
                if let Some(name) = car_define_chunks(&items[1], &mut model)? {
                    let desc = name.map(ChunkDesc::Named);
                    model.initial_goal = desc.clone();
                    if let Some(d) = desc {
                        model.goal_focuses.push(d);
                    }
                } else {
                    let desc = match &items[1] {
                        Sexp::List(spec) => parse_chunk_desc(spec)?,
                        Sexp::Sym(n) => ChunkDesc::Named(n.clone()),
                        got => return structural(format!("bad goal-focus arg: {got:?}")),
                    };
                    model.initial_goal = Some(desc.clone());
                    model.goal_focuses.push(desc);
                }
            }
            // `(set-similarities (a b v) ...)`: explicit Sji between named
            // chunks (`commands/dm-commands.lisp:513`). Values are stored
            // both ways at reset.
            "SET-SIMILARITIES" => {
                for e in &items[1..] {
                    if let Sexp::List(p) = e {
                        if let (Some(a), Some(b), Some(v)) = (
                            p.first().and_then(|x| x.as_sym()),
                            p.get(1).and_then(|x| x.as_sym()),
                            p.get(2).and_then(sexp_f64),
                        ) {
                            model
                                .similarities
                                .push((a.to_string(), b.to_string(), v as f32));
                        }
                    }
                }
            }
            // `(set-base-levels (name level [creation-seconds]) ...)`: fixed
            // base-level history (`commands/dm-commands.lisp:557`).
            "SET-BASE-LEVELS" => {
                for e in &items[1..] {
                    if let Sexp::List(p) = e {
                        if let (Some(name), Some(level)) = (
                            p.first().and_then(|x| x.as_sym()),
                            p.get(1).and_then(sexp_f64),
                        ) {
                            let creation = p.get(2).and_then(sexp_f64);
                            model
                                .base_levels
                                .push((name.to_string(), level as f32, creation));
                        }
                    }
                }
            }
            // `(set-all-base-levels <level> [creation-seconds])`: fixed
            // base-level for every DM chunk (`commands/dm-commands.lisp:600`).
            "SET-ALL-BASE-LEVELS" => {
                if let Some(level) = items.get(1).and_then(sexp_f64) {
                    let creation = items.get(2).and_then(sexp_f64);
                    model.set_all_base_levels = Some((level as f32, creation));
                }
            }
            // `(declare-buffer-usage <buffer> <chunk-type> <slots...>)`:
            // records the buffer's default chunk type (`:buffer`
            // declaration), used by `set-buffer-chunk`.
            "DECLARE-BUFFER-USAGE" => {
                if let (Some(b), Some(t)) = (
                    items.get(1).and_then(|x| x.as_sym()),
                    items.get(2).and_then(|x| x.as_sym()),
                ) {
                    model
                        .buffer_decls
                        .push((b.to_uppercase(), t.to_uppercase()));
                }
            }
            // `(set-visloc-default <slot> <value> ...)`: default constraints
            // for an unconstrained visual-location request.
            "SET-VISLOC-DEFAULT" => {
                let mut i = 1;
                while i + 1 < items.len() {
                    if let Some(k) = items[i].as_sym() {
                        model
                            .visloc_default
                            .push((k.to_uppercase(), items[i + 1].clone()));
                    }
                    i += 2;
                }
            }
            // `(spp <production> :param value ...)` — production-level
            // parameters (`:reward`, `:at`). Collected and applied after the
            // body so they may precede their production.
            "SPP" => {
                if let Some(p) = items.get(1).and_then(|x| x.as_sym()) {
                    let mut kv = Vec::new();
                    let rest = &items[2..];
                    for pair in rest.as_chunks::<2>().0 {
                        if let Some(k) = pair[0].as_sym() {
                            kv.push((k.to_uppercase(), pair[1].clone()));
                        }
                    }
                    spp_calls.push((p.to_uppercase(), kv));
                }
            }
            // `(schedule-event <secs> "command" :params (...) ...)`:
            // schedule a task command relative to model time 0.
            "SCHEDULE-EVENT" | "SCHEDULE-EVENT-RELATIVE" => {
                let at = items
                    .get(1)
                    .and_then(sexp_f64)
                    .ok_or_else(|| LoadError::Structural("schedule-event needs a time".into()))?;
                let command = items
                    .get(2)
                    .and_then(|x| match x {
                        Sexp::Str(s) => Some(s.clone()),
                        Sexp::Sym(s) => Some(s.clone()),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        LoadError::Structural("schedule-event needs a command".into())
                    })?;
                let mut params = Vec::new();
                let mut i = 3;
                while i + 1 < items.len() {
                    if items[i].as_sym() == Some(":PARAMS") {
                        if let Sexp::List(ps) = &items[i + 1] {
                            // `(list a b ...)`: keep the elements as forms.
                            params = if ps.first().and_then(|x| x.as_sym()) == Some("LIST") {
                                ps[1..].to_vec()
                            } else {
                                ps.clone()
                            };
                        }
                    }
                    i += 2;
                }
                model
                    .scheduled_events
                    .push(((at * 1000.0).round() as i64, command, params));
            }
            "P" => {
                let extra: Vec<String> =
                    model.buffer_decls.iter().map(|(b, _)| b.clone()).collect();
                match parse_production(f, &extra) {
                    Ok(p) => {
                        let mut known: Vec<String> = model
                            .chunks
                            .iter()
                            .map(|c| c.name.clone())
                            .chain(model.defined_chunks.iter().map(|c| c.name.clone()))
                            .chain(created_chunks.iter().cloned())
                            .collect();
                        if let Sexp::List(list) = f {
                            let body = &list[2..];
                            if let Some(split) = body
                                .iter()
                                .position(|s| matches!(s, Sexp::Sym(t) if t == "==>"))
                            {
                                let lhs = &body[..split];
                                let rhs = &body[split + 1..];
                                model
                                    .reset_warnings
                                    .extend(reorder_warning(&p.name, rhs, &extra));
                                model
                                    .reset_warnings
                                    .extend(undefined_chunk_warnings(lhs, rhs, &mut known, &extra));
                            }
                        }
                        created_chunks = known
                            .into_iter()
                            .filter(|k| {
                                !model.chunks.iter().any(|c| &c.name == k)
                                    && !model.defined_chunks.iter().any(|c| &c.name == k)
                            })
                            .collect();
                        push_production(&mut model, p);
                    }
                    Err(LoadError::RejectedProduction(w)) => model.reset_warnings.extend(w),
                    Err(e) => return Err(e),
                }
            }
            // `(p-fct \`(name ... ,*var* ...))`: a backquoted production with
            // model-file `defparameter` substitutions.
            "P-FCT" => {
                if let Some(Sexp::List(list)) = items.get(2) {
                    model.pfct.push(list.clone());
                    let sub = substitute_commas(list, defs);
                    let mut form = vec![Sexp::Sym("P".to_string())];
                    form.extend(sub);
                    let extra: Vec<String> =
                        model.buffer_decls.iter().map(|(b, _)| b.clone()).collect();
                    match parse_production(&Sexp::List(form), &extra) {
                        Ok(p) => push_production(&mut model, p),
                        Err(LoadError::RejectedProduction(w)) => model.reset_warnings.extend(w),
                        Err(e) => return Err(e),
                    }
                }
            }
            // `(install-device ...)`: task device setup (the spec's device
            // sidecar models the observable effects).
            "INSTALL-DEVICE" => {}
            // `(buffer-requires-copies 'buffer)`: the named buffer will not
            // reuse a single chunk (`buffers.lisp:1920`). Recorded so the
            // imaginal/associative-learning source buffers create a fresh
            // chunk per request.
            "BUFFER-REQUIRES-COPIES" => {
                // `(buffer-requires-copies 'imaginal)`: the argument arrives
                // as `(QUOTE IMAGINAL)`.
                let name = items.get(1).and_then(|x| match x {
                    Sexp::Sym(s) => Some(s.clone()),
                    Sexp::List(q) => match q.as_slice() {
                        [Sexp::Sym(q), Sexp::Sym(s)] if q == "QUOTE" => Some(s.clone()),
                        _ => None,
                    },
                    _ => None,
                });
                if let Some(b) = name {
                    model.no_reuse_buffers.push(b.to_uppercase());
                }
            }
            // `(pdisable name...)` / `(penable name...)` in the model body:
            // ACT-R re-evaluates the body on every reset (`model.lisp:983`),
            // so these are re-applied then. `(pdisable-fct (all-productions))`
            // is recorded as `"*"` (every production).
            "PDISABLE" | "PENABLE" | "PDISABLE-FCT" | "PENABLE-FCT" => {
                let disable = head.starts_with("PDISABLE");
                if items.len() == 1 {
                    if !disable {
                        model.disabled_prods.clear();
                    }
                } else if let Some(Sexp::List(call)) = items.get(1) {
                    if call.first().and_then(|x| x.as_sym()) == Some("ALL-PRODUCTIONS") {
                        if !model.disabled_prods.contains(&"*".to_string()) {
                            model.disabled_prods.push("*".to_string());
                        }
                    }
                } else {
                    for e in &items[1..] {
                        if let Some(n) = e.as_sym() {
                            let n = n.to_uppercase();
                            if disable {
                                if !model.disabled_prods.contains(&n) {
                                    model.disabled_prods.push(n);
                                }
                            } else {
                                model.disabled_prods.retain(|x| x != &n);
                            }
                        }
                    }
                }
            }
            // `(set-buffer-chunk '<buffer> '<chunk>)` in the model body: an
            // initial buffer content, re-applied on every reset (silently —
            // no trace line).
            "SET-BUFFER-CHUNK" => {
                let sym = |e: Option<&Sexp>| -> Option<String> {
                    match e {
                        Some(Sexp::Sym(s)) => Some(s.clone()),
                        Some(Sexp::List(q)) => match q.as_slice() {
                            [Sexp::Sym(q), Sexp::Sym(s)] if q == "QUOTE" => Some(s.clone()),
                            _ => None,
                        },
                        _ => None,
                    }
                };
                if let (Some(b), Some(c)) = (sym(items.get(1)), sym(items.get(2))) {
                    model.initial_buffer_sets.push((b.to_uppercase(), c));
                }
            }
            // `(declare-buffer-usage <buffer> <type> <slots>)`: compiler
            // metadata (which chunk type a buffer holds); it does not affect
            // the trace, so it is accepted and ignored.
            "DECLARE-BUFFER-USAGE" => {}
            got => return structural(format!("unknown model body form: {got}")),
        }
    }
    // Apply collected `spp` parameters to their productions.
    for (pname, kv) in &spp_calls {
        if let Some(prod) = model
            .productions
            .iter_mut()
            .find(|p| p.name.eq_ignore_ascii_case(pname))
        {
            for (k, v) in kv {
                if k == ":REWARD" {
                    if let Some(r) = sexp_f64(v) {
                        prod.reward = Some(r as f32);
                    }
                } else if k == ":AT" {
                    if let Some(a) = sexp_f64(v) {
                        prod.at = Some(a as f32);
                    }
                } else if k == ":U" {
                    if let Some(u) = sexp_f64(v) {
                        prod.utility = Some(u as f32);
                    }
                }
            }
        }
    }
    // Apply chunk-type default slot values to chunks that don't set them.
    let types = model.chunk_types.clone();
    for c in model
        .chunks
        .iter_mut()
        .chain(model.defined_chunks.iter_mut())
    {
        if let Some(t) = &c.ctype {
            if let Some(ct) = types.iter().find(|ct| &ct.name == t) {
                for (s, p) in &ct.defaults {
                    if !c.slots.iter().any(|(k, _)| k == s) {
                        c.slots.push((s.clone(), p.clone()));
                    }
                }
            }
        }
    }
    Ok(model)
}

/// Add a production, replacing any earlier one of the same name
/// (`redefine-production`): ACT-R keeps the last definition and warns at the
/// next reset (`#|Warning: Production F already exists …|#`).
fn push_production(model: &mut Model, p: Production) {
    if let Some(pos) = model.productions.iter().position(|q| q.name == p.name) {
        model.productions.remove(pos);
        model.reset_warnings.push(format!(
            "#|Warning: Production {} already exists and it is being redefined. |#",
            p.name
        ));
    }
    model.productions.push(p);
}

/// Load every `define-model` in a `.lisp` model file (M0 files hold one).
/// `(clear-all)` is acknowledged and skipped; anything else top-level errors.
pub fn load_models(path: &Path) -> Result<Vec<Model>, LoadError> {
    let src = fs::read_to_string(path).map_err(LoadError::Io)?;
    let mut models = Vec::new();
    let mut required_extras: Vec<String> = Vec::new();
    let mut defs: std::collections::HashMap<String, Sexp> = std::collections::HashMap::new();
    for form in parse_all(&src)? {
        match &form {
            Sexp::List(items) => match items.first() {
                Some(Sexp::Sym(h)) if h == "CLEAR-ALL" => {}
                // `(require-extra "name")` loads an extra Lisp module; record
                // it so the engine can tell which behaviors are expected.
                Some(Sexp::Sym(h)) if h == "REQUIRE-EXTRA" => {
                    if let Some(Sexp::Str(n)) = items.get(1) {
                        required_extras.push(n.clone());
                    }
                }
                Some(Sexp::Sym(h)) if h == "DEFINE-MODEL" => {
                    models.push(parse_model(&form, &defs)?)
                }
                // `(written-for-act-r-version "7.15" "...")`: a version
                // annotation with no runtime effect.
                Some(Sexp::Sym(h)) if h == "WRITTEN-FOR-ACT-R-VERSION" => {}
                // Task-file data alongside a model (`defun`/`defvar`/...):
                // skipped, not treated as a model form.
                Some(Sexp::Sym(h))
                    if matches!(
                        h.as_str(),
                        "DEFUN"
                            | "DEFVAR"
                            | "DEFPARAMETER"
                            | "DEFCONSTANT"
                            | "DEFMACRO"
                            | "DEFMETHOD"
                            | "ADD-ACT-R-COMMAND"
                    ) =>
                {
                    // Model-file `defparameter`/`defvar` values feed `p-fct`
                    // backquote substitutions.
                    if matches!(h.as_str(), "DEFPARAMETER" | "DEFVAR") {
                        if let Some(Sexp::Sym(n)) = items.get(1) {
                            let v = items
                                .get(2)
                                .cloned()
                                .unwrap_or(Sexp::Sym("NIL".to_string()));
                            defs.insert(n.to_uppercase(), v);
                        }
                    }
                }
                got => return structural(format!("unexpected top-level form: {got:?}")),
            },
            got => return structural(format!("unexpected top-level form: {got:?}")),
        }
    }
    for m in &mut models {
        m.required_extras = required_extras.clone();
    }
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn m0(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("actr7.x")
            .join("tutorial")
            .join("unit1")
            .join(name)
    }

    fn the_only(path: &str) -> Model {
        let models = load_models(&m0(path)).unwrap();
        assert_eq!(models.len(), 1);
        models.into_iter().next().unwrap()
    }

    fn slots_of(chunk: &Chunk) -> Vec<(&str, String)> {
        chunk
            .slots
            .iter()
            .map(|(s, v)| (s.as_str(), format!("{v:?}")))
            .collect()
    }

    #[test]
    fn count_loads() {
        let m = the_only("count.lisp");
        assert_eq!(m.name, "COUNT");
        assert_eq!(
            m.chunk_types,
            vec![
                ChunkType {
                    name: "NUMBER".into(),
                    slots: vec!["NUMBER".into(), "NEXT".into()],
                    defaults: vec![],
                },
                ChunkType {
                    name: "COUNT-FROM".into(),
                    slots: vec!["START".into(), "END".into(), "COUNT".into()],
                    defaults: vec![],
                },
            ]
        );
        assert_eq!(m.chunks.len(), 5);
        let one = &m.chunks[0];
        assert_eq!(one.name, "ONE");
        assert_eq!(one.ctype.as_deref(), Some("NUMBER"));
        assert_eq!(
            slots_of(one),
            vec![
                ("NUMBER", "Sym(\"ONE\")".into()),
                ("NEXT", "Sym(\"TWO\")".into())
            ]
        );
        // `(five ISA number number five)` — type with a single slot.
        let five = &m.chunks[4];
        assert_eq!(five.ctype.as_deref(), Some("NUMBER"));
        assert_eq!(slots_of(five), vec![("NUMBER", "Sym(\"FIVE\")".into())]);

        match m.initial_goal.as_ref().unwrap() {
            ChunkDesc::Spec { ctype, slots, .. } => {
                assert_eq!(ctype, "COUNT-FROM");
                let get = |s: &str| slots.iter().find(|(k, _)| k == s).map(|(_, v)| v.clone());
                assert_eq!(get("START"), Some(Pattern::Sym("TWO".into())));
                assert_eq!(get("END"), Some(Pattern::Sym("FOUR".into())));
            }
            got => panic!("goal should be a spec: {got:?}"),
        }

        let names: Vec<&str> = m.productions.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["START", "INCREMENT", "STOP"]);

        // `start` LHS: =goal> with ISA + var + nil tests.
        let start = &m.productions[0];
        assert_eq!(start.lhs.len(), 1);
        match &start.lhs[0] {
            LhsItem::Match { buffer, tests } => {
                assert_eq!(buffer, "GOAL");
                assert_eq!(tests.len(), 3);
                assert_eq!(tests[0].slot, "ISA");
                assert_eq!(tests[0].value, Pattern::Sym("COUNT-FROM".into()));
                assert_eq!(tests[1].value, Pattern::Var("NUM1".into()));
                assert_eq!(tests[1].op, SlotOp::Eq);
                assert_eq!(tests[2].slot, "COUNT");
                assert_eq!(tests[2].value, Pattern::Nil);
            }
            got => panic!("{got:?}"),
        }
        // `increment` has a `- end =num1` negated test.
        let inc = &m.productions[1];
        let neg: Vec<_> = inc
            .lhs
            .iter()
            .flat_map(|item| match item {
                LhsItem::Match { tests, .. } => tests.clone(),
                _ => vec![],
            })
            .filter(|t| t.op == SlotOp::Neg)
            .collect();
        assert_eq!(neg.len(), 1);
        assert_eq!(neg[0].slot, "END");
        assert_eq!(neg[0].value, Pattern::Var("NUM1".into()));
        // `increment` RHS requests a retrieval and prints.
        assert!(matches!(
            &inc.rhs[1],
            RhsAction::Request { buffer, .. } if buffer == "RETRIEVAL"
        ));
        assert!(matches!(inc.rhs[2], RhsAction::Output { .. }));
        // `stop` RHS clears goal then prints.
        let stop = &m.productions[2];
        assert_eq!(stop.rhs.len(), 2);
        assert!(matches!(
            &stop.rhs[0],
            RhsAction::Clear { buffer } if buffer == "GOAL"
        ));

        let param = |k: &str| m.params.iter().find(|(key, _)| key == k).map(|(_, v)| v);
        assert_eq!(param(":ESC"), Some(&Sexp::Sym("T".into())));
        assert!(matches!(param(":LF"), Some(Sexp::Float(x)) if (*x - 0.05).abs() < 1e-12));
    }

    #[test]
    fn semantic_loads_bare_chunks_and_queries() {
        let m = the_only("semantic.lisp");
        assert_eq!(m.name, "SEMANTIC");
        // `(shark)` — chunk with no slots and no type.
        let shark = m.chunks.iter().find(|c| c.name == "SHARK").unwrap();
        assert_eq!(shark.ctype, None);
        assert!(shark.slots.is_empty());
        let p14 = m.chunks.iter().find(|c| c.name == "P14").unwrap();
        assert_eq!(p14.ctype.as_deref(), Some("PROPERTY"));

        let names: Vec<&str> = m.productions.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "INITIAL-RETRIEVE",
                "DIRECT-VERIFY",
                "CHAIN-CATEGORY",
                "FAIL"
            ]
        );
        // `fail` LHS ends with `?retrieval> buffer failure`.
        let fail = &m.productions[3];
        match fail.lhs.last().unwrap() {
            LhsItem::Query { buffer, queries } => {
                assert_eq!(buffer, "RETRIEVAL");
                assert_eq!(
                    queries,
                    &vec![("BUFFER".to_string(), Pattern::Sym("FAILURE".into()))]
                );
            }
            got => panic!("{got:?}"),
        }
    }

    #[test]
    fn addition_loads() {
        let m = the_only("addition.lisp");
        assert_eq!(m.name, "ADDITION");
        assert_eq!(m.chunks.len(), 11);
        let names: Vec<&str> = m.productions.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "INITIALIZE-ADDITION",
                "TERMINATE-ADDITION",
                "INCREMENT-COUNT",
                "INCREMENT-SUM"
            ]
        );
        match m.initial_goal.as_ref().unwrap() {
            ChunkDesc::Spec { ctype, slots, .. } => {
                assert_eq!(ctype, "ADD");
                assert!(slots.contains(&("ARG1".to_string(), Pattern::Sym("FIVE".into()))));
            }
            got => panic!("{got:?}"),
        }
    }

    #[test]
    fn structural_errors_are_loud() {
        // `==>` missing.
        assert!(parse_production(&parse_all("(p x =goal> ISA y)").unwrap()[0], &[]).is_err());
        // Odd slot/value count.
        assert!(parse_pairs(&[Sexp::Sym("A".into())]).is_err());
        // Clear with args.
        assert!(parse_rhs(&parse_all("-goal> ISA x").unwrap())
            .unwrap_err()
            .to_string()
            .contains("takes no args"));
    }
}

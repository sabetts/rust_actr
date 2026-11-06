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
    /// compare byte-for-byte (PORTING-NOTES #13).
    Int(i64),
    Num(f64),
    Str(String),
    Nil,
    List(Vec<Pattern>),
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
            Sexp::List(v) => Ok(Pattern::List(
                v.iter()
                    .map(Pattern::from_sexp)
                    .collect::<Result<Vec<_>, _>>()?,
            )),
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
        /// Set when `expand_request_defaults` inserted the module `CMD`
        /// default, so `pp` can print it after the explicit slots.
        auto_cmd: bool,
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
    /// `!output! (...)` — args kept raw for the evaluator.
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
    /// Optional documentation string after the name (`(p name "doc" ...)`).
    pub documentation: Option<String>,
    pub lhs: Vec<LhsItem>,
    pub rhs: Vec<RhsAction>,
    /// `(spp <name> :reward R)`: propagated as a reward when the production
    /// is selected (`commands/procedural-cmds.lisp`).
    pub reward: Option<Reward>,
    /// `(spp <name> :at S)`: production action time override in seconds.
    pub at: Option<f32>,
    /// `(spp <name> :u U)`: initial utility override (else `:iu`).
    pub utility: Option<f32>,
}

/// A `(spp <prod> :reward R)` value. `propagate-reward`
/// (`modules/utility-and-reward-1.lisp:421`) branches on `numberp`: a number
/// updates utilities, any other non-nil value clears the utility-learning
/// history. `Other` carries the Lisp text for the `PROPAGATE-REWARD` trace
/// line (`:reward t` → `PROPAGATE-REWARD T`).
#[derive(Debug, Clone, PartialEq)]
pub enum Reward {
    Num(f32),
    Other(String),
}

impl Reward {
    /// Lisp display for the `PROPAGATE-REWARD <value>` trace line. ACT-R prints
    /// the event parameter with `~s`, so an integer reward reads `0`, not
    /// `0.0`; Rust's `f32: Display` matches for the corpus's integer rewards.
    pub fn trace_text(&self) -> String {
        match self {
            Reward::Num(v) => format!("{v}"),
            Reward::Other(s) => s.clone(),
        }
    }
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

/// Collect task-file `defparameter`/`defvar` literals for the bounded
/// evaluator. Values are stored with a leading `QUOTE` unwrapped; complex
/// initializer expressions are ignored.
pub fn parse_task_globals(src: &str) -> HashMap<String, Sexp> {
    let mut map = HashMap::new();
    let Ok(forms) = parse_all(src) else {
        return map;
    };
    for f in forms {
        let Sexp::List(items) = &f else {
            continue;
        };
        if !matches!(
            items.first().and_then(|s| s.as_sym()),
            Some("DEFPARAMETER" | "DEFVAR" | "DEFCONSTANT")
        ) {
            continue;
        }
        let Some(name) = items.get(1).and_then(|s| s.as_sym()) else {
            continue;
        };
        let raw = items
            .get(2)
            .cloned()
            .unwrap_or(Sexp::Sym("NIL".to_string()));
        let value = match &raw {
            Sexp::List(v) if v.len() == 2 && v[0].as_sym() == Some("QUOTE") => v[1].clone(),
            _ => raw,
        };
        map.insert(name.to_uppercase(), value);
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

/// Top-level `(delete-model name)` forms in a code file. `None` means
/// `(delete-model)` and targets the current model. Parse failures yield an
/// empty list; model-file loading still reports their syntax errors itself.
pub fn parse_model_deletions(src: &str) -> Vec<Option<String>> {
    let Ok(forms) = parse_all(src) else {
        return Vec::new();
    };
    forms
        .iter()
        .filter_map(|f| {
            let Sexp::List(items) = f else {
                return None;
            };
            if items.first().and_then(Sexp::as_sym) != Some("DELETE-MODEL") {
                return None;
            }
            match items.get(1) {
                None => Some(None),
                Some(Sexp::Sym(name)) => Some(Some(name.clone())),
                Some(Sexp::Str(name)) => Some(Some(name.clone())),
                Some(_) => None,
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkType {
    pub name: String,
    pub parents: Vec<String>,
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
    /// Model-body `(merge-chunks a b)` aliases, applied after the named
    /// chunks exist on reset.
    pub merge_chunks: Vec<(String, String)>,
    /// Model-body `(set-similarities (a b v) ...)`.
    pub similarities: Vec<(String, String, f32)>,
    /// Model-body `(set-base-levels (name level [creation-seconds]) ...)`.
    pub base_levels: Vec<(String, f32, Option<f64>)>,
    /// Model-body `(set-all-base-levels <level> [creation-seconds])`: a fixed
    /// base level for every DM chunk (`commands/dm-commands.lisp:600`).
    pub set_all_base_levels: Option<(f32, Option<f64>)>,
    /// Model-body `(sdp :creation-time S ...)`: the creation time applied to
    /// every DM chunk on reset (`commands/dm-commands.lisp:860`).
    pub sdp_creation_time: Option<f64>,
    /// Model-body `(sdp :references (...))`: the reference list (seconds)
    /// applied to every DM chunk on reset.
    pub sdp_references: Option<Vec<f64>>,
    /// Extras requested via top-level `(require-extra "name")`.
    pub required_extras: Vec<String>,
    pub initial_goal: Option<ChunkDesc>,
    /// All model-body `(goal-focus ...)` calls in order (the threads extra
    /// applies every one).
    pub goal_focuses: Vec<ChunkDesc>,
    /// Warnings for goal-focus specs naming an unknown chunk type.
    pub goal_warnings: Vec<String>,
    pub productions: Vec<Production>,
    /// Model-body `(schedule-event <secs> "command" :params (...) ...)`:
    /// scheduled relative to model time 0 on each reset.
    pub scheduled_events: Vec<(i64, String, Vec<Sexp>)>,
    /// Model-body `(model-output "name" args...)`: literal output lines
    /// emitted when the model body is evaluated.
    pub model_outputs: Vec<String>,
    /// Model-body `(call-act-r-command "name" args...)`: run the registered
    /// task command at load and on every reset.
    pub call_commands: Vec<(String, Vec<Sexp>)>,
    /// `(declare-buffer-usage <buffer> <chunk-type> <slots...>)`: the default
    /// chunk type per buffer, used by `set-buffer-chunk`.
    pub buffer_decls: Vec<(String, String)>,
    /// The slots of each `declare-buffer-usage` (`:ALL` expands to the type's
    /// slots). ACT-R adds them to the style-check tables to suppress
    /// "tested but never set" warnings.
    pub buffer_decl_slots: Vec<(String, Vec<String>)>,

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
    /// Model-body `(set-buffer-chunk '<buffer> '(slot value ...))`.
    pub initial_buffer_specs: Vec<(String, ChunkDesc)>,
    /// Model-body `(start-hand-at-mouse)` / `(start-hand-at-keypad)`: the
    /// right hand starts on the mouse/keypad device instead of the keyboard
    /// home row (`other-files/mouse.lisp:554`, `keyboard.lisp:766`).
    pub hand_start: Option<String>,
    /// Model-body `(set-audloc-default slot value ...)`: the default
    /// constraints for an unconstrained aural-location request (the audio
    /// analogue of `set-visloc-default`).
    pub audloc_default: Vec<(String, Sexp)>,
    /// Negative default aural-location constraints (`- slot value`).
    pub audloc_neg: Vec<(String, Sexp)>,
    /// Model-body `(set-parameter-value :module-param value)`: a module
    /// parameter set from a task-file variable. ACT-R re-evaluates the model
    /// body on every reset (`model.lisp:983`), so the variable's current
    /// value is re-applied then. `(key, value)`.
    pub set_param_values: Vec<(String, Sexp)>,
    /// Model-body `(install-device (open-exp-window … :x X :y Y))`: the
    /// experiment window's origin. `open-exp-window`'s default is (300, 300);
    /// the visicon offsets window-relative coords by it and the virtual mouse
    /// cursor starts there (`mouse.lisp:554`).
    pub window_origin: Option<(i64, i64)>,
    /// Warning blocks from productions rejected by the validator. ACT-R
    /// re-evaluates the model body on `reset`, so these are re-emitted then
    /// (not at load, where the warning monitor isn't installed yet).
    pub reset_warnings: Vec<String>,
    /// `model-warning` lines from model-body evaluation (e.g. undefined
    /// chunks created by `define-chunks`). Emitted on reset only when
    /// `:model-warnings` is on.
    pub model_warnings: Vec<String>,
}

fn pattern_text(p: &Pattern) -> String {
    match p {
        Pattern::Var(v) => format!("={v}"),
        Pattern::Sym(v) => v.clone(),
        Pattern::Int(i) => i.to_string(),
        Pattern::Num(x) => x.to_string(),
        Pattern::Str(s) => format!("\"{s}\""),
        Pattern::Nil => "NIL".into(),
        Pattern::List(items) => {
            format!(
                "({})",
                items.iter().map(pattern_text).collect::<Vec<_>>().join(" ")
            )
        }
    }
}

fn chunk_desc_text(desc: &ChunkDesc) -> String {
    match desc {
        ChunkDesc::Named(name) => format!("({name})"),
        ChunkDesc::Spec { ctype, slots, .. } => {
            let mut parts = vec!["ISA".to_string(), ctype.clone()];
            for (slot, value) in slots {
                parts.push(slot.clone());
                parts.push(pattern_text(value));
            }
            format!("({})", parts.join(" "))
        }
    }
}

fn goal_focus_valid(model: &Model, desc: &ChunkDesc) -> bool {
    match desc {
        ChunkDesc::Named(name) => model
            .chunks
            .iter()
            .chain(model.defined_chunks.iter())
            .any(|c| c.name.eq_ignore_ascii_case(name)),
        ChunkDesc::Spec { ctype, .. } => {
            ctype.is_empty()
                || model
                    .chunk_types
                    .iter()
                    .any(|t| t.name.eq_ignore_ascii_case(ctype))
        }
    }
}

fn validate_goal_focus(model: &mut Model) {
    let mut invalid = Vec::new();
    if let Some(goal) = model.initial_goal.clone() {
        if !goal_focus_valid(model, &goal) {
            invalid.push(goal);
            model.initial_goal = None;
        }
    }
    let valid: Vec<bool> = model
        .goal_focuses
        .iter()
        .map(|goal| goal_focus_valid(model, goal))
        .collect();
    let goals = std::mem::take(&mut model.goal_focuses);
    let mut kept = Vec::new();
    for (goal, ok) in goals.into_iter().zip(valid) {
        if ok {
            kept.push(goal);
        } else {
            invalid.push(goal);
        }
    }
    model.goal_focuses = kept;
    let mut seen = Vec::new();
    for goal in invalid {
        let text = chunk_desc_text(&goal);
        if seen.contains(&text) {
            continue;
        }
        seen.push(text.clone());
        model.goal_warnings.push(format!(
            "Element after isa in define-chunk-spec isn't a chunk-type. {text}"
        ));
        model.goal_warnings.push(format!(
            "{text} is not a valid chunk description - goal-focus failed."
        ));
    }
}

impl Model {
    /// Apply chunk-type default slot values to chunks that don't set them.
    pub fn apply_chunk_defaults(&mut self) {
        let types = self.chunk_types.clone();
        for c in self.chunks.iter_mut().chain(self.defined_chunks.iter_mut()) {
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
    }

    /// Re-parse the `p-fct` productions with `query` substituted for
    /// `,*query*` (ACT-R re-evaluates the model body on `reset`).
    pub fn reparse_pfct(&mut self, query: &str) {
        for raw in &self.pfct {
            let mut list = raw.clone();
            for e in list.iter_mut() {
                replace_query(e, query);
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

    /// ACT-R's model-wide `slot->index` table (`chunk-type-fct`,
    /// `framework/chunk-types.lisp:434`): a slot name takes the next index the
    /// first time it appears in *any* chunk-type definition, so the table is
    /// shared across types rather than per type. `pprint-a-chunk`
    /// (`framework/chunks.lisp:686`) prints a chunk's set slots sorted by this
    /// index — which is why `math-fact`'s `result` (declared first, in
    /// `simple-value`) precedes `arg1`/`arg2`. Keyed upper-case.
    pub fn slot_index(&self) -> HashMap<String, usize> {
        let mut map: HashMap<String, usize> = HashMap::new();
        // `create-model-default-chunk-types` (`model.lisp:753`) runs before the
        // model body: CHUNK, CONSTANT-CHUNKS (name), CLEAR (clear), and
        // QUERY-SLOTS (state buffer error). Each standard module then defines
        // its chunk types in load order, and those names claim their slots
        // before the model body's chunk types. This is the complete index from
        // ACT-R 7 with the standard modules loaded; it matters because a model
        // slot that already exists here (e.g. `value` from visual-object or
        // `object` from the motor vocabulary) keeps that earlier index.
        for s in [
            "NAME",
            "CLEAR",
            "STATE",
            "BUFFER",
            "ERROR",
            "ONSET",
            "OFFSET",
            "DURATION",
            "PITCH",
            "KIND",
            "LOCATION",
            "ID",
            "SET-AUDLOC-DEFAULT",
            "CONTENT",
            "EVENT",
            "ACTION",
            "SLOTS",
            "RESULT",
            "SIMPLE",
            "CMD",
            "STYLE",
            "HAND",
            "FINGER",
            "R",
            "THETA",
            "OBJECT",
            "LOC",
            "KEY",
            "TO-KEY",
            "OFFSETS",
            "STRING",
            "SPEAK",
            "SUBVOCALIZE",
            "TICKS",
            "SCREEN-POS",
            "VALUE",
            "STATUS",
            "COLOR",
            "HEIGHT",
            "WIDTH",
            "TEXT",
            "OVAL",
            "LINE",
            "END1-X",
            "END1-Y",
            "END2-X",
            "END2-Y",
            "SCREEN-X",
            "SCREEN-Y",
            "DISTANCE",
            "SIZE",
            "SET-VISLOC-DEFAULT",
            "SCALE",
            "PHRASE!",
            "OBJECTS",
            "WORDS",
            "COLORS",
            "LEFT",
            "RIGHT",
            "EMPTY-SPACE",
            "RECENTLY-RETRIEVED",
            "MP-VALUE",
            "RT-VALUE",
            "BLL-VALUE",
            "MAS-VALUE",
            "ANS-VALUE",
            "LF-VALUE",
            "SCENE-CHANGE-VALUE",
            "SCENE-CHANGE",
            "MODALITY",
            "PREPARATION",
            "EXECUTION",
            "PROCESSOR",
            "LAST-COMMAND",
            "ATTENDED",
            "NEAREST",
            "CENTER",
            "FINISHED",
        ] {
            let n = map.len();
            map.entry(s.to_string()).or_insert(n);
        }
        for ct in &self.chunk_types {
            for s in &ct.slots {
                let n = map.len();
                map.entry(s.to_uppercase()).or_insert(n);
            }
        }
        map
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

/// Normalize a variable reference to a comparison key: strip the leading `=`
/// (`Pattern::Var` stores without it, `find_variables` with it) and upper-case.
fn var_key(s: &str) -> String {
    s.trim_start_matches('=').to_uppercase()
}

/// The buffers a production makes *non-reusable*: if the buffer's variable
/// (`=buffer`) appears anywhere in the production's LHS or RHS, ACT-R calls
/// `buffer-requires-copies` for that buffer
/// (`support/production-parsing.lisp:482-488`), so every set into it creates a
/// fresh `CHUNK{n}` instead of reusing `{BUFFER}-CHUNK0`.
///
/// That is why `simple-dynamic-model`'s imaginal chunk is `IMAGINAL-CHUNK0`
/// (no production names `=imaginal`) while `act-touch`'s and
/// `merging-test`'s are `CHUNK{n}` (`loc =imaginal`, `value =imaginal`).
pub fn var_referenced_buffers(p: &Production) -> Vec<String> {
    let mut vars: std::collections::HashSet<String> = std::collections::HashSet::new();
    let add = |pat: &Pattern, vars: &mut std::collections::HashSet<String>| {
        if let Pattern::Var(x) = pat {
            vars.insert(var_key(x));
        }
    };
    for item in &p.lhs {
        match item {
            LhsItem::Match { tests, .. } => {
                for t in tests {
                    add(&t.value, &mut vars);
                }
            }
            LhsItem::Query { queries, .. } => {
                for (_, v) in queries {
                    add(v, &mut vars);
                }
            }
            LhsItem::Eval { value, .. }
            | LhsItem::Bind { value, .. }
            | LhsItem::MvBind { value, .. } => {
                for v in find_variables(value) {
                    vars.insert(var_key(&v));
                }
            }
        }
    }
    for a in &p.rhs {
        match a {
            RhsAction::Modify { slots, .. } | RhsAction::ModifyRequest { slots, .. } => {
                for (_, v) in slots {
                    add(v, &mut vars);
                }
            }
            RhsAction::Request { spec, .. } | RhsAction::DirectSet { spec, .. } => {
                if let ChunkDesc::Spec {
                    slots, neg, rels, ..
                } = spec
                {
                    for (_, v) in slots.iter().chain(neg.iter()) {
                        add(v, &mut vars);
                    }
                    for (_, v, _) in rels {
                        add(v, &mut vars);
                    }
                }
            }
            RhsAction::Output { args } | RhsAction::Other { args, .. } => {
                for x in args {
                    for v in find_variables(x) {
                        vars.insert(var_key(&v));
                    }
                }
            }
            RhsAction::Eval { value, .. }
            | RhsAction::Bind { value, .. }
            | RhsAction::MvBind { value, .. } => {
                for v in find_variables(value) {
                    vars.insert(var_key(&v));
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for item in &p.lhs {
        if let LhsItem::Match { buffer, .. } = item {
            if vars.contains(&var_key(buffer)) && !out.contains(buffer) {
                out.push(buffer.clone());
            }
        }
    }
    out
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

/// A numeric inequality whose value is the literal `NIL`. ACT-R rejects the
/// entire production and prints the predicate name in the warning.
fn nil_inequality(definition: &[Sexp]) -> Option<(String, String)> {
    let mut i = 0;
    if matches!(definition.first(), Some(Sexp::Sym(isa)) if isa == "ISA") {
        i = 2;
    }
    while i + 2 < definition.len() {
        let op = match &definition[i] {
            Sexp::Sym(op) if matches!(op.as_str(), "<" | "<=" | ">" | ">=") => op.clone(),
            _ => {
                i += 2;
                continue;
            }
        };
        let slot = match &definition[i + 1] {
            Sexp::Sym(slot) if !slot.starts_with('=') => slot.clone(),
            _ => {
                i += 3;
                continue;
            }
        };
        if matches!(&definition[i + 2], Sexp::Sym(v) if v == "NIL") {
            return Some((op, slot));
        }
        i += 3;
    }
    None
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
                if let Some((op, slot)) = nil_inequality(definition) {
                    return Some(vec![
                        format!("Invalid syntax in {} condition.", seg_text(seg)),
                        format!(
                            "A {op} test for slot {slot} with a value of nil not allowed in chunk-spec."
                        ),
                    ]);
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
) -> (Vec<String>, Vec<String>) {
    let mut lhs_out = Vec::new();
    let mut rhs_out = Vec::new();
    let check = |v: &Sexp, known: &mut Vec<String>, out: &mut Vec<String>| {
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
                check(v, known, &mut lhs_out);
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
                        check(v, known, &mut rhs_out);
                    }
                }
                '+' | '*' => {
                    if definition.len() == 1 {
                        match &definition[0] {
                            Sexp::Sym(v) => check(&Sexp::Sym(v.clone()), known, &mut rhs_out),
                            Sexp::List(l) => {
                                if let Some(v) = l.first().and_then(|s| s.as_sym()) {
                                    check(&Sexp::Sym(v.to_string()), known, &mut rhs_out);
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
    (lhs_out, rhs_out)
}

/// Slot names supplied by the core modules. ACT-R's `valid-slot-name` sees
/// these in the module chunk-types before any model type is declared.
const STANDARD_MODULE_SLOTS: &[&str] = &[
    "ACTION",
    "BAD-SLOT",
    "BUFFER",
    "CMD",
    "COLOR",
    "CONTENT",
    "CONTROL-COUNT",
    "CONTROL-SLOT",
    "DELAY",
    "DISTANCE",
    "DURATION",
    "END-WIDTH",
    "ERROR",
    "EVENT",
    "FINGER",
    "GOOD-SLOT",
    "HAND",
    "HEIGHT",
    "ID",
    "INDEX",
    "KEY",
    "KIND",
    "LOC",
    "LOCATION",
    "MAX",
    "MIN",
    "NAME",
    "NUM-FNGRS",
    "NUMBER",
    "OBJECT",
    "OFFSET",
    "ONSET",
    "PITCH",
    "PREPARATION",
    "PROCESSOR",
    "PROMPT",
    "R",
    "ROTATION",
    "SCREEN-POS",
    "SCREEN-X",
    "SCREEN-Y",
    "SIMPLE",
    "SIZE",
    "SLOTS",
    "SOUND",
    "START-WIDTH",
    "STATE",
    "STATUS",
    "STRING",
    "STYLE",
    "SWIPE-SPEED",
    "TEXT",
    "THETA",
    "TICKS",
    "TO-KEY",
    "VALUE",
    "WIDTH",
    "WORD",
];

/// The slot universe ACT-R's `valid-slot-name` starts with: module defaults,
/// every declared chunk-type slot, and slots already present on chunks.
fn model_known_slots(model: &Model) -> std::collections::HashSet<String> {
    let mut set: std::collections::HashSet<String> = STANDARD_MODULE_SLOTS
        .iter()
        .map(|s| s.to_string())
        .collect();
    // `extended-motor-actions` adds the per-hand stages and finger slots to
    // the manual-left/manual-right buffers (`motor-extension.lisp:1477`).
    if model
        .required_extras
        .iter()
        .any(|e| e.eq_ignore_ascii_case("extended-motor-actions"))
    {
        for s in [
            "EXECUTION",
            "PROCESSOR",
            "LAST-COMMAND",
            "INDEX",
            "MIDDLE",
            "RING",
            "PINKIE",
            "THUMB",
        ] {
            set.insert(s.to_string());
        }
    }
    for ct in &model.chunk_types {
        for s in &ct.slots {
            set.insert(s.to_uppercase());
        }
    }
    for c in model.chunks.iter().chain(model.defined_chunks.iter()) {
        for (s, _) in &c.slots {
            set.insert(s.to_uppercase());
        }
    }
    for (buffer, slots) in &model.buffer_decl_slots {
        if slots.iter().any(|s| s == ":ALL") {
            if let Some(ct) = model
                .buffer_decls
                .iter()
                .find(|(b, _)| b == buffer)
                .and_then(|(_, t)| {
                    model
                        .chunk_types
                        .iter()
                        .find(|ct| ct.name.eq_ignore_ascii_case(t))
                })
            {
                for s in &ct.slots {
                    set.insert(s.to_uppercase());
                }
            }
        } else {
            for s in slots {
                set.insert(s.to_uppercase());
            }
        }
    }
    set
}
fn is_slot_name(s: &str) -> bool {
    s != "ISA" && !s.starts_with(':') && !s.starts_with('=')
}

fn push_slot(out: &mut Vec<String>, slot: &str) {
    if is_slot_name(slot) {
        let slot = slot.to_uppercase();
        if !out.iter().any(|s| s == &slot) {
            out.push(slot);
        }
    }
}

fn chunk_desc_slots(desc: &ChunkDesc, out: &mut Vec<String>) {
    if let ChunkDesc::Spec { slots, .. } = desc {
        for (s, _) in slots {
            push_slot(out, s);
        }
    }
}

/// All slot names named by a production, in parse order (LHS then RHS).
fn production_slots(p: &Production) -> Vec<String> {
    let mut out = Vec::new();
    for item in &p.lhs {
        match item {
            LhsItem::Match { tests, .. } => {
                for t in tests {
                    push_slot(&mut out, &t.slot);
                }
            }
            LhsItem::Query { queries, .. } => {
                for (s, _) in queries {
                    push_slot(&mut out, s);
                }
            }
            LhsItem::Eval { .. } | LhsItem::Bind { .. } | LhsItem::MvBind { .. } => {}
        }
    }
    for action in &p.rhs {
        match action {
            RhsAction::Modify { slots, .. } | RhsAction::ModifyRequest { slots, .. } => {
                for (s, _) in slots {
                    push_slot(&mut out, s);
                }
            }
            RhsAction::Request { spec, .. } | RhsAction::DirectSet { spec, .. } => {
                chunk_desc_slots(spec, &mut out);
            }
            RhsAction::Clear { .. }
            | RhsAction::DirectErase { .. }
            | RhsAction::Output { .. }
            | RhsAction::Eval { .. }
            | RhsAction::Bind { .. }
            | RhsAction::MvBind { .. }
            | RhsAction::Other { .. } => {}
        }
    }
    out
}

/// `parse-conditions`/`parse-actions`'s `uses previously undefined slot`
/// warning. The cache path prints one line per slot; the fresh parse can
/// print a parenthesised list, but the reset-time refs this port targets use
/// the singular form.
fn undefined_slot_warnings(
    p: &Production,
    model: &Model,
    extended: &mut Vec<String>,
) -> Vec<String> {
    let mut known = model_known_slots(model);
    for s in extended.iter() {
        known.insert(s.clone());
    }
    let mut new_slots = Vec::new();
    for slot in production_slots(p) {
        if known.contains(&slot) {
            continue;
        }
        known.insert(slot.clone());
        extended.push(slot.clone());
        new_slots.push(slot);
    }
    if new_slots.is_empty() {
        return Vec::new();
    }
    if style_warnings_enabled(model) {
        vec![format!(
            "Production {} uses previously undefined slots ({}).",
            p.name,
            new_slots.join(" ")
        )]
    } else {
        new_slots
            .into_iter()
            .map(|s| format!("Production {} uses previously undefined slot {s}.", p.name))
            .collect()
    }
}
/// ACT-R's type-specific slot validation for a runtime production
/// (`define-chunk-spec-fct` + `parse-conditions`/`parse-actions`). Returns the
/// chunk-spec definition warnings first, then the production-level warnings.
fn type_includes(
    model: &Model,
    child: &str,
    parent: &str,
    seen: &mut std::collections::HashSet<String>,
) -> bool {
    if child.eq_ignore_ascii_case(parent) {
        return true;
    }
    if !seen.insert(child.to_uppercase()) {
        return false;
    }
    model
        .chunk_types
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(child))
        .is_some_and(|t| {
            t.parents
                .iter()
                .any(|p| type_includes(model, p, parent, seen))
        })
}
pub fn production_type_warnings(p: &Production, model: &Model) -> (Vec<String>, Vec<String>) {
    let mut spec_warnings = Vec::new();
    let mut prod_warnings = Vec::new();
    let type_slots = |name: &str| -> Option<Vec<String>> {
        if !model
            .chunk_types
            .iter()
            .any(|t| t.name.eq_ignore_ascii_case(name))
        {
            return None;
        }
        let mut slots: Vec<String> = Vec::new();
        for t in &model.chunk_types {
            let related = t.name.eq_ignore_ascii_case(name)
                || type_includes(model, &t.name, name, &mut std::collections::HashSet::new())
                || type_includes(model, name, &t.name, &mut std::collections::HashSet::new());
            if related {
                for slot in &t.slots {
                    let slot = slot.to_uppercase();
                    if !slots.contains(&slot) {
                        slots.push(slot);
                    }
                }
            }
        }
        Some(slots)
    };
    for item in &p.lhs {
        let LhsItem::Match { tests, .. } = item else {
            continue;
        };
        let Some(ctype) = isa_type(tests) else {
            continue;
        };
        let Some(slots) = type_slots(ctype) else {
            continue;
        };
        for test in tests {
            if test.slot == "ISA"
                || test.slot.starts_with('=')
                || slots.iter().any(|s| s == &test.slot)
            {
                continue;
            }
            spec_warnings.push(format!(
                "Slot {} invalid for type {} but chunk-spec definition still created.",
                test.slot,
                ctype.to_uppercase()
            ));
            prod_warnings.push(format!(
                "Production {} tests invalid slot {} for type {}.",
                p.name,
                test.slot,
                ctype.to_uppercase()
            ));
        }
    }
    for action in &p.rhs {
        let spec = match action {
            RhsAction::Request { spec, .. } | RhsAction::DirectSet { spec, .. } => spec,
            _ => continue,
        };
        let ChunkDesc::Spec {
            ctype, slots, neg, ..
        } = spec
        else {
            continue;
        };
        let Some(type_slots) = type_slots(ctype) else {
            continue;
        };
        for (slot, _) in slots.iter().chain(neg.iter()) {
            if slot == "ISA" || slot.starts_with('=') || type_slots.iter().any(|s| s == slot) {
                continue;
            }
            spec_warnings.push(format!(
                "Slot {} invalid for type {} but chunk-spec definition still created.",
                slot,
                ctype.to_uppercase()
            ));
            prod_warnings.push(format!(
                "Production {} action has invalid slot {} for type {}.",
                p.name,
                slot,
                ctype.to_uppercase()
            ));
        }
    }
    (spec_warnings, prod_warnings)
}

pub(crate) fn style_warnings_enabled(model: &Model) -> bool {
    match model
        .params
        .iter()
        .rev()
        .find(|(k, _)| k == ":STYLE-WARNINGS")
        .map(|(_, v)| v)
    {
        Some(Sexp::Sym(s)) if s == "NIL" => false,
        _ => true,
    }
}

fn isa_type(tests: &[SlotTest]) -> Option<&str> {
    tests.iter().find_map(|t| {
        if t.slot == "ISA" {
            match &t.value {
                Pattern::Sym(s) | Pattern::Var(s) => Some(s.as_str()),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn has_isa(tests: &[SlotTest]) -> bool {
    tests.iter().any(|t| t.slot == "ISA")
}

fn add_named_slots(
    model: &Model,
    name: &str,
    out: &mut HashMap<String, Vec<String>>,
    buffer: &str,
) {
    let chunk = model
        .chunks
        .iter()
        .chain(model.defined_chunks.iter())
        .find(|c| c.name.eq_ignore_ascii_case(name));
    if let Some(c) = chunk {
        let e = out.entry(buffer.to_string()).or_default();
        for (s, _) in &c.slots {
            push_slot(e, s);
        }
    }
}

fn initial_buffer_slots(model: &Model) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(desc) = &model.initial_goal {
        match desc {
            ChunkDesc::Spec { slots, .. } => {
                let e = out.entry("GOAL".to_string()).or_default();
                for (s, _) in slots {
                    push_slot(e, s);
                }
            }
            ChunkDesc::Named(n) => add_named_slots(model, n, &mut out, "GOAL"),
        }
    }
    for desc in &model.goal_focuses {
        match desc {
            ChunkDesc::Spec { slots, .. } => {
                let e = out.entry("GOAL".to_string()).or_default();
                for (s, _) in slots {
                    push_slot(e, s);
                }
            }
            ChunkDesc::Named(n) => add_named_slots(model, n, &mut out, "GOAL"),
        }
    }
    for (buffer, desc) in &model.initial_buffer_specs {
        let e = out.entry(buffer.clone()).or_default();
        if let ChunkDesc::Spec { slots, .. } = desc {
            for (s, _) in slots {
                push_slot(e, s);
            }
        }
    }
    for (buffer, name) in &model.initial_buffer_sets {
        add_named_slots(model, name, &mut out, buffer);
    }
    // `declare-buffer-usage` adds its slots to the style-check tables, which
    // suppresses "tested but not requested/modified" warnings for them.
    for (buffer, slots) in &model.buffer_decl_slots {
        let e = out.entry(buffer.clone()).or_default();
        if slots.iter().any(|s| s == ":ALL") {
            if let Some(ct) = model
                .buffer_decls
                .iter()
                .find(|(b, _)| b == buffer)
                .and_then(|(_, t)| {
                    model
                        .chunk_types
                        .iter()
                        .find(|ct| ct.name.eq_ignore_ascii_case(t))
                })
            {
                for s in &ct.slots {
                    push_slot(e, s);
                }
            }
        } else {
            for s in slots {
                push_slot(e, s);
            }
        }
    }
    out
}

/// `check-production-for-style` + `check-between-production-style`
/// (`procedural.lisp:1327`/`:1428`). Only the checks the tutorial refs expose:
/// an `isa` with no tests, a request with no matching query, and GOAL/IMAGINAL
/// condition slots that no production requests, modifies, or initializes.
fn style_warnings(model: &Model, extended: &[String]) -> Vec<String> {
    if !style_warnings_enabled(model) {
        return Vec::new();
    }
    let mut out = Vec::new();

    for p in &model.productions {
        for item in &p.lhs {
            if let LhsItem::Match { buffer, tests } = item {
                if has_isa(tests)
                    && tests.iter().all(|t| t.slot == "ISA")
                    && !matches!(isa_type(tests), Some(t) if t.eq_ignore_ascii_case("CHUNK"))
                {
                    out.push(format!(
                        "Production {} has a condition for buffer {} with an isa that provides no tests.",
                        p.name, buffer
                    ));
                }
            }
        }
    }
    let mut tested: Vec<(String, Vec<String>)> = Vec::new();
    let mut usage: HashMap<String, Vec<String>> = HashMap::new();
    for p in &model.productions {
        for item in &p.lhs {
            if let LhsItem::Match { buffer, tests } = item {
                if buffer != "GOAL" && buffer != "IMAGINAL" {
                    continue;
                }
                let entry = if let Some(e) = tested.iter_mut().find(|(b, _)| b == buffer) {
                    e
                } else {
                    tested.push((buffer.clone(), Vec::new()));
                    tested.last_mut().expect("just pushed")
                };
                for t in tests {
                    push_slot(&mut entry.1, &t.slot);
                }
            }
        }
        for action in &p.rhs {
            let (buffer, slots) = match action {
                RhsAction::Modify { buffer, slots }
                | RhsAction::ModifyRequest { buffer, slots } => (buffer, slots),
                RhsAction::Request { buffer, spec } => {
                    let mut slots = Vec::new();
                    chunk_desc_slots(spec, &mut slots);
                    let e = usage.entry(buffer.clone()).or_default();
                    for s in slots {
                        if !e.contains(&s) {
                            e.push(s);
                        }
                    }
                    continue;
                }
                _ => continue,
            };
            let e = usage.entry(buffer.clone()).or_default();
            for (s, _) in slots {
                if is_slot_name(s) {
                    let s = s.to_uppercase();
                    if !e.contains(&s) {
                        e.push(s);
                    }
                }
            }
        }
    }
    let init = initial_buffer_slots(model);
    let new: std::collections::HashSet<String> =
        extended.iter().map(|s| s.to_uppercase()).collect();
    for (buffer, mut slots) in tested {
        let reqmod = usage.get(&buffer);
        let initial = init.get(&buffer);
        slots.retain(|slot| {
            !reqmod.is_some_and(|v| v.iter().any(|s| s == slot))
                && !initial.is_some_and(|v| v.iter().any(|s| s == slot))
        });
        // ACT-R's extended slots hash ahead of the pre-existing ones for the
        // fixtures this port covers; preserve source order within each group.
        slots.sort_by_key(|s| if new.contains(s) { 0 } else { 1 });
        for slot in slots {
            out.push(format!(
                "Productions test the {} slot in the {} buffer which is not requested or modified in any productions.",
                slot, buffer
            ));
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
                        auto_cmd: false,
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
        auto_cmd: false,
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
                    // `+` on the LHS never occurs in models; accept like `=`
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
    // An optional documentation string follows the name
    // (`production-parsing.lisp`: `(p name "doc" lhs ==> rhs)`), exactly as a
    // chunk type may carry one. It is not an LHS item.
    let (documentation, body) = match body.first() {
        Some(Sexp::Str(doc)) => (Some(doc.clone()), &body[1..]),
        _ => (None, body),
    };
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
        documentation,
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
    let mut parents = Vec::new();
    if let Sexp::List(parts) = name_desc {
        for part in &parts[1..] {
            if let Sexp::List(pair) = part {
                if pair.first().and_then(|x| x.as_sym()) == Some(":INCLUDE") {
                    if let Some(parent) = pair.get(1).and_then(|x| x.as_sym()) {
                        parents.push(parent.to_string());
                    }
                }
            }
        }
    }
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
        parents,
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
        Sexp::Sym(s) if s.starts_with(',') => resolve_def(s.trim_start_matches(','), defs),
        Sexp::List(v)
            if v.len() == 2 && v[0].as_sym() == Some("UNQUOTE") && v[1].as_sym().is_some() =>
        {
            resolve_def(v[1].as_sym().unwrap(), defs)
        }
        Sexp::List(v) => Sexp::List(v.iter().map(|x| subst_commas(x, defs)).collect()),
        other => other.clone(),
    }
}

fn resolve_def(name: &str, defs: &std::collections::HashMap<String, Sexp>) -> Sexp {
    match defs.get(&name.to_uppercase()) {
        Some(Sexp::List(v)) if v.first().and_then(|x| x.as_sym()) == Some("QUOTE") => {
            v.get(1).cloned().unwrap_or(Sexp::Sym("NIL".to_string()))
        }
        Some(v) => v.clone(),
        None => Sexp::Sym("NIL".to_string()),
    }
}

fn replace_query(e: &mut Sexp, query: &str) {
    match e {
        Sexp::Sym(s) if s.trim_start_matches(',').eq_ignore_ascii_case("*QUERY*") => {
            *e = Sexp::Sym(query.to_uppercase());
        }
        Sexp::List(v) => {
            if v.len() == 2
                && v[0].as_sym() == Some("UNQUOTE")
                && v[1]
                    .as_sym()
                    .is_some_and(|s| s.eq_ignore_ascii_case("*QUERY*"))
            {
                *e = Sexp::Sym(query.to_uppercase());
            } else {
                for x in v {
                    replace_query(x, query);
                }
            }
        }
        _ => {}
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

fn p_fct_body(e: Option<&Sexp>) -> Option<&Vec<Sexp>> {
    match e {
        Some(Sexp::List(list)) if list.first().and_then(|x| x.as_sym()) == Some("QUASIQUOTE") => {
            match list.get(1) {
                Some(Sexp::List(body)) => Some(body),
                _ => None,
            }
        }
        Some(Sexp::List(list)) => Some(list),
        _ => None,
    }
}

fn parse_model(
    form: &Sexp,
    defs: &std::collections::HashMap<String, Sexp>,
    required_extras: &[String],
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
        merge_chunks: Vec::new(),
        similarities: Vec::new(),
        base_levels: Vec::new(),
        set_all_base_levels: None,
        sdp_creation_time: None,
        sdp_references: None,
        required_extras: required_extras.to_vec(),
        initial_goal: None,
        goal_focuses: Vec::new(),
        productions: Vec::new(),
        scheduled_events: Vec::new(),
        model_outputs: Vec::new(),
        call_commands: Vec::new(),
        buffer_decls: Vec::new(),
        buffer_decl_slots: Vec::new(),
        visloc_default: Vec::new(),
        pfct: Vec::new(),
        no_reuse_buffers: Vec::new(),
        disabled_prods: Vec::new(),
        initial_buffer_sets: Vec::new(),
        initial_buffer_specs: Vec::new(),
        hand_start: None,
        audloc_default: Vec::new(),
        audloc_neg: Vec::new(),
        set_param_values: Vec::new(),
        window_origin: None,
        reset_warnings: Vec::new(),
        model_warnings: Vec::new(),
        goal_warnings: Vec::new(),
    };
    // `(spp <production> :key value ...)` calls, applied after the body so
    // they may appear before their production.
    let mut spp_calls: Vec<(String, Vec<(String, Sexp)>)> = Vec::new();
    // Chunks ACT-R would have created via `create-undefined-chunk` while
    // scanning productions (`chunks.lisp:850`); later productions see them.
    let mut created_chunks: Vec<String> = Vec::new();
    // Slots extended by a production parse (`extend-possible-slots`);
    // `check-between-production-style` treats them as new for ordering.
    let mut extended_slots: Vec<String> = Vec::new();
    // ACT-R emits the LHS-condition chunk warnings before the RHS-action ones,
    // even when the productions are interleaved (`tutorial-unit1-semantic-warnings`).
    let mut lhs_undefined: Vec<String> = Vec::new();
    let mut rhs_undefined: Vec<String> = Vec::new();
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
                // A one-key form is a parameter getter (`(sgp :seed)`), not a
                // setting; the trace-producing runtime handles it.
                if rest.len() == 1 {
                    continue;
                }
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
                    let name = define_chunk_table_entry(e, &mut model)?;
                    // `create-undefined-chunk` warns for bare slot values that
                    // are not known chunks (`framework/chunks.lisp:852`).
                    let mut known: Vec<String> = model
                        .defined_chunks
                        .iter()
                        .map(|c| c.name.clone())
                        .chain(model.chunks.iter().map(|c| c.name.clone()))
                        .chain(model.chunk_types.iter().map(|ct| ct.name.clone()))
                        .collect();
                    if let Some(c) = model.defined_chunks.iter().find(|c| c.name == name) {
                        for (_, v) in &c.slots {
                            if let Pattern::Sym(s) = v {
                                if is_undefined_chunk_value(s, &known) {
                                    model
                                        .model_warnings
                                        .push(format!("Creating chunk {s} with no slots"));
                                    known.push(s.clone());
                                }
                            }
                        }
                    }
                }
            }
            "MERGE-CHUNKS" | "MERGE-CHUNKS-FCT" => {
                let a = match items.get(1).and_then(Sexp::as_sym) {
                    Some(a) => a.to_uppercase(),
                    None => return structural("merge-chunks needs two chunk names"),
                };
                let b = match items.get(2).and_then(Sexp::as_sym) {
                    Some(b) => b.to_uppercase(),
                    None => return structural("merge-chunks needs two chunk names"),
                };
                model.merge_chunks.push((a, b));
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
            // `(sdp :creation-time S :references (...))`: global DM history
            // parameters. The model body is re-evaluated on reset, so these
            // are stored and applied after the DM histories are rebuilt.
            "SDP" => {
                let mut i = 1;
                while i + 1 < items.len() {
                    if let Some(key) = items[i].as_sym() {
                        match key.to_uppercase().as_str() {
                            ":CREATION-TIME" => {
                                if let Some(v) = sexp_f64(&items[i + 1]) {
                                    model.sdp_creation_time = Some(v);
                                }
                            }
                            ":REFERENCES" => {
                                if let Sexp::List(refs) = &items[i + 1] {
                                    model.sdp_references =
                                        Some(refs.iter().filter_map(sexp_f64).collect::<Vec<_>>());
                                }
                            }
                            _ => {}
                        }
                    }
                    i += 2;
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
                    let b = b.to_uppercase();
                    let t = t.to_uppercase();
                    let slots: Vec<String> = items[3..]
                        .iter()
                        .filter_map(|x| x.as_sym().map(|s| s.to_uppercase()))
                        .collect();
                    model.buffer_decls.push((b.clone(), t));
                    model.buffer_decl_slots.push((b, slots));
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
            "MODEL-OUTPUT" => {
                if let Some(Sexp::Str(text)) = items.get(1) {
                    model.model_outputs.push(text.clone());
                }
            }
            "CALL-ACT-R-COMMAND" => {
                let command = items
                    .get(1)
                    .and_then(|x| match x {
                        Sexp::Str(s) | Sexp::Sym(s) => Some(s.clone()),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        LoadError::Structural("call-act-r-command needs a command name".into())
                    })?;
                model.call_commands.push((command, items[2..].to_vec()));
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
                                let (lhs_warnings, rhs_warnings) =
                                    undefined_chunk_warnings(lhs, rhs, &mut known, &extra);
                                model.reset_warnings.extend(lhs_warnings.iter().cloned());
                                model.reset_warnings.extend(rhs_warnings.iter().cloned());
                                lhs_undefined.extend(lhs_warnings);
                                rhs_undefined.extend(rhs_warnings);
                            }
                        }
                        created_chunks = known
                            .into_iter()
                            .filter(|k| {
                                !model.chunks.iter().any(|c| &c.name == k)
                                    && !model.defined_chunks.iter().any(|c| &c.name == k)
                            })
                            .collect();
                        model.reset_warnings.extend(undefined_slot_warnings(
                            &p,
                            &model,
                            &mut extended_slots,
                        ));
                        push_production(&mut model, p);
                    }
                    Err(LoadError::RejectedProduction(w)) => model.reset_warnings.extend(w),
                    Err(e) => return Err(e),
                }
            }
            // `(p-fct \`(name ... ,*var* ...))`: a backquoted production with
            // model-file `defparameter` substitutions.
            "P-FCT" => {
                let raw = p_fct_body(items.get(1)).or_else(|| p_fct_body(items.get(2)));
                if let Some(list) = raw {
                    model.pfct.push(list.clone());
                    let sub = substitute_commas(list, defs);
                    let mut form = vec![Sexp::Sym("P".to_string())];
                    form.extend(sub);
                    let extra: Vec<String> =
                        model.buffer_decls.iter().map(|(b, _)| b.clone()).collect();
                    match parse_production(&Sexp::List(form), &extra) {
                        Ok(p) => {
                            model.reset_warnings.extend(undefined_slot_warnings(
                                &p,
                                &model,
                                &mut extended_slots,
                            ));
                            push_production(&mut model, p);
                        }
                        Err(LoadError::RejectedProduction(w)) => model.reset_warnings.extend(w),
                        Err(e) => return Err(e),
                    }
                }
            }
            // `(install-device (open-exp-window …))`: task device setup (the
            // spec's device sidecar models the observable effects). The
            // nested `open-exp-window`'s `:x`/`:y` give the window origin,
            // which offsets visicon coords and is where the virtual mouse
            // cursor starts.
            "INSTALL-DEVICE" => {
                if let Some(Sexp::List(win)) = items.get(1) {
                    if win.first().and_then(|x| x.as_sym()) == Some("OPEN-EXP-WINDOW") {
                        // `(open-exp-window "name" :visible v :x X :y Y …)`: the
                        // name is the first argument, so scan for the keywords
                        // rather than stepping in pairs from the start.
                        let (mut x, mut y) = (None, None);
                        for i in 0..win.len() {
                            match win[i].as_sym() {
                                Some(":X") => {
                                    if let Some(Sexp::Int(v)) = win.get(i + 1) {
                                        x = Some(*v);
                                    }
                                }
                                Some(":Y") => {
                                    if let Some(Sexp::Int(v)) = win.get(i + 1) {
                                        y = Some(*v);
                                    }
                                }
                                _ => {}
                            }
                        }
                        model.window_origin = Some((x.unwrap_or(300), y.unwrap_or(300)));
                    }
                }
            }
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
                if let Some(b) = sym(items.get(1)) {
                    if let Some(c) = sym(items.get(2)) {
                        model.initial_buffer_sets.push((b.to_uppercase(), c));
                    } else {
                        let spec = match items.get(2) {
                            Some(Sexp::List(q))
                                if q.len() == 2 && q[0].as_sym() == Some("QUOTE") =>
                            {
                                match &q[1] {
                                    Sexp::List(v) => Some(v.as_slice()),
                                    _ => None,
                                }
                            }
                            Some(Sexp::List(v)) => Some(v.as_slice()),
                            _ => None,
                        };
                        if let Some(spec) = spec {
                            if let Ok(desc) = parse_chunk_desc(spec) {
                                model.initial_buffer_specs.push((b.to_uppercase(), desc));
                            }
                        }
                    }
                }
            }
            // `(start-hand-at-mouse)` / `(start-hand-at-keypad)`: place the
            // right hand on that device before running
            // (`other-files/mouse.lisp:554`, `other-files/keyboard.lisp:766`).
            // The device install is a no-op here (the spec's device sidecar
            // models observable effects); the position is recorded for the
            // motor module.
            "START-HAND-AT-MOUSE" => model.hand_start = Some("MOUSE".to_string()),
            "START-HAND-AT-KEYPAD" => model.hand_start = Some("KEYPAD".to_string()),
            // `(set-cursor-position x y)`: the virtual cursor's screen position.
            // The engine's mouse starts at the window origin, which is what
            // the AGI tasks set here; the position is not observable in the
            // trace until a move, so no state is needed yet.
            "SET-CURSOR-POSITION" => {}
            // `(set-audloc-default slot value ...)`: default constraints for
            // an unconstrained aural-location request.
            "SET-AUDLOC-DEFAULT" => {
                let mut i = 1;
                while i + 1 < items.len() {
                    let negated = items[i].as_sym() == Some("-");
                    if negated {
                        i += 1;
                    }
                    let Some(slot) = items.get(i).and_then(|x| x.as_sym()) else {
                        break;
                    };
                    let value = items[i + 1].clone();
                    if negated {
                        model.audloc_neg.push((slot.to_uppercase(), value));
                    } else {
                        model.audloc_default.push((slot.to_uppercase(), value));
                    }
                    i += 2;
                }
            }
            // `(set-parameter-value :module-param value)`: set a module
            // parameter. The model body is re-evaluated on every reset, so a
            // task-file variable here is re-read each time.
            "SET-PARAMETER-VALUE" => {
                if let (Some(k), Some(v)) = (items.get(1).and_then(|x| x.as_sym()), items.get(2)) {
                    // Resolve a `*var*` reference to its model-file value
                    // when known.
                    let v = match v {
                        Sexp::Sym(s) => defs
                            .get(&s.to_uppercase())
                            .cloned()
                            .unwrap_or_else(|| v.clone()),
                        _ => v.clone(),
                    };
                    model.set_param_values.push((k.to_uppercase(), v));
                }
            }
            // Model-body `(when *flag* (sgp ...) (setf ...))`: the pcomp
            // models use this to seed only on a file's first load. The engine
            // owns the seed through the `:seed` parameter, so fold the active
            // `sgp` forms into the model params; the `setf` is bookkeeping.
            "WHEN" => {
                let active = match items.get(1) {
                    Some(Sexp::Sym(name)) => defs
                        .get(&name.to_uppercase())
                        .map(|v| !matches!(v, Sexp::Sym(s) if s.eq_ignore_ascii_case("NIL")))
                        .unwrap_or(false),
                    Some(_) => true,
                    None => false,
                };
                if active {
                    for sub in &items[2..] {
                        let Sexp::List(parts) = sub else { continue };
                        if parts.first().and_then(Sexp::as_sym) != Some("SGP") {
                            continue;
                        }
                        let rest = &parts[1..];
                        if rest.len() == 1 {
                            continue;
                        }
                        if rest.len() % 2 != 0 {
                            return structural("sgp needs even key/value count");
                        }
                        for kv in rest.as_chunks::<2>().0 {
                            let key = match &kv[0] {
                                Sexp::Sym(k) => k.clone(),
                                got => {
                                    return structural(format!("sgp key must be a symbol: {got:?}"))
                                }
                            };
                            model.params.push((key, subst_bare(&kv[1], defs)));
                        }
                    }
                }
            }
            // Task models may define local helper functions inside the
            // `define-model` body. Their productions are loaded separately
            // by the driver, so the loader only needs to ignore the defun.
            "DEFUN" => {}
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
                    // `learn-parameters` (`utility-and-reward-1.lisp:470`)
                    // schedules a reward only when it is non-nil; a number
                    // updates utilities, any other value clears the history.
                    if let Some(r) = sexp_f64(v) {
                        prod.reward = Some(Reward::Num(r as f32));
                    } else if let Sexp::Sym(s) = v {
                        if s != "NIL" {
                            prod.reward = Some(Reward::Other(s.clone()));
                        }
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
    model.apply_chunk_defaults();
    validate_goal_focus(&mut model);
    if !lhs_undefined.is_empty() || !rhs_undefined.is_empty() {
        let mut ordered = lhs_undefined;
        ordered.extend(rhs_undefined);
        let mut ordered = ordered.into_iter();
        for warning in &mut model.reset_warnings {
            if warning.starts_with("Creating chunk ") && warning.ends_with(" with no slots") {
                if let Some(next) = ordered.next() {
                    *warning = next;
                }
            }
        }
    }
    model
        .reset_warnings
        .extend(style_warnings(&model, &extended_slots));
    // A production that names a buffer's variable (`=buffer`) anywhere makes
    // that buffer non-reusable (`support/production-parsing.lisp:482`), on top
    // of any model-body `(buffer-requires-copies …)`.
    for p in &model.productions {
        for b in var_referenced_buffers(p) {
            if !model.no_reuse_buffers.contains(&b) {
                model.no_reuse_buffers.push(b);
            }
        }
    }
    Ok(model)
}

/// Add a production, replacing any earlier one of the same name
/// (`redefine-production`): ACT-R keeps the last definition and warns at the
/// next reset (`#|Warning: Production F already exists …|#`). The message is
/// the *inner* text; `Engine::emit_warning` adds the `#|Warning: … |#` wrap.
fn push_production(model: &mut Model, p: Production) {
    if let Some(pos) = model.productions.iter().position(|q| q.name == p.name) {
        model.productions.remove(pos);
        model.reset_warnings.push(format!(
            "Production {} already exists and it is being redefined.",
            p.name
        ));
    }
    model.productions.push(p);
}

/// Load every `define-model` in a `.lisp` model file (these files hold one).
/// `(clear-all)` is acknowledged and skipped; anything else top-level errors.
pub fn load_models(path: &Path) -> Result<Vec<Model>, LoadError> {
    let src = fs::read_to_string(path).map_err(LoadError::Io)?;
    load_models_from_str(&src)
}

/// Parse every `define-model` in already-read Lisp source — the filesystem-free
/// twin of [`load_models`], for embedders (the wasm build) that hold the model
/// text rather than a path. `(clear-all)` is acknowledged and skipped; anything
/// else top-level errors.
fn collect_required_extras(form: &Sexp, out: &mut Vec<String>) {
    let Sexp::List(items) = form else {
        return;
    };
    if items.first().and_then(|x| x.as_sym()) == Some("REQUIRE-EXTRA") {
        if let Some(Sexp::Str(n)) = items.get(1) {
            out.push(n.clone());
        }
        return;
    }
    for item in items {
        collect_required_extras(item, out);
    }
}

/// Whether a model/task source has a top-level `(clear-all)` form. ACT-R
/// executes it while loading, so a second load replaces the existing models.
pub fn source_clears_all(src: &str) -> bool {
    parse_all(src)
        .map(|forms| {
            forms.iter().any(|form| {
                matches!(
                    form,
                    Sexp::List(items)
                        if items.first().and_then(|x| x.as_sym()) == Some("CLEAR-ALL")
                )
            })
        })
        .unwrap_or(false)
}

pub fn load_models_from_str(src: &str) -> Result<Vec<Model>, LoadError> {
    let mut models = Vec::new();
    let mut required_extras: Vec<String> = Vec::new();
    let mut defs: std::collections::HashMap<String, Sexp> = std::collections::HashMap::new();
    for form in parse_all(src)? {
        match &form {
            Sexp::List(items) => match items.first() {
                Some(Sexp::Sym(h)) if h == "CLEAR-ALL" => {}
                // `(require-extra "name")` loads an extra Lisp module; record
                // it so the engine can tell which behaviors are expected.
                // Task files wrap it in `suppress-warnings`, `let` output
                // redirection, or `progn` to hide Lisp compiler chatter.
                Some(Sexp::Sym(h))
                    if matches!(
                        h.as_str(),
                        "REQUIRE-EXTRA" | "SUPPRESS-WARNINGS" | "LET" | "PROGN"
                    ) =>
                {
                    collect_required_extras(&form, &mut required_extras);
                }
                Some(Sexp::Sym(h)) if h == "DEFINE-MODEL" => {
                    models.push(parse_model(&form, &defs, &required_extras)?)
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

    fn tutorial_model(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("actr7.x")
            .join("tutorial")
            .join("unit1")
            .join(name)
    }

    fn the_only(path: &str) -> Model {
        let models = load_models(&tutorial_model(path)).unwrap();
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
                    parents: vec![],
                    slots: vec!["NUMBER".into(), "NEXT".into()],
                    defaults: vec![],
                },
                ChunkType {
                    name: "COUNT-FROM".into(),
                    parents: vec![],
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
    fn p_fct_quasiquote_loads() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("actr7.x")
            .join("extras")
            .join("extended-motor-actions")
            .join("test-model.lisp");
        let m = load_models(&path).unwrap().into_iter().next().unwrap();
        assert!(
            m.productions
                .iter()
                .any(|p| p.name == "TEST1-SECOND-PRESS-SAME"),
            "{:?}",
            m.productions
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn model_body_command_and_task_globals() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("actr7.x");
        let model_path = root
            .join("tutorial")
            .join("unit8")
            .join("categorize-model.lisp");
        let model = load_models(&model_path)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(model.call_commands.len(), 1);
        assert_eq!(model.call_commands[0].0, "create-example-memories");
        assert!(model
            .initial_buffer_specs
            .iter()
            .any(|(buffer, _)| buffer == "IMAGINAL"));

        let task =
            std::fs::read_to_string(root.join("tutorial").join("lisp").join("categorize.lisp"))
                .unwrap();
        let globals = parse_task_globals(&task);
        assert_eq!(
            globals.get("*SLOTS*"),
            Some(&Sexp::List(vec![
                Sexp::Sym("EH".into()),
                Sexp::Sym("ES".into()),
                Sexp::Sym("NL".into()),
                Sexp::Sym("MH".into()),
            ]))
        );
    }

    #[test]
    fn source_clear_all_is_detected_at_top_level_only() {
        assert!(source_clears_all("(clear-all)\n(define-model x)"));
        assert!(!source_clears_all("(define-model x (clear-all))"));
        assert!(!source_clears_all(";; (clear-all)\n(define-model x)"));
    }

    #[test]
    fn delete_model_forms_are_read_from_code_files() {
        assert_eq!(
            parse_model_deletions("(delete-model mp-c)\n(delete-model \"MP-D\")"),
            vec![Some("MP-C".to_string()), Some("MP-D".to_string())]
        );
        assert_eq!(parse_model_deletions("(delete-model)"), vec![None]);
        assert!(parse_model_deletions("(define-model x (delete-model y))").is_empty());
    }

    #[test]
    fn required_extra_wrapped_in_let_enables_extended_motor_slots() {
        let src = r#"
(let ((*standard-output* (make-broadcast-stream))
      (*error-output* (make-broadcast-stream)))
  (require-extra "extended-motor-actions"))

(define-model probe
  (chunk-type goal state)
  (add-dm (start isa goal state 0))
  (p type-right
    =goal>
      isa goal
      state 0
    ?manual-right>
      ring free
    ==>
    +manual>
      isa type-key
      key "l"
    =goal>
      state 1))
"#;
        let models = load_models_from_str(src).unwrap();
        assert_eq!(
            models[0].required_extras,
            vec!["extended-motor-actions".to_string()]
        );
        assert!(
            !models[0]
                .reset_warnings
                .iter()
                .any(|w| w.contains("previously undefined slot")),
            "{:?}",
            models[0].reset_warnings
        );
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

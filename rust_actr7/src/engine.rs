//! M2b discrete-event engine: scheduler, buffers, procedural cycle,
//! stub-free declarative retrieval for fully-specified requests.
//!
//! Execution model (all from `actr7.x` sources, see GOTCHAS.md):
//! - Integer millisecond clock; FIFO queue ordered by (time, priority, seq).
//!   Priorities: `:max` first, numeric descending, `:min` last; ties FIFO
//!   (`compare-events`, `framework/meta-process.lisp:659`).
//! - Conflict-resolution runs after reset, after every firing (`:min`),
//!   and after any buffer change when a previous CR found no match
//!   (`schedule-event-after-change`, procedural.lisp:2329).
//! - Selection schedules buffer-reads (LHS order) plus firing at
//!   `+dat` (default 50 ms). Firing runs `!output!` binds, then RHS
//!   action events in order: modify (100), request (50), clears (10),
//!   then CR (`:min`). Implicit harvest clears are appended at build
//!   (`support/production-parsing.lisp:1323`).
//! - Retrieval: exact slot match, base-level learning (`:bll`) via
//!   tracked reference histories, activation/utility noise. Still out:
//!   spreading (`:mas`), partial matching (`:mp`), utility learning.
//!
//! Out of scope (error loudly): multi-model files beyond the last model,
//! `!eval!`/`!bind!`, request modifiers, non-fully-specified requests with
//! `-` slots (rejected at load), queries other than `buffer failure` and
//! `state free/busy`, chunk extension events, `:ignore-buffers`.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::model::{
    Chunk, ChunkDesc, LhsItem, Model, Pattern, Production, RhsAction, SlotOp,
};
use crate::rand::Mt;
use crate::sexpr::Sexp;
use crate::trace::{format_line, Detail, Out, MODULE_WIDTH};

#[derive(Debug)]
pub enum EngineError {
    NotImplemented(String),
    Model(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::NotImplemented(m) => write!(f, "not implemented: {m}"),
            EngineError::Model(m) => write!(f, "bad model: {m}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<crate::model::LoadError> for EngineError {
    fn from(e: crate::model::LoadError) -> Self {
        EngineError::Model(e.to_string())
    }
}

fn unimpl(what: impl Into<String>) -> EngineError {
    EngineError::NotImplemented(what.into())
}

fn bad(what: impl Into<String>) -> EngineError {
    EngineError::Model(what.into())
}

// ---------------------------------------------------------------- values

#[derive(Debug, Clone)]
enum Value {
    Sym(String),
    Int(i64),
    Num(f64),
    Str(String),
    Nil,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        use Value::*;
        match (self, other) {
            (Int(a), Int(b)) => a == b,
            (Num(a), Num(b)) => a == b,
            // Integer and float slots compare numerically so a `3` pattern
            // still matches a `3.0` stored value and vice versa.
            (Int(a), Num(b)) | (Num(b), Int(a)) => (*a as f64) == *b,
            (Sym(a), Sym(b)) => a == b,
            (Str(a), Str(b)) => a == b,
            (Nil, Nil) => true,
            _ => false,
        }
    }
}

impl Value {
    fn from_pattern(p: &Pattern) -> Result<Value, EngineError> {
        match p {
            Pattern::Var(v) => Err(bad(format!("unbound variable ={v} in chunk store"))),
            Pattern::Sym(s) => Ok(Value::Sym(s.clone())),
            Pattern::Int(i) => Ok(Value::Int(*i)),
            Pattern::Num(x) => Ok(Value::Num(*x)),
            Pattern::Str(s) => Ok(Value::Str(s.clone())),
            Pattern::Nil => Ok(Value::Nil),
        }
    }

    /// Lisp `~S` printing, as used by `!output!` and SET-BUFFER-CHUNK specs.
    fn show(&self) -> String {
        match self {
            Value::Sym(s) => s.clone(),
            Value::Int(i) => i.to_string(),
            // ACT-R numbers are single-floats; print the shortest
            // round-tripping decimal the way SBCL `~S`/`~A` does
            // (exponent form outside [1e-3, 1e7)).
            Value::Num(x) => lisp_s_f32(*x as f32),
            Value::Str(s) => format!("{s:?}"),
            Value::Nil => "NIL".to_string(),
        }
    }
}

type Bindings = HashMap<String, Value>;

/// `(buffer, slot, requested, actual, similarity)` recorded by procedural
/// partial matching (`:ppm`) for one mismatched LHS equality test.
type PpmMismatch = (String, String, Value, Value, f32);

impl Engine {
    /// Substitute a RHS pattern: bound variables first, then
    /// `=buffer` whole-chunk references (the buffer's chunk name),
    /// else literals. Unbound anything else errors.
    fn subst(&self, p: &Pattern, b: &Bindings) -> Result<Value, EngineError> {
        match p {
            Pattern::Var(v) => {
                if let Some(val) = b.get(v) {
                    return Ok(val.clone());
                }
                if let Some(chunk) = self.buffers.get(v).and_then(|buf| buf.chunk.as_ref()) {
                    if let Some(name) = &chunk.name {
                        return Ok(Value::Sym(name.clone()));
                    }
                }
                Err(bad(format!("unbound variable ={v} on RHS")))
            }
            other => Value::from_pattern(other),
        }
    }
}

// ---------------------------------------------------------------- config

/// Activation trace level (`:act`). `dm-act-level` (`declarative-memory.lisp:1042`):
/// `low` shows for any truthy act, `medium` unless `:act low`, `high` only
/// for `:act high`/`t`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Act {
    T,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
struct Config {
    detail: Detail,
    esc: bool,
    lf_ms: i64,
    le: f32,
    rt: f32,
    blc: f32,
    bll: Option<f32>,
    ol: Ol,
    act_scale: f32,
    ans: Option<f32>,
    egs: f32,
    ul: bool,
    alpha: f32,
    iu: f32,
    mp: Option<f32>,
    ms: f32,
    md: f32,
    /// `:ppm` — procedural partial matching penalty factor. When set, LHS
    /// equality tests may mismatch when the value similarity exceeds `:md`,
    /// and utility is adjusted by `ppm * Σ similarities`.
    ppm: Option<f32>,
    /// Production-compilation `:epl` (enable production learning).
    epl: bool,
    /// Production-compilation `:pct` (trace).
    pct: bool,
    /// Production-compilation `:tt` (max seconds between composed fires).
    tt_ms: i64,
    /// Production-compilation `:cia` (compile indirect actions).
    cia: bool,
    /// Production-compilation `:rir` (retain indirect retrievals).
    rir: bool,
    /// Maximum associative strength (`:mas`). `Some` enables spreading.
    mas: Option<f32>,
    /// `:nsji` — do not clamp negative Sji at 0.
    nsji: bool,
    /// Goal buffer source spread (`:ga`, default 0).
    ga: f32,
    /// Imaginal buffer source spread (`:imaginal-activation`, default 1.0).
    imaginal_activation: f32,
    /// Activation trace (`:act`); see [`Act`].
    act: Option<Act>,
    /// Conflict-resolution trace (`:crt`).
    crt: bool,
    /// Conflict-set trace (`:cst`): print each matching production's
    /// instantiation and parameters at conflict resolution.
    cst: bool,
    /// Blending extra enabled (`require-extra "blending"`): installs the
    /// `BLENDING` buffer and its request/query handling.
    blending: bool,
    /// `:blt` — blending module trace.
    blt: bool,
    /// `:tmp` — blending temperature (overrides `:ans`-derived default).
    tmp_: Option<f32>,
    /// `:min-bl` — minimum base-level for a chunk to be blended.
    min_bl: Option<f32>,
    /// `:blend-all-slots` — blend requested slots too.
    blend_all_slots: bool,
    /// Retrieval finsts: how many / how long (`:recently-retrieved`).
    num_finsts: usize,
    finst_span_ms: i64,
    dat_ms: i64,
    attention_ms: i64,
    imaginal_delay_ms: i64,
    feat_prep_ms: i64,
    init_ms: i64,
    burst_ms: i64,
    peck_fitts: f32,
    min_fitts_ms: i64,
    rhst: bool,
    er: bool,
    do_not_query: Vec<String>,
    do_not_harvest: Vec<String>,
    /// Module column width in trace lines (`max-module-name-length`). The
    /// normal value is 22; test-model stubs may override it (GOTCHAS M10).
    module_width: usize,
}

/// One scripted device action: show text at a time and position.
/// Mirrors what a task file does with `open-exp-window` /
/// `add-text-to-exp-window` after install.
#[derive(Debug, Clone)]
pub struct DeviceText {
    pub at_ms: i64,
    pub text: String,
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, Default)]
pub struct DeviceSetup {
    pub texts: Vec<DeviceText>,
    /// Window clears at ms (clear-exp-window): all features removed.
    pub clears: Vec<i64>,
    /// Run-full-time dummy: keep running to this ms, then time-limit stop.
    pub end: Option<i64>,
    /// Task rewards `(ms, value)`, mirroring scheduled `trigger-reward`
    /// calls (the invisible wrapper is skipped; propagate is scheduled
    /// directly with identical timing).
    pub rewards: Vec<(i64, f32)>,
    /// Chunk similarities `(a, b, value)`, mirrored from the task
    /// file's `set-similarities` (which must run post-reset: reset
    /// rebuilds chunk structs, wiping table state set at load).
    pub sims: Vec<(String, String, f32)>,
}

/// A visicon feature: what the device shows and vision can find.
#[derive(Debug, Clone)]
struct Feature {
    /// VISUAL-LOCATION{i} at proc-display time.
    name: String,
    kind: String,
    value: String,
    x: i64,
    y: i64,
    tstamp_ms: i64,
    attended: bool,
}

/// Virtual keyboard entry: hand geometry for prep/fitss math.
/// M5a seeds Q; digits etc. join with device-task goldens.
#[derive(Debug, Clone)]
struct KeySpec {
    hand: &'static str,
    finger: &'static str,
    r: f32,
    theta: f32,
}

fn key_table() -> HashMap<String, KeySpec> {
    HashMap::from([
        (
            "Q".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "PINKIE",
                r: 1.0,
                theta: -1.57,
            },
        ),
        (
            "0".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "PINKIE",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "1".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "PINKIE",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "2".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "RING",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "3".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "MIDDLE",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "4".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "INDEX",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "5".to_string(),
            KeySpec {
                hand: "LEFT",
                finger: "INDEX",
                r: 2.24,
                theta: -1.11,
            },
        ),
        (
            "6".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 2.24,
                theta: -2.03,
            },
        ),
        (
            "7".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "INDEX",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "8".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "MIDDLE",
                r: 2.0,
                theta: -1.57,
            },
        ),
        (
            "9".to_string(),
            KeySpec {
                hand: "RIGHT",
                finger: "RING",
                r: 2.0,
                theta: -1.57,
            },
        ),
    ])
}

fn sgp_num(params: &[(String, Sexp)], key: &str, default: f64) -> Result<f64, EngineError> {
    match params.iter().find(|(k, _)| k == key) {
        None => Ok(default),
        Some((_, Sexp::Float(x))) => Ok(*x),
        Some((_, Sexp::Int(i))) => Ok(*i as f64),
        Some((_, got)) => Err(bad(format!("{key} must be a number, got {got:?}"))),
    }
}

fn sgp_single(params: &[(String, Sexp)], key: &str) -> Result<Option<f32>, EngineError> {
    match params.iter().find(|(k, _)| k == key) {
        None => Ok(None),
        // Source literals read as single-float in Lisp; narrow the same way.
        Some((_, Sexp::Float(x))) => Ok(Some(*x as f32)),
        Some((_, Sexp::Int(i))) => Ok(Some(*i as f32)),
        Some((_, Sexp::Sym(s))) if s == "NIL" => Ok(None),
        Some((_, got)) => Err(bad(format!("{key} must be a number or nil, got {got:?}"))),
    }
}
fn sgp_bool(params: &[(String, Sexp)], key: &str) -> bool {
    match params.iter().find(|(k, _)| k == key) {
        // `t` (any non-nil symbol) is true; explicit NIL is false.
        Some((_, Sexp::Sym(s))) => s != "NIL",
        Some(_) => true,
        None => false,
    }
}

impl Config {
    fn from_model(m: &Model) -> Result<Config, EngineError> {
        // Still beyond reach: utility learning hooks, permanent activation
        // sources. (`:mas` spreading is implemented below.)
        for key in [":PAS"] {
            if let Some((_, v)) = m.params.iter().find(|(k, _)| k == key) {
                if !matches!(v, Sexp::Sym(s) if s == "NIL") {
                    return Err(unimpl(format!("subsymbolic parameter {key}")));
                }
            }
        }
        let detail = match m.params.iter().find(|(k, _)| k == ":TRACE-DETAIL") {
            None => Detail::Medium,
            Some((_, Sexp::Sym(s))) => match s.as_str() {
                "HIGH" => Detail::High,
                "MEDIUM" => Detail::Medium,
                "LOW" => Detail::Low,
                got => return Err(bad(format!("bad :trace-detail {got}"))),
            },
            Some((_, got)) => return Err(bad(format!("bad :trace-detail {got:?}"))),
        };
        // `:lf` is stored as integer ms: (round (* x 1000)).
        let lf_ms = (sgp_num(&m.params, ":LF", 1.0)? * 1000.0).round() as i64;
        // Buffers the owning modules exempt from implicit processing.
        // Defaults come from the modules themselves (goal.lisp:233,
        // vision.lisp:3124, declarative secondary-reset, temporal):
        // goal/retrieval/visual-location/temporal skip implicit
        // state-free queries; goal/temporal skip harvest clears.
        // (`:do-not-query`/`:do-not-harvest` extend or `:remove`.)
        let mut do_not_query = vec!["GOAL", "RETRIEVAL", "VISUAL-LOCATION", "TEMPORAL"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut do_not_harvest = vec!["GOAL", "TEMPORAL"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        for (key, target) in [
            (":DO-NOT-QUERY", &mut do_not_query),
            (":DO-NOT-HARVEST", &mut do_not_harvest),
        ] {
            if let Some((_, v)) = m.params.iter().find(|(k, _)| k == key) {
                apply_buffer_list(target, v)?;
            }
        }
        if sgp_bool(&m.params, ":RANDOMIZE-TIME") {
            return Err(unimpl(":randomize-time"));
        }
        if sgp_bool(&m.params, ":AUTO-ATTEND") {
            return Err(unimpl(":auto-attend"));
        }
        if sgp_bool(&m.params, ":VIDT") {
            return Err(unimpl(":vidt variable imaginal delay"));
        }
        match m.params.iter().find(|(k, _)| k == ":OPTIMIZE-VISUAL") {
            Some((_, Sexp::Sym(s))) if s == "NIL" => {
                return Err(unimpl(":optimize-visual nil"));
            }
            _ => {}
        }
        let secs_to_ms = |key: &str, default: f64| -> Result<i64, EngineError> {
            Ok((sgp_num(&m.params, key, default)? * 1000.0).round() as i64)
        };
        let bll = sgp_single(&m.params, ":BLL")?.filter(|b| *b > 0.0);
        let ol = match m.params.iter().find(|(k, _)| k == ":OL") {
            // Unset :ol inherits central-parameters' default, which is T
            // (`:owner nil`; central default-value t).
            None => Ol::On,
            Some((_, Sexp::Sym(s))) if s == "NIL" => Ol::Off,
            Some((_, Sexp::Sym(s))) if s == "T" => Ol::On,
            Some((_, Sexp::Int(i))) => Ol::Num(*i as f64),
            Some((_, Sexp::Float(x))) => Ol::Num(*x),
            Some((_, got)) => return Err(bad(format!("bad :ol {got:?}"))),
        };
        // (log (expt 1/1000 (- bll))): ratio coerced to single, as read.
        let act_scale = match bll {
            Some(b) => (0.001f32).powf(-b).ln(),
            None => 0.0,
        };
        Ok(Config {
            detail,
            esc: sgp_bool(&m.params, ":ESC"),
            lf_ms,
            le: sgp_single(&m.params, ":LE")?.unwrap_or(1.0),
            rt: sgp_single(&m.params, ":RT")?.unwrap_or(0.0),
            blc: sgp_single(&m.params, ":BLC")?.unwrap_or(0.0),
            ans: sgp_single(&m.params, ":ANS")?.filter(|s| *s > 0.0),
            ul: sgp_bool(&m.params, ":UL"),
            alpha: sgp_single(&m.params, ":ALPHA")?.unwrap_or(0.2),
            iu: sgp_single(&m.params, ":IU")?.unwrap_or(0.0),
            mp: sgp_single(&m.params, ":MP")?,
            ms: sgp_single(&m.params, ":MS")?.unwrap_or(0.0),
            md: sgp_single(&m.params, ":MD")?.unwrap_or(-1.0),
            ppm: sgp_single(&m.params, ":PPM")?,
            epl: sgp_bool(&m.params, ":EPL"),
            pct: sgp_bool(&m.params, ":PCT"),
            tt_ms: (sgp_num(&m.params, ":TT", 2.0)? * 1000.0).round() as i64,
            cia: sgp_bool(&m.params, ":CIA"),
            rir: sgp_bool(&m.params, ":RIR"),
            mas: sgp_single(&m.params, ":MAS")?,
            nsji: sgp_bool(&m.params, ":NSJI"),
            ga: sgp_single(&m.params, ":GA")?.unwrap_or(0.0),
            imaginal_activation: sgp_single(&m.params, ":IMAGINAL-ACTIVATION")?.unwrap_or(1.0),
            act: match m.params.iter().find(|(k, _)| k == ":ACT") {
                None => None,
                Some((_, Sexp::Sym(s))) if s == "NIL" => None,
                Some((_, Sexp::Sym(s))) => Some(match s.as_str() {
                    "T" => Act::T,
                    "HIGH" => Act::High,
                    "MEDIUM" => Act::Medium,
                    "LOW" => Act::Low,
                    got => return Err(bad(format!("bad :act {got}"))),
                }),
                Some((_, got)) => return Err(bad(format!("bad :act {got:?}"))),
            },
            crt: sgp_bool(&m.params, ":CRT"),
            cst: sgp_bool(&m.params, ":CST"),
            blending: m
                .required_extras
                .iter()
                .any(|e| e.eq_ignore_ascii_case("blending")),
            blt: sgp_bool(&m.params, ":BLT"),
            tmp_: sgp_single(&m.params, ":TMP")?,
            min_bl: sgp_single(&m.params, ":MIN-BL")?,
            blend_all_slots: sgp_bool(&m.params, ":BLEND-ALL-SLOTS"),
            num_finsts: sgp_num(&m.params, ":DECLARATIVE-NUM-FINSTS", 4.0)? as usize,
            finst_span_ms: (sgp_num(&m.params, ":DECLARATIVE-FINST-SPAN", 3.0)? * 1000.0)
                .round() as i64,
            bll,
            ol,
            act_scale,
            egs: sgp_single(&m.params, ":EGS")?.unwrap_or(0.0),
            dat_ms: (sgp_num(&m.params, ":DAT", 0.05)? * 1000.0).round() as i64,
            attention_ms: secs_to_ms(":VISUAL-ATTENTION-LATENCY", 0.085)?,
            imaginal_delay_ms: secs_to_ms(":IMAGINAL-DELAY", 0.2)?,
            feat_prep_ms: secs_to_ms(":MOTOR-FEATURE-PREP-TIME", 0.05)?,
            init_ms: secs_to_ms(":MOTOR-INITIATION-TIME", 0.05)?,
            burst_ms: secs_to_ms(":MOTOR-BURST-TIME", 0.05)?,
            peck_fitts: sgp_single(&m.params, ":PECK-FITTS-COEFF")?.unwrap_or(0.075),
            min_fitts_ms: secs_to_ms(":MIN-FITTS-TIME", 0.1)?,
            rhst: match m.params.iter().find(|(k, _)| k == ":RHST") {
                None => true,
                Some((_, Sexp::Sym(s))) => s != "NIL",
                Some(_) => true,
            },
            er: sgp_bool(&m.params, ":ER"),
            do_not_query,
            do_not_harvest,
            module_width: MODULE_WIDTH,
        })
    }
}

/// Extend (or `:remove` from) an implicit-processing exemption list.
fn apply_buffer_list(target: &mut Vec<String>, v: &Sexp) -> Result<(), EngineError> {
    match v {
        Sexp::Sym(s) => {
            if !target.contains(s) {
                target.push(s.clone());
            }
            Ok(())
        }
        Sexp::List(items) => {
            for it in items {
                match it {
                    Sexp::Sym(s) => {
                        if !target.contains(s) {
                            target.push(s.clone());
                        }
                    }
                    Sexp::List(rm) if matches!(rm.as_slice(), [Sexp::Sym(k), _] if k == ":REMOVE") => {
                        if let [_, Sexp::Sym(x)] = rm.as_slice() {
                            target.retain(|b| b != x);
                        }
                    }
                    got => {
                        return Err(bad(format!("bad buffer list entry: {got:?}")));
                    }
                }
            }
            Ok(())
        }
        got => Err(bad(format!("bad buffer list: {got:?}"))),
    }
}

// ---------------------------------------------------------------- chunks

#[derive(Debug, Clone)]
struct ChunkInst {
    name: Option<String>,
    ctype: String,
    slots: Vec<(String, Value)>,
    /// Print slots-only (`(PROBE "bank")`) instead of with ISA.
    /// Set for request-built specs (chunk-spec-to-chunk-def drops ISA
    /// and type); goal-focus lists print with ISA.
    bare: bool,
}

impl ChunkInst {
    /// Missing slots read as NIL (matches `count nil`-style tests).
    fn get(&self, slot: &str) -> Value {
        self.slots
            .iter()
            .find(|(s, _)| s == slot)
            .map(|(_, v)| v.clone())
            .unwrap_or(Value::Nil)
    }
}

#[derive(Debug, Clone)]
struct Buffer {
    chunk: Option<ChunkInst>,
    failed: bool,
    busy: bool,
}

/// Base-level history per DM chunk, aligned with `Engine.dm` by index:
/// creation time, reference times, and total reference count
/// (`merge-reference-list/count`; the list truncates under numeric
/// `:ol` but the count never does).
#[derive(Debug, Clone)]
struct History {
    creation_ms: i64,
    refs: Vec<i64>,
    count: usize,
    /// Fixed `chunk-base-level` set by `set-base-levels` when `:bll` is nil
    /// (`set-bl` else-branch, dm-commands.lisp:623). Used verbatim as the
    /// base level; `None` falls back to `:blc`.
    fixed_base: Option<f32>,
}

impl Buffer {
    fn empty() -> Buffer {
        Buffer {
            chunk: None,
            failed: false,
            busy: false,
        }
    }
}

// ---------------------------------------------------------------- events

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prio {
    Min,
    Num(i64),
    Max,
}

impl Ord for Prio {
    fn cmp(&self, other: &Self) -> Ordering {
        use Prio::*;
        match (self, other) {
            (Max, Max) | (Min, Min) => Ordering::Equal,
            (Max, _) => Ordering::Greater,
            (_, Max) => Ordering::Less,
            (Min, _) => Ordering::Less,
            (_, Min) => Ordering::Greater,
            (Num(a), Num(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Prio {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
enum NewChunk {
    Named(String),
    Spec {
        ctype: String,
        slots: Vec<(String, Value)>,
        /// `- slot value` exclusions (request specs only).
        neg: Vec<(String, Value)>,
        /// `> < >= <= slot value` numeric inequalities (request specs only).
        rels: Vec<(String, Value, SlotOp)>,
        /// Request-built specs print slots-only (`(PROBE "bank")`);
        /// goal-focus lists print with ISA. See chunk-spec-to-chunk-def.
        bare: bool,
    },
    /// A fully-built instance (visual locations/objects, goal specs
    /// with generated names): placed as-is, never DM-resolved.
    Instance(ChunkInst),
}

#[derive(Debug, Clone)]
enum Event {
    SetBuffer {
        buffer: String,
        chunk: NewChunk,
        requested: bool,
        /// Name shown in the trace when it differs from the stored
        /// chunk (visual-location SETs store a copy but print the
        /// requested name).
        shown: Option<String>,
    },
    ClearBuffer {
        buffer: String,
    },
    ModBuffer {
        buffer: String,
        slots: Vec<(String, Value)>,
    },
    ModuleRequest {
        buffer: String,
        spec: NewChunk,
        /// Slots whose request value was a variable (skipped by
        /// partial matching, which only scores constants).
        var_slots: Vec<String>,
    },
    StartRetrieval {
        spec: NewChunk,
        var_slots: Vec<String>,
    },
    RetrievedChunk {
        name: String,
        gen: u64,
    },
    RetrievalFailure {
        gen: u64,
    },
    /// Blending request processing (`start-blending`, priority -3000).
    StartBlending {
        spec: NewChunk,
    },
    /// Blended chunk ready: place it in BLENDING (priority -1000).
    BlendingComplete {
        spec: NewChunk,
    },
    /// Blending failed (no matches / below threshold).
    BlendingFailure,
    ConflictResolution,
    ProductionSelected {
        prod: usize,
        bindings: Bindings,
    },
    BufferRead {
        buffer: String,
    },
    QueryBuffer {
        buffer: String,
    },
    ProductionFired {
        prod: usize,
        bindings: Bindings,
    },
    /// Vision maintenance: move due device items into the visicon.
    ProcDisplay,
    /// Vision marker that the visicon was updated.
    VisiconUpdate,
    /// Visual-location request resolution.
    FindLocation {
        spec: NewChunk,
    },
    /// Visual-location request found nothing.
    FindLocFailure,
    /// Attention shift to a location chunk.
    MoveAttention {
        loc: String,
    },
    /// Imaginal request delay elapsed: free the module and place the chunk.
    ImaginalDone {
        spec: NewChunk,
    },
    /// Attention landed: build the visual object.
    EncodingComplete {
        loc: String,
    },
    /// `+visual> cmd clear` direct event (output low); the buffer
    /// itself is cleared by the request-rule implicit event.
    VisualClear,
    /// Run-full-time dummy: keeps the run going to the time limit.
    Dummy,
    /// Utility reward propagation (output medium).
    PropagateReward {
        value: f32,
    },
    /// Preparation-state release after a module clear. High-detail
    /// only; matters because executing it wakes armed CR.
    ChangeState {
        module: String,
    },
    /// Manual style request (only press-key in M5a).
    PressKey {
        key: String,
        req_ms: i64,
    },
    PreparationComplete {
        key: String,
        req_ms: i64,
    },
    InitiationComplete {
        req_ms: i64,
    },
    MotorNotify {
        key: String,
        req_ms: i64,
    },
    /// Keyboard `output-key`: maintenance class (never wakes CR).
    OutputKey {
        key: String,
    },
    FinishMovement {
        req_ms: i64,
    },
}

struct Queued {
    time_ms: i64,
    prio: Prio,
    seq: u64,
    out: Out,
    /// Maintenance-class events never wake armed conflict-resolution
    /// (`conditions-met` requires non-maintenance).
    maintenance: bool,
    event: Event,
}

impl Ord for Queued {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap pops the greatest: invert so the earliest time,
        // highest priority, lowest seq comes out first.
        other
            .time_ms
            .cmp(&self.time_ms)
            .then(self.prio.cmp(&other.prio))
            .then(other.seq.cmp(&self.seq))
    }
}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Queued {
    fn eq(&self, other: &Self) -> bool {
        self.time_ms == other.time_ms && self.prio == other.prio && self.seq == other.seq
    }
}

impl Eq for Queued {}

// ---------------------------------------------------------------- engine

struct ExecProd {
    index: usize,
    /// Buffers implicitly cleared on firing (strict harvesting +
    /// request rule, computed at build).
    implicit_clears: Vec<String>,
    /// Buffers with implicit `?buffer> state free` tests (computed at
    /// build, matched and selected like explicit queries).
    implicit_queries: Vec<String>,
}

impl Clone for ExecProd {
    fn clone(&self) -> ExecProd {
        ExecProd {
            index: self.index,
            implicit_clears: self.implicit_clears.clone(),
            implicit_queries: self.implicit_queries.clone(),
        }
    }
}

pub struct Engine {
    model: Model,
    cfg: Config,
    rng: Mt,
    dm: Vec<ChunkInst>,
    /// `define-chunks` named chunks (not in DM), for name resolution.
    named: HashMap<String, ChunkInst>,
    /// Spreading fan-out per chunk name (DM chunks self-count 1; each
    /// reference from a DM chunk adds 1).
    fan_out: HashMap<String, u32>,
    /// Spreading fan-in: candidate chunk -> source chunk -> occurrences.
    fan_in: HashMap<String, HashMap<String, u32>>,
    /// Every name resolvable as a chunk (DM plus `define-chunks`).
    chunk_names: HashSet<String>,
    hist: Vec<History>,
    buffers: HashMap<String, Buffer>,
    queue: BinaryHeap<Queued>,
    seq: u64,
    time_ms: i64,
    limit_ms: i64,
    out_lines: Vec<String>,
    exec_prods: Vec<ExecProd>,
    cr_armed: bool,
    dm_gen: u64,
    dm_busy: bool,
    /// Production utilities (init :iu) and selection history
    /// (prod index, selection time ms) for reward propagation.
    utilities: Vec<f32>,
    history: Vec<(usize, i64)>,
    /// Chunk similarities, as given (lookup tries both orders;
    /// mirrors set-similarities).
    sims: HashMap<(String, String), f32>,
    /// Scripted device items not yet shown, by appearance time.
    pending_texts: Vec<DeviceText>,
    /// Scripted window clears not yet applied, sorted.
    pending_clears: Vec<i64>,
    /// Live visicon features, in proc-display order.
    features: Vec<Feature>,
    feat_seq: u64,
    /// Location-name copy counter: requested base -> copies made.
    copy_count: HashMap<String, u64>,
    /// Copy fan-out (`c-fan-out`): a copied chunk's source fan-out at copy
    /// time, used for Sji when the source is not in DM
    /// (`chunk-fan-j-to-i`, declarative-memory.lisp:2554).
    c_fan_out: HashMap<String, u32>,
    /// Buffer chunk name -> final DM chunk name, for chunk references that
    /// were merged into an existing DM chunk (`merging-test`).
    chunk_alias: HashMap<String, String>,
    /// `!stop!` was executed: end the run with a BREAK-EVENT.
    stopped: bool,
    /// Recently-retrieved chunk finsts: `(name, time_ms)`, newest last.
    finsts: Vec<(String, i64)>,
    /// SET-copy name -> feature index, resolving copies back for
    /// attention and encoding.
    vl_copies: HashMap<String, usize>,
    /// Visual-object names per kind: KIND -> objects so far.
    obj_built: HashMap<String, u64>,
    visual_busy: bool,
    man_prep_busy: bool,
    man_exec_busy: bool,
    imag_busy: bool,
    /// Blending module busy/error flags (`blending-module-busy/-error`).
    blending_busy: bool,
    blending_error: bool,
    /// Production compilation: count of compiled productions, the previous
    /// fired production `(index, bindings)`, and its fire time.
    compiled_count: usize,
    prev_prod: Option<(usize, Bindings)>,
    prev_prod_ms: i64,
    /// Last press-key geometry, for preparation differences.
    last_press: Option<(String, String, f32, f32)>,
    /// Trace output enabled (`:v`). Scripted drivers can toggle it.
    trace_on: bool,
}

impl Engine {
    pub fn new(
        model: Model,
        seed: (u64, u64),
        run_secs: f64,
        device: DeviceSetup,
    ) -> Result<Engine, EngineError> {
        Engine::new_with_width(model, seed, run_secs, device, MODULE_WIDTH)
    }

    /// Like [`Engine::new`], but with an explicit trace module-column width
    /// (see [`crate::trace::format_line`]).
    pub fn new_with_width(
        model: Model,
        seed: (u64, u64),
        run_secs: f64,
        device: DeviceSetup,
        module_width: usize,
    ) -> Result<Engine, EngineError> {
        let cfg = Config::from_model(&model)?;
        let mut rng = Mt::fresh();
        rng.set_seed(seed.0, seed.1);
        let DeviceSetup {
            texts,
            clears,
            end,
            rewards,
            sims,
        } = device;
        let mut eng = Engine {
            model,
            cfg,
            rng,
            dm: Vec::new(),
            named: HashMap::new(),
            fan_out: HashMap::new(),
            fan_in: HashMap::new(),
            chunk_names: HashSet::new(),
            hist: Vec::new(),
            buffers: HashMap::new(),
            queue: BinaryHeap::new(),
            seq: 0,
            time_ms: 0,
            limit_ms: (run_secs * 1000.0).round() as i64,
            out_lines: Vec::new(),
            exec_prods: Vec::new(),
            cr_armed: false,
            dm_gen: 0,
            dm_busy: false,
            utilities: Vec::new(),
            history: Vec::new(),
            sims: sims
                .iter()
                .map(|(a, b, v)| ((a.clone(), b.clone()), *v))
                .collect(),
            pending_texts: {
                let mut v = texts;
                v.sort_by_key(|t| t.at_ms);
                v
            },
            pending_clears: {
                let mut v = clears;
                v.sort_unstable();
                v
            },
            features: Vec::new(),
            feat_seq: 0,
            copy_count: HashMap::new(),
            c_fan_out: HashMap::new(),
            chunk_alias: HashMap::new(),
            stopped: false,
            finsts: Vec::new(),
            vl_copies: HashMap::new(),
            obj_built: HashMap::new(),
            visual_busy: false,
            man_prep_busy: false,
            man_exec_busy: false,
            imag_busy: false,
            blending_busy: false,
            blending_error: false,
            compiled_count: 0,
            prev_prod: None,
            prev_prod_ms: 0,
            last_press: None,
            trace_on: true,
        };
        eng.reset()?;
        eng.cfg.module_width = module_width;
        // Device script: a run-full-time dummy and scheduled rewards are part
        // of the task, not the model reset, so they are armed only here.
        if let Some(end) = end {
            eng.push_m(end, Prio::Min, Out::Never, true, Event::Dummy);
        }
        for (at, value) in &rewards {
            eng.push(
                *at,
                Prio::Max,
                Out::Medium,
                Event::PropagateReward { value: *value },
            );
        }
        Ok(eng)
    }

    /// `(reset)`: re-evaluate the model body's effect on engine state —
    /// rebuild DM and the named-chunk table, clear buffers, reset the clock
    /// to 0, and re-arm the initial goal + conflict resolution. The RNG is
    /// intentionally NOT reseeded (GOTCHAS #5/#35); scripted `:seed` steps
    /// handle that. Trace history (`out_lines`) is preserved.
    pub fn reset(&mut self) -> Result<(), EngineError> {
        self.cfg = Config::from_model(&self.model)?;
        self.trace_on = true;
        self.dm = Vec::new();
        for c in &self.model.chunks {
            self.dm.push(chunk_inst(c)?);
        }
        self.named = HashMap::new();
        for c in &self.model.defined_chunks {
            self.named.insert(c.name.clone(), chunk_inst(c)?);
        }
        self.build_fans();
        let dm_len = self.dm.len();
        self.hist = vec![
            History {
                creation_ms: 0,
                refs: vec![0],
                count: 1,
                fixed_base: None,
            };
            dm_len
        ];
        // Model-body `set-base-levels`: fixed history for named DM chunks.
        for (name, level, creation) in &self.model.base_levels {
            if let Some(i) = self.dm.iter().position(|c| c.name.as_deref() == Some(name.as_str())) {
                let creation_ms = creation
                    .map(|s| (s * 1000.0).round() as i64)
                    .unwrap_or(self.hist[i].creation_ms);
                let n = level.max(0.0).round() as usize;
                self.hist[i].creation_ms = creation_ms;
                self.hist[i].count = n;
                self.hist[i].fixed_base = Some(*level);
                self.hist[i].refs = match &self.cfg.ol {
                    Ol::On => vec![creation_ms; n],
                    Ol::Off => even_references(creation_ms, 0, n),
                    Ol::Num(ol) => even_references(creation_ms, 0, (*ol as usize).min(n)),
                };
            }
        }
        // Model-body `set-similarities`: explicit Sji both directions.
        for (a, b, v) in &self.model.similarities {
            self.sims.insert((a.clone(), b.clone()), *v);
            self.sims.insert((b.clone(), a.clone()), *v);
        }
        self.buffers.clear();
        for b in [
            "GOAL",
            "RETRIEVAL",
            "IMAGINAL",
            "VISUAL",
            "VISUAL-LOCATION",
            "MANUAL",
        ] {
            self.buffers.insert(b.to_string(), Buffer::empty());
        }
        if self.cfg.blending {
            self.buffers
                .insert("BLENDING".to_string(), Buffer::empty());
        }
        self.queue = BinaryHeap::new();
        self.time_ms = 0;
        self.cr_armed = false;
        self.dm_gen = 0;
        self.dm_busy = false;
        self.utilities = vec![self.cfg.iu; self.model.productions.len()];
        self.history.clear();
        self.features.clear();
        self.feat_seq = 0;
        self.copy_count.clear();
        self.c_fan_out.clear();
        self.chunk_alias.clear();
        self.stopped = false;
        self.finsts.clear();
        self.vl_copies.clear();
        self.obj_built.clear();
        self.visual_busy = false;
        self.man_prep_busy = false;
        self.man_exec_busy = false;
        self.imag_busy = false;
        self.blending_busy = false;
        self.blending_error = false;
        self.compiled_count = 0;
        self.prev_prod = None;
        self.prev_prod_ms = 0;
        self.last_press = None;
        self.exec_prods = self
            .model
            .productions
            .iter()
            .enumerate()
            .map(|(index, prod)| ExecProd {
                index,
                implicit_clears: implicit_clears(prod, &self.cfg.do_not_harvest),
                implicit_queries: implicit_queries(prod, &self.cfg.do_not_query),
            })
            .collect();
        // Device proc-display already due at t=0 (maintenance).
        if !self.pending_texts.is_empty() && self.pending_texts[0].at_ms <= 0 {
            self.push_m(0, Prio::Num(-100), Out::Always, true, Event::ProcDisplay);
        }
        if let Some(goal) = self.model.initial_goal.clone() {
            let chunk = new_chunk(&goal)?;
            self.push(
                0,
                Prio::Num(0),
                Out::Low,
                Event::SetBuffer {
                    buffer: "GOAL".to_string(),
                    chunk,
                    requested: false,
                    shown: None,
                },
            );
        }
        self.push(0, Prio::Min, Out::Medium, Event::ConflictResolution);
        Ok(())
    }

    /// Rebuild the spreading fan tables from the model's DM (`add-dm`)
    /// chunks. Mirrors `add-chunk-into-dm`'s fan updates
    /// (`declarative-memory.lisp:1953`): each chunk counts itself in
    /// fan-out, and every slot value that names a chunk adds to that
    /// chunk's fan-out and to the owner's fan-in. Only DM chunks get a
    /// fan-in (spreading candidates come from DM).
    fn build_fans(&mut self) {
        let mut names: HashSet<String> =
            self.model.chunks.iter().map(|c| c.name.clone()).collect();
        names.extend(self.model.defined_chunks.iter().map(|c| c.name.clone()));
        self.chunk_names = names.clone();
        self.fan_out.clear();
        self.fan_in.clear();
        for c in &self.model.chunks {
            *self.fan_out.entry(c.name.clone()).or_insert(0) += 1;
            let mut counts: HashMap<String, u32> = HashMap::new();
            for (_, v) in &c.slots {
                if let Pattern::Sym(s) = v {
                    if names.contains(s) {
                        *self.fan_out.entry(s.clone()).or_insert(0) += 1;
                        *counts.entry(s.clone()).or_insert(0) += 1;
                    }
                }
            }
            *counts.entry(c.name.clone()).or_insert(0) += 1;
            self.fan_in.insert(c.name.clone(), counts);
        }
    }

    /// The name of the chunk a slot value denotes, if any (`chunk-p-fct`).
    fn chunk_name_of(&self, v: &Value) -> Option<String> {
        match v {
            Value::Sym(s) if self.chunk_names.contains(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// `compute-sji` (`declarative-memory.lisp:2536`): an explicit
    /// similarity wins; otherwise, when the source `j` occurs in `i`'s
    /// fan-in, `mas - ln(fan-out(j)/count)`, clamped at 0 unless `:nsji`.
    fn compute_sji(&self, j: &str, i: &str, mas: f32) -> f32 {
        if let Some(v) = self
            .sims
            .get(&(j.to_string(), i.to_string()))
            .or_else(|| self.sims.get(&(i.to_string(), j.to_string())))
        {
            return *v;
        }
        if let Some(cnt) = self.fan_in.get(i).and_then(|m| m.get(j)) {
            // `chunk-fan-j-to-i` (declarative-memory.lisp:2554): DM chunks
            // use fan-out directly; copies/non-DM add `max(1, c-fan-out)`.
            let fo = self.fan_out.get(j).copied().unwrap_or(0) as f32;
            let eff = if self.dm.iter().any(|c| c.name.as_deref() == Some(j)) {
                fo
            } else {
                fo + self.c_fan_out.get(j).copied().unwrap_or(0).max(1) as f32
            };
            let val = mas - (eff / *cnt as f32).ln();
            return if self.cfg.nsji { val } else { val.max(0.0) };
        }
        0.0
    }

    /// `spreading-activation` (`declarative-memory.lisp:2455`): sum over
    /// buffers with a nonzero source spread of `(spread/len(sources)) *
    /// sji(source, candidate)`. Goal spread is `:ga` (default 0), imaginal
    /// is `:imaginal-activation` (default 1.0); other buffer spreads
    /// default 0.
    fn spreading_activation(&mut self, candidate: &str) -> f32 {
        let Some(mas) = self.cfg.mas else {
            return 0.0;
        };
        let verbose = self.act_medium();
        if verbose {
            self.out("Computing activation spreading from buffers".to_string());
        }
        let mut total = 0.0f32;
        for (buffer, spread) in [
            ("GOAL", self.cfg.ga),
            ("IMAGINAL", self.cfg.imaginal_activation),
        ] {
            if spread == 0.0 {
                continue;
            }
            let Some(chunk) = self.buffers.get(buffer).and_then(|b| b.chunk.as_ref()).cloned() else {
                continue;
            };
            // `chunk-filled-slots-list-fct` returns the chunk's internal
            // slot list, which is built by prepending (reverse insertion).
            let js: Vec<String> = chunk
                .slots
                .iter()
                .rev()
                .filter_map(|(_, v)| self.chunk_name_of(v))
                .collect();
            if js.is_empty() {
                continue;
            }
            if verbose {
                let cname = chunk
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("{buffer}-CHUNK0"));
                self.out(format!(
                    "  Spreading {} from buffer {} chunk {}",
                    lisp_f(spread),
                    buffer,
                    cname
                ));
                self.out(format!(
                    "    sources of activation are: ({})",
                    js.join(" ")
                ));
            }
            let level = spread / js.len() as f32;
            for j in &js {
                let sji = self.compute_sji(j, candidate, mas);
                if verbose {
                    self.out(format!(
                        "    Spreading activation  {} from source {} level  {} times Sji {}",
                        lisp_f(level * sji),
                        j,
                        lisp_f(level),
                        lisp_f(sji)
                    ));
                }
                total += level * sji;
            }
        }
        if verbose {
            self.out(format!("Total spreading activation: {}", lisp_f(total)));
        }
        total
    }

    fn push(&mut self, time_ms: i64, prio: Prio, out: Out, event: Event) {
        self.push_m(time_ms, prio, out, false, event);
    }

    fn push_m(&mut self, time_ms: i64, prio: Prio, out: Out, maintenance: bool, event: Event) {
        let seq = self.seq;
        self.seq += 1;
        self.queue.push(Queued {
            time_ms,
            prio,
            seq,
            out,
            maintenance,
            event,
        });
    }

    fn now(&mut self, prio: Prio, out: Out, event: Event) {
        let t = self.time_ms;
        self.push(t, prio, out, event);
    }

    fn now_m(&mut self, prio: Prio, out: Out, maintenance: bool, event: Event) {
        let t = self.time_ms;
        self.push_m(t, prio, out, maintenance, event);
    }

    fn emit(&mut self, module: &str, details: String) {
        if !self.trace_on {
            return;
        }
        self.out_lines.push(format_line(
            self.time_ms,
            module,
            &details,
            self.cfg.module_width,
        ));
    }

    /// Raw command/`model-output` line (no timestamp/module formatting),
    /// interleaved with the trace like ACT-R's `model-output`.
    fn out(&mut self, s: String) {
        if self.trace_on {
            self.out_lines.push(s);
        }
    }

    /// `:act` visibility by level (`dm-act-level`).
    fn act_low(&self) -> bool {
        self.cfg.act.is_some()
    }

    fn act_medium(&self) -> bool {
        matches!(
            self.cfg.act,
            Some(Act::T) | Some(Act::High) | Some(Act::Medium)
        )
    }

    fn act_high(&self) -> bool {
        matches!(self.cfg.act, Some(Act::T) | Some(Act::High))
    }

    fn buffer_mut(&mut self, name: &str) -> &mut Buffer {
        self.buffers
            .entry(name.to_string())
            .or_insert_with(Buffer::empty)
    }

    /// Merge a cleared buffer chunk into DM by content match, adding a
    /// reference (`merge-reference-list/count`); fresh contents are
    /// added with creation=now, refs=(now), count=1. Buffers in
    /// `:ignore-buffers` skip this; no golden sets it.
    fn merge_into_dm(&mut self, mut chunk: ChunkInst, buffer: &str) {
        let now = self.time_ms;
        let found = self
            .dm
            .iter()
            .position(|c| c.ctype == chunk.ctype && c.slots == chunk.slots);
        match found {
            Some(i) => {
                let h = &mut self.hist[i];
                h.count += 1;
                match &self.cfg.ol {
                    Ol::Off => h.refs.insert(0, now),
                    Ol::Num(ol) => {
                        h.refs.insert(0, now);
                        let keep = (*ol as usize).min(h.refs.len());
                        h.refs.truncate(keep);
                    }
                    Ol::On => h.refs.clear(),
                }
                // A buffer chunk referenced elsewhere (a goal slot) merges
                // into an existing DM chunk: alias the buffer name to the DM
                // name so `value B` matches the stored reference
                // (`merging-test`).
                if let Some(dm_name) = self.dm[i].name.clone() {
                    if let Some(n) = chunk.name.clone() {
                        if n != dm_name {
                            self.chunk_alias.insert(n, dm_name);
                        }
                    }
                }
            }
            None => {
                // Unnamed buffer contents (imaginal/goal request specs)
                // land as reuse-chunk copies: {BUFFER}-CHUNK0-{k}, which
                // is the name later retrievals print. A *named*
                // `define-chunks` chunk modified in a buffer must not
                // shadow its table entry when merged, so it gets a fresh
                // `NAME-{k}` copy instead.
                if let Some(n) = chunk.name.clone() {
                    if self.named.contains_key(&n) {
                        let k = *self.copy_count.entry(n.clone()).or_insert(0);
                        self.copy_count.insert(n.clone(), k + 1);
                        chunk.name = Some(format!("{n}-{k}"));
                    }
                } else {
                    let base = format!("{buffer}-CHUNK0");
                    let n = *self.copy_count.entry(base.clone()).or_insert(0);
                    self.copy_count.insert(base.clone(), n + 1);
                    chunk.name = Some(format!("{base}-{n}"));
                }
                self.dm.push(chunk);
                self.hist.push(History {
                    creation_ms: now,
                    refs: vec![now],
                    count: 1,
                    fixed_base: None,
                });
                // New DM chunk: update spreading fans like
                // `add-chunk-into-dm` (self fan-out plus each chunk-valued
                // slot).
                let name = self.dm.last().and_then(|c| c.name.clone());
                if let Some(name) = name {
                    self.chunk_names.insert(name.clone());
                    *self.fan_out.entry(name.clone()).or_insert(0) += 1;
                    let mut counts: HashMap<String, u32> = HashMap::new();
                    let slots = self.dm.last().map(|c| c.slots.clone()).unwrap_or_default();
                    for (_, v) in &slots {
                        if let Some(j) = self.chunk_name_of(v) {
                            *self.fan_out.entry(j.clone()).or_insert(0) += 1;
                            *counts.entry(j).or_insert(0) += 1;
                        }
                    }
                    *counts.entry(name.clone()).or_insert(0) += 1;
                    self.fan_in.insert(name, counts);
                }
            }
        }
    }

    /// Any buffer mutation wakes an armed conflict-resolution.
    fn changed(&mut self) {
        if self.cr_armed {
            self.cr_armed = false;
            self.now(Prio::Min, Out::Medium, Event::ConflictResolution);
        }
    }

    /// Run once and return the full trace (single-shot contract).
    pub fn run(mut self) -> Result<String, EngineError> {
        let secs = self.limit_ms as f64 / 1000.0;
        self.run_secs(secs)?;
        Ok(self.output())
    }

    /// One `(run secs)` step: process events until the queue drains or the
    /// relative limit (`time_ms + secs*1000`) is passed. Emits its own
    /// `Stopped because ...` terminator, exactly like Lisp `run`
    /// (`framework/scheduling.lisp:816`, which bounds each run relative to
    /// the current model time).
    pub fn run_secs(&mut self, secs: f64) -> Result<(), EngineError> {
        let limit_ms = self.time_ms + (secs * 1000.0).round() as i64;
        loop {
            let next = match self.queue.pop() {
                None => {
                    // Queue empty before the limit: out of events; at or
                    // past the limit (run-full-time dummy consumed):
                    // time limit reached, like Lisp `run`.
                    if self.time_ms >= limit_ms {
                        self.time_ms = limit_ms;
                        self.emit("------", "Stopped because time limit reached".into());
                    } else {
                        self.emit("------", "Stopped because no events left to process".into());
                    }
                    break;
                }
                Some(q) => q,
            };
            if next.time_ms > limit_ms {
                self.time_ms = limit_ms;
                self.emit("------", "Stopped because time limit reached".into());
                break;
            }
            self.time_ms = next.time_ms;
            let show = self.cfg.detail.shows(next.out);
            let is_cr = matches!(next.event, Event::ConflictResolution);
            let maintenance = next.maintenance;
            self.exec(next.event, show)?;
            if self.stopped {
                self.queue.clear();
                self.emit("------", "BREAK-EVENT Stopped by !stop!".into());
                break;
            }
            // Armed conflict-resolution wakes after any later
            // non-maintenance event (`conditions-met`); the arming CR
            // itself does not count.
            if !is_cr && !maintenance {
                self.changed();
            }
        }
        Ok(())
    }

    /// All trace emitted so far, newline-terminated (empty if none).
    pub fn output(&self) -> String {
        if self.out_lines.is_empty() {
            return String::new();
        }
        let mut s = self.out_lines.join("\n");
        s.push('\n');
        s
    }

    /// `(goal-focus <spec>)`: place a chunk in the goal buffer at the
    /// current time (`core-modules/goal.lisp:275`, priority `:max`,
    /// `requested nil`). Any armed conflict-resolution wakes on the buffer
    /// change via [`Engine::changed`], matching the Lisp trace.
    pub fn goal_focus(&mut self, desc: &ChunkDesc) -> Result<(), EngineError> {
        let chunk = new_chunk(desc)?;
        self.now(
            Prio::Max,
            Out::Low,
            Event::SetBuffer {
                buffer: "GOAL".to_string(),
                chunk,
                requested: false,
                shown: None,
            },
        );
        Ok(())
    }

    /// Trace on/off (`:v`), used by scripted drivers that hide segments.
    pub fn set_trace(&mut self, on: bool) {
        self.trace_on = on;
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
            ":PPM" => self.cfg.ppm = num(val),
            ":EGS" => self.cfg.egs = num(val).unwrap_or(0.0),
            ":CIA" => self.cfg.cia = truthy,
            ":RIR" => self.cfg.rir = truthy,
            ":EPL" => self.cfg.epl = truthy,
            ":PCT" => self.cfg.pct = truthy,
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
            ":MAS" => self.cfg.mas = num(val),
            ":BLC" => {
                if let Some(x) = num(val) {
                    self.cfg.blc = x;
                }
            }
            ":BLL" => {
                self.cfg.bll = num(val).filter(|b| *b > 0.0);
                self.cfg.act_scale = match self.cfg.bll {
                    Some(b) => (0.001f32).powf(-b).ln(),
                    None => 0.0,
                };
            }
            ":UL" => self.cfg.ul = truthy,
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
            ":CRT" => self.cfg.crt = truthy,
            ":CST" => self.cfg.cst = truthy,
            ":NSJI" => self.cfg.nsji = truthy,
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
            // `:UTILITY-OFFSETS`, seeds, etc. accepted without an observable
            // effect here (the ppm offset is implicit in `:ppm`).
            _ => {}
        }
        Ok(())
    }

    /// `(sgp :seed (S C))`: init + discard, same as construction.
    pub fn set_seed(&mut self, seed: (u64, u64)) {
        self.rng.set_seed(seed.0, seed.1);
    }

    fn exec(&mut self, event: Event, show: bool) -> Result<(), EngineError> {
        match event {
            Event::SetBuffer {
                buffer,
                chunk,
                requested,
                shown,
            } => {
                let inst = self.instantiate(&buffer, chunk)?;
                if show {
                    let text = shown.unwrap_or_else(|| inst.show());
                    let mut details = format!("SET-BUFFER-CHUNK {buffer} {text}");
                    if !requested {
                        details.push_str(" NIL");
                    }
                    // The SET event prints under the buffer's owning
                    // module (the `:module` of `schedule-set-buffer-chunk`).
                    let module = match buffer.as_str() {
                        "RETRIEVAL" => "DECLARATIVE",
                        "VISUAL-LOCATION" | "VISUAL" => "VISION",
                        "MANUAL" => "MOTOR",
                        other => other,
                    };
                    self.emit(module, details);
                }
                self.buffer_mut(&buffer).chunk = Some(inst);
                self.buffer_mut(&buffer).failed = false;
                self.changed();
            }
            Event::ClearBuffer { buffer } => {
                if show {
                    self.emit("PROCEDURAL", format!("CLEAR-BUFFER {buffer}"));
                }
                let cleared = self.buffer_mut(&buffer).chunk.take();
                self.buffer_mut(&buffer).failed = false;
                // `:notify-on-clear` for declarative: the cleared chunk
                // merges into DM, gaining a reference
                // (`merge-chunk-into-dm`). Empty clears merge nothing.
                if let Some(chunk) = cleared {
                    self.merge_into_dm(chunk, &buffer);
                }
                self.changed();
            }
            Event::ModBuffer { buffer, slots } => {
                if show {
                    self.emit("PROCEDURAL", format!("MOD-BUFFER-CHUNK {buffer}"));
                }
                let b = self.buffer_mut(&buffer);
                let chunk = b
                    .chunk
                    .as_mut()
                    .ok_or_else(|| bad(format!("MOD-BUFFER-CHUNK with empty {buffer}")))?;
                for (s, v) in slots {
                    match chunk.slots.iter_mut().find(|(k, _)| k == &s) {
                        Some((_, old)) => *old = v,
                        None => chunk.slots.push((s, v)),
                    }
                }
                self.changed();
            }
            Event::ModuleRequest {
                buffer,
                spec,
                var_slots,
            } => {
                if show {
                    self.emit("PROCEDURAL", format!("MODULE-REQUEST {buffer}"));
                }
                match buffer.as_str() {
                    "RETRIEVAL" => {
                        // dm-request: abort any in-flight retrieval, then start.
                        self.dm_gen += 1;
                        self.dm_busy = true;
                        self.buffers.get_mut("RETRIEVAL").unwrap().busy = true;
                        self.buffers.get_mut("RETRIEVAL").unwrap().failed = false;
                        self.now(
                            Prio::Num(-2000),
                            Out::Medium,
                            Event::StartRetrieval { spec, var_slots },
                        );
                    }
                    "VISUAL-LOCATION" => {
                        self.now(Prio::Num(0), Out::Medium, Event::FindLocation { spec });
                    }
                    "VISUAL" => {
                        self.visual_request(spec)?;
                    }
                    "MANUAL" => {
                        self.manual_request(spec)?;
                    }
                    "IMAGINAL" => {
                        // imaginal-request: busy requests are ignored
                        // (warning to stdout in Lisp, nothing in trace).
                        if !self.imag_busy {
                            self.imag_busy = true;
                            let at = self.time_ms + self.cfg.imaginal_delay_ms;
                            self.push(at, Prio::Num(0), Out::Never, Event::ImaginalDone { spec });
                        }
                    }
                    "BLENDING" => {
                        // blending-request: clear the error flag, mark busy,
                        // and schedule the real work at priority -3000 so
                        // other 0-time buffer changes settle first
                        // (blending.lisp:465).
                        self.blending_busy = true;
                        self.blending_error = false;
                        self.now(
                            Prio::Num(-3000),
                            Out::Medium,
                            Event::StartBlending { spec },
                        );
                    }
                    "GOAL" => {
                        // goal-style-request: delay 0, priority -1000, so the
                        // new goal lands after the production's clears
                        // (`support/goal-style-module.lisp:245`). Request
                        // specs print slots-only (GOTCHA #44).
                        self.push(
                            self.time_ms,
                            Prio::Num(-1000),
                            Out::Medium,
                            Event::SetBuffer {
                                buffer: "GOAL".to_string(),
                                chunk: spec,
                                requested: true,
                                shown: None,
                            },
                        );
                    }
                    got => {
                        return Err(unimpl(format!("request to buffer {got}")));
                    }
                }
            }
            Event::StartRetrieval { spec, var_slots } => {
                if show {
                    self.emit("DECLARATIVE", "start-retrieval".into());
                }
                self.start_retrieval(spec, &var_slots)?;
            }
            Event::RetrievedChunk { name, gen } => {
                if gen != self.dm_gen {
                    return Ok(()); // aborted by a newer request: silent.
                }
                if show {
                    self.emit("DECLARATIVE", format!("RETRIEVED-CHUNK {name}"));
                }
                self.dm_busy = false;
                self.buffers.get_mut("RETRIEVAL").unwrap().busy = false;
                self.add_finst(&name);
                self.now(
                    Prio::Max,
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "RETRIEVAL".to_string(),
                        chunk: NewChunk::Named(name),
                        requested: true,
                        shown: None,
                    },
                );
            }
            Event::RetrievalFailure { gen } => {
                if gen != self.dm_gen {
                    return Ok(());
                }
                if show {
                    self.emit("DECLARATIVE", "RETRIEVAL-FAILURE".into());
                }
                self.dm_busy = false;
                let b = self.buffer_mut("RETRIEVAL");
                b.busy = false;
                b.failed = true;
                self.changed();
            }
            Event::StartBlending { spec } => {
                if show {
                    self.emit("BLENDING", "START-BLENDING".into());
                }
                self.start_blending(&spec)?;
            }
            Event::BlendingComplete { spec } => {
                if show {
                    self.emit("BLENDING", "BLENDING-COMPLETE".into());
                }
                // blending-complete: free the module, then place the chunk
                // (priority -1000, default `low` output).
                self.blending_busy = false;
                self.now(
                    Prio::Num(-1000),
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "BLENDING".to_string(),
                        chunk: spec,
                        requested: true,
                        shown: None,
                    },
                );
            }
            Event::BlendingFailure => {
                if show {
                    self.emit("BLENDING", "BLENDING-FAILURE".into());
                }
                self.blending_busy = false;
                self.blending_error = true;
                let b = self.buffer_mut("BLENDING");
                b.busy = false;
                b.failed = true;
                self.changed();
            }
            Event::ConflictResolution => {
                if show {
                    self.emit("PROCEDURAL", "CONFLICT-RESOLUTION".into());
                }
                self.conflict_resolution()?;
            }
            Event::ProductionSelected { prod, bindings } => {
                let name = self.model.productions[prod].name.clone();
                if show {
                    self.emit("PROCEDURAL", format!("PRODUCTION-SELECTED {name}"));
                }
                // Utility-learning history records selections.
                if self.cfg.ul {
                    self.history.push((prod, self.time_ms));
                }
                // Reads and queries in LHS order, implicit queries last,
                // then firing at +dat.
                for item in self.model.productions[prod].lhs.clone() {
                    match item {
                        LhsItem::Match { buffer, .. } => {
                            self.now(Prio::Num(0), Out::Always, Event::BufferRead { buffer });
                        }
                        LhsItem::Query { buffer, .. } => {
                            self.now(Prio::Num(0), Out::Always, Event::QueryBuffer { buffer });
                        }
                    }
                }
                for buffer in self.exec_prods[prod].implicit_queries.clone() {
                    self.now(Prio::Num(0), Out::Always, Event::QueryBuffer { buffer });
                }
                let at = self.time_ms + self.cfg.dat_ms;
                self.push(
                    at,
                    Prio::Num(0),
                    Out::Low,
                    Event::ProductionFired { prod, bindings },
                );
            }
            Event::BufferRead { buffer } => {
                if show {
                    self.emit("PROCEDURAL", format!("BUFFER-READ-ACTION {buffer}"));
                }
            }
            Event::QueryBuffer { buffer } => {
                if show {
                    self.emit("PROCEDURAL", format!("QUERY-BUFFER-ACTION {buffer}"));
                }
            }
            Event::ProductionFired { prod, bindings } => {
                let name = self.model.productions[prod].name.clone();
                if show {
                    self.emit("PROCEDURAL", format!("PRODUCTION-FIRED {name}"));
                }
                self.fire(prod, &bindings)?;
            }
            Event::ProcDisplay => {
                if show {
                    self.emit("VISION", "proc-display".into());
                }
                self.proc_display()?;
            }
            Event::VisiconUpdate => {
                if show {
                    self.emit("VISION", "visicon-update".into());
                }
            }
            Event::FindLocation { spec } => {
                if show {
                    self.emit("VISION", "Find-location".into());
                }
                self.find_location(spec)?;
            }
            Event::FindLocFailure => {
                if show {
                    self.emit("VISION", "find-loc-failure".into());
                }
                // Buffer failure was set when the search failed.
            }
            Event::ImaginalDone { spec } => {
                // set-imaginal-free + goal-style-request, both invisible.
                self.imag_busy = false;
                self.now(
                    Prio::Num(-1000),
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "IMAGINAL".to_string(),
                        chunk: spec,
                        requested: true,
                        shown: None,
                    },
                );
            }
            Event::MoveAttention { loc } => {
                if show {
                    self.emit("VISION", format!("Move-attention {loc} NIL"));
                }
                self.visual_busy = true;
                let at = self.time_ms + self.cfg.attention_ms;
                self.push(
                    at,
                    Prio::Num(0),
                    Out::Medium,
                    Event::EncodingComplete { loc },
                );
            }
            Event::EncodingComplete { loc } => {
                if show {
                    self.emit("VISION", format!("Encoding-complete {loc} NIL"));
                }
                self.encoding_complete(loc)?;
            }
            Event::VisualClear => {
                if show {
                    self.emit("VISION", "CLEAR".into());
                }
                self.visual_busy = false;
                // pm-module clear also frees preparation state 50 ms
                // later via an (at most high-detail) change-state event;
                // executing it wakes an armed conflict-resolution.
                // Details inferred from the scheduled params; no golden
                // shows this line yet (high-detail visual clears only).
                let at = self.time_ms + self.cfg.feat_prep_ms;
                self.push(
                    at,
                    Prio::Num(0),
                    Out::Always,
                    Event::ChangeState {
                        module: "VISION".to_string(),
                    },
                );
            }
            Event::Dummy => {}
            Event::PropagateReward { value } => {
                if show {
                    self.emit("UTILITY", format!("PROPAGATE-REWARD {value}"));
                }
                // Rewards apply only with subsymbolics + learning on;
                // otherwise silently ignored (Lisp warns to stdout).
                if self.cfg.esc && self.cfg.ul {
                    for (prod, at) in self.history.clone() {
                        let dt = (self.time_ms - at) as f32 / 1000.0;
                        let r = value - dt;
                        let u = &mut self.utilities[prod];
                        *u += self.cfg.alpha * (r - *u);
                    }
                    self.history.clear();
                }
            }
            Event::ChangeState { module } => {
                if show {
                    self.emit(
                        &module,
                        "CHANGE-STATE :LAST NONE :PREP FREE :PROC FREE".into(),
                    );
                }
            }
            Event::PressKey { key, req_ms } => {
                if show {
                    self.emit("MOTOR", format!("PRESS-KEY KEY {key}"));
                }
                // Preparation features: 5 fresh, else hfrt differences
                // vs the last press (maxfeats is 1+4 slots = 5: hand 4,
                // finger 3, else r/theta diffs).
                let spec = Self::key_spec(&key)?;
                let feats = match &self.last_press {
                    None => 5,
                    Some((h, f, r, t)) => {
                        if *h != spec.hand {
                            4
                        } else if *f != spec.finger {
                            3
                        } else {
                            (if *r != spec.r { 1 } else { 0 })
                                + (if *t != spec.theta { 1 } else { 0 })
                        }
                    }
                };
                self.last_press = Some((
                    spec.hand.to_string(),
                    spec.finger.to_string(),
                    spec.r,
                    spec.theta,
                ));
                self.man_prep_busy = true;
                let prep = feats as i64 * self.cfg.feat_prep_ms;
                let at = self.time_ms + prep;
                self.push(
                    at,
                    Prio::Num(0),
                    Out::Always,
                    Event::PreparationComplete { key, req_ms },
                );
            }
            Event::PreparationComplete { key, req_ms } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!("PREPARATION-COMPLETE {}", Self::secs_str(req_ms)),
                    );
                }
                self.man_prep_busy = false;
                self.man_exec_busy = true;
                // peck-recoil: exec = init + max(burst, move-time),
                // finish = exec + burst + max(burst, move-time),
                // all in seconds, rounded to ms at scheduling.
                let spec = Self::key_spec(&key)?;
                let fit = (self.cfg.peck_fitts * (spec.r + 0.5).log2())
                    .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                let move_s = (self.cfg.burst_ms as f32 / 1000.0).max(fit);
                // Lisp rounds seconds->ms at scheduling time.
                let exec_ms = ((self.cfg.init_ms as f32 / 1000.0 + move_s) * 1000.0).round() as i64;
                let finish_ms =
                    ((exec_ms as f32 / 1000.0 + self.cfg.burst_ms as f32 / 1000.0 + move_s)
                        * 1000.0)
                        .round() as i64;
                let t = self.time_ms;
                self.push(
                    t + self.cfg.init_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::InitiationComplete { req_ms },
                );
                self.push(
                    t + exec_ms,
                    Prio::Num(0),
                    Out::Never,
                    Event::MotorNotify { key, req_ms },
                );
                self.push(
                    t + finish_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::FinishMovement { req_ms },
                );
            }
            Event::InitiationComplete { req_ms } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!("INITIATION-COMPLETE {}", Self::secs_str(req_ms)),
                    );
                }
            }
            Event::MotorNotify { key, req_ms } => {
                self.now_m(Prio::Num(0), Out::Always, true, Event::OutputKey { key });
                let _ = req_ms;
            }
            Event::OutputKey { key } => {
                if show {
                    // The device reports its own key name (lowercase as
                    // specified in the keyboard table).
                    self.emit(
                        "KEYBOARD",
                        format!("output-key {} {}", self.model.name, key.to_lowercase()),
                    );
                }
            }
            Event::FinishMovement { req_ms } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!("FINISH-MOVEMENT {}", Self::secs_str(req_ms)),
                    );
                }
                self.man_exec_busy = false;
            }
        }
        Ok(())
    }

    /// Seconds from integer ms, printed as Lisp `princ` shows it.
    fn secs_str(req_ms: i64) -> String {
        format!("{}", req_ms as f32 / 1000.0)
    }

    /// Virtual keyboard geometry for press-key movements (hand/finger/r/theta
    /// per key name as requested; output uses the table's lowercase name).
    /// M5a seeds Q; digits join with device-task goldens.
    fn key_spec(key: &str) -> Result<KeySpec, EngineError> {
        key_table()
            .get(key)
            .cloned()
            .ok_or_else(|| unimpl(format!("press-key for unknown key {key}")))
    }

    fn instantiate(&self, _buffer: &str, chunk: NewChunk) -> Result<ChunkInst, EngineError> {
        match chunk {
            NewChunk::Named(name) => self
                .dm
                .iter()
                .find(|c| c.name.as_deref() == Some(name.as_str()))
                .or_else(|| self.named.get(&name))
                .cloned()
                .ok_or_else(|| bad(format!("no chunk named {name}"))),
            NewChunk::Spec {
                ctype,
                slots,
                bare,
                ..
            } => Ok(ChunkInst {
                name: None,
                ctype,
                slots,
                bare,
            }),
            NewChunk::Instance(inst) => Ok(inst),
        }
    }

    fn conflict_resolution(&mut self) -> Result<(), EngineError> {
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
            if crt {
                self.out(format!("Trying production: {pname}"));
            }
            let prod = self.model.productions[ep.index].clone();
            let mut mism: Vec<PpmMismatch> = Vec::new();
            if let Some(b) = self.match_prod(&prod, &ep.implicit_queries, &mut mism)? {
                let mut u = self.utilities[ep.index];
                // `ppm-offset` (procedural.lisp:2373): penalty is
                // `:ppm * Σ similarity` over the mismatched slots (the
                // similarities are negative, so this lowers utility).
                if let Some(p) = self.cfg.ppm {
                    u += p * mism.iter().map(|m| m.4).sum::<f32>();
                }
                if self.cfg.egs != 0.0 {
                    u += self.rng.noise(self.cfg.egs);
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
                let i = self.rng.random_int(tied.len() as u64) as usize;
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
    fn crt_failure_reason(&self, prod: &Production) -> String {
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
                            .and_then(|b| b.chunk.as_ref())
                            .is_none(),
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
                        ("ERROR", Pattern::Sym(s)) if s == "T" => {
                            !self.buffer_error(buffer)
                        }
                        _ => true,
                    };
                    if !ok {
                        let v = match val {
                            Pattern::Sym(s) => s.clone(),
                            other => format!("{other:?}"),
                        };
                        return format!("The {} {} query of the {} buffer failed.", attr, v, buffer);
                    }
                }
            }
        }
        "The production did not match.".to_string()
    }

    fn match_prod(
        &self,
        prod: &Production,
        implicit_queries: &[String],
        ppm: &mut Vec<PpmMismatch>,
    ) -> Result<Option<Bindings>, EngineError> {
        let mut b = Bindings::new();
        for item in &prod.lhs {
            if self.match_item(item, &mut b, ppm)?.is_none() {
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

    fn match_item(
        &self,
        item: &LhsItem,
        b: &mut Bindings,
        ppm: &mut Vec<PpmMismatch>,
    ) -> Result<Option<()>, EngineError> {
        match item {
            LhsItem::Match { buffer, tests } => {
                let buf = self
                    .buffers
                    .get(buffer)
                    .ok_or_else(|| bad(format!("unknown buffer ={buffer}>")))?;
                let chunk = match &buf.chunk {
                    Some(c) => c,
                    None => return Ok(None),
                };
                for t in tests {
                    // ACT-R compiles a LHS `ISA` line into a slot signature
                    // (filled/empty masks), not a chunk-type comparison
                    // (`production-parsing.lisp:786`); the per-slot tests
                    // below already reproduce that signature.
                    if t.slot == "ISA" {
                        continue;
                    }
                    // ACT-R `test-var-slot`: the slot *name* itself may be an
                    // LHS variable (`=attr =val`), substituted from bindings.
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
                        return Ok(None);
                    }
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
                            self.buffers
                                .get(buffer)
                                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?
                                .chunk
                                .is_none()
                        }
                        ("BUFFER", Pattern::Sym(s)) if s == "FULL" => {
                            self.buffers
                                .get(buffer)
                                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?
                                .chunk
                                .is_some()
                        }
                        ("STATE", Pattern::Sym(s)) if s == "FREE" => self.query_free(buffer)?,
                        ("STATE", Pattern::Sym(s)) if s == "BUSY" => !self.query_free(buffer)?,
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
        }
        Ok(Some(()))
    }

    /// LHS `=` equality test with procedural partial matching (`:ppm`).
    /// An exact miss may still pass when the value similarity is strictly
    /// greater than `:md`; the mismatch is recorded for the utility penalty
    /// (`test-slot`, procedural.lisp:1774).
    fn match_eq(
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
    fn ppm_fallback(
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

    /// `?buffer> state free`: module busy flags for perceptual-motor
    /// buffers, buffer flag otherwise. (Motor free = neither preparing
    /// nor executing; visual free = no attention shift in flight.)
    fn query_free(&self, buffer: &str) -> Result<bool, EngineError> {
        match buffer {
            "VISUAL" => Ok(!self.visual_busy),
            "MANUAL" => Ok(!self.man_prep_busy && !self.man_exec_busy),
            "IMAGINAL" => Ok(!self.imag_busy),
            "BLENDING" => Ok(!self.blending_busy),
            _ => Ok(!self
                .buffers
                .get(buffer)
                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?
                .busy),
        }
    }

    /// `?buffer> error t/nil`: only the blending module tracks an error
    /// flag; every other buffer reports no error.
    fn buffer_error(&self, buffer: &str) -> bool {
        buffer == "BLENDING" && self.blending_error
    }

    fn fire(&mut self, prod_idx: usize, bindings: &Bindings) -> Result<(), EngineError> {
        let prod = self.model.productions[prod_idx].clone();
        // Copies of chunk-valued `=buffer` refs are deduped within one
        // firing (GOTCHA #41): a modify and a request referencing the same
        // buffer share `NAME-0`.
        let mut copied: HashMap<String, String> = HashMap::new();
        // `!output!` binds run during firing, before action events.
        for action in &prod.rhs {
            if let RhsAction::Output { args } = action {
                let mut text = String::new();
                for a in args {
                    text.push_str(&self.show_output_arg(a, bindings)?);
                    text.push(' ');
                }
                self.out_lines.push(text);
            } else if let RhsAction::Other { head, .. } = action {
                // `!stop!` ends the run with a BREAK-EVENT.
                if head == "!STOP!" {
                    self.stopped = true;
                } else {
                    return Err(unimpl(format!("RHS {head} needs a Lisp evaluator")));
                }
            }
        }
        if self.stopped {
            return Ok(());
        }
        // Explicit actions in RHS order, then implicit harvest clears.
        for action in &prod.rhs {
            match action {
                RhsAction::Modify { buffer, slots } => {
                    // An empty `=buffer>` (no slots) is a no-op: ACT-R
                    // schedules no MOD event for it (e.g. `merging-test` P2).
                    if slots.is_empty() {
                        continue;
                    }
                    let copy = matches!(buffer.as_str(), "GOAL" | "IMAGINAL");
                    let mut sub = Vec::new();
                    for (s, p) in slots {
                        sub.push((s.clone(), self.rhs_value(p, bindings, copy, &mut copied)?));
                    }
                    self.now(
                        Prio::Num(100),
                        explicit_out(self.cfg.rhst),
                        Event::ModBuffer {
                            buffer: buffer.clone(),
                            slots: sub,
                        },
                    );
                }
                RhsAction::Request { buffer, spec } => {
                    let copy = matches!(buffer.as_str(), "GOAL" | "IMAGINAL");
                    let concrete = self.concretize(spec, bindings, copy, &mut copied)?;
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
                        },
                    );
                }
                RhsAction::Clear { buffer } => {
                    self.now(
                        Prio::Num(10),
                        Out::Medium,
                        Event::ClearBuffer {
                            buffer: buffer.clone(),
                        },
                    );
                }
                RhsAction::Output { .. } | RhsAction::Other { .. } => {}
            }
        }
        for buffer in &self.exec_prods[prod_idx].implicit_clears.clone() {
            self.now(
                Prio::Num(10),
                Out::Medium,
                Event::ClearBuffer {
                    buffer: buffer.clone(),
                },
            );
        }
        self.now(Prio::Min, Out::Medium, Event::ConflictResolution);
        // `production-fired` runs actions, then compiles
        // (`production-compilation.lisp`, called from `production-fired`).
        self.compile_productions(prod_idx, bindings);
        Ok(())
    }

    /// Chunk similarity for partial matching (`chunks-similarity`):
    /// explicit table value (either direction), else `ms` for
    /// identical/equal values, else `md`. Chunkhood is DM membership
    /// (an approximation documented in GOTCHAS).
    /// `compile-productions` (`production-compilation.lisp:1129`): compose
    /// the previous fired production with the current one and emit the
    /// `:epl`/`:pct` trace. This port covers the goal-style test+modify
    /// composition exercised by `ppm-compilation`; compiled productions are
    /// not (yet) added to the runtime set.
    fn compile_productions(&mut self, idx: usize, bindings: &Bindings) {
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
        if let Some((p1_idx, p1_bindings)) = self.prev_prod.clone() {
            let p1name = self.model.productions[p1_idx].name.clone();
            if self.time_ms - self.prev_prod_ms > self.cfg.tt_ms {
                if trace {
                    self.out(format!(
                        "  Cannot compile {p1name} and {p2name} because the time between them exceeds the threshold time."
                    ));
                }
            } else {
                let (lhs, rhs) = self.compose_pair(p1_idx, &p1_bindings, idx, bindings);
                if trace {
                    self.out(format!(
                        "  Production {p1name} and {p2name} are being composed."
                    ));
                    self.out("  New production:\n".to_string());
                    let cname = format!("PRODUCTION{}", self.compiled_count);
                    self.out(render_compiled(&cname, &p1name, &p2name, &lhs, &rhs));
                    self.out(format!("Parameters for production {cname}:"));
                    if self.cfg.esc {
                        // Compiled productions have no numeric utility yet.
                        self.out(" :utility    NIL".to_string());
                    }
                    self.out(" :u   0.000".to_string());
                    self.out(format!(
                        " :at {}",
                        fmt_6_3(self.cfg.dat_ms as f32 / 1000.0)
                    ));
                }
                self.compiled_count += 1;
            }
        } else if trace {
            self.out("  No previous production to compose with.".to_string());
        }
        self.prev_prod = Some((idx, bindings.clone()));
        self.prev_prod_ms = self.time_ms;
        if trace {
            let name = self.model.productions[idx].name.clone();
            self.out(format!("  Setting previous production to {name}."));
        }
    }

    /// Compose two productions per buffer (`buffer-condition-union`,
    /// production-compilation.lisp:2320): the first's conditions, then the
    /// second's conditions minus the slots the first modifies, with
    /// `remove-duplicates` keeping the later copy; the actions are the
    /// first's untouched slots followed by the second's.
    #[allow(clippy::type_complexity)]
    fn compose_pair(
        &self,
        i1: usize,
        b1: &Bindings,
        i2: usize,
        b2: &Bindings,
    ) -> (
        Vec<(String, Vec<(String, SlotVal)>)>,
        Vec<(String, Vec<(String, String)>)>,
    ) {
        let p1 = &self.model.productions[i1];
        let p2 = &self.model.productions[i2];
        let p1_lhs = prod_lhs_slots(p1, b1);
        let p2_lhs = prod_lhs_slots(p2, b2);
        let p1_mod = prod_rhs_mods(p1, b1);
        let p2_mod = prod_rhs_mods(p2, b2);

        // Cross-production variable mappings for same-slot `=` pairs
        // (`MAP-GOAL-BUFFER`, goal-compilation.lisp:320-330): two variables
        // unify onto the first's; a variable facing a constant is
        // instantiated from its own bindings.
        let mut map: HashMap<String, SlotVal> = HashMap::new();
        for (buf, slots1) in &p1_lhs {
            for (name1, v1) in slots1 {
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
        let subst = |v: &SlotVal| -> SlotVal {
            match v {
                SlotVal::Var(n) => map.get(n).cloned().unwrap_or_else(|| v.clone()),
                SlotVal::Lit(s) => SlotVal::Lit(s.clone()),
            }
        };
        let apply = |lhs: &[(String, Vec<(String, SlotVal)>)]| {
            lhs.iter()
                .map(|(b, s)| {
                    (
                        b.clone(),
                        s.iter()
                            .map(|(n, v)| (n.clone(), subst(v)))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let p1_lhs = apply(&p1_lhs);
        let p2_lhs = apply(&p2_lhs);

        let mut buffers: Vec<String> = Vec::new();
        for (b, _) in p1_lhs.iter().chain(&p2_lhs) {
            if !buffers.contains(b) {
                buffers.push(b.clone());
            }
        }
        for (b, _) in p1_mod.iter().chain(&p2_mod) {
            if !buffers.contains(b) {
                buffers.push(b.clone());
            }
        }
        buffers.sort();

        let find_lhs =
            |v: &[(String, Vec<(String, SlotVal)>)], b: &str| -> Vec<(String, SlotVal)> {
                v.iter()
                    .find(|(x, _)| x == b)
                    .map(|(_, s)| s.clone())
                    .unwrap_or_default()
            };
        let find_mod = |v: &[(String, Vec<(String, String)>)], b: &str| -> Vec<(String, String)> {
            v.iter()
                .find(|(x, _)| x == b)
                .map(|(_, s)| s.clone())
                .unwrap_or_default()
        };

        let mut lhs = Vec::new();
        for buf in &buffers {
            let c1 = find_lhs(&p1_lhs, buf);
            let c2 = find_lhs(&p2_lhs, buf);
            if c1.is_empty() && c2.is_empty() {
                continue;
            }
            let modified: Vec<String> = find_mod(&p1_mod, buf)
                .into_iter()
                .map(|(n, _)| n)
                .collect();
            let mut slots = c1;
            for (n, v) in c2 {
                if !modified.contains(&n) {
                    slots.push((n, v));
                }
            }
            lhs.push((format!("={buf}>"), remove_dup_keep_last(slots)));
        }

        let mut rhs = Vec::new();
        for buf in &buffers {
            let a1 = find_mod(&p1_mod, buf);
            let a2 = find_mod(&p2_mod, buf);
            let a2names: Vec<String> = a2.iter().map(|(n, _)| n.clone()).collect();
            let mut slots: Vec<(String, String)> = a1
                .into_iter()
                .filter(|(n, _)| !a2names.contains(n))
                .collect();
            slots.extend(a2);
            if !slots.is_empty() {
                rhs.push((format!("={buf}>"), slots));
            }
        }

        (lhs, rhs)
    }

    fn similarity(&self, a: &Value, b: &Value) -> f32 {
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

    fn start_retrieval(&mut self, spec: NewChunk, var_slots: &[String]) -> Result<(), EngineError> {
        let (want_slots, want_neg) = match &spec {
            NewChunk::Named(n) => {
                return Err(unimpl(format!("single-item retrieval request {n}")));
            }
            NewChunk::Instance(_) => {
                return Err(bad(
                    "retrieval request cannot be a buffer instance".to_string(),
                ));
            }
            NewChunk::Spec { slots, neg, .. } => (slots.clone(), neg.clone()),
        };
        // Partial matching (mp set, esc on): candidacy is the request's
        // slot signature — required-filled and required-empty slot names —
        // NOT the chunk type (`slots-vector-match-signature`,
        // framework/chunk-spec.lisp:895). `=`/`-` values score activation
        // instead of filtering. Otherwise exact matching as before.
        let partial = self.cfg.esc && self.cfg.mp.is_some();
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
        let mut best_a: Option<f32> = None;
        let mut tied: Vec<ChunkInst> = Vec::new();
        for idx in 0..self.dm.len() {
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
            if self.act_medium() {
                self.out(format!("Computing activation for chunk {cname}"));
                self.out("Computing base-level".to_string());
            }
            // A = base-level + spreading + partial matching + activation
            // noise. Base-level from the decay equation when :bll is set,
            // else blc. Noise drawn per candidate in DM order.
            let base = match self.cfg.bll {
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
                        self.cfg.blc
                            + bl_equation(
                                self.time_ms,
                                &refs,
                                count as f32,
                                creation,
                                bll,
                                &self.cfg.ol,
                                self.cfg.act_scale,
                            )
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
                self.spreading_activation(&cname)
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
            if let Some(mp) = self.cfg.mp {
                for (s, v) in &want_slots {
                    if s.starts_with(':') || var_slots.contains(s) || *v == Value::Nil {
                        continue;
                    }
                    a += mp * self.similarity(v, &c.get(s));
                }
                for (s, v) in &want_neg {
                    if s.starts_with(':') || var_slots.contains(s) || *v == Value::Nil {
                        continue;
                    }
                    if self.similarity(v, &c.get(s)) == self.cfg.ms {
                        a += mp * self.cfg.md;
                    }
                }
            }
            let noise = match self.cfg.ans {
                Some(s) => self.rng.noise(s),
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
                let i = self.rng.random_int(tied.len() as u64) as usize;
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
        if self.act_low() {
            if let Some(w) = &best {
                let wname = w.name.clone().unwrap_or_default();
                self.out(format!(
                    "Chunk {wname} with activation {} is the best",
                    lisp_f(best_a.unwrap_or(0.0))
                ));
            }
        }
        let act = best_a.unwrap_or(0.0);
        let gen = self.dm_gen;
        match best {
            None => {
                let wait = failure_wait(&self.cfg);
                let at = self.time_ms + wait;
                self.push(at, Prio::Num(0), Out::Low, Event::RetrievalFailure { gen });
            }
            Some(_) if self.cfg.esc && act < self.cfg.rt => {
                let wait = failure_wait(&self.cfg);
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

impl Engine {
    /// Filled-slot names of a chunk (`chunk-slots-vector` mask, as names):
    /// non-nil slots only; `isa` is the chunk type, not a slot.
    fn filled_slots(&self, c: &ChunkInst) -> Vec<String> {
        c.slots
            .iter()
            .filter(|(_, v)| *v != Value::Nil)
            .map(|(s, _)| s.clone())
            .collect()
    }

    fn find_chunk(&self, name: &str) -> Option<&ChunkInst> {
        self.dm
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .or_else(|| self.named.get(name))
    }

    /// DM chunk order as `all-dm-chunks` (`declarative-memory.lisp:2747`):
    /// `dm-chunks` is an alist of signature -> chunks, built by pushing new
    /// signature groups and new chunks to the front, then flattened. So
    /// newest signature group first, newest chunk first within a group.
    fn dm_alist_order(&self) -> Vec<usize> {
        let mut groups: Vec<(Vec<String>, Vec<usize>)> = Vec::new();
        for (i, c) in self.dm.iter().enumerate() {
            let mut sig = self.filled_slots(c);
            sig.sort();
            if let Some(g) = groups.iter_mut().find(|(s, _)| *s == sig) {
                g.1.insert(0, i);
            } else {
                groups.insert(0, (sig, vec![i]));
            }
        }
        groups.into_iter().flat_map(|(_, v)| v).collect()
    }

    /// Global slot index order from chunk-type declarations (first
    /// appearance wins), mirroring `slot-name->index`.
    fn slot_index_map(&self) -> HashMap<String, usize> {
        let mut m: HashMap<String, usize> = HashMap::new();
        let mut n = 0usize;
        for ct in &self.model.chunk_types {
            for s in &ct.slots {
                if !m.contains_key(s) {
                    m.insert(s.clone(), n);
                    n += 1;
                }
            }
        }
        m
    }

    /// `slot-mask->names`: the given names in slot-index order.
    fn names_by_index(names: &HashSet<String>, idx: &HashMap<String, usize>) -> Vec<String> {
        let mut v: Vec<String> = names.iter().cloned().collect();
        v.sort_by_key(|s| idx.get(s).copied().unwrap_or(usize::MAX));
        v
    }

    /// Numeric `> < >= <=` against runtime values.
    fn rel_test_v(&self, op: SlotOp, rhs: &Value, got: &Value) -> bool {
        let rhs = match rhs {
            Value::Int(i) => *i as f64,
            Value::Num(x) => *x,
            _ => return false,
        };
        let lhs = match got {
            Value::Int(i) => *i as f64,
            Value::Num(x) => *x,
            _ => return false,
        };
        match op {
            SlotOp::Gt => lhs > rhs,
            SlotOp::Lt => lhs < rhs,
            SlotOp::Ge => lhs >= rhs,
            SlotOp::Le => lhs <= rhs,
            _ => false,
        }
    }

    /// Full `find-matching-chunks` test of a chunk against a request spec
    /// (`=`, `-` and the relative slots). Nil values are signature-only.
    fn chunk_matches_req(
        &self,
        c: &ChunkInst,
        slots: &[(String, Value)],
        neg: &[(String, Value)],
        rels: &[(String, Value, SlotOp)],
    ) -> bool {
        for (s, v) in slots {
            if s.starts_with(':') || *v == Value::Nil {
                continue;
            }
            if !self.value_eq(&c.get(s), v) {
                return false;
            }
        }
        for (s, v) in neg {
            if s.starts_with(':') {
                continue;
            }
            if *v == Value::Nil {
                if c.get(s) == Value::Nil {
                    return false;
                }
            } else if self.value_eq(&c.get(s), v) {
                return false;
            }
        }
        for (s, v, op) in rels {
            if s.starts_with(':') {
                continue;
            }
            if !self.rel_test_v(*op, v, &c.get(s)) {
                return false;
            }
        }
        true
    }

    /// `compute-activation` for a blending candidate: base + spreading +
    /// partial matching + one logistic noise draw. `:min-bl` suppression is
    /// applied by the caller (but the draw still happens, as in Lisp).
    fn compute_blending_activation(
        &mut self,
        idx: usize,
        want_slots: &[(String, Value)],
        want_neg: &[(String, Value)],
    ) -> (f32, f32) {
        let base = match self.cfg.bll {
            Some(bll) => {
                let (refs, creation, count) = {
                    let h = &self.hist[idx];
                    (h.refs.clone(), h.creation_ms, h.count)
                };
                if count == 0 {
                    -999999.0
                } else {
                    self.cfg.blc
                        + bl_equation(
                            self.time_ms,
                            &refs,
                            count as f32,
                            creation,
                            bll,
                            &self.cfg.ol,
                            self.cfg.act_scale,
                        )
                }
            }
            None => self.hist[idx].fixed_base.unwrap_or(self.cfg.blc),
        };
        let name = self.dm[idx].name.clone().unwrap_or_default();
        let spread = if self.dm[idx].name.is_some() {
            self.spreading_activation(&name)
        } else {
            0.0
        };
        let mut a = base + spread;
        if let Some(mp) = self.cfg.mp {
            for (s, v) in want_slots {
                if s.starts_with(':') || *v == Value::Nil {
                    continue;
                }
                a += mp * self.similarity(v, &self.dm[idx].get(s));
            }
            for (s, v) in want_neg {
                if s.starts_with(':') || *v == Value::Nil {
                    continue;
                }
                if self.similarity(v, &self.dm[idx].get(s)) == self.cfg.ms {
                    a += mp * self.cfg.md;
                }
            }
        }
        let noise = match self.cfg.ans {
            Some(s) => self.rng.noise(s),
            None => 0.0,
        };
        (a + noise, base)
    }

    /// `start-blending` (`extras/blending/blending.lisp:474`). Operates on
    /// the concretized request spec; emits the `:blt` trace and schedules
    /// either the completion (with the blended chunk) or the failure.
    fn start_blending(&mut self, spec: &NewChunk) -> Result<(), EngineError> {
        let (slots, neg, rels) = match spec {
            NewChunk::Spec {
                slots,
                neg,
                rels,
                ..
            } => (slots.clone(), neg.clone(), rels.clone()),
            _ => return Err(bad("blending request must be a chunk spec".to_string())),
        };
        let idx_map = self.slot_index_map();

        // Request signature (`add-slot-spec-to-chunk-spec`): `= V` (V
        // non-nil) and `- nil` are filled; `= nil` is empty.
        let mut req_filled: HashSet<String> = HashSet::new();
        let mut req_empty: HashSet<String> = HashSet::new();
        for (s, v) in &slots {
            if s.starts_with(':') {
                continue;
            }
            if *v == Value::Nil {
                req_empty.insert(s.clone());
            } else {
                req_filled.insert(s.clone());
            }
        }
        for (s, v) in &neg {
            if s.starts_with(':') {
                continue;
            }
            if *v == Value::Nil {
                req_filled.insert(s.clone());
            }
        }
        for (s, _, _) in &rels {
            if !s.starts_with(':') {
                req_filled.insert(s.clone());
            }
        }
        let filled_names = Self::names_by_index(&req_filled, &idx_map);
        let empty_names = Self::names_by_index(&req_empty, &idx_map);

        // Candidates: DM order filtered by the request signature.
        let chunk_list: Vec<usize> = self
            .dm_alist_order()
            .into_iter()
            .filter(|i| {
                let fs = self.filled_slots(&self.dm[*i]);
                req_filled.iter().all(|f| fs.contains(f))
                    && req_empty.iter().all(|e| !fs.contains(e))
            })
            .collect();

        // `find-matching-chunks`: full spec unless (esc && mp), in which
        // case only the relative slots filter; the pushing loop reverses.
        let partial = self.cfg.esc && self.cfg.mp.is_some();
        let mut matched: Vec<usize> = Vec::new();
        for i in &chunk_list {
            let c = &self.dm[*i];
            let ok = if partial {
                rels.iter().all(|(s, v, op)| {
                    s.starts_with(':') || self.rel_test_v(*op, v, &c.get(s))
                })
            } else {
                self.chunk_matches_req(c, &slots, &neg, &rels)
            };
            if ok {
                matched.push(*i);
            }
        }
        matched.reverse();

        // Blended slots: union of matching chunks' filled slots minus the
        // fixed (`=`) slots unless `:blend-all-slots`.
        let mut all_slots: HashSet<String> = HashSet::new();
        for i in &matched {
            for s in self.filled_slots(&self.dm[*i]) {
                all_slots.insert(s);
            }
        }
        let fixed: HashSet<String> = slots
            .iter()
            .filter(|(s, _)| !s.starts_with(':'))
            .map(|(s, _)| s.clone())
            .collect();
        let mut blended_set: HashSet<String> = if self.cfg.blend_all_slots {
            all_slots
        } else {
            all_slots.difference(&fixed).cloned().collect()
        };
        blended_set.retain(|s| !s.starts_with(':'));
        let blended_slots = Self::names_by_index(&blended_set, &idx_map);

        // Temperature.
        let (temperature, temp_line) = if let Some(t) = self.cfg.tmp_ {
            (t, format!("Blending temperature is: {}", lisp_s_f32(t)))
        } else if let Some(a) = self.cfg.ans {
            let t = std::f32::consts::SQRT_2 * a;
            (
                t,
                format!(
                    "Blending temperature defaults to (* (sqrt 2) :ans): {}",
                    lisp_s_f32(t)
                ),
            )
        } else {
            (
                1.0,
                "Blending temperature defaults to 1 when :ans and :tmp are both nil"
                    .to_string(),
            )
        };

        // Activations (one noise draw per matching chunk, in match order).
        let mut act_list: Vec<(f32, usize, String)> = Vec::new();
        for i in &matched {
            let (a, base) = self.compute_blending_activation(*i, &slots, &neg);
            // `:min-bl`: a chunk whose base level is below the minimum is
            // suppressed (its activation and noise draw already happened).
            if let Some(minb) = self.cfg.min_bl {
                if base < minb {
                    continue;
                }
            }
            let name = self.dm[*i].name.clone().unwrap_or_default();
            act_list.push((a, *i, name));
        }

        if self.cfg.blt {
            let mut line = "Blending request for chunks".to_string();
            if !filled_names.is_empty() {
                line.push_str(&format!(" with slots ({})", filled_names.join(" ")));
            }
            line.push(' ');
            if !empty_names.is_empty() {
                line.push_str(&format!("without slots ({})", empty_names.join(" ")));
            }
            self.out(line);
            self.out(temp_line);
        }

        if act_list.is_empty() {
            if self.cfg.blt {
                self.out("No matching chunks found.".to_string());
                self.out("Blending request fails.".to_string());
            }
            let wait = failure_wait(&self.cfg);
            self.push(
                self.time_ms + wait,
                Prio::Num(0),
                Out::Low,
                Event::BlendingFailure,
            );
            return Ok(());
        }

        // Softmax probabilities over the activations.
        let max_ai = act_list
            .iter()
            .map(|(a, _, _)| *a)
            .fold(f32::NEG_INFINITY, f32::max);
        let mut probs: Vec<f32> = act_list
            .iter()
            .map(|(a, _, _)| ((a - max_ai) / temperature).exp())
            .collect();
        let sum: f32 = probs.iter().sum();
        for p in probs.iter_mut() {
            *p /= sum;
        }

        if self.cfg.blt {
            for ((a, _, name), p) in act_list.iter().zip(probs.iter()) {
                self.out(format!(
                    "Chunk {name} matches blending request\n  Activation {}\n  Probability of recall {}\n",
                    lisp_s_f32(*a),
                    lisp_s_f32(*p)
                ));
            }
            self.out(format!(
                "\nSlots to be blended: ({})",
                blended_slots.join(" ")
            ));
        }

        // Per-slot blending. `results` keeps blended-slots order.
        let mut results: Vec<(String, Option<Value>)> = blended_slots
            .iter()
            .map(|s| (s.clone(), None))
            .collect();

        for slot in &blended_slots {
            if self.cfg.blt {
                self.out(format!("Finding blended value for slot: {slot}"));
            }
            // possible-values: (name, value, prob, magnitude) for each
            // activation chunk with a non-nil slot value; magnitude is
            // `:value->mag` (default identity).
            let possible: Vec<(String, Value, f32, Value)> = act_list
                .iter()
                .zip(probs.iter())
                .filter_map(|((_, i, name), p)| {
                    let v = self.dm[*i].get(slot);
                    if v == Value::Nil {
                        None
                    } else {
                        Some((name.clone(), v.clone(), *p, v))
                    }
                })
                .collect();
            let slot_vals: Vec<Value> = possible.iter().map(|(_, v, _, _)| v.clone()).collect();
            let mags: Vec<Value> = possible.iter().map(|(_, _, _, m)| m.clone()).collect();
            if self.cfg.blt {
                self.out(format!(
                    "Matched chunks' slots contain: {}",
                    show_values(&slot_vals)
                ));
                self.out(format!(
                    "Magnitude values for those items: {}",
                    show_values(&mags)
                ));
            }

            let true_mags: Vec<Value> =
                mags.iter().filter(|m| **m != Value::Nil).cloned().collect();

            if true_mags.is_empty() {
                if self.cfg.blt {
                    self.out(
                        "When all magnitudes are nil there's nothing to blend and the slot is ignored"
                            .to_string(),
                    );
                }
                results.retain(|(s, _)| s != slot);
                continue;
            }

            if true_mags.iter().all(is_number) {
                if self.cfg.blt {
                    self.out("With numeric magnitudes blending by weighted average".to_string());
                }
                let mut sum = 0.0f32;
                for (name, _v, p, m) in &possible {
                    if *m == Value::Nil {
                        continue;
                    }
                    let mag = f32_of(m).unwrap_or(0.0);
                    let inc = p * mag;
                    sum += inc;
                    if self.cfg.blt {
                        self.out(format!(
                            " Chunk {name} with probability {} times magnitude {} = {} cumulative result: {}",
                            lisp_s_f32(*p),
                            lisp_s_f32(mag),
                            lisp_s_f32(inc),
                            lisp_s_f32(sum)
                        ));
                    }
                }
                set_result(&mut results, slot, Value::Num(sum as f64));
                if self.cfg.blt {
                    self.out(format!(" Final result: {}", lisp_s_f32(sum)));
                }
                continue;
            }

            // Chunk (or mixed) magnitudes: pick the candidate value whose
            // similarity-weighted probability mass is smallest.
            let which_chunks = true_mags.iter().all(|m| self.chunk_name_of(m).is_some());
            if self.cfg.blt {
                if which_chunks {
                    self.out(
                        "When all magnitudes are chunks blending based on similarities to all related chunks"
                            .to_string(),
                    );
                } else {
                    self.out(format!(
                        "When not all magnitudes are numbers or chunks blending based on similarities using those unique values: {}",
                        show_values(&true_mags)
                    ));
                }
            }
            let chunks: Vec<Value> = if which_chunks {
                // Intersection of the magnitudes' filled-slot sets.
                let mut inter: Option<HashSet<String>> = None;
                for m in &true_mags {
                    if let Some(n) = self.chunk_name_of(m) {
                        if let Some(c) = self.find_chunk(&n) {
                            let s: HashSet<String> = self.filled_slots(c).into_iter().collect();
                            inter = Some(match inter {
                                None => s,
                                Some(x) => x.intersection(&s).cloned().collect(),
                            });
                        }
                    }
                }
                let inter = inter.unwrap_or_default();
                if inter.is_empty() {
                    if self.cfg.blt {
                        self.out(
                            "No intersecting slots found all chunks will be tested".to_string(),
                        );
                    }
                    self.dm_alist_order()
                        .into_iter()
                        .map(|i| Value::Sym(self.dm[i].name.clone().unwrap_or_default()))
                        .collect()
                } else {
                    if self.cfg.blt {
                        self.out(format!(
                            "Intersection of slots for values is: ({})",
                            Self::names_by_index(&inter, &idx_map).join(" ")
                        ));
                    }
                    let list: Vec<Value> = self
                        .dm_alist_order()
                        .into_iter()
                        .filter(|i| {
                            let fs = self.filled_slots(&self.dm[*i]);
                            inter.iter().all(|f| fs.contains(f))
                        })
                        .map(|i| Value::Sym(self.dm[i].name.clone().unwrap_or_default()))
                        .collect();
                    if self.cfg.blt {
                        self.out(format!(
                            "Chunks from DM with those slots: {}",
                            show_values(&list)
                        ));
                    }
                    list
                }
            } else {
                let mut uniq: Vec<Value> = Vec::new();
                for m in &true_mags {
                    if !uniq.iter().any(|u| self.value_eq(u, m)) {
                        uniq.push(m.clone());
                    }
                }
                uniq
            };

            let best = if chunks.len() == 1 {
                if self.cfg.blt {
                    self.out(format!(" Only one possible value {}", chunks[0].show()));
                }
                chunks[0].clone()
            } else {
                let mut best_val: Option<Value> = None;
                let mut best_mag: Option<f32> = None;
                for val in &chunks {
                    if self.cfg.blt {
                        self.out(format!(" Comparing value {}", val.show()));
                    }
                    let mut sum = 0.0f32;
                    for (name, v, p, m) in &possible {
                        let sim = self.similarity(val, m);
                        sum += p * (sim * sim);
                        if self.cfg.blt {
                            self.out(format!(
                                "  Chunk {name} with probability {} slot value {} similarity: {} cumulative result: {}",
                                lisp_s_f32(*p),
                                v.show(),
                                lisp_s_f32(sim),
                                lisp_s_f32(sum)
                            ));
                        }
                    }
                    if best_mag.is_none() || sum < best_mag.unwrap() {
                        best_mag = Some(sum);
                        best_val = Some(val.clone());
                    }
                }
                best_val.unwrap_or(Value::Nil)
            };
            if self.cfg.blt {
                self.out(format!(" Final result: {}", best.show()));
            }
            set_result(&mut results, slot, best);
        }

        // Build the blended chunk def: fixed values (unless blend-all)
        // followed by the blended values, in blended-slots order.
        let mut new_slots: Vec<(String, Value)> = Vec::new();
        if !self.cfg.blend_all_slots {
            for (s, v) in &slots {
                if s.starts_with(':') {
                    continue;
                }
                new_slots.push((s.clone(), v.clone()));
            }
        }
        for (s, opt) in &results {
            if let Some(v) = opt {
                new_slots.push((s.clone(), v.clone()));
            }
        }

        if self.cfg.blt {
            let mut flat: Vec<String> = Vec::new();
            for (s, v) in &new_slots {
                flat.push(s.clone());
                flat.push(v.show());
            }
            self.out(format!(
                "This is the definition of the blended chunk:\n({})",
                flat.join(" ")
            ));
            self.out("\nComputing activation and latency for the blended chunk".to_string());
        }

        let mut act = 0.0f32;
        for (a, _, name) in &act_list {
            act += a.exp();
            if self.cfg.blt {
                self.out(format!(" Activation of chunk {name} is {}", lisp_s_f32(*a)));
            }
        }
        act = act.ln();
        if self.cfg.blt {
            self.out(format!(
                "Activation for blended chunk is: {}",
                lisp_s_f32(act)
            ));
        }

        let blended = NewChunk::Spec {
            ctype: String::new(),
            slots: new_slots,
            neg: Vec::new(),
            rels: Vec::new(),
            bare: true,
        };

        if act >= self.cfg.rt {
            let wait = latency(self.cfg.lf_ms, self.cfg.le, act);
            self.push(
                self.time_ms + wait,
                Prio::Num(0),
                Out::Medium,
                Event::BlendingComplete { spec: blended },
            );
        } else {
            if self.cfg.blt {
                self.out("Not above threshold so blending failed".to_string());
            }
            let wait = failure_wait(&self.cfg);
            self.push(
                self.time_ms + wait,
                Prio::Num(0),
                Out::Low,
                Event::BlendingFailure,
            );
        }
        Ok(())
    }
}

/// A composed condition/action slot value: an as-yet-uninstantiated LHS
/// variable (`=s`) or a literal string.
#[derive(Clone, PartialEq, Debug)]
enum SlotVal {
    Var(String),
    Lit(String),
}

impl SlotVal {
    fn render(&self) -> String {
        match self {
            SlotVal::Var(v) => format!("={v}"),
            SlotVal::Lit(s) => s.clone(),
        }
    }
}

/// Instantiated LHS match slots per buffer, sorted by slot name
/// (`sort-production-slots`). `isa` is the chunk type, not a slot. Values
/// stay symbolic (`=var`) so cross-production mappings can apply first.
fn prod_lhs_slots(p: &Production, _b: &Bindings) -> Vec<(String, Vec<(String, SlotVal)>)> {
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
                            Value::from_pattern(other).map(|x| x.show()).unwrap_or_default(),
                        ),
                    };
                    (t.slot.clone(), v)
                })
                .collect();
            slots.sort_by(|a, b| a.0.cmp(&b.0));
            out.push((buffer.clone(), slots));
        }
    }
    out
}

/// The printed value bound to an LHS variable, if bound.
fn instantiate_var(name: &str, b: &Bindings) -> Option<String> {
    b.get(name).map(|v| v.show())
}

/// Instantiated RHS `=` modify slots per buffer.
fn prod_rhs_mods(p: &Production, b: &Bindings) -> Vec<(String, Vec<(String, String)>)> {
    let mut out = Vec::new();
    for a in &p.rhs {
        if let RhsAction::Modify { buffer, slots } = a {
            let s = slots
                .iter()
                .map(|(n, v)| (n.clone(), show_pat(v, b)))
                .collect();
            out.push((buffer.clone(), s));
        }
    }
    out
}

/// CL `remove-duplicates` ordering: drop earlier duplicates, keeping the
/// later copy, preserving the original order of the kept elements.
fn remove_dup_keep_last<T: PartialEq + Clone>(v: Vec<T>) -> Vec<T> {
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
fn render_compiled(
    name: &str,
    p1: &str,
    p2: &str,
    lhs: &[(String, Vec<(String, SlotVal)>)],
    rhs: &[(String, Vec<(String, String)>)],
) -> String {
    let mut s = format!("(P {name}\n  \"{p1} & {p2}\"\n");
    for (token, slots) in lhs {
        s.push_str(&format!("   {token}\n"));
        for (n, v) in slots {
            s.push_str(&format!("       {n} {}\n", v.render()));
        }
    }
    s.push_str(" ==>\n");
    for (token, slots) in rhs {
        s.push_str(&format!("   {token}\n"));
        for (n, v) in slots {
            s.push_str(&format!("       {n} {v}\n"));
        }
    }
    s.push(')');
    s
}

/// `(A B C)` from a slice of values (`~S` of a list).
fn show_values(vals: &[Value]) -> String {
    format!(
        "({})",
        vals.iter().map(|v| v.show()).collect::<Vec<_>>().join(" ")
    )
}

/// `~6,3F` — fixed 3 decimals, minimum width 6 (space padded).
fn fmt_6_3(x: f32) -> String {
    format!("{:>6.3}", x)
}

/// Instantiate a pattern for `print-production-text`: bound variables show
/// their value, unbound ones keep `=NAME`.
fn show_pat(p: &Pattern, b: &Bindings) -> String {
    match p {
        Pattern::Var(v) => match b.get(v) {
            Some(val) => val.show(),
            None => format!("={v}"),
        },
        other => Value::from_pattern(other).map(|v| v.show()).unwrap_or_default(),
    }
}

/// Instantiate a raw RHS `!output!`/`!action!` argument.
fn show_sexp_arg(a: &Sexp, b: &Bindings) -> String {
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

fn slot_op_str(op: SlotOp) -> &'static str {
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
fn render_production(prod: &Production, b: &Bindings, partials: &[PpmMismatch]) -> String {
    let mut s = format!("(P {}\n", prod.name);
    for item in &prod.lhs {
        match item {
            LhsItem::Match { buffer, tests } => {
                s.push_str(&format!("   ={buffer}>\n"));
                for t in tests {
                    if t.slot == "ISA" {
                        continue;
                    }
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
                                t.slot,
                                val,
                                actual.show(),
                                lisp_s_f32(*sim)
                            ));
                        } else {
                            s.push_str(&format!("       {} {val}\n", t.slot));
                        }
                    } else {
                        s.push_str(&format!(
                            "    {:<2} {} {val}\n",
                            slot_op_str(t.op),
                            t.slot
                        ));
                    }
                }
            }
            LhsItem::Query { buffer, queries } => {
                s.push_str(&format!("   ?{buffer}>\n"));
                for (attr, val) in queries {
                    s.push_str(&format!("       {attr} {}\n", show_pat(val, b)));
                }
            }
        }
    }
    s.push_str(" ==>\n");
    for a in &prod.rhs {
        match a {
            RhsAction::Modify { buffer, slots } => {
                s.push_str(&format!("   ={buffer}>\n"));
                for (name, val) in slots {
                    s.push_str(&format!("       {name} {}\n", show_pat(val, b)));
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
                        s.push_str(&format!("       {name} {}\n", show_pat(val, b)));
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
            RhsAction::Output { args } => {
                let body: Vec<String> = args.iter().map(|a| show_sexp_arg(a, b)).collect();
                s.push_str(&format!("   !OUTPUT! {}\n", body.join(" ")));
            }
            RhsAction::Other { head, args } => {
                let name = head.trim_matches('!');
                let body: Vec<String> = args.iter().map(|a| show_sexp_arg(a, b)).collect();
                s.push_str(&format!("   !{name}! {}\n", body.join(" ")));
            }
        }
    }
    s.push(')');
    s
}

fn is_number(v: &Value) -> bool {
    matches!(v, Value::Int(_) | Value::Num(_))
}

fn f32_of(v: &Value) -> Option<f32> {
    match v {
        Value::Int(i) => Some(*i as f32),
        Value::Num(x) => Some(*x as f32),
        _ => None,
    }
}

fn set_result(results: &mut [(String, Option<Value>)], slot: &str, v: Value) {
    if let Some(e) = results.iter_mut().find(|(s, _)| s == slot) {
        e.1 = Some(v);
    }
}

/// Failure latency uses the threshold: `round(lf * e^(-le*rt))`.
fn failure_wait(cfg: &Config) -> i64 {
    if cfg.esc {
        latency(cfg.lf_ms, cfg.le, cfg.rt)
    } else {
        0
    }
}

/// `round(lf_ms * e^(-le*A))`, all single-float like the Lisp
/// (`compute-activation-latency`; `:lf` is stored as integer ms).
/// Rounds halves to even, as CL `round` does (units table ROUND rows).
fn latency(lf_ms: i64, le: f32, act: f32) -> i64 {
    lisp_round((lf_ms as f32) * (-le * act).exp())
}

impl Engine {
    /// Device request routing for the visual buffer: only
    /// move-attention in M5a (plus finst assignment later).
    fn visual_request(&mut self, spec: NewChunk) -> Result<(), EngineError> {
        let (slots, ctype_ok) = match &spec {
            NewChunk::Spec { ctype, slots, .. } => (slots.clone(), ctype.is_empty()),
            _ => return Err(unimpl("non-spec visual request")),
        };
        let _ = ctype_ok;
        let mut cmd: Option<String> = None;
        let mut screen_pos: Option<String> = None;
        let mut scale: Option<String> = None;
        for (s, v) in &slots {
            match (s.as_str(), v) {
                ("CMD", Value::Sym(c)) => cmd = Some(c.clone()),
                ("SCREEN-POS", Value::Sym(n)) => screen_pos = Some(n.clone()),
                ("SCALE", Value::Sym(n)) => scale = Some(n.clone()),
                got => {
                    return Err(unimpl(format!("visual request slot {got:?}")));
                }
            }
        }
        match cmd.as_deref() {
            Some("MOVE-ATTENTION") => {
                let loc = screen_pos
                    .ok_or_else(|| bad("move-attention without screen-pos".to_string()))?;
                if !self.loc_known(&loc) {
                    return Err(bad(format!("screen-pos {loc} is not a visual location")));
                }
                let _ = scale;
                self.now(Prio::Num(0), Out::Always, Event::MoveAttention { loc });
                Ok(())
            }
            // `+visual> cmd clear`: direct clear event (output low);
            // the buffer itself is cleared by the request-rule implicit
            // ClearBuffer already queued at firing.
            Some("CLEAR") => {
                self.now(Prio::Num(0), Out::Low, Event::VisualClear);
                Ok(())
            }
            got => Err(unimpl(format!("visual command {got:?}"))),
        }
    }

    /// Manual request routing: only press-key in M5a.
    fn manual_request(&mut self, spec: NewChunk) -> Result<(), EngineError> {
        let slots = match &spec {
            NewChunk::Spec { slots, .. } => slots.clone(),
            _ => return Err(unimpl("non-spec manual request")),
        };
        let mut cmd: Option<String> = None;
        let mut key: Option<String> = None;
        for (s, v) in &slots {
            match (s.as_str(), v) {
                ("CMD", Value::Sym(c)) => cmd = Some(c.clone()),
                ("KEY", Value::Sym(k)) => key = Some(k.clone()),
                ("KEY", Value::Str(k)) => key = Some(k.clone()),
                got => {
                    return Err(unimpl(format!("manual request slot {got:?}")));
                }
            }
        }
        match (cmd.as_deref(), key) {
            (Some("PRESS-KEY"), Some(k)) => {
                Self::key_spec(&k)?;
                let req_ms = self.time_ms;
                self.now(Prio::Num(0), Out::Low, Event::PressKey { key: k, req_ms });
                Ok(())
            }
            got => Err(unimpl(format!("manual request {got:?}"))),
        }
    }

    fn loc_known(&self, loc: &str) -> bool {
        self.vl_copies.contains_key(loc)
            || self.features.iter().any(|f| f.name == loc)
            || self
                .buffers
                .get("VISUAL-LOCATION")
                .and_then(|b| b.chunk.as_ref())
                .and_then(|c| c.name.as_deref())
                == Some(loc)
    }

    /// A location name (feature base, request copy, or buffer chunk) to
    /// its feature index.
    fn resolve_loc(&self, loc: &str) -> Result<usize, EngineError> {
        if let Some(&idx) = self.vl_copies.get(loc) {
            return Ok(idx);
        }
        if let Some(idx) = self.features.iter().position(|f| f.name == loc) {
            return Ok(idx);
        }
        Err(bad(format!("screen-pos {loc} is not a visual location")))
    }

    /// proc-display: move due device texts into the visicon, stuff the
    /// visual-location buffer if empty, and note the update.
    fn proc_display(&mut self) -> Result<(), EngineError> {
        let now = self.time_ms;
        let mut due = Vec::new();
        self.pending_texts.retain(|t| {
            if t.at_ms <= now {
                due.push(t.clone());
                false
            } else {
                true
            }
        });
        let mut cleared = false;
        self.pending_clears.retain(|c| {
            if *c <= now {
                cleared = true;
                false
            } else {
                true
            }
        });
        if cleared {
            self.features.clear();
        }
        for t in due {
            let n = self.feat_seq;
            self.feat_seq += 1;
            self.features.push(Feature {
                name: format!("VISUAL-LOCATION{n}"),
                kind: "TEXT".to_string(),
                value: t.text.clone(),
                x: t.x,
                y: t.y,
                tstamp_ms: now,
                attended: false,
            });
        }
        let empty = self
            .buffers
            .get("VISUAL-LOCATION")
            .and_then(|b| b.chunk.as_ref())
            .is_none();
        if empty {
            if let Some(feat) = self.newest_feature() {
                let base = feat.name.clone();
                self.place_vl(&base, false)?;
            }
        }
        self.now(Prio::Num(0), Out::Medium, Event::VisiconUpdate);
        // Schedule the next wave for remaining texts or clears.
        let mut next_at: Option<i64> = None;
        if let Some(next) = self.pending_texts.first() {
            next_at = Some(next.at_ms);
        }
        if let Some(clear) = self.pending_clears.first() {
            next_at = Some(next_at.map_or(*clear, |n| n.min(*clear)));
        }
        if let Some(at) = next_at {
            self.push_m(at, Prio::Num(-100), Out::Always, true, Event::ProcDisplay);
        }
        Ok(())
    }

    /// Place a visual-location chunk for a feature: the trace shows the
    /// requested base name, but the buffer holds a `-k` copy (which is
    /// what later requests bind). Aliased back to the feature.
    fn place_vl(&mut self, base: &str, requested: bool) -> Result<(), EngineError> {
        let idx = self
            .features
            .iter()
            .position(|f| f.name == base)
            .ok_or_else(|| bad(format!("no such visicon feature {base}")))?;
        let feat = self.features[idx].clone();
        // Copy counter starts at 0 and advances after use
        // (`new-name-fct` semantics; existing-name skips cannot occur
        // in fresh models).
        let n = *self.copy_count.entry(base.to_string()).or_insert(0);
        self.copy_count.insert(base.to_string(), n + 1);
        let copy = format!("{base}-{n}");
        self.vl_copies.insert(copy.clone(), idx);
        let inst = ChunkInst {
            name: Some(copy),
            ctype: "VISUAL-LOCATION".to_string(),
            slots: feat.loc_slots(),
            bare: false,
        };
        self.buffer_mut("VISUAL-LOCATION").failed = false;
        self.now(
            Prio::Num(10),
            Out::Low,
            Event::SetBuffer {
                buffer: "VISUAL-LOCATION".to_string(),
                chunk: NewChunk::Instance(inst),
                requested,
                shown: Some(base.to_string()),
            },
        );
        Ok(())
    }

    fn newest_feature(&mut self) -> Option<Feature> {
        let top = self.features.iter().map(|f| f.tstamp_ms).max()?;
        let mut names: Vec<String> = self
            .features
            .iter()
            .filter(|f| f.tstamp_ms == top)
            .map(|f| f.name.clone())
            .collect();
        names.sort();
        // `random-item` consumes exactly one draw, even for singletons.
        let pick = self.rng_draw(names.len());
        let want = names[pick].clone();
        self.features.iter().find(|f| f.name == want).cloned()
    }

    /// Random index uniform in 0..n, always consuming exactly one draw
    /// (`random-item` draws even for singletons).
    fn rng_draw(&mut self, n: usize) -> usize {
        self.rng.random_int(n as u64) as usize
    }

    /// Find-location: newest-tstamp candidates, random among them,
    /// construct a fresh (dedup-named) location chunk into the buffer.
    fn find_location(&mut self, spec: NewChunk) -> Result<(), EngineError> {
        let want = match &spec {
            NewChunk::Spec { slots, .. } => slots.clone(),
            _ => return Err(unimpl("non-spec visual-location request")),
        };
        let mut cands: Vec<Feature> = Vec::new();
        for f in &self.features {
            if loc_matches(f, &want)? {
                cands.push(f.clone());
            }
        }
        if cands.is_empty() {
            self.buffer_mut("VISUAL-LOCATION").failed = true;
            self.now(Prio::Num(0), Out::Low, Event::FindLocFailure);
            return Ok(());
        }
        let top = cands.iter().map(|f| f.tstamp_ms).max().unwrap();
        let mut names: Vec<String> = cands
            .iter()
            .filter(|f| f.tstamp_ms == top)
            .map(|f| f.name.clone())
            .collect();
        names.sort();
        // `random-item` consumes exactly one draw, even for singletons.
        let pick = self.rng_draw(names.len());
        let feat = cands
            .iter()
            .find(|f| f.name == names[pick])
            .cloned()
            .unwrap();
        let base = feat.name.clone();
        self.buffer_mut("VISUAL-LOCATION").failed = false;
        self.place_vl(&base, true)?;
        Ok(())
    }

    /// Attention landed: build the TEXT object and fill the buffer.
    fn encoding_complete(&mut self, loc: String) -> Result<(), EngineError> {
        let idx = self.resolve_loc(&loc)?;
        let feat = self.features[idx].clone();
        let k = self.obj_built.entry(feat.kind.clone()).or_insert(0);
        let name = format!("{}{}", feat.kind, *k);
        *k += 1;
        let inst = ChunkInst {
            name: Some(name),
            ctype: "VISUAL-OBJECT".to_string(),
            slots: vec![("VALUE".to_string(), Value::Str(feat.value.clone()))],
            bare: false,
        };
        if let Ok(idx) = self.resolve_loc(&loc) {
            self.features[idx].attended = true;
        }
        self.visual_busy = false;
        self.now(
            Prio::Num(10),
            Out::Low,
            Event::SetBuffer {
                buffer: "VISUAL".to_string(),
                chunk: NewChunk::Instance(inst),
                requested: true,
                shown: None,
            },
        );
        Ok(())
    }
}

/// Screen-pos name a location chunk carries after attention bookkeeping.
/// Trace shows requested base names; buffers hold `-k` copies recorded
/// in `vl_copies`.
impl Feature {
    fn loc_slots(&self) -> Vec<(String, Value)> {
        vec![
            ("KIND".to_string(), Value::Sym(self.kind.clone())),
            ("VALUE".to_string(), Value::Str(self.value.clone())),
            ("SCREEN-X".to_string(), Value::Num(self.x as f64)),
            ("SCREEN-Y".to_string(), Value::Num(self.y as f64)),
        ]
    }
}

/// Visual-location request matching over features. Unconstrained
/// requests match everything; unknown slots error.
fn loc_matches(feat: &Feature, want: &[(String, Value)]) -> Result<bool, EngineError> {
    for (s, v) in want {
        match (s.as_str(), v) {
            ("ISA", _) => {}
            ("ATTENDED", Value::Sym(x)) if x == "T" => {
                if !feat.attended {
                    return Ok(false);
                }
            }
            ("ATTENDED", Value::Sym(x)) if x == "NIL" => {
                if feat.attended {
                    return Ok(false);
                }
            }
            ("ATTENDED", _) => {
                return Err(unimpl("non-t/nil :attended"));
            }
            ("KIND", Value::Sym(k)) => {
                if *k != feat.kind {
                    return Ok(false);
                }
            }
            ("VALUE", Value::Str(val)) => {
                if *val != feat.value {
                    return Ok(false);
                }
            }
            ("VALUE", Value::Sym(val)) => {
                if *val != feat.value {
                    return Ok(false);
                }
            }
            ("SCREEN-X", Value::Num(x)) => {
                if (*x as i64) != feat.x {
                    return Ok(false);
                }
            }
            ("SCREEN-Y", Value::Num(y)) => {
                if (*y as i64) != feat.y {
                    return Ok(false);
                }
            }
            got => {
                return Err(unimpl(format!("visual-location slot {got:?}")));
            }
        }
    }
    Ok(true)
}

/// CL `round`: fractional .5 goes to the even neighbor.
fn lisp_round(x: f32) -> i64 {
    let f = x.floor();
    let d = x - f;
    if d < 0.5 {
        f as i64
    } else if d > 0.5 {
        f as i64 + 1
    } else {
        let fi = f as i64;
        if fi % 2 == 0 {
            fi
        } else {
            fi + 1
        }
    }
}

/// Lisp `~f` for a single-float: integer-valued floats get a `.0`.
fn lisp_f(x: f32) -> String {
    if x.is_finite() && x.fract() == 0.0 {
        format!("{x:.1}")
    } else {
        format!("{x}")
    }
}

/// SBCL `~S`/`~A` for a single-float: shortest round-tripping decimal,
/// rendered fixed for decimal exponents in `-3..=6` and in `m.mmmmmmeN`
/// form otherwise (e.g. `3.7122225e-6`, `1.0e7`). The blending trace prints
/// its floats with this form (the refs are the arbiter).
fn lisp_s_f32(x: f32) -> String {
    if !x.is_finite() {
        return format!("{x}");
    }
    if x == 0.0 {
        return if x.is_sign_negative() {
            "-0.0".to_string()
        } else {
            "0.0".to_string()
        };
    }
    // `{:e}` gives the shortest digits (`d.dddde<exp>` or `de<exp>`).
    let s = format!("{x:e}");
    let (mant, exp) = s.split_once('e').expect("exp form has an e");
    let exp: i32 = exp.parse().expect("exp is an integer");
    let neg = mant.starts_with('-');
    let mant = mant.trim_start_matches('-');
    if (-3..=6).contains(&exp) {
        let digits: String = mant.chars().filter(|c| *c != '.').collect();
        let point = exp + 1; // digits before the decimal point
        let body = if point <= 0 {
            format!("0.{}{}", "0".repeat((-point) as usize), digits)
        } else if (point as usize) >= digits.len() {
            format!(
                "{digits}{}.0",
                "0".repeat(point as usize - digits.len())
            )
        } else {
            let (a, b) = digits.split_at(point as usize);
            format!("{a}.{b}")
        };
        format!("{}{body}", if neg { "-" } else { "" })
    } else {
        let m = if mant.contains('.') {
            mant.to_string()
        } else {
            format!("{mant}.0")
        };
        format!("{}{m}e{exp}", if neg { "-" } else { "" })
    }
}

/// Distribute `m` references evenly from `end` back toward `start`
/// (`even-references`, commands/dm-commands.lisp:639). Integer ms.
fn even_references(start: i64, end: i64, m: usize) -> Vec<i64> {
    if m == 0 {
        return Vec::new();
    }
    let n = m as f64;
    let decrement = (end - start) as f64 / n;
    let mut time = end as f64;
    let mut out = Vec::with_capacity(m);
    for _ in 0..m {
        time -= decrement;
        out.push(time.round() as i64);
    }
    out
}

/// Optimized-learning flag for [`bl_equation`]: off, numeric, or `t`.
#[derive(Debug, Clone, PartialEq)]
enum Ol {
    Off,
    Num(f64),
    On,
}

/// Generalized base-level decay (`compute-references`,
/// declarative-memory.lisp:2695), all single-float. Pure: takes the
/// clock, reference/creation times in ms, decay, OL flag and the
/// `:bll`-derived act-scale. Chunks with zero references never reach
/// here (the caller returns -999999.0 with a warning instead).
fn bl_equation(
    now_ms: i64,
    refs: &[i64],
    count: f32,
    creation_ms: i64,
    decay: f32,
    ol: &Ol,
    act_scale: f32,
) -> f32 {
    let mut value = 0.0f32;
    let mut last_ref = 0i64;
    for r in refs {
        let lag = (now_ms - r).max(50);
        value += (lag as f32).powf(-decay);
        last_ref = *r;
    }
    match ol {
        Ol::Off => value.ln() + act_scale,
        Ol::Num(ol) => {
            if count as f64 > *ol {
                // (+ 1.0 minus-decay)
                let denom = 1.0f32 - decay;
                let a = (now_ms - creation_ms).max(0) as f32;
                let b = (now_ms - last_ref).max(0) as f32;
                value += ((count - *ol as f32) * (a.powf(denom) - b.powf(denom)))
                    / ((last_ref - creation_ms).max(50) as f32 * denom);
            }
            value.ln() + act_scale
        }
        Ol::On => {
            // NOTE: unlike the other branches, this one adds NO
            // act-scale (declarative-memory.lisp:2725). The units table
            // caught an early draft that added it. Uses the reference
            // *count* `n` (not the list length — under `:ol t` merges
            // clear the list but keep counting).
            let t = ((now_ms - creation_ms).max(0) as f32 / 1000.0).max(0.05);
            let denom = 1.0f32 - decay;
            (count * t.powf(-decay) / denom).ln()
        }
    }
}

impl Engine {
    /// Whether `name` was retrieved within the current finst span.
    fn is_recent(&self, name: &str) -> bool {
        let now = self.time_ms;
        self.finsts
            .iter()
            .any(|(n, t)| n == name && now - t <= self.cfg.finst_span_ms)
    }

    /// Record a retrieval finst (newest last, capped at
    /// `:declarative-num-finsts`).
    fn add_finst(&mut self, name: &str) {
        self.finsts.retain(|(n, _)| n != name);
        self.finsts.push((name.to_string(), self.time_ms));
        let cap = self.cfg.num_finsts;
        if self.finsts.len() > cap {
            let drop = self.finsts.len() - cap;
            self.finsts.drain(0..drop);
        }
    }

    /// Resolve a chunk name through merge aliases (`merging-test`).
    fn resolve_alias(&self, n: &str) -> String {
        let mut cur = n.to_string();
        for _ in 0..8 {
            match self.chunk_alias.get(&cur) {
                Some(next) if *next != cur => cur = next.clone(),
                _ => break,
            }
        }
        cur
    }

    /// Value equality with chunk-name aliases applied to `Sym`s.
    fn value_eq(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Sym(x), Value::Sym(y)) => self.resolve_alias(x) == self.resolve_alias(y),
            _ => a == b,
        }
    }

    /// Numeric inequality test (`> < >= <=`) for LHS/request slots.
    fn rel_test(&self, op: SlotOp, pat: &Pattern, got: &Value) -> bool {
        let rhs = match pat {
            Pattern::Int(i) => *i as f64,
            Pattern::Num(x) => *x,
            _ => return false,
        };
        let lhs = match got {
            Value::Int(i) => *i as f64,
            Value::Num(x) => *x,
            _ => return false,
        };
        match op {
            SlotOp::Gt => lhs > rhs,
            SlotOp::Lt => lhs < rhs,
            SlotOp::Ge => lhs >= rhs,
            SlotOp::Le => lhs <= rhs,
            _ => false,
        }
    }

    /// Match one slot test; returns whether it passes.
    fn test_value(
        &self,
        pat: &Pattern,
        got: &Value,
        b: &mut Bindings,
        negated: bool,
    ) -> Result<bool, EngineError> {
        let ok = match pat {
            Pattern::Nil => *got == Value::Nil,
            Pattern::Var(v) => match b.get(v) {
                Some(prev) => self.value_eq(prev, got),
                None => {
                    // A LHS `slot =var` requires the slot to be filled
                    // (`add-slot-spec-to-chunk-spec` sets the signature's
                    // filled bit); an empty slot does not bind.
                    if *got == Value::Nil {
                        false
                    } else {
                        b.insert(v.clone(), got.clone());
                        true
                    }
                }
            },
            Pattern::Sym(s) => self.value_eq(got, &Value::Sym(s.clone())),
            Pattern::Int(i) => *got == Value::Int(*i),
            Pattern::Num(x) => *got == Value::Num(*x),
            Pattern::Str(s) => *got == Value::Str(s.clone()),
        };
        Ok(if negated { !ok } else { ok })
    }
}

impl Engine {
    fn show_output_arg(&self, a: &crate::sexpr::Sexp, b: &Bindings) -> Result<String, EngineError> {
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
fn chunk_inst(c: &Chunk) -> Result<ChunkInst, EngineError> {
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

fn new_chunk(desc: &ChunkDesc) -> Result<NewChunk, EngineError> {
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
    fn resolve_slot_name(&self, s: &str, b: &Bindings) -> String {
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
    fn copy_chunk(&mut self, name: &str) -> String {
        let n = *self.copy_count.entry(name.to_string()).or_insert(0);
        self.copy_count.insert(name.to_string(), n + 1);
        let copy = format!("{name}-{n}");
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
    fn rhs_value(
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
        self.subst(p, b)
    }

    fn concretize(
        &mut self,
        spec: &ChunkDesc,
        b: &Bindings,
        copy: bool,
        copied: &mut HashMap<String, String>,
    ) -> Result<NewChunk, EngineError> {
        match spec {
            ChunkDesc::Named(n) => Ok(NewChunk::Named(n.clone())),
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
fn implicit_clears(prod: &Production, exempt: &[String]) -> Vec<String> {
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
        }
    }
    let mut targeted: Vec<(char, String)> = Vec::new();
    for a in &prod.rhs {
        match a {
            RhsAction::Modify { buffer, .. } => targeted.push(('=', buffer.clone())),
            RhsAction::Request { buffer, .. } => targeted.push(('+', buffer.clone())),
            RhsAction::Clear { buffer } => targeted.push(('-', buffer.clone())),
            RhsAction::Output { .. } | RhsAction::Other { .. } => {}
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
        if *op == '+' && !targeted.iter().any(|(o, b)| *o == '-' && b == t) && !clears.contains(t) {
            clears.push(t.clone());
        }
    }
    clears
}

/// Buffers gaining an implicit `?buffer> state free` test: RHS `+`
/// (or `*`) requests whose target is not exempt and has no explicit
/// `?` test on the LHS (`production-parsing.lisp:1287`). Appended to
/// the LHS, matched, and emitted as query-buffer selection events.
fn implicit_queries(prod: &Production, exempt: &[String]) -> Vec<String> {
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
fn explicit_out(rhst: bool) -> Out {
    if rhst {
        Out::Always
    } else {
        Out::Never
    }
}

impl ChunkInst {
    fn show(&self) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn m0(name: &str) -> Model {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../actr7.x/tutorial/unit1")
            .join(name);
        let ms = crate::model::load_models(&p).unwrap();
        assert_eq!(ms.len(), 1);
        ms.into_iter().next().unwrap()
    }

    fn inst(ctype: &str, slots: Vec<(&str, Value)>) -> ChunkInst {
        ChunkInst {
            name: None,
            ctype: ctype.into(),
            slots: slots.into_iter().map(|(s, v)| (s.into(), v)).collect(),
            bare: false,
        }
    }

    fn sym(s: &str) -> Value {
        Value::Sym(s.into())
    }

    #[test]
    fn scripted_reset_is_idempotent() {
        // Construction performs a reset; an explicit `(reset)` at t=0 must
        // reproduce the same run (same DM, goal, CR) exactly.
        let a = Engine::new(m0("count.lisp"), (100, 0), 10.0, DeviceSetup::default()).unwrap();
        let out_a = a.run().unwrap();
        let mut b = Engine::new(m0("count.lisp"), (100, 0), 10.0, DeviceSetup::default()).unwrap();
        b.reset().unwrap();
        b.run_secs(10.0).unwrap();
        assert_eq!(out_a, b.output());
    }

    #[test]
    fn queue_orders_time_then_priority_then_fifo() {
        let m = m0("count.lisp");
        let mut e = Engine::new(m, (100, 0), 10.0, DeviceSetup::default()).unwrap();
        e.queue.clear();
        e.seq = 0;
        let rd = |b: &str| Event::BufferRead { buffer: b.into() };
        e.push(100, Prio::Num(0), Out::Low, rd("A"));
        e.push(50, Prio::Min, Out::Low, rd("B"));
        e.push(50, Prio::Num(10), Out::Low, rd("C"));
        e.push(50, Prio::Num(10), Out::Low, rd("D"));
        e.push(50, Prio::Max, Out::Low, rd("E"));
        e.push(50, Prio::Num(0), Out::Low, rd("F"));
        let mut order = Vec::new();
        while let Some(q) = e.queue.pop() {
            match q.event {
                Event::BufferRead { buffer } => order.push(buffer),
                _ => unreachable!(),
            }
        }
        assert_eq!(order, vec!["E", "C", "D", "F", "B", "A"]);
    }

    #[test]
    fn harvest_rules() {
        let exempt: Vec<String> = vec!["GOAL".to_string()];
        let m = m0("count.lisp");
        let clears =
            |n: &str| implicit_clears(m.productions.iter().find(|p| p.name == n).unwrap(), &exempt);
        // START tests only =goal> but requests retrieval: request rule.
        assert_eq!(clears("START"), vec!["RETRIEVAL".to_string()]);
        // INCREMENT tests both, requests retrieval: request rule (no double).
        assert_eq!(clears("INCREMENT"), vec!["RETRIEVAL".to_string()]);
        // STOP has explicit -goal>: only the untested-by-rhs retrieval.
        assert_eq!(clears("STOP"), vec!["RETRIEVAL".to_string()]);
        let s = m0("semantic.lisp");
        let dv = s
            .productions
            .iter()
            .find(|p| p.name == "DIRECT-VERIFY")
            .unwrap();
        assert_eq!(implicit_clears(dv, &exempt), vec!["RETRIEVAL".to_string()]);
        // Goal is harvest-exempt even when tested-but-untargeted.
        use crate::model::Pattern as Pat;
        use crate::model::{LhsItem, Production, RhsAction, SlotTest};
        let probe = Production {
            name: "PROBE".to_string(),
            lhs: vec![LhsItem::Match {
                buffer: "GOAL".to_string(),
                tests: vec![SlotTest {
                    slot: "ISA".to_string(),
                    value: Pat::Sym("GOAL".to_string()),
                    op: SlotOp::Eq,
                }],
            }],
            rhs: vec![RhsAction::Request {
                buffer: "VISUAL".to_string(),
                spec: crate::model::ChunkDesc::Spec {
                    ctype: String::new(),
                    slots: vec![],
                    neg: vec![],
                    rels: vec![],
                },
            }],
        };
        let no_exempt: Vec<String> = vec![];
        assert_eq!(
            implicit_clears(&probe, &no_exempt),
            vec!["GOAL".to_string(), "VISUAL".to_string()]
        );
        assert_eq!(implicit_clears(&probe, &exempt), vec!["VISUAL".to_string()]);
    }

    #[test]
    fn matcher_vars_nil_negation() {
        let m = m0("count.lisp");
        let mut e = Engine::new(m, (100, 0), 10.0, DeviceSetup::default()).unwrap();
        // goal-focus state: COUNT slot absent (= nil for tests).
        e.buffers.get_mut("GOAL").unwrap().chunk = Some(inst(
            "COUNT-FROM",
            vec![("START", sym("TWO")), ("END", sym("FOUR"))],
        ));
        let start = e.model.productions[0].clone();
        let b = e
            .match_prod(&start, &[], &mut Vec::new())
            .unwrap()
            .expect("START should match");
        assert_eq!(b.get("NUM1"), Some(&sym("TWO")));
        // INCREMENT needs =retrieval>: absent, no match.
        let inc = e.model.productions[1].clone();
        assert!(e.match_prod(&inc, &[], &mut Vec::new()).unwrap().is_none());
        // Retrieval TWO (number TWO next THREE): full match, NUM2=THREE.
        e.buffers.get_mut("RETRIEVAL").unwrap().chunk = Some(inst(
            "NUMBER",
            vec![("NUMBER", sym("TWO")), ("NEXT", sym("THREE"))],
        ));
        // Goal still count-less: `count =num1` fails on nil... START's
        // `count nil` passes but increment needs a real count.
        assert!(e.match_prod(&inc, &[], &mut Vec::new()).unwrap().is_none());
        // Advance the goal as START's firing would: count=TWO.
        e.buffers.get_mut("GOAL").unwrap().chunk = Some(inst(
            "COUNT-FROM",
            vec![
                ("START", sym("TWO")),
                ("END", sym("FOUR")),
                ("COUNT", sym("TWO")),
            ],
        ));
        let b = e.match_prod(&inc, &[], &mut Vec::new()).unwrap().expect("INCREMENT matches");
        assert_eq!(b.get("NUM1"), Some(&sym("TWO")));
        assert_eq!(b.get("NUM2"), Some(&sym("THREE")));
        // STOP needs count=end: not yet.
        let stop = e.model.productions[2].clone();
        assert!(e.match_prod(&stop, &[], &mut Vec::new()).unwrap().is_none());
    }

    #[test]
    fn matcher_failure_query() {
        let m = m0("semantic.lisp");
        let mut e = Engine::new(m, (100, 0), 10.0, DeviceSetup::default()).unwrap();
        e.buffers.get_mut("GOAL").unwrap().chunk = Some(inst(
            "IS-MEMBER",
            vec![
                ("OBJECT", sym("CANARY")),
                ("CATEGORY", sym("BIRD")),
                ("JUDGMENT", sym("PENDING")),
            ],
        ));
        let fail = e.model.productions[3].clone();
        // Retrieval merely empty is not failure.
        assert!(e.match_prod(&fail, &[], &mut Vec::new()).unwrap().is_none());
        e.buffers.get_mut("RETRIEVAL").unwrap().failed = true;
        assert!(e.match_prod(&fail, &[], &mut Vec::new()).unwrap().is_some());
    }

    /// SBCL `~S` single-float rendering (the blending trace's arbiter):
    /// shortest digits, fixed for exponents -3..=6, exponent form outside.
    #[test]
    fn lisp_s_f32_matches_sbcl() {
        assert_eq!(lisp_s_f32(0.35355338), "0.35355338");
        assert_eq!(lisp_s_f32(2.5325232), "2.5325232");
        assert_eq!(lisp_s_f32(3.7122225e-6), "3.7122225e-6");
        assert_eq!(lisp_s_f32(1.1136667e-5), "1.1136667e-5");
        assert_eq!(lisp_s_f32(0.001), "0.001");
        assert_eq!(lisp_s_f32(1.0e7), "1.0e7");
        assert_eq!(lisp_s_f32(9999999.0), "9999999.0");
        assert_eq!(lisp_s_f32(-0.9), "-0.9");
        assert_eq!(lisp_s_f32(0.0), "0.0");
        assert_eq!(lisp_s_f32(2.0), "2.0");
    }

    /// CL `remove-duplicates` keeps the later copy (production
    /// compilation's condition union depends on it).
    #[test]
    fn remove_dup_keeps_last() {
        assert_eq!(remove_dup_keep_last(vec![1, 2, 1, 3]), vec![2, 1, 3]);
        let a = ("SLOT".to_string(), SlotVal::Lit("SMALL".to_string()));
        let b = ("STATE".to_string(), SlotVal::Lit("P2".to_string()));
        assert_eq!(
            remove_dup_keep_last(vec![a.clone(), b.clone(), a]),
            vec![b, ("SLOT".to_string(), SlotVal::Lit("SMALL".to_string()))]
        );
    }

    /// `render_production` reproduces the `:cst` instantiation block
    /// (`tutorial-unit8-simple-ppm-model-initial-run.ref`).
    #[test]
    fn render_production_instantiated() {
        use crate::model::{LhsItem, Pattern as Pat, Production, RhsAction, SlotTest};
        let prod = Production {
            name: "SMALL".to_string(),
            lhs: vec![LhsItem::Match {
                buffer: "GOAL".to_string(),
                tests: vec![
                    SlotTest {
                        slot: "ISA".to_string(),
                        value: Pat::Sym("TEST".to_string()),
                        op: SlotOp::Eq,
                    },
                    SlotTest {
                        slot: "VALUE".to_string(),
                        value: Pat::Sym("SMALL".to_string()),
                        op: SlotOp::Eq,
                    },
                    SlotTest {
                        slot: "VALUE".to_string(),
                        value: Pat::Var("VAL".to_string()),
                        op: SlotOp::Eq,
                    },
                ],
            }],
            rhs: vec![
                RhsAction::Output {
                    args: vec![Sexp::Sym("=VAL".to_string())],
                },
                RhsAction::Clear {
                    buffer: "GOAL".to_string(),
                },
            ],
        };
        let mut b = Bindings::new();
        b.insert("VAL".to_string(), Value::Sym("SMALL".to_string()));
        assert_eq!(
            render_production(&prod, &b, &[]),
            "(P SMALL\n   =GOAL>\n       VALUE SMALL\n       VALUE SMALL\n ==>\n   !OUTPUT! SMALL\n   -GOAL>\n)"
        );
    }

    /// Row-by-row against `golden/units/tables/latency.sexp`: 802
    /// `compute-activation-latency` points plus CL half-even ROUND probes.
    #[test]
    fn latency_matches_lisp_table() {
        use crate::sexpr::{parse_all, Sexp};
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden/units/tables/latency.sexp");
        let src = std::fs::read_to_string(&path).unwrap();
        let forms = parse_all(&src).unwrap();
        let (mut lat, mut rnd) = (0, 0);
        for f in &forms {
            let items = match f {
                Sexp::List(it) => it,
                _ => panic!("table row not a list: {f:?}"),
            };
            match items.as_slice() {
                [Sexp::Sym(h), Sexp::Float(a), Sexp::Int(lf), Sexp::Float(le), Sexp::Int(want)]
                    if h == "LAT" =>
                {
                    assert_eq!(latency(*lf, *le as f32, *a as f32), *want, "row {f:?}");
                    lat += 1;
                }
                [Sexp::Sym(h), Sexp::Float(x), Sexp::Int(want)] if h == "ROUND" => {
                    assert_eq!(lisp_round(*x as f32), *want, "row {f:?}");
                    rnd += 1;
                }
                row => panic!("bad table row: {row:?}"),
            }
        }
        assert!(lat >= 800 && rnd >= 7, "table shrank: lat={lat} rnd={rnd}");
    }

    /// Row-by-row against `golden/units/tables/base-level.sexp`:
    /// act-scale derivation plus the decay equation over three OL
    /// modes. Compared with epsilon (f32 `powf`/`ln` may differ 1ulp
    /// across libms); the table pins the equation, not the last bit.
    #[test]
    fn base_level_matches_lisp_table() {
        use crate::sexpr::{parse_all, Sexp};
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden/units/tables/base-level.sexp");
        let src = std::fs::read_to_string(&path).unwrap();
        let forms = parse_all(&src).unwrap();
        let (mut acts, mut rows, mut now_ms) = (0, 0, 0i64);
        for f in &forms {
            let items = match f {
                Sexp::List(it) => it,
                _ => panic!("table row not a list: {f:?}"),
            };
            match items.as_slice() {
                [Sexp::Sym(h), Sexp::Int(t)] if h == "NOW" => now_ms = *t,
                [Sexp::Sym(h), Sexp::Float(bll), Sexp::Float(want)] if h == "ACTSCALE" => {
                    // (log (expt 1/1000 (- bll))): ratio coerced to single.
                    let got = (0.001f32).powf(-(*bll as f32)).ln();
                    assert!(
                        (got - *want as f32).abs() <= 1e-6,
                        "ACTSCALE {bll}: got {got}, want {want}"
                    );
                    acts += 1;
                }
                [Sexp::Sym(h), Sexp::Float(bll), ol, Sexp::Float(scale), Sexp::Int(n), Sexp::Int(ct), rest @ ..]
                    if h == "BL" =>
                {
                    let ol = match ol {
                        Sexp::Sym(s) if s == "NIL" => Ol::Off,
                        Sexp::Int(i) => Ol::Num(*i as f64),
                        Sexp::Sym(s) if s == "T" => Ol::On,
                        got => panic!("bad OL flag: {got:?}"),
                    };
                    assert!(rest.len() >= 1, "BL row missing RESULT: {f:?}");
                    let (refs_s, res_s) = rest.split_at(rest.len() - 1);
                    let refs: Vec<i64> = refs_s
                        .iter()
                        .map(|r| match r {
                            Sexp::Int(t) => *t,
                            got => panic!("bad ref time: {got:?}"),
                        })
                        .collect();
                    let want = match &res_s[0] {
                        Sexp::Float(x) => *x as f32,
                        got => panic!("bad RESULT: {got:?}"),
                    };
                    assert_eq!(refs.len(), *n as usize, "row {f:?}");
                    let got =
                        bl_equation(now_ms, &refs, *n as f32, *ct, *bll as f32, &ol, *scale as f32);
                    let tol = 1e-5 * want.abs().max(1.0);
                    assert!(
                        (got - want).abs() <= tol,
                        "BL row {f:?}: got {got}, want {want}"
                    );
                    rows += 1;
                }
                row => panic!("bad table row: {row:?}"),
            }
        }
        assert!(
            acts >= 2 && rows >= 24,
            "table shrank: acts={acts} rows={rows}"
        );
    }
}

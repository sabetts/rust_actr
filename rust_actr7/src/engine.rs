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

use crate::model::{Chunk, ChunkDesc, LhsItem, Model, Pattern, Production, RhsAction};
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
            Value::Num(x) => {
                if x.fract() == 0.0 && x.abs() < 1e15 {
                    format!("{x:.1}")
                } else {
                    format!("{x}")
                }
            }
            Value::Str(s) => format!("{s:?}"),
            Value::Nil => "NIL".to_string(),
        }
    }
}

type Bindings = HashMap<String, Value>;

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
    /// SET-copy name -> feature index, resolving copies back for
    /// attention and encoding.
    vl_copies: HashMap<String, usize>,
    /// Visual-object names per kind: KIND -> objects so far.
    obj_built: HashMap<String, u64>,
    visual_busy: bool,
    man_prep_busy: bool,
    man_exec_busy: bool,
    imag_busy: bool,
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
            vl_copies: HashMap::new(),
            obj_built: HashMap::new(),
            visual_busy: false,
            man_prep_busy: false,
            man_exec_busy: false,
            imag_busy: false,
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
        self.vl_copies.clear();
        self.obj_built.clear();
        self.visual_busy = false;
        self.man_prep_busy = false;
        self.man_exec_busy = false;
        self.imag_busy = false;
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
            }
            None => {
                // Unnamed buffer contents (imaginal/goal request specs)
                // land as reuse-chunk copies: {BUFFER}-CHUNK0-{k}, which
                // is the name later retrievals print.
                if chunk.name.is_none() {
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
        let mut matched: Vec<String> = Vec::new();
        for ep in self.exec_prods.clone() {
            let pname = self.model.productions[ep.index].name.clone();
            if crt {
                self.out(format!("Trying production: {pname}"));
            }
            let prod = self.model.productions[ep.index].clone();
            if let Some(b) = self.match_prod(&prod, &ep.implicit_queries)? {
                matched.push(pname);
                let mut u = self.utilities[ep.index];
                if self.cfg.egs != 0.0 {
                    u += self.rng.noise(self.cfg.egs);
                }
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
            for name in &matched {
                self.out(format!("Production {name} matches"));
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
    ) -> Result<Option<Bindings>, EngineError> {
        let mut b = Bindings::new();
        for item in &prod.lhs {
            if self.match_item(item, &mut b)?.is_none() {
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

    fn match_item(&self, item: &LhsItem, b: &mut Bindings) -> Result<Option<()>, EngineError> {
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
                    if t.slot == "ISA" {
                        let ok = match &t.value {
                            Pattern::Var(v) => match b.get(v) {
                                Some(Value::Sym(s)) => s == &chunk.ctype,
                                Some(_) => false,
                                None => {
                                    b.insert(v.clone(), Value::Sym(chunk.ctype.clone()));
                                    true
                                }
                            },
                            Pattern::Sym(s) => s == &chunk.ctype,
                            _ => false,
                        };
                        if t.negated == ok {
                            return Ok(None);
                        }
                        continue;
                    }
                    if !test_value(&t.value, &chunk.get(&t.slot), &mut *b, t.negated)? {
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

    /// `?buffer> state free`: module busy flags for perceptual-motor
    /// buffers, buffer flag otherwise. (Motor free = neither preparing
    /// nor executing; visual free = no attention shift in flight.)
    fn query_free(&self, buffer: &str) -> Result<bool, EngineError> {
        match buffer {
            "VISUAL" => Ok(!self.visual_busy),
            "MANUAL" => Ok(!self.man_prep_busy && !self.man_exec_busy),
            "IMAGINAL" => Ok(!self.imag_busy),
            _ => Ok(!self
                .buffers
                .get(buffer)
                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?
                .busy),
        }
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
                return Err(unimpl(format!("RHS {head} needs a Lisp evaluator")));
            }
        }
        // Explicit actions in RHS order, then implicit harvest clears.
        for action in &prod.rhs {
            match action {
                RhsAction::Modify { buffer, slots } => {
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
        Ok(())
    }

    /// Chunk similarity for partial matching (`chunks-similarity`):
    /// explicit table value (either direction), else `ms` for
    /// identical/equal values, else `md`. Chunkhood is DM membership
    /// (an approximation documented in GOTCHAS).
    fn similarity(&self, a: &Value, b: &Value) -> f32 {
        let a_name = match a {
            Value::Sym(s) => Some(s.as_str()),
            _ => None,
        };
        let b_name = match b {
            Value::Sym(s) => Some(s.as_str()),
            _ => None,
        };
        match (a_name, b_name) {
            (Some(x), Some(y))
                if self.dm.iter().any(|c| c.name.as_deref() == Some(x))
                    && self.dm.iter().any(|c| c.name.as_deref() == Some(y)) =>
            {
                if let Some(v) = self
                    .sims
                    .get(&(x.to_string(), y.to_string()))
                    .or_else(|| self.sims.get(&(y.to_string(), x.to_string())))
                {
                    return *v;
                }
                if x == y {
                    return self.cfg.ms;
                }
                self.cfg.md
            }
            _ => {
                if a == b {
                    self.cfg.ms
                } else {
                    self.cfg.md
                }
            }
        }
    }

    fn start_retrieval(&mut self, spec: NewChunk, var_slots: &[String]) -> Result<(), EngineError> {
        let (want_type, want_slots, want_neg) = match &spec {
            NewChunk::Named(n) => {
                return Err(unimpl(format!("single-item retrieval request {n}")));
            }
            NewChunk::Instance(_) => {
                return Err(bad(
                    "retrieval request cannot be a buffer instance".to_string(),
                ));
            }
            NewChunk::Spec {
                ctype, slots, neg, ..
            } => (ctype.clone(), slots.clone(), neg.clone()),
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
            if !partial {
                let matched = if !want_type.is_empty() && c.ctype != want_type {
                    false
                } else {
                    let mut ok = true;
                    for (s, v) in &want_slots {
                        if s.starts_with(':') {
                            continue;
                        }
                        if c.get(s) != *v {
                            ok = false;
                            break;
                        }
                    }
                    if ok {
                        for (s, v) in &want_neg {
                            if s.starts_with(':') {
                                continue;
                            }
                            if c.get(s) == *v {
                                ok = false;
                                break;
                            }
                        }
                    }
                    ok
                };
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
                                creation,
                                bll,
                                &self.cfg.ol,
                                self.cfg.act_scale,
                            )
                    }
                }
                None => {
                    if self.act_medium() {
                        self.out(format!("Starting with blc: {}", lisp_f(self.cfg.blc)));
                    }
                    self.cfg.blc
                }
            };
            if self.act_medium() {
                self.out(format!("Total base-level: {}", lisp_f(base)));
            }
            let mut a = base;
            // Spreading activation (0 unless `:mas` is set): sources are
            // the chunk-valued slots of the spread buffers.
            if c.name.is_some() {
                a += self.spreading_activation(&cname);
            }
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
            if refs.len() as f64 > *ol {
                // (+ 1.0 minus-decay)
                let denom = 1.0f32 - decay;
                let a = (now_ms - creation_ms).max(0) as f32;
                let b = (now_ms - last_ref).max(0) as f32;
                value += ((refs.len() as f32 - *ol as f32) * (a.powf(denom) - b.powf(denom)))
                    / ((last_ref - creation_ms).max(50) as f32 * denom);
            }
            value.ln() + act_scale
        }
        Ol::On => {
            // NOTE: unlike the other branches, this one adds NO
            // act-scale (declarative-memory.lisp:2725). The units table
            // caught an early draft that added it.
            // ms->seconds is (/ x 1000.0), single-float.
            let t = ((now_ms - creation_ms).max(0) as f32 / 1000.0).max(0.05);
            let denom = 1.0f32 - decay;
            (refs.len() as f32 * t.powf(-decay) / denom).ln()
        }
    }
}

/// Match one slot test; returns whether it passes.
fn test_value(
    pat: &Pattern,
    got: &Value,
    b: &mut Bindings,
    negated: bool,
) -> Result<bool, EngineError> {
    let ok = match pat {
        Pattern::Nil => *got == Value::Nil,
        Pattern::Var(v) => match b.get(v) {
            Some(prev) => prev == got,
            None => {
                b.insert(v.clone(), got.clone());
                true
            }
        },
        Pattern::Sym(s) => *got == Value::Sym(s.clone()),
        Pattern::Int(i) => *got == Value::Int(*i),
        Pattern::Num(x) => *got == Value::Num(*x),
        Pattern::Str(s) => *got == Value::Str(s.clone()),
    };
    Ok(if negated { !ok } else { ok })
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
        ChunkDesc::Spec { ctype, slots, neg } => {
            let mut out = Vec::new();
            for (s, p) in slots {
                out.push((s.clone(), Value::from_pattern(p)?));
            }
            let mut negv = Vec::new();
            for (s, p) in neg {
                negv.push((s.clone(), Value::from_pattern(p)?));
            }
            Ok(NewChunk::Spec {
                ctype: ctype.clone(),
                slots: out,
                neg: negv,
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
                    if let Some(chunk) = self.buffers.get(v).and_then(|buf| buf.chunk.as_ref()) {
                        if let Some(name) = chunk.name.clone() {
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
            ChunkDesc::Spec { ctype, slots, neg } => {
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
                Ok(NewChunk::Spec {
                    ctype: ctype.clone(),
                    slots: out,
                    neg: negv,
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
                    negated: false,
                }],
            }],
            rhs: vec![RhsAction::Request {
                buffer: "VISUAL".to_string(),
                spec: crate::model::ChunkDesc::Spec {
                    ctype: String::new(),
                    slots: vec![],
                    neg: vec![],
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
            .match_prod(&start, &[])
            .unwrap()
            .expect("START should match");
        assert_eq!(b.get("NUM1"), Some(&sym("TWO")));
        // INCREMENT needs =retrieval>: absent, no match.
        let inc = e.model.productions[1].clone();
        assert!(e.match_prod(&inc, &[]).unwrap().is_none());
        // Retrieval TWO (number TWO next THREE): full match, NUM2=THREE.
        e.buffers.get_mut("RETRIEVAL").unwrap().chunk = Some(inst(
            "NUMBER",
            vec![("NUMBER", sym("TWO")), ("NEXT", sym("THREE"))],
        ));
        // Goal still count-less: `count =num1` fails on nil... START's
        // `count nil` passes but increment needs a real count.
        assert!(e.match_prod(&inc, &[]).unwrap().is_none());
        // Advance the goal as START's firing would: count=TWO.
        e.buffers.get_mut("GOAL").unwrap().chunk = Some(inst(
            "COUNT-FROM",
            vec![
                ("START", sym("TWO")),
                ("END", sym("FOUR")),
                ("COUNT", sym("TWO")),
            ],
        ));
        let b = e.match_prod(&inc, &[]).unwrap().expect("INCREMENT matches");
        assert_eq!(b.get("NUM1"), Some(&sym("TWO")));
        assert_eq!(b.get("NUM2"), Some(&sym("THREE")));
        // STOP needs count=end: not yet.
        let stop = e.model.productions[2].clone();
        assert!(e.match_prod(&stop, &[]).unwrap().is_none());
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
        assert!(e.match_prod(&fail, &[]).unwrap().is_none());
        e.buffers.get_mut("RETRIEVAL").unwrap().failed = true;
        assert!(e.match_prod(&fail, &[]).unwrap().is_some());
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
                    let got = bl_equation(now_ms, &refs, *ct, *bll as f32, &ol, *scale as f32);
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

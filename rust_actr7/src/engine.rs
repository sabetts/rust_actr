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
//!
//! Layout: this file holds the shared types and the core kernel (scheduler,
//! buffers, activation, run loop). The per-subsystem `impl Engine` blocks
//! live in submodules — `events`, `commands`, `eval`, `productions`,
//! `compilation`, `dm`, `perception`, `motor`, `driver`, `debug`, `rhs` —
//! with `tests` for the unit tests.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::model::{
    Chunk, ChunkDesc, LhsItem, Model, Pattern, Production, Reward, RhsAction, SlotOp,
};
use crate::rand::Mt;
use crate::sexpr::Sexp;
use crate::trace::{format_line, Detail, Out, MODULE_WIDTH};

mod commands;
mod compilation;
mod debug;
mod dm;
mod driver;
mod eval;
mod events;
mod motor;
mod perception;
mod productions;
mod rhs;
#[cfg(test)]
mod tests;

use motor::even_references;
use rhs::{chunk_inst, implicit_clears, implicit_queries, new_chunk};

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

/// Evaluated Lisp value usable by the bounded task-function evaluator.
#[derive(Debug, Clone)]
enum Value {
    Sym(String),
    Int(i64),
    Num(f64),
    Str(String),
    Nil,
    /// A list of values (`(list 'answer choice)` → `(ANSWER A)`).
    List(Vec<Value>),
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
            (List(a), List(b)) => a == b,
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

    /// A literal `Sexp` as a stored value (for `mod-focus` etc.).
    fn from_sexp(s: &Sexp) -> Result<Value, EngineError> {
        match s {
            Sexp::Sym(x) if x == "NIL" => Ok(Value::Nil),
            Sexp::Sym(x) => Ok(Value::Sym(x.clone())),
            Sexp::Int(i) => Ok(Value::Int(*i)),
            Sexp::Float(x) => Ok(Value::Num(*x)),
            Sexp::Str(x) => Ok(Value::Str(x.clone())),
            Sexp::List(v) => {
                let mut out = Vec::new();
                for e in v {
                    out.push(Value::from_sexp(e)?);
                }
                Ok(Value::List(out))
            }
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
            Value::List(v) => format!(
                "({})",
                v.iter().map(Value::show).collect::<Vec<_>>().join(" ")
            ),
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
    /// `:ult` — print the utility-learning update trace.
    ult: bool,
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
    /// Temporal module `:time-noise` (default 0.015).
    time_noise: f32,
    /// Temporal module `:time-mult` (default 1.1).
    time_mult: f32,
    /// Temporal module `:time-master-start-increment` in ms (default 11).
    time_start_ms: f32,
    /// Temporal module `:record-ticks` (default t).
    record_ticks: bool,
    /// `:randomize-time` (random module): `Some(scale)` jitters module
    /// latencies (`t/scale` uniform by default scale 3).
    randomize_scale: Option<f32>,
    /// `:vpft` — randomize production action times.
    vpft: bool,
    /// `:vidt` — randomize the imaginal module delay.
    vidt: bool,
    /// `update-chunks-on-the-fly`: `:ncnar` non-nil AND `:dcnn` (both `t` by
    /// default). On a merge, rewrite references to the merged buffer chunk to
    /// the DM chunk's name (`framework/naming-module.lisp:452`).
    chunk_update: bool,    /// `require-extra "threads"`: the goal buffer is a searchable
    /// multi-buffer holding a set of concurrent goals (`new-threads.lisp`).
    threads: bool,
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
    /// Visual finsts (`:visual-num-finsts` / `:visual-finst-span`,
    /// `vision.lisp`): how many attended features are remembered, and how long.
    visual_num_finsts: usize,
    visual_finst_span_ms: i64,
    dat_ms: i64,
    attention_ms: i64,
    /// `:unstuff-visual-location` (`vision.lisp:3499`): `None` disables the
    /// automatic unstuff; `Some(delay)` is the unstuff delay in ms.
    unstuff_vl_ms: Option<i64>,
    imaginal_delay_ms: i64,
    feat_prep_ms: i64,
    init_ms: i64,
    burst_ms: i64,
    /// EMMA vision extension, enabled via
    /// `(ssp :starting-parameters (:emma t))` (`extras/emma/emma.lisp:885`).
    emma: bool,
    /// `:eye-saccade-rate` — saccade rate per degree of visual angle (0.002).
    emma_sacc_rate: f32,
    /// `:saccade-base-time` — base saccade execution time (0.020).
    emma_base_exe: f32,
    /// `:saccade-init-time` — non-labile saccade initiation (0.050).
    emma_init_time: f32,
    /// `:saccade-feat-time` — prep cost per saccade feature (0.050).
    emma_feat_prep: f32,
    /// `:visual-encoding-factor` — recognition-time factor K (0.006).
    emma_enc_factor: f32,
    /// `:visual-encoding-exponent` — recognition-time exponent k (0.4).
    emma_enc_exp: f32,
    /// `:vis-obj-freq` — default visual-object frequency (0.1).
    emma_obj_freq: f32,
    /// `key-closure-time` (`motor.lisp:477`, default 0.01): the punch style's
    /// exec time is `init + closure` (`motor.lisp:938`).
    key_closure_ms: i64,
    /// act-touch module (`extras/act-touch/act-touch.lisp`): enabled by
    /// `(require-extra "act-touch")`.
    touch: bool,
    touch_ppi: f32,
    touch_height: f32,
    touch_finger_spacing: f32,
    touch_thumb_width: f32,
    touch_peck_coeff: f32,
    touch_cursor_coeff: f32,
    touch_swipe_speed: i64,
    touch_rotate_width: f32,
    touch_drag_delay: bool,
    touch_left_start: (i64, i64),
    touch_right_start: (i64, i64),
    /// tracker extra (`extras/tracker/tracker.lisp`).
    tracker: bool,
    trace_tracker: bool,
    /// extended-motor-actions extra (`extras/extended-motor-actions`): a
    /// dual-execution motor module plus new styles/devices.
    zz_ext: bool,
    /// Driver state for the `zz` task (`*dp*`/`*de*`/`*query*`/…).
    zz_two_proc: bool,
    zz_two_exec: bool,
    zz_query: String,
    zz_kct_ms: i64,
    zz_krt_ms: i64,
    /// `:cursor-drag-delay` (`nil`/`t`→0.8/`0.5`/`1.0`).
    zz_cdd: Option<f64>,
    /// `:peck-strike-distance` (`*psd*`).
    zz_psd: Option<f32>,
    /// associative-learning extra: `:eal` (decay; `nil` disables the module),
    /// `:al-prior-p`, `:al-prior-w`, `:al-show`.
    al_eal: Option<f32>,
    al_prior_p: f32,
    al_prior_w: f32,
    al_show: bool,
    /// `:default-punch-delay`/`:fast-punch-delay`/`:slow-punch-delay`
    /// (`*dpd*`/`*fpd*`/`*spd*`, `motor-extension.lisp:1482`).
    zz_dpd_ms: i64,
    zz_fpd_ms: i64,
    zz_spd_ms: i64,
    peck_fitts: f32,
    min_fitts_ms: i64,
    rhst: bool,
    er: bool,
    do_not_query: Vec<String>,
    do_not_harvest: Vec<String>,
    /// Module column width in trace lines (`max-module-name-length`). The
    /// normal value is 22; test-model stubs may override it (GOTCHAS M10).
    module_width: usize,
    /// Model-body `(sgp :seed (S C))` (`random.lisp:417`). ACT-R re-applies
    /// the model's seed on every reset (`model.lisp:972`), so it wins over
    /// the harness's per-run seed — this is the seed the `.ref` used.
    seed: Option<(u64, u64)>,
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
    /// The reset this op belongs to (`(RUN n …)` grouping): the task
    /// re-applies its window content per run. `None` = every reset.
    pub run: Option<u32>,
}

/// One scripted window-item move (`modify-text-for-exp-window`): the task's
/// `move-it` is scheduled by the model's `!eval!`, so the engine replays the
/// resulting position change from the captured op.
#[derive(Debug, Clone)]
pub struct DeviceModify {
    pub at_ms: i64,
    pub text: String,
    pub x: i64,
    pub y: i64,
    /// See [`DeviceText::run`].
    pub run: Option<u32>,
}

/// One scripted command trace line: a task's scheduled command event.
#[derive(Debug, Clone)]
pub struct DeviceCommand {
    pub at_ms: i64,
    /// `low`/`medium`/`high`/`t` (`:output`).
    pub output: String,
    /// The `:module` the task scheduled the command under
    /// (`schedule-event … :module …`), which is the trace's module column.
    /// Defaults to `NONE`.
    pub module: String,
    pub details: String,
    /// See [`DeviceText::run`].
    pub run: Option<u32>,
}

/// One scripted tone (`new-tone-sound`): the audio event is created at reset
/// and its onset scheduled at `at_ms` (the tone's `onset`). The task's
/// `new-tone-sound` has no textual trace, so the sidecar records the
/// parameters directly rather than a `COMMAND` line.
#[derive(Debug, Clone)]
pub struct DeviceSound {
    /// Onset in ms (when `new-sound` fires).
    pub at_ms: i64,
    pub freq: i64,
    /// Sound duration in ms.
    pub duration: i64,
    /// See [`DeviceText::run`].
    pub run: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct DeviceSetup {
    pub texts: Vec<DeviceText>,
    /// Scripted window-item moves (`modify-text-for-exp-window`).
    pub modifies: Vec<DeviceModify>,
    /// Window clears at ms (clear-exp-window), with the run grouping.
    pub clears: Vec<(i64, Option<u32>)>,
    /// Scheduled command lines `(ms, output, details)` — a task's
    /// `schedule-event-relative`/`schedule-event` with a string action
    /// prints `NONE <action> <params>` at its `:output` level.
    pub commands: Vec<DeviceCommand>,
    /// Scripted tone sounds (`new-tone-sound`), re-armed on every reset.
    pub sounds: Vec<DeviceSound>,
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
    /// The task clears the window from its `output-key` handler
    /// (`clear-exp-window`, e.g. the demo2 task), so the clear happens
    /// at the moment of a keypress rather than at a scripted time.
    pub clear_on_keypress: bool,
    /// Model-RNG draws the task performs during setup but that this
    /// scripted driver does not replay (e.g. demo2's `permute-list`
    /// selecting the window text), consumed after the initial reset so
    /// `:randomize-time` draws line up.
    pub consume_rng: u64,
}

/// EMMA `saccade` movement style (`emma.lisp:448`): the geometry and times
/// of one eye movement.
#[derive(Debug, Clone)]
struct EmmaSacc {
    /// `r` — angle from the eye to the (noisy) target, in degrees.
    r: f32,
    /// `theta` — direction angle.
    theta: f32,
    /// `trgt` — the object chunk the movement is aimed at.
    trgt: Option<String>,
    /// `new-loc` — the (noisy, rounded) landing point.
    new_loc: (f32, f32, f32),
    /// `fprep-time` — preparation time in seconds.
    fprep_time: f64,
    /// `exec-time` — execution time in seconds.
    exec_time: f64,
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
    /// `width`/`height` slots (0 when absent) for `approach-width`.
    width: i64,
    height: i64,
    /// Additional slots (`prompt` etc.) preserved in the visual-location.
    extra: Vec<(String, Value)>,
    tstamp_ms: i64,
    /// `chunk-visual-new-p`: `NEW` until the feature is attended
    /// (`add-finst` clears it, `vision.lisp:2297`). The default-spec's
    /// `:attended new` (`vision.lisp:3178`) selects on this.
    new_p: bool,
    /// The feature's finst timestamp (`finst-lst` entry, `vision.lisp:2317`):
    /// `Some` while recently attended. `:attended t` iff `Some`; `:attended
    /// nil` iff `None`. Capped by `:visual-num-finsts` (oldest evicted), which
    /// is what re-exposes Sperling's auto-stuffed location to a later find.
    finst_ms: Option<i64>,
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

/// extended-motor-actions: a movement waiting for its hand's execution stage.
#[derive(Debug, Clone)]
enum ZzKind {
    /// Base `press-key`: `MotorNotify` → `OutputKey`.
    Press,
    /// Extended keyboard style: `output-key` at exec (unless `output`),
    /// `release-key` at `exec2`, plus finger `down`/`busy` bookkeeping.
    Key {
        finger: usize,
        exec2_ms: Option<i64>,
        /// `output-key` (finger down) at exec.
        output: bool,
        /// `release-key` (finger up) at exec — `release`/`release-recoil`.
        release_exec: bool,
    },
    /// `hand-to-mouse`/`hand-to-home` (`hand-ply`).
    Hand { to_mouse: bool, r: f32, theta: f32 },
    /// `prepare`/`execute`/`point-hand-at-key`: a `hand-ply` that moves the
    /// hand *loc* by a polar vector (`motor.lisp:1096`/`:1149`). `r`/`theta`
    /// are pre-rendered because a `prepare` request's `r 1` prints as an
    /// integer (`1`) while a computed polar prints as a float (`1.0`).
    HandPly {
        target: (i64, i64),
        r: String,
        theta: String,
    },
    /// `all-fingers-to-home`: both hands to the home row.
    AllHome { hands: Vec<usize> },
    /// `release-all-fingers` (`motor-extension.lisp:2512`): release the held
    /// fingers of the given hands, without moving them.
    ReleaseAll { hands: Vec<usize> },
}

/// extended-motor-actions: the last prepared movement's features, for
/// `compute-prep-time`/`feat-differences` (`general-pm.lisp:670`).
#[derive(Debug, Clone)]
struct ZzMoveInfo {
    /// `style-name` (e.g. `PECK`/`PUNCH`/`PLY`), compared by
    /// `compute-prep-time`.
    style_name: String,
    hand: usize,
    finger: usize,
    r: f32,
    theta: f32,
    /// `num-possible-feats` = 1 + feature-slot count.
    maxfeats: i64,
    /// Whether the style is `hfrt-movement` (hand finger r theta).
    hfrt: bool,
}

#[derive(Debug, Clone)]
struct ZzPending {
    key: String,
    req_ms: i64,
    hand: usize,
    /// `hand BOTH` (`all-fingers-to-home`): occupies both exec stages.
    both: bool,
    style: String,
    /// Time offsets from the start of execution.
    exec_ms: i64,
    finish_ms: i64,
    kind: ZzKind,
}

/// tracker extra: a learned value tracker (`extras/tracker/tracker.lisp`).
#[derive(Debug, Clone)]
struct Tracker {
    name: String,
    active: bool,
    control_buffer: String,
    control_slot: String,
    choices: Vec<f64>,
    /// `(N, SX1, SX2, SX3, SX4, SYg, SX1Yg, SX2Yg, SYb, SX1Yb, SX2Yb)`.
    statistics: Vec<f64>,
    params: [f64; 3],
    init_temp: f64,
    temperature: f64,
    delay: f64,
    scale_temp_ms: f64,
    active_time: i64,
    good_buffer: Option<String>,
    good_slot: Option<String>,
    good_weight: f64,
    good_value: f64,
    bad_buffer: Option<String>,
    bad_slot: Option<String>,
    bad_weight: f64,
    bad_value: f64,
    last_update: i64,
    setting: f64,
    update_gen: u64,
}

/// act-touch: a computed touch movement (`extras/act-touch/act-touch.lisp`).
/// Times are relative to the request; `signal`/`monitor` are the exact trace
/// and task-output lines emitted at execution time.
#[derive(Debug, Clone)]
struct TouchPlan {
    req_ms: i64,
    prep_ms: i64,
    init_ms: i64,
    exec_ms: i64,
    finish_ms: i64,
    signal: String,
    signal_module: String,
    monitor: String,
    hand: String,
    finger: String,
    down_at_init: bool,
    down_at_exec: bool,
    up_at_exec: bool,
    up_at_exec_burst: bool,
    move_to: Option<(i64, i64)>,
    put_hand: bool,
}

/// Standard finger offsets (relative to the hand loc) per hand
/// (`motor.lisp:508`): `(index, middle, ring, pinkie, thumb)`.
const LEFT_FINGER_OFFSETS: [(i64, i64); 5] = [(0, 0), (-1, 0), (-2, 0), (-3, 0), (1, 2)];
const RIGHT_FINGER_OFFSETS: [(i64, i64); 5] = [(0, 0), (1, 0), (2, 0), (3, 0), (-1, 2)];

/// `hand-ply` and `all-fingers-to-home` set `finger :dummy`
/// (`motor.lisp:1063`, `motor-extension.lisp:2263`); a sentinel lets
/// `feat-differences` compare two dummies as equal without colliding with a
/// real finger index.
const ZZ_DUMMY_FINGER: usize = usize::MAX;

/// associative-learning: the learned state attached to one chunk
/// (`associative-learning.lisp:205`+, the `al-ni-count`/`al-cj-*`/… extended
/// slots).
#[derive(Debug, Clone, Default)]
struct AlChunk {
    /// `al-ni-count` / `al-ni-refs`: times this chunk was the needed item.
    ni_count: i64,
    ni_refs: Vec<i64>,
    /// `al-cj-count` / `al-cj-refs`: times this chunk was in the context.
    cj_count: i64,
    cj_refs: Vec<i64>,
    /// `al-ni+cj-count` / `al-ni+cj-refs`: per needed-item `i` while this
    /// chunk was in the context (alists keyed by `i`'s true name).
    ni_cj_count: Vec<(String, i64)>,
    ni_cj_refs: Vec<(String, Vec<i64>)>,
    /// `associated-js` (only used by the `sdp` reporting path).
    associated_js: Vec<String>,
}

/// extended-motor-actions: a hand's current keyboard position.
#[derive(Debug, Clone)]
struct ZzHand {
    loc: (i64, i64),
    offsets: [(i64, i64); 5],
    down: [bool; 5],
    busy: [bool; 5],
    /// The key each finger is holding down, if any.
    held: [Option<String>; 5],
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

/// Like [`sgp_bool`] but `true` when the key is absent — for parameters whose
/// ACT-R default is `t` (`:ncnar`, `:dcnn`).
fn sgp_tri_state(params: &[(String, Sexp)], key: &str) -> bool {
    match params.iter().find(|(k, _)| k == key) {
        Some((_, Sexp::Sym(s))) => s != "NIL",
        Some(_) => true,
        None => true,
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
            // `(quote high)` from `sgp-fct` with `'high`.
            Some((_, Sexp::List(v))) if v.len() == 2 && v[0].as_sym() == Some("QUOTE") => {
                match v[1].as_sym() {
                    Some("HIGH") => Detail::High,
                    Some("MEDIUM") => Detail::Medium,
                    Some("LOW") => Detail::Low,
                    got => return Err(bad(format!("bad :trace-detail {got:?}"))),
                }
            }
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
            // A repeated key (`:do-not-harvest imaginal :do-not-harvest
            // visual-location`) extends the set for every occurrence.
            for (_, v) in m.params.iter().filter(|(k, _)| k == key) {
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
        // `:unstuff-visual-location` (`vision.lisp:3499`): nil disables the
        // automatic unstuff; t uses `new-span`; a number is the delay itself.
        let onset_span_ms = secs_to_ms(":VISUAL-ONSET-SPAN", 0.5)?;
        let unstuff_vl_ms = match m
            .params
            .iter()
            .find(|(k, _)| k == ":UNSTUFF-VISUAL-LOCATION")
        {
            None => Some(onset_span_ms),
            Some((_, Sexp::Sym(s))) if s == "NIL" => None,
            Some((_, Sexp::Sym(_))) => Some(onset_span_ms),
            Some((_, Sexp::Float(x))) => Some((x * 1000.0).round() as i64),
            Some((_, Sexp::Int(i))) => Some(i * 1000),
            Some((_, got)) => return Err(bad(format!("bad :unstuff-visual-location {got:?}"))),
        };
        let mut cfg = Config {
            detail,
            esc: sgp_bool(&m.params, ":ESC"),
            lf_ms,
            le: sgp_single(&m.params, ":LE")?.unwrap_or(1.0),
            rt: sgp_single(&m.params, ":RT")?.unwrap_or(0.0),
            blc: sgp_single(&m.params, ":BLC")?.unwrap_or(0.0),
            ans: sgp_single(&m.params, ":ANS")?.filter(|s| *s > 0.0),
            ul: sgp_bool(&m.params, ":UL"),
            ult: sgp_bool(&m.params, ":ULT"),
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
            time_noise: sgp_single(&m.params, ":TIME-NOISE")?.unwrap_or(0.015),
            time_mult: sgp_single(&m.params, ":TIME-MULT")?.unwrap_or(1.1),
            time_start_ms: (sgp_num(&m.params, ":TIME-MASTER-START-INCREMENT", 0.011)? * 1000.0)
                as f32,
            record_ticks: match m.params.iter().find(|(k, _)| k == ":RECORD-TICKS") {
                None => true,
                Some((_, Sexp::Sym(s))) => s != "NIL",
                Some(_) => true,
            },
            randomize_scale: match m.params.iter().find(|(k, _)| k == ":RANDOMIZE-TIME") {
                None => None,
                Some((_, Sexp::Sym(s))) if s == "NIL" => None,
                Some((_, Sexp::Sym(_))) => Some(3.0),
                Some((_, Sexp::Float(x))) => Some(*x as f32),
                Some((_, Sexp::Int(i))) => Some(*i as f32),
                _ => None,
            },
            vpft: sgp_bool(&m.params, ":VPFT"),
            vidt: sgp_bool(&m.params, ":VIDT"),
            // `update-chunks-on-the-fly` = `:ncnar` non-nil AND `:dcnn`
            // (`framework/naming-module.lisp:452`). Both default to `t`; a
            // model that sets either to nil opts out of name remapping
            // (`sji-merge` sets `:dcnn nil`).
            chunk_update: sgp_tri_state(&m.params, ":NCNAR")
                && sgp_tri_state(&m.params, ":DCNN"),
            threads: m
                .required_extras
                .iter()
                .any(|e| e.eq_ignore_ascii_case("threads")),
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
            finst_span_ms: (sgp_num(&m.params, ":DECLARATIVE-FINST-SPAN", 3.0)? * 1000.0).round()
                as i64,
            visual_num_finsts: sgp_num(&m.params, ":VISUAL-NUM-FINSTS", 4.0)? as usize,
            visual_finst_span_ms: (sgp_num(&m.params, ":VISUAL-FINST-SPAN", 3.0)? * 1000.0).round()
                as i64,
            bll,
            ol,
            act_scale,
            egs: sgp_single(&m.params, ":EGS")?.unwrap_or(0.0),
            dat_ms: (sgp_num(&m.params, ":DAT", 0.05)? * 1000.0).round() as i64,
            attention_ms: secs_to_ms(":VISUAL-ATTENTION-LATENCY", 0.085)?,
            unstuff_vl_ms,
            imaginal_delay_ms: secs_to_ms(":IMAGINAL-DELAY", 0.2)?,
            feat_prep_ms: secs_to_ms(":MOTOR-FEATURE-PREP-TIME", 0.05)?,
            init_ms: secs_to_ms(":MOTOR-INITIATION-TIME", 0.05)?,
            burst_ms: secs_to_ms(":MOTOR-BURST-TIME", 0.05)?,
            emma: false,
            emma_sacc_rate: sgp_single(&m.params, ":EYE-SACCADE-RATE")?.unwrap_or(0.002),
            emma_base_exe: sgp_single(&m.params, ":SACCADE-BASE-TIME")?.unwrap_or(0.020),
            emma_init_time: sgp_single(&m.params, ":SACCADE-INIT-TIME")?.unwrap_or(0.050),
            emma_feat_prep: sgp_single(&m.params, ":SACCADE-FEAT-TIME")?.unwrap_or(0.050),
            emma_enc_factor: sgp_single(&m.params, ":VISUAL-ENCODING-FACTOR")?.unwrap_or(0.006),
            emma_enc_exp: sgp_single(&m.params, ":VISUAL-ENCODING-EXPONENT")?.unwrap_or(0.4),
            emma_obj_freq: sgp_single(&m.params, ":VIS-OBJ-FREQ")?.unwrap_or(0.1),
            key_closure_ms: secs_to_ms(":KEY-CLOSURE-TIME", 0.01)?,
            touch: m
                .required_extras
                .iter()
                .any(|e| e.eq_ignore_ascii_case("act-touch")),
            touch_ppi: sgp_single(&m.params, ":PIXELS-PER-INCH")?.unwrap_or(72.0),
            touch_height: sgp_single(&m.params, ":FINGER-HEIGHT")?.unwrap_or(1.0),
            touch_finger_spacing: sgp_single(&m.params, ":FINGER-SPACING")?.unwrap_or(0.75),
            touch_thumb_width: sgp_single(&m.params, ":THUMB-WIDTH")?.unwrap_or(1.0),
            touch_peck_coeff: sgp_single(&m.params, ":PECK-FITTS-COEFF")?.unwrap_or(0.075),
            touch_cursor_coeff: sgp_single(&m.params, ":CURSOR-FITTS-COEFF")?.unwrap_or(0.1),
            touch_swipe_speed: sgp_single(&m.params, ":SWIPE-SPEED")?.unwrap_or(3.0) as i64,
            touch_rotate_width: sgp_single(&m.params, ":TOUCH-ROTATE-WIDTH")?.unwrap_or(5.0),
            touch_drag_delay: sgp_bool(&m.params, ":TOUCH-DRAG-DELAY"),
            touch_left_start: (0, 300),
            touch_right_start: (1024, 300),
            tracker: m
                .required_extras
                .iter()
                .any(|e| e.eq_ignore_ascii_case("tracker")),
            trace_tracker: sgp_bool(&m.params, ":TRACE-TRACKER"),
            zz_ext: m
                .required_extras
                .iter()
                .any(|e| e.eq_ignore_ascii_case("extended-motor-actions")),
            zz_two_proc: false,
            zz_two_exec: false,
            zz_query: "STATE".to_string(),
            zz_kct_ms: 10,
            zz_krt_ms: 40,
            zz_cdd: None,
            zz_psd: None,
            al_eal: sgp_single(&m.params, ":EAL")?,
            al_prior_p: sgp_single(&m.params, ":AL-PRIOR-P")?.unwrap_or(0.01),
            al_prior_w: sgp_single(&m.params, ":AL-PRIOR-W")?.unwrap_or(10.0),
            // The module's `:al-show` default is `t`.
            al_show: match m.params.iter().find(|(k, _)| k == ":AL-SHOW") {
                Some((_, Sexp::Sym(s))) if s == "NIL" => false,
                _ => true,
            },
            zz_dpd_ms: 75,
            zz_fpd_ms: 50,
            zz_spd_ms: 100,
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
            seed: match m.params.iter().find(|(k, _)| k == ":SEED") {
                Some((_, Sexp::List(v))) if v.len() == 2 => match (&v[0], &v[1]) {
                    (Sexp::Int(a), Sexp::Int(b)) => Some((*a as u64, *b as u64)),
                    _ => None,
                },
                _ => None,
            },
        };
        // `al-params` (`associative-learning.lisp:421`): enabling `:eal` sets
        // `:mas` to π so spreading activation runs (the module replaces the
        // Sji values via its hook).
        if cfg.al_eal.is_some() {
            cfg.mas = Some(std::f32::consts::PI);
        }
        Ok(cfg)
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

#[derive(Debug, Clone)]
struct Buffer {
    chunk: Option<ChunkInst>,
    failed: bool,
    busy: bool,
    /// Set by `mod-buffer-chunk` (a production's `=buffer>` RHS): the vision
    /// module's `check-unstuff-buffer` only unstuffs an *unmodified* copy.
    modified: bool,
    /// `act-r-buffer-requested` (`buffers.lisp`): whether the chunk in the
    /// buffer was explicitly requested. A `?buffer> buffer requested/
    /// unrequested` query tests it (`buffers.lisp:1113`), and the vision
    /// module auto-stuffs a location with `:requested nil`.
    requested: bool,
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
            modified: false,
            requested: true,
        }
    }
}

/// One audio sound event (`sound-event`, `core-modules/audio.lisp:571`). A
/// tone's `sname` is `TONE{n}` (chunk `isa sound`) and its `ename` is
/// `AUDIO-EVENT{n}` (chunk `isa audio-event`); the two name counters are
/// independent (`new-name-fct`).
#[derive(Debug, Clone)]
struct Sound {
    sname: String,
    ename: String,
    kind: String,
    content: i64,
    onset: i64,
    offset: i64,
    duration: i64,
    /// `tone-detect-delay` (50 ms): detect time is `onset + delay`.
    delay: i64,
    /// `tone-recode-delay` (285 ms): `attend-sound` encoding time.
    recode: i64,
    pitch: String,
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
        /// Trace module: `PROCEDURAL` for production clears, `NONE` for a
        /// scheduled `clear-buffer` (`buffers.lisp:1265`).
        module: String,
    },
    ModBuffer {
        buffer: String,
        slots: Vec<(String, Value)>,
        /// Trace module: `PROCEDURAL` for `=buffer>`, the buffer's owning
        /// module for `*buffer>` (`buffer-mod-request`).
        module: String,
    },
    ModuleRequest {
        buffer: String,
        spec: NewChunk,
        /// Slots whose request value was a variable (skipped by
        /// partial matching, which only scores constants).
        var_slots: Vec<String>,
        /// Whether to print `MODULE-REQUEST <buffer>`: production requests do
        /// (`schedule-module-request`, output medium), a task's
        /// `module-request` does not.
        trace: bool,
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
    /// `start-tracking` (`vision.lisp:2665`): begin tracking the currently
    /// attended object (scheduled so the request rule's implicit
    /// clear-buffer lands first).
    StartTracking,
    /// Imaginal request delay elapsed: free the module and place the chunk.
    ImaginalDone {
        spec: NewChunk,
    },
    /// Attention landed: build the visual object.
    EncodingComplete {
        loc: String,
        /// `:requested` (`encoding-complete`): a model-requested attention
        /// sets the buffer requested; the `update-attended-loc` re-encode
        /// does not.
        requested: bool,
    },
    /// EMMA: a saccade's preparation finished (`PREPARATION-COMPLETE`).
    EmmaPrepComplete {
        trgt: Option<String>,
    },
    /// EMMA: a saccade's initiation finished (`INITIATION-COMPLETE`).
    EmmaInitComplete,
    /// EMMA: a saccade completed (`COMPLETE-EYE-MOVE`).
    EmmaEyeMoveComplete {
        sacc: EmmaSacc,
    },
    /// `+visual> cmd clear` direct event (output low); the buffer
    /// itself is cleared by the request-rule implicit event.
    VisualClear,
    /// Vision `unstuff-buffer` (`general-pm.lisp:896`): erase an unused
    /// auto-stuffed visual-location chunk once `new-span` elapses.
    UnstuffVl {
        copy: String,
    },
    /// Run-full-time dummy: keeps the run going to the time limit. Also used
    /// non-maintenance for a real-time slack hook's `(schedule-event <t> 'fn)`
    /// no-op, which is hidden at medium but wakes an armed CR.
    Dummy,
    /// A deferred trace line (`schedule-event-now` from a request handler).
    Emit {
        module: String,
        line: String,
    },
    /// tracker extra: a scheduled `update-tracker`.
    TrackerUpdate {
        gen: u64,
    },
    /// tracker task: `choose` re-prompts the visicon feature.
    Choose,
    /// associative-learning: `al-buffer-cleared`
    /// (`associative-learning.lisp:667`) scheduled at priority 5.
    AlBufferCleared {
        chunk: String,
    },
    /// `zz-extended-motor-actions`: a keyboard style's preparation complete.
    ZzKeyPrep {
        cmd: String,
        style: String,
        key: String,
        hand: usize,
        finger: usize,
        r: f32,
        theta: f32,
        /// Absolute target location for the finger-offset update; `None`
        /// derives it from `key` (keyboard styles), `Some` passes a home
        /// position (`move-finger-to-home`).
        loc: Option<(i64, i64)>,
        /// `delayed-punch`'s release delay (`delay` slot); 0 for other styles.
        delay_ms: i64,
        req_ms: i64,
    },
    /// extended-motor-actions: keyboard `output-key`/`release-key` signal.
    ZzKeyOutput {
        key: String,
        down: bool,
        hand: usize,
        finger: usize,
    },
    /// extended-motor-actions: `hand-to-mouse`/`hand-to-home` prep complete.
    ZzHandPrep {
        req_ms: i64,
        r: f32,
        theta: f32,
        to_mouse: bool,
    },
    /// extended-motor-actions: `prepare`/`execute`/`point-hand-at-key`
    /// (`hand-ply`) prep complete. `deferred` marks a `prepare` whose
    /// execution waits for an `execute` request (`general-pm.lisp:562`).
    ZzHandPlyPrep {
        class: String,
        hand: usize,
        r: f32,
        theta: f32,
        r_str: String,
        theta_str: String,
        target: (i64, i64),
        req_ms: i64,
        deferred: bool,
    },
    /// extended-motor-actions: `all-fingers-to-home` prep complete.
    ZzAllHomePrep {
        req_ms: i64,
        hands: Vec<usize>,
        r: f32,
    },
    /// extended-motor-actions: `release-all-fingers` prep complete.
    ZzReleaseAllPrep {
        req_ms: i64,
        hands: Vec<usize>,
    },
    /// extended-motor-actions: mouse movement preparation complete.
    ZzMousePrep {
        cmd: String,
        req_ms: i64,
        loc: Option<(i64, i64, String)>,
    },
    /// extended-motor-actions: the MOUSE device signal at execution.
    ZzMouseNotify {
        cmd: String,
        req_ms: i64,
        loc: Option<(i64, i64, String)>,
    },
    /// extended-motor-actions: `release-mouse` at the second exec time.
    ZzMouseRelease {
        req_ms: i64,
    },
    /// `(mod-focus slot value ...)`: goal-module modification
    /// (`goal.lisp:346`), traced as `GOAL-MODIFICATION`.
    ModFocus {
        slots: Vec<(String, Value)>,
    },
    /// `(modify-visicon-features ...)`: move a visicon feature
    /// (`vision.lisp`); hidden at medium, updates the feature's position.
    ModifyVisicon {
        name: String,
        x: i64,
        y: i64,
    },
    /// act-touch: `preparation-complete` for a touch movement.
    TouchPrep {
        plan: Box<TouchPlan>,
    },
    /// act-touch: `initiation-complete`.
    TouchInit {
        plan: Box<TouchPlan>,
    },
    /// act-touch: execution — the device signal + task monitor output.
    TouchSignal {
        plan: Box<TouchPlan>,
    },
    /// act-touch: a scheduled finger-up.
    TouchUp {
        plan: Box<TouchPlan>,
    },
    /// act-touch: `finish-movement`.
    TouchFinish {
        plan: Box<TouchPlan>,
    },
    /// Temporal module tick: emits the "Incrementing time ticks to N"
    /// detail, bumps the count, and schedules the next tick. A stale `gen`
    /// (a newer request cancelled it) is ignored.
    TemporalTick {
        gen: u64,
        details: String,
    },
    /// Temporal `isa clear` request: emits `TEMPORAL Clear`, drops the timer.
    TemporalClear,
    /// Utility reward propagation (output medium).
    PropagateReward {
        value: Reward,
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
        /// `isa press-key key X` form (no `cmd`): fewer prepared features.
        isa: bool,
    },
    PreparationComplete {
        key: String,
        req_ms: i64,
    },
    InitiationComplete {
        req_ms: i64,
        /// extended-motor-actions: the hand whose processor stage frees.
        hand: usize,
        /// extended-motor-actions: `hand BOTH` (`all-fingers-to-home`).
        both: bool,
        /// extended-motor-actions: the movement's style name.
        style: String,
    },
    MotorNotify {
        key: String,
        req_ms: i64,
    },
    /// Keyboard `output-key`: maintenance class (never wakes CR).
    OutputKey {
        key: String,
    },
    /// extended-motor-actions: `keyboard-with-release` `release-key` signal.
    ReleaseKey {
        key: String,
    },
    FinishMovement {
        req_ms: i64,
        /// extended-motor-actions: the hand whose execution stage frees.
        hand: usize,
        /// extended-motor-actions: `hand BOTH` (`all-fingers-to-home`).
        both: bool,
        /// extended-motor-actions: the movement's style name.
        style: String,
    },
    /// Speech module: the vocal buffer finished its utterance.
    VocalFree,
    /// Speech module: an utterance starts (`SPEAK TEXT ...`).
    Speak {
        text: String,
    },
    /// Speech module: `speak` preparation completed; queues initiation,
    /// the hear-self sound onset, and the finish.
    SpeechPrep {
        text: String,
    },
    /// Speech module: `initiation-complete` (procedural released).
    SpeechInit,
    /// Audio module: a sound begins (`new-sound`).
    NewSound,
    /// Audio module: the sound's aural location is set.
    AuralLocation,
    /// Audio module: a tone's onset (`new-sound`, then schedule
    /// `stuff-sound-buffer` at `onset + detect-delay`).
    ToneSound {
        idx: usize,
    },
    /// Audio module: `stuff-sound-buffer` sets `AURAL-LOCATION` to the
    /// sound's `audio-event` chunk.
    StuffSound {
        idx: usize,
    },
    /// Audio module: `attend-sound` finishes, building the sound chunk and
    /// setting the `AURAL` buffer.
    AudioEncodingComplete {
        idx: usize,
    },
    /// Threads extra: clear the specific goal chunk a production matched
    /// (emits `CLEAR-BUFFER GOAL`, removes it from the goal set).
    ClearGoalThread {
        name: String,
    },
    /// Threads extra: overwrite the goal multi-buffer's *visible* chunk
    /// without clearing the rest of the goal set (`OVERWRITE-BUFFER-CHUNK`).
    /// Also a production `@buffer>` direct set (`production-parsing.lisp:1147`),
    /// traced under `PROCEDURAL`.
    OverwriteBuffer {
        buffer: String,
        chunk: NewChunk,
        requested: bool,
        shown: Option<String>,
        module: String,
    },
    /// A production `@buffer>` direct erase (`production-parsing.lisp:1140`):
    /// `ERASE-BUFFER <buffer>` (remove the chunk, keep the buffer state).
    EraseBuffer {
        buffer: String,
        module: String,
    },
    /// A task command scheduled by model-body `schedule-event` or by
    /// `schedule-event-relative` inside another command. Emits a `NONE`
    /// trace line then runs the registered task function.
    Command {
        name: String,
        params: Vec<Value>,
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
    rng: RefCell<Mt>,
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
    /// `pdisable-fct` (`procedural-cmds.lisp:541`): productions skipped by
    /// conflict resolution. Cleared on `reset` (the model body is
    /// re-evaluated, recreating the productions).
    disabled_prods: HashSet<String>,
    /// Names of the productions in the last computed conflict set (returned
    /// by `whynot`, `procedural-cmds.lisp:597`).
    last_conflict_set: Vec<String>,
    cr_armed: bool,
    dm_gen: u64,
    dm_busy: bool,
    /// Production utilities (init :iu) and selection history
    /// (prod index, selection time ms) for reward propagation.
    utilities: Vec<f32>,
    /// `production-utility` (`utility-and-reward-1.lisp:357`): the utility
    /// cached by the last conflict resolution (`compute-utility` with
    /// `save`), printed as `:utility` (nil until a candidate).
    production_utility: Vec<Option<f32>>,
    history: Vec<(usize, i64)>,
    /// Chunk similarities, as given (lookup tries both orders;
    /// mirrors set-similarities).
    sims: HashMap<(String, String), f32>,
    /// The model's possible slot names (`valid-slot-name`,
    /// `chunk-types.lisp:590`): the union of every chunk-type's slots, plus
    /// slots added at run time. A production's `=buffer>`/`*buffer>` mod that
    /// instantiates a slot outside this set extends the chunk and emits
    /// `EXTEND-BUFFER-CHUNK <buffer>` before the modification
    /// (`instantiate-chunk-spec`, `chunk-spec.lisp:777`).
    valid_slots: HashSet<String>,
    /// Scripted device items not yet shown, by appearance time.
    pending_texts: Vec<DeviceText>,
    /// Scripted window-item moves not yet applied, by time.
    pending_modifies: Vec<DeviceModify>,
    /// Scripted window clears not yet applied, sorted.
    pending_clears: Vec<i64>,
    /// The full device script, re-armed on every `reset` (a task re-applies
    /// its window setup each run; `pending_*` is consumed as time advances).
    device_texts: Vec<DeviceText>,
    device_modifies: Vec<DeviceModify>,
    device_clears: Vec<(i64, Option<u32>)>,
    /// Scheduled command lines, re-armed on every `reset` like the rest of
    /// the device script.
    device_commands: Vec<DeviceCommand>,
    /// Scripted tone sounds, re-armed on every `reset`.
    device_sounds: Vec<DeviceSound>,
    /// The current run index for `(RUN n …)` device groups; the construction
    /// reset is run 0, the spec's first `(reset)` run 1.
    device_run: u32,
    /// The task's non-replayed setup draws (device sidecar): applied after
    /// the model-body seed on every `reset`, since the task redoes its setup
    /// per run.
    consume_rng: u64,
    /// Live visicon features, in proc-display order.
    features: Vec<Feature>,
    feat_seq: u64,
    /// Live experiment-window origin (`open-exp-window` x/y; ACT-R default
    /// 300,300). RPC-driven window items are placed relative to it.
    window_origin: (i64, i64),
    /// Live window items: item key -> the visicon feature names it created
    /// (`add-text-to-exp-window` etc.), for `remove-items-from-exp-window`.
    window_items: Vec<(String, Vec<String>)>,
    /// Auto-naming counter for live window items (`TEXT-VDI-{n}`).
    window_item_seq: u64,
    /// BOLD demand recorder (`other-files/bold.lisp` + the buffer-trace):
    /// the traced buffers, the open demand-period start (seconds) per
    /// buffer, and the recorded `(start, end)` periods in seconds.
    bold_buffers: Vec<String>,
    bold_open: HashMap<String, f64>,
    bold_demand: HashMap<String, Vec<(f64, f64)>>,
    /// Location-name copy counter: requested base -> copies made.
    copy_count: HashMap<String, u64>,
    /// Copy fan-out (`c-fan-out`): a copied chunk's source fan-out at copy
    /// time, used for Sji when the source is not in DM
    /// (`chunk-fan-j-to-i`, declarative-memory.lisp:2554).
    c_fan_out: HashMap<String, u32>,
    /// Buffer chunk name -> final DM chunk name, for chunk references that
    /// were merged into an existing DM chunk (`merging-test`).
    chunk_alias: HashMap<String, String>,
    /// Copy name -> its source chunk name (`copy_chunk`).
    copy_source: HashMap<String, String>,
    /// `!stop!` was executed: end the run with a BREAK-EVENT.
    stopped: bool,
    /// Recently-retrieved chunk finsts: `(name, time_ms)`, newest last.
    finsts: Vec<(String, i64)>,
    /// Threads extra: the goal multi-buffer's ordered chunk set.
    goal_set: Vec<ChunkInst>,
    /// Speech module: the vocal buffer is busy speaking/subvocalizing.
    vocal_busy: bool,
    /// Audio module: sounds emitted so far (`AUDIO-EVENT{n}`).
    audio_events: u64,
    /// Audio module: live sound events (`TONE{n}`/`AUDIO-EVENT{n}`).
    sounds: Vec<Sound>,
    /// Audio module: next `TONE{n}` index (`new-name-fct "TONE"`).
    tone_seq: u64,
    /// Auto-naming counters for `define-chunks`-style names (`CHUNK0`, ...).
    name_count: HashMap<String, u64>,
    /// A reusable buffer's chunk name (`act-r-buffer-reuse-chunk`,
    /// `framework/buffers.lisp:1351`): assigned once per run and reused, so
    /// every set into the buffer keeps the same `{BUFFER}-CHUNK{n}` name.
    buffer_reuse: HashMap<String, String>,
    /// SET-copy name -> feature index, resolving copies back for
    /// attention and encoding.
    vl_copies: HashMap<String, usize>,
    /// Visual-location copy name -> the chunk it was copied from
    /// (`chunk-copied-from-fct`), printed as `[source]` by `buffer-chunk`.
    vl_copied_from: HashMap<String, String>,
    /// Visual-object names per kind: KIND -> objects so far.
    obj_built: HashMap<String, u64>,
    /// Visual-object chunk name → the visicon feature it was encoded from
    /// (`chunk-visicon-entry`), for tracking's `vis-obj-marker` test.
    obj_feature: HashMap<String, String>,
    visual_busy: bool,
    /// `loc-failure` (`vision.lisp:2931`): the last `find-location` found no
    /// matching location; `?visual-location> state error` tests it.
    loc_failure: bool,
    /// `clear` on the vision module (`pm-module` clear, `general-pm.lisp:333`)
    /// sets `prep` busy for `feat-prep-time`, so `?visual> state free` stays
    /// false for ~50 ms after a `+visual> cmd clear` (`VISION CLEAR`).
    visual_prep_busy: bool,
    /// Location copy named in the last `move-attention`, re-encoded when
    /// the window changes underneath it.
    last_attention: Option<String>,
    /// Vision tracking (`start-tracking`/`update-tracking-mth`,
    /// `vision.lisp:2665`): the visicon feature being tracked and the
    /// location chunk that last held it (`tracked-obj-lastloc`).
    tracked: Option<String>,
    tracked_lastloc: Option<String>,
    /// EMMA `current-marker`: the eye position as a 3-vector (default
    /// `#(0 0 1)`, `emma.lisp:386`).
    emma_eye: (f32, f32, f32),
    /// EMMA `prep-s`/`exec-s` movement states.
    emma_prep_s: bool,
    emma_exec_s: bool,
    /// EMMA `next-loc`: where the eye will land after the current movement.
    emma_next_loc: Option<(f32, f32, f32)>,
    /// EMMA `final-shift`: the requested landing point (a location's xyz).
    emma_final_shift: (f32, f32, f32),
    /// EMMA `shift-target`: the object chunk the eye is moving to.
    emma_shift_target: Option<String>,
    emma_shift_start_ms: i64,
    /// EMMA `shift-duration` in seconds.
    emma_shift_duration: f64,
    /// EMMA `encoded`: whether `initiate-eye-move` scheduled the encoding.
    emma_encoded: bool,
    /// EMMA `last-prep`: the most recently prepared saccade.
    emma_last_prep: Option<EmmaSacc>,
    /// `(ssp :starting-parameters (:key val ...))` (`central-parameters.lisp`):
    /// re-applied on every reset, like the model's own parameters.
    starting_params: Vec<(String, Sexp)>,
    /// Device clears the window on each keypress (task `output-key` handler).
    clear_on_keypress: bool,
    man_prep_busy: bool,
    man_exec_busy: bool,
    /// extended-motor-actions: per-hand processor busy (left=0, right=1).
    man_proc: [bool; 2],
    /// extended-motor-actions: per-hand execution busy.
    man_exec: [bool; 2],
    /// extended-motor-actions: movements waiting for an execution stage.
    man_exec_queue: Vec<ZzPending>,
    /// extended-motor-actions: hand of the last *prepared* movement; the
    /// `initiation-complete` handler frees this hand, not the movement's own
    /// (`motor-extension.lisp:1005`).
    man_last_prep: Option<usize>,
    /// extended-motor-actions: per-hand last request class for `last-command`
    /// (`motor-extension.lisp:1244`, set from `track-hand-requests`).
    zz_last_cmd: [String; 2],
    /// extended-motor-actions: recorded `(kind, time_ms, key)` for the
    /// `display-data` summary (`:pressed`/`:released`/…).
    zz_data: Vec<(String, i64, String)>,
    /// extended-motor-actions: the virtual mouse cursor `(x, y, z)`.
    zz_mouse_pos: (i64, i64, i64),
    /// extended-motor-actions: the mouse cursor's device position
    /// (`cursor-pos`, `mouse.lisp:192`), used by `hand-to-cursor`.
    zz_mouse_cursor: (i64, i64),
    /// extended-motor-actions: whether the mouse button is held down.
    zz_mouse_down: bool,
    /// extended-motor-actions: per-hand keyboard geometry (left=0, right=1).
    zz_hands: [ZzHand; 2],
    /// extended-motor-actions: the last movement's `(class, style-name, hand)`
    /// for the `style X hand Y` event details (class) and
    /// `compute-prep-time` differences (style-name).
    zz_last_style: Option<(String, String, String)>,
    /// extended-motor-actions: the last prepared movement for feature-prep
    /// timing (`compute-prep-time`).
    zz_last_move: Option<ZzMoveInfo>,
    /// extended-motor-actions: a `prepare`d movement awaiting `execute`
    /// (`general-pm.lisp:562`/`:580`); the movement is prepared but not queued
    /// for execution until `execute` arrives.
    zz_prepared: Option<ZzPending>,
    /// extended-motor-actions: `pbreak` production names that halt a run.
    zz_breaks: Vec<String>,
    /// Set when the current run hit one of the `zz_breaks` productions.
    zz_break_hit: bool,
    /// `associative-learning` driver: permuted people/places and the
    /// `create-stims` lists (`al-fan-study.lisp:245`).
    al_people: Vec<String>,
    al_places: Vec<String>,
    al_sentences: Vec<(String, String, i64, i64)>,
    al_targets: Vec<(String, String, i64, i64)>,
    al_foils: Vec<(String, String, i64, i64)>,
    al_fillers: Vec<(String, String, i64, i64)>,
    /// associative-learning module state (`associative-learning.lisp`).
    al_chunks: HashMap<String, AlChunk>,
    al_total_count: i64,
    al_total_refs: Vec<i64>,
    al_total_creation_time: i64,
    /// `al-module-last-retrieval-contexts`: the source-buffer chunks at the
    /// last retrieval request.
    al_last_contexts: Vec<String>,
    imag_busy: bool,
    /// Temporal module: current tick interval (ms), tick count, the
    /// one-time start increment (ms), and a generation counter that cancels
    /// a pending tick when a new request arrives.
    temporal_tick: f32,
    temporal_ticks: u64,
    temporal_start: Option<f32>,
    temporal_gen: u64,
    /// Set by a handler whose event must not wake conflict resolution
    /// (e.g. a tick cancelled by a newer temporal request).
    event_silent: bool,
    /// Blending module busy/error flags (`blending-module-busy/-error`).
    blending_busy: bool,
    blending_error: bool,
    /// Production compilation: count of compiled productions, the previous
    /// fired production `(index, bindings)`, and its fire time.
    compiled_count: usize,
    /// Number of model productions before any compilation (compiled
    /// productions are dropped on reset).
    base_prods: usize,
    /// `(index, bindings, whole-chunk requests)` at the previous fire.
    prev_prod: Option<(usize, Bindings, Vec<(String, String)>)>,
    prev_prod_ms: i64,
    /// Last press-key `(key, hand, finger, r, theta)`, for preparation
    /// differences (style is recovered from the key).
    last_press: Option<(String, String, String, f32, f32)>,
    /// Last spoken text, for the `speak` style's preparation differences.
    last_speak: Option<String>,
    /// act-touch: which hands are on the touch device (left=0, right=1).
    touch_hands: [bool; 2],
    /// act-touch: finger positions `[hand][finger] = (x, y, z)`, z=1 up.
    touch_fingers: [[(i64, i64, i64); 5]; 2],
    /// act-touch: `(style, hand, finger, r, theta)` of the last prepared
    /// movement, for `compute-prep-time` differences.
    touch_last: Option<(String, String, String, f32, f32)>,
    /// Visual-location copy name → its captured `(screen-x, screen-y)`
    /// (the chunk keeps its position even if the feature later moves).
    vl_xy: HashMap<String, (i64, i64)>,
    /// tracker extra: live trackers.
    trackers: Vec<Tracker>,
    /// tracker extra: generation counter for scheduled update events.
    tracker_gen: u64,
    /// tracker extra: the task's visicon feature (prompt cycling).
    tracker_task: bool,
    tracker_feature: Option<String>,
    tracker_target: String,
    /// Trace output enabled (`:v`). Scripted drivers can toggle it.
    trace_on: bool,
    /// Task-file `defun`s available to `!eval!`/`!bind!`.
    user_fns: HashMap<String, crate::model::UserDefun>,
}

/// Buffers whose `+buffer>` requests name a module command (`+visual>`,
/// `+manual>`, `+vocal>`, `+aural>`, `+temporal>`); a `+retrieval>`/`+goal>`/
/// `+imaginal>` request's `isa` is a chunk type instead.
fn is_module_command_buffer(buffer: &str) -> bool {
    matches!(
        buffer.to_uppercase().as_str(),
        "VISUAL" | "MANUAL" | "VOCAL" | "AURAL" | "TEMPORAL"
    )
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
            modifies,
            clears,
            commands,
            sounds,
            end,
            rewards,
            sims,
            clear_on_keypress,
            consume_rng,
        } = device;
        let base_prods = model.productions.len();
        let valid_slots: HashSet<String> = model
            .chunk_types
            .iter()
            .flat_map(|ct| ct.slots.iter())
            .map(|s| s.to_uppercase())
            .collect();
        let mut eng = Engine {
            model,
            cfg,
            rng: RefCell::new(rng),
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
            disabled_prods: HashSet::new(),
            last_conflict_set: Vec::new(),
            cr_armed: false,
            dm_gen: 0,
            dm_busy: false,
            utilities: Vec::new(),
            production_utility: Vec::new(),
            history: Vec::new(),
            sims: sims
                .iter()
                .map(|(a, b, v)| ((a.clone(), b.clone()), *v))
                .collect(),
            valid_slots,
            pending_texts: {
                let mut v = texts.clone();
                v.sort_by_key(|t| t.at_ms);
                v
            },
            pending_modifies: {
                let mut v = modifies.clone();
                v.sort_by_key(|m| m.at_ms);
                v
            },
            pending_clears: {
                let mut v: Vec<i64> = clears.iter().map(|(t, _)| *t).collect();
                v.sort_unstable();
                v
            },
            device_texts: {
                let mut v = texts;
                v.sort_by_key(|t| t.at_ms);
                v
            },
            device_modifies: {
                let mut v = modifies;
                v.sort_by_key(|m| m.at_ms);
                v
            },
            device_clears: {
                let mut v = clears;
                v.sort_by_key(|(t, _)| *t);
                v
            },
            device_commands: {
                let mut v = commands;
                v.sort_by_key(|c| c.at_ms);
                v
            },
            device_sounds: {
                let mut v = sounds;
                v.sort_by_key(|s| s.at_ms);
                v
            },
            device_run: 0,
            consume_rng,
            features: Vec::new(),
            feat_seq: 0,
            window_origin: (300, 300),
            window_items: Vec::new(),
            window_item_seq: 0,
            bold_buffers: Vec::new(),
            bold_open: HashMap::new(),
            bold_demand: HashMap::new(),
            copy_count: HashMap::new(),
            c_fan_out: HashMap::new(),
            chunk_alias: HashMap::new(),
            copy_source: HashMap::new(),
            stopped: false,
            finsts: Vec::new(),
            goal_set: Vec::new(),
            vocal_busy: false,
            audio_events: 0,
            sounds: Vec::new(),
            tone_seq: 0,
            name_count: HashMap::new(),
            buffer_reuse: HashMap::new(),
            vl_copies: HashMap::new(),
            vl_copied_from: HashMap::new(),
            obj_built: HashMap::new(),
            obj_feature: HashMap::new(),
            visual_busy: false,
            loc_failure: false,
            visual_prep_busy: false,
            last_attention: None,
            tracked: None,
            tracked_lastloc: None,
            emma_eye: (0.0, 0.0, 1080.0),
            emma_prep_s: false,
            emma_exec_s: false,
            emma_next_loc: None,
            emma_final_shift: (0.0, 0.0, 0.0),
            emma_shift_target: None,
            emma_shift_start_ms: 0,
            emma_shift_duration: 0.0,
            emma_encoded: false,
            emma_last_prep: None,
            starting_params: Vec::new(),
            clear_on_keypress,
            man_prep_busy: false,
            man_exec_busy: false,
            man_proc: [false, false],
            man_exec: [false, false],
            man_exec_queue: Vec::new(),
            man_last_prep: None,
            zz_last_cmd: ["NONE".to_string(), "NONE".to_string()],
            zz_data: Vec::new(),
            zz_mouse_pos: (0, 0, 1080),
            zz_mouse_cursor: (28, 2),
            zz_mouse_down: false,
            zz_hands: [
                ZzHand {
                    loc: (4, 4),
                    offsets: LEFT_FINGER_OFFSETS,
                    down: [false; 5],
                    busy: [false; 5],
                    held: [None, None, None, None, None],
                },
                ZzHand {
                    loc: (7, 4),
                    offsets: RIGHT_FINGER_OFFSETS,
                    down: [false; 5],
                    busy: [false; 5],
                    held: [None, None, None, None, None],
                },
            ],
            zz_last_style: None,
            zz_last_move: None,
            zz_prepared: None,
            zz_breaks: Vec::new(),
            zz_break_hit: false,
            al_people: Vec::new(),
            al_places: Vec::new(),
            al_sentences: Vec::new(),
            al_targets: Vec::new(),
            al_foils: Vec::new(),
            al_fillers: Vec::new(),
            al_chunks: HashMap::new(),
            al_total_count: 0,
            al_total_refs: Vec::new(),
            al_total_creation_time: 0,
            al_last_contexts: Vec::new(),
            imag_busy: false,
            temporal_tick: 0.0,
            temporal_ticks: 0,
            temporal_start: None,
            temporal_gen: 0,
            event_silent: false,
            blending_busy: false,
            blending_error: false,
            compiled_count: 0,
            base_prods,
            prev_prod: None,
            prev_prod_ms: 0,
            last_press: None,
            last_speak: None,
            touch_hands: [false, false],
            touch_fingers: [[(0, 0, 1); 5]; 2],
            touch_last: None,
            vl_xy: HashMap::new(),
            trackers: Vec::new(),
            tracker_gen: 0,
            tracker_task: false,
            tracker_feature: None,
            tracker_target: "2".to_string(),
            trace_on: true,
            user_fns: HashMap::new(),
        };
        // `process-slots-specs` (`chunk-spec.lisp:597`): a request
        // `+buffer> isa TYPE` carries the chunk-type's default slots (e.g.
        // `cmd move-attention`), which the rendering and composition see.
        eng.expand_request_defaults();
        eng.reset_impl()?;
        eng.cfg.module_width = module_width;
        // The construction reset is not part of the recorded trace: ACT-R
        // installs its monitors after the model is loaded, so any warnings it
        // emitted (e.g. the validator's) are dropped. The spec's own `(reset)`
        // step re-emits them. `reset` applies the model-body seed and the
        // task's setup draws (see `reset`).
        eng.out_lines.clear();
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
                Event::PropagateReward {
                    value: Reward::Num(*value),
                },
            );
        }
        Ok(eng)
    }

    /// `process-slots-specs` (`chunk-spec.lisp:597`): expand a request spec's
    /// `isa TYPE` into that chunk type's default slots (e.g. `cmd
    /// move-attention`), so rendering and composition match ACT-R.
    fn expand_request_defaults(&mut self) {
        let types: Vec<(String, Vec<(String, Pattern)>)> = self
            .model
            .chunk_types
            .iter()
            .map(|t| (t.name.to_uppercase(), t.defaults.clone()))
            .collect();
        for prod in self.model.productions.iter_mut() {
            for a in prod.rhs.iter_mut() {
                if let RhsAction::Request {
                    buffer,
                    spec: ChunkDesc::Spec { ctype, slots, .. },
                } = a
                {
                    if ctype.is_empty() {
                        continue;
                    }
                    // A module command request (`+visual> isa move-attention`)
                    // carries `cmd <isa>` (`vision.lisp:3086` etc.).
                    if is_module_command_buffer(buffer)
                        && !slots.iter().any(|(s, _)| s.eq_ignore_ascii_case("CMD"))
                    {
                        slots.insert(0, ("CMD".to_string(), Pattern::Sym(ctype.to_uppercase())));
                    }
                    if let Some((_, defs)) =
                        types.iter().find(|(n, _)| n.eq_ignore_ascii_case(ctype))
                    {
                        for (slot, pat) in defs {
                            if !slots.iter().any(|(s, _)| s.eq_ignore_ascii_case(slot)) {
                                slots.push((slot.to_uppercase(), pat.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    /// `(reset)`: re-evaluate the model body's effect on engine state —
    /// rebuild DM and the named-chunk table, clear buffers, reset the clock
    /// to 0, and re-arm the initial goal + conflict resolution. The RNG is
    /// intentionally NOT reseeded (GOTCHAS #5/#35); scripted `:seed` steps
    /// handle that. Trace history (`out_lines`) is preserved.
    pub fn reset(&mut self) -> Result<(), EngineError> {
        self.device_run += 1;
        self.reset_impl()
    }

    /// The body of `reset`; the construction reset calls this directly so the
    /// spec's first `(reset)` is device run 1.
    fn reset_impl(&mut self) -> Result<(), EngineError> {
        // Driver-set extended-motor-actions params survive a reset.
        let zz = (
            self.cfg.zz_two_proc,
            self.cfg.zz_two_exec,
            self.cfg.zz_query.clone(),
            self.cfg.zz_kct_ms,
            self.cfg.zz_krt_ms,
            self.cfg.zz_cdd,
            self.cfg.zz_psd,
            self.cfg.zz_dpd_ms,
            self.cfg.zz_fpd_ms,
            self.cfg.zz_spd_ms,
        );
        self.cfg = Config::from_model(&self.model)?;
        self.cfg.zz_two_proc = zz.0;
        self.cfg.zz_two_exec = zz.1;
        self.cfg.zz_query = zz.2;
        self.cfg.zz_kct_ms = zz.3;
        self.cfg.zz_krt_ms = zz.4;
        self.cfg.zz_cdd = zz.5;
        self.cfg.zz_psd = zz.6;
        self.cfg.zz_dpd_ms = zz.7;
        self.cfg.zz_fpd_ms = zz.8;
        self.cfg.zz_spd_ms = zz.9;
        // `(ssp :starting-parameters ...)` survives a reset, like the
        // model's own parameters (`model.lisp:972`).
        for (k, v) in self.starting_params.clone() {
            self.apply_sgp(&k, &v)?;
        }
        // Compiled productions do not survive a reset.
        self.model.productions.truncate(self.base_prods);
        self.trace_on = true;
        // ACT-R re-evaluates the model body on reset, so the validator's
        // rejection warnings reappear here (the load-time ones are dropped).
        for w in self.model.reset_warnings.clone() {
            self.emit_warning(&w);
        }
        self.dm = Vec::new();
        for c in &self.model.chunks {
            self.dm.push(chunk_inst(c)?);
        }
        self.named = HashMap::new();
        // `create-model-default-chunks` (`model.lisp:753`): every model starts
        // with these state chunks (`(free name free)` etc., plus `clear`); a
        // model-body definition of the same name overrides them below.
        for name in [
            "FREE",
            "BUSY",
            "ERROR",
            "EMPTY",
            "FULL",
            "FAILURE",
            "REQUESTED",
            "UNREQUESTED",
        ] {
            self.named.insert(
                name.to_string(),
                ChunkInst {
                    name: Some(name.to_string()),
                    ctype: "CHUNK".to_string(),
                    slots: vec![("NAME".to_string(), Value::Sym(name.to_string()))],
                    bare: false,
                },
            );
        }
        self.named.insert(
            "CLEAR".to_string(),
            ChunkInst {
                name: Some("CLEAR".to_string()),
                ctype: "CLEAR".to_string(),
                slots: Vec::new(),
                bare: false,
            },
        );
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
        // Model-body `set-all-base-levels`: fixed history for every DM chunk
        // (`commands/dm-commands.lisp:600`); a later named `set-base-levels`
        // overrides.
        if let Some((level, creation)) = self.model.set_all_base_levels {
            let n = level.max(0.0).round() as usize;
            for i in 0..self.hist.len() {
                let creation_ms = creation
                    .map(|s| (s * 1000.0).round() as i64)
                    .unwrap_or(self.hist[i].creation_ms);
                self.hist[i].creation_ms = creation_ms;
                self.hist[i].count = n;
                self.hist[i].fixed_base = Some(level);
                self.hist[i].refs = match &self.cfg.ol {
                    Ol::On => vec![creation_ms; n],
                    Ol::Off => even_references(creation_ms, 0, n),
                    Ol::Num(ol) => even_references(creation_ms, 0, (*ol as usize).min(n)),
                };
            }
        }
        // Model-body `set-base-levels`: fixed history for named DM chunks.
        for (name, level, creation) in &self.model.base_levels {
            if let Some(i) = self
                .dm
                .iter()
                .position(|c| c.name.as_deref() == Some(name.as_str()))
            {
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
            "MANUAL-LEFT",
            "MANUAL-RIGHT",
            "TEMPORAL",
            "VOCAL",
            "AURAL",
            "AURAL-LOCATION",
        ] {
            self.buffers.insert(b.to_string(), Buffer::empty());
        }
        if self.cfg.blending {
            self.buffers.insert("BLENDING".to_string(), Buffer::empty());
        }
        self.queue = BinaryHeap::new();
        // The BOLD demand recorder tracks the current task only: `reset`
        // restarts the meta-process clock, and the buffer-trace recorder the
        // predictor reads is reset with it.
        self.bold_open.clear();
        self.bold_demand.clear();
        self.time_ms = 0;
        self.cr_armed = false;
        self.dm_gen = 0;
        self.dm_busy = false;
        // `(spp <prod> :u U)` overrides the initial utility (`:iu`).
        self.utilities = self
            .model
            .productions
            .iter()
            .map(|p| p.utility.unwrap_or(self.cfg.iu))
            .collect();
        self.production_utility = vec![None; self.model.productions.len()];
        self.history.clear();
        self.features.clear();
        // A device task re-applies its window setup after each reset; restore
        // the script so the next run sees it (`pending_*` is consumed as time
        // advances, and a task runs its setup again per run).
        // Device script: flat ops re-arm on every reset; `(RUN n …)` ops
        // belong to run `n` (the task re-applies its window content per run).
        let run = self.device_run;
        self.pending_texts = self
            .device_texts
            .iter()
            .filter(|t| t.run.is_none() || t.run == Some(run))
            .cloned()
            .collect();
        self.pending_modifies = self
            .device_modifies
            .iter()
            .filter(|m| m.run.is_none() || m.run == Some(run))
            .cloned()
            .collect();
        self.pending_clears = self
            .device_clears
            .iter()
            .filter(|(_, r)| r.is_none() || *r == Some(run))
            .map(|(t, _)| *t)
            .collect();
        // Scheduled command lines re-fire each run (`schedule-event-relative`
        // runs again in the task's setup).
        for c in self
            .device_commands
            .clone()
            .into_iter()
            .filter(|c| c.run.is_none() || c.run == Some(run))
        {
            let out = match c.output.as_str() {
                "low" => Out::Low,
                "medium" => Out::Medium,
                "high" | "t" => Out::Always,
                _ => Out::Low,
            };
            self.push_m(
                c.at_ms,
                Prio::Num(0),
                out,
                false,
                Event::Emit {
                    module: c.module.clone(),
                    line: c.details.clone(),
                },
            );
        }
        self.feat_seq = 0;
        self.copy_count.clear();
        self.c_fan_out.clear();
        self.chunk_alias.clear();
        self.copy_source.clear();
        self.stopped = false;
        self.finsts.clear();
        self.goal_set.clear();
        self.vocal_busy = false;
        self.audio_events = 0;
        // Scripted tones: the task calls `new-tone-sound` during per-run
        // setup, creating the event and scheduling its onset (`new-sound`).
        self.sounds.clear();
        self.tone_seq = 0;
        for s in self
            .device_sounds
            .clone()
            .into_iter()
            .filter(|s| s.run.is_none() || s.run == Some(run))
        {
            let idx = self.add_sound(s.freq, s.duration, s.at_ms);
            self.push(s.at_ms, Prio::Num(0), Out::Medium, Event::ToneSound { idx });
        }
        self.name_count.clear();
        self.buffer_reuse.clear();
        self.vl_copies.clear();
        self.vl_copied_from.clear();
        self.vl_xy.clear();
        self.trackers.clear();
        self.tracker_gen = 0;
        self.tracker_task = false;
        self.tracker_feature = None;
        self.obj_built.clear();
        self.obj_feature.clear();
        self.visual_busy = false;
        self.visual_prep_busy = false;
        self.loc_failure = false;
        self.last_attention = None;
        self.tracked = None;
        self.tracked_lastloc = None;
        self.emma_eye = (0.0, 0.0, 1080.0);
        self.emma_prep_s = false;
        self.emma_exec_s = false;
        self.emma_next_loc = None;
        self.emma_final_shift = (0.0, 0.0, 0.0);
        self.emma_shift_target = None;
        self.emma_shift_start_ms = 0;
        self.emma_shift_duration = 0.0;
        self.emma_encoded = false;
        self.emma_last_prep = None;
        self.man_prep_busy = false;
        self.man_exec_busy = false;
        self.man_proc = [false, false];
        self.man_exec = [false, false];
        self.man_exec_queue.clear();
        self.man_last_prep = None;
        // `reset-hand-tracker` (`motor-extension.lisp:1263`) sets both to `none`.
        self.zz_last_cmd = ["NONE".to_string(), "NONE".to_string()];
        self.zz_last_style = None;
        self.zz_last_move = None;
        self.zz_prepared = None;
        self.zz_breaks.clear();
        self.zz_break_hit = false;
        // `primary-al-reset` (`associative-learning.lisp:364`).
        self.al_chunks.clear();
        self.al_total_count = 0;
        self.al_total_refs.clear();
        self.al_total_creation_time = 0;
        self.al_last_contexts.clear();
        self.zz_mouse_down = false;
        // The virtual mouse cursor's current/device positions
        // (`mouse.lisp:178`/`:192`).
        self.zz_mouse_pos = (0, 0, 1080);
        self.zz_mouse_cursor = (28, 2);
        self.zz_hands = [
            ZzHand {
                loc: (4, 4),
                offsets: LEFT_FINGER_OFFSETS,
                down: [false; 5],
                busy: [false; 5],
                held: [None, None, None, None, None],
            },
            ZzHand {
                loc: (7, 4),
                offsets: RIGHT_FINGER_OFFSETS,
                down: [false; 5],
                busy: [false; 5],
                held: [None, None, None, None, None],
            },
        ];
        // ACT-R re-evaluates the model body on reset (`model.lisp:983`);
        // re-parse `p-fct` productions with the current `*query*`.
        if !self.model.pfct.is_empty() {
            let q = self.cfg.zz_query.clone();
            self.model.reparse_pfct(&q);
        }
        self.imag_busy = false;
        self.temporal_tick = 0.0;
        self.temporal_ticks = 0;
        self.temporal_start = None;
        self.temporal_gen = 0;
        self.event_silent = false;
        self.blending_busy = false;
        self.blending_error = false;
        self.compiled_count = 0;
        self.prev_prod = None;
        self.prev_prod_ms = 0;
        self.last_press = None;
        self.last_speak = None;
        self.touch_hands = [false, false];
        self.touch_fingers = [[(0, 0, 1); 5]; 2];
        self.touch_last = None;
        // Productions are recreated from the model body on reset, so any
        // `pdisable` is forgotten; a model-body `pdisable` is re-applied
        // (`model.lisp:983` re-evaluates the body).
        self.disabled_prods.clear();
        for n in &self.model.disabled_prods {
            if n == "*" {
                for p in &self.model.productions {
                    self.disabled_prods.insert(p.name.to_uppercase());
                }
            } else {
                self.disabled_prods.insert(n.clone());
            }
        }
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
        // Device proc-display already due at t=0 (maintenance).
        if !self.pending_texts.is_empty() && self.pending_texts[0].at_ms <= 0 {
            self.push_m(0, Prio::Num(-100), Out::Always, true, Event::ProcDisplay);
        }
        if self.cfg.threads {
            // The threads extra applies every model-body `goal-focus`.
            for desc in self.model.goal_focuses.clone() {
                self.goal_focus(&desc)?;
            }
        } else if let Some(goal) = self.model.initial_goal.clone() {
            let chunk = new_chunk(&goal)?;
            // Same priority as `goal_focus` so a later driver `(goal-focus …)`
            // (inserted after this at the same time) overwrites it, as ACT-R
            // does (both are `set-buffer-chunk` events at t=0, in call order).
            self.push(
                0,
                Prio::Max,
                Out::Low,
                Event::SetBuffer {
                    buffer: "GOAL".to_string(),
                    chunk,
                    requested: false,
                    shown: None,
                },
            );
            // `goal-focus-fct` also schedules a `clear-delayed-goal`
            // maintenance event (`goal.lisp:284`); it counts toward
            // `run-n-events`.
            self.push_m(0, Prio::Max, Out::Never, true, Event::Dummy);
        }
        // Model-body `(set-buffer-chunk '<buffer> '<chunk>)`: silent initial
        // buffer contents, applied on every reset.
        for (buffer, chunk_name) in self.model.initial_buffer_sets.clone() {
            let inst = self.instantiate(&buffer, NewChunk::Named(chunk_name))?;
            let b = self.buffer_mut(&buffer);
            b.chunk = Some(inst);
            b.failed = false;
            b.modified = false;
        }
        // `(sgp :seed (S C))` in the model body (`random.lisp:417`): ACT-R
        // re-evaluates the model body on reset (`model.lisp:983`), reseeding
        // the RNG. The task then redoes its setup draws (device sidecar),
        // which is why `consume_rng` is applied here rather than once at
        // construction.
        if let Some((s, c)) = self.cfg.seed {
            self.rng.borrow_mut().set_seed(s, c);
        }
        for _ in 0..self.consume_rng {
            self.rng.borrow_mut().next_u32();
        }
        self.push(0, Prio::Min, Out::Medium, Event::ConflictResolution);
        // Model-body `(schedule-event <secs> ...)`: armed relative to model
        // time 0 (params evaluated with no bindings).
        for (at_ms, command, params) in self.model.scheduled_events.clone() {
            let empty = Bindings::new();
            let mut vals = Vec::new();
            for p in &params {
                match self.m_eval(p, &empty) {
                    Ok(v) => vals.push(v),
                    Err(e) => return Err(bad(e)),
                }
            }
            self.push(
                at_ms,
                Prio::Num(0),
                Out::Medium,
                Event::Command {
                    name: command,
                    params: vals,
                },
            );
        }
        Ok(())
    }

    /// Rebuild the spreading fan tables from the model's DM (`add-dm`)
    /// chunks. Mirrors `add-chunk-into-dm`'s fan updates
    /// (`declarative-memory.lisp:1953`): each chunk counts itself in
    /// fan-out, and every slot value that names a chunk adds to that
    /// chunk's fan-out and to the owner's fan-in. Only DM chunks get a
    /// fan-in (spreading candidates come from DM).
    fn build_fans(&mut self) {
        let mut names: HashSet<String> = self.model.chunks.iter().map(|c| c.name.clone()).collect();
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
    fn compute_sji(&mut self, j: &str, i: &str, mas: f32) -> f32 {
        // associative-learning's `:sji-hook` (`compute-learned-sji`) replaces
        // the default Sji when the module is enabled (`:eal`).
        if self.cfg.al_eal.is_some() {
            return self.al_compute_learned_sji(j, i);
        }
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
    fn spreading_activation(&mut self, candidate: &str, mas: Option<f32>) -> f32 {
        let Some(mas) = mas else {
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
            let Some(chunk) = self
                .buffers
                .get(buffer)
                .and_then(|b| b.chunk.as_ref())
                .cloned()
            else {
                continue;
            };
            // `chunk-filled-slots-list-fct` returns the chunk's internal slot
            // list, which is built by prepending (reverse insertion) — except
            // that a bare request spec is itself reversed by
            // `chunk-spec-to-chunk-def`, so the associative-learning imaginal
            // probes come out in written order (the ref's Sji trace order).
            let js: Vec<String> = if self.cfg.al_eal.is_some() && chunk.bare {
                chunk
                    .slots
                    .iter()
                    .filter_map(|(_, v)| self.chunk_name_of(v))
                    .collect()
            } else {
                chunk
                    .slots
                    .iter()
                    .rev()
                    .filter_map(|(_, v)| self.chunk_name_of(v))
                    .collect()
            };
            if js.is_empty() {
                continue;
            }
            if verbose {
                // The goal buffer is a `reuse?` buffer (`buffers.lisp:1349`):
                // its chunk is the reusable copy `GOAL-CHUNK0`. The engine
                // keeps the source chunk so `=goal`/retrieval resolve, so
                // report the copy's name here. The threads extra replaces the
                // goal buffer and keeps source names.
                let cname = if buffer == "GOAL" && !self.cfg.threads {
                    "GOAL-CHUNK0".to_string()
                } else {
                    chunk
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("{buffer}-CHUNK0"))
                };
                self.out(format!(
                    "  Spreading {} from buffer {} chunk {}",
                    lisp_f(spread),
                    buffer,
                    cname
                ));
                self.out(format!("    sources of activation are: ({})", js.join(" ")));
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

    /// `merge-chunks-fct`'s on-the-fly renaming
    /// (`framework/chunks.lisp:1736`): rewrite every slot value that names
    /// the merged buffer chunk (`from`) to the DM chunk it merged into (`to`),
    /// across DM, `define-chunks` chunks, buffer contents, retrieval finsts
    /// and copy-source records. Enabled by `update-chunks-on-the-fly`
    /// (`:ncnar`/`:dcnn`, both `t` by default; `naming-module.lisp:452`).
    fn rename_chunk_refs(&mut self, from: &str, to: &str) {
        fn rewrite(slots: &mut [(String, Value)], from: &str, to: &str) {
            for (_, v) in slots.iter_mut() {
                if let Value::Sym(s) = v {
                    if s == from {
                        *s = to.to_string();
                    }
                }
            }
        }
        for c in self.dm.iter_mut() {
            rewrite(&mut c.slots, from, to);
        }
        for c in self.named.values_mut() {
            rewrite(&mut c.slots, from, to);
        }
        for b in self.buffers.values_mut() {
            if let Some(c) = b.chunk.as_mut() {
                rewrite(&mut c.slots, from, to);
            }
        }
        for (n, _) in self.finsts.iter_mut() {
            if n == from {
                *n = to.to_string();
            }
        }
        for v in self.copy_source.values_mut() {
            if v == from {
                *v = to.to_string();
            }
        }
    }

    /// Merge a cleared buffer chunk into DM by content match, adding a
    /// reference (`merge-reference-list/count`); fresh contents are
    /// added with creation=now, refs=(now), count=1. Buffers in
    /// `:ignore-buffers` skip this; no golden sets it.
    fn merge_into_dm(&mut self, mut chunk: ChunkInst, buffer: &str) {
        let now = self.time_ms;
        // A vision object's `screen-pos` names a buffer copy that is
        // recreated on each attention, so the same object re-encountered
        // later must match an existing DM chunk on its other slots rather
        // than add a new one (verified against `paired`'s DM, which keeps two
        // objects across task 1's two identical trials).
        let no_pos: Vec<(String, Value)> = chunk
            .slots
            .iter()
            .filter(|(k, _)| k != "SCREEN-POS")
            .cloned()
            .collect();
        let found = self.dm.iter().position(|c| {
            c.ctype == chunk.ctype
                && c.slots.len() == chunk.slots.len()
                && c.slots
                    .iter()
                    .filter(|(k, _)| k != "SCREEN-POS")
                    .cloned()
                    .collect::<Vec<_>>()
                    == no_pos
        });
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
                            self.chunk_alias.insert(n.clone(), dm_name.clone());
                            // `merge-chunks-fct` with on-the-fly naming
                            // (`update-chunks-on-the-fly`, on by default:
                            // `:ncnar`/`:dcnn` are `t`) rewrites every slot
                            // value that referenced the merged buffer chunk to
                            // the DM chunk's name
                            // (`framework/chunks.lisp:1736`,
                            // `naming-module.lisp:452`). That is why the
                            // imaginal chunk's `goal =goal` later prints `G1`,
                            // not `CHUNK0` (`sji-merge` sets `:dcnn nil` to opt
                            // out).
                            if self.cfg.chunk_update {
                                self.rename_chunk_refs(&n, &dm_name);
                            }
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
                    // A `define-chunks` chunk modified in a buffer, and a
                    // buffer-copy chunk (`VISUAL-CHUNK0`), get a fresh
                    // `NAME-{k}` copy on merge (`chunk-not-storable` →
                    // `copy-chunk-fct`, `declarative-memory.lisp:1908`).
                    let copy_prefix = format!("{}-CHUNK", buffer.to_uppercase());
                    if self.named.contains_key(&n) || n.starts_with(&copy_prefix) {
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

    /// `check-unstuff-buffer` (`vision.lisp:1944`): the visual-location buffer
    /// still holds the *unchanged*, auto-stuffed copy.
    fn unstuff_valid(&self, copy: &str) -> bool {
        self.buffers.get("VISUAL-LOCATION").map_or(false, |b| {
            !b.modified && b.chunk.as_ref().and_then(|c| c.name.as_deref()) == Some(copy)
        })
    }

    /// `skip-unnecessary-events` (`scheduling.lisp:588`): drop the next
    /// queued event while its precondition no longer holds. Only the vision
    /// unstuff event has one, so a run that modified the location stops
    /// instead of lingering until the unstuff time.
    fn skip_unnecessary_events(&mut self) {
        loop {
            let stale = match self.queue.peek() {
                Some(q) => match &q.event {
                    Event::UnstuffVl { copy } => !self.unstuff_valid(copy),
                    _ => false,
                },
                None => false,
            };
            if stale {
                self.queue.pop();
            } else {
                break;
            }
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
        self.zz_run_step_mode(secs, false).map(|_| ())
    }

    /// `run-full-time` (`framework/scheduling.lisp:986`): run for `secs` of
    /// model time, guaranteed to reach the limit by a hidden `:maintenance`
    /// dummy event at the end (priority `:min`, `:output nil`). An empty
    /// queue therefore stops with "time limit reached", not "no events left".
    pub fn run_full_time(&mut self, secs: f64) -> Result<(), EngineError> {
        let end = self.time_ms + (secs * 1000.0).round() as i64;
        self.push_m(end, Prio::Min, Out::Never, true, Event::Dummy);
        self.zz_run_step_mode(secs, true).map(|_| ())
    }

    /// `run-until-time` (`framework/scheduling.lisp:1030`): run to an absolute
    /// model time (seconds). Its stop test is `next-time > end`, so an event
    /// *at* the end is still processed (`run-full-time` leaves it queued),
    /// while the hidden dummy event at the end keeps the terminator "time
    /// limit reached" rather than "no events left".
    pub fn run_until(&mut self, time: f64) -> Result<(), EngineError> {
        let end_ms = (time * 1000.0).round() as i64;
        let secs = (end_ms - self.time_ms) as f64 / 1000.0;
        if secs > 0.0 {
            self.push_m(end_ms, Prio::Min, Out::Never, true, Event::Dummy);
            self.zz_run_step_mode(secs, false).map(|_| ())
        } else {
            Ok(())
        }
    }

    /// `run-n-events` (`framework/scheduling.lisp:1081`): process `count`
    /// events, then stop ("event limit reached"); an exhausted queue stops
    /// with "no events to process".
    pub fn run_n_events(&mut self, count: u64) -> Result<(), EngineError> {
        let mut n = 0u64;
        loop {
            self.skip_unnecessary_events();
            if self.queue.peek().is_none() {
                self.emit("------", "Stopped because no events to process".into());
                break;
            }
            let next = self.queue.pop().expect("peeked non-empty");
            self.time_ms = next.time_ms;
            let show = self.cfg.detail.shows(next.out);
            let is_cr = matches!(next.event, Event::ConflictResolution);
            let maintenance = next.maintenance;
            self.exec(next.event, show)?;
            self.bold_record();
            n += 1;
            if self.zz_break_hit {
                self.zz_break_hit = false;
                break;
            }
            if self.stopped {
                self.queue.clear();
                self.emit("------", "BREAK-EVENT Stopped by !stop!".into());
                break;
            }
            let silent = self.event_silent;
            self.event_silent = false;
            if !is_cr && !maintenance && !silent {
                self.changed();
            }
            if n >= count {
                self.emit("------", "Stopped because event limit reached".into());
                break;
            }
        }
        Ok(())
    }

    /// Like `run_secs`, but reports whether any event was executed. The
    /// `zz` task driver loops `(run 10)` until it returns 0 (`test-model.lisp`
    /// `test5`): a run that processes nothing is the terminator.
    fn zz_run_step(&mut self, secs: f64) -> Result<bool, EngineError> {
        self.zz_run_step_mode(secs, false)
    }

    /// `full` selects `run-full-time`'s stop: events at exactly the limit
    /// stay queued. The task's window change between runs schedules its
    /// `proc-display` at the boundary *after* the previous run returns
    /// (`paired`), so the engine must not consume it in the earlier run.
    fn zz_run_step_mode(&mut self, secs: f64, full: bool) -> Result<bool, EngineError> {
        let limit_ms = self.time_ms + (secs * 1000.0).round() as i64;
        let mut processed = false;
        loop {
            // `skip-unnecessary-events` runs before the stop check, so a
            // stale maintenance event (failing precondition) doesn't keep
            // the run alive.
            self.skip_unnecessary_events();
            // Peek so an event beyond the limit stays queued for a later
            // `run` (ACT-R bounds each run relative to the current time and
            // leaves future events in place).
            match self.queue.peek().map(|q| q.time_ms) {
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
                Some(t) if (if full { t >= limit_ms } else { t > limit_ms }) => {
                    self.time_ms = limit_ms;
                    self.emit("------", "Stopped because time limit reached".into());
                    break;
                }
                Some(_) => {}
            }
            let next = self.queue.pop().expect("peeked non-empty");
            self.time_ms = next.time_ms;
            let show = self.cfg.detail.shows(next.out);
            let is_cr = matches!(next.event, Event::ConflictResolution);
            let maintenance = next.maintenance;
            self.exec(next.event, show)?;
            self.bold_record();
            processed = true;
            if self.zz_break_hit {
                self.zz_break_hit = false;
                break;
            }
            if self.stopped {
                self.queue.clear();
                self.emit("------", "BREAK-EVENT Stopped by !stop!".into());
                break;
            }
            // Armed conflict-resolution wakes after any later
            // non-maintenance event (`conditions-met`); the arming CR
            // itself does not count.
            let silent = self.event_silent;
            self.event_silent = false;
            if !is_cr && !maintenance && !silent {
                self.changed();
            }
        }
        Ok(processed)
    }
}

/// A composed condition/action slot value: an as-yet-uninstantiated LHS
/// variable (`=s`) or a literal string.
#[derive(Clone, PartialEq, Debug)]
enum SlotVal {
    Var(String),
    Lit(String),
}

/// The RETRIEVAL compilation type's table (`retrieval-compilation.lisp:279`):
/// `T` entries plus the `R-B-C1`/`R-B-C2` checks, which are not modelled and
/// treated as composing.
/// A compilation-type table entry's check (`define-compilation-type`,
/// `production-compilation.lisp:925`): `T` always composes; the named
/// checks are evaluated against the two productions.
#[derive(Clone, Copy, PartialEq)]
enum CompCheck {
    T,
    RbC1,
    RbC2,
}

/// The RETRIEVAL compilation type's table (`retrieval-compilation.lisp:279`).
const RETRIEVAL_COMPOSE: &[(u32, u32, CompCheck)] = &[
    (28, 28, CompCheck::T),
    (28, 24, CompCheck::T),
    (28, 20, CompCheck::RbC1),
    (28, 16, CompCheck::RbC1),
    (28, 12, CompCheck::T),
    (28, 8, CompCheck::T),
    (28, 4, CompCheck::T),
    (28, 0, CompCheck::T),
    (24, 20, CompCheck::RbC2),
    (24, 16, CompCheck::RbC2),
    (24, 4, CompCheck::T),
    (24, 0, CompCheck::T),
    (20, 28, CompCheck::T),
    (20, 24, CompCheck::T),
    (20, 20, CompCheck::RbC1),
    (20, 16, CompCheck::RbC1),
    (20, 12, CompCheck::T),
    (20, 8, CompCheck::T),
    (20, 4, CompCheck::T),
    (20, 0, CompCheck::T),
    (16, 28, CompCheck::RbC2),
    (16, 24, CompCheck::RbC2),
    (16, 20, CompCheck::RbC2),
    (16, 16, CompCheck::RbC2),
    (16, 12, CompCheck::T),
    (16, 8, CompCheck::T),
    (16, 4, CompCheck::T),
    (16, 0, CompCheck::T),
    (12, 28, CompCheck::T),
    (12, 24, CompCheck::T),
    (12, 20, CompCheck::RbC1),
    (12, 16, CompCheck::RbC1),
    (12, 12, CompCheck::T),
    (12, 8, CompCheck::T),
    (12, 4, CompCheck::T),
    (12, 0, CompCheck::T),
    (8, 20, CompCheck::T),
    (8, 16, CompCheck::T),
    (8, 4, CompCheck::T),
    (8, 0, CompCheck::T),
    (4, 28, CompCheck::T),
    (4, 24, CompCheck::T),
    (4, 20, CompCheck::RbC1),
    (4, 16, CompCheck::RbC1),
    (4, 12, CompCheck::T),
    (4, 8, CompCheck::T),
    (4, 4, CompCheck::T),
    (4, 0, CompCheck::T),
    (0, 28, CompCheck::T),
    (0, 24, CompCheck::T),
    (0, 20, CompCheck::T),
    (0, 16, CompCheck::T),
    (0, 12, CompCheck::T),
    (0, 8, CompCheck::T),
    (0, 4, CompCheck::T),
];

/// The GOAL compilation type's table (`goal-compilation.lisp:704`): `T`
/// entries plus the `G-B-C1`/`NO-RHS-GOAL-REF`/`NO-GOAL-HARVESTING*` checks,
/// which are not modelled and treated as composing.
const GOAL_COMPOSE: &[(u32, u32)] = &[
    (44, 40),
    (44, 9),
    (44, 8),
    (44, 0),
    (40, 44),
    (40, 40),
    (40, 13),
    (40, 12),
    (40, 9),
    (40, 8),
    (40, 4),
    (40, 0),
    (13, 40),
    (13, 9),
    (13, 8),
    (13, 0),
    (12, 40),
    (12, 9),
    (12, 8),
    (12, 0),
    (9, 13),
    (9, 12),
    (9, 9),
    (9, 8),
    (9, 4),
    (9, 0),
    (8, 44),
    (8, 40),
    (8, 13),
    (8, 12),
    (8, 9),
    (8, 8),
    (8, 4),
    (8, 0),
    (4, 40),
    (4, 9),
    (4, 8),
    (4, 0),
    (0, 44),
    (0, 40),
    (0, 13),
    (0, 12),
    (0, 9),
    (0, 8),
    (0, 4),
];

/// The PERCEPTUAL compilation type's table
/// (`perceptual-compilation.lisp:290`) for VISUAL-LOCATION, VISUAL,
/// AURAL-LOCATION, AURAL and TEMPORAL; `T` entries plus the `P-B-C*` checks,
/// which are not modelled and treated as composing.
const PERCEPTUAL_COMPOSE: &[(u32, u32)] = &[
    (28, 16),
    (28, 0),
    (24, 28),
    (24, 24),
    (24, 20),
    (24, 16),
    (24, 12),
    (24, 8),
    (24, 4),
    (24, 0),
    (20, 16),
    (20, 0),
    (16, 28),
    (16, 24),
    (16, 20),
    (16, 16),
    (16, 12),
    (16, 8),
    (16, 4),
    (16, 0),
    (12, 16),
    (12, 0),
    (8, 28),
    (8, 24),
    (8, 20),
    (8, 16),
    (8, 12),
    (8, 8),
    (8, 4),
    (8, 0),
    (4, 16),
    (4, 0),
    (0, 28),
    (0, 24),
    (0, 20),
    (0, 16),
    (0, 12),
    (0, 8),
    (0, 4),
];

/// The IMAGINAL compilation type's table (`imaginal-compilation.lisp:802`);
/// `T` entries plus the `I-B-C*` checks, which are not modelled and treated
/// as composing.
const IMAGINAL_COMPOSE: &[(u32, u32)] = &[
    (56, 56),
    (56, 40),
    (56, 25),
    (56, 24),
    (56, 16),
    (56, 9),
    (56, 8),
    (56, 0),
    (40, 56),
    (40, 40),
    (40, 25),
    (40, 24),
    (40, 16),
    (40, 9),
    (40, 8),
    (40, 0),
    (29, 56),
    (29, 40),
    (29, 25),
    (29, 24),
    (29, 16),
    (29, 9),
    (29, 8),
    (29, 0),
    (28, 56),
    (28, 40),
    (28, 25),
    (28, 24),
    (28, 16),
    (28, 9),
    (28, 8),
    (28, 0),
    (25, 29),
    (25, 28),
    (25, 25),
    (25, 24),
    (25, 20),
    (25, 16),
    (25, 13),
    (25, 12),
    (25, 9),
    (25, 8),
    (25, 4),
    (25, 0),
    (24, 56),
    (24, 40),
    (24, 29),
    (24, 28),
    (24, 25),
    (24, 24),
    (24, 20),
    (24, 16),
    (24, 13),
    (24, 12),
    (24, 9),
    (24, 8),
    (24, 4),
    (24, 0),
    (20, 56),
    (20, 40),
    (20, 25),
    (20, 24),
    (20, 16),
    (20, 9),
    (20, 8),
    (20, 0),
    (16, 56),
    (16, 40),
    (16, 29),
    (16, 28),
    (16, 25),
    (16, 24),
    (16, 20),
    (16, 16),
    (16, 13),
    (16, 12),
    (16, 9),
    (16, 8),
    (16, 4),
    (16, 0),
    (13, 56),
    (13, 40),
    (13, 25),
    (13, 24),
    (13, 16),
    (13, 9),
    (13, 8),
    (13, 0),
    (12, 56),
    (12, 40),
    (12, 25),
    (12, 24),
    (12, 16),
    (12, 9),
    (12, 8),
    (12, 0),
    (9, 29),
    (9, 28),
    (9, 25),
    (9, 24),
    (9, 20),
    (9, 16),
    (9, 13),
    (9, 12),
    (9, 9),
    (9, 8),
    (9, 4),
    (9, 0),
    (8, 56),
    (8, 40),
    (8, 29),
    (8, 28),
    (8, 25),
    (8, 24),
    (8, 20),
    (8, 16),
    (8, 13),
    (8, 12),
    (8, 9),
    (8, 8),
    (8, 4),
    (8, 0),
    (4, 56),
    (4, 40),
    (4, 25),
    (4, 24),
    (4, 16),
    (4, 9),
    (4, 8),
    (4, 0),
    (0, 56),
    (0, 40),
    (0, 29),
    (0, 28),
    (0, 25),
    (0, 24),
    (0, 20),
    (0, 16),
    (0, 13),
    (0, 12),
    (0, 9),
    (0, 8),
    (0, 4),
];

/// The MOTOR compilation type's table (`motor-compilation.lisp:145`), which is
/// also the `:default` type for every buffer without its own type (MANUAL,
/// VOCAL, ...); `T` entries plus the `M-B-C*` checks, treated as composing.
const MOTOR_COMPOSE: &[(u32, u32)] = &[
    (20, 16),
    (20, 0),
    (16, 20),
    (16, 16),
    (16, 4),
    (16, 0),
    (4, 16),
    (4, 0),
    (0, 20),
    (0, 16),
    (0, 4),
];

/// `~6,3F` — fixed 3 decimals, minimum width 6 (space padded).
fn fmt_6_3(x: f32) -> String {
    format!("{:>6.3}", x)
}

/// `:visual-movement-tolerance` (`vision.lisp:3466`, default 0.5): how far
/// an object may shift and still count as the same object (`within-move`,
/// `vision.lisp:2367`). The base vision module uses this for the
/// `get-obj-at-location` lookup in `encoding-complete`.
const VISUAL_MOVEMENT_TOLERANCE: f32 = 0.5;

/// `~s` of an optional symbol: `NIL` when absent.
fn lisp_sym_opt(s: &Option<String>) -> String {
    s.clone().unwrap_or_else(|| "NIL".to_string())
}

/// `write-to-string` of a 3-vector of whole numbers: `#(320 522 1080)`.
fn fmt_vec3(v: (f32, f32, f32)) -> String {
    format!("#({} {} {})", v.0 as i64, v.1 as i64, v.2 as i64)
}

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

/// `print-time-in-seconds` (`misc-utils.lisp:1518`): ms as `S.mmm`.
fn fmt_secs(ms: i64) -> String {
    let a = ms.abs();
    format!(
        "{}{}.{:03}",
        if ms < 0 { "-" } else { "" },
        a / 1000,
        a % 1000
    )
}

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
            format!("{digits}{}.0", "0".repeat(point as usize - digits.len()))
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

/// Optimized-learning flag for [`bl_equation`]: off, numeric, or `t`.
#[derive(Debug, Clone, PartialEq)]
enum Ol {
    Off,
    Num(f64),
    On,
}

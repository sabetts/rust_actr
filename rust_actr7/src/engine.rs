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

use std::cell::RefCell;
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
    /// `require-extra "threads"`: the goal buffer is a searchable
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
    pub details: String,
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
struct TouchPlan {    req_ms: i64,
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
        // QWERTY letter rows (`other-files/keyboard.lisp`). Punch-style
        // home keys (A S D F J K L) have no explicit polar geometry; use
        // the home position as a stand-in.
        ("Q".to_string(), KeySpec { hand: "LEFT", finger: "PINKIE", r: 1.0, theta: -1.57 }),
        ("W".to_string(), KeySpec { hand: "LEFT", finger: "RING", r: 1.0, theta: -1.57 }),
        ("E".to_string(), KeySpec { hand: "LEFT", finger: "MIDDLE", r: 1.0, theta: -1.57 }),
        ("R".to_string(), KeySpec { hand: "LEFT", finger: "INDEX", r: 1.0, theta: -1.57 }),
        ("T".to_string(), KeySpec { hand: "LEFT", finger: "INDEX", r: 1.41, theta: -0.79 }),
        ("Y".to_string(), KeySpec { hand: "RIGHT", finger: "INDEX", r: 1.41, theta: -2.36 }),
        ("U".to_string(), KeySpec { hand: "RIGHT", finger: "INDEX", r: 1.0, theta: -1.57 }),
        ("I".to_string(), KeySpec { hand: "RIGHT", finger: "MIDDLE", r: 1.0, theta: -1.57 }),
        ("O".to_string(), KeySpec { hand: "RIGHT", finger: "RING", r: 1.0, theta: -1.57 }),
        ("P".to_string(), KeySpec { hand: "RIGHT", finger: "PINKIE", r: 1.0, theta: -1.57 }),
        ("A".to_string(), KeySpec { hand: "LEFT", finger: "PINKIE", r: 1.0, theta: 0.0 }),
        ("S".to_string(), KeySpec { hand: "LEFT", finger: "RING", r: 1.0, theta: 0.0 }),
        ("D".to_string(), KeySpec { hand: "LEFT", finger: "MIDDLE", r: 1.0, theta: 0.0 }),
        ("F".to_string(), KeySpec { hand: "LEFT", finger: "INDEX", r: 1.0, theta: 0.0 }),
        ("G".to_string(), KeySpec { hand: "LEFT", finger: "INDEX", r: 1.0, theta: 0.0 }),
        ("H".to_string(), KeySpec { hand: "RIGHT", finger: "INDEX", r: 1.0, theta: 3.14 }),
        ("J".to_string(), KeySpec { hand: "RIGHT", finger: "INDEX", r: 1.0, theta: 0.0 }),
        ("K".to_string(), KeySpec { hand: "RIGHT", finger: "MIDDLE", r: 1.0, theta: 0.0 }),
        ("L".to_string(), KeySpec { hand: "RIGHT", finger: "RING", r: 1.0, theta: 0.0 }),
        ("Z".to_string(), KeySpec { hand: "LEFT", finger: "PINKIE", r: 1.0, theta: 1.57 }),
        ("X".to_string(), KeySpec { hand: "LEFT", finger: "RING", r: 1.0, theta: 1.57 }),
        ("C".to_string(), KeySpec { hand: "LEFT", finger: "MIDDLE", r: 1.0, theta: 1.57 }),
        ("V".to_string(), KeySpec { hand: "LEFT", finger: "INDEX", r: 1.0, theta: 1.57 }),
        ("B".to_string(), KeySpec { hand: "LEFT", finger: "INDEX", r: 1.41, theta: 0.79 }),
        ("N".to_string(), KeySpec { hand: "RIGHT", finger: "INDEX", r: 1.41, theta: 2.36 }),
        ("M".to_string(), KeySpec { hand: "RIGHT", finger: "INDEX", r: 1.0, theta: 1.57 }),
        ("SPACE".to_string(), KeySpec { hand: "LEFT", finger: "THUMB", r: 1.0, theta: 0.0 }),
    ])
}

/// Keys whose `key->cmd` mapping is a `punch` movement (`other-files/keyboard.lisp`
/// ASDF row and space): the finger is already home, so no polar geometry is
/// prepared. Everything else is a `peck-recoil` with hand/finger/r/theta.
/// Punch prepares 3 features (hand, finger, and the implicit `+1`), peck 5.
fn key_is_punch(key: &str) -> bool {
    matches!(
        key.to_uppercase().as_str(),
        "A" | "S" | "D" | "F" | "J" | "K" | "L" | ";" | "SPACE"
    )
}

/// A key's absolute `(x, y)` in the virtual keyboard grid
/// (`other-files/keyboard.lisp` `specify-key`).
fn key_xy(key: &str) -> Option<(i64, i64)> {
    let p = match key.to_uppercase().as_str() {
        "`" | "BACKQUOTE" => (0, 2),
        "1" => (1, 2),
        "2" => (2, 2),
        "3" => (3, 2),
        "4" => (4, 2),
        "5" => (5, 2),
        "6" => (6, 2),
        "7" => (7, 2),
        "8" => (8, 2),
        "9" => (9, 2),
        "0" => (10, 2),
        "-" | "HYPHEN" => (11, 2),
        "=" | "EQUAL" => (12, 2),
        "DELETE" => (13, 2),
        "TAB" => (0, 3),
        "Q" => (1, 3),
        "W" => (2, 3),
        "E" => (3, 3),
        "R" => (4, 3),
        "T" => (5, 3),
        "Y" => (6, 3),
        "U" => (7, 3),
        "I" => (8, 3),
        "O" => (9, 3),
        "P" => (10, 3),
        "[" | "LEFT-BRACKET" => (11, 3),
        "]" | "RIGHT-BRACKET" => (12, 3),
        "\\" | "BACKSLASH" => (13, 3),
        "CAPS-LOCK" => (0, 4),
        "A" => (1, 4),
        "S" => (2, 4),
        "D" => (3, 4),
        "F" => (4, 4),
        "G" => (5, 4),
        "H" => (6, 4),
        "J" => (7, 4),
        "K" => (8, 4),
        "L" => (9, 4),
        ";" | "SEMICOLON" => (10, 4),
        "'" | "QUOTE" => (11, 4),
        "RETURN" | "NEWLINE" => (12, 4),
        "SHIFT" | "LEFT-SHIFT" => (0, 5),
        "Z" => (1, 5),
        "X" => (2, 5),
        "C" => (3, 5),
        "V" => (4, 5),
        "B" => (5, 5),
        "N" => (6, 5),
        "M" => (7, 5),
        "," | "COMMA" => (8, 5),
        "." | "PERIOD" | "DOT" => (9, 5),
        "/" | "SLASH" => (10, 5),
        "RIGHT-SHIFT" => (11, 5),
        "SPACE" => (5, 6),
        _ => return None,
    };
    Some(p)
}

/// `key->hand&finger` (`keyboard.lisp` `key->cmd` → the style's hand/finger).
/// A key's canonical lowercase name at absolute `(x, y)` — the inverse of
/// [`key_xy`], used to name the device's `output-key` from the finger position
/// (`motor.lisp:960`).
fn xy_key(x: i64, y: i64) -> Option<&'static str> {
    let s = match (x, y) {
        (1, 2) => "1",
        (2, 2) => "2",
        (3, 2) => "3",
        (4, 2) => "4",
        (5, 2) => "5",
        (6, 2) => "6",
        (7, 2) => "7",
        (8, 2) => "8",
        (9, 2) => "9",
        (10, 2) => "0",
        (1, 3) => "q",
        (2, 3) => "w",
        (3, 3) => "e",
        (4, 3) => "r",
        (5, 3) => "t",
        (6, 3) => "y",
        (7, 3) => "u",
        (8, 3) => "i",
        (9, 3) => "o",
        (10, 3) => "p",
        (1, 4) => "a",
        (2, 4) => "s",
        (3, 4) => "d",
        (4, 4) => "f",
        (5, 4) => "g",
        (6, 4) => "h",
        (7, 4) => "j",
        (8, 4) => "k",
        (9, 4) => "l",
        (1, 5) => "z",
        (2, 5) => "x",
        (3, 5) => "c",
        (4, 5) => "v",
        (5, 5) => "b",
        (6, 5) => "n",
        (7, 5) => "m",
        _ => return None,
    };
    Some(s)
}

fn key_hand_finger(key: &str) -> Option<(usize, usize)> {
    let (hand, finger) = match key.to_uppercase().as_str() {
        "`" | "BACKQUOTE" | "1" | "Q" | "A" | "Z" | "TAB" | "CAPS-LOCK" | "SHIFT"
        | "LEFT-SHIFT" => (0usize, 3usize),
        "2" | "W" | "S" | "X" => (0, 2),
        "3" | "E" | "D" | "C" => (0, 1),
        "4" | "R" | "F" | "V" | "5" | "T" | "G" | "B" => (0, 0),
        "6" | "Y" | "H" | "N" => (1, 0),
        "7" | "U" | "J" | "M" => (1, 0),
        "8" | "I" | "K" | "," | "COMMA" => (1, 1),
        "9" | "O" | "L" | "." | "PERIOD" | "DOT" => (1, 2),
        "0" | "P" | ";" | "SEMICOLON" | "/" | "SLASH" | "-" | "HYPHEN" | "="
        | "EQUAL" | "[" | "LEFT-BRACKET" | "]" | "RIGHT-BRACKET" | "\\"
        | "BACKSLASH" | "'" | "QUOTE" | "DELETE" | "RETURN" | "NEWLINE"
        | "RIGHT-SHIFT" => (1, 3),
        "SPACE" => (0, 4),
        _ => return None,
    };
    Some((hand, finger))
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
            Some((_, Sexp::List(v)))
                if v.len() == 2 && v[0].as_sym() == Some("QUOTE") =>
            {
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
        let unstuff_vl_ms = match m.params.iter().find(|(k, _)| k == ":UNSTUFF-VISUAL-LOCATION") {
            None => Some(onset_span_ms),
            Some((_, Sexp::Sym(s))) if s == "NIL" => None,
            Some((_, Sexp::Sym(_))) => Some(onset_span_ms),
            Some((_, Sexp::Float(x))) => Some((x * 1000.0).round() as i64),
            Some((_, Sexp::Int(i))) => Some(i * 1000),
            Some((_, got)) => {
                return Err(bad(format!("bad :unstuff-visual-location {got:?}")))
            }
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
            finst_span_ms: (sgp_num(&m.params, ":DECLARATIVE-FINST-SPAN", 3.0)? * 1000.0)
                .round() as i64,
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
    /// Set by `mod-buffer-chunk` (a production's `=buffer>` RHS): the vision
    /// module's `check-unstuff-buffer` only unstuffs an *unmodified* copy.
    modified: bool,
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
    ZzMousePrep {        cmd: String,
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
    /// Auto-naming counters for `define-chunks`-style names (`CHUNK0`, ...).
    name_count: HashMap<String, u64>,
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
            end,
            rewards,
            sims,
            clear_on_keypress,
            consume_rng,
        } = device;
        let base_prods = model.productions.len();
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
            device_run: 0,
            consume_rng,
            features: Vec::new(),
            feat_seq: 0,
            copy_count: HashMap::new(),
            c_fan_out: HashMap::new(),
            chunk_alias: HashMap::new(),
            copy_source: HashMap::new(),
            stopped: false,
            finsts: Vec::new(),
            goal_set: Vec::new(),
            vocal_busy: false,
            audio_events: 0,
            name_count: HashMap::new(),
            vl_copies: HashMap::new(),
            vl_copied_from: HashMap::new(),
            obj_built: HashMap::new(),
            obj_feature: HashMap::new(),
            visual_busy: false,
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
            "FREE", "BUSY", "ERROR", "EMPTY", "FULL", "FAILURE", "REQUESTED",
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
                    module: "NONE".to_string(),
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
        self.name_count.clear();
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
            let Some(chunk) = self.buffers.get(buffer).and_then(|b| b.chunk.as_ref()).cloned() else {
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
            !b.modified
                && b.chunk.as_ref().and_then(|c| c.name.as_deref()) == Some(copy)
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
        if self.cfg.threads {
            let inst = match desc {
                ChunkDesc::Named(n) => {
                    // Copy the named chunk (`START-GOAL` -> `START-GOAL-0`).
                    let copy = self.copy_chunk(n);
                    self.named
                        .get(&copy)
                        .cloned()
                        .or_else(|| {
                            self.dm
                                .iter()
                                .find(|c| c.name.as_deref() == Some(copy.as_str()))
                                .cloned()
                        })
                        .ok_or_else(|| bad(format!("no chunk named {n}")))?
                }
                ChunkDesc::Spec { .. } => {
                    let nc = new_chunk(desc)?;
                    let mut c = self.instantiate("GOAL", nc)?;
                    // `define-chunks-fct` auto-names a spec by its type
                    // (`(isa initial ...)` -> `INITIAL0`).
                    let ctype = c.ctype.clone();
                    let k = *self.name_count.entry(ctype.clone()).or_insert(0);
                    self.name_count.insert(ctype.clone(), k + 1);
                    c.name = Some(format!("{ctype}{k}"));
                    c
                }
            };
            let first = self.goal_set.is_empty();
            self.goal_set.push(inst.clone());
            if first {
                self.now(
                    Prio::Max,
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "GOAL".to_string(),
                        chunk: NewChunk::Instance(inst),
                        requested: false,
                        shown: None,
                    },
                );
            } else {
                self.now(
                    Prio::Max,
                    Out::Low,
                    Event::OverwriteBuffer {
                        buffer: "GOAL".to_string(),
                        chunk: NewChunk::Instance(inst),
                        requested: false,
                        shown: None,
                        module: "GOAL".to_string(),
                    },
                );
            }
            return Ok(());
        }
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

    /// `summarize-tests`/`display-data` (`test-model.lisp:84`): print one
    /// `#####` block per recorded `:test` marker.
    fn zz_summarize(&mut self) {
        let data = std::mem::take(&mut self.zz_data);
        let mut i = 0;
        while i < data.len() {
            if data[i].0 == "TEST" {
                let details = data[i].2.clone();
                let mut j = i + 1;
                while j < data.len() && data[j].0 != "TEST" {
                    j += 1;
                }
                let events: Vec<(String, i64, String)> = data[i + 1..j].to_vec();
                self.zz_display_data(&details, &events);
                i = j;
            } else {
                i += 1;
            }
        }
    }

    /// `display-data` (`test-model.lisp:91`): the `#####` summary block.
    fn zz_display_data(&mut self, details: &str, events: &[(String, i64, String)]) {
        self.out_lines.push("#####".to_string());
        self.out_lines.push(format!("\"{details}\""));
        self.out_lines.push("#####".to_string());
        for action in ["PRESSED", "RELEASED", "CLICKED", "UNCLICKED"] {
            let mut line = format!(" {action}:");
            let mut last: Option<i64> = None;
            let entries: Vec<(i64, String)> = events
                .iter()
                .filter(|(k, _, _)| k == action)
                .map(|(_, t, key)| (*t, key.clone()))
                .collect();
            for (t, key) in entries {
                let delta = last.map(|l| (t - l) as f64 / 1000.0);
                // Key names are strings (`"r"`); mouse positions are lists
                // (`(10 20 1080)`) printed without quotes.
                let key_str = if action == "CLICKED" || action == "UNCLICKED" {
                    key
                } else {
                    format!("{key:?}")
                };
                line.push_str(&format!(
                    "\n   {:>7.3} {key_str:<10} {}",
                    t as f64 / 1000.0,
                    delta.map(|d| format!("({d:.3})")).unwrap_or_default()
                ));
                last = Some(t);
            }
            self.out_lines.push(line);
        }
        self.out_lines.push(" Held:".to_string());
        let presses: Vec<(i64, String)> = events
            .iter()
            .filter(|(k, _, _)| k == "PRESSED")
            .map(|(_, t, key)| (*t, key.clone()))
            .collect();
        for (t, key) in presses {
            let release = events
                .iter()
                .find(|(k, t2, k2)| k == "RELEASED" && k2 == &key && *t2 >= t);
            let held = release.map(|(_, t2, _)| (*t2 - t) as f64 / 1000.0);
            self.out_lines.push(format!(
                "  {:<10} {}",
                format!("{key:?}"),
                held.map(|h| format!("{h:.3}")).unwrap_or_default()
            ));
        }
        self.out_lines.push(" Clicked:".to_string());
        let clicks: Vec<(i64, String)> = events
            .iter()
            .filter(|(k, _, _)| k == "CLICKED")
            .map(|(_, t, key)| (*t, key.clone()))
            .collect();
        for (t, key) in clicks {
            let release = events
                .iter()
                .find(|(k, t2, _)| k == "UNCLICKED" && *t2 >= t);
            if let Some((_, t2, pos)) = release {
                let delta = (*t2 - t) as f64 / 1000.0;
                self.out_lines
                    .push(format!("  {key} {pos} {delta:.3}"));
            }
        }
    }

    /// `zz-extended-motor-actions` task driver (`test-model.lisp:65`):
    /// reproduce `run-test`/`testN` (the extractor only records the
    /// `run`/`reset` calls).
    pub fn zz_driver(&mut self) -> Result<(), EngineError> {
        self.zz_test1()?;
        self.zz_test2()?;
        self.zz_test3()?;
        self.zz_test4()?;
        self.zz_test5()?;
        self.zz_test6()?;
        self.zz_test7()?;
        self.zz_test8()?;
        self.zz_test9()?;
        Ok(())
    }

    // ------------------------------------------------ associative-learning

    /// `associative-learning` task driver: `run-it`
    /// (`test-models/associative-learning.lisp`) →
    /// `(approach-1-results 1 .75 :prior-p .08 :prior-w 2.5 :show t)`.
    ///
    /// `create-stims` runs before `reset`, so it consumes the stream left by
    /// the model's initial reset; `reset` re-applies the model's
    /// `:starting-parameters (:seed (1 0) …)` (`central-parameters.lisp:131`),
    /// so `initial-presentation` starts from a fresh `(1 0)` stream.
    pub fn al_driver(&mut self) -> Result<(), EngineError> {
        // The model's `define-model` reset applied the `:starting-parameters`
        // seed, so `create-stims` starts from a fresh `(1 0)` stream.
        self.rng.borrow_mut().set_seed(1, 0);
        self.al_create_stims();
        self.reset()?;
        self.rng.borrow_mut().set_seed(1, 0);
        // `(set-parameter-value …)`: :lf .75, :al-show t, :ol t, :al-prior-p
        // .08, :al-prior-w 2.5 (`approach-1-results`).
        self.cfg.lf_ms = 750;
        self.cfg.al_show = true;
        self.cfg.ol = Ol::On;
        self.cfg.al_prior_p = 0.08;
        self.cfg.al_prior_w = 2.5;
        self.al_create_initial_chunks();
        self.al_initial_presentation()?;
        self.al_drop_out()?;
        self.al_drop_out()?;
        // `time-to-retrieve-at-start` + `output-person-location`
        // (`al-fan-study.lisp:382`/`:467`).
        let data = self.al_time_to_retrieve();
        self.al_output_person_location(&data);
        Ok(())
    }

    /// `permute-list` (`random.lisp:562`): each pick is consed onto the front,
    /// so the result is the reverse of the pick order.
    fn al_permute<T: Clone>(&mut self, items: &[T]) -> Vec<T> {
        let mut temp: Vec<T> = items.to_vec();
        let mut result: Vec<T> = Vec::new();
        while !temp.is_empty() {
            let i = self.rng.borrow_mut().random_int(temp.len() as u64) as usize;
            let item = temp.remove(i);
            result.insert(0, item);
        }
        result
    }

    /// The stimulus word lists (`al-fan-study.lisp:220`), before permutation.
    fn al_people_list() -> [&'static str; 16] {
        [
            "hippie", "guard", "earl", "giant", "captain", "debutante", "fireman",
            "lawyer", "wizard", "clerk", "writer", "doctor", "teacher", "boxer",
            "queen", "singer",
        ]
    }

    fn al_places_list() -> [&'static str; 16] {
        [
            "beach", "castle", "dungeon", "forest", "park", "church", "cave", "bank",
            "store", "tower", "lake", "stadium", "casino", "school", "diner", "office",
        ]
    }

    /// `create-stims` (`al-fan-study.lisp:245`): permute people/places, build
    /// the sentence/target/foil index lists, then the filler permutes (kept so
    /// the RNG stream stays aligned even though the fillers are unused).
    fn al_create_stims(&mut self) {
        let people: Vec<String> = Self::al_people_list()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let places: Vec<String> = Self::al_places_list()
            .iter()
            .map(|s| s.to_string())
            .collect();
        self.al_people = self.al_permute(&people);
        self.al_places = self.al_permute(&places);
        let s_idx: [(usize, usize, i64, i64); 26] = [
            (0, 0, 1, 1), (1, 1, 1, 1), (2, 2, 1, 1),
            (3, 3, 2, 1), (4, 4, 2, 1), (5, 5, 2, 1),
            (6, 6, 3, 1), (7, 7, 3, 1), (8, 8, 3, 1),
            (9, 9, 1, 2), (10, 10, 1, 2), (11, 11, 1, 2),
            (12, 12, 1, 3), (13, 13, 1, 3), (14, 14, 1, 3),
            (4, 10, 2, 2), (15, 15, 2, 2),
            (6, 9, 3, 2), (7, 15, 3, 2), (8, 11, 3, 2),
            (3, 12, 2, 3), (15, 13, 2, 3), (5, 14, 2, 3),
            (6, 12, 3, 3), (7, 13, 3, 3), (8, 14, 3, 3),
        ];
        let t_idx: [(usize, usize, i64, i64); 9] = [
            (0, 0, 1, 1), (9, 9, 1, 2), (12, 12, 1, 3),
            (3, 3, 2, 1), (4, 10, 2, 2), (15, 13, 2, 3),
            (6, 6, 3, 1), (7, 15, 3, 2), (8, 14, 3, 3),
        ];
        let f_idx: [(usize, usize, i64, i64); 18] = [
            (0, 3, 1, 1), (9, 10, 1, 2), (12, 13, 1, 3),
            (3, 0, 2, 1), (4, 9, 2, 2), (15, 12, 2, 3),
            (6, 0, 3, 1), (7, 9, 3, 2), (8, 12, 3, 3),
            (0, 6, 1, 1), (9, 15, 1, 2), (12, 14, 1, 3),
            (3, 6, 2, 1), (4, 15, 2, 2), (15, 14, 2, 3),
            (6, 3, 3, 1), (7, 10, 3, 2), (8, 13, 3, 3),
        ];
        let build = |idx: &[(usize, usize, i64, i64)],
                     people: &[String],
                     places: &[String]| -> Vec<(String, String, i64, i64)> {
            idx.iter()
                .map(|(a, b, pf, lf)| (people[*a].clone(), places[*b].clone(), *pf, *lf))
                .collect()
        };
        self.al_sentences = build(&s_idx, &self.al_people, &self.al_places);
        self.al_targets = build(&t_idx, &self.al_people, &self.al_places);
        self.al_foils = build(&f_idx, &self.al_people, &self.al_places);
        self.al_fillers.clear();
        for _ in 0..6 {
            let op = self.al_permute(&[1usize, 2, 5, 10, 11, 13, 14]);
            let pl = self.al_permute(&[1usize, 2, 4, 5, 7, 8, 11]);
            for (x, y) in op.iter().zip(pl.iter()) {
                self.al_fillers.push((
                    self.al_people[*x].clone(),
                    self.al_places[*y].clone(),
                    -1,
                    -1,
                ));
            }
        }
    }

    /// `create-initial-chunks` (`al-fan-study.lisp:327`): a named `meaning`
    /// chunk for each person/place plus `in`.
    fn al_create_initial_chunks(&mut self) {
        let words: Vec<String> = self
            .al_people
            .iter()
            .chain(self.al_places.iter())
            .cloned()
            .chain(std::iter::once("in".to_string()))
            .collect();
        for w in &words {
            self.al_add_meaning(w);
        }
    }

    fn al_add_meaning(&mut self, word: &str) {
        let name = word.to_uppercase();
        if self.dm.iter().any(|c| c.name.as_deref() == Some(name.as_str())) {
            return;
        }
        let inst = ChunkInst {
            name: Some(name.clone()),
            ctype: "MEANING".to_string(),
            slots: vec![("WORD".to_string(), Value::Str(word.to_string()))],
            bare: false,
        };
        self.dm.push(inst.clone());
        self.hist.push(History {
            creation_ms: 0,
            refs: vec![0],
            count: 1,
            fixed_base: None,
        });
        self.named.insert(name.clone(), inst);
        self.chunk_names.insert(name.clone());
        *self.fan_out.entry(name).or_insert(0) += 1;
    }

    /// `initial-presentation` (`al-fan-study.lisp:338`): present a random
    /// ordering of the study sentences, 10 s each.
    fn al_initial_presentation(&mut self) -> Result<(), EngineError> {
        let sentences = self.al_sentences.clone();
        let presented = self.al_permute(&sentences);
        for (person, place, _pf, _lf) in &presented {
            self.al_imaginal_request(person, place);
            // `(schedule-clear-buffer 'imaginal 1.5)`.
            self.push(
                self.time_ms + 1500,
                Prio::Num(0),
                Out::Low,
                Event::ClearBuffer {
                    buffer: "IMAGINAL".to_string(),
                    module: "NONE".to_string(),
                },
            );
            self.run_secs(10.0)?;
        }
        Ok(())
    }

    /// `(module-request 'imaginal (define-chunk-spec-fct …))` with the
    /// `(relation in arg1 X arg2 Y)` spec.
    fn al_imaginal_request(&mut self, arg1: &str, arg2: &str) {
        let spec = NewChunk::Spec {
            ctype: String::new(),
            slots: vec![
                ("RELATION".to_string(), Value::Sym("IN".to_string())),
                ("ARG1".to_string(), Value::Sym(arg1.to_uppercase())),
                ("ARG2".to_string(), Value::Sym(arg2.to_uppercase())),
            ],
            neg: Vec::new(),
            rels: Vec::new(),
            bare: true,
        };
        self.now(
            Prio::Num(0),
            Out::Medium,
            Event::ModuleRequest {
                buffer: "IMAGINAL".to_string(),
                spec,
                var_slots: Vec::new(),
                trace: false,
            },
        );
    }

    /// `(module-request 'imaginal (relation in arg1 X))`: the drop-out probe
    /// (no `arg2`).
    fn al_imaginal_probe(&mut self, arg1: &str) {
        let spec = NewChunk::Spec {
            ctype: String::new(),
            slots: vec![
                ("RELATION".to_string(), Value::Sym("IN".to_string())),
                ("ARG1".to_string(), Value::Sym(arg1.to_uppercase())),
            ],
            neg: Vec::new(),
            rels: Vec::new(),
            bare: true,
        };
        self.now(
            Prio::Num(0),
            Out::Medium,
            Event::ModuleRequest {
                buffer: "IMAGINAL".to_string(),
                spec,
                var_slots: Vec::new(),
                trace: false,
            },
        );
    }

    /// `(module-request 'retrieval (relation in arg1 X arg2 Y))`.
    fn al_retrieval_request(&mut self, arg1: &str, arg2: &str) {
        let spec = NewChunk::Spec {
            ctype: String::new(),
            slots: vec![
                ("RELATION".to_string(), Value::Sym("IN".to_string())),
                ("ARG1".to_string(), Value::Sym(arg1.to_uppercase())),
                ("ARG2".to_string(), Value::Sym(arg2.to_uppercase())),
            ],
            neg: Vec::new(),
            rels: Vec::new(),
            bare: true,
        };
        self.now(
            Prio::Num(0),
            Out::Medium,
            Event::ModuleRequest {
                buffer: "RETRIEVAL".to_string(),
                spec,
                var_slots: Vec::new(),
                trace: false,
            },
        );
    }

    /// `(run-full-time secs)` (`scheduling.lisp:986`): schedule a
    /// `:min`-priority maintenance dummy at the limit so the run always
    /// reaches it (`Stopped because time limit reached`).
    fn al_run_full_time(&mut self, secs: f64) -> Result<(), EngineError> {
        let limit = self.time_ms + (secs * 1000.0).round() as i64;
        self.push_m(limit, Prio::Min, Out::Never, true, Event::Dummy);
        self.run_secs(secs)
    }

    /// `drop-out` (`al-fan-study.lisp:349`): present each probe once, then
    /// retrieve every sentence it appeared in, 20 s each.
    fn al_drop_out(&mut self) -> Result<(), EngineError> {
        // `(set-parameter-value :lf 0)` for the study phase; restored after.
        let lf = self.cfg.lf_ms;
        self.cfg.lf_ms = 0;
        // `(permute-list (append (copy-list *people*) (copy-list *places*)))`:
        // the *original* lists, not the `create-stims` permutations.
        let mut all: Vec<String> = Self::al_people_list()
            .iter()
            .map(|s| s.to_string())
            .collect();
        all.extend(Self::al_places_list().iter().map(|s| s.to_string()));
        let order = self.al_permute(&all);
        for x in &order {
            // `(erase-buffer 'imaginal)`: silent, and the removed chunk does
            // not merge into DM.
            self.buffer_mut("IMAGINAL").chunk = None;
            self.al_imaginal_probe(x);
            self.run_secs(0.2)?;
            let matching: Vec<(String, String)> = self
                .al_sentences
                .iter()
                .filter(|(p, pl, _, _)| p == x || pl == x)
                .map(|(p, pl, _, _)| (p.clone(), pl.clone()))
                .collect();
            for (p, pl) in matching {
                self.al_retrieval_request(&p, &pl);
                self.al_run_full_time(20.0)?;
            }
        }
        self.cfg.lf_ms = lf;
        Ok(())
    }

    /// Whether `buffer` is a source buffer for associative learning: a
    /// non-zero spread, not retrieval (`finish-al-reset`,
    /// `associative-learning.lisp:376`).
    fn al_is_source_buffer(&self, buffer: &str) -> bool {
        match buffer {
            "GOAL" => self.cfg.ga != 0.0,
            "IMAGINAL" => self.cfg.imaginal_activation != 0.0,
            _ => false,
        }
    }

    /// `source-buffer-chunks` (`associative-learning.lisp:633`).
    fn al_source_buffer_chunks(&self) -> Vec<String> {
        let mut r = Vec::new();
        for buffer in ["GOAL", "IMAGINAL"] {
            if !self.al_is_source_buffer(buffer) {
                continue;
            }
            if let Some(chunk) = self.buffers.get(buffer).and_then(|b| b.chunk.as_ref()) {
                for (_, v) in &chunk.slots {
                    if let Some(name) = self.chunk_name_of(v) {
                        r.push(name);
                    }
                }
            }
        }
        r
    }

    /// The chunk-valued slots of the DM chunk named `name` (the `self-sources`
    /// of `al-buffer-cleared`, `associative-learning.lisp:673`).
    fn al_chunk_sources(&self, name: &str) -> Vec<String> {
        self.dm
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .map(|c| {
                c.slots
                    .iter()
                    .filter_map(|(_, v)| self.chunk_name_of(v))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `chunk-creation-time` for the DM chunk named `name`.
    fn dm_creation_time(&self, name: &str) -> i64 {
        self.dm
            .iter()
            .position(|c| c.name.as_deref() == Some(name))
            .map(|i| self.hist[i].creation_ms)
            .unwrap_or(0)
    }

    fn al_ni_cj_count(&self, j: &str, i: &str) -> i64 {
        self.al_chunks
            .get(j)
            .and_then(|c| c.ni_cj_count.iter().find(|(n, _)| n == i))
            .map(|(_, v)| *v)
            .unwrap_or(0)
    }

    fn al_ni_cj_refs(&self, j: &str, i: &str) -> Vec<i64> {
        self.al_chunks
            .get(j)
            .and_then(|c| c.ni_cj_refs.iter().find(|(n, _)| n == i))
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }

    /// `(model-output ...)` from the associative-learning module: a raw line
    /// with no timestamp/module prefix.
    fn al_out(&mut self, line: String) {
        self.out_lines.push(line);
    }

    /// `update-al-parameters` (`associative-learning.lisp:712`).
    fn al_update(&mut self, i: &str, js: &[String]) {
        let ct = self.time_ms;
        let ol = self.cfg.ol.clone();
        self.al_total_count += 1;
        al_needed_refs(&ol, &mut self.al_total_refs, ct);
        {
            let c = self.al_chunks.entry(i.to_string()).or_default();
            c.ni_count += 1;
            al_needed_refs(&ol, &mut c.ni_refs, ct);
        }
        for j in js {
            // `(pushnew j (chunk-associated-js i))`.
            {
                let ci = self.al_chunks.entry(i.to_string()).or_default();
                if !ci.associated_js.iter().any(|x| x == j) {
                    ci.associated_js.insert(0, j.clone());
                }
            }
            let cj = self.al_chunks.entry(j.clone()).or_default();
            cj.cj_count += 1;
            al_needed_refs(&ol, &mut cj.cj_refs, ct);
            match cj.ni_cj_count.iter_mut().find(|(n, _)| n == i) {
                Some(e) => e.1 += 1,
                None => cj.ni_cj_count.insert(0, (i.to_string(), 1)),
            }
            match cj.ni_cj_refs.iter_mut().find(|(n, _)| n == i) {
                Some(e) => al_needed_refs(&ol, &mut e.1, ct),
                None => {
                    let mut r = Vec::new();
                    al_needed_refs(&ol, &mut r, ct);
                    cj.ni_cj_refs.insert(0, (i.to_string(), r));
                }
            }
        }
    }

    /// `compute-learned-sji` (`associative-learning.lisp:288`): the learned Sji
    /// between needed item `i` and context chunk `j`, printing the module's
    /// detailed trace when `:al-show`/`:act` is on.
    fn al_compute_learned_sji(&mut self, j: &str, i: &str) -> f32 {
        let Some(eal) = self.cfg.al_eal else {
            return 0.0;
        };
        let p = self.cfg.al_prior_p;
        let w = self.cfg.al_prior_w;
        let decay = eal;
        let scale = 0.001f32.powf(-eal).ln();
        let now = self.time_ms;
        let ol = self.cfg.ol.clone();

        let nicj_count = self.al_ni_cj_count(j, i);
        let nicj_refs = self.al_ni_cj_refs(j, i);
        let (ni_count, ni_refs) = self
            .al_chunks
            .get(i)
            .map(|c| (c.ni_count, c.ni_refs.clone()))
            .unwrap_or((0, Vec::new()));
        let not_ni_refs: Vec<i64> = self
            .al_total_refs
            .iter()
            .filter(|r| !ni_refs.contains(r))
            .cloned()
            .collect();
        let (cj_count, cj_refs) = self
            .al_chunks
            .get(j)
            .map(|c| (c.cj_count, c.cj_refs.clone()))
            .unwrap_or((0, Vec::new()));
        let not_nicj_refs: Vec<i64> = cj_refs
            .iter()
            .filter(|r| !nicj_refs.contains(r))
            .cloned()
            .collect();
        let creation_i = self.dm_creation_time(i);
        let creation_j = self.dm_creation_time(j);
        let total = self.al_total_count;
        let total_creation = self.al_total_creation_time;

        let f_not_ni = if total - ni_count == 0 {
            0.0
        } else {
            let ol2 = if not_ni_refs.is_empty() {
                Ol::On
            } else {
                ol.clone()
            };
            exp_coerced(bl_equation(
                now,
                &not_ni_refs,
                (total - ni_count) as f32,
                total_creation,
                decay,
                &ol2,
                scale,
            ))
        };
        let f_ni = if ni_count == 0 {
            0.0
        } else {
            exp_coerced(bl_equation(
                now,
                &ni_refs,
                ni_count as f32,
                creation_i,
                decay,
                &ol,
                scale,
            ))
        };
        let f_not_nicj = if cj_count - nicj_count == 0 {
            0.0
        } else {
            let ol2 = if not_nicj_refs.is_empty() {
                Ol::On
            } else {
                ol.clone()
            };
            exp_coerced(bl_equation(
                now,
                &not_nicj_refs,
                (cj_count - nicj_count) as f32,
                creation_j,
                decay,
                &ol2,
                scale,
            ))
        };
        let f_nicj = if nicj_count == 0 {
            0.0
        } else {
            exp_coerced(bl_equation(
                now,
                &nicj_refs,
                nicj_count as f32,
                creation_i,
                decay,
                &ol,
                scale,
            ))
        };
        let pcjni = (p * w + f_nicj) / (w + f_ni);
        let pcj_not_ni = (p * w + f_not_nicj) / (w + f_not_ni);
        let mut sji = log_coerced(pcjni / pcj_not_ni);

        if self.cfg.al_show || self.cfg.act.is_some() {
            self.al_out(format!(
                "     Computing Sji from {} to {} with the decay set to {} prior probability {} and prior weight {}",
                j,
                i,
                lisp_f(decay),
                lisp_f(p),
                lisp_f(w)
            ));
            self.al_out(format!("      F(Ni&Cj) = {}", lisp_f(f_nicj)));
            if nicj_count != 0 {
                self.al_out(format!(
                    "      {} was needed while {} was in the context {} times with history {}",
                    i,
                    j,
                    nicj_count,
                    al_history(&nicj_refs)
                ));
            } else {
                self.al_out(format!(
                    "      {} was needed while {} was in the context 0 times with history nil",
                    i, j
                ));
            }
            self.al_out(format!("      F(Ni) = {}", lisp_f(f_ni)));
            self.al_out(format!(
                "        {} was needed {} times with history {}",
                i,
                ni_count,
                al_history(&ni_refs)
            ));
            self.al_out(format!("      F(~Ni&Cj) = {}", lisp_f(f_not_nicj)));
            self.al_out(format!(
                "        Retrieval of something other than {} while {} was in the context {} times with history {}",
                i,
                j,
                cj_count - nicj_count,
                al_history(&not_nicj_refs)
            ));
            self.al_out(format!("      F(~Ni) = {}", lisp_f(f_not_ni)));
            self.al_out(format!(
                "        Retrieval of something other than {} happened {} times with history {}",
                i,
                total - ni_count,
                al_history(&not_ni_refs)
            ));
            self.al_out(format!("      P(Cj|Ni) = {}", lisp_f(pcjni)));
            self.al_out(format!("      P(Cj|~Ni) = {}", lisp_f(pcj_not_ni)));
            self.al_out(format!("      Sji = {}", lisp_f(sji)));
        }

        // `:nsji`: `nil` (the default) clamps negatives to 0; `t` keeps them.
        if sji < 0.0 && !self.cfg.nsji {
            sji = 0.0;
        }
        sji
    }

    /// `time-to-retrieve-at-start` (`al-fan-study.lisp:382`): for each target
    /// and foil, compute the activation of every matching chunk (which prints
    /// the Sji traces) and convert it to a retrieval time; returns the 18
    /// averaged cells.
    fn al_time_to_retrieve(&mut self) -> Vec<f64> {
        // `(set-buffer-chunk 'imaginal (car (define-chunks (isa
        // comprehend-sentence relation in))))`. The slot order matches the
        // ref's source order in the Sji trace (arg1 arg2 relation).
        let ctx = ChunkInst {
            name: Some("COMPREHEND-SENTENCE0".to_string()),
            ctype: "COMPREHEND-SENTENCE".to_string(),
            slots: vec![
                ("RELATION".to_string(), Value::Sym("IN".to_string())),
                ("ARG1".to_string(), Value::Nil),
                ("ARG2".to_string(), Value::Nil),
            ],
            bare: false,
        };
        self.buffer_mut("IMAGINAL").chunk = Some(ctx);
        let lf = self.cfg.lf_ms as f64 / 1000.0;
        let mut data: Vec<Vec<Vec<Vec<f64>>>> = vec![vec![vec![Vec::new(); 2]; 3]; 3];
        let mut items: Vec<(String, String, usize, usize, bool)> = Vec::new();
        for (p, pl, pf, lf) in &self.al_targets {
            items.push((p.clone(), pl.clone(), *pf as usize, *lf as usize, true));
        }
        for (p, pl, pf, lf) in &self.al_foils {
            items.push((p.clone(), pl.clone(), *pf as usize, *lf as usize, false));
        }
        for (person, place, pf, lfan, is_target) in &items {
            if let Some(c) = self.buffers.get_mut("IMAGINAL").and_then(|b| b.chunk.as_mut()) {
                for (s, v) in c.slots.iter_mut() {
                    if s.as_str() == "ARG1" {
                        *v = Value::Sym(person.to_uppercase());
                    } else if s.as_str() == "ARG2" {
                        *v = Value::Sym(place.to_uppercase());
                    }
                }
            }
            let ap = self.al_sdm_sdp(Some(person), None);
            let al = self.al_sdm_sdp(None, Some(place));
            let act_p = ap.iter().cloned().fold(f64::MIN, f64::max);
            let act_l = al.iter().cloned().fold(f64::MIN, f64::max);
            let idx = if *is_target { 0 } else { 1 };
            data[pf - 1][lfan - 1][idx].push(0.845 + lf * (-act_p).exp());
            data[pf - 1][lfan - 1][idx].push(0.845 + lf * (-act_l).exp());
        }
        let mut d = vec![0.0f64; 18];
        for pf in 0..3 {
            for lf in 0..3 {
                for tf in 0..2 {
                    let v = &data[pf][lf][tf];
                    d[pf + 3 * lf + 9 * tf] = v.iter().sum::<f64>() / v.len() as f64;
                }
            }
        }
        d
    }

    /// `(sdm-fct (list slot value '- other nil))` + `(sdp-fct (list name :name
    /// :activation))` (`dm-commands.lisp:491`/`:667`): the activation of every
    /// chunk with `slot = value` and `other` filled, in DM order.
    fn al_sdm_sdp(&mut self, arg1: Option<&str>, arg2: Option<&str>) -> Vec<f64> {
        let a1 = arg1.map(|s| s.to_uppercase());
        let a2 = arg2.map(|s| s.to_uppercase());
        let candidates: Vec<String> = self
            .dm
            .iter()
            .filter(|c| {
                if let Some(v) = &a1 {
                    if c.get("ARG1") != Value::Sym(v.clone()) {
                        return false;
                    }
                    if c.get("ARG2") == Value::Nil {
                        return false;
                    }
                }
                if let Some(v) = &a2 {
                    if c.get("ARG2") != Value::Sym(v.clone()) {
                        return false;
                    }
                    if c.get("ARG1") == Value::Nil {
                        return false;
                    }
                }
                true
            })
            .filter_map(|c| c.name.clone())
            .collect();
        candidates
            .iter()
            .map(|n| self.al_sdp_activation(n))
            .collect()
    }

    /// `compute-activation` for a named DM chunk with an empty request spec
    /// (`dm-commands.lisp:738`): base-level + spreading (no partial matching,
    /// `:ans` is nil).
    fn al_sdp_activation(&mut self, name: &str) -> f64 {
        let Some(idx) = self
            .dm
            .iter()
            .position(|c| c.name.as_deref() == Some(name))
        else {
            return f64::MIN;
        };
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
        let spread = self.spreading_activation(name, self.cfg.mas);
        (base + spread) as f64
    }

    /// `output-person-location` (`al-fan-study.lisp:467`): the correlation and
    /// RMSD against the experimental data plus the TARGETS/FOILS table.
    fn al_output_person_location(&mut self, d: &[f64]) {
        let data = [
            1.11, 1.17, 1.22, 1.17, 1.20, 1.22, 1.15, 1.23, 1.36, 1.20, 1.22, 1.26, 1.25,
            1.36, 1.29, 1.26, 1.47, 1.47,
        ];
        let n = d.len() as f64;
        let sum_r: f64 = d.iter().sum();
        let sum_d: f64 = data.iter().sum();
        let sum_r2: f64 = d.iter().map(|x| x * x).sum();
        let sum_d2: f64 = data.iter().map(|x| x * x).sum();
        let sum_rd: f64 = d.iter().zip(data.iter()).map(|(a, b)| a * b).sum();
        let avg_r = sum_r / n;
        let avg_d = sum_d / n;
        let corr = (sum_rd / n - avg_r * avg_d)
            / ((sum_r2 / n - avg_r * avg_r).sqrt() * (sum_d2 / n - avg_d * avg_d).sqrt());
        let md = ((sum_r2 + sum_d2 - 2.0 * sum_rd) / n).sqrt();
        self.out_lines
            .push(format!("CORRELATION: {corr:6.3}"));
        self.out_lines
            .push(format!("MEAN DEVIATION: {md:6.3}"));
        self.out_lines.push("TARGETS:".to_string());
        self.out_lines
            .push("                         Person fan".to_string());
        self.out_lines
            .push("  Location      1             2             3".to_string());
        self.out_lines.push("    fan".to_string());
        for i in 0..3 {
            let vals: Vec<String> = (0..3).map(|j| format!("{:8.3}", d[j + i * 3])).collect();
            self.out_lines
                .push(format!("     {}    {}", i + 1, vals.join("      ")));
        }
        self.out_lines.push(String::new());
        self.out_lines.push(String::new());
        self.out_lines.push("FOILS:".to_string());
        for i in 0..3 {
            let vals: Vec<String> = (0..3)
                .map(|j| format!("{:8.3}", d[j + (i + 3) * 3]))
                .collect();
            self.out_lines
                .push(format!("     {}    {}", i + 1, vals.join("      ")));
        }
    }

    /// `(goal-focus-fct (car (define-chunks-fct '(...))))`: define a named
    /// chunk (`{CTYPE}{n}`) and focus it.
    fn zz_goal_focus(
        &mut self,
        ctype: &str,
        slots: Vec<(String, Value)>,
    ) -> Result<(), EngineError> {
        let k = *self.name_count.entry(ctype.to_string()).or_insert(0);
        self.name_count.insert(ctype.to_string(), k + 1);
        let name = format!("{ctype}{k}");
        let inst = ChunkInst {
            name: Some(name.clone()),
            ctype: ctype.to_string(),
            slots,
            bare: false,
        };
        self.named.insert(name.clone(), inst);
        self.goal_focus(&ChunkDesc::Named(name))
    }

    /// `run-test` testing defaults (`test-model.lisp:67`): reset before each
    /// test, then apply the test's overrides.
    fn zz_run_test_defaults(&mut self) {
        self.cfg.zz_kct_ms = 10;
        self.cfg.zz_krt_ms = 40;
        self.cfg.zz_psd = None;
        self.cfg.zz_cdd = None;
        self.cfg.zz_two_proc = false;
        self.cfg.zz_two_exec = true;
        self.cfg.zz_query = "STATE".to_string();
        // `run-test` (`test-model.lisp:73`): `*dpd* .075` / `*fpd* .05` /
        // `*spd* .1`.
        self.cfg.zz_dpd_ms = 75;
        self.cfg.zz_fpd_ms = 50;
        self.cfg.zz_spd_ms = 100;
    }

    fn zz_test1(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        for hand in ["same", "diff"] {
            for query in ["state", "processor", "preparation"] {
                for processors in [true, false] {
                    for executions in [true, false] {
                        self.cfg.zz_two_proc = processors;
                        self.cfg.zz_two_exec = executions;
                        self.cfg.zz_query = query.to_uppercase();
                        self.reset()?;
                        self.out_lines
                            .push("########################".to_string());
                        self.out_lines.push(format!(
                            "\"{} hand {} processors {} executions query {}\"",
                            hand.to_uppercase(),
                            if processors { 2 } else { 1 },
                            if executions { 2 } else { 1 },
                            query.to_uppercase()
                        ));
                        self.out_lines
                            .push("########################".to_string());
                        self.zz_goal_focus(
                            "GOAL",
                            vec![
                                ("TEST".to_string(), Value::Sym("TEST1".to_string())),
                                ("PARAM".to_string(), Value::Sym(hand.to_uppercase())),
                            ],
                        )?;
                        let d = format!(
                            "{} hand {} processors {} executions query {}",
                            hand.to_uppercase(),
                            if processors { 2 } else { 1 },
                            if executions { 2 } else { 1 },
                            query.to_uppercase()
                        );
                        self.zz_data.push(("TEST".to_string(), 0, d));
                        self.run_secs(10.0)?;
                    }
                }
            }
        }
        self.zz_summarize();
        Ok(())
    }

    fn zz_test2(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        for (cdd, disp) in [
            (None, "NIL"),
            (Some(0.8), "T"),
            (Some(0.5), "0.5"),
            (Some(1.0), "1.0"),
        ] {
            self.cfg.zz_cdd = cdd;
            self.reset()?;
            self.zz_start_hand_at_mouse();
            let d = format!("click and hold mouse *cdd* {disp}");
            self.zz_data.push(("TEST".to_string(), 0, d));
            self.zz_goal_focus(
                "GOAL",
                vec![("TEST".to_string(), Value::Sym("TEST2".to_string()))],
            )?;
            self.run_secs(10.0)?;
        }
        self.zz_summarize();
        Ok(())
    }

    /// `(start-hand-at-mouse)` (`mouse.lisp:554`): put the right hand at the
    /// mouse cursor's current position.
    fn zz_start_hand_at_mouse(&mut self) {
        let (mx, my) = self.zz_mouse_cursor;
        self.zz_hands[1].loc = (mx, my);
    }

    fn zz_test3(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.cfg.zz_kct_ms = 25;
        self.cfg.zz_krt_ms = 25;
        self.zz_data
            .push(("TEST".to_string(), 0, "Auto release key & mouse when moving hand".to_string()));
        self.reset()?;
        self.zz_goal_focus(
            "GOAL",
            vec![("TEST".to_string(), Value::Sym("TEST3".to_string()))],
        )?;
        self.run_secs(0.5)?;
        self.zz_buffer_status();
        self.run_secs(0.75)?;
        self.zz_buffer_status();
        self.run_secs(5.0)?;
        self.zz_summarize();
        Ok(())
    }

    fn zz_test4(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.cfg.zz_psd = Some(0.2);
        self.zz_data
            .push(("TEST".to_string(), 0, "Some various key pressing".to_string()));
        self.reset()?;
        self.zz_goal_focus(
            "GOAL",
            vec![("TEST".to_string(), Value::Sym("TEST4".to_string()))],
        )?;
        self.zz_breaks = vec!["TEST4-7".to_string()];
        self.run_secs(10.0)?;
        self.zz_buffer_status();
        self.zz_print_finger_locs();
        self.run_secs(10.0)?;
        self.zz_buffer_status();
        self.zz_print_finger_locs();
        self.zz_summarize();
        Ok(())
    }

    /// `test5` (`test-model.lisp:188`): `(do ((time (run 10) (run 10))) ((= time
    /// 0.0)) ...)` — each run stops at a `pbreak`, and the body dumps the
    /// manual buffers and finger locations after every non-empty run.
    fn zz_test5(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Move the fingers and hands around".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "GOAL",
            vec![("TEST".to_string(), Value::Sym("TEST5".to_string()))],
        )?;
        self.zz_breaks = vec![
            "TEST5-5".to_string(),
            "TEST5-9".to_string(),
            "TEST5-11".to_string(),
            "TEST5-12".to_string(),
        ];
        loop {
            let ran = self.zz_run_step(10.0)?;
            if !ran {
                break;
            }
            self.zz_buffer_status();
            self.zz_print_finger_locs();
        }
        self.zz_summarize();
        Ok(())
    }

    /// `test6` (`test-model.lisp:202`): a single `(run 10)` exercising
    /// `type-key` from a non-home position, then the buffer/finger dump.
    fn zz_test6(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Some type-key actions from non-home position.".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "GOAL",
            vec![("TEST".to_string(), Value::Sym("TEST6".to_string()))],
        )?;
        self.run_secs(10.0)?;
        self.zz_buffer_status();
        self.zz_print_finger_locs();
        self.zz_summarize();
        Ok(())
    }

    /// `test7` (`test-model.lisp:212`): two runs of `delayed-punch` actions,
    /// the second with faster punch-delay parameters.
    fn zz_test7(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Default timed delayed-punch actions".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "CHUNK",
            vec![("TEST".to_string(), Value::Sym("TEST7".to_string()))],
        )?;
        self.run_secs(10.0)?;
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Faster timed delayed-punch actions".to_string(),
        ));
        self.cfg.zz_dpd_ms = 50;
        self.cfg.zz_fpd_ms = 25;
        self.cfg.zz_spd_ms = 70;
        self.reset()?;
        self.zz_goal_focus(
            "CHUNK",
            vec![("TEST".to_string(), Value::Sym("TEST7".to_string()))],
        )?;
        self.run_secs(10.0)?;
        self.zz_summarize();
        Ok(())
    }

    /// `test8` (`test-model.lisp:225`): hold several fingers down, then
    /// `all-fingers-to-home`, then dump buffers/finger locations.
    fn zz_test8(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "All home with multiple fingers down".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "CHUNK",
            vec![("TEST".to_string(), Value::Sym("TEST8".to_string()))],
        )?;
        self.run_secs(10.0)?;
        self.zz_buffer_status();
        self.zz_print_finger_locs();
        self.zz_summarize();
        Ok(())
    }

    /// `test9` (`test-model.lisp:236`): like test8 but `release-all-fingers`
    /// (including the second, no-op one that prints a warning).
    fn zz_test9(&mut self) -> Result<(), EngineError> {
        self.zz_run_test_defaults();
        self.zz_data.push((
            "TEST".to_string(),
            0,
            "Release-all-fingers with multiple fingers down".to_string(),
        ));
        self.reset()?;
        self.zz_goal_focus(
            "CHUNK",
            vec![("TEST".to_string(), Value::Sym("TEST9".to_string()))],
        )?;
        self.run_secs(10.0)?;
        self.zz_buffer_status();
        self.zz_print_finger_locs();
        self.zz_summarize();
        Ok(())
    }

    /// `(buffer-status manual-left manual-right)` (`motor-extension.lisp:1306`,
    /// `generic-state-query` at 1337).
    fn zz_buffer_status(&mut self) {        for (name, hand) in [("MANUAL-LEFT", 0usize), ("MANUAL-RIGHT", 1)] {
            let h = &self.zz_hands[hand];
            let proc = self.man_proc[hand];
            let exec = self.man_exec[hand];
            let state_busy = proc || exec;
            self.out_lines.push(format!("{name}:"));
            self.out_lines
                .push("  buffer empty          : T".to_string());
            self.out_lines
                .push("  buffer full           : NIL".to_string());
            self.out_lines
                .push("  buffer failure        : NIL".to_string());
            self.out_lines
                .push("  buffer requested      : NIL".to_string());
            self.out_lines
                .push("  buffer unrequested    : NIL".to_string());
            self.out_lines.push(format!(
                "  state free            : {}",
                if state_busy { "NIL" } else { "T" }
            ));
            self.out_lines.push(format!(
                "  state busy            : {}",
                if state_busy { "T" } else { "NIL" }
            ));
            self.out_lines
                .push("  state error           : NIL".to_string());
            self.out_lines.push(format!(
                "  processor free        : {}",
                if proc { "NIL" } else { "T" }
            ));
            self.out_lines.push(format!(
                "  processor busy        : {}",
                if proc { "T" } else { "NIL" }
            ));
            self.out_lines.push(format!(
                "  execution free        : {}",
                if exec { "NIL" } else { "T" }
            ));
            self.out_lines.push(format!(
                "  execution busy        : {}",
                if exec { "T" } else { "NIL" }
            ));
            let last = self.zz_last_cmd[hand].clone();
            self.out_lines
                .push(format!("  last-command          : {last}"));
            for (i, fname) in ["index", "middle", "ring", "pinkie", "thumb"]
                .iter()
                .enumerate()
            {
                self.out_lines.push(format!(
                    "  {fname:<7}free           : {}",
                    if h.busy[i] { "NIL" } else { "T" }
                ));
                self.out_lines.push(format!(
                    "  {fname:<7}down           : {}",
                    if h.down[i] { "T" } else { "NIL" }
                ));
            }
            self.out_lines.push(String::new());
        }
    }

    /// The test driver's `(finger-loc-m …)` dump (`test-model.lisp:178`).
    fn zz_print_finger_locs(&mut self) {
        for (hand, hname) in [(0usize, "LEFT"), (1, "RIGHT")] {
            for (f, fname) in ["INDEX", "MIDDLE", "RING", "PINKIE", "THUMB"]
                .iter()
                .enumerate()
            {
                let (x, y) = self.zz_finger_loc(hand, f);
                self.out_lines.push(format!("  {hname} {fname}: #({x} {y})"));
            }
        }
    }

    /// `print-instantiation` (`procedural.lisp:2441`): the selected
    /// production with its matched bindings, as a `pbreak` prints it.
    fn zz_print_instantiation(&mut self, prod: usize, bindings: &Bindings) {
        let p = self.model.productions[prod].clone();
        let mut lines = vec![format!("(P {}", p.name)];
        for item in &p.lhs {
            match item {
                LhsItem::Match { buffer, tests } => {
                    lines.push(format!("   ={buffer}>"));
                    for t in tests {
                        if t.slot.eq_ignore_ascii_case("ISA") {
                            continue;
                        }
                        lines.push(format!("       {} {}", t.slot, show_pat(&t.value, bindings)));
                    }
                }
                LhsItem::Query { buffer, queries } => {
                    lines.push(format!("   ?{buffer}>"));
                    for (slot, val) in queries {
                        lines.push(format!("       {slot} {}", show_pat(val, bindings)));
                    }
                }
                _ => {}
            }
        }
        lines.push(" ==>".to_string());
        for item in &p.rhs {
            match item {
                RhsAction::Request { buffer, spec } => {
                    lines.push(format!("   +{buffer}>"));
                    if let ChunkDesc::Spec { ctype, slots, .. } = spec {
                        // `production-statement-text` (`procedural.lisp:2555`)
                        // prints the spec's slots, then the request type as
                        // `CMD`.
                        for (slot, val) in slots {
                            lines.push(format!("       {slot} {}", show_pat(val, bindings)));
                        }
                        if !ctype.is_empty() {
                            lines.push(format!("       CMD {ctype}"));
                        }
                    }
                }
                RhsAction::Clear { buffer } => lines.push(format!("   -{buffer}>")),
                RhsAction::Modify { buffer, slots } => {
                    lines.push(format!("   ={buffer}>"));
                    for (slot, val) in slots {
                        lines.push(format!("       {slot} {}", show_pat(val, bindings)));
                    }
                }
                _ => {}
            }
        }
        lines.push(")".to_string());
        self.out_lines.extend(lines);
    }

    /// `(buffer-chunk <name>...)`: append the command-layer buffer dump the
    /// task driver prints after a run (`framework/buffers.lisp:881` +
    /// `framework/chunks.lisp:736`). Only the fields the corpus exercises:
    /// the buffer/chunk header (with the multi-buffer set for threads goals)
    /// and each slot. Not a trace line, so it is appended verbatim.
    pub fn print_buffer_chunk(&mut self, buffers: &[String]) {
        for name in buffers {
            let bname = name.to_uppercase();
            let chunk = self.buffers.get(&bname).and_then(|b| b.chunk.clone());
            let cname = chunk
                .as_ref()
                .and_then(|c| c.name.clone())
                .unwrap_or_default();
            let mut header = format!("{bname}: {cname} ");
            if let Some(src) = self.vl_copied_from.get(&cname) {
                // `~@[[~s]~]` (`chunk-copied-from-fct`): the source chunk.
                header.push_str(&format!("[{src}]"));
            }
            if self.cfg.threads && bname == "GOAL" {
                // `hash-table-keys` (`misc-utils.lisp:657`) pushes while
                // walking the goal m-buffer set; SBCL visits these small
                // tables in insertion order, so the printed set is reversed.
                let mut set: Vec<String> =
                    self.goal_set.iter().filter_map(|c| c.name.clone()).collect();
                set.reverse();
                header.push_str(&format!(" {{{}}}", set.join(" ")));
            }
            self.out_lines.push(header);
            if let Some(c) = &chunk {
                self.push_chunk(c);
            }
        }
    }

    /// `pprint-a-chunk` (`chunks.lisp:736`): the chunk's name, then
    /// `slot  value` lines, then a blank line (the `~%` plus
    /// `command-output`'s newline).
    fn push_chunk(&mut self, c: &ChunkInst) {
        self.out_lines.push(c.name.clone().unwrap_or_default());
        for (k, v) in &c.slots {
            self.out_lines.push(format!("   {k}  {}", v.show()));
        }
        self.out_lines.push(String::new());
    }

    /// `(sdm <spec>...)`: print the declarative-memory chunks matching the
    /// task's spec (`commands/dm-commands.lisp:491`). The corpus calls
    /// `(sdm isa pair)`, and `pair`'s only members are unnamed run chunks, so
    /// every DM chunk is printed (the merged buffer chunks, then the goals in
    /// `dm-chunks` order).
    pub fn print_sdm(&mut self, _spec: &[String]) {
        // `dm-chunks` (`declarative-memory.lisp:912`, `add-chunk-into-dm`
        // 1918) is an alist keyed by the chunk's slot signature, and a new
        // set is pushed to the *front* (1931), so `sdm` prints the merged
        // sets newest-first, with each set's chunks in insertion order,
        // followed by the model's `add-dm` chunks.
        let n_model = self.model.chunks.len().min(self.dm.len());
        let key = |c: &ChunkInst| {
            let mut names: Vec<String> = c.slots.iter().map(|(k, _)| k.clone()).collect();
            names.sort();
            (c.ctype.clone(), names)
        };
        let mut groups: Vec<Vec<usize>> = Vec::new();
        for i in n_model..self.dm.len() {
            let ki = key(&self.dm[i]);
            let found = groups.iter().position(|grp| {
                let j = grp[0];
                key(&self.dm[j]) == ki
            });
            match found {
                Some(g) => groups[g].push(i),
                None => groups.push(vec![i]),
            }
        }
        let mut prints: Vec<usize> = Vec::new();
        for grp in groups.iter().rev() {
            prints.extend(grp.iter().copied());
        }
        prints.extend(0..n_model);
        for i in prints {
            let c = self.dm[i].clone();
            self.push_chunk(&c);
        }
    }

    /// Schedule a no-op event at absolute model time `at_ms` (a real-time
    /// slack hook's `(schedule-event .5 'fn)`, `meta-process.lisp`): hidden at
    /// medium but wakes an armed conflict-resolution.
    pub fn schedule_dummy(&mut self, at_ms: i64) {
        self.push(at_ms, Prio::Num(0), Out::Always, Event::Dummy);
    }

    /// Emit a raw task warning line (`print-warning`), e.g. the act-touch
    /// stub's "Installing a default touch device…" banner.
    pub fn emit_warning(&mut self, msg: &str) {
        self.out_lines.push(format!("#|Warning: {msg} |#"));
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
    pub fn add_visicon(&mut self, slots: &[(String, Sexp)]) {
        let n = *self.name_count.entry("CHUNK".to_string()).or_insert(0);
        self.name_count.insert("CHUNK".to_string(), n + 1);
        let mut x = 0i64;
        let mut y = 0i64;
        let mut w = 0i64;
        let mut ht = 0i64;
        let mut extra: Vec<(String, Value)> = Vec::new();
        for (k, v) in slots {
            let ku = k.to_uppercase();
            match (ku.as_str(), v) {
                ("SCREEN-X", Sexp::Int(i)) => x = *i,
                ("SCREEN-Y", Sexp::Int(i)) => y = *i,
                ("WIDTH", Sexp::Int(i)) => w = *i,
                ("HEIGHT", Sexp::Int(i)) => ht = *i,
                _ => {
                    if let Ok(val) = Value::from_sexp(v) {
                        extra.push((ku, val));
                    }
                }
            }
        }
        self.features.push(Feature {
            name: format!("CHUNK{n}"),
            kind: "VISUAL-LOCATION".to_string(),
            value: String::new(),
            x,
            y,
            width: w,
            height: ht,
            extra,
            tstamp_ms: self.time_ms,
            attended: false,
        });
        self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
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

    /// Install task-file `defun`s for the bounded `!eval!`/`!bind!` evaluator.
    pub fn set_user_fns(&mut self, fns: HashMap<String, crate::model::UserDefun>) {
        self.user_fns = fns;
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
        self.rng.borrow_mut().set_seed(seed.0, seed.1);
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

    fn exec(&mut self, event: Event, show: bool) -> Result<(), EngineError> {
        match event {
            Event::SetBuffer {
                buffer,
                chunk,
                requested,
                shown,
            } => {
                let mut inst = self.instantiate(&buffer, chunk)?;
                // `set-buffer-chunk` on a buffer with `reuse?` nil (a
                // `buffer-requires-copies` buffer) converts the spec with
                // `define-chunks-fct` and so creates a fresh chunk auto-named
                // `CHUNK{k}` (`buffers.lisp:1360`); the trace still prints the
                // spec/chunk-def, so capture it before naming.
                let text = shown.unwrap_or_else(|| inst.show());
                if inst.name.is_none()
                    && self
                        .model
                        .no_reuse_buffers
                        .iter()
                        .any(|b| b == &buffer)
                {
                    let n = *self.name_count.entry("CHUNK".to_string()).or_insert(0);
                    self.name_count.insert("CHUNK".to_string(), n + 1);
                    inst.name = Some(format!("CHUNK{n}"));
                }
                if show {
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
                if buffer == "GOAL" && self.cfg.threads
                    && !self.goal_set.iter().any(|c| c.name == inst.name)
                {
                    self.goal_set.push(inst.clone());
                }
                self.buffer_mut(&buffer).chunk = Some(inst);
                self.buffer_mut(&buffer).failed = false;
                self.buffer_mut(&buffer).modified = false;
                self.changed();
            }
            Event::OverwriteBuffer {
                buffer,
                chunk,
                requested,
                shown,
                module,
            } => {
                let mut inst = self.instantiate(&buffer, chunk)?;
                let source = shown.clone().unwrap_or_else(|| inst.show());
                if buffer == "VISUAL" {
                    // The VISUAL buffer holds a `VISUAL-CHUNK0` copy of the
                    // overwritten chunk (`chunk-copied-from-fct`), so
                    // `buffer-chunk` prints `VISUAL-CHUNK0 [SOURCE]`.
                    self.vl_copied_from
                        .insert("VISUAL-CHUNK0".to_string(), source.clone());
                    inst.name = Some("VISUAL-CHUNK0".to_string());
                } else if buffer == "VISUAL-LOCATION" {
                    // `@visual-location> vl-1` copies into `VL-1-{k}`.
                    let n = *self.copy_count.entry(source.clone()).or_insert(0);
                    self.copy_count.insert(source.clone(), n + 1);
                    let copy = format!("{source}-{n}");
                    self.vl_copied_from.insert(copy.clone(), source.clone());
                    inst.name = Some(copy);
                }
                if show {
                    let mut details = format!("OVERWRITE-BUFFER-CHUNK {buffer} {source}");
                    if !requested {
                        details.push_str(" NIL");
                    }
                    self.emit(&module, details);
                }
                if buffer == "GOAL" && self.cfg.threads
                    && !self.goal_set.iter().any(|c| c.name == inst.name)
                {
                    self.goal_set.push(inst.clone());
                }
                self.buffer_mut(&buffer).chunk = Some(inst);
                self.buffer_mut(&buffer).failed = false;
                self.buffer_mut(&buffer).modified = false;
                self.changed();
            }
            Event::EraseBuffer { buffer, module } => {
                // `erase-buffer` (`buffers.lisp:1273`): drop the chunk but
                // keep the buffer state (no clear-buffer).
                if show {
                    self.emit(&module, format!("ERASE-BUFFER {buffer}"));
                }
                self.buffer_mut(&buffer).chunk = None;
                self.buffer_mut(&buffer).failed = false;
                self.buffer_mut(&buffer).modified = false;
                self.changed();
            }
            Event::VocalFree => {
                self.vocal_busy = false;
                self.changed();
            }
            Event::Speak { text } => {
                self.emit("SPEECH", format!("SPEAK TEXT {text}"));
                self.vocal_busy = true;
                // `speak` preparation (`speech.lisp:278`/`291`): 3 features
                // for a fresh style, else 0 for the same text, 2 otherwise.
                let feats = match self.last_speak.as_deref() {
                    None => 3,
                    Some(prev) if prev.eq_ignore_ascii_case(&text) => 0,
                    Some(_) => 2,
                };
                self.last_speak = Some(text.clone());
                let prep = feats * self.cfg.feat_prep_ms;
                self.push(
                    self.time_ms + prep,
                    Prio::Num(0),
                    Out::Always,
                    Event::SpeechPrep { text },
                );
            }
            Event::SpeechPrep { text } => {
                // `perform-movement`: initiation-complete at init-time, then
                // finish at exec + articulation (`speech.lisp:281`/`285`).
                // The hear-self sound onset equals the exec time.
                let init = self.cfg.init_ms;
                let art = (150.0f32)
                    .max(150.0 * text.chars().count() as f32 / 3.0)
                    .round() as i64;
                let finish = init + art;
                self.push(
                    self.time_ms + init,
                    Prio::Num(0),
                    Out::Always,
                    Event::SpeechInit,
                );
                self.push(
                    self.time_ms + init,
                    Prio::Num(0),
                    Out::Medium,
                    Event::NewSound,
                );
                self.push(
                    self.time_ms + finish,
                    Prio::Num(0),
                    Out::Never,
                    Event::VocalFree,
                );
            }
            Event::SpeechInit => {}
            Event::NewSound => {
                self.emit("AUDIO", "new-sound".to_string());
                // `stuff-sound-buffer` only sets AURAL-LOCATION while the
                // buffer is empty (`audio.lisp:716`); later sounds leave the
                // first location in place. Aural detection/recode delay.
                if self.audio_events == 0 {
                    self.push(
                        self.time_ms + 300,
                        Prio::Num(0),
                        Out::Medium,
                        Event::AuralLocation,
                    );
                }
            }
            Event::AuralLocation => {
                let n = self.audio_events;
                self.audio_events += 1;
                self.emit(
                    "AUDIO",
                    format!("SET-BUFFER-CHUNK AURAL-LOCATION AUDIO-EVENT{n} NIL"),
                );
            }
            Event::ClearGoalThread { name } => {
                if show {
                    self.emit("PROCEDURAL", "CLEAR-BUFFER GOAL".to_string());
                }
                self.goal_set
                    .retain(|g| g.name.as_deref() != Some(name.as_str()));
                if self
                    .buffers
                    .get("GOAL")
                    .and_then(|b| b.chunk.as_ref())
                    .and_then(|c| c.name.as_deref())
                    == Some(name.as_str())
                {
                    self.buffer_mut("GOAL").chunk = None;
                }
                self.buffer_mut("GOAL").failed = false;
                self.changed();
            }
            Event::ClearBuffer { buffer, module } => {
                if show {
                    self.emit(&module, format!("CLEAR-BUFFER {buffer}"));
                }
                let cleared = self.buffer_mut(&buffer).chunk.take();
                self.buffer_mut(&buffer).modified = false;
                if buffer == "GOAL" && self.cfg.threads {
                    if let Some(n) = cleared.as_ref().and_then(|c| c.name.clone()) {
                        self.goal_set
                            .retain(|g| g.name.as_deref() != Some(n.as_str()));
                    }
                }
                self.buffer_mut(&buffer).failed = false;
                // `:notify-on-clear` for declarative: the cleared chunk
                // merges into DM, gaining a reference
                // (`merge-chunk-into-dm`). Empty clears merge nothing.
                if let Some(chunk) = cleared {
                    // `al-check-cleared-chunks` (`associative-learning.lisp:562`):
                    // a source buffer's clear schedules the parameter update at
                    // priority 5 (after the merge, before a retrieval's -2000).
                    let al = if self.cfg.al_eal.is_some() && self.al_is_source_buffer(&buffer) {
                        Some(chunk.name.clone())
                    } else {
                        None
                    };
                    self.merge_into_dm(chunk, &buffer);
                    if let Some(name) = al {
                        let true_name = name
                            .as_ref()
                            .and_then(|n| self.chunk_alias.get(n).cloned())
                            .or(name);
                        if let Some(true_name) = true_name {
                            self.now(
                                Prio::Num(5),
                                Out::Never,
                                Event::AlBufferCleared { chunk: true_name },
                            );
                        }
                    }
                }
                self.changed();
            }
            Event::UnstuffVl { copy } => {
                // `check-unstuff-buffer` (`vision.lisp:1944`): only an
                // unchanged, unrequested auto-stuffed copy is erased.
                let valid = self.unstuff_valid(&copy);
                if valid {
                    if show {
                        self.emit("VISION", "ERASE-BUFFER VISUAL-LOCATION".to_string());
                    }
                    self.buffer_mut("VISUAL-LOCATION").chunk = None;
                    self.buffer_mut("VISUAL-LOCATION").failed = false;
                    self.changed();
                }
            }
            Event::AlBufferCleared { chunk } => {
                // `al-buffer-cleared` (`associative-learning.lisp:667`): update
                // with the chunk's own sources plus the other source buffers.
                self.event_silent = true;
                let mut js = self.al_chunk_sources(&chunk);
                js.extend(self.al_source_buffer_chunks());
                self.al_update(&chunk, &js);
            }
            Event::ModBuffer {
                buffer,
                slots,
                module,
            } => {
                if show {
                    self.emit(&module, format!("MOD-BUFFER-CHUNK {buffer}"));
                }
                {
                    let b = self.buffer_mut(&buffer);
                    let chunk = b
                        .chunk
                        .as_mut()
                        .ok_or_else(|| bad(format!("MOD-BUFFER-CHUNK with empty {buffer}")))?;
                    for (s, v) in slots.iter() {
                        match chunk.slots.iter_mut().find(|(k, _)| k == s) {
                            Some((_, old)) => *old = v.clone(),
                            None => chunk.slots.push((s.clone(), v.clone())),
                        }
                    }
                }
                // `check-unstuff-buffer` sees a modified (non-copy) chunk.
                self.buffer_mut(&buffer).modified = true;
                // A `*imaginal>` mod-request frees the imaginal module once
                // its delayed change lands (`module-mod-request`).
                if buffer == "IMAGINAL" && module == "IMAGINAL" {
                    self.imag_busy = false;
                }
                if self.cfg.tracker {
                    let names: Vec<String> = slots.iter().map(|(s, _)| s.clone()).collect();
                    for s in names {
                        self.tracker_check_buffer_changes(&buffer, Some(&s));
                    }
                }
                self.changed();
            }
            Event::ModuleRequest {
                buffer,
                spec,
                var_slots,
                trace,
            } => {
                if show && trace {
                    self.emit("PROCEDURAL", format!("MODULE-REQUEST {buffer}"));
                }
                if !trace {
                    // A task `module-request` dispatches straight to the
                    // module (`request-module`, `modules.lisp:819`); it is not
                    // a buffer change, so it must not wake conflict resolution
                    // (the module's own set/clear does that).
                    self.event_silent = true;
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
                            let delay = if self.cfg.vidt {
                                self.randomize_ms(self.cfg.imaginal_delay_ms)
                            } else {
                                self.cfg.imaginal_delay_ms
                            };
                            let at = self.time_ms + delay;
                            self.push(at, Prio::Num(0), Out::Never, Event::ImaginalDone { spec });
                        }
                    }
                    "TRACKER" => {
                        if let NewChunk::Spec { slots, .. } = spec {
                            self.tracker_request(&slots)?;
                        }
                    }
                    "TEMPORAL" => {
                        self.temporal_request(spec)?;
                    }
                    "VOCAL" => {
                        self.vocal_request(spec)?;
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
                        if self.cfg.threads {
                            // Threaded goal request: create a fresh chunk
                            // (`CHUNKn`) and overwrite it into the
                            // multi-buffer while keeping the other goals.
                            let mut inst = self.instantiate("GOAL", spec)?;
                            let k = *self.name_count.entry("CHUNK".to_string()).or_insert(0);
                            self.name_count.insert("CHUNK".to_string(), k + 1);
                            inst.name = Some(format!("CHUNK{k}"));
                            self.now(
                                Prio::Num(-1000),
                                Out::Low,
                                Event::OverwriteBuffer {
                                    buffer: "GOAL".to_string(),
                                    chunk: NewChunk::Instance(inst),
                                    requested: true,
                                    shown: None,
                                    module: "GOAL".to_string(),
                                },
                            );
                            return Ok(());
                        }
                        // goal-style-request: delay 0, priority -1000, so the
                        // new goal lands after the production's clears
                        // (`support/goal-style-module.lisp:245`). Request
                        // specs print slots-only (GOTCHA #44); a whole-chunk
                        // reference (`+goal> =retrieval`) prints the
                        // referenced chunk's slots.
                        let shown = self.named_chunk_shown(&spec);
                        self.push(
                            self.time_ms,
                            Prio::Num(-1000),
                            // `schedule-set-buffer-chunk` defaults to
                            // `:output low` (`buffers.lisp:1403`), so the
                            // goal set prints at any detail level.
                            Out::Low,
                            Event::SetBuffer {
                                buffer: "GOAL".to_string(),
                                chunk: spec,
                                requested: true,
                                shown,
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
                // `retrieval-request-hook` → `al-chunk-retrieval-request`
                // (`associative-learning.lisp:644`): record the source-buffer
                // chunks the retrieval will learn from.
                if self.cfg.al_eal.is_some() {
                    self.al_last_contexts = self.al_source_buffer_chunks();
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
                // `retrieved-chunk-hook` → `al-chunk-retrieved`
                // (`associative-learning.lisp:655`): learn from the retrieved
                // chunk and the context recorded at the request.
                if self.cfg.al_eal.is_some() {
                    let ctx = self.al_last_contexts.clone();
                    self.al_update(&name, &ctx);
                }
                // `set-buffer-chunk` copies the retrieved chunk (`buffer-copy`),
                // so the buffer holds `G-k` while the trace shows the source.
                let copy = self.copy_chunk(&name);
                self.now(
                    Prio::Max,
                    Out::Low,
                    Event::SetBuffer {
                        buffer: "RETRIEVAL".to_string(),
                        chunk: NewChunk::Named(copy),
                        requested: true,
                        shown: Some(name),
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
                // `pbreak` (`procedural.lisp:2318`): print the instantiation,
                // emit a break event, and halt the run before the reads.
                if self.zz_breaks.iter().any(|b| b == &name) {
                    self.zz_print_instantiation(prod, &bindings);
                    self.emit("------", format!("BREAK-EVENT PRODUCTION {name}"));
                    self.zz_break_hit = true;
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
                        LhsItem::Eval { .. }
                        | LhsItem::Bind { .. }
                        | LhsItem::MvBind { .. } => {}
                    }
                }
                for buffer in self.exec_prods[prod].implicit_queries.clone() {
                    self.now(Prio::Num(0), Out::Always, Event::QueryBuffer { buffer });
                }
                let action_ms = self.model.productions[prod]
                    .at
                    .map(|s| (s * 1000.0).round() as i64)
                    .unwrap_or(self.cfg.dat_ms);
                let action_ms = if self.cfg.vpft {
                    self.randomize_ms(action_ms)
                } else {
                    action_ms
                };
                let at = self.time_ms + action_ms;
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
                // `(spp <prod> :reward R)`: propagate the reward at fire time
                // (after the RHS actions), like ACT-R's `propagate-reward`.
                if let Some(r) = self.model.productions[prod].reward {
                    self.now(Prio::Num(0), Out::Medium, Event::PropagateReward { value: r });
                }
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
            Event::StartTracking => {
                if show {
                    self.emit("VISION", "START-TRACKING".into());
                }
                let feat = self
                    .last_attention
                    .as_ref()
                    .and_then(|loc| self.resolve_loc(loc).ok())
                    .map(|i| self.features[i].name.clone());
                if let Some(name) = feat {
                    self.tracked = Some(name);
                    self.tracked_lastloc = None;
                    // `change-state :exec 'BUSY` (`vision.lisp:2677`): the
                    // vision module stays busy while tracking, so
                    // `?visual> state free` fails.
                    self.visual_busy = true;
                    self.update_tracking(false);
                }
            }
            Event::MoveAttention { loc } => {
                if show {
                    self.emit("VISION", format!("Move-attention {loc} NIL"));
                }
                self.last_attention = Some(loc.clone());
                self.visual_busy = true;
                if self.cfg.emma {
                    // EMMA replaces the fixed-latency encoding with its
                    // saccade machinery (`emma-attention-hook`).
                    let end = self.emma_loc_xyz(&loc);
                    self.emma_attention_hook(end);
                } else {
                    let lat = self.randomize_ms(self.cfg.attention_ms);
                    let at = self.time_ms + lat;
                    self.push(
                        at,
                        Prio::Num(0),
                        Out::Medium,
                        Event::EncodingComplete { loc, requested: true },
                    );
                }
            }
            Event::EncodingComplete { loc, requested } => {
                if show {
                    self.emit("VISION", format!("Encoding-complete {loc} NIL"));
                }
                self.encoding_complete(loc, requested)?;
            }
            Event::EmmaPrepComplete { trgt } => {
                if show {
                    self.emit(
                        "EMMA",
                        format!("Preparation-complete {}", lisp_sym_opt(&trgt)),
                    );
                }
                self.emma_preparation_complete();
            }
            Event::EmmaInitComplete => {
                if show {
                    self.emit("EMMA", "INITIATION-COMPLETE".into());
                }
                // `initiation-complete`: free the processor state.
            }
            Event::EmmaEyeMoveComplete { sacc } => {
                if show {
                    self.emit(
                        "EMMA",
                        format!(
                            "Complete-eye-movement {} {}",
                            lisp_sym_opt(&sacc.trgt),
                            fmt_vec3(sacc.new_loc)
                        ),
                    );
                }
                self.emma_complete_eye_move(sacc);
            }
            Event::VisualClear => {
                if show {
                    self.emit("VISION", "CLEAR".into());
                }
                self.visual_busy = false;
                // `clear :after` (`vision.lisp:2886`) drops the attended
                // object and the current marker, so a later window change
                // no longer re-encodes the old location.
                self.last_attention = None;
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
            Event::Emit { module, line } => {
                if show {
                    self.emit(&module, line);
                }
            }
            Event::ZzKeyPrep {
                cmd,
                style,
                key,
                hand,
                finger,
                r,
                theta,
                loc,
                delay_ms,
                req_ms,
            } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_for(&style, hand)
                        ),
                    );
                }
                let _ = theta;
                let _ = cmd;
                self.man_prep_busy = false;
                let init = self.cfg.init_ms;
                let burst = self.cfg.burst_ms;
                let kct = self.cfg.zz_kct_ms;
                let mv = (burst as f32 / 1000.0)
                    .max((self.cfg.touch_peck_coeff * (r + 0.5).log2())
                        .max(self.cfg.min_fitts_ms as f32 / 1000.0));
                let move_ms = (mv * 1000.0).round() as i64;
                let strike = self.zz_movement_strike_time(move_ms, r);
                let (exec_ms, exec2_ms, finish_ms) = match style.as_str() {
                    // `hold-peck` (`motor-extension.lisp:1909`): exec is the
                    // strike time, finish is the full move time (they differ
                    // only when `peck-strike-distance` is set).
                    "HOLD-PECK" => (init + strike, None, init + move_ms),
                    "PECK" => (
                        init + strike,
                        Some(init + move_ms + self.cfg.zz_krt_ms),
                        init + move_ms + burst,
                    ),
                    // `peck-recoil` (`motor-extension.lisp:1792`): finish adds a
                    // second full move time (strike out and recoil back).
                    "PECK-RECOIL" => (
                        init + strike,
                        Some(init + move_ms + self.cfg.zz_krt_ms),
                        init + move_ms + burst + move_ms,
                    ),
                    "HOLD-PUNCH" => (init + kct, None, init + burst),
                    // `delayed-punch` (`motor-extension.lisp:2657`): the release
                    // (exec2) and finish are pushed out by the `delay` slot.
                    "DELAYED-PUNCH" => (
                        init + kct,
                        Some(init + kct + delay_ms),
                        init + kct + delay_ms + burst,
                    ),
                    "PUNCH" => (
                        init + kct,
                        Some(init + self.cfg.zz_krt_ms),
                        init + burst + burst,
                    ),
                    "RELEASE" => (init + self.cfg.zz_krt_ms, None, init + burst),
                    // `release-recoil` (`motor-extension.lisp:1991`): a peck
                    // that releases at exec and recoils home.
                    "RELEASE-RECOIL" => (
                        init + self.cfg.zz_krt_ms,
                        None,
                        init + move_ms + burst,
                    ),
                    // `point-finger` (`motor-extension.lisp:2036`): exec is
                    // `max(init, move-time)`, finish `init + move-time`.
                    "POINT-FINGER" => (init.max(move_ms), None, init + move_ms),
                    _ => (init, None, init + burst),
                };
                self.man_exec_queue.push(ZzPending {
                    key: key.clone(),
                    req_ms,
                    hand,
                    both: false,
                    style: style.clone(),
                    exec_ms,
                    finish_ms,
                    kind: ZzKind::Key {
                        finger,
                        exec2_ms,
                        // `point-finger` and the release styles produce no
                        // `output-key`; `release`/`release-recoil` emit a
                        // `release-key` at exec instead.
                        output: !matches!(style.as_str(), "POINT-FINGER" | "RELEASE" | "RELEASE-RECOIL"),
                        release_exec: matches!(style.as_str(), "RELEASE" | "RELEASE-RECOIL"),
                    },
                });
                // `move-a-finger` (`motor.lisp:712`) shifts the finger's
                // *offset*; the hand loc is unchanged (only `move-hand` moves
                // it). The next same/move test reads `hand-loc + finger-offset`
                // (`motor-extension.lisp:2120`).
                let target = loc.or_else(|| key_xy(&key));
                if let Some(kl) = target {
                    // `peck`/`point-finger`/`hold-peck` leave the finger at the
                    // key; `peck-recoil` returns it to where it started
                    // (`updated-pos` is the start, `motor-extension.lisp:1776`),
                    // and `release` doesn't move it.
                    if !matches!(style.as_str(), "RELEASE" | "PECK-RECOIL") {
                        let (hx, hy) = self.zz_hands[hand].loc;
                        self.zz_hands[hand].offsets[finger] = (kl.0 - hx, kl.1 - hy);
                    }
                }
                self.zz_maybe_execute();
            }
            Event::ZzKeyOutput {
                key,
                down,
                hand,
                finger,
            } => {
                // `set-finger-down`/`set-finger-up` (`motor-extension.lisp:1579`/
                // `:1593`) update the tracker and then `notify-motor-device`,
                // which *schedules* `output-key`/`release-key`
                // (`keyboard.lisp:503`) — so the trace is a separate event whose
                // sequence lands after events already queued at this time.
                let name = if down { "output-key" } else { "release-key" };
                self.now(
                    Prio::Num(0),
                    Out::Always,
                    Event::Emit {
                        module: "KEYBOARD".to_string(),
                        line: format!("{name} {} {}", self.model.name, key.to_lowercase()),
                    },
                );
                // The finger goes down/up at execution time (`set-finger-down`
                // / `set-finger-up`).
                self.zz_hands[hand].down[finger] = down;
                self.zz_hands[hand].held[finger] = if down { Some(key.clone()) } else { None };
                if down {
                    self.zz_data
                        .push(("PRESSED".to_string(), self.time_ms, key));
                } else {
                    self.zz_data
                        .push(("RELEASED".to_string(), self.time_ms, key));
                }
            }
            Event::ZzHandPrep {
                req_ms,
                r,
                theta,
                to_mouse,
            } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_for("HAND-PLY", 1)
                        ),
                    );
                }
                self.man_prep_busy = false;
                let init = self.cfg.init_ms;
                let burst = self.cfg.burst_ms;
                let fit = (self.cfg.touch_cursor_coeff * (r / 4.0 + 0.5).log2())
                    .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                let exec = init + (fit * 1000.0).round() as i64;
                let finish = exec + burst;
                self.man_exec_queue.push(ZzPending {
                    key: String::new(),
                    req_ms,
                    hand: 1,
                    both: false,
                    style: "HAND-PLY".to_string(),
                    exec_ms: exec,
                    finish_ms: finish,
                    kind: ZzKind::Hand { to_mouse, r, theta },
                });
                self.zz_maybe_execute();
            }
            Event::ZzHandPlyPrep {
                class,
                hand,
                r,
                theta,
                r_str,
                theta_str,
                target,
                req_ms,
                deferred,
            } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_hand(&class, hand, false)
                        ),
                    );
                }
                let _ = theta;
                self.man_prep_busy = false;
                let init = self.cfg.init_ms;
                let burst = self.cfg.burst_ms;
                // `hand-ply` (`motor.lisp:1084`): exec = init + fitts with the
                // mouse coefficient and the default target width (1.0); finish
                // = exec + burst.
                let fit = (self.cfg.touch_cursor_coeff * (r + 0.5).log2())
                    .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                let exec = init + (fit * 1000.0).round() as i64;
                let finish = exec + burst;
                let pending = ZzPending {
                    key: String::new(),
                    req_ms,
                    hand,
                    both: false,
                    style: class,
                    exec_ms: exec,
                    finish_ms: finish,
                    kind: ZzKind::HandPly {
                        target,
                        r: r_str,
                        theta: theta_str,
                    },
                };
                if deferred {
                    // `preparation-complete` (`motor-extension.lisp:1019`): a
                    // non-immediate movement is not queued, and its processor is
                    // freed here rather than at `initiation-complete`.
                    self.zz_prepared = Some(pending);
                    if self.cfg.zz_two_proc {
                        self.man_proc[hand] = false;
                    } else {
                        self.man_proc[0] = false;
                        self.man_proc[1] = false;
                    }
                } else {
                    self.man_exec_queue.push(pending);
                    self.zz_maybe_execute();
                }
            }
            Event::ZzAllHomePrep { req_ms, hands, r } => {
                let both = hands.len() == 2;
                let hand = if both { 1 } else { hands[0] };
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_hand("ALL-FINGERS-TO-HOME", hand, both)
                        ),
                    );
                }
                self.man_prep_busy = false;
                let init = self.cfg.init_ms;
                // `all-fingers-to-home` (`motor-extension.lisp:2349`): exec is
                // `max(init, randomize(max(.2, fitts(mouse-coeff, r, 4.0))))`;
                // finish is `init + exec`.
                let fit = (self.cfg.touch_cursor_coeff * (r / 4.0 + 0.5).log2())
                    .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                let m = 0.2f32.max(fit);
                let exec = init.max((m * 1000.0).round() as i64);
                let finish = init + exec;
                self.man_exec_queue.push(ZzPending {
                    key: String::new(),
                    req_ms,
                    hand,
                    both,
                    style: "ALL-FINGERS-TO-HOME".to_string(),
                    exec_ms: exec,
                    finish_ms: finish,
                    kind: ZzKind::AllHome { hands },
                });
                self.zz_maybe_execute();
            }
            Event::ZzReleaseAllPrep { req_ms, hands } => {
                let both = hands.len() == 2;
                let hand = if both { 1 } else { hands[0] };
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_hand("RELEASE-ALL-FINGERS", hand, both)
                        ),
                    );
                }
                self.man_prep_busy = false;
                // `release-all-fingers` (`motor-extension.lisp:2577`): exec is
                // `init + key-release-time`, finish is `init + 2*burst`.
                let exec = self.cfg.init_ms + self.cfg.zz_krt_ms;
                let finish = self.cfg.init_ms + 2 * self.cfg.burst_ms;
                self.man_exec_queue.push(ZzPending {
                    key: String::new(),
                    req_ms,
                    hand,
                    both,
                    style: "RELEASE-ALL-FINGERS".to_string(),
                    exec_ms: exec,
                    finish_ms: finish,
                    kind: ZzKind::ReleaseAll { hands },
                });
                self.zz_maybe_execute();
            }
            Event::ZzMousePrep { cmd, req_ms, loc } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix()
                        ),
                    );
                }
                self.man_prep_busy = false;
                if self.cfg.zz_two_exec {
                    self.man_exec[1] = true;
                } else {
                    self.man_exec[0] = true;
                    self.man_exec[1] = true;
                }
                let style = self.zz_last_style.clone().map(|x| x.0).unwrap_or_default();
                let init = self.cfg.init_ms;
                let burst = self.cfg.burst_ms;
                let (exec_ms, finish_ms, exec2_ms) = match cmd.as_str() {
                    "MOVE-CURSOR" => {
                        let (tx, ty, _) = loc.clone().unwrap_or((0, 0, String::new()));
                        let (cx, cy) = (self.zz_mouse_pos.0, self.zz_mouse_pos.1);
                        let r = (((tx - cx) as f32).powi(2) + ((ty - cy) as f32).powi(2)).sqrt();
                        let mut coef = self.cfg.touch_cursor_coeff;
                        if let Some(d) = self.cfg.zz_cdd {
                            if self.zz_mouse_down {
                                coef *= 2.0f32.powf(d as f32);
                            }
                        }
                        let fit = (coef * (r / 18.85 + 0.5).log2())
                            .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                        let e = init + (fit * 1000.0).round() as i64;
                        (e, e + burst, None)
                    }
                    "CLICK-MOUSE" => (
                        init + self.cfg.zz_kct_ms,
                        init + burst + burst,
                        Some(init + self.cfg.zz_krt_ms),
                    ),
                    "HOLD-MOUSE" => (init + self.cfg.zz_kct_ms, init + burst, None),
                    "RELEASE-MOUSE" => (init + self.cfg.zz_krt_ms, init + burst, None),
                    _ => (init, init + burst, None),
                };
                let t = self.time_ms;
                self.push(
                    t + init,
                    Prio::Num(0),
                    Out::Always,
                    Event::InitiationComplete {
                        req_ms,
                        hand: 1,
                        both: false,
                        style: style.clone(),
                    },
                );
                self.push(
                    t + exec_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::ZzMouseNotify {
                        cmd: cmd.clone(),
                        req_ms,
                        loc: loc.clone(),
                    },
                );
                if let Some(e2) = exec2_ms {
                    self.push(
                        t + e2,
                        Prio::Num(0),
                        Out::Always,
                        Event::ZzMouseRelease { req_ms },
                    );
                }
                self.push(
                    t + finish_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::FinishMovement {
                        req_ms,
                        hand: 1,
                        both: false,
                        style,
                    },
                );
            }
            Event::ZzMouseNotify { cmd, req_ms, loc } => {
                let _ = req_ms;
                let (mx, my, mz) = self.zz_mouse_pos;
                match cmd.as_str() {
                    "MOVE-CURSOR" => {
                        let (tx, ty, _) = loc.clone().unwrap_or((0, 0, String::new()));
                        self.zz_mouse_pos = (tx, ty, 1080);
                        self.emit(
                            "MOUSE",
                            format!("move-cursor {} mouse ({} {} {})", self.model.name, tx, ty, 1080),
                        );
                    }
                    "CLICK-MOUSE" | "HOLD-MOUSE" => {
                        self.zz_mouse_down = true;
                        self.emit(
                            "MOUSE",
                            format!("click-mouse {} ({} {} {}) INDEX", self.model.name, mx, my, mz),
                        );
                        self.zz_data.push((
                            "CLICKED".to_string(),
                            self.time_ms,
                            format!("({mx} {my} {mz})"),
                        ));
                    }
                    "RELEASE-MOUSE" => {
                        self.zz_mouse_down = false;
                        self.emit(
                            "MOUSE",
                            format!("release-mouse {} ({} {} {}) index", self.model.name, mx, my, mz),
                        );
                        self.zz_data.push((
                            "UNCLICKED".to_string(),
                            self.time_ms,
                            format!("({mx} {my} {mz})"),
                        ));
                    }
                    _ => {}
                }
            }
            Event::ZzMouseRelease { req_ms } => {
                let _ = req_ms;
                self.zz_mouse_down = false;
                let (mx, my, mz) = self.zz_mouse_pos;
                self.emit(
                    "MOUSE",
                    format!("release-mouse {} ({} {} {}) INDEX", self.model.name, mx, my, mz),
                );
                self.zz_data.push((
                    "UNCLICKED".to_string(),
                    self.time_ms,
                    format!("({mx} {my} {mz})"),
                ));
            }
            Event::TrackerUpdate { gen } => {
                if gen == self.tracker_gen {
                    if show {
                        if let Some(t) = self.trackers.first() {
                            self.emit(
                                "TRACKER",
                                format!("update-tracker {} {}", t.name, t.control_slot),
                            );
                        }
                    }
                    self.tracker_update()?;
                } else {
                    self.event_silent = true;
                }
            }
            Event::Choose => {
                self.tracker_set_prompt("CHOOSE");
            }
            Event::ModFocus { slots } => {
                if show {
                    self.emit("GOAL", "GOAL-MODIFICATION".into());
                }
                if let Some(c) = self.buffers.get_mut("GOAL").and_then(|b| b.chunk.as_mut()) {
                    for (k, v) in slots {
                        match c.slots.iter_mut().find(|(s, _)| s == &k) {
                            Some((_, old)) => *old = v.clone(),
                            None => c.slots.push((k.clone(), v.clone())),
                        }
                    }
                }
                self.changed();
            }
            Event::ModifyVisicon { name, x, y } => {
                if let Some(f) = self.features.iter_mut().find(|f| f.name == name) {
                    f.x = x;
                    f.y = y;
                    f.tstamp_ms = self.time_ms;
                }
                // `modify-visicon-features` triggers a visicon update and
                // `proc-display` (`vision.lisp`), re-stuffing the location.
                self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
            }
            Event::TouchPrep { plan } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!("PREPARATION-COMPLETE {}", Self::secs_str(plan.req_ms)),
                    );
                }
                self.man_prep_busy = false;
                self.man_exec_busy = true;
                let t = self.time_ms;
                self.push(
                    t + plan.init_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::TouchInit { plan: plan.clone() },
                );
                self.push(
                    t + plan.exec_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::TouchSignal { plan: plan.clone() },
                );
                if plan.up_at_exec {
                    self.push(
                        t + plan.exec_ms,
                        Prio::Num(-1),
                        Out::Never,
                        Event::TouchUp { plan: plan.clone() },
                    );
                } else if plan.up_at_exec_burst {
                    self.push(
                        t + plan.exec_ms + self.cfg.burst_ms,
                        Prio::Num(-1),
                        Out::Never,
                        Event::TouchUp { plan: plan.clone() },
                    );
                }
                self.push(
                    t + plan.finish_ms,
                    Prio::Num(0),
                    Out::Always,
                    Event::TouchFinish { plan },
                );
            }
            Event::TouchInit { plan } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!("INITIATION-COMPLETE {}", Self::secs_str(plan.req_ms)),
                    );
                }
                if plan.down_at_init {
                    if let (Some(h), Some(f)) = (
                        Self::touch_hand_idx(&plan.hand),
                        Self::touch_finger_idx(&plan.finger),
                    ) {
                        self.touch_fingers[h][f].2 = 0;
                    }
                }
            }
            Event::TouchSignal { plan } => {
                self.emit(&plan.signal_module, plan.signal.clone());
                if !plan.monitor.is_empty() {
                    self.out_lines.push(plan.monitor.clone());
                }
                if let (Some(h), Some(f)) = (
                    Self::touch_hand_idx(&plan.hand),
                    Self::touch_finger_idx(&plan.finger),
                ) {
                    if plan.down_at_exec {
                        self.touch_fingers[h][f].2 = 0;
                    }
                    if plan.up_at_exec {
                        self.touch_fingers[h][f].2 = 1;
                    }
                    if let Some((x, y)) = plan.move_to {
                        self.touch_fingers[h][0] = (x, y, self.touch_fingers[h][0].2);
                    }
                    if plan.put_hand {
                        self.start_hand_at_touch(&plan.hand);
                    }
                }
            }
            Event::TouchUp { plan } => {
                if let (Some(h), Some(f)) = (
                    Self::touch_hand_idx(&plan.hand),
                    Self::touch_finger_idx(&plan.finger),
                ) {
                    self.touch_fingers[h][f].2 = 1;
                }
            }
            Event::TouchFinish { plan } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!("FINISH-MOVEMENT {}", Self::secs_str(plan.req_ms)),
                    );
                }
                self.man_exec_busy = false;
            }
            Event::TemporalTick { gen, details } => {
                if gen == self.temporal_gen {
                    if show {
                        self.emit("TEMPORAL", details);
                    }
                    self.temporal_tick(show)?;
                } else {
                    // Cancelled by a newer request (`delete-event`): it must
                    // not wake conflict resolution.
                    self.event_silent = true;
                }
            }
            Event::TemporalClear => {
                if show {
                    self.emit("TEMPORAL", "Clear".into());
                }
                let b = self.buffer_mut("TEMPORAL");
                b.chunk = None;
                b.failed = false;
            }
            Event::PropagateReward { value } => {
                if show {
                    self.emit("UTILITY", format!("PROPAGATE-REWARD {value}"));
                }
                // Rewards apply only with subsymbolics + learning on;
                // otherwise silently ignored (Lisp warns to stdout).
                if self.cfg.esc && self.cfg.ul {
                    if self.cfg.ult {
                        self.out(format!(
                            " Utility updates with Reward = {}   alpha = {}",
                            lisp_s_f32(value),
                            lisp_s_f32(self.cfg.alpha)
                        ));
                    }
                    for (prod, at) in self.history.clone() {
                        let dt = (self.time_ms - at) as f32 / 1000.0;
                        let r = value - dt;
                        let u_old = self.utilities[prod];
                        let alpha = self.cfg.alpha;
                        let u_new = u_old + alpha * (r - u_old);
                        self.utilities[prod] = u_new;
                        if self.cfg.ult {
                            let pname = self.model.productions[prod].name.clone();
                            self.out(format!("  Updating utility of production {pname}"));
                            self.out(format!(
                                "   U(n-1) = {}   R(n) = {} [{} - {} seconds since selection]",
                                lisp_s_f32(u_old),
                                lisp_s_f32(r),
                                lisp_s_f32(value),
                                lisp_s_f32(dt)
                            ));
                            self.out(format!("   U(n) = {}", lisp_s_f32(u_new)));
                        }
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
            Event::PressKey { key, req_ms, isa: _isa } => {
                if show {
                    self.emit("MOTOR", format!("PRESS-KEY KEY {key}"));
                }
                // Preparation features. `key->cmd` maps home-row keys to the
                // `punch` style (`motor.lisp:935`, slots hand/finger) and the
                // rest to `peck-recoil` (slots hand/finger/r/theta). Fresh
                // movement uses `num-to-prepare` (`general-pm.lisp:790`); a
                // repeat uses the style's `feat-differences`
                // (`motor.lisp:950`/`970`).
                let spec = Self::key_spec(&key)?;
                let punch = key_is_punch(&key);
                let prep_feats = |p: bool| if p { 3 } else { 5 };
                let feats = match &self.last_press {
                    None => prep_feats(punch),
                    Some((pk, h, f, r, t)) => {
                        if key_is_punch(pk) != punch {
                            prep_feats(punch)
                        } else if punch {
                            if *h != spec.hand {
                                2
                            } else if *f != spec.finger {
                                1
                            } else {
                                0
                            }
                        } else if *h != spec.hand {
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
                    key.to_uppercase(),
                    spec.hand.to_string(),
                    spec.finger.to_string(),
                    spec.r,
                    spec.theta,
                ));
                self.man_prep_busy = true;
                if self.cfg.zz_ext {
                    let style = if key_is_punch(&key) { "PUNCH" } else { "PECK-RECOIL" };
                    self.zz_last_style =
                        Some((style.to_string(), style.to_string(), spec.hand.to_string()));
                    let h = Self::touch_hand_idx(spec.hand).unwrap_or(1);
                    self.man_last_prep = Some(h);
                    self.zz_last_cmd[h] = style.to_string();
                    if self.cfg.zz_two_proc {
                        if let Some(h) = Self::touch_hand_idx(spec.hand) {
                            self.man_proc[h] = true;
                        }
                    } else {
                        self.man_proc[0] = true;
                        self.man_proc[1] = true;
                    }
                }
                let prep_s = feats as f32 * self.cfg.feat_prep_ms as f32 / 1000.0;
                let prep = (self.randomize_secs(prep_s) * 1000.0).round() as i64;
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
                        format!(
                            "PREPARATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix()
                        ),
                    );
                }
                self.man_prep_busy = false;
                // `punch` (`motor.lisp:938`/`944`): exec = init + key-closure,
                // finish = init + burst + burst (no Fitts term).
                // `peck-recoil`: exec = init + max(burst, randomize(move)),
                // finish = exec + burst + max(burst, randomize(move)), with
                // move = max(burst, fitts(r)) (`motor.lisp:1019`/`1028`).
                let (exec_ms, finish_ms) = if key_is_punch(&key) {
                    (
                        self.cfg.init_ms + self.cfg.key_closure_ms,
                        self.cfg.init_ms + self.cfg.burst_ms + self.cfg.burst_ms,
                    )
                } else {
                    let spec = Self::key_spec(&key)?;
                    let burst_s = self.cfg.burst_ms as f32 / 1000.0;
                    let fit = (self.cfg.peck_fitts * (spec.r + 0.5).log2())
                        .max(self.cfg.min_fitts_ms as f32 / 1000.0);
                    let move_s = burst_s.max(fit);
                    let exec_s = self.cfg.init_ms as f32 / 1000.0
                        + burst_s.max(self.randomize_secs(move_s));
                    let finish_s =
                        exec_s + burst_s + burst_s.max(self.randomize_secs(move_s));
                    // Lisp rounds seconds->ms at scheduling time.
                    (
                        (exec_s * 1000.0).round() as i64,
                        (finish_s * 1000.0).round() as i64,
                    )
                };
                if self.cfg.zz_ext {
                    // Queue for the hand's execution stage (`maybe-execute-movement`).
                    let (class, hand_name) = self
                        .zz_last_style
                        .clone()
                        .map(|x| (x.0, x.2))
                        .unwrap_or_else(|| ("PECK-RECOIL".to_string(), "RIGHT".to_string()));
                    let hand = Self::touch_hand_idx(&hand_name).unwrap_or(1);
                    self.man_exec_queue.push(ZzPending {
                        key,
                        req_ms,
                        hand,
                        both: false,
                        style: class,
                        exec_ms,
                        finish_ms,
                        kind: ZzKind::Press,
                    });
                    self.zz_maybe_execute();
                } else {
                    self.man_exec_busy = true;
                    let t = self.time_ms;
                    self.push(
                        t + self.cfg.init_ms,
                        Prio::Num(0),
                        Out::Always,
                        Event::InitiationComplete {
                            req_ms,
                            hand: 1,
                            both: false,
                            style: String::new(),
                        },
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
                        Event::FinishMovement {
                            req_ms,
                            hand: 1,
                            both: false,
                            style: String::new(),
                        },
                    );
                }
            }
            Event::InitiationComplete {
                req_ms,
                hand,
                both,
                style,
            } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "INITIATION-COMPLETE {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_hand(&style, hand, both)
                        ),
                    );
                }
                if self.cfg.zz_ext {
                    // ACT-R frees the hand of the *last prepared* movement
                    // (`motor-extension.lisp:1008`), which can differ from the
                    // movement whose initiation just completed.
                    if let Some(h) = self.man_last_prep {
                        if self.cfg.zz_two_proc {
                            self.man_proc[h] = false;
                        } else {
                            self.man_proc[0] = false;
                            self.man_proc[1] = false;
                        }
                    }
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
                if self.tracker_task {
                    // `response` monitor (`simple.lisp:29`): prompt correct/
                    // wrong, then re-prompt `choose` 5 s later.
                    let prompt = if key == self.tracker_target {
                        "CORRECT"
                    } else {
                        "WRONG"
                    };
                    self.tracker_set_prompt(prompt);
                    self.push(
                        self.time_ms + 5000,
                        Prio::Num(0),
                        Out::Never,
                        Event::Choose,
                    );
                }
                if self.cfg.zz_ext {
                    self.zz_data
                        .push(("PRESSED".to_string(), self.time_ms, key.clone()));
                    // `keyboard-with-release` (`motor-extension.lisp:612`):
                    // a `release-key` signal `key-release-time` later.
                    self.push(
                        self.time_ms + self.cfg.zz_krt_ms,
                        Prio::Num(0),
                        Out::Always,
                        Event::ReleaseKey { key: key.clone() },
                    );
                }
                if self.clear_on_keypress {
                    // `clear-exp-window` from the task's output-key handler:
                    // drop the visicon and re-display. The re-encode of the
                    // attended location happens in `proc_display`
                    // (`update-attended-loc`), which also sees the cleared
                    // window because the visicon is dropped here first.
                    self.pending_clears.push(self.time_ms);
                    self.features.clear();
                    self.now_m(
                        Prio::Num(-100),
                        Out::Always,
                        true,
                        Event::ProcDisplay,
                    );
                }
            }
            Event::ReleaseKey { key } => {
                if show {
                    self.emit(
                        "KEYBOARD",
                        format!("release-key {} {}", self.model.name, key.to_lowercase()),
                    );
                }
                if self.cfg.zz_ext {
                    self.zz_data
                        .push(("RELEASED".to_string(), self.time_ms, key.clone()));
                }
            }
            Event::FinishMovement {
                req_ms,
                hand,
                both,
                style,
            } => {
                if show {
                    self.emit(
                        "MOTOR",
                        format!(
                            "FINISH-MOVEMENT {}{}",
                            Self::secs_str(req_ms),
                            self.zz_style_suffix_hand(&style, hand, both)
                        ),
                    );
                }
                if self.cfg.zz_ext {
                    if self.cfg.zz_two_exec && !both {
                        self.man_exec[hand] = false;
                    } else {
                        self.man_exec[0] = false;
                        self.man_exec[1] = false;
                    }
                    // `finish-movement-dual` (`motor-extension.lisp:1205`)
                    // frees the movement's fingers *before*
                    // `maybe-execute-movement` starts the next movement on the
                    // hand, so the free must precede the re-dispatch.
                    if both {
                        for h in 0..2 {
                            self.zz_hands[h].busy = [false; 5];
                        }
                    } else {
                        self.zz_hands[hand].busy = [false; 5];
                    }
                    self.zz_maybe_execute();
                } else {
                    self.man_exec_busy = false;
                }
            }
            Event::Command { name, params } => {
                if show {
                    let suffix = if params.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " {}",
                            params
                                .iter()
                                .map(Value::show)
                                .collect::<Vec<_>>()
                                .join(" ")
                        )
                    };
                    self.emit("NONE", format!("{name}{suffix}"));
                }
                self.run_command(&name, params)?;
            }
        }
        Ok(())
    }

    /// Run a scheduled task command: bind its `:params` to the `defun`'s
    /// parameters and execute the body (side-effecting forms handled by
    /// [`Engine::m_eval`]).
    fn run_command(&mut self, name: &str, params: Vec<Value>) -> Result<(), EngineError> {
        let key = name.to_uppercase();
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
    fn m_eval(&mut self, e: &Sexp, b: &Bindings) -> Result<Value, String> {
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
                    "ACT-R-RANDOM" => {
                        let lim =
                            self.m_eval(items.get(1).ok_or("act-r-random needs a limit")?, b)?;
                        return match lim {
                            Value::Int(n) if n > 0 => Ok(Value::Int(
                                self.rng.borrow_mut().random_int(n as u64) as i64,
                            )),
                            Value::Int(_) => Ok(Value::Int(0)),
                            Value::Num(x) => Ok(Value::Num(
                                self.rng.borrow_mut().random_float(x as f32) as f64
                            )),
                            other => {
                                Err(format!("act-r-random needs a number, got {}", other.show()))
                            }
                        };
                    }
                    "GOAL-FOCUS" | "GOAL-FOCUS-FCT" => {
                        let desc = self
                            .m_chunk_desc(items.get(1).ok_or("goal-focus needs an argument")?)?;
                        self.goal_focus(&desc).map_err(|e| e.to_string())?;
                        return Ok(Value::Nil);
                    }
                    "SET-BUFFER-CHUNK" => {
                        let buffer = self
                            .m_eval(items.get(1).ok_or("set-buffer-chunk needs a buffer")?, b)?;
                        let chunk = self
                            .m_eval(items.get(2).ok_or("set-buffer-chunk needs a chunk")?, b)?;
                        self.set_buffer_chunk_cmd(&buffer, &chunk)?;
                        return Ok(Value::Nil);
                    }
                    // The task's `move-it` is scheduled by the model's
                    // `!eval! (schedule-event-relative … 'move-it)`. Its
                    // effect on the window is replayed from the captured
                    // device op (`MODIFY-TEXT`), so the body is a no-op here
                    // (the arguments reference task globals the engine does
                    // not model).
                    "INCF" | "MODIFY-TEXT-FOR-EXP-WINDOW" => {
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
                    "SCHEDULE-EVENT-RELATIVE" | "SCHEDULE-EVENT" => {
                        let relative = h.eq_ignore_ascii_case("schedule-event-relative");
                        let at = self
                            .m_eval(items.get(1).ok_or("schedule-event needs a time")?, b)?;
                        let at_secs = as_num(&at)
                            .ok_or_else(|| "schedule-event time is not a number".to_string())?;
                        let name_v = self
                            .m_eval(items.get(2).ok_or("schedule-event needs a command")?, b)?;
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
                        self.push(
                            at_ms,
                            Prio::Num(0),
                            Out::Medium,
                            Event::Command { name, params },
                        );
                        return Ok(Value::Nil);
                    }
                    _ => {}
                }
            }
        }
        self.eval_expr(e, b)
    }

    /// A `goal-focus` argument: a chunk spec list, a symbol, or a string.
    fn m_chunk_desc(&self, e: &Sexp) -> Result<ChunkDesc, String> {
        match e {
            Sexp::List(spec) => crate::model::parse_chunk_desc(spec).map_err(|err| err.to_string()),
            Sexp::Sym(n) | Sexp::Str(n) => Ok(ChunkDesc::Named(n.clone())),
            other => Err(format!("bad goal-focus argument {other:?}")),
        }
    }

    /// `set-buffer-chunk <buffer> <(slot value ...)>`: replace the buffer's
    /// chunk silently (no module trace at the tested detail), using the
    /// buffer's `declare-buffer-usage` type for the chunk.
    fn set_buffer_chunk_cmd(&mut self, buffer: &Value, chunk: &Value) -> Result<(), String> {
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
    fn vocal_request(&mut self, spec: NewChunk) -> Result<(), EngineError> {
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
        let is_speak = ctype.eq_ignore_ascii_case("SPEAK")
            || slots.iter().any(|(s, v)| {
                s == "CMD" && matches!(v, Value::Sym(x) if x.eq_ignore_ascii_case("SPEAK"))
            });
        if is_speak {
            if let Some(t) = text {
                // The `speak` action is scheduled at default priority so the
                // request's implicit clear-buffer precedes it.
                self.now(Prio::Num(0), Out::Medium, Event::Speak { text: t });
            }
        }
        Ok(())
    }

    /// `+temporal>` request (`modules/temporal.lisp:186`): `isa clear`
    /// stops the timer; `isa time` (re)starts it, placing `(isa time
    /// ticks 0)` and arming the first tick.
    fn temporal_request(&mut self, spec: NewChunk) -> Result<(), EngineError> {
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
        let tick = (start as f64
            + self.rng.borrow_mut().noise(self.cfg.time_noise * start) as f64)
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
    fn temporal_tick(&mut self, show: bool) -> Result<(), EngineError> {
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
                    None => c.slots.push(("TICKS".to_string(), Value::Int(ticks as i64))),
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
    fn secs_str(req_ms: i64) -> String {
        lisp_s_f32(req_ms as f32 / 1000.0)
    }

    /// Virtual keyboard geometry for press-key movements (hand/finger/r/theta
    /// per key name as requested; output uses the table's lowercase name).
    /// M5a seeds Q; digits join with device-task goldens.
    fn key_spec(key: &str) -> Result<KeySpec, EngineError> {
        key_table()
            .get(&key.to_uppercase())
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
            // `conflict-resolution` skips disabled productions
            // (`procedural.lisp:2195`).
            if self.disabled_prods.contains(&pname.to_uppercase()) {
                continue;
            }
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
                    u += self.rng.borrow_mut().noise(self.cfg.egs);
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
                let i = self.rng.borrow_mut().random_int(tied.len() as u64) as usize;
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
                            .map(|b| b.chunk.is_none() && !b.failed)
                            .unwrap_or(false),
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
        // Buffer matches/queries run first (they bind the variables that
        // LHS `!eval!`/`!bind!` conditions may use; ACT-R orders these by
        // dependency, `production-parsing.lisp`).
        for item in &prod.lhs {
            if matches!(item, LhsItem::Match { .. } | LhsItem::Query { .. })
                && self.match_item(item, &mut b, ppm)?.is_none()
            {
                return Ok(None);
            }
        }
        for item in &prod.lhs {
            if matches!(
                item,
                LhsItem::Eval { .. } | LhsItem::Bind { .. } | LhsItem::MvBind { .. }
            ) && self.match_item(item, &mut b, ppm)?.is_none()
            {
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

    /// ACT-R per-chunk LHS tests (`test-var-slot`, slot ops).
    fn match_chunk_tests(
        &self,
        chunk: &ChunkInst,
        buffer: &str,
        tests: &[crate::model::SlotTest],
        only_isa: bool,
        b: &mut Bindings,
        ppm: &mut Vec<PpmMismatch>,
    ) -> Result<bool, EngineError> {
        for t in tests {
            if t.slot == "ISA" {
                if only_isa {
                    if let Pattern::Sym(want) = &t.value {
                        if want != "CHUNK" && &chunk.ctype != want {
                            return Ok(false);
                        }
                    }
                }
                continue;
            }
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
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Threads extra: the goal chunk a production matches (the first in the
    /// goal set satisfying its `=goal>` conditions), for the matched-chunk
    /// clear.
    fn threaded_goal_match(&self, prod: &Production) -> Option<String> {
        for item in &prod.lhs {
            if let LhsItem::Match { buffer, tests } = item {
                if buffer == "GOAL" {
                    let only_isa = tests.iter().all(|t| t.slot == "ISA");
                    for c in &self.goal_set {
                        let mut b = Bindings::new();
                        let mut ppm = Vec::new();
                        if self
                            .match_chunk_tests(c, buffer, tests, only_isa, &mut b, &mut ppm)
                            .unwrap_or(false)
                        {
                            return c.name.clone();
                        }
                    }
                }
            }
        }
        None
    }

    fn match_item(
        &self,
        item: &LhsItem,
        b: &mut Bindings,
        ppm: &mut Vec<PpmMismatch>,
    ) -> Result<Option<()>, EngineError> {
        match item {
            LhsItem::Match { buffer, tests } => {
                // A LHS `ISA TYPE` as the *only* test constrains the buffer
                // chunk's type: a request-built `(ISA CHUNK)` chunk must not
                // satisfy `ISA GOAL` (bind-test). With other slot tests the
                // type is carried for slot indices only, so the tests
                // themselves decide (blending/addition).
                let only_isa = tests.iter().all(|t| t.slot == "ISA");
                // Threaded goal: search the goal set in order for the first
                // chunk satisfying the tests.
                if buffer == "GOAL" && self.cfg.threads {
                    for chunk in &self.goal_set {
                        let mut b2 = b.clone();
                        let mut ppm2 = ppm.clone();
                        if self.match_chunk_tests(
                            chunk, buffer, tests, only_isa, &mut b2, &mut ppm2,
                        )? {
                            *b = b2;
                            *ppm = ppm2;
                            if let Some(name) = &chunk.name {
                                b.insert(format!("={buffer}"), Value::Sym(name.clone()));
                            }
                            return Ok(Some(()));
                        }
                    }
                    return Ok(None);
                }
                let buf = self
                    .buffers
                    .get(buffer)
                    .ok_or_else(|| bad(format!("unknown buffer ={buffer}>")))?;
                let chunk = match &buf.chunk {
                    Some(c) => c,
                    None => return Ok(None),
                };
                if !self.match_chunk_tests(chunk, buffer, tests, only_isa, b, ppm)? {
                    return Ok(None);
                }
                // `buffer-name->variable` (`support/production-parsing.lisp:481`):
                // a `=buffer>` match binds `=BUFFER` to the chunk it held, which
                // production compilation uses for drop-out naming.
                if let Some(name) = &chunk.name {
                    b.insert(format!("={buffer}"), Value::Sym(name.clone()));
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
                            let b = self
                                .buffers
                                .get(buffer)
                                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?;
                            b.chunk.is_none() && !b.failed
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
                        // extended-motor-actions adds per-stage queries.
                        ("PROCESSOR", Pattern::Sym(s)) if s == "FREE" => {
                            self.query_processor_free(buffer)?
                        }
                        ("PROCESSOR", Pattern::Sym(s)) if s == "BUSY" => {
                            !self.query_processor_free(buffer)?
                        }
                        ("PREPARATION", Pattern::Sym(s)) if s == "FREE" => {
                            self.query_prep_free(buffer)?
                        }
                        ("PREPARATION", Pattern::Sym(s)) if s == "BUSY" => {
                            !self.query_prep_free(buffer)?
                        }
                        // `?manual-left> index free/down` etc.
                        (attr, Pattern::Sym(s))
                            if matches!(
                                attr,
                                "INDEX" | "MIDDLE" | "RING" | "PINKIE" | "THUMB"
                            ) =>
                        {
                            match s.as_str() {
                                "FREE" => self.query_finger_free(buffer, attr)?,
                                "BUSY" => !self.query_finger_free(buffer, attr)?,
                                "DOWN" => self.query_finger_down(buffer, attr)?,
                                _ => {
                                    return Err(unimpl(format!("query ?{buffer}> {attr} {val:?}")))
                                }
                            }
                        }
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
            LhsItem::Eval { value, safe } => match self.eval_expr(value, b) {
                Ok(v) => {
                    if v == Value::Nil {
                        return Ok(None);
                    }
                }
                Err(e) => {
                    if !*safe {
                        return Err(unimpl(format!("!EVAL! {e}")));
                    }
                    return Ok(None);
                }
            },
            LhsItem::Bind { var, value, safe } => match self.eval_expr(value, b) {
                Ok(v) => {
                    b.insert(var.clone(), v);
                }
                Err(e) => {
                    if !*safe {
                        return Err(unimpl(format!("!BIND! {e}")));
                    }
                    b.insert(var.clone(), Value::Nil);
                }
            },
            LhsItem::MvBind { vars, value } => {
                let vals = self
                    .eval_values(value, b)
                    .map_err(|e| unimpl(format!("!MV-BIND! {e}")))?;
                for (i, var) in vars.iter().enumerate() {
                    b.insert(var.clone(), vals.get(i).cloned().unwrap_or(Value::Nil));
                }
            }
        }
        Ok(Some(()))
    }

    /// LHS/RHS `!eval!`/`!bind!` expressions, evaluated against bindings.
    fn eval_expr(&self, e: &Sexp, b: &Bindings) -> Result<Value, String> {
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

    fn eval_symbol(&self, s: &str, b: &Bindings) -> Result<Value, String> {
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
        // it is literal data (e.g. a quoted symbol argument).
        if let Some(v) = b.get(s) {
            return Ok(v.clone());
        }
        Ok(Value::Sym(s.to_string()))
    }

    fn quote_value(&self, e: Option<&Sexp>) -> Result<Value, String> {
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
    fn eval_values(&self, e: &Sexp, b: &Bindings) -> Result<Vec<Value>, String> {
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
    fn user_fn_for_head(&self, head: &Sexp) -> Option<&crate::model::UserDefun> {
        let name = match head {
            Sexp::Sym(s) | Sexp::Str(s) => s,
            _ => return None,
        };
        self.user_fns.get(&name.to_uppercase())
    }

    /// Call a task-file `defun`: evaluate args in the caller's scope, bind
    /// parameters, and evaluate the body (last expression may be `values`).
    fn user_fn_values(
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

    fn eval_call(&self, head: &Sexp, args: &[Sexp], b: &Bindings) -> Result<Value, String> {
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
                    Value::Int(n) if n > 0 => Ok(Value::Int(
                        self.rng.borrow_mut().random_int(n as u64) as i64
                    )),
                    Value::Int(_) => Ok(Value::Int(0)),
                    Value::Num(x) => {
                        Ok(Value::Num(self.rng.borrow_mut().random_float(x as f32) as f64))
                    }
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
            other => Err(format!("unsupported eval function {other}")),
        }
    }

    /// Remote-command call by name (a string head). No commands are
    /// registered yet, so these fail loudly rather than silently.
    fn eval_command(&self, name: &str, _args: &[Sexp], _b: &Bindings) -> Result<Value, String> {
        // A registered task command (`add-act-r-command`): its side effects
        // (e.g. recording a response in the task's own variable) are not
        // part of the trace, so a no-op matches.
        if self.user_fns.contains_key(&name.to_uppercase()) {
            return Ok(Value::Nil);
        }
        Err(format!("unsupported eval command \"{name}\""))
    }

    fn eval_num(&self, args: &[Sexp], b: &Bindings) -> Result<f64, String> {
        let e = args.first().ok_or("number argument expected")?;
        let v = self.eval_expr(e, b)?;
        as_num(&v).ok_or_else(|| format!("not a number: {}", v.show()))
    }

    fn eval_nums(&self, args: &[Sexp], b: &Bindings) -> Result<Vec<f64>, String> {
        let mut out = Vec::new();
        for e in args {
            let v = self.eval_expr(e, b)?;
            out.push(as_num(&v).ok_or_else(|| format!("not a number: {}", v.show()))?);
        }
        Ok(out)
    }

    /// Preserve integer-ness when a numeric result is integral.
    fn num_value(&self, x: f64) -> Value {
        if x.fract() == 0.0 && x.abs() < 9e15 {
            Value::Int(x as i64)
        } else {
            Value::Num(x)
        }
    }

    /// `randomize-time-ms` (`framework/random.lisp:541`): with
    /// `:randomize-time` enabled, `time - round(time/scale) + act-r-random(1
    /// + 2·round(time/scale))`.
    fn randomize_ms(&mut self, time_ms: i64) -> i64 {
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
    fn randomize_secs(&mut self, secs: f32) -> f32 {
        match self.cfg.randomize_scale {
            None => secs,
            Some(_) if secs == 0.0 => secs,
            Some(scale) => {
                (secs / scale) * (scale - 1.0 + self.rng.borrow_mut().random_float(2.0))
            }
        }
    }

    fn chunk_slot_value(&self, chunk: &Value, slot: &Value) -> Result<Value, String> {
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
                .or_else(|| self.named.get(name)),
            Value::Nil => return Ok(Value::Nil),
            _ => return Err("chunk-slot-value-fct needs a chunk".to_string()),
        };
        Ok(c.map(|c| c.get(&slot)).unwrap_or(Value::Nil))
    }

    /// The printed value of `slot` on the chunk named `chunk` (following a
    /// copy back to its source), or `None` when the chunk or slot is absent.
    /// Used by `MAP-RETRIEVAL-BUFFER` to instantiate p2's condition
    /// variables from the retrieved chunk.
    fn chunk_name_slot(&self, chunk: &str, slot: &str) -> Option<String> {
        let name = self.copy_source.get(chunk).map(|s| s.as_str()).unwrap_or(chunk);
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
        match buffer {            "VISUAL" => Ok(!self.visual_busy),
            "MANUAL" => {
                if self.cfg.zz_ext {
                    Ok(!self.man_prep_busy
                        && !self.man_proc[0]
                        && !self.man_proc[1]
                        && !self.man_exec[0]
                        && !self.man_exec[1])
                } else {
                    Ok(!self.man_prep_busy && !self.man_exec_busy)
                }
            }
            "IMAGINAL" => Ok(!self.imag_busy),
            "TRACKER" => Ok(true),
            "VOCAL" => Ok(!self.vocal_busy),
            "BLENDING" => Ok(!self.blending_busy),
            _ => Ok(!self
                .buffers
                .get(buffer)
                .ok_or_else(|| bad(format!("unknown buffer ?{buffer}>")))?
                .busy),
        }
    }

    /// `style X hand Y` suffix for motor events when the extended-motor-actions
    /// module is loaded (`motor-extension.lisp:979`).
    fn zz_style_suffix(&self) -> String {
        if !self.cfg.zz_ext {
            return String::new();
        }
        match &self.zz_last_style {
            Some((class, _, hand)) => format!(" style {class} hand {hand}"),
            None => String::new(),
        }
    }

    /// `style X hand Y` suffix from an explicit `(style, hand)`.
    fn zz_style_suffix_for(&self, style: &str, hand: usize) -> String {
        self.zz_style_suffix_hand(style, hand, false)
    }

    /// `style X hand Y` suffix, with `BOTH` for the two-handed styles
    /// (`all-fingers-to-home`).
    fn zz_style_suffix_hand(&self, style: &str, hand: usize, both: bool) -> String {
        if !self.cfg.zz_ext {
            return String::new();
        }
        let h = if both {
            "BOTH"
        } else if hand == 0 {
            "LEFT"
        } else {
            "RIGHT"
        };
        format!(" style {style} hand {h}")
    }

    /// `?manual> processor free/busy` (extended-motor-actions).
    fn query_processor_free(&self, buffer: &str) -> Result<bool, EngineError> {        match buffer {
            "MANUAL" => Ok(!self.man_proc[0] && !self.man_proc[1]),
            _ => self.query_free(buffer),
        }
    }

    /// `?manual> preparation free/busy` (extended-motor-actions).
    fn query_prep_free(&self, buffer: &str) -> Result<bool, EngineError> {
        match buffer {
            "MANUAL" => Ok(!self.man_prep_busy),
            _ => self.query_free(buffer),
        }
    }

    /// `?manual-left> index free` (`motor-extension.lisp:1306`): whether a
    /// hand's finger is free (not busy with a movement).
    fn query_finger_free(&self, buffer: &str, finger: &str) -> Result<bool, EngineError> {
        let hand = match buffer {
            "MANUAL-LEFT" => 0usize,
            "MANUAL-RIGHT" => 1,
            _ => return self.query_free(buffer),
        };
        let f = Self::touch_finger_idx(finger)
            .ok_or_else(|| bad(format!("unknown finger {finger}")))?;
        Ok(!self.zz_hands[hand].busy[f])
    }

    /// `?manual-left> index down`: whether a hand's finger is held down.
    fn query_finger_down(&self, buffer: &str, finger: &str) -> Result<bool, EngineError> {
        let hand = match buffer {
            "MANUAL-LEFT" => 0usize,
            "MANUAL-RIGHT" => 1,
            _ => return self.query_free(buffer),
        };
        let f = Self::touch_finger_idx(finger)
            .ok_or_else(|| bad(format!("unknown finger {finger}")))?;
        Ok(self.zz_hands[hand].down[f])
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
        // RHS `!bind!`/`!mv-bind!` run first (they may be referenced by
        // later actions), then `!eval!`, then `!output!`. ACT-R reorders
        // these by variable dependencies (`production-parsing.lisp:1879`).
        let mut b = bindings.clone();
        for action in &prod.rhs {
            if let RhsAction::Bind { var, value, safe } = action {
                match self.eval_expr(value, &b) {
                    Ok(v) => {
                        b.insert(var.clone(), v);
                    }
                    Err(e) => {
                        if !*safe {
                            return Err(unimpl(format!("!BIND! {e}")));
                        }
                        b.insert(var.clone(), Value::Nil);
                    }
                }
            }
        }
        for action in &prod.rhs {
            if let RhsAction::MvBind { vars, value } = action {
                let vals = self
                    .eval_values(value, &b)
                    .map_err(|e| unimpl(format!("!MV-BIND! {e}")))?;
                for (i, var) in vars.iter().enumerate() {
                    b.insert(var.clone(), vals.get(i).cloned().unwrap_or(Value::Nil));
                }
            }
        }
        for action in &prod.rhs {
            if let RhsAction::Eval { value, safe } = action {
                // `(buffer-chunk <buffer> …)` runs in source order (after
                // `!output!`); see the action loop below.
                if let Sexp::List(items) = value {
                    if items.first().and_then(|x| x.as_sym()) == Some("BUFFER-CHUNK") {
                        continue;
                    }
                }
                if let Err(e) = self.m_eval(value, &b) {
                    if !*safe {
                        return Err(unimpl(format!("!EVAL! {e}")));
                    }
                }
            }
        }
        for action in &prod.rhs {
            match action {
                RhsAction::Output { args } => {
                    let mut text = String::new();
                    for a in args {
                        text.push_str(&self.show_output_arg(a, &b)?);
                        text.push(' ');
                    }
                    self.out_lines.push(text);
                }
                RhsAction::Other { head, .. } => {
                    // `!stop!` ends the run with a BREAK-EVENT.
                    if head == "!STOP!" {
                        self.stopped = true;
                    } else {
                        return Err(unimpl(format!("RHS {head} needs a Lisp evaluator")));
                    }
                }
                _ => {}
            }
        }
        if self.stopped {
            return Ok(());
        }
        // Whole-chunk requests resolved this fire (`+retrieval> =goal`),
        // used by production compilation for indirect actions.
        let mut reqs: Vec<(String, String)> = Vec::new();
        // Explicit actions in RHS order, then implicit harvest clears.
        for action in &prod.rhs {
            match action {
                RhsAction::Modify { buffer, slots } => {
                    // An empty `=buffer>` (no slots) is a no-op: ACT-R
                    // schedules no MOD event for it (e.g. `merging-test` P2).
                    if slots.is_empty() {
                        continue;
                    }
                    // A `=buffer>` modify stores chunk-valued refs by
                    // reference unless the ref is an unnamed spec chunk
                    // (`paired-2x2`), matching `instantiate-chunk-spec`.
                    let copy = matches!(buffer.as_str(), "GOAL" | "IMAGINAL");
                    let mut sub = Vec::new();
                    for (s, p) in slots {
                        sub.push((s.clone(), self.rhs_value(p, &b, copy, &mut copied)?));
                    }
                    self.now(
                        Prio::Num(100),
                        explicit_out(self.cfg.rhst),
                        Event::ModBuffer {
                            buffer: buffer.clone(),
                            slots: sub,
                            module: "PROCEDURAL".to_string(),
                        },
                    );
                }
                RhsAction::ModifyRequest { buffer, slots } => {
                    // `*buffer>` (`buffer-mod-request`): request a change from
                    // the buffer's module, which goes busy and applies it
                    // after its delay (the imaginal module's 200 ms).
                    if slots.is_empty() {
                        continue;
                    }
                    let copy = matches!(buffer.as_str(), "GOAL" | "IMAGINAL");
                    let mut sub = Vec::new();
                    for (s, p) in slots {
                        sub.push((s.clone(), self.rhs_value(p, &b, copy, &mut copied)?));
                    }
                    if buffer == "IMAGINAL" {
                        self.imag_busy = true;
                        let delay = if self.cfg.vidt {
                            self.randomize_ms(self.cfg.imaginal_delay_ms)
                        } else {
                            self.cfg.imaginal_delay_ms
                        };
                        self.push(
                            self.time_ms + delay,
                            Prio::Num(0),
                            Out::Low,
                            Event::ModBuffer {
                                buffer: buffer.clone(),
                                slots: sub,
                                module: buffer_module(buffer),
                            },
                        );
                    } else {
                        self.now(
                            Prio::Num(100),
                            Out::Medium,
                            Event::ModBuffer {
                                buffer: buffer.clone(),
                                slots: sub,
                                module: buffer_module(buffer),
                            },
                        );
                    }
                }
                RhsAction::Request { buffer, spec } => {
                    let copy = matches!(buffer.as_str(), "GOAL" | "IMAGINAL");
                    let concrete = self.concretize(spec, &b, copy, &mut copied)?;
                    if let NewChunk::Named(n) = &concrete {
                        reqs.push((buffer.clone(), n.clone()));
                    }
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
                            trace: true,
                        },
                    );
                }
                RhsAction::Clear { buffer } => {
                    self.now(
                        Prio::Num(10),
                        Out::Medium,
                        Event::ClearBuffer {
                            buffer: buffer.clone(),
                            module: "PROCEDURAL".to_string(),
                        },
                    );
                }
                RhsAction::DirectSet { buffer, spec } => {
                    // `@buffer> chunk` (`production-parsing.lisp:1147`):
                    // overwrite the buffer now (priority 90).
                    let copy = matches!(buffer.as_str(), "GOAL" | "IMAGINAL");
                    let concrete = self.concretize(spec, &b, copy, &mut copied)?;
                    self.now(
                        Prio::Num(90),
                        explicit_out(self.cfg.rhst),
                        Event::OverwriteBuffer {
                            buffer: buffer.clone(),
                            chunk: concrete,
                            requested: false,
                            shown: None,
                            module: "PROCEDURAL".to_string(),
                        },
                    );
                }
                RhsAction::DirectErase { buffer } => {
                    // `@buffer>` (`production-parsing.lisp:1140`): erase-buffer.
                    self.now(
                        Prio::Num(90),
                        explicit_out(self.cfg.rhst),
                        Event::EraseBuffer {
                            buffer: buffer.clone(),
                            module: "PROCEDURAL".to_string(),
                        },
                    );
                }
                // `(buffer-chunk <buffer> …)`: a model-body `!eval!` prints
                // the command-layer buffer dump (`tracking-test`'s `report`
                // production) in source order; buffer names are symbols.
                RhsAction::Eval { value, .. } => {
                    if let Sexp::List(items) = value {
                        if items.first().and_then(|x| x.as_sym()) == Some("BUFFER-CHUNK") {
                            let names: Vec<String> = items[1..]
                                .iter()
                                .filter_map(|a| a.as_sym().map(|s| s.to_string()))
                                .collect();
                            self.print_buffer_chunk(&names);
                        }
                    }
                }
                RhsAction::Output { .. }
                | RhsAction::Other { .. }
                | RhsAction::Bind { .. }
                | RhsAction::MvBind { .. } => {}
            }
        }
        for buffer in &self.exec_prods[prod_idx].implicit_clears.clone() {
            if buffer == "GOAL" && self.cfg.threads {
                if let Some(name) = self.threaded_goal_match(&prod) {
                    self.now(Prio::Num(10), Out::Medium, Event::ClearGoalThread { name });
                    continue;
                }
            }
            self.now(
                Prio::Num(10),
                Out::Medium,
                Event::ClearBuffer {
                    buffer: buffer.clone(),
                    module: "PROCEDURAL".to_string(),
                },
            );
        }
        self.now(Prio::Min, Out::Medium, Event::ConflictResolution);
        // `production-fired` runs actions, then compiles
        // (`production-compilation.lisp`, called from `production-fired`).
        self.compile_productions(prod_idx, &b, &reqs);
        Ok(())
    }

    /// First reason a production is invalid for compilation
    /// (`invalid-compilation-reason`, production-compilation.lisp:1243).
    /// `None` means valid. With `:cia` off, any indirect action invalidates.
    fn invalid_compilation_reason(&self, idx: usize) -> Option<String> {
        let prod = &self.model.productions[idx];
        if let Some(buf) = indirect_action_buffer_name(prod) {
            if !self.cfg.cia {
                return Some(format!(
                    "it has an indirect action with the {buf} buffer"
                ));
            }
            // With :cia, an indirect action whose variable is bound by a
            // `!bind!`/`!safe-bind!` is invalid (`:rir` ignores the retrieval
            // style).
            let ivars = indirect_action_vars(prod, self.cfg.rir);
            let bvars = production_bind_vars(prod);
            if ivars.iter().any(|v| bvars.contains(v)) {
                return Some(
                    "it has an indirect action using a variable bound with a !safe-bind!"
                        .to_string(),
                );
            }
        }
        None
    }

    /// `valid-1st-p` (`production-compilation.lisp:1315`): a production can
    /// become the "previous" only if it has no `!stop!` and no duplicate
    /// buffer action of the same kind.
    fn valid_first_production(&self, idx: usize) -> bool {
        let prod = &self.model.productions[idx];
        for a in &prod.rhs {
            match a {
                RhsAction::Other { head, .. } if head == "!STOP!" => return false,
                _ => {}
            }
        }
        let mut seen: Vec<(char, &str)> = Vec::new();
        for a in &prod.rhs {
            let key = match a {
                RhsAction::Modify { buffer, .. } => Some(('=', buffer.as_str())),
                RhsAction::ModifyRequest { buffer, .. } => Some(('=', buffer.as_str())),
                RhsAction::Request { buffer, .. } => Some(('+', buffer.as_str())),
                _ => None,
            };
            if let Some(k) = key {
                if seen.contains(&k) {
                    return false;
                }
                seen.push(k);
            }
        }
        true
    }

    /// `invalid-1st-p-reason` (`production-compilation.lisp:1326`).
    fn invalid_first_reason(&self, idx: usize) -> String {
        let prod = &self.model.productions[idx];
        for a in &prod.rhs {
            if let RhsAction::Other { head, .. } = a {
                if head == "!STOP!" {
                    return "it has a !stop! action".to_string();
                }
            }
        }
        let mut seen: Vec<(char, String)> = Vec::new();
        for a in &prod.rhs {
            let kv = match a {
                RhsAction::Modify { buffer, .. } | RhsAction::ModifyRequest { buffer, .. } => {
                    Some(('=', buffer.clone()))
                }
                RhsAction::Request { buffer, .. } => Some(('+', buffer.clone())),
                _ => None,
            };
            if let Some(k) = kv {
                if seen.contains(&k) {
                    return match k.0 {
                        '=' => format!(
                            "it has multiple modification actions for the {} buffer",
                            k.1
                        ),
                        _ => format!(
                            "it has multiple requests to the {} buffer",
                            k.1
                        ),
                    };
                }
                seen.push(k);
            }
        }
        "of unknown reason. (This should not happen. Please report this issue.)".to_string()
    }

    /// A buffer both productions make a `+` request to: composition is
    /// refused (`Buffer X prevents composition … both make requests`).
    /// `determine-composable` (`production-compilation.lisp:1375`): the first
    /// buffer, in `remove-duplicates` order (the last occurrence wins), whose
    /// `(p1-usage, p2-usage)` pair is not in its compilation type's table,
    /// with the type's `whynot-reason` string.
    fn determine_composable(&self, i1: usize, i2: usize) -> Option<(String, String)> {
        let p1 = &self.model.productions[i1];
        let p2 = &self.model.productions[i2];
        let mut all: Vec<String> = lhs_buffers(p1);
        all.extend(lhs_buffers(p2));
        all.extend(rhs_buffers(p1));
        all.extend(rhs_buffers(p2));
        let buffers: Vec<String> = all
            .iter()
            .enumerate()
            .filter(|(i, b)| !all[i + 1..].contains(b))
            .map(|(_, b)| b.clone())
            .collect();
        for buffer in &buffers {
            let u1 = buffer_usage(p1, buffer);
            let u2 = buffer_usage(p2, buffer);
            let (table, reason): (&[(u32, u32)], fn(u32, u32) -> String) =
                match buffer.as_str() {
                    "RETRIEVAL" => (RETRIEVAL_COMPOSE, retrieval_reason),
                    "GOAL" => (GOAL_COMPOSE, goal_reason),
                    "VISUAL-LOCATION" | "VISUAL" | "AURAL-LOCATION" | "AURAL" | "TEMPORAL" => {
                        (PERCEPTUAL_COMPOSE, perceptual_reason)
                    }
                    "IMAGINAL" => (IMAGINAL_COMPOSE, imaginal_reason),
                    _ => (MOTOR_COMPOSE, motor_reason),
                };
            if !table.contains(&(u1, u2)) {
                return Some((buffer.clone(), reason(u1, u2)));
            }
        }
        None
    }

    /// Install a composed production into the runtime set (parse its text
    /// and add it with utility 0). Compiled productions with contradictory
    /// conditions simply never match.
    fn install_compiled(
        &mut self,
        name: &str,
        lhs: &[(String, Vec<(String, SlotVal)>)],
        rhs: &[(String, Vec<(String, String)>)],
    ) {
        let mut text = format!("(P {name}");
        for (token, slots) in lhs {
            text.push_str(&format!(" {token}"));
            for (n, v) in slots {
                text.push_str(&format!(" {n} {}", v.render()));
            }
        }
        text.push_str(" ==>");
        for (token, slots) in rhs {
            text.push_str(&format!(" {token}"));
            for (n, v) in slots {
                text.push_str(&format!(" {n} {v}"));
            }
        }
        text.push(')');
        if let Ok(prod) = crate::model::parse_production_text(&text) {
            self.model.productions.push(prod);
            self.utilities.push(0.0);
            let idx = self.model.productions.len() - 1;
            let p = self.model.productions[idx].clone();
            self.exec_prods.push(ExecProd {
                index: idx,
                implicit_clears: implicit_clears(&p, &self.cfg.do_not_harvest, self.cfg.threads),
                implicit_queries: implicit_queries(&p, &self.cfg.do_not_query),
            });
        }
    }

    /// `compile-productions` (`production-compilation.lisp:1129`): compose
    /// the previous fired production with the current one and emit the
    /// `:epl`/`:pct` trace. This port covers the goal-style test+modify
    /// composition exercised by `ppm-compilation`; compiled productions are
    /// not (yet) added to the runtime set.
    fn compile_productions(&mut self, idx: usize, bindings: &Bindings, reqs: &[(String, String)]) {
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
        // `valid-compilation-production` (`production-compilation.lisp:1188`).
        if let Some(reason) = self.invalid_compilation_reason(idx) {
            if trace {
                self.out(format!(
                    "  Production {p2name} is not valid for compilation"
                ));
                self.out(format!("   because {reason}"));
            }
            self.prev_prod = None;
            return;
        }
        if let Some((p1_idx, p1_bindings, p1_reqs)) = self.prev_prod.clone() {
            let p1name = self.model.productions[p1_idx].name.clone();
            if self.time_ms - self.prev_prod_ms > self.cfg.tt_ms {
                if trace {
                    self.out(format!(
                        "  Cannot compile {p1name} and {p2name} because the time between them exceeds the threshold time."
                    ));
                }
            } else if let Some((buf, reason)) = self.determine_composable(p1_idx, idx) {
                if trace {
                    self.out(format!(
                        "  Buffer {buf} prevents composition of these productions"
                    ));
                    self.out(format!("   because {reason}"));
                    self.out(format!(
                        "  Production {p1name} and {p2name} cannot be composed."
                    ));
                }
            } else {
                let (retain1, retain2) = if self.cfg.cia && self.cfg.rir {
                    compute_retain_buffers(
                        &self.model.productions[p1_idx],
                        &self.model.productions[idx],
                    )
                } else {
                    (Vec::new(), Vec::new())
                };
                let reason = indirect_safe_bind_reason(&self.model.productions[p1_idx], &retain1)
                    .or_else(|| {
                        indirect_safe_bind_reason(&self.model.productions[idx], &retain2)
                    });
                if let Some(reason) = reason {
                    if trace {
                        self.out(format!(
                            "  Production {p1name} and {p2name} cannot be composed"
                        ));
                        self.out(format!("   because {reason}"));
                    }
                } else {
                let (lhs, rhs, _compose_dropped) = self.compose_pair(
                    p1_idx,
                    &p1_bindings,
                    &p1_reqs,
                    idx,
                    bindings,
                    &retain1,
                    &retain2,
                );
                let cname = format!("PRODUCTION{}", self.compiled_count);
                // `check-for-duplicate-productions`
                // (`production-compilation.lisp:1915`): a composed production
                // equivalent to an existing one is not installed.
                let dup = {
                    let crep = canon_composed(&lhs, &rhs);
                    self.model.productions.iter().enumerate().find_map(|(i, p)| {
                        if canon_prod(p) == crep {
                            Some((p.name.clone(), i >= self.base_prods))
                        } else {
                            None
                        }
                    })
                };
                if trace {
                    self.out(format!(
                        "  Production {p1name} and {p2name} are being composed."
                    ));
                    if let Some((dname, compiled)) = &dup {
                        if *compiled {
                            self.out(format!("  Recreating production {dname}"));
                        } else {
                            self.out(format!(
                                "  Result is production {dname}\n  No actions taken."
                            ));
                        }
                    } else {
                    self.out("  New production:\n".to_string());
                    // `dropped-chunks` (`production-compilation.lisp:1723`):
                    // for each `=` buffer of p2 of a drop-out type that p1
                    // requested, the chunk p2's buffer variable held (the
                    // copy's source if it was one).
                    let mut dropped: Vec<(String, String)> = Vec::new();
                    for x in lhs_buffers(&self.model.productions[idx]) {
                        if x != "RETRIEVAL" {
                            continue;
                        }
                        if buffer_usage(&self.model.productions[p1_idx], &x) & 4 == 0 {
                            continue;
                        }
                        if let Some(v) = bindings.get(&format!("={x}")) {
                            let name = v.show();
                            let name = self.copy_source.get(&name).cloned().unwrap_or(name);
                            dropped.push((x, name));
                        }
                    }
                    dropped.sort_by(|a, b| a.0.cmp(&b.0));
                    let doc = if dropped.is_empty() {
                        format!("{p1name} & {p2name}")
                    } else {
                        let vals: Vec<String> =
                            dropped.iter().map(|(_, n)| n.clone()).collect();
                        format!("{p1name} & {p2name} - {}", vals.join(" - "))
                    };
                    // `sort-conditions` (`production-compilation.lisp:1903`)
                    // plus the assembly at 1741: `!`-conditions are collected
                    // and prepended (in reverse `push` order), then the rest
                    // sorted by token.
                    let mut bangs: Vec<(String, Vec<(String, SlotVal)>)> = lhs
                        .iter()
                        .filter(|(t, _)| t.starts_with('!'))
                        .cloned()
                        .collect();
                    bangs.sort_by(|a, b| sort_token(&a.0).cmp(&sort_token(&b.0)));
                    bangs.reverse();
                    let mut rest: Vec<(String, Vec<(String, SlotVal)>)> = lhs
                        .iter()
                        .filter(|(t, _)| !t.starts_with('!'))
                        .cloned()
                        .collect();
                    rest.sort_by(|a, b| sort_token(&a.0).cmp(&sort_token(&b.0)));
                    let sorted: Vec<(String, Vec<(String, SlotVal)>)> =
                        bangs.into_iter().chain(rest).collect();
                    self.out(render_compiled(&cname, &doc, &sorted, &rhs));
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
                    if self.cfg.esc && self.cfg.ul {
                        // `production-parameter-fct`
                        // (`utility-and-reward-1.lisp:686`): the reward
                        // lines print only with `:esc` + `:ul`.
                        self.out(" :reward    NIL".to_string());
                        self.out(" :fixed-utility    NIL".to_string());
                    }
                    }
                }
                if dup.is_none() {
                    self.install_compiled(&cname, &lhs, &rhs);
                    self.compiled_count += 1;
                }
                }
            }
        } else if trace {
            self.out("  No previous production to compose with.".to_string());
        }
        // `handle-check-valid-1st-p` (`production-compilation.lisp:1338`).
        if self.valid_first_production(idx) {
            self.prev_prod = Some((idx, bindings.clone(), reqs.to_vec()));
            self.prev_prod_ms = self.time_ms;
            if trace {
                let name = self.model.productions[idx].name.clone();
                self.out(format!("  Setting previous production to {name}."));
            }
        } else {
            if trace {
                self.out(format!(
                    "  Production {p2name} is not valid as a first production for compilation"
                ));
                self.out(format!("   because {}", self.invalid_first_reason(idx)));
            }
            self.prev_prod = None;
        }
    }

    /// Compose two productions per buffer (`buffer-condition-union`,
    /// production-compilation.lisp:2320): the first's conditions, then the
    /// second's conditions minus the slots the first modifies, with
    /// `remove-duplicates` keeping the later copy; the actions are the
    /// first's untouched slots followed by the second's.
    #[allow(clippy::type_complexity)]
    #[allow(clippy::type_complexity)]
    fn compose_pair(
        &self,
        i1: usize,
        b1: &Bindings,
        reqs1: &[(String, String)],
        i2: usize,
        b2: &Bindings,
        retain1: &[String],
        retain2: &[String],
    ) -> (
        Vec<(String, Vec<(String, SlotVal)>)>,
        Vec<(String, Vec<(String, String)>)>,
        Vec<String>,
    ) {
        let p1 = &self.model.productions[i1];
        let p2 = &self.model.productions[i2];
        let p1_lhs = prod_lhs_slots(p1, b1, &self.model.chunk_types);
        let p2_lhs = prod_lhs_slots(p2, b2, &self.model.chunk_types);
        let p1_acts = prod_rhs_actions(p1, b1);
        let p2_acts = prod_rhs_actions(p2, b2);

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
        // `MAP-RETRIEVAL-BUFFER` (`retrieval-compilation.lisp:71`): p1
        // requests the buffer (4/12/20/28) and p2 matches it (8/12/24/28).
        // The retrieved chunk's slots instantiate p1's request-action
        // variables and p2's condition variables, and the buffer variable
        // maps to the chunk itself.
        let u1r = buffer_usage(p1, "RETRIEVAL");
        let u2r = buffer_usage(p2, "RETRIEVAL");
        if matches!(u1r, 4 | 12 | 20 | 28) && matches!(u2r, 8 | 12 | 24 | 28) {
            if let Some(chunk) = b2.get("=RETRIEVAL").map(|v| v.show()) {
                map.insert("RETRIEVAL".to_string(), SlotVal::Lit(chunk.clone()));
                for (op, buf, slots, _) in &p1_acts {
                    if buf == "RETRIEVAL" && (*op == '+' || *op == 'r') {
                        for (_, v) in slots {
                            if let SlotVal::Var(n) = v {
                                if let Some(val) = instantiate_var(n, b1) {
                                    map.insert(n.clone(), SlotVal::Lit(val));
                                }
                            }
                        }
                    }
                }
                if let Some((_, slots2)) = p2_lhs.iter().find(|(b, _)| b == "RETRIEVAL") {
                    for (name, v) in slots2 {
                        if let SlotVal::Var(n) = v {
                            if let Some(cv) = self.chunk_name_slot(&chunk, name) {
                                map.insert(n.clone(), SlotVal::Lit(cv));
                            }
                        }
                    }
                }
            }
        }
        // `MAP-IMAGINAL-BUFFER`'s RHS `=` to LHS `=` case
        // (`imaginal-compilation.lisp:212`): p1 modified the imaginal buffer
        // and p2 matches it. Slots common to p1's (LHS ∪ RHS) and p2's LHS
        // map by value — a p2 constant wins, otherwise p2's variable maps to
        // p1's value.
        let u1i = buffer_usage(p1, "IMAGINAL");
        let u2i = buffer_usage(p2, "IMAGINAL");
        if matches!(u1i, 9 | 25) && matches!(u2i, 8 | 9 | 12 | 13 | 24 | 25 | 28 | 29) {
            let p1_lhs_i = p1_lhs
                .iter()
                .find(|(b, _)| b == "IMAGINAL")
                .map(|(_, s)| s.clone())
                .unwrap_or_default();
            let p1_rhs_i: Vec<(String, SlotVal)> = p1_acts
                .iter()
                .filter(|(op, b, _, _)| *op == '=' && b == "IMAGINAL")
                .flat_map(|(_, _, s, _)| s.clone())
                .collect();
            let rhs_names: Vec<String> = p1_rhs_i.iter().map(|(n, _)| n.clone()).collect();
            let mut p1_slots: Vec<(String, SlotVal)> = p1_lhs_i
                .into_iter()
                .filter(|(n, _)| !rhs_names.contains(n))
                .collect();
            p1_slots.extend(p1_rhs_i);
            if let Some((_, p2_slots)) = p2_lhs.iter().find(|(b, _)| b == "IMAGINAL") {
                for (n1, v1) in &p1_slots {
                    if let Some((_, v2)) = p2_slots.iter().find(|(n, _)| n == n1) {
                        match (v1, v2) {
                            (SlotVal::Var(a), SlotVal::Lit(_)) => {
                                if let Some(val) = instantiate_var(a, b1) {
                                    map.insert(a.clone(), SlotVal::Lit(val));
                                }
                            }
                            (_, SlotVal::Var(c)) => {
                                map.insert(c.clone(), v1.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        // Only variables referenced by an *indirect* request (`+buffer> =c`,
        // not `+goal> =retrieval`) are resolved from the firings; other
        // variables stay symbolic (compiled productions are not otherwise
        // specialized — cf. `ppm-compilation`). With `:rir`, a retained
        // retrieval request keeps its variable.
        let collect = |acts: &[(char, String, Vec<(String, SlotVal)>, Option<String>)],
                       retain: &[String]| {
            let mut s: HashSet<String> = HashSet::new();
            for (op, buf, _slots, var) in acts {
                if *op == 'r' {
                    if let Some(v) = var {
                        if !is_buffer_name(v) && !retain.contains(buf) {
                            s.insert(v.clone());
                        }
                    }
                }
            }
            s
        };
        let indirect_vars = {
            let mut s = collect(&p1_acts, retain1);
            s.extend(collect(&p2_acts, retain2));
            s
        };
        // `retrieval-buffers-map` (`production-compilation.lisp:297`): the
        // drop-out buffer variable (`=RETRIEVAL`) is instantiated to the
        // chunk it held when the first production requested that buffer;
        // other buffer variables (`=VISUAL-LOCATION`, ...) stay symbolic.
        let drop_retrieval = p1_acts
            .iter()
            .any(|(op, buf, _, _)| (*op == '+' || *op == 'r') && buf == "RETRIEVAL");
        // A variable not unified across the pair, but resolved by an
        // indirect action, is instantiated from the firing bindings.
        let subst = |v: &SlotVal, b: &Bindings, ivars: &HashSet<String>| -> SlotVal {
            match v {
                SlotVal::Var(n) => {
                    if ivars.contains(n) && b.contains_key(n) {
                        SlotVal::Lit(b[n].show())
                    } else if let Some(m) = map.get(n) {
                        m.clone()
                    } else if drop_retrieval && n == "RETRIEVAL" {
                        self.buffers
                            .get(n)
                            .and_then(|buf| buf.chunk.as_ref())
                            .and_then(|c| c.name.clone())
                            .map(SlotVal::Lit)
                            .unwrap_or_else(|| SlotVal::Var(n.clone()))
                    } else {
                        SlotVal::Var(n.clone())
                    }
                }
                SlotVal::Lit(s) => SlotVal::Lit(s.clone()),
            }
        };
        let apply_lhs = |lhs: &[(String, Vec<(String, SlotVal)>)],
                         b: &Bindings,
                         ivars: &HashSet<String>| {
            lhs.iter()
                .map(|(buf, s)| {
                    (
                        buf.clone(),
                        s.iter()
                            .map(|(n, v)| (n.clone(), subst(v, b, ivars)))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let p1_lhs = apply_lhs(&p1_lhs, b1, &indirect_vars);
        let p2_lhs = apply_lhs(&p2_lhs, b2, &indirect_vars);
        let inst_acts = |acts: &[(char, String, Vec<(String, SlotVal)>, Option<String>)],
                         b: &Bindings,
                         ivars: &HashSet<String>| {
            acts.iter()
                .map(|(op, buf, slots, var)| {
                    (
                        *op,
                        buf.clone(),
                        slots
                            .iter()
                            .map(|(n, v)| (n.clone(), subst(v, b, ivars)))
                            .collect::<Vec<_>>(),
                        var.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let p1_acts = inst_acts(&p1_acts, b1, &indirect_vars);
        let p2_acts = inst_acts(&p2_acts, b2, &indirect_vars);

        let mut buffers: Vec<String> = Vec::new();
        let add_buf = |buffers: &mut Vec<String>, b: &str| {
            if !buffers.iter().any(|x| x == b) {
                buffers.push(b.to_string());
            }
        };
        for (b, _) in p1_lhs.iter().chain(&p2_lhs) {
            if !b.starts_with('?') && !b.starts_with('!') {
                add_buf(&mut buffers, b);
            }
        }
        for (_, b, _, _) in p1_acts.iter().chain(&p2_acts) {
            add_buf(&mut buffers, b);
        }
        buffers.sort();

        let lhs_for = |v: &[(String, Vec<(String, SlotVal)>)], b: &str| -> Vec<(String, SlotVal)> {
            v.iter()
                .find(|(x, _)| x == b)
                .map(|(_, s)| s.clone())
                .unwrap_or_default()
        };
        let acts_for =
            |v: &[(char, String, Vec<(String, SlotVal)>, Option<String>)],
             b: &str,
             ops: &[char]|
             -> Vec<(Vec<(String, SlotVal)>, Option<String>)> {
                v.iter()
                    .filter(|(op, bb, _, _)| bb == b && ops.contains(op))
                    .map(|(_, _, s, var)| (s.clone(), var.clone()))
                    .collect()
            };

        let mut lhs = Vec::new();
        for buf in &buffers {
            let c1 = lhs_for(&p1_lhs, buf);
            let c2 = lhs_for(&p2_lhs, buf);
            let mut modified: Vec<String> = Vec::new();
            for (s, _) in acts_for(&p1_acts, buf, &['=', '*']) {
                for (n, _) in s {
                    modified.push(n);
                }
            }
            let p1_req = p1_acts
                .iter()
                .any(|(op, bb, _, _)| bb == buf && (*op == '+' || *op == 'r'));
            if p1_req {
                // The request supplies the buffer; drop the second's tests,
                // but keep p1's own condition even if it has no slots
                // (`=retrieval>`).
                if p1_lhs.iter().any(|(b, _)| b == buf) {
                    lhs.push((format!("={buf}>"), remove_dup_keep_last(c1)));
                }
            } else {
                let mut slots = c1;
                for (n, v) in c2 {
                    if !modified.contains(&n) {
                        slots.push((n, v));
                    }
                }
                let had = p1_lhs.iter().any(|(b, _)| b == buf)
                    || p2_lhs.iter().any(|(b, _)| b == buf);
                if had {
                    lhs.push((format!("={buf}>"), remove_dup_keep_last(slots)));
                }
            }
        }
        // Query conditions (`?buffer> state free`) follow the matches.
        for (b, s) in p1_lhs.iter().chain(&p2_lhs) {
            if b.starts_with('?') && !lhs.iter().any(|(t, _)| t == b) {
                lhs.push((b.clone(), remove_dup_keep_last(s.clone())));
            }
        }
        // `!bind!`/`!safe-bind!` conditions are prepended
        // (`post-conditions`, production-compilation.lisp:1742).
        let mut binds: Vec<(String, Vec<(String, SlotVal)>)> = Vec::new();
        for (b, s) in p1_lhs.iter().chain(&p2_lhs) {
            if b.starts_with('!') {
                binds.push((b.clone(), s.clone()));
            }
        }
        if !binds.is_empty() {
            binds.extend(lhs);
            lhs = binds;
        }

        let mut rhs: Vec<(String, Vec<(String, String)>)> = Vec::new();
        let mut dropped: Vec<String> = Vec::new();
        for buf in &buffers {
            let p2_tests = p2_lhs.iter().any(|(b, _)| b == buf);
            let sv = |s: Vec<(String, SlotVal)>| -> Vec<(String, String)> {
                s.iter().map(|(n, v)| (n.clone(), v.render())).collect()
            };
            // `=` modify union (second overrides). A null `=buffer>` modify
            // is kept (`COMPOSE-GOAL-BUFFER`'s `(when a2= (list a2=))`), so a
            // self-composition like `G2 & G2` matches `G2`.
            let has_eq1 = !acts_for(&p1_acts, buf, &['=']).is_empty();
            let has_eq2 = !acts_for(&p2_acts, buf, &['=']).is_empty();
            let eq1: Vec<_> = acts_for(&p1_acts, buf, &['='])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            let eq2: Vec<_> = acts_for(&p2_acts, buf, &['='])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            if has_eq1 || has_eq2 {
                let names2: Vec<String> = eq2.iter().map(|(n, _)| n.clone()).collect();
                let mut slots: Vec<(String, SlotVal)> =
                    eq1.into_iter().filter(|(n, _)| !names2.contains(n)).collect();
                slots.extend(eq2);
                rhs.push((format!("={buf}>"), sv(slots)));
            }
            // `*` modify-request union.
            let st1: Vec<_> = acts_for(&p1_acts, buf, &['*'])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            let st2: Vec<_> = acts_for(&p2_acts, buf, &['*'])
                .into_iter()
                .flat_map(|(s, _)| s)
                .collect();
            if !st1.is_empty() || !st2.is_empty() {
                let names2: Vec<String> = st2.iter().map(|(n, _)| n.clone()).collect();
                let mut slots: Vec<(String, SlotVal)> =
                    st1.into_iter().filter(|(n, _)| !names2.contains(n)).collect();
                slots.extend(st2);
                rhs.push((format!("*{buf}>"), sv(slots)));
            }
            // `+` requests: p1's is dropped when p2 tests the buffer (and
            // the requested chunk is noted); p2's indirect ones are
            // resolved from p1's requests and expanded to slots.
            // p1 requests: kept unless p2 tests the buffer; an indirect
            // request (`+retrieval> =c`) expands to the referenced chunk's
            // spec.
            let p2_reqs_buf = p2_acts
                .iter()
                .any(|(op, bb, _, _)| bb == buf && (*op == '+' || *op == 'r'));
            for (s, var) in acts_for(&p1_acts, buf, &['+', 'r']) {
                if p2_tests {
                    if reqs1.iter().any(|(bb, _)| bb == buf) {
                        // The dropped name is the chunk p1's request actually
                        // produced (e.g. the retrieved `G`, not the requested
                        // `red`), i.e. the buffer's current content.
                        let name = self
                            .buffers
                            .get(buf)
                            .and_then(|b| b.chunk.as_ref())
                            .and_then(|c| c.name.clone())
                            .map(|n| self.copy_source.get(&n).cloned().unwrap_or(n))
                            .or_else(|| {
                                reqs1.iter().find(|(bb, _)| bb == buf).map(|(_, n)| n.clone())
                            });
                        if let Some(n) = name {
                            dropped.push(n);
                        }
                    }
                } else if p2_reqs_buf {
                    // Both productions request the buffer; p1's request is
                    // superseded by p2's (no dropped-chunk note).
                } else if retain1.contains(buf) {
                    // `:rir`: keep the indirect retrieval request as its
                    // variable (`+RETRIEVAL> =C`), or its instantiated value
                    // when the variable is also being resolved.
                    if let Some(v) = &var {
                        let shown = if indirect_vars.contains(v) && b1.contains_key(v) {
                            b1[v].show()
                        } else {
                            format!("={v}")
                        };
                        rhs.push((format!("+{buf}> {shown}"), Vec::new()));
                    }
                } else {
                    let slots = match &var {
                        Some(v) => self
                            .binding_chunk_name(v, b1)
                            .map(|n| self.chunk_spec_slots(&n))
                            .unwrap_or_default(),
                        None => sv(s),
                    };
                    rhs.push((format!("+{buf}>"), slots));
                }
            }
            // p2 requests.
            for (s, var) in acts_for(&p2_acts, buf, &['+', 'r']) {
                if retain2.contains(buf) {
                    if let Some(v) = &var {
                        let shown = if indirect_vars.contains(v) && b2.contains_key(v) {
                            b2[v].show()
                        } else {
                            format!("={v}")
                        };
                        rhs.push((format!("+{buf}> {shown}"), Vec::new()));
                        continue;
                    }
                }
                if let Some(v) = var {
                    let slots = if let Some(chunk) = reqs1
                        .iter()
                        .find(|(bb, _)| bb == &v || bb == buf)
                        .map(|(_, c)| c.clone())
                    {
                        self.chunk_spec_slots(&chunk)
                    } else {
                        self.binding_chunk_name(&v, b2)
                            .map(|n| self.chunk_spec_slots(&n))
                            .unwrap_or_default()
                    };
                    rhs.push((format!("+{buf}>"), slots));
                } else {
                    rhs.push((format!("+{buf}>"), sv(s)));
                }
            }
        }
        // `sort-actions`: descending by token (op+buffer), stable.
        rhs.sort_by(|a, b| b.0.cmp(&a.0));
        (lhs, rhs, dropped)
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

    /// The chunk name bound to a variable (for an indirect request).
    fn binding_chunk_name(&self, var: &str, b: &Bindings) -> Option<String> {
        match b.get(var) {
            Some(Value::Sym(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// `chunk-name-to-chunk-spec`: a chunk's slots; for an undefined chunk
    /// (a bare-symbol slot value like `red`), the slot(s) that reference it
    /// (`add-dm`'s `get-true-slot-value-name` gives the chunk that slot).
    fn chunk_spec_slots(&self, name: &str) -> Vec<(String, String)> {
        if let Some(c) = self
            .dm
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .or_else(|| self.named.get(name))
        {
            return c.slots.iter().map(|(k, v)| (k.clone(), v.show())).collect();
        }
        let mut out = Vec::new();
        for c in self.dm.iter() {
            for (k, v) in &c.slots {
                if let Value::Sym(s) = v {
                    if s == name && !out.contains(&(k.clone(), name.to_string())) {
                        out.push((k.clone(), name.to_string()));
                    }
                }
            }
        }
        out
    }

    /// For a whole-chunk request (`+goal> =retrieval`), the trace shows the
    /// referenced chunk's slots (`(COLOR RED VALUE 3)`), sorted by name.
    fn named_chunk_shown(&self, spec: &NewChunk) -> Option<String> {
        if let NewChunk::Named(n) = spec {
            if let Some(c) = self
                .dm
                .iter()
                .find(|c| c.name.as_deref() == Some(n.as_str()))
                .or_else(|| self.named.get(n.as_str()))
            {
                let mut slots: Vec<(String, String)> =
                    c.slots.iter().map(|(k, v)| (k.clone(), v.show())).collect();
                slots.sort_by(|a, b| a.0.cmp(&b.0));
                let mut parts = Vec::new();
                for (k, v) in slots {
                    parts.push(k);
                    parts.push(v);
                }
                return Some(format!("({})", parts.join(" ")));
            }
        }
        None
    }

    /// `get-chunk`: a chunk by name from DM, the model's named chunks, or a
    /// buffer (a `reuse?` copy like `GOAL-CHUNK0`, `buffers.lisp:1349`).
    fn find_chunk_by_name(&self, name: &str) -> Option<&ChunkInst> {
        self.dm
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .or_else(|| self.named.get(name))
            .or_else(|| {
                self.buffers
                    .values()
                    .filter_map(|b| b.chunk.as_ref())
                    .find(|c| c.name.as_deref() == Some(name))
            })
    }

    fn start_retrieval(&mut self, spec: NewChunk, var_slots: &[String]) -> Result<(), EngineError> {
        // A whole-chunk request (`+retrieval> =goal`) expands the referenced
        // chunk to its slot spec (`chunk-name-to-chunk-spec`) and retrieves
        // by matching; a name that is not a chunk (e.g. `=c` bound to a
        // bare-symbol slot value like `red`) expands to an unconstrained
        // request matching any chunk.
        if let NewChunk::Named(n) = &spec {
            let chunk = self.find_chunk_by_name(n);
            let expanded = match chunk {
                Some(c) => NewChunk::Spec {
                    ctype: String::new(),
                    slots: c.slots.clone(),
                    neg: Vec::new(),
                    rels: Vec::new(),
                    bare: true,
                },
                None => NewChunk::Spec {
                    ctype: String::new(),
                    slots: Vec::new(),
                    neg: Vec::new(),
                    rels: Vec::new(),
                    bare: true,
                },
            };
            return self.start_retrieval(expanded, var_slots);
        }
        // A spec whose "type" names a DM chunk (e.g. `(b :mp-value 2)`)
        // expands to that chunk's slots plus the request parameters
        // (`define-chunk-spec-fct` with a chunk name, chunk-spec.lisp:437).
        if let NewChunk::Spec {
            ctype,
            slots,
            neg,
            rels,
            ..
        } = &spec
        {
            if !ctype.is_empty() {
                if let Some(chunk) = self
                    .dm
                    .iter()
                    .find(|c| c.name.as_deref() == Some(ctype.as_str()))
                {
                    let mut merged = chunk.slots.clone();
                    merged.extend(slots.clone());
                    let expanded = NewChunk::Spec {
                        ctype: String::new(),
                        slots: merged,
                        neg: neg.clone(),
                        rels: rels.clone(),
                        bare: true,
                    };
                    return self.start_retrieval(expanded, var_slots);
                }
            }
        }
        let (mut want_slots, want_neg) = match &spec {
            NewChunk::Named(_) => unreachable!(),
            NewChunk::Instance(_) => {
                return Err(bad(
                    "retrieval request cannot be a buffer instance".to_string(),
                ));
            }
            NewChunk::Spec { slots, neg, .. } => (slots.clone(), neg.clone()),
        };
        // `isa TYPE` (`define-chunk-spec-fct`, chunk-spec.lisp:441): the
        // chunk-type's default slot tests are added for any slot not already
        // tested (`process-slots-specs`, chunk-spec.lisp:597) — e.g.
        // `isa response` on `(chunk-type response key (is-response t))`
        // requests `is-response = t`.
        if let NewChunk::Spec { ctype, .. } = &spec {
            if !ctype.is_empty() {
                let defaults = self
                    .model
                    .chunk_types
                    .iter()
                    .find(|t| t.name.eq_ignore_ascii_case(ctype))
                    .map(|t| t.defaults.clone())
                    .unwrap_or_default();
                for (slot, pat) in defaults {
                    if !want_slots
                        .iter()
                        .any(|(s, _)| s.eq_ignore_ascii_case(&slot))
                    {
                        want_slots.push((slot.to_uppercase(), Value::from_pattern(&pat)?));
                    }
                }
            }
        }
        // `:mp-value` request parameter (`+retrieval> isa test :mp-value 3`)
        // overrides `:mp` for this request; `nil` disables partial matching.
        let mp_req = want_slots
            .iter()
            .find(|(s, _)| s == ":MP-VALUE")
            .map(|(_, v)| v.clone());
        let mp = match &mp_req {
            Some(Value::Nil) => None,
            Some(Value::Num(x)) => Some(*x as f32),
            Some(Value::Int(i)) => Some(*i as f32),
            _ => self.cfg.mp,
        };
        // Validate the `:mp-value` parameter (`declarative-memory.lisp`):
        // an invalid value or a repeat aborts the request with a warning.
        if want_slots.iter().filter(|(s, _)| s == ":MP-VALUE").count() > 1 {
            self.emit_warning("Invalid retrieval request.");
            self.emit_warning(":mp-value parameter used more than once.");
            self.dm_busy = false;
            self.buffers.get_mut("RETRIEVAL").unwrap().busy = false;
            return Ok(());
        }
        if let Some(v) = &mp_req {
            if !matches!(v, Value::Nil | Value::Num(_) | Value::Int(_)) {
                self.emit_warning("Invalid retrieval request.");
                self.emit_warning(":mp-value parameter's value can only be nil or a number.");
                self.dm_busy = false;
                self.buffers.get_mut("RETRIEVAL").unwrap().busy = false;
                return Ok(());
            }
        }
        // `:rt-value` request parameter (`declarative-memory.lisp:1292`)
        // overrides `:rt` for this request; the value must be a number.
        let rt = match want_slots
            .iter()
            .find(|(s, _)| s == ":RT-VALUE")
            .map(|(_, v)| v.clone())
        {
            Some(Value::Num(x)) => x as f32,
            Some(Value::Int(i)) => i as f32,
            _ => self.cfg.rt,
        };
        // `:bll-value`/`:mas-value`/`:ans-value` (`declarative-memory.lisp:1243`):
        // nil disables that term for this request.
        let bll = match want_slots
            .iter()
            .find(|(s, _)| s == ":BLL-VALUE")
            .map(|(_, v)| v.clone())
        {
            Some(Value::Nil) => None,
            Some(Value::Num(x)) => Some(x as f32),
            Some(Value::Int(i)) => Some(i as f32),
            _ => self.cfg.bll,
        };
        let mas = match want_slots
            .iter()
            .find(|(s, _)| s == ":MAS-VALUE")
            .map(|(_, v)| v.clone())
        {
            Some(Value::Nil) => None,
            Some(Value::Num(x)) => Some(x as f32),
            Some(Value::Int(i)) => Some(i as f32),
            _ => self.cfg.mas,
        };
        let ans = match want_slots
            .iter()
            .find(|(s, _)| s == ":ANS-VALUE")
            .map(|(_, v)| v.clone())
        {
            Some(Value::Nil) => None,
            Some(Value::Num(x)) => Some(x as f32),
            Some(Value::Int(i)) => Some(i as f32),
            _ => self.cfg.ans,
        };
        // Partial matching (mp set, esc on): candidacy is the request's
        // slot signature — required-filled and required-empty slot names —
        // NOT the chunk type (`slots-vector-match-signature`,
        // framework/chunk-spec.lisp:895). `=`/`-` values score activation
        // instead of filtering. Otherwise exact matching as before.
        let partial = self.cfg.esc && mp.is_some();
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
        // `:recently-retrieved nil` removes the marked candidates
        // (`declarative-memory.lisp:1192`), tracing the header followed by
        // each (DM order, newest chunk first).
        let mut recent_removed: Vec<String> = Vec::new();
        if let Some(rv) = &recent_req {
            if matches!(rv, Value::Nil) {
                for idx in (0..self.dm.len()).rev() {
                    let c = &self.dm[idx];
                    let cname = c.name.clone().unwrap_or_default();
                    let sig_ok = req_filled.iter().all(|s| c.get(s) != Value::Nil)
                        && req_empty.iter().all(|s| c.get(s) == Value::Nil);
                    if sig_ok && self.is_recent(&cname) {
                        recent_removed.push(cname);
                    }
                }
                if self.act_high() {
                    self.out("Removing recently retrieved chunks:".to_string());
                    for n in &recent_removed {
                        self.out(n.clone());
                    }
                }
            }
        }
        let mut best_a: Option<f32> = None;
        let mut tied: Vec<ChunkInst> = Vec::new();
        // `chunk-list` (the DM hash-table order) is reverse-insertion.
        let order: Vec<usize> = (0..self.dm.len()).rev().collect();
        // Pass 1 — candidacy: ACT-R prints every match/no-match line before
        // any activation line (`declarative-memory.lisp:1314`).
        let mut candidates: Vec<usize> = Vec::new();
        for idx in order {
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
            candidates.push(idx);
        }
        // Pass 2 — activation. Exact matching keeps `chunk-list` order;
        // partial matching uses `(nreverse chunk-list)`
        // (`declarative-memory.lisp:1349`).
        let act_order: Vec<usize> = if partial {
            candidates.iter().rev().cloned().collect()
        } else {
            candidates.clone()
        };
        for idx in act_order {
            let c = self.dm[idx].clone();
            let cname = c.name.clone().unwrap_or_default();
            if self.act_medium() {
                self.out(format!("Computing activation for chunk {cname}"));
                self.out("Computing base-level".to_string());
            }
            // A = base-level + spreading + partial matching + activation
            // noise. Base-level from the decay equation when :bll is set,
            // else blc. Noise drawn per candidate in DM order.
            let base = match bll {
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
                        if self.act_medium() {
                            self.out(format!("Starting with blc: {}", lisp_f(self.cfg.blc)));
                            let refs_str: Vec<String> =
                                refs.iter().map(|r| fmt_secs(*r)).collect();
                            self.out(format!(
                                "Computing base-level from {count} references ({})",
                                refs_str.join(" ")
                            ));
                            self.out(format!(
                                "  creation time: {} decay: {}  Optimized-learning: {}",
                                fmt_secs(creation),
                                lisp_f(bll),
                                ol_str(&self.cfg.ol)
                            ));
                        }
                        let value = bl_equation(
                            self.time_ms,
                            &refs,
                            count as f32,
                            creation,
                            bll,
                            &self.cfg.ol,
                            self.cfg.act_scale,
                        );
                        if self.act_medium() {
                            self.out(format!("base-level value: {}", lisp_f(value)));
                        }
                        self.cfg.blc + value
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
                self.spreading_activation(&cname, mas)
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
            if let Some(mp) = mp {
                let show = self.act_medium();
                if show {
                    self.out("Computing partial matching component".to_string());
                }
                let mut total = 0.0f32;
                for (s, v) in &want_slots {
                    if s.starts_with(':') || *v == Value::Nil {
                        continue;
                    }
                    let cv = c.get(s);
                    let sim = self.similarity(v, &cv);
                    let simd = mp * sim;
                    if show {
                        self.out(format!("  comparing slot {s}"));
                        self.out(format!(
                            "  Requested: = {}  Chunk's slot value: {}",
                            v.show(),
                            cv.show()
                        ));
                        self.out(format!("  similarity: {}", lisp_f(sim)));
                        self.out(format!("  effective similarity value is {}", lisp_f(simd)));
                    }
                    total += simd;
                    a += simd;
                }
                for (s, v) in &want_neg {
                    if s.starts_with(':') || *v == Value::Nil {
                        continue;
                    }
                    let cv = c.get(s);
                    let sim = self.similarity(v, &cv);
                    if show {
                        self.out(format!("  comparing slot {s}"));
                        self.out(format!(
                            "  Requested: - {}  Chunk's slot value: {}",
                            v.show(),
                            cv.show()
                        ));
                        self.out(format!("  similarity: {}", lisp_f(sim)));
                    }
                    if sim == self.cfg.ms {
                        let simd = mp * self.cfg.md;
                        if show {
                            self.out(format!(
                                "  effective similarity value is {}",
                                lisp_f(simd)
                            ));
                        }
                        total += simd;
                        a += simd;
                    } else if show {
                        self.out("  negation test with similarity not ms has no effect".to_string());
                    }
                }
                if show {
                    self.out(format!("Total similarity score {}", lisp_f(total)));
                }
            }
            let noise = match ans {
                Some(s) => self.rng.borrow_mut().noise(s),
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
                let i = self.rng.borrow_mut().random_int(tied.len() as u64) as usize;
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
        let act = best_a.unwrap_or(0.0);
        let below = self.cfg.esc && act < rt;
        if self.act_low() {
            match &best {
                Some(_) if below => {
                    self.out(format!(
                        "No chunk above the retrieval threshold: {}",
                        lisp_f(rt)
                    ));
                }
                Some(w) => {
                    let wname = w.name.clone().unwrap_or_default();
                    self.out(format!(
                        "Chunk {wname} with activation {} is the best",
                        lisp_f(act)
                    ));
                }
                None => {
                    self.out("No matching chunk found retrieval failure".to_string());
                }
            }
        }
        let gen = self.dm_gen;
        match best {
            None => {
                let wait = failure_wait(&self.cfg, rt);
                let at = self.time_ms + wait;
                self.push(at, Prio::Num(0), Out::Low, Event::RetrievalFailure { gen });
            }
            Some(_) if self.cfg.esc && act < rt => {
                let wait = failure_wait(&self.cfg, rt);
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
            self.spreading_activation(&name, self.cfg.mas)
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
            Some(s) => self.rng.borrow_mut().noise(s),
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
            let wait = failure_wait(&self.cfg, self.cfg.rt);
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
            let wait = failure_wait(&self.cfg, self.cfg.rt);
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
fn prod_lhs_slots(
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
                            Value::from_pattern(other).map(|x| x.show()).unwrap_or_default(),
                        ),
                    };
                    (t.slot.clone(), v)
                })
                .collect();
            // `isa TYPE` adds the chunk-type's default slot tests
            // (`process-slots-specs`, chunk-spec.lisp:597), so the composed
            // condition prints them too (`isa response` -> `is-response t`).
            if let Some(ct) = tests.iter().find(|t| t.slot == "ISA").and_then(|t| {
                match &t.value {
                    Pattern::Sym(s) => Some(s.clone()),
                    _ => None,
                }
            }) {
                if let Some(t) = types.iter().find(|t| t.name.eq_ignore_ascii_case(&ct)) {
                    for (slot, pat) in &t.defaults {
                        if !slots.iter().any(|(s, _)| s.eq_ignore_ascii_case(slot)) {
                            slots.push((
                                slot.to_uppercase(),
                                SlotVal::Lit(
                                    Value::from_pattern(pat)
                                        .map(|x| x.show())
                                        .unwrap_or_default(),
                                ),
                            ));
                        }
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
                            Value::from_pattern(other).map(|x| x.show()).unwrap_or_default(),
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
fn instantiate_var(name: &str, b: &Bindings) -> Option<String> {
    b.get(name).map(|v| v.show())
}

/// A canonical production comparison rep (`production-comparison-rep`,
/// `production-compilation.lisp:2044`): LHS/RHS tokens with their slots,
/// tokens and slots sorted. Null `=buffer>` modifies are dropped (ACT-R's
/// `COMPOSE-GOAL-BUFFER` may keep them; the composed side here does not).
#[allow(clippy::type_complexity)]
fn canon_prod(
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
fn canon_action(a: &RhsAction) -> Option<(String, Vec<(String, String)>)> {
    let pair = |buffer: &str, slots: Vec<(String, String)>| {
        Some((format!("{buffer}>"), slots))
    };
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

fn canon_pattern(p: &Pattern) -> String {
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
fn canon_composed(
    lhs: &[(String, Vec<(String, SlotVal)>)],
    rhs: &[(String, Vec<(String, String)>)],
) -> (
    Vec<(String, Vec<(String, String)>)>,
    Vec<(String, Vec<(String, String)>)>,
) {
    let mut cl: Vec<(String, Vec<(String, String)>)> = lhs
        .iter()
        .map(|(tok, slots)| {
            let mut s: Vec<(String, String)> = slots
                .iter()
                .map(|(n, v)| (n.clone(), v.render()))
                .collect();
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
fn prod_rhs_actions(
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
                ChunkDesc::Named(n) if n.starts_with('=') => out.push((
                    'r',
                    buffer.clone(),
                    Vec::new(),
                    Some(n[1..].to_string()),
                )),
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
            RhsAction::Clear { buffer } => {
                out.push(('-', buffer.clone(), Vec::new(), None))
            }
            _ => {}
        }
    }
    let _ = b;
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
    doc: &str,
    lhs: &[(String, Vec<(String, SlotVal)>)],
    rhs: &[(String, Vec<(String, String)>)],
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
            s.push_str(&format!("       {n} {}\n", v.render()));
        }
    }
    s.push_str(" ==>\n");
    for (token, slots) in rhs {
        s.push_str(&format!("   {token}\n"));
        for (n, v) in slots {
            s.push_str(&render_rhs_slot(n, v));
        }
    }
    s.push(')');
    s
}

/// One composed-action slot line: a relative pseudo-slot (`"<op> <slot>"`,
/// from a request's `> slot value`) prints the way `render_production` does
/// (`    >  slot value`); everything else is `       slot value`.
fn render_rhs_slot(n: &str, v: &str) -> String {
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
fn sort_token(token: &str) -> String {
    if token.starts_with('!') {
        "z".to_string()
    } else {
        token.to_uppercase()
    }
}

/// `get-buffer-index` (`production-compilation.lisp:1441`): bitmask of how a
/// production uses a buffer — LHS match +8, LHS query +16, RHS `=` +1,
/// RHS `-` +2, RHS `+` +4, RHS `*` +32.
fn buffer_usage(p: &crate::model::Production, buffer: &str) -> u32 {
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
fn lhs_buffers(p: &crate::model::Production) -> Vec<String> {
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
fn rhs_buffers(p: &crate::model::Production) -> Vec<String> {
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

/// The RETRIEVAL compilation type's table (`retrieval-compilation.lisp:279`):
/// `T` entries plus the `R-B-C1`/`R-B-C2` checks, which are not modelled and
/// treated as composing.
const RETRIEVAL_COMPOSE: &[(u32, u32)] = &[
    (28, 28), (28, 24), (28, 20), (28, 16), (28, 12), (28, 8), (28, 4), (28, 0),
    (24, 20), (24, 16), (24, 4), (24, 0),
    (20, 28), (20, 24), (20, 20), (20, 16), (20, 12), (20, 8), (20, 4), (20, 0),
    (16, 28), (16, 24), (16, 20), (16, 16), (16, 12), (16, 8), (16, 4), (16, 0),
    (12, 28), (12, 24), (12, 20), (12, 16), (12, 12), (12, 8), (12, 4), (12, 0),
    (8, 20), (8, 16), (8, 4), (8, 0),
    (4, 28), (4, 24), (4, 20), (4, 16), (4, 12), (4, 8), (4, 4), (4, 0),
    (0, 28), (0, 24), (0, 20), (0, 16), (0, 12), (0, 8), (0, 4),
];

/// The GOAL compilation type's table (`goal-compilation.lisp:704`): `T`
/// entries plus the `G-B-C1`/`NO-RHS-GOAL-REF`/`NO-GOAL-HARVESTING*` checks,
/// which are not modelled and treated as composing.
const GOAL_COMPOSE: &[(u32, u32)] = &[
    (44, 40), (44, 9), (44, 8), (44, 0),
    (40, 44), (40, 40), (40, 13), (40, 12), (40, 9), (40, 8), (40, 4), (40, 0),
    (13, 40), (13, 9), (13, 8), (13, 0),
    (12, 40), (12, 9), (12, 8), (12, 0),
    (9, 13), (9, 12), (9, 9), (9, 8), (9, 4), (9, 0),
    (8, 44), (8, 40), (8, 13), (8, 12), (8, 9), (8, 8), (8, 4), (8, 0),
    (4, 40), (4, 9), (4, 8), (4, 0),
    (0, 44), (0, 40), (0, 13), (0, 12), (0, 9), (0, 8), (0, 4),
];

/// The PERCEPTUAL compilation type's table
/// (`perceptual-compilation.lisp:290`) for VISUAL-LOCATION, VISUAL,
/// AURAL-LOCATION, AURAL and TEMPORAL; `T` entries plus the `P-B-C*` checks,
/// which are not modelled and treated as composing.
const PERCEPTUAL_COMPOSE: &[(u32, u32)] = &[
    (28, 16), (28, 0),
    (24, 28), (24, 24), (24, 20), (24, 16), (24, 12), (24, 8), (24, 4), (24, 0),
    (20, 16), (20, 0),
    (16, 28), (16, 24), (16, 20), (16, 16), (16, 12), (16, 8), (16, 4), (16, 0),
    (12, 16), (12, 0),
    (8, 28), (8, 24), (8, 20), (8, 16), (8, 12), (8, 8), (8, 4), (8, 0),
    (4, 16), (4, 0),
    (0, 28), (0, 24), (0, 20), (0, 16), (0, 12), (0, 8), (0, 4),
];

/// The IMAGINAL compilation type's table (`imaginal-compilation.lisp:802`);
/// `T` entries plus the `I-B-C*` checks, which are not modelled and treated
/// as composing.
const IMAGINAL_COMPOSE: &[(u32, u32)] = &[
    (56, 56), (56, 40), (56, 25), (56, 24), (56, 16), (56, 9), (56, 8), (56, 0),
    (40, 56), (40, 40), (40, 25), (40, 24), (40, 16), (40, 9), (40, 8), (40, 0),
    (29, 56), (29, 40), (29, 25), (29, 24), (29, 16), (29, 9), (29, 8), (29, 0),
    (28, 56), (28, 40), (28, 25), (28, 24), (28, 16), (28, 9), (28, 8), (28, 0),
    (25, 29), (25, 28), (25, 25), (25, 24), (25, 20), (25, 16), (25, 13), (25, 12),
    (25, 9), (25, 8), (25, 4), (25, 0),
    (24, 56), (24, 40), (24, 29), (24, 28), (24, 25), (24, 24), (24, 20), (24, 16),
    (24, 13), (24, 12), (24, 9), (24, 8), (24, 4), (24, 0),
    (20, 56), (20, 40), (20, 25), (20, 24), (20, 16), (20, 9), (20, 8), (20, 0),
    (16, 56), (16, 40), (16, 29), (16, 28), (16, 25), (16, 24), (16, 20), (16, 16),
    (16, 13), (16, 12), (16, 9), (16, 8), (16, 4), (16, 0),
    (13, 56), (13, 40), (13, 25), (13, 24), (13, 16), (13, 9), (13, 8), (13, 0),
    (12, 56), (12, 40), (12, 25), (12, 24), (12, 16), (12, 9), (12, 8), (12, 0),
    (9, 29), (9, 28), (9, 25), (9, 24), (9, 20), (9, 16), (9, 13), (9, 12), (9, 9),
    (9, 8), (9, 4), (9, 0),
    (8, 56), (8, 40), (8, 29), (8, 28), (8, 25), (8, 24), (8, 20), (8, 16), (8, 13),
    (8, 12), (8, 9), (8, 8), (8, 4), (8, 0),
    (4, 56), (4, 40), (4, 25), (4, 24), (4, 16), (4, 9), (4, 8), (4, 0),
    (0, 56), (0, 40), (0, 29), (0, 28), (0, 25), (0, 24), (0, 20), (0, 16), (0, 13),
    (0, 12), (0, 9), (0, 8), (0, 4),
];

/// The MOTOR compilation type's table (`motor-compilation.lisp:145`), which is
/// also the `:default` type for every buffer without its own type (MANUAL,
/// VOCAL, ...); `T` entries plus the `M-B-C*` checks, treated as composing.
const MOTOR_COMPOSE: &[(u32, u32)] = &[
    (20, 16), (20, 0),
    (16, 20), (16, 16), (16, 4), (16, 0),
    (4, 16), (4, 0),
    (0, 20), (0, 16), (0, 4),
];

/// `retrieval-reason` (`retrieval-compilation.lisp:255`), the
/// `failed-function = nil` branch (checks are not modelled).
fn retrieval_reason(u1: u32, u2: u32) -> String {
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
fn perceptual_reason(u1: u32, u2: u32) -> String {
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
fn goal_reason(u1: u32, u2: u32) -> String {
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
fn imaginal_reason(u1: u32, u2: u32) -> String {
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
fn motor_reason(u1: u32, u2: u32) -> String {
    const TESTS: &[u32] = &[
        8, 9, 10, 11, 12, 13, 14, 15, 24, 25, 26, 27, 28, 29, 30, 31, 40, 41, 42, 44, 43, 45,
        46, 47, 56, 57, 58, 60, 59, 61, 62, 63,
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
fn rhs_buffer(a: &RhsAction) -> Option<String> {
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
fn compute_retain_buffers(p1: &Production, p2: &Production) -> (Vec<String>, Vec<String>) {
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
fn sexp_lisp(e: &Sexp) -> String {
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
fn indirect_safe_bind_reason(p: &Production, retain: &[String]) -> Option<String> {
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
fn is_buffer_name(s: &str) -> bool {
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
fn buffer_module(buffer: &str) -> String {
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
fn indirect_action_buffer_name(prod: &Production) -> Option<String> {
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
fn indirect_action_vars(prod: &Production, ignore_retrieval: bool) -> Vec<String> {
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
fn production_bind_vars(prod: &Production) -> Vec<String> {
    let mut out = Vec::new();
    for item in &prod.lhs {
        if let LhsItem::Bind { var, .. } = item {
            out.push(var.to_uppercase());
        }
    }
    for a in &prod.rhs {
        match a {
            RhsAction::Bind { var, .. } => out.push(var.to_uppercase()),
            RhsAction::MvBind { vars, .. } => {
                out.extend(vars.iter().map(|v| v.to_uppercase()))
            }
            _ => {}
        }
    }
    out
}

/// Numeric view of a value for the `!eval!`/`!bind!` evaluator.
fn as_num(v: &Value) -> Option<f64> {
    match v {
        Value::Int(i) => Some(*i as f64),
        Value::Num(x) => Some(*x),
        _ => None,
    }
}

/// CL truth: `T` or `NIL`.
fn bool_value(b: bool) -> Value {
    if b {
        Value::Sym("T".to_string())
    } else {
        Value::Nil
    }
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
                    s.push_str(&format!("       {name} {}\n", show_pat(val, b)));
                }
            }
            RhsAction::ModifyRequest { buffer, slots } => {
                s.push_str(&format!("   *{buffer}>\n"));
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
            RhsAction::DirectSet { buffer, spec } => match spec {
                ChunkDesc::Named(n) => s.push_str(&format!("   @{buffer}> {n}\n")),
                ChunkDesc::Spec { slots, .. } => {
                    s.push_str(&format!("   @{buffer}>\n"));
                    for (name, val) in slots {
                        if name == "ISA" {
                            continue;
                        }
                        s.push_str(&format!("       {name} {}\n", show_pat(val, b)));
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
fn failure_wait(cfg: &Config, rt: f32) -> i64 {
    if cfg.esc {
        latency(cfg.lf_ms, cfg.le, rt)
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
        let (slots, ctype) = match &spec {
            NewChunk::Spec { ctype, slots, .. } => (slots.clone(), ctype.clone()),
            _ => return Err(unimpl("non-spec visual request")),
        };
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
        // A `+visual>` request's command is its chunk-type (`isa
        // move-attention` / `isa start-tracking`), with a `cmd` slot
        // overriding it.
        let command = cmd
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| ctype.clone());
        match command.as_str() {
            "MOVE-ATTENTION" => {
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
            "CLEAR" => {
                self.now(Prio::Num(0), Out::Low, Event::VisualClear);
                Ok(())
            }
            // `start-tracking` (`vision.lisp:2665`): begin tracking the
            // currently attended object; `update-tracking-mth` then stuffs
            // the vision buffers. Scheduled so the request rule's implicit
            // clear-buffer lands first.
            "START-TRACKING" => {
                self.now(Prio::Num(0), Out::Always, Event::StartTracking);
                Ok(())
            }
            got => Err(unimpl(format!("visual command {got:?}"))),
        }
    }

    /// act-touch: hand index (left=0, right=1).
    fn touch_hand_idx(hand: &str) -> Option<usize> {
        match hand.to_uppercase().as_str() {
            "LEFT" => Some(0),
            "RIGHT" => Some(1),
            _ => None,
        }
    }

    /// act-touch: finger index (`index middle ring pinkie thumb`).
    fn touch_finger_idx(finger: &str) -> Option<usize> {
        match finger.to_uppercase().as_str() {
            "INDEX" => Some(0),
            "MIDDLE" => Some(1),
            "RING" => Some(2),
            "PINKIE" => Some(3),
            "THUMB" => Some(4),
            _ => None,
        }
    }

    fn touch_any_finger_down(&self, h: usize) -> bool {
        self.touch_fingers[h].iter().any(|f| f.2 == 0)
    }

    /// A visual location's `(width, height)` (0,0 when absent).
    fn loc_wh(&self, name: &str) -> (i64, i64) {
        if let Some(&idx) = self.vl_copies.get(name) {
            return (self.features[idx].width, self.features[idx].height);
        }
        if let Some(f) = self.features.iter().find(|f| f.name == name) {
            return (f.width, f.height);
        }
        (0, 0)
    }

    /// A chunk-valued slot's `(screen-x, screen-y, name)` (`chunk-to-visual-position`).
    fn chunk_xy_name(&self, v: &Value) -> Option<(i64, i64, String)> {
        let name = match v {
            Value::Sym(s) | Value::Str(s) => s.clone(),
            _ => return None,
        };
        if self.vl_copies.contains_key(&name) {
            if let Some(&(x, y)) = self.vl_xy.get(&name) {
                return Some((x, y, name));
            }
        }
        if let Some(idx) = self.features.iter().position(|f| f.name == name) {
            let f = &self.features[idx];
            return Some((f.x, f.y, name));
        }
        let chunk = self
            .named
            .get(&name)
            .cloned()
            .or_else(|| self.dm.iter().find(|c| c.name.as_deref() == Some(&name)).cloned())
            .or_else(|| {
                self.buffers
                    .values()
                    .find_map(|b| b.chunk.as_ref().filter(|c| c.name.as_deref() == Some(&name)).cloned())
            });
        let c = chunk?;
        let slot = |k: &str| -> Option<i64> {
            c.slots.iter().find(|(s, _)| s == k).and_then(|(_, v)| match v {
                Value::Int(i) => Some(*i),
                Value::Num(x) => Some(*x as i64),
                _ => None,
            })
        };
        Some((slot("SCREEN-X")?, slot("SCREEN-Y")?, name))
    }

    /// `(start-hand-at-touch [hand])`: put a hand on the touch device at its
    /// configured start position (`act-touch.lisp:500`). The standard finger
    /// offsets (`motor.lisp:508`) place the index finger at the start.
    pub fn start_hand_at_touch(&mut self, hand: &str) {
        let Some(h) = Self::touch_hand_idx(hand) else {
            return;
        };
        let (sx, sy) = if h == 0 {
            self.cfg.touch_left_start
        } else {
            self.cfg.touch_right_start
        };
        let dist = self.cfg.touch_ppi * self.cfg.touch_finger_spacing;
        // (index, middle, ring, pinkie, thumb) standard offsets, mirrored
        // for the left hand.
        let offs: [(f32, f32); 5] = if h == 0 {
            [(0.0, 0.0), (-1.0, 0.0), (-2.0, 0.0), (-3.0, 0.0), (1.0, 2.0)]
        } else {
            [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0), (-1.0, 2.0)]
        };
        for (i, (dx, dy)) in offs.iter().enumerate() {
            self.touch_fingers[h][i] = (
                (sx as f32 + dx * dist).round() as i64,
                (sy as f32 + dy * dist).round() as i64,
                1,
            );
        }
        self.touch_hands[h] = true;
    }

    /// A touch movement request (`act-touch.lisp`): compute the style's
    /// prep/exec/finish and schedule the device signal, monitor output and
    /// finger state changes.
    fn touch_request(&mut self, slots: &[(String, Value)]) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        let mut cmd = String::new();
        let mut hand = "RIGHT".to_string();
        let mut finger = "INDEX".to_string();
        let mut loc: Option<(i64, i64, String)> = None;
        let mut object_given = false;
        let mut r_req: Option<f64> = None;
        let mut theta_req: Option<f64> = None;
        let mut num_fngrs: i64 = 1;
        let mut swipe_speed: Option<i64> = None;
        let mut start_width = 0.0f32;
        let mut end_width = 0.0f32;
        let mut rotation = 0.0f32;
        let mut disp: Vec<(String, String)> = Vec::new();
        for (s, v) in slots {
            disp.push((s.clone(), v.show()));
            match s.as_str() {
                "CMD" => {
                    if let Value::Sym(c) = v {
                        cmd = c.to_uppercase();
                    }
                }
                "HAND" => hand = name_of(v),
                "FINGER" => finger = name_of(v),
                "LOC" => loc = self.chunk_xy_name(v),
                "OBJECT" => {
                    object_given = true;
                    if loc.is_none() {
                        loc = self.chunk_xy_name(v);
                    }
                }
                "R" => r_req = num_of(v),
                "THETA" => theta_req = num_of(v),
                "NUM-FNGRS" => num_fngrs = num_of(v).unwrap_or(1.0) as i64,
                "SWIPE-SPEED" => swipe_speed = num_of(v).map(|x| x as i64),
                "START-WIDTH" => start_width = num_of(v).unwrap_or(0.0) as f32,
                "END-WIDTH" => end_width = num_of(v).unwrap_or(0.0) as f32,
                "ROTATION" => rotation = num_of(v).unwrap_or(0.0) as f32,
                _ => {}
            }
        }
        let h = Self::touch_hand_idx(&hand)
            .ok_or_else(|| unimpl(format!("touch hand {hand}")))?;
        let init = self.cfg.init_ms;
        let burst = self.cfg.burst_ms;
        let feat = self.cfg.feat_prep_ms;
        let model = self.model.name.to_uppercase();
        let min_fitts = self.cfg.min_fitts_ms as f32 / 1000.0;

        // Style name + fresh feature count + feat-differences parameters.
        let (style, fresh, nf) = match cmd.as_str() {
            "MOVE-HAND-TOUCH" => ("TOUCH-PLY", 4i64, 4i64),
            "TAP" => ("TAP", 3, 3),
            "TAP-HOLD" => ("TAP-HOLD", 3, 3),
            "TAP-RELEASE" => ("TAP-RELEASE", 3, 3),
            "SWIPE" => ("SWIPE", 7, 7),
            "PINCH" => ("PINCH", 5, 5),
            "ROTATE" => ("ROTATE", 4, 4),
            "HAND-TO-TOUCH" => ("HAND-TO-TOUCH", 4, 4),
            other => return Err(unimpl(format!("touch command {other}"))),
        };
        let feats = match &self.touch_last {
            None => fresh,
            Some((ls, lh, lf, lr, lt)) if ls == style => {
                if lh != &hand {
                    nf - 1
                } else if lf != &finger {
                    nf - 2
                } else {
                    // `rotate`/`pinch` leave r/theta unset (`nil`), so
                    // `distance=`/`direction=` report both as different;
                    // `tap` has no r/theta slots at all.
                    match style {
                        "ROTATE" | "PINCH" => 2,
                        "TAP" | "TAP-HOLD" | "TAP-RELEASE" => 0,
                        _ => {
                            let mut n = 0;
                            if (*lr - r_req.unwrap_or(0.0) as f32).abs() > 1e-4 {
                                n += 1;
                            }
                            if (*lt - theta_req.unwrap_or(0.0) as f32).abs() > 1e-4 {
                                n += 1;
                            }
                            n
                        }
                    }
                }
            }
            Some(_) => fresh,
        };
        self.touch_last = Some((
            style.to_string(),
            hand.clone(),
            finger.clone(),
            r_req.unwrap_or(0.0) as f32,
            theta_req.unwrap_or(0.0) as f32,
        ));
        let prep_ms = feats * feat;
        let _ = object_given;

        // Per-style exec/finish and device signal.
        let mut plan = TouchPlan {
            req_ms,
            prep_ms,
            init_ms: init,
            exec_ms: 0,
            finish_ms: 0,
            signal: String::new(),
            signal_module: "TOUCH-INTERFACE".to_string(),
            monitor: String::new(),
            hand: hand.clone(),
            finger: finger.clone(),
            down_at_init: false,
            down_at_exec: false,
            up_at_exec: false,
            up_at_exec_burst: false,
            move_to: None,
            put_hand: false,
        };
        let fitts = |coef: f32, d: f32, w: f32| -> f32 {
            (coef * (d / w + 0.5).log2()).max(min_fitts)
        };
        let get = |k: &str| -> String {
            disp.iter()
                .find(|(s, _)| s == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        match cmd.as_str() {
            "MOVE-HAND-TOUCH" => {
                let Some((tx, ty, _lname)) = loc else {
                    return Err(unimpl("move-hand-touch without a location"));
                };
                let (cx, cy) = (self.touch_fingers[h][0].0, self.touch_fingers[h][0].1);
                let dx = (tx - cx) as f32;
                let dy = (ty - cy) as f32;
                let r = (dx * dx + dy * dy).sqrt();
                let theta = dy.atan2(dx);
                // `approach-width` (`vision.lisp:3717`): use the loc's
                // width/height if present, else the 1° default.
                let (fw, fh) = self.loc_wh(&_lname);
                let w = if fw > 0 && fh > 0 {
                    let (x, y) = (fw as f32, fh as f32);
                    let critical = (y / x).atan();
                    let mut th = theta.abs();
                    if th > std::f32::consts::FRAC_PI_2 {
                        th = std::f32::consts::PI - th;
                    }
                    let ret = if th == 0.0 {
                        x
                    } else if th == std::f32::consts::FRAC_PI_2 {
                        y
                    } else if th <= critical {
                        x / th.cos()
                    } else {
                        y / (std::f32::consts::FRAC_PI_2 - th).cos()
                    };
                    ret.round()
                } else {
                    // `approach-width` default (1°) → `pm-angle-to-pixels`
                    // rounds to 19 px.
                    19.0
                };
                let mut coef = self.cfg.touch_cursor_coeff;
                if self.cfg.touch_drag_delay && self.touch_any_finger_down(h) {
                    coef *= 2.0f32.powf(0.8);
                }
                let t = fitts(coef, r, w);
                plan.exec_ms = init + (t * 1000.0).round() as i64;
                plan.finish_ms = plan.exec_ms + burst;
                plan.signal = format!("touch-point {model} {hand} {tx} {ty}");
                plan.monitor =
                    format!("model {model} moved the {hand} hand to point {tx} , {ty}");
                plan.move_to = Some((tx, ty));
            }
            "TAP" | "TAP-HOLD" => {
                let angle = (2.0f32
                    * ((self.cfg.touch_ppi * self.cfg.touch_height) / 2.0
                        / (self.cfg.touch_ppi * 15.0))
                        .atan())
                .to_degrees();
                let t = fitts(self.cfg.touch_peck_coeff, angle, 1.0);
                plan.exec_ms = init + (burst as f32).max(t * 1000.0).round() as i64;
                plan.finish_ms = plan.exec_ms + burst;
                let (fx, fy) = {
                    let fi = Self::touch_finger_idx(&finger).unwrap_or(0);
                    (self.touch_fingers[h][fi].0, self.touch_fingers[h][fi].1)
                };
                plan.down_at_exec = true;
                if cmd == "TAP" {
                    plan.up_at_exec_burst = true;
                }
                let sig = if cmd == "TAP" { "touch-tap" } else { "touch-tap-hold" };
                plan.signal = format!("{sig} {model} {hand} {finger} {fx} {fy}");
                let verb = if cmd == "TAP" {
                    format!("tapped the {finger} finger on the {hand} hand at point {fx} , {fy}")
                } else {
                    format!("tapped and held the {finger} finger on the {hand} hand at point {fx} , {fy}")
                };
                plan.monitor = format!("model {model} {verb}");
            }
            "TAP-RELEASE" => {
                plan.exec_ms = init + burst;
                plan.finish_ms = plan.exec_ms + burst;
                let fi = Self::touch_finger_idx(&finger).unwrap_or(0);
                let (fx, fy) = (self.touch_fingers[h][fi].0, self.touch_fingers[h][fi].1);
                plan.up_at_exec = true;
                plan.signal = format!("touch-tap-release {model} {hand} {finger} {fx} {fy}");
                plan.monitor = format!(
                    "model {model} released the {finger} finger on the {hand} hand at point {fx} , {fy}"
                );
            }
            "SWIPE" => {
                let r = r_req.unwrap_or(0.0) as f32;
                let theta = theta_req.unwrap_or(0.0) as f32;
                let w = self.cfg.touch_ppi * self.cfg.touch_thumb_width;
                let mv = (burst as f32 / 1000.0).max(fitts(self.cfg.touch_peck_coeff, r, w));
                let exec = init as f32 / 1000.0 + (burst as f32 / 1000.0).max(mv);
                plan.exec_ms = (exec * 1000.0).round() as i64;
                plan.finish_ms = ((exec + burst as f32 / 1000.0 + mv) * 1000.0).round() as i64;
                plan.down_at_init = true;
                plan.up_at_exec_burst = true;
                let speed = swipe_speed.unwrap_or(self.cfg.touch_swipe_speed);
                let r_disp = get("R");
                let t_disp = get("THETA");
                plan.signal = format!(
                    "touch-swipe {model} {hand} {finger} {r_disp} {t_disp} {num_fngrs} {speed}"
                );
                plan.monitor = format!(
                    "model {model} swiped {num_fngrs} fingers on the {hand} hand starting with the {finger} finger a distance of {} in the {} direction at speed {}",
                    lisp_s_f32(r), lisp_s_f32(theta), lisp_s_f32(speed as f32)
                );
            }
            "PINCH" => {
                let mv = (burst as f32 / 1000.0)
                    .max(fitts(self.cfg.touch_peck_coeff, (start_width - end_width).abs(), self.cfg.touch_ppi * self.cfg.touch_thumb_width));
                let exec = init as f32 / 1000.0 + (burst as f32 / 1000.0).max(mv);
                plan.exec_ms = (exec * 1000.0).round() as i64;
                plan.finish_ms = ((exec + burst as f32 / 1000.0 + mv) * 1000.0).round() as i64;
                plan.down_at_init = true;
                plan.up_at_exec_burst = true;
                plan.signal = format!(
                    "touch-pinch {model} {hand} {finger} {} {}",
                    start_width as i64, end_width as i64
                );
                plan.monitor = format!(
                    "model {model} pinched with the {finger} finger on the {hand} hand from a width of {} to {}",
                    lisp_s_f32(start_width), lisp_s_f32(end_width)
                );
            }
            "ROTATE" => {
                let diameter = 54.0f32 * 5.0f32.sqrt();
                let w = 0.5 * diameter * std::f32::consts::PI * (1.0 / 180.0) * self.cfg.touch_rotate_width;
                let mv = (burst as f32 / 1000.0)
                    .max(fitts(self.cfg.touch_peck_coeff, 0.5 * diameter * rotation.abs(), w));
                let exec = init as f32 / 1000.0 + (burst as f32 / 1000.0).max(mv);
                plan.exec_ms = (exec * 1000.0).round() as i64;
                plan.finish_ms = ((exec + burst as f32 / 1000.0 + mv) * 1000.0).round() as i64;
                plan.down_at_init = true;
                plan.up_at_exec_burst = true;
                plan.signal = format!(
                    "touch-rotate {model} {hand} {finger} {}",
                    lisp_s_f32(rotation)
                );
                plan.monitor = format!(
                    "model {model} rotated with the {finger} finger on the {hand} hand  {} radians",
                    lisp_s_f32(rotation)
                );
            }
            "HAND-TO-TOUCH" => {
                // to-touch-ply (hand-ply): exec = init + fitts(mouse, r, twidth).
                let r = 5.3851647f32;
                let theta = 1.19029f32;
                let t = fitts(self.cfg.touch_cursor_coeff, r, 4.0);
                plan.exec_ms = init + (t * 1000.0).round() as i64;
                plan.finish_ms = plan.exec_ms + burst;
                plan.signal_module = "MOTOR".to_string();
                plan.signal = format!("MOVE-A-HAND {hand} {} {}", lisp_s_f32(r), lisp_s_f32(theta));
                plan.put_hand = true;
            }
            _ => unreachable!(),
        }
        let trace = match cmd.as_str() {
            "MOVE-HAND-TOUCH" => {
                format!("MOVE-HAND-TOUCH OBJECT NIL LOC {} HAND {hand}", get("LOC"))
            }
            "TAP" | "TAP-HOLD" | "TAP-RELEASE" => {
                format!("{cmd} HAND {hand} FINGER {finger}")
            }
            "PINCH" => format!(
                "PINCH HAND {hand} FINGER {finger} START-WIDTH {} END-WIDTH {}",
                get("START-WIDTH"),
                get("END-WIDTH")
            ),
            "ROTATE" => format!(
                "ROTATE HAND {hand} FINGER {finger} ROTATION {}",
                get("ROTATION")
            ),
            "SWIPE" => {
                let mut s = format!(
                    "SWIPE HAND {hand} FINGER {finger} R {} THETA {} NUM-FNGRS {}",
                    get("R"),
                    get("THETA"),
                    get("NUM-FNGRS")
                );
                if disp.iter().any(|(k, _)| k == "SWIPE-SPEED") {
                    s.push_str(&format!(" SWIPE-SPEED {}", get("SWIPE-SPEED")));
                }
                s
            }
            "HAND-TO-TOUCH" => "HAND-TO-TOUCH".to_string(),
            _ => String::new(),
        };
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: trace,
            },
        );
        self.man_prep_busy = true;
        let at = self.time_ms + plan.prep_ms;
        self.push(
            at,
            Prio::Num(0),
            Out::Always,
            Event::TouchPrep {
                plan: Box::new(plan),
            },
        );
        Ok(())
    }

    /// tracker extra: choices uniformly spaced from `min` to `max`
    /// (`generate-tracker-choices`).
    fn tracker_choices(n: usize, min: f64, max: f64) -> Vec<f64> {
        let interval = max - min;
        let step = interval / (n as f64 - 1.0);
        let mut out = Vec::new();
        let mut val = min;
        for i in 0..n {
            out.push(val);
            if i + 1 < n {
                val = max.min(val + step);
            }
        }
        out
    }

    /// tracker extra: initial experience statistics (`tracker-prior-statistics`).
    fn tracker_prior_stats(choices: &[f64]) -> Vec<f64> {
        let mut s = vec![0.0f64; 11];
        for &v in choices {
            s[0] += 1.0;
            s[1] += v;
            s[2] += v * v;
            s[3] += v * v * v;
            s[4] += v * v * v * v;
        }
        s
    }

    /// tracker extra: `+tracker>` create request (`tracker.lisp:608`).
    fn tracker_request(&mut self, slots: &[(String, Value)]) -> Result<(), EngineError> {
        let get = |k: &str| slots.iter().find(|(s, _)| s == k).map(|(_, v)| v.clone());
        let name_of = |v: &Value| match v {
            Value::Sym(s) | Value::Str(s) => s.clone(),
            _ => String::new(),
        };
        let num_of = |v: &Value| match v {
            Value::Int(i) => Some(*i as f64),
            Value::Num(x) => Some(*x),
            _ => None,
        };
        let control_slot = get("CONTROL-SLOT").map(|v| name_of(&v)).unwrap_or_default();
        let good_slot = get("GOOD-SLOT").map(|v| name_of(&v));
        let bad_slot = get("BAD-SLOT").map(|v| name_of(&v));
        let min = get("MIN").and_then(|v| num_of(&v)).unwrap_or(0.0);
        let max = get("MAX").and_then(|v| num_of(&v)).unwrap_or(1.0);
        let count = get("CONTROL-COUNT").and_then(|v| num_of(&v)).unwrap_or(21.0) as usize;
        let delay = get("DELAY").and_then(|v| num_of(&v)).unwrap_or(10.0);
        let temp = get("TEMP").and_then(|v| num_of(&v)).unwrap_or(1.0);
        let scale = get("SCALE").and_then(|v| num_of(&v)).unwrap_or(180.0);
        let good_weight = get("GOOD-WEIGHT").and_then(|v| num_of(&v)).unwrap_or(1.0);
        let bad_weight = get("BAD-WEIGHT").and_then(|v| num_of(&v)).unwrap_or(-1.0);
        let control_buffer = "GOAL".to_string();
        let values_buffer = "IMAGINAL".to_string();
        let now = self.time_ms;
        let n = *self.name_count.entry("TRACKER".to_string()).or_insert(0);
        self.name_count.insert("TRACKER".to_string(), n + 1);
        let name = format!("TRACKER{n}");
        let choices = Self::tracker_choices(count, min, max);
        let statistics = Self::tracker_prior_stats(&choices);
        let idx = self.trackers.len();
        self.trackers.push(Tracker {
            name,
            active: true,
            control_buffer: control_buffer.clone(),
            control_slot: control_slot.clone(),
            choices,
            statistics,
            params: [0.0; 3],
            init_temp: temp,
            temperature: temp,
            delay,
            scale_temp_ms: scale * 1000.0,
            active_time: 0,
            good_buffer: good_slot.as_ref().map(|_| values_buffer.clone()),
            good_slot,
            good_weight,
            good_value: 0.0,
            bad_buffer: bad_slot.as_ref().map(|_| values_buffer.clone()),
            bad_slot,
            bad_weight,
            bad_value: 0.0,
            last_update: now,
            setting: 0.0,
            update_gen: 0,
        });
        let setting = self.tracker_gen_quad_value(idx);
        self.trackers[idx].last_update = self.time_ms;
        self.trackers[idx].setting = setting;
        // `schedule-mod-buffer-chunk` at priority -2000, under TRACKER.
        self.now(
            Prio::Num(-2000),
            Out::Medium,
            Event::ModBuffer {
                buffer: control_buffer,
                slots: vec![(control_slot, num_value(setting))],
                module: "TRACKER".to_string(),
            },
        );
        self.tracker_schedule_update(idx, now);
        Ok(())
    }

    /// tracker extra: `create-update-tracker-event` (`tracker.lisp:578`).
    fn tracker_schedule_update(&mut self, idx: usize, from_ms: i64) {
        let delay = self.trackers[idx].delay;
        let p = self.rng.borrow_mut().random_float(1.0) as f64;
        let t = (p.ln() * -delay).clamp(1.0, 60.0);
        let at = from_ms + (t * 1000.0).round() as i64;
        self.tracker_gen += 1;
        let gen = self.tracker_gen;
        self.trackers[idx].update_gen = gen;
        let name = self.trackers[idx].name.clone();
        let slot = self.trackers[idx].control_slot.clone();
        self.push(
            at,
            Prio::Num(0),
            Out::Always,
            Event::TrackerUpdate { gen },
        );
        let _ = (name, slot);
    }

    /// tracker extra: Boltzmann choice + trace (`tracker-genQuadValue`).
    fn tracker_gen_quad_value(&mut self, idx: usize) -> f64 {
        let choices = self.trackers[idx].choices.clone();
        let temperature = self.trackers[idx].temperature;
        let params = self.trackers[idx].params;
        let ys: Vec<f64> = choices
            .iter()
            .map(|&x| params[0] + x * params[1] + x * x * params[2])
            .collect();
        let max = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let vals: Vec<f64> = ys.iter().map(|&y| ((y - max) / temperature).exp()).collect();
        let sum: f64 = vals.iter().sum();
        let probs: Vec<f64> = vals.iter().map(|&v| v / sum).collect();
        let p = self.rng.borrow_mut().random_float(1.0) as f64;
        if self.cfg.trace_tracker {
            let name = self.trackers[idx].name.clone();
            let cb = self.trackers[idx].control_buffer.clone();
            let cs = self.trackers[idx].control_slot.clone();
            self.out_lines.push(format!(
                "Tracker {name} generating value for buffer {cb} slot {cs} with temperature {}:",
                lisp_s_f32(temperature as f32)
            ));
            let w = choices
                .iter()
                .map(|x| format!("{x:.3}").len())
                .max()
                .unwrap_or(5);
            let fmt = |xs: &[f64]| -> String {
                xs.iter()
                    .map(|x| format!("{x:>w$.3}", w = w))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            self.out_lines.push(format!("  Choices: {}", fmt(&choices)));
            self.out_lines.push(format!("  Probs:   {}", fmt(&probs)));
        }
        let mut result = *choices.last().unwrap();
        let mut cumsum = 0.0;
        for i in 0..choices.len() {
            cumsum += probs[i];
            if p < cumsum {
                result = choices[i];
                break;
            }
        }
        if self.cfg.trace_tracker {
            self.out_lines
                .push(format!(" Chosen result is {}", lisp_s_f32(result as f32)));
        }
        result
    }

    /// tracker extra: `update-tracker` (`tracker.lisp:1298`).
    fn tracker_update(&mut self) -> Result<(), EngineError> {
        for idx in 0..self.trackers.len() {
            if !self.trackers[idx].active {
                continue;
            }
            self.tracker_update_stats(idx, false);
            self.tracker_calc_quad(idx);
            // `create-update-tracker-event` draws the next delay BEFORE
            // `genQuadValue` in `update-tracker` (`tracker.lisp:1303`).
            self.tracker_schedule_update(idx, self.time_ms);
            if self.cfg.trace_tracker {
                let name = self.trackers[idx].name.clone();
                let cs = self.trackers[idx].control_slot.clone();
                let p = self.trackers[idx].params;
                self.out_lines.push(format!(
                    "Tracker {name} for {cs} slot updating the equation"
                ));
                self.out_lines.push(format!(
                    "  new equation is {} + {}x + {}x^2",
                    lisp_s_f32(p[0] as f32),
                    lisp_s_f32(p[1] as f32),
                    lisp_s_f32(p[2] as f32)
                ));
            }
            let setting = self.tracker_gen_quad_value(idx);
            self.trackers[idx].setting = setting;
            self.trackers[idx].last_update = self.time_ms;
            let cb = self.trackers[idx].control_buffer.clone();
            let cs = self.trackers[idx].control_slot.clone();
            self.now(
                Prio::Num(-2000),
                Out::Medium,
                Event::ModBuffer {
                    buffer: cb,
                    slots: vec![(cs, num_value(setting))],
                    module: "TRACKER".to_string(),
                },
            );
        }
        Ok(())
    }

    /// tracker extra: `tracker-update-stats` (`tracker.lisp:1338`). ACT-R
    /// accumulates in single-float (`ms->seconds` returns one).
    fn tracker_update_stats(&mut self, idx: usize, skip: bool) {
        let weight = ((self.time_ms - self.trackers[idx].last_update) as f32 / 1000.0).max(0.05);
        let weight2 = 1.0f32;
        let x = self.trackers[idx].setting as f32;
        let x2 = x * x;
        let good_y = self.trackers[idx].good_value as f32 / weight;
        let bad_y = self.trackers[idx].bad_value as f32 / weight;
        let current: [f32; 11] = [
            1.0,
            x,
            x2,
            x * x2,
            x2 * x2,
            good_y,
            x * good_y,
            x2 * good_y,
            bad_y,
            x * bad_y,
            x2 * bad_y,
        ];
        let old: Vec<f32> = self.trackers[idx].statistics.iter().map(|v| *v as f32).collect();
        let new: Vec<f32> = if skip {
            old.iter().map(|o| weight2 * o).collect()
        } else {
            current
                .iter()
                .zip(old.iter())
                .map(|(c, o)| weight * c + weight2 * o)
                .collect()
        };
        self.trackers[idx].good_value = 0.0;
        self.trackers[idx].bad_value = 0.0;
        if !skip {
            self.trackers[idx].active_time += self.time_ms - self.trackers[idx].last_update;
            // ACT-R computes this as an exact rational; keep f64 precision
            // so the single-float coercion matches (`~f` in the trace).
            self.trackers[idx].temperature = self.trackers[idx].init_temp
                * self.trackers[idx].scale_temp_ms
                / (self.trackers[idx].scale_temp_ms + self.trackers[idx].active_time as f64);
        }
        self.trackers[idx].statistics = new.iter().map(|v| *v as f64).collect();
    }

    /// tracker extra: least-squares quadratic fit (`tracker-calc-quad`).
    fn tracker_calc_quad(&mut self, idx: usize) {
        let s: Vec<f32> = self.trackers[idx].statistics.iter().map(|v| *v as f32).collect();
        let wg = self.trackers[idx].good_weight as f32;
        let wb = self.trackers[idx].bad_weight as f32;
        let n = s[0];
        let (sx1, sx2, sx3, sx4) = (s[1], s[2], s[3], s[4]);
        let sy = wg * s[5] + wb * s[8];
        let sx1y = wg * s[6] + wb * s[9];
        let sx2y = wg * s[7] + wb * s[10];
        let lx2 = sx2 - sx1 * sx1 / n;
        let lx3 = sx3 - sx1 * sx2 / n;
        let lx4 = sx4 - sx2 * sx2 / n;
        let lx1y = sx1y - sx1 * sy / n;
        let lx2y = sx2y - sx2 * sy / n;
        let denom = lx2 * lx4 - lx3 * lx3;
        if denom == 0.0 {
            return;
        }
        let p2 = (lx4 * lx1y - lx3 * lx2y) / denom;
        let p3 = (lx2 * lx2y - lx3 * lx1y) / denom;
        let p1 = (sy - p2 * sx1 - p3 * sx2) / n;
        self.trackers[idx].params = [p1 as f64, p2 as f64, p3 as f64];
    }

    /// tracker extra: buffer-change watcher (`tracker-check-for-buffer-changes`).
    fn tracker_check_buffer_changes(&mut self, buffer: &str, slot: Option<&str>) {
        if !self.cfg.tracker || self.trackers.is_empty() {
            return;
        }
        for idx in 0..self.trackers.len() {
            if !self.trackers[idx].active {
                continue;
            }
            let good = self.trackers[idx].good_buffer.as_deref() == Some(buffer);
            let bad = self.trackers[idx].bad_buffer.as_deref() == Some(buffer);
            if !good && !bad {
                continue;
            }
            let which_good = good
                && slot
                    .map(|s| Some(s) == self.trackers[idx].good_slot.as_deref())
                    .unwrap_or(false);
            let which_bad = bad
                && slot
                    .map(|s| Some(s) == self.trackers[idx].bad_slot.as_deref())
                    .unwrap_or(false);
            if !which_good && !which_bad {
                continue;
            }
            let name = self.trackers[idx].name.clone();
            let cs = self.trackers[idx].control_slot.clone();
            let which = if which_good { "GOOD" } else { "BAD" };
            if self.cfg.trace_tracker {
                self.out_lines.push(format!(
                    "Tracker {name} for {cs} updating the current {which} value"
                ));
                self.out_lines.push(format!(
                    " Updating event occurred in the {which} slot of the {buffer} buffer"
                ));
                self.out_lines.push("  updating with value 1.0".to_string());
            }
            if which_good {
                self.trackers[idx].good_value += 1.0;
            } else {
                self.trackers[idx].bad_value += 1.0;
            }
        }
    }

    /// tracker extra: the `run-test` setup (`simple.lisp:38`) — add the
    /// prompt feature and arm the keypress/choose prompt cycling.
    pub fn tracker_setup(&mut self, slots: &[(String, Sexp)]) {
        self.tracker_task = true;
        let before = self.features.len();
        self.add_visicon(slots);
        if self.features.len() > before {
            self.tracker_feature = Some(self.features[before].name.clone());
        }
    }

    /// tracker task: set the visicon feature's `prompt` and re-display.
    fn tracker_set_prompt(&mut self, prompt: &str) {
        if let Some(name) = self.tracker_feature.clone() {
            if let Some(f) = self.features.iter_mut().find(|f| f.name == name) {
                match f.extra.iter_mut().find(|(k, _)| k == "PROMPT") {
                    Some((_, v)) => *v = Value::Sym(prompt.to_string()),
                    None => f.extra.push(("PROMPT".to_string(), Value::Sym(prompt.to_string()))),
                }
            }
        }
        self.now(Prio::Num(-100), Out::Always, Event::ProcDisplay);
    }

    /// extended-motor-actions `maybe-execute-movement`
    /// (`motor-extension.lisp:1069`): run queued movements whose hand's
    /// execution stage is free.
    fn zz_maybe_execute(&mut self) {
        loop {
            let mut ran = false;
            let mut i = 0;
            while i < self.man_exec_queue.len() {
                let p = self.man_exec_queue[i].clone();
                let free = if self.cfg.zz_two_exec && !p.both {
                    !self.man_exec[p.hand]
                } else {
                    !self.man_exec[0] && !self.man_exec[1]
                };
                if free {
                    self.man_exec_queue.remove(i);
                    if self.cfg.zz_two_exec && !p.both {
                        self.man_exec[p.hand] = true;
                    } else {
                        self.man_exec[0] = true;
                        self.man_exec[1] = true;
                    }
                    let hand_name = if p.both {
                        "BOTH"
                    } else if p.hand == 0 {
                        "LEFT"
                    } else {
                        "RIGHT"
                    };
                    self.zz_last_style =
                        Some((p.style.clone(), p.style.clone(), hand_name.to_string()));
                    let t = self.time_ms;
                    self.push(
                        t + self.cfg.init_ms,
                        Prio::Num(0),
                        Out::Always,
                        Event::InitiationComplete {
                            req_ms: p.req_ms,
                            hand: p.hand,
                            both: p.both,
                            style: p.style.clone(),
                        },
                    );
                    self.push(
                        t + p.finish_ms,
                        Prio::Num(0),
                        Out::Always,
                        Event::FinishMovement {
                            req_ms: p.req_ms,
                            hand: p.hand,
                            both: p.both,
                            style: p.style.clone(),
                        },
                    );
                    match p.kind.clone() {
                        ZzKind::Press => {
                            self.push(
                                t + p.exec_ms,
                                Prio::Num(0),
                                Out::Never,
                                Event::MotorNotify {
                                    key: p.key.clone(),
                                    req_ms: p.req_ms,
                                },
                            );
                        }
                        ZzKind::Key {
                            finger,
                            exec2_ms,
                            output,
                            release_exec,
                        } => {
                            if output {
                                self.push(
                                    t + p.exec_ms,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzKeyOutput {
                                        key: p.key.clone(),
                                        down: true,
                                        hand: p.hand,
                                        finger,
                                    },
                                );
                            } else if release_exec {
                                // `release`/`release-recoil` put the finger up
                                // at exec (`motor-extension.lisp:1968`/`:2012`).
                                self.push(
                                    t + p.exec_ms,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzKeyOutput {
                                        key: p.key.clone(),
                                        down: false,
                                        hand: p.hand,
                                        finger,
                                    },
                                );
                            } else {
                                // `point-finger`'s `set-hand-position`
                                // (`motor-extension.lisp:2053`) has no details
                                // but still wakes conflict resolution.
                                self.push(
                                    t + p.exec_ms,
                                    Prio::Num(0),
                                    Out::Never,
                                    Event::Dummy,
                                );
                            }
                            if let Some(e2) = exec2_ms {
                                self.push(
                                    t + e2,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzKeyOutput {
                                        key: p.key.clone(),
                                        down: false,
                                        hand: p.hand,
                                        finger,
                                    },
                                );
                            }
                            // `perform-movement` marks the finger busy
                            // (`motor-extension.lisp:1186`); the finish handler
                            // frees it before the next movement starts (`:1205`).
                            self.zz_hands[p.hand].busy[finger] = true;
                        }
                        ZzKind::Hand { to_mouse, r, theta } => {
                            self.push(
                                t + p.exec_ms,
                                Prio::Num(0),
                                Out::Low,
                                Event::Emit {
                                    module: "MOTOR".to_string(),
                                    line: format!(
                                        "MOVE-A-HAND RIGHT {} {}",
                                        lisp_s_f32(r),
                                        lisp_s_f32(theta)
                                    ),
                                },
                            );
                            // `perform-movement` auto-releases any finger that
                            // is down `key-release-time` after initiation.
                            let held: Vec<(usize, String)> = (0..5)
                                .filter_map(|f| {
                                    self.zz_hands[p.hand].held[f].clone().map(|k| (f, k))
                                })
                                .collect();
                            for (f, k) in held {
                                self.push(
                                    t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzKeyOutput {
                                        key: k,
                                        down: false,
                                        hand: p.hand,
                                        finger: f,
                                    },
                                );
                                self.zz_hands[p.hand].held[f] = None;
                            }
                            if self.zz_mouse_down {
                                self.zz_mouse_down = false;
                                self.push(
                                    t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzMouseRelease { req_ms: p.req_ms },
                                );
                            }
                            let target = if to_mouse {
                                self.zz_mouse_cursor
                            } else {
                                self.zz_home_position(p.hand, 0)
                            };
                            let off = self.zz_hands[p.hand].offsets[0];
                            self.zz_hands[p.hand].loc = (target.0 - off.0, target.1 - off.1);
                        }
                        ZzKind::HandPly { target, r, theta } => {
                            // `queue-output-events` (`motor.lisp:1078`): the
                            // details are `move-a-hand <hand> <r> <theta>`.
                            self.push(
                                t + p.exec_ms,
                                Prio::Num(0),
                                Out::Low,
                                Event::Emit {
                                    module: "MOTOR".to_string(),
                                    line: format!(
                                        "MOVE-A-HAND {} {} {}",
                                        if p.hand == 0 { "LEFT" } else { "RIGHT" },
                                        r,
                                        theta
                                    ),
                                },
                            );
                            // `perform-movement` (`motor-extension.lisp:1163`):
                            // a `hand-ply` releases any held finger
                            // `key-release-time` after initiation.
                            let held: Vec<(usize, String)> = (0..5)
                                .filter_map(|f| {
                                    self.zz_hands[p.hand].held[f].clone().map(|k| (f, k))
                                })
                                .collect();
                            for (f, k) in held {
                                self.push(
                                    t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                    Prio::Num(0),
                                    Out::Always,
                                    Event::ZzKeyOutput {
                                        key: k,
                                        down: false,
                                        hand: p.hand,
                                        finger: f,
                                    },
                                );
                                self.zz_hands[p.hand].held[f] = None;
                            }
                            self.zz_hands[p.hand].loc = target;
                        }
                        ZzKind::AllHome { hands } => {
                            // `queue-output-events` (`motor-extension.lisp:2363`):
                            // a `ALL-FINGERS-TO-HOME <hand>` details line per
                            // hand at exec, and the hand reset to home.
                            for h in &hands {
                                self.push(
                                    t + p.exec_ms,
                                    Prio::Num(0),
                                    Out::Low,
                                    Event::Emit {
                                        module: "MOTOR".to_string(),
                                        line: format!(
                                            "ALL-FINGERS-TO-HOME {}",
                                            if *h == 0 { "LEFT" } else { "RIGHT" }
                                        ),
                                    },
                                );
                            }
                            for h in &hands {
                                // Auto-release any held/down finger, then reset
                                // the hand to the home row (`prepare-features`,
                                // `motor-extension.lisp:2271`).
                                let held: Vec<(usize, String)> = (0..5)
                                    .filter_map(|f| {
                                        self.zz_hands[*h].held[f].clone().map(|k| (f, k))
                                    })
                                    .collect();
                                for (f, k) in held {
                                    self.push(
                                        t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                        Prio::Num(0),
                                        Out::Always,
                                        Event::ZzKeyOutput {
                                            key: k,
                                            down: false,
                                            hand: *h,
                                            finger: f,
                                        },
                                    );
                                    self.zz_hands[*h].held[f] = None;
                                }
                                if self.zz_mouse_down {
                                    self.zz_mouse_down = false;
                                    self.push(
                                        t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                        Prio::Num(0),
                                        Out::Always,
                                        Event::ZzMouseRelease { req_ms: p.req_ms },
                                    );
                                }
                                self.zz_hands[*h].loc = if *h == 0 { (4, 4) } else { (7, 4) };
                                self.zz_hands[*h].offsets = if *h == 0 {
                                    LEFT_FINGER_OFFSETS
                                } else {
                                    RIGHT_FINGER_OFFSETS
                                };
                            }
                        }
                        ZzKind::ReleaseAll { hands } => {
                            // `queue-output-events` (`motor-extension.lisp:2610`):
                            // a `RELEASING-ALL-FINGERS` details line at exec; the
                            // finger positions are unchanged.
                            self.push(
                                t + p.exec_ms,
                                Prio::Num(0),
                                Out::Medium,
                                Event::Emit {
                                    module: "MOTOR".to_string(),
                                    line: "RELEASING-ALL-FINGERS".to_string(),
                                },
                            );
                            for h in &hands {
                                // Auto-release any held finger at
                                // `init + key-release-time`.
                                let held: Vec<(usize, String)> = (0..5)
                                    .filter_map(|f| {
                                        self.zz_hands[*h].held[f].clone().map(|k| (f, k))
                                    })
                                    .collect();
                                for (f, k) in held {
                                    self.push(
                                        t + self.cfg.init_ms + self.cfg.zz_krt_ms,
                                        Prio::Num(0),
                                        Out::Always,
                                        Event::ZzKeyOutput {
                                            key: k,
                                            down: false,
                                            hand: *h,
                                            finger: f,
                                        },
                                    );
                                    self.zz_hands[*h].held[f] = None;
                                }
                            }
                        }
                    }
                    ran = true;
                    break;
                }
                i += 1;
            }
            if !ran {
                break;
            }
        }
    }

    /// `movement-strike-time` (`motor-extension.lisp:1557`): the delay from
    /// initiation to the finger contacting the key, when
    /// `peck-strike-distance` (`*psd*`) is set.
    fn zz_movement_strike_time(&self, tt_ms: i64, r: f32) -> i64 {
        match self.cfg.zz_psd {
            None => tt_ms,
            Some(psd) if psd == 0.0 => tt_ms + self.cfg.zz_kct_ms,
            Some(psd) => {
                let tt = tt_ms as f32 / 1000.0;
                let target = r - psd;
                let mut width = 0.25 * tt;
                let mut guess = 0.75 * tt;
                let mut value = minjerk_dist(guess, r, tt);
                let mut dir: f32 = if value < target { 1.0 } else { -1.0 };
                while (target - value).abs() >= 0.01 {
                    width /= 2.0;
                    guess += dir * width;
                    value = minjerk_dist(guess, r, tt);
                    dir = if value < target { 1.0 } else { -1.0 };
                }
                ((self.cfg.zz_kct_ms as f32 / 1000.0 + guess) * 1000.0).round() as i64
            }
        }
    }

    /// extended-motor-actions mouse request (`move-cursor`/`click-mouse`/
    /// `hold-mouse`/`release-mouse`).
    fn zz_mouse_request(&mut self, cmd: &str, slots: &[(String, Value)]) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        let loc = slots
            .iter()
            .find(|(s, _)| s == "LOC")
            .and_then(|(_, v)| self.chunk_xy_name(v));
        // `(class, style-name, fresh-features)`: the details use the class
        // (`type-of`), `compute-prep-time` compares the style-name.
        let (class, style_name, fresh) = match cmd {
            "MOVE-CURSOR" => ("CURSOR-PLY", "CURSOR-PLY", 4i64),
            "CLICK-MOUSE" => ("PUNCH", "PUNCH", 3),
            "HOLD-MOUSE" => ("HOLD-PUNCH", "PUNCH", 3),
            "RELEASE-MOUSE" => ("RELEASE", "PUNCH", 3),
            _ => unreachable!(),
        };
        let feats = match &self.zz_last_style {
            Some((_, ls, _)) if ls == style_name => 0,
            _ => fresh,
        };
        self.zz_last_style = Some((
            class.to_string(),
            style_name.to_string(),
            "RIGHT".to_string(),
        ));
        self.man_prep_busy = true;
        self.man_last_prep = Some(1);
        self.zz_last_cmd[1] = class.to_string();
        if self.cfg.zz_two_proc {
            self.man_proc[1] = true;
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        let prep = feats * self.cfg.feat_prep_ms;
        // `hold-mouse`/`release-mouse` call `hold-punch`/`release` directly
        // (`motor-extension.lisp:2469`/`2486`), so they have no request trace.
        let trace = match cmd {
            "MOVE-CURSOR" => Some(format!(
                "MOVE-CURSOR LOC {}",
                loc.as_ref().map(|l| l.2.clone()).unwrap_or_default()
            )),
            "CLICK-MOUSE" => Some("CLICK-MOUSE".to_string()),
            _ => None,
        };
        if let Some(line) = trace {
            self.now(
                Prio::Num(0),
                Out::Low,
                Event::Emit {
                    module: "MOTOR".to_string(),
                    line,
                },
            );
        }
        self.push(
            self.time_ms + prep,
            Prio::Num(0),
            Out::Always,
            Event::ZzMousePrep {
                cmd: cmd.to_string(),
                req_ms,
                loc,
            },
        );
        Ok(())
    }

    /// extended-motor-actions: a finger's absolute keyboard location.
    fn zz_finger_loc(&self, hand: usize, finger: usize) -> (i64, i64) {
        let h = &self.zz_hands[hand];
        (
            h.loc.0 + h.offsets[finger].0,
            h.loc.1 + h.offsets[finger].1,
        )
    }

    /// extended-motor-actions: a finger's home keyboard location.
    fn zz_home_position(&self, hand: usize, finger: usize) -> (i64, i64) {
        let home = if hand == 0 { (4, 4) } else { (7, 4) };
        let offs = if hand == 0 {
            LEFT_FINGER_OFFSETS
        } else {
            RIGHT_FINGER_OFFSETS
        };
        (home.0 + offs[finger].0, home.1 + offs[finger].1)
    }

    /// `(style-name, num-possible-feats, hfrt)` for a movement class
    /// (`motor-extension.lisp:1861`+ / `motor.lisp:984`).
    fn zz_style_info(class: &str) -> (&'static str, i64, bool) {
        match class {
            "PECK" | "PECK-RECOIL" | "HOLD-PECK" | "POINT-FINGER" | "RELEASE-RECOIL" => {
                ("PECK", 5, true)
            }
            "PUNCH" | "HOLD-PUNCH" | "RELEASE" | "DELAYED-PUNCH" => ("PUNCH", 3, false),
            // `hand-ply` (`motor.lisp:1091`): `ply`'s `num-possible-feats`
            // `:around` drops the `:dummy` finger, so 4 features to prepare.
            "HAND-PLY" => ("PLY", 4, true),
            _ => ("PLY", 3, false),
        }
    }

    /// `feat-differences` (`motor.lisp:970`) / `compute-prep-time`
    /// (`general-pm.lisp:670`): how many features need preparing given the
    /// previously prepared movement.
    fn zz_feat_differences(
        &self,
        style_name: &str,
        maxfeats: i64,
        hand: usize,
        finger: usize,
        r: f32,
        theta: f32,
    ) -> i64 {
        match &self.zz_last_move {
            None => maxfeats,
            Some(prev) if prev.style_name != style_name => maxfeats,
            Some(prev) if prev.hfrt => {
                let maxfeats = prev.maxfeats;
                if prev.hand != hand {
                    maxfeats - 1
                } else if prev.finger != finger {
                    maxfeats - 2
                } else {
                    let mut n = 0;
                    if (prev.r - r).abs() >= 2.0 {
                        n += 1;
                    }
                    if !direction_eq(prev.theta, theta) {
                        n += 1;
                    }
                    n
                }
            }
            Some(prev) => {
                if prev.hand != hand {
                    2
                } else if prev.finger != finger {
                    1
                } else {
                    0
                }
            }
        }
    }

    /// `extended-finger-keyboard` (`motor-extension.lisp:2103`): resolve a key
    /// to a hand/finger, then dispatch "same-loc" or "move-loc".
    fn zz_extended_finger_keyboard(
        &mut self,
        cmd: &str,
        key: &str,
        same_style: &str,
        move_style: &str,
    ) -> Result<(), EngineError> {
        let Some((hand, finger)) = key_hand_finger(key) else {
            return Ok(());
        };
        let Some(key_loc) = key_xy(key) else {
            return Ok(());
        };
        let finger_loc = self.zz_finger_loc(hand, finger);
        if key_loc == finger_loc {
            self.zz_key_request(cmd, same_style, key, hand, finger, 0.0, 0.0, Some(key_loc), 0)
        } else {
            let dx = (key_loc.0 - finger_loc.0) as f64;
            let dy = (key_loc.1 - finger_loc.1) as f64;
            let r = (dx * dx + dy * dy).sqrt() as f32;
            let theta = dy.atan2(dx) as f32;
            self.zz_key_request(cmd, move_style, key, hand, finger, r, theta, Some(key_loc), 0)
        }
    }

    /// Schedule a keyboard movement style's prep/exec/finish.
    #[allow(clippy::too_many_arguments)]
    fn zz_key_request(
        &mut self,
        cmd: &str,
        class: &str,
        key: &str,
        hand: usize,
        finger: usize,
        r: f32,
        theta: f32,
        loc: Option<(i64, i64)>,
        delay_ms: i64,
    ) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        let (style_name, maxfeats, hfrt) = Self::zz_style_info(class);
        // `compute-prep-time`/`feat-differences` (`general-pm.lisp:670`,
        // `motor.lisp:970`).
        let feats = self.zz_feat_differences(style_name, maxfeats, hand, finger, r, theta);
        let hand_name = if hand == 0 { "LEFT" } else { "RIGHT" };
        self.zz_last_style = Some((
            class.to_string(),
            style_name.to_string(),
            hand_name.to_string(),
        ));
        self.zz_last_move = Some(ZzMoveInfo {
            style_name: style_name.to_string(),
            hand,
            finger,
            r,
            theta,
            maxfeats,
            hfrt,
        });
        self.man_prep_busy = true;
        self.man_last_prep = Some(hand);
        self.zz_last_cmd[hand] = class.to_string();
        if self.cfg.zz_two_proc {
            self.man_proc[hand] = true;
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        let prep = feats * self.cfg.feat_prep_ms;
        self.push(
            self.time_ms + prep,
            Prio::Num(0),
            Out::Always,
            Event::ZzKeyPrep {
                cmd: cmd.to_string(),
                style: class.to_string(),
                key: key.to_string(),
                hand,
                finger,
                r,
                theta,
                loc,
                delay_ms,
                req_ms,
            },
        );
        Ok(())
    }

    /// extended-motor-actions keyboard/mouse-high-level commands.
    fn zz_key_cmd(&mut self, cmd: &str, slots: &[(String, Value)]) -> Result<(), EngineError> {
        let key = slots
            .iter()
            .find(|(s, _)| s == "KEY")
            .map(|(_, v)| name_of(v))
            .unwrap_or_default();
        match cmd {
            "HOLD-KEY" => self.zz_extended_finger_keyboard(cmd, &key, "HOLD-PUNCH", "HOLD-PECK"),
            "HIT-KEY" => self.zz_extended_finger_keyboard(cmd, &key, "PUNCH", "PECK"),
            "TYPE-KEY" => self.zz_extended_finger_keyboard(cmd, &key, "PUNCH", "PECK-RECOIL"),
            "MOVE-TO-KEY" => {
                self.zz_extended_finger_keyboard(cmd, &key, "POINT-FINGER", "POINT-FINGER")
            }
            "RELEASE-KEY" => self.zz_extended_finger_keyboard(cmd, &key, "RELEASE", "RELEASE"),
            "RELEASE-KEY-TO-HOME" => self.zz_release_key_to_home(&key),
            "MOVE-FINGER-TO-HOME" => self.zz_move_finger_to_home(slots),
            "ALL-FINGERS-TO-HOME" => self.zz_all_fingers_to_home(slots),
            "HAND-TO-MOUSE" => self.zz_hand_to(true),
            "HAND-TO-HOME" => self.zz_hand_to(false),
            _ => Ok(()),
        }
    }

    /// A direct style request (`handle-style-request`, `motor.lisp:1447`):
    /// trace the request, then schedule the style with the spec's features.
    fn zz_direct_style(
        &mut self,
        cmd: &str,
        slots: &[(String, Value)],
    ) -> Result<(), EngineError> {
        let hand = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .and_then(|(_, v)| Self::touch_hand_idx(&name_of(v)))
            .unwrap_or(0);
        let finger = slots
            .iter()
            .find(|(s, _)| s == "FINGER")
            .and_then(|(_, v)| Self::touch_finger_idx(&name_of(v)))
            .unwrap_or(0);
        let r = slots
            .iter()
            .find(|(s, _)| s == "R")
            .and_then(|(_, v)| num_of(v))
            .unwrap_or(0.0) as f32;
        let theta = slots
            .iter()
            .find(|(s, _)| s == "THETA")
            .and_then(|(_, v)| num_of(v))
            .unwrap_or(0.0) as f32;
        self.zz_direct_style_trace(cmd, hand, finger, r, theta);
        // `move-a-finger` shifts the offset by the polar vector.
        let cur = self.zz_finger_loc(hand, finger);
        let target = if r == 0.0 {
            cur
        } else {
            (
                (cur.0 as f32 + r * theta.cos()).round() as i64,
                (cur.1 as f32 + r * theta.sin()).round() as i64,
            )
        };
        // A direct style has no `key` slot; the device names the output from
        // the finger's location (`motor.lisp:960`).
        let key = xy_key(target.0, target.1).unwrap_or("").to_string();
        self.zz_key_request(cmd, cmd, &key, hand, finger, r, theta, Some(target), 0)
    }

    /// `handle-style-request` details (`motor.lisp:1459`): the style name
    /// followed by its feature slots.
    fn zz_direct_style_trace(
        &mut self,
        class: &str,
        hand: usize,
        finger: usize,
        r: f32,
        theta: f32,
    ) {
        let h = if hand == 0 { "LEFT" } else { "RIGHT" };
        let f = ["INDEX", "MIDDLE", "RING", "PINKIE", "THUMB"][finger];
        let (_, _, hfrt) = Self::zz_style_info(class);
        let line = if hfrt {
            format!(
                "{class} HAND {h} FINGER {f} R {} THETA {}",
                lisp_s_f32(r),
                lisp_s_f32(theta)
            )
        } else {
            format!("{class} HAND {h} FINGER {f}")
        };
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line,
            },
        );
    }

    /// `all-fingers-to-home` (`motor-extension.lisp:2401`): move the requested
    /// hand (or both) if it is not already at the home row.
    fn zz_all_fingers_to_home(
        &mut self,
        slots: &[(String, Value)],
    ) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        // `:2405`: no `hand` slot means both hands; otherwise the named hand
        // (`both` is the two-hand case).
        let hand_slot = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .map(|(_, v)| name_of(v).to_uppercase());
        let candidates: Vec<usize> = match hand_slot.as_deref() {
            None | Some("BOTH") => vec![0, 1],
            Some("LEFT") => vec![0],
            Some("RIGHT") => vec![1],
            Some(_) => return Ok(()),
        };
        let mut hands: Vec<usize> = Vec::new();
        for h in candidates {
            if self.zz_hand_needs_home(h) {
                hands.push(h);
            }
        }
        if hands.is_empty() {
            return Ok(());
        }
        let both = hands.len() == 2;
        let single = if both { 1 } else { hands[0] };
        // `r` = max hand-ply distance to the home row (`:2317`/`:2346`).
        let r = hands
            .iter()
            .map(|h| {
                let home = if *h == 0 { (4, 4) } else { (7, 4) };
                let cur = self.zz_hands[*h].loc;
                let dx = (home.0 - cur.0) as f64;
                let dy = (home.1 - cur.1) as f64;
                (dx * dx + dy * dy).sqrt() as f32
            })
            .fold(0.0f32, f32::max);
        // `compute-prep-time`: the previous style-name differs (`peck` vs
        // `all-home`), so `num-to-prepare` = hand + r minus the direction
        // (`:2265`) = 2.
        let feats = 2i64;
        self.zz_last_style = Some((
            "ALL-FINGERS-TO-HOME".to_string(),
            "ALL-HOME".to_string(),
            if both {
                "BOTH".to_string()
            } else if single == 0 {
                "LEFT".to_string()
            } else {
                "RIGHT".to_string()
            },
        ));
        self.zz_last_move = Some(ZzMoveInfo {
            style_name: "ALL-HOME".to_string(),
            hand: single,
            finger: 4,
            r,
            theta: 0.0,
            maxfeats: 5,
            hfrt: true,
        });
        self.man_prep_busy = true;
        self.man_last_prep = Some(single);
        for h in &hands {
            self.zz_last_cmd[*h] = "ALL-FINGERS-TO-HOME".to_string();
        }
        if self.cfg.zz_two_proc {
            for h in &hands {
                self.man_proc[*h] = true;
            }
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        let prep = feats * self.cfg.feat_prep_ms;
        self.push(
            self.time_ms + prep,
            Prio::Num(0),
            Out::Always,
            Event::ZzAllHomePrep { req_ms, hands, r },
        );
        Ok(())
    }

    /// `release-all-fingers` (`motor-extension.lisp:2616`): release the held
    /// fingers of whichever hands have one down; with none, warn and do
    /// nothing.
    fn zz_release_all_fingers(&mut self) -> Result<(), EngineError> {
        let mut hands: Vec<usize> = Vec::new();
        for h in 0..2 {
            if (0..5).any(|f| self.zz_hands[h].down[f]) {
                hands.push(h);
            }
        }
        if hands.is_empty() {
            // `:2637`: `print-warning`, no movement.
            self.emit_warning(
                "Release-all-fingers action called when no fingers need to be released.",
            );
            return Ok(());
        }
        let req_ms = self.time_ms;
        let both = hands.len() == 2;
        let single = if both { 1 } else { hands[0] };
        let hand_name = if both {
            "BOTH"
        } else if single == 0 {
            "LEFT"
        } else {
            "RIGHT"
        };
        // The request trace is an explicit `:details` string, so it stays
        // lowercase (`motor-extension.lisp:2628`).
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: "release-all-fingers".to_string(),
            },
        );
        self.zz_last_style = Some((
            "RELEASE-ALL-FINGERS".to_string(),
            "RELEASE-ALL-FINGERS".to_string(),
            hand_name.to_string(),
        ));
        self.zz_last_move = Some(ZzMoveInfo {
            style_name: "RELEASE-ALL-FINGERS".to_string(),
            hand: single,
            finger: ZZ_DUMMY_FINGER,
            r: 0.0,
            theta: 0.0,
            maxfeats: 1,
            hfrt: false,
        });
        self.man_prep_busy = true;
        self.man_last_prep = Some(single);
        for h in &hands {
            self.zz_last_cmd[*h] = "RELEASE-ALL-FINGERS".to_string();
        }
        if self.cfg.zz_two_proc {
            for h in &hands {
                self.man_proc[*h] = true;
            }
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        // `num-to-prepare` is 1 (`release-all-fingers` feature-slots is
        // `(hand)` and its `:around` drops one, `:2524`).
        let prep = self.cfg.feat_prep_ms;
        self.push(
            self.time_ms + prep,
            Prio::Num(0),
            Out::Always,
            Event::ZzReleaseAllPrep { req_ms, hands },
        );
        Ok(())
    }

    /// `all-fingers-to-home` (`motor-extension.lisp:2419`): whether a hand
    /// needs moving — off the home location, any finger down, or a finger off
    /// its home position.
    fn zz_hand_needs_home(&self, h: usize) -> bool {
        let home_loc = if h == 0 { (4, 4) } else { (7, 4) };
        if self.zz_hands[h].loc != home_loc {
            return true;
        }
        for f in 0..5 {
            if self.zz_hands[h].down[f] {
                return true;
            }
            if self.zz_finger_loc(h, f) != self.zz_home_position(h, f) {
                return true;
            }
        }
        false
    }

    /// `move-finger-to-home` (`motor-extension.lisp:2222`): if the finger is
    /// not already home, `point-finger` it back.
    fn zz_move_finger_to_home(
        &mut self,
        slots: &[(String, Value)],
    ) -> Result<(), EngineError> {
        let Some(hand) = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .and_then(|(_, v)| Self::touch_hand_idx(&name_of(v)))
        else {
            return Ok(());
        };
        let Some(finger) = slots
            .iter()
            .find(|(s, _)| s == "FINGER")
            .and_then(|(_, v)| Self::touch_finger_idx(&name_of(v)))
        else {
            return Ok(());
        };
        let home = self.zz_home_position(hand, finger);
        let cur = self.zz_finger_loc(hand, finger);
        if home == cur {
            // `:2236`: warning only, no movement.
            return Ok(());
        }
        let dx = (home.0 - cur.0) as f64;
        let dy = (home.1 - cur.1) as f64;
        let r = (dx * dx + dy * dy).sqrt() as f32;
        let theta = dy.atan2(dx) as f32;
        self.zz_key_request(
            "MOVE-FINGER-TO-HOME",
            "POINT-FINGER",
            "",
            hand,
            finger,
            r,
            theta,
            Some(home),
            0,
        )
    }

    /// `release-key-to-home` (`motor-extension.lisp:2183`): release the key
    /// under the finger, recoiling to home if the finger is off home.
    fn zz_release_key_to_home(&mut self, key: &str) -> Result<(), EngineError> {
        let Some((hand, finger)) = key_hand_finger(key) else {
            return Ok(());
        };
        let Some(key_loc) = key_xy(key) else {
            return Ok(());
        };
        let finger_loc = self.zz_finger_loc(hand, finger);
        if key_loc != finger_loc {
            // `:2195`: warning only, no action.
            return Ok(());
        }
        let home = self.zz_home_position(hand, finger);
        if home == finger_loc {
            self.zz_key_request(
                "RELEASE-KEY-TO-HOME",
                "RELEASE",
                key,
                hand,
                finger,
                0.0,
                0.0,
                None,
                0,
            )
        } else {
            let dx = (home.0 - finger_loc.0) as f64;
            let dy = (home.1 - finger_loc.1) as f64;
            let r = (dx * dx + dy * dy).sqrt() as f32;
            let theta = dy.atan2(dx) as f32;
            self.zz_key_request(
                "RELEASE-KEY-TO-HOME",
                "RELEASE-RECOIL",
                key,
                hand,
                finger,
                r,
                theta,
                Some(home),
                0,
            )
        }
    }

    /// `hand-to-mouse`/`hand-to-home` (a `hand-ply`): move the right hand to
    /// the mouse / home, auto-releasing any held key.
    fn zz_hand_to(&mut self, to_mouse: bool) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        let target = if to_mouse {
            self.zz_mouse_cursor
        } else {
            self.zz_home_position(1, 0)
        };
        let (cx, cy) = self.zz_finger_loc(1, 0);
        let dx = (target.0 - cx) as f64;
        let dy = (target.1 - cy) as f64;
        let r = (dx * dx + dy * dy).sqrt() as f32;
        let theta = dy.atan2(dx) as f32;
        self.zz_last_style = Some((
            "HAND-PLY".to_string(),
            "HAND-PLY".to_string(),
            "RIGHT".to_string(),
        ));
        self.zz_last_move = Some(ZzMoveInfo {
            style_name: "HAND-PLY".to_string(),
            hand: 1,
            finger: 0,
            r,
            theta,
            maxfeats: 3,
            hfrt: false,
        });
        self.man_prep_busy = true;
        self.man_last_prep = Some(1);
        self.zz_last_cmd[1] = "HAND-PLY".to_string();
        if self.cfg.zz_two_proc {
            self.man_proc[1] = true;
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        // `HAND-TO-MOUSE`/`HAND-TO-HOME` request trace (custom).
        let line = if to_mouse {
            "HAND-TO-MOUSE".to_string()
        } else {
            "HAND-TO-HOME".to_string()
        };
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line,
            },
        );
        self.push(
            self.time_ms + 4 * self.cfg.feat_prep_ms,
            Prio::Num(0),
            Out::Always,
            Event::ZzHandPrep { req_ms, r, theta, to_mouse },
        );
        Ok(())
    }

    /// Schedule a `hand-ply` (a `ply` that moves the hand loc) movement.
    /// Used by `prepare`/`execute` and `point-hand-at-key`
    /// (`motor.lisp:1096`/`:1149`). `r_str`/`theta_str` are the pre-rendered
    /// `~a` spellings for the `MOVE-A-HAND` details; `deferred` marks a
    /// `prepare` whose execution waits for an `execute` request.
    #[allow(clippy::too_many_arguments)]
    fn zz_hand_ply_request(
        &mut self,
        class: &str,
        hand: usize,
        r: f32,
        theta: f32,
        r_str: String,
        theta_str: String,
        target: (i64, i64),
        deferred: bool,
    ) -> Result<(), EngineError> {
        let req_ms = self.time_ms;
        let (style_name, maxfeats, hfrt) = Self::zz_style_info(class);
        let feats =
            self.zz_feat_differences(style_name, maxfeats, hand, ZZ_DUMMY_FINGER, r, theta);
        let hand_name = if hand == 0 { "LEFT" } else { "RIGHT" };
        self.zz_last_style = Some((
            class.to_string(),
            style_name.to_string(),
            hand_name.to_string(),
        ));
        self.zz_last_move = Some(ZzMoveInfo {
            style_name: style_name.to_string(),
            hand,
            finger: ZZ_DUMMY_FINGER,
            r,
            theta,
            maxfeats,
            hfrt,
        });
        self.man_prep_busy = true;
        self.man_last_prep = Some(hand);
        self.zz_last_cmd[hand] = class.to_string();
        if self.cfg.zz_two_proc {
            self.man_proc[hand] = true;
        } else {
            self.man_proc[0] = true;
            self.man_proc[1] = true;
        }
        let prep = feats * self.cfg.feat_prep_ms;
        self.push(
            self.time_ms + prep,
            Prio::Num(0),
            Out::Always,
            Event::ZzHandPlyPrep {
                class: class.to_string(),
                hand,
                r,
                theta,
                r_str,
                theta_str,
                target,
                req_ms,
                deferred,
            },
        );
        Ok(())
    }

    /// `prepare` (`general-pm.lisp:562`): build a `hand-ply` and prepare it,
    /// but do not execute it until an `execute` request arrives. The trace is
    /// the event name plus its params (`scheduling.lisp` `format-event`).
    fn zz_prepare(&mut self, slots: &[(String, Value)]) -> Result<(), EngineError> {
        let hand = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .and_then(|(_, v)| Self::touch_hand_idx(&name_of(v)))
            .unwrap_or(1);
        let r_val = slots.iter().find(|(s, _)| s == "R").map(|(_, v)| v);
        let theta_val = slots.iter().find(|(s, _)| s == "THETA").map(|(_, v)| v);
        let r = r_val.and_then(num_of).unwrap_or(0.0) as f32;
        let theta = theta_val.and_then(num_of).unwrap_or(0.0) as f32;
        let r_str = r_val.map(lisp_num_str).unwrap_or_else(|| "0".to_string());
        let theta_str = theta_val.map(lisp_num_str).unwrap_or_else(|| "0".to_string());
        let hand_name = if hand == 0 { "LEFT" } else { "RIGHT" };
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: format!("PREPARE HAND-PLY HAND {hand_name} R {r_str} THETA {theta_str}"),
            },
        );
        // `prepare-features` (`motor.lisp:1106`): `move-a-hand` shifts the
        // hand loc by the polar vector (`polar-move-xy` rounds each axis).
        let (hx, hy) = self.zz_hands[hand].loc;
        let target = (
            (hx as f32 + r * theta.cos()).round() as i64,
            (hy as f32 + r * theta.sin()).round() as i64,
        );
        self.zz_hand_ply_request(
            "HAND-PLY", hand, r, theta, r_str, theta_str, target, true,
        )
    }

    /// `execute` (`general-pm.lisp:580`): queue the last prepared movement and
    /// run it.
    fn zz_execute(&mut self) -> Result<(), EngineError> {
        self.now(
            Prio::Num(0),
            Out::Medium,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: "EXECUTE".to_string(),
            },
        );
        if let Some(p) = self.zz_prepared.take() {
            self.man_exec_queue.push(p);
            self.zz_maybe_execute();
        }
        Ok(())
    }

    /// `point-hand-at-key` (`keyboard.lisp:653`): point the hand's index finger
    /// at `to-key` via a `hand-ply`, leaving the finger offsets unchanged.
    fn zz_point_hand_at_key(&mut self, slots: &[(String, Value)]) -> Result<(), EngineError> {
        let hand = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .and_then(|(_, v)| Self::touch_hand_idx(&name_of(v)))
            .unwrap_or(1);
        let to_key = slots
            .iter()
            .find(|(s, _)| s == "TO-KEY")
            .map(|(_, v)| name_of(v))
            .unwrap_or_default();
        let hand_name = if hand == 0 { "LEFT" } else { "RIGHT" };
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: format!("POINT-HAND-AT-KEY HAND {hand_name} TO-KEY {to_key}"),
            },
        );
        let Some(new_loc) = key_xy(&to_key) else {
            return Ok(());
        };
        let (hx, hy) = self.zz_hands[hand].loc;
        let dx = (new_loc.0 - hx) as f64;
        let dy = (new_loc.1 - hy) as f64;
        let r = (dx * dx + dy * dy).sqrt() as f32;
        if r == 0.0 {
            // `:686`: warning only, no movement.
            return Ok(());
        }
        let theta = dy.atan2(dx) as f32;
        let target = (
            (hx as f32 + r * theta.cos()).round() as i64,
            (hy as f32 + r * theta.sin()).round() as i64,
        );
        self.zz_hand_ply_request(
            "HAND-PLY",
            hand,
            r,
            theta,
            lisp_s_f32(r),
            lisp_s_f32(theta),
            target,
            false,
        )
    }

    /// `delayed-punch` (`motor-extension.lisp:2699`): a punch whose release is
    /// `delay` after the strike (no `key-release-time`). `delay` is
    /// `default`/`fast`/`slow`/a positive number (`parse-delay-time`, `:2685`).
    fn zz_delayed_punch(&mut self, slots: &[(String, Value)]) -> Result<(), EngineError> {
        let hand = slots
            .iter()
            .find(|(s, _)| s == "HAND")
            .and_then(|(_, v)| Self::touch_hand_idx(&name_of(v)))
            .unwrap_or(1);
        let finger = slots
            .iter()
            .find(|(s, _)| s == "FINGER")
            .and_then(|(_, v)| Self::touch_finger_idx(&name_of(v)))
            .unwrap_or(0);
        // `parse-delay-time` (`:2685`): nil/`default` -> default; a positive
        // number as-is; `slow`/`fast` -> the named delays.
        let delay_ms = match slots.iter().find(|(s, _)| s == "DELAY").map(|(_, v)| v) {
            None => self.cfg.zz_dpd_ms,
            Some(Value::Sym(s)) if s.eq_ignore_ascii_case("DEFAULT") => self.cfg.zz_dpd_ms,
            Some(Value::Sym(s)) if s.eq_ignore_ascii_case("FAST") => self.cfg.zz_fpd_ms,
            Some(Value::Sym(s)) if s.eq_ignore_ascii_case("SLOW") => self.cfg.zz_spd_ms,
            Some(v) => num_of(v)
                .filter(|x| *x > 0.0)
                .map(|x| (x * 1000.0).round() as i64)
                .unwrap_or(self.cfg.zz_dpd_ms),
        };
        let delay_str = lisp_s_f32(delay_ms as f32 / 1000.0);
        let hand_name = if hand == 0 { "LEFT" } else { "RIGHT" };
        let finger_name = ["INDEX", "MIDDLE", "RING", "PINKIE", "THUMB"][finger];
        self.now(
            Prio::Num(0),
            Out::Low,
            Event::Emit {
                module: "MOTOR".to_string(),
                line: format!(
                    "DELAYED-PUNCH HAND {hand_name} FINGER {finger_name} DELAY {delay_str}"
                ),
            },
        );
        // The device names the key from the finger's location (`motor.lisp:960`);
        // `delayed-punch` doesn't move the finger.
        let floc = self.zz_finger_loc(hand, finger);
        let key = xy_key(floc.0, floc.1).unwrap_or("").to_string();
        self.zz_key_request(
            "DELAYED-PUNCH",
            "DELAYED-PUNCH",
            &key,
            hand,
            finger,
            0.0,
            0.0,
            None,
            delay_ms,
        )
    }

    /// Manual request routing: only press-key in M5a.
    fn manual_request(&mut self, spec: NewChunk) -> Result<(), EngineError> {
        let (ctype, slots) = match &spec {
            NewChunk::Spec { ctype, slots, .. } => (ctype.clone(), slots.clone()),
            _ => return Err(unimpl("non-spec manual request")),
        };
        // `isa press-key key X` (no `cmd`).
        let mut cmd: Option<String> = if ctype.eq_ignore_ascii_case("PRESS-KEY") {
            Some("PRESS-KEY".to_string())
        } else if self.cfg.touch || self.cfg.zz_ext {
            // act-touch / extended-motor-actions request chunk types carry
            // the command as their type name (`extend-manual-requests-fct`).
            let t = ctype.to_uppercase();
            if matches!(
                t.as_str(),
                "TAP" | "TAP-HOLD" | "TAP-RELEASE" | "SWIPE" | "PINCH" | "ROTATE"
                    | "MOVE-HAND-TOUCH" | "HAND-TO-TOUCH"
                    | "MOVE-CURSOR" | "CLICK-MOUSE" | "HOLD-MOUSE" | "RELEASE-MOUSE"
                    | "HOLD-KEY" | "HIT-KEY" | "TYPE-KEY" | "MOVE-TO-KEY" | "RELEASE-KEY"
                    | "RELEASE-KEY-TO-HOME" | "MOVE-FINGER-TO-HOME" | "ALL-FINGERS-TO-HOME"
                    | "HAND-TO-MOUSE" | "HAND-TO-HOME"
                    | "HOLD-PUNCH" | "HOLD-PECK" | "PUNCH" | "PECK" | "PECK-RECOIL"
                    | "POINT-FINGER" | "RELEASE"
                    | "PREPARE" | "EXECUTE" | "POINT-HAND-AT-KEY" | "DELAYED-PUNCH"
                    | "RELEASE-ALL-FINGERS"
            ) {
                Some(t)
            } else {
                None
            }
        } else {
            None
        };
        let mut key: Option<String> = None;
        for (s, v) in &slots {
            match (s.as_str(), v) {
                ("CMD", Value::Sym(c)) => cmd = Some(c.to_uppercase()),
                ("KEY", Value::Sym(k)) => key = Some(k.clone()),
                ("KEY", Value::Str(k)) => key = Some(k.clone()),
                ("KEY", Value::Int(i)) => key = Some(i.to_string()),
                ("KEY", Value::Num(x)) => key = Some(lisp_s_f32(*x as f32)),
                _ if self.cfg.touch || self.cfg.zz_ext => {}
                got => {
                    return Err(unimpl(format!("manual request slot {got:?}")));
                }
            }
        }
        match (cmd.as_deref(), key) {
            (Some("PRESS-KEY"), Some(k)) => {
                Self::key_spec(&k)?;
                let req_ms = self.time_ms;
                // `cmd press-key` vs `isa press-key` (no cmd).
                let isa = !slots
                    .iter()
                    .any(|(s, _)| s == "CMD");
                self.now(
                    Prio::Num(0),
                    Out::Low,
                    Event::PressKey { key: k, req_ms, isa },
                );
                Ok(())
            }
            (Some(c), None)
                if self.cfg.touch
                    && matches!(
                        c,
                        "TAP" | "TAP-HOLD" | "TAP-RELEASE" | "SWIPE" | "PINCH" | "ROTATE"
                            | "MOVE-HAND-TOUCH" | "HAND-TO-TOUCH"
                    ) =>
            {
                self.touch_request(&slots)
            }
            (Some(c), None)
                if self.cfg.zz_ext
                    && matches!(
                        c,
                        "MOVE-CURSOR" | "CLICK-MOUSE" | "HOLD-MOUSE" | "RELEASE-MOUSE"
                    ) =>
            {
                self.zz_mouse_request(c, &slots)
            }
            (Some(c), _)
                if self.cfg.zz_ext
                    && matches!(
                        c,
                        "HOLD-KEY" | "HIT-KEY" | "TYPE-KEY" | "MOVE-TO-KEY" | "RELEASE-KEY"
                            | "RELEASE-KEY-TO-HOME" | "MOVE-FINGER-TO-HOME"
                            | "ALL-FINGERS-TO-HOME" | "HAND-TO-MOUSE" | "HAND-TO-HOME"
                    ) =>
            {
                self.zz_key_cmd(c, &slots)
            }
            (Some(c), _)
                if self.cfg.zz_ext
                    && matches!(
                        c,
                        "HOLD-PUNCH" | "HOLD-PECK" | "PUNCH" | "PECK" | "PECK-RECOIL"
                            | "POINT-FINGER" | "RELEASE"
                    ) =>
            {
                self.zz_direct_style(c, &slots)
            }
            (Some("PREPARE"), _) if self.cfg.zz_ext => self.zz_prepare(&slots),
            (Some("EXECUTE"), _) if self.cfg.zz_ext => self.zz_execute(),
            (Some("POINT-HAND-AT-KEY"), _) if self.cfg.zz_ext => {
                self.zz_point_hand_at_key(&slots)
            }
            (Some("DELAYED-PUNCH"), _) if self.cfg.zz_ext => self.zz_delayed_punch(&slots),
            (Some("RELEASE-ALL-FINGERS"), _) if self.cfg.zz_ext => {
                self.zz_release_all_fingers()
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
    /// The xyz vector of a location chunk (`xyz-loc`, `vision.lisp:1839`):
    /// `(screen-x screen-y distance)`, the distance being
    /// `viewing-distance × pixels-per-inch` (18 × 60 = 1080).
    fn emma_loc_xyz(&self, loc: &str) -> (f32, f32, f32) {
        let (x, y) = self
            .vl_xy
            .get(loc)
            .copied()
            .or_else(|| {
                self.features
                    .iter()
                    .find(|f| f.name == loc)
                    .map(|f| (f.x, f.y))
            })
            .unwrap_or((0, 0));
        (x as f32, y as f32, 1080.0)
    }

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
    // ------------------------------------------------------------------
    // EMMA (`extras/emma/emma.lisp`): eye-movement timing and scheduling.
    // ------------------------------------------------------------------

    /// `emma-attention-hook` (`emma.lisp:502`): `move-attention` calls this
    /// instead of the plain encoding schedule when `:emma` is enabled.
    fn emma_attention_hook(&mut self, end_loc: (f32, f32, f32)) {
        // `next-destination` (`:546`): the current marker when the eye is
        // free, otherwise where it will land.
        let start = if !self.emma_exec_s {
            self.emma_eye
        } else {
            self.emma_next_loc.unwrap_or(self.emma_eye)
        };
        let return_obj = self.emma_obj_at(end_loc);
        let sacc = self.emma_compute_saccade(start, end_loc);
        let dur = self.emma_recog_time(sacc.r);
        self.emma_shift_target = return_obj;
        self.emma_final_shift = end_loc;
        self.emma_initiate_eye_move(dur, sacc);
    }

    /// `get-obj-at-location` (`vision.lisp:2485`): the object at `loc`, or
    /// none. Building one consumes an object number (`TEXT{n}`), matching
    /// the ref's TEXT0/TEXT1 then TEXT2 from the encoding.
    fn emma_obj_at(&mut self, loc: (f32, f32, f32)) -> Option<String> {
        // `within-move` (`vision.lisp:2367`): features within the movement
        // allowance of the location (default `:visual-movement-tolerance`).
        let idx = self.features.iter().position(|f| {
            let fl = (f.x as f32, f.y as f32, loc.2);
            angle_between(loc, fl) <= 1.0
        })?;
        let kind = self.features[idx].kind.clone();
        let k = self.obj_built.entry(kind.clone()).or_insert(0);
        let name = format!("{kind}{k}");
        *k += 1;
        Some(name)
    }

    /// `compute-saccade-r-theta` (`emma.lisp:675`).
    fn emma_compute_saccade(&mut self, start: (f32, f32, f32), end: (f32, f32, f32)) -> EmmaSacc {
        let stddev = 0.1 * vec_dist(start, end);
        let ex = lisp_round(add_gaussian_noise(&mut self.rng.borrow_mut(), end.0, stddev)) as f32;
        let ey = lisp_round(add_gaussian_noise(&mut self.rng.borrow_mut(), end.1, stddev)) as f32;
        let new_loc = (ex, ey, end.2);
        let r = angle_between(start, new_loc);
        let theta = (start.1 - end.1).atan2(end.0 - start.0);
        let fprep_time = self.emma_compute_prep_time(r, theta);
        let exec_time = self.emma_compute_exec_time(r);
        EmmaSacc {
            r,
            theta,
            trgt: None,
            new_loc,
            fprep_time,
            exec_time,
        }
    }

    /// `compute-prep-time` (`general-pm.lisp:670`): 3 features when there is
    /// no previous movement, else the differing r/theta features
    /// (`feat-differences`, `emma.lisp:482`; `distance=`/`direction=`,
    /// `motor.lisp:888`/`:894`).
    fn emma_compute_prep_time(&self, r: f32, theta: f32) -> f64 {
        let base = self.cfg.emma_feat_prep as f64;
        match &self.emma_last_prep {
            None => base * 3.0,
            Some(prev) => {
                let mut n = 0.0;
                if !(2.0 > (r - prev.r).abs()) {
                    n += 1.0;
                }
                let delta = (theta - prev.theta).abs();
                let same = (std::f32::consts::PI / 4.0) > delta
                    || delta > (7.0 / 4.0 * std::f32::consts::PI);
                if !same {
                    n += 1.0;
                }
                base * n
            }
        }
    }

    /// `compute-exec-time` (`emma.lisp:461`): `init + r·rate + base`.
    fn emma_compute_exec_time(&self, r: f32) -> f64 {
        (self.cfg.emma_init_time + r * self.cfg.emma_sacc_rate + self.cfg.emma_base_exe) as f64
    }

    /// `recog-time` (`emma.lisp:696`): `K·(−ln f)·e^(r·k)`.
    fn emma_recog_time(&self, r: f32) -> f64 {
        let freq = self.cfg.emma_obj_freq;
        (self.cfg.emma_enc_factor * (-freq.ln()) * (r * self.cfg.emma_enc_exp).exp()) as f64
    }

    /// `total-time` (`emma.lisp:494`): prep + exec.
    fn emma_total_time(sacc: &EmmaSacc) -> f64 {
        sacc.fprep_time + sacc.exec_time
    }

    /// `initiate-eye-move` (`emma.lisp:566`).
    fn emma_initiate_eye_move(&mut self, recog_time: f64, mut sacc: EmmaSacc) {
        sacc.trgt = self.emma_shift_target.clone();
        self.emma_shift_start_ms = self.time_ms;
        self.emma_prep_s = true;
        self.emma_last_prep = Some(sacc.clone());
        let at = self.time_ms + (sacc.fprep_time.max(0.0) * 1000.0).round() as i64;
        self.push(
            at,
            Prio::Num(0),
            Out::Medium,
            Event::EmmaPrepComplete {
                trgt: sacc.trgt.clone(),
            },
        );
        if recog_time <= Self::emma_total_time(&sacc) {
            self.emma_schedule_encoding_complete(recog_time);
            self.emma_encoded = true;
        } else {
            self.emma_encoded = false;
        }
    }

    /// `schedule-encoding-complete` (`vision.lisp:3043`): the plain VISION
    /// encoding at the last attention location.
    fn emma_schedule_encoding_complete(&mut self, delay: f64) {
        let Some(loc) = self.last_attention.clone() else {
            return;
        };
        let at = self.time_ms + (delay.max(0.0) * 1000.0).round() as i64;
        self.push(
            at,
            Prio::Num(0),
            Out::Medium,
            Event::EncodingComplete {
                loc,
                requested: true,
            },
        );
    }

    /// `preparation-complete` + `maybe-execute-movement` (`general-pm.lisp`).
    fn emma_preparation_complete(&mut self) {
        self.emma_prep_s = false;
        let Some(sacc) = self.emma_last_prep.clone() else {
            return;
        };
        if !self.emma_exec_s {
            self.emma_perform_movement(sacc);
        }
    }

    /// EMMA's `perform-movement` override (`emma.lisp:653`).
    fn emma_perform_movement(&mut self, sacc: EmmaSacc) {
        self.emma_exec_s = true;
        self.emma_next_loc = Some(sacc.new_loc);
        let init = sacc.exec_time.max(0.0).min(self.cfg.emma_init_time as f64);
        let init_at = self.time_ms + (init * 1000.0).round() as i64;
        self.push(init_at, Prio::Num(0), Out::Medium, Event::EmmaInitComplete);
        let move_at = self.time_ms + (sacc.exec_time.max(0.0) * 1000.0).round() as i64;
        self.push(
            move_at,
            Prio::Num(0),
            Out::Medium,
            Event::EmmaEyeMoveComplete { sacc },
        );
    }

    /// `complete-eye-move` (`emma.lisp:620`): land, then re-fixate when the
    /// object was not encoded before the movement finished.
    fn emma_complete_eye_move(&mut self, sacc: EmmaSacc) {
        self.emma_eye = sacc.new_loc;
        if !self.emma_encoded {
            let new_target = self.emma_obj_at(self.emma_final_shift);
            let start = self.emma_eye;
            let end = self.emma_final_shift;
            let sacc2 = self.emma_compute_saccade(start, end);
            let elapsed = (self.time_ms - self.emma_shift_start_ms) as f64 / 1000.0;
            let frac = if self.emma_shift_duration != 0.0 {
                elapsed / self.emma_shift_duration
            } else {
                0.0
            };
            let new_duration = self.emma_recog_time(sacc2.r) * (1.0 - frac);
            self.emma_shift_target = new_target;
            self.emma_shift_start_ms = self.time_ms;
            self.emma_shift_duration = new_duration;
            self.emma_initiate_eye_move(new_duration, sacc2);
        }
        self.emma_exec_s = false;
    }

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
        // Window-item moves: re-apply the captured position change to the
        // visicon feature carrying that text value (the model's `!eval!`
        // schedules `move-it`, which the engine replays as a no-op; the
        // captured op supplies the coordinates).
        let mut moved = Vec::new();
        self.pending_modifies.retain(|m| {
            if m.at_ms <= now {
                moved.push(m.clone());
                false
            } else {
                true
            }
        });
        for m in &moved {
            if let Some(f) = self.features.iter_mut().find(|f| f.value == m.text) {
                f.x = m.x + 305;
                f.y = m.y + 306;
                f.tstamp_ms = now;
            }
        }
        // `update-attended-loc` (`vision.lisp:1870`): re-encode only when
        // the attended object is gone (a clear) or was modified. A new text
        // elsewhere leaves it untouched, so adding texts must not trigger
        // this (paired-static: the old location stays). EMMA re-invokes its
        // hook instead (a fresh saccade); the base module re-schedules
        // `encoding-complete` after the attention latency.
        if cleared {
            if let Some(loc) = self.last_attention.clone() {
                if self.cfg.emma {
                    let end = self.emma_loc_xyz(&loc);
                    self.emma_attention_hook(end);
                } else {
                    // The re-encode occupies the visual module, so
                    // `?visual> state free` fails until it lands
                    // (`update-attended-loc` -> `encode-visual-object`).
                    self.visual_busy = true;
                    let at = self.time_ms + self.randomize_ms(self.cfg.attention_ms);
                    self.push(
                        at,
                        Prio::Num(0),
                        Out::Medium,
                        Event::EncodingComplete {
                            loc,
                            requested: false,
                        },
                    );
                }
            }
        }
        // The environment returns window items column-major, so the
        // `VISUAL-LOCATION{n}` names follow `(screen-x, screen-y)` order
        // (verified against `sperling`: the input order `V N T Z / C R Y K /
        // W J G F` names `V C W N R J T Y G Z K F` as `0..11`).
        due.sort_by_key(|t| (t.x, t.y));
        for t in due {
            let n = self.feat_seq;
            self.feat_seq += 1;
            // `add-text-to-exp-window` places a text item relative to the
            // window by its top-left, and the visicon uses the item's
            // centre: `screen-x = window-x + text-x + width/2`,
            // `screen-y = window-y + text-y + height/2` (verified against
            // SBCL). The test-model windows use the default origin
            // (300, 300) and the font is 7 px/char, 10 px tall, so the
            // offsets are `305 + (width-7)/2` and `306`.
            let width = 7 * t.text.chars().count() as i64;
            self.features.push(Feature {
                name: format!("VISUAL-LOCATION{n}"),
                kind: "TEXT".to_string(),
                value: t.text.clone(),
                x: t.x + 305 + (width - 7) / 2,
                y: t.y + 306,
                width,
                height: 10,
                extra: Vec::new(),
                tstamp_ms: now,
                attended: false,
            });
        }
        let empty = self
            .buffers
            .get("VISUAL-LOCATION")
            .and_then(|b| b.chunk.as_ref())
            .is_none();
        if empty && self.tracked.is_none() {
            if let Some(feat) = self.newest_feature() {
                let base = feat.name.clone();
                self.place_vl(&base, false)?;
            }
        }
        // Tracking re-syncs the vision buffers to the moved object
        // (`update-tracking-mth`) before the visicon update is announced.
        if self.tracked.is_some() {
            self.update_tracking(true);
        }
        self.now(Prio::Num(0), Out::Medium, Event::VisiconUpdate);
        // Schedule the next wave for remaining texts, moves or clears.
        let mut next_at: Option<i64> = None;
        if let Some(next) = self.pending_texts.first() {
            next_at = Some(next.at_ms);
        }
        if let Some(m) = self.pending_modifies.first() {
            next_at = Some(next_at.map_or(m.at_ms, |n| n.min(m.at_ms)));
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
        self.place_vl_at(base, requested, 10)
    }

    fn place_vl_at(&mut self, base: &str, requested: bool, prio: i64) -> Result<(), EngineError> {
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
        self.vl_copied_from.insert(copy.clone(), base.to_string());
        self.vl_xy.insert(copy.clone(), (feat.x, feat.y));
        let inst = ChunkInst {
            name: Some(copy.clone()),
            ctype: "VISUAL-LOCATION".to_string(),
            slots: feat.loc_slots(),
            bare: false,
        };
        self.buffer_mut("VISUAL-LOCATION").failed = false;
        self.buffer_mut("VISUAL-LOCATION").modified = false;
        self.now(
            Prio::Num(prio),
            Out::Low,
            Event::SetBuffer {
                buffer: "VISUAL-LOCATION".to_string(),
                chunk: NewChunk::Instance(inst),
                requested,
                shown: Some(base.to_string()),
            },
        );
        // `stuff-visloc-buffer` (`vision.lisp:1995`): an auto-stuffed
        // (unrequested) location is unstuffed after `new-span`, unless
        // `:unstuff-visual-location` is nil.
        if !requested {
            if let Some(delay) = self.cfg.unstuff_vl_ms {
                self.push_m(
                    self.time_ms + delay,
                    Prio::Num(0),
                    // `unstuff-buffer` schedules `erase-buffer` with
                    // `schedule-event-now`'s default `:output t`
                    // (`support/general-pm.lisp:899`, `scheduling.lisp:1292`):
                    // high detail only.
                    Out::Always,
                    true,
                    Event::UnstuffVl { copy: copy.clone() },
                );
            }
        }
        Ok(())
    }

    /// The feature name a visual-location copy aliases (`chunk-visicon-entry`).
    fn loc_feature(&self, copy: &str) -> Option<String> {
        self.vl_copies
            .get(copy)
            .map(|&i| self.features[i].name.clone())
    }

    /// The source chunk a visual-buffer copy was made from
    /// (`chunk-copied-from-fct`), for the `vis-obj-marker` test: the
    /// `VISUAL-CHUNK0` copy of `VO-1` is not the tracked object.
    fn obj_source(&self, name: &str) -> String {
        self.vl_copied_from
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// `convert-loc-to-object`: build a fresh visual-object chunk for the
    /// tracked feature. The object counter advances on every tracking
    /// update even when the chunk is only used for a modification
    /// (`vision.lisp:2762`).
    fn convert_loc_to_object(&mut self, feat: &Feature) -> ChunkInst {
        let k = self.obj_built.entry(feat.kind.clone()).or_insert(0);
        let name = format!("{}{}", feat.kind, *k);
        *k += 1;
        self.obj_feature
            .insert(name.clone(), feat.name.clone());
        self.obj_feature
            .insert("VISUAL-CHUNK0".to_string(), feat.name.clone());
        let pos = self
            .buffers
            .get("VISUAL-LOCATION")
            .and_then(|b| b.chunk.as_ref())
            .and_then(|c| c.name.clone())
            .unwrap_or_else(|| feat.name.clone());
        ChunkInst {
            name: Some(name),
            ctype: "VISUAL-OBJECT".to_string(),
            slots: Self::object_slots(feat, &pos),
            bare: false,
        }
    }

    /// The visual-object chunk slots (`SCREEN-POS`, `VALUE`, `COLOR`,
    /// `HEIGHT`, `WIDTH`, `TEXT`).
    fn object_slots(feat: &Feature, screen_pos: &str) -> Vec<(String, Value)> {
        vec![
            ("SCREEN-POS".to_string(), Value::Sym(screen_pos.to_string())),
            ("VALUE".to_string(), Value::Str(feat.value.clone())),
            ("COLOR".to_string(), Value::Sym("BLACK".to_string())),
            ("HEIGHT".to_string(), Value::Int(feat.height)),
            ("WIDTH".to_string(), Value::Int(feat.width)),
            ("TEXT".to_string(), Value::Sym("T".to_string())),
        ]
    }

    /// `set-vis-loc`: stuff the tracked object's location chunk.
    fn track_set_vis_loc(&mut self, feature: &str) {
        let _ = self.place_vl_at(feature, false, 15);
    }

    /// `mod-vis-loc`: refresh the location chunk in place. A modified chunk
    /// is no longer a copy (`chunk-copied-from-fct` clears).
    fn track_mod_vis_loc(&mut self, feature: &str) {
        if let Some(cur) = self
            .buffers
            .get("VISUAL-LOCATION")
            .and_then(|b| b.chunk.as_ref())
            .and_then(|c| c.name.clone())
        {
            self.vl_copied_from.remove(&cur);
        }
        if let Some(f) = self.features.iter().find(|f| f.name == feature).cloned() {
            self.now(
                Prio::Num(15),
                Out::Always,
                Event::ModBuffer {
                    buffer: "VISUAL-LOCATION".to_string(),
                    slots: vec![
                        ("SCREEN-X".to_string(), Value::Int(f.x)),
                        ("SCREEN-Y".to_string(), Value::Int(f.y)),
                    ],
                    module: "VISION".to_string(),
                },
            );
        }
    }

    /// The copy name `place_vl_at` will use for `base` next.
    fn next_vl_copy(&self, base: &str) -> String {
        format!("{base}-{}", *self.copy_count.get(base).unwrap_or(&0))
    }

    /// `set-vis-obj`: stuff the fresh object into the visual buffer at the
    /// location chunk `pos`. The buffer holds a `VISUAL-CHUNK0` copy; the
    /// trace shows the source object name. `copy` marks the `(null
    /// vis-obj-chunk)` branch, which copies (`chunk-copied-from-fct`).
    fn track_set_vis_obj(&mut self, obj: &ChunkInst, pos: &str, copy: bool) {
        let source = obj.name.clone().unwrap_or_default();
        let mut obj = obj.clone();
        obj.name = Some("VISUAL-CHUNK0".to_string());
        if let Some((_, v)) = obj.slots.iter_mut().find(|(k, _)| k == "SCREEN-POS") {
            *v = Value::Sym(pos.to_string());
        }
        if copy {
            self.vl_copied_from
                .insert("VISUAL-CHUNK0".to_string(), source.clone());
        } else {
            self.vl_copied_from.remove("VISUAL-CHUNK0");
        }
        self.now(
            Prio::Num(14),
            Out::Always,
            Event::SetBuffer {
                buffer: "VISUAL".to_string(),
                chunk: NewChunk::Instance(obj),
                requested: false,
                shown: Some(source),
            },
        );
    }

    /// `mod-vis-obj`: refresh the object's `screen-pos`. A modified chunk is
    /// no longer a copy (`chunk-copied-from-fct` clears).
    fn track_mod_vis_obj(&mut self, pos: &str) {
        self.vl_copied_from.remove("VISUAL-CHUNK0");
        self.now(
            Prio::Num(14),
            Out::Always,
            Event::ModBuffer {
                buffer: "VISUAL".to_string(),
                slots: vec![("SCREEN-POS".to_string(), Value::Sym(pos.to_string()))],
                module: "VISION".to_string(),
            },
        );
    }

    /// `update-tracking-loc-chunk`: record the location chunk; `modify` also
    /// refreshes the visual object's `screen-pos` (silently, `:output nil`).
    fn track_update_loc(&mut self, modify: bool, pos: &str) {
        self.tracked_lastloc = Some(pos.to_string());
        if modify {
            self.now(
                Prio::Num(13),
                Out::Never,
                Event::ModBuffer {
                    buffer: "VISUAL".to_string(),
                    slots: vec![("SCREEN-POS".to_string(), Value::Sym(pos.to_string()))],
                    module: "VISION".to_string(),
                },
            );
        }
    }

    /// `tracking-failed`: the tracked object left the visicon; clear the
    /// visual buffer and stop tracking.
    fn tracking_failed(&mut self) {
        self.tracked = None;
        self.tracked_lastloc = None;
        self.last_attention = None;
        self.visual_busy = false;
        self.now(Prio::Num(13), Out::Always, Event::VisualClear);
        self.buffer_mut("VISUAL").failed = true;
    }

    /// `update-tracking-mth` (`vision.lisp:2706`): re-sync the vision
    /// buffers to the tracked object's new position.
    fn update_tracking(&mut self, _from_proc_display: bool) {
        let Some(tracked) = self.tracked.clone() else {
            return;
        };
        let Some(feat) = self.features.iter().find(|f| f.name == tracked).cloned() else {
            self.tracking_failed();
            return;
        };
        let new_obj = self.convert_loc_to_object(&feat);
        let vis_loc = self
            .buffers
            .get("VISUAL-LOCATION")
            .and_then(|b| b.chunk.as_ref())
            .and_then(|c| c.name.clone());
        let vis_obj = self
            .buffers
            .get("VISUAL")
            .and_then(|b| b.chunk.as_ref())
            .and_then(|c| c.name.clone());
        let obj_marker = vis_obj
            .as_ref()
            .and_then(|n| self.obj_feature.get(&self.obj_source(n)).cloned());
        let mut old_loc = self.tracked_lastloc.clone();
        if old_loc.is_none() {
            if let Some(vlc) = &vis_loc {
                if self.loc_feature(vlc).as_deref() == Some(tracked.as_str()) {
                    old_loc = Some(vlc.clone());
                }
            }
        }
        let loc_is_old = vis_loc.is_some() && vis_loc == old_loc;
        let obj_is_tracked = obj_marker.as_deref() == Some(tracked.as_str());
        // The location chunk the buffers hold after a `set-vis-loc`.
        let stuffed = self.next_vl_copy(&tracked);
        if vis_loc.is_none() && vis_obj.is_none() {
            self.track_set_vis_loc(&tracked);
            self.track_set_vis_obj(&new_obj, &stuffed, false);
            self.track_update_loc(true, &stuffed);
        } else if vis_loc.is_none() && obj_is_tracked {
            self.track_set_vis_loc(&tracked);
            self.track_mod_vis_obj(&stuffed);
            self.track_update_loc(true, &stuffed);
        } else if vis_loc.is_none() {
            self.track_set_vis_loc(&tracked);
            self.track_update_loc(false, &stuffed);
        } else if loc_is_old && vis_obj.is_none() {
            let loc = vis_loc.clone().unwrap();
            self.track_mod_vis_loc(&tracked);
            self.track_set_vis_obj(&new_obj, &loc, false);
            self.track_update_loc(true, &loc);
        } else if loc_is_old && obj_is_tracked {
            let loc = vis_loc.clone().unwrap();
            self.track_mod_vis_loc(&tracked);
            self.track_mod_vis_obj(&loc);
            self.track_update_loc(true, &loc);
        } else if loc_is_old {
            self.track_mod_vis_loc(&tracked);
        } else if vis_obj.is_none() {
            // The visual buffer holds a non-tracked location, so the object's
            // `screen-pos` is the tracked feature's own chunk.
            self.track_set_vis_obj(&new_obj, &tracked, true);
        } else if obj_is_tracked {
            // The visual buffer holds the tracked object but the location
            // buffer holds some other chunk, so `screen-pos` is the tracked
            // feature's own chunk.
            self.track_mod_vis_obj(&tracked);
        }
    }

    fn newest_feature(&mut self) -> Option<Feature> {
        let top = self.features.iter().map(|f| f.tstamp_ms).max()?;
        // The visicon keeps entries in creation order (window items
        // column-major), but `stuff-visloc-buffer` (`vision.lisp:1978`)
        // sorts the max-tstamp objects by symbol name before
        // `random-item`.
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
        self.rng.borrow_mut().random_int(n as u64) as usize
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
    fn encoding_complete(&mut self, loc: String, requested: bool) -> Result<(), EngineError> {
        let idx = match self.resolve_loc(&loc) {
            Ok(i) if i < self.features.len() => i,
            _ => {
                // The attended object disappeared before the encoding
                // landed (e.g. the window was cleared underneath it):
                // vision reports no object and leaves VISUAL unchanged.
                // `:output medium` + `:maintenance t` (`vision.lisp:2436`).
                if self.cfg.detail.shows(Out::Medium) {
                    self.emit("VISION", "No visual-object found".into());
                }
                self.visual_busy = false;
                // Becoming free again wakes the armed conflict-resolution
                // (`conditions-met`).
                self.changed();
                return Ok(());
            }
        };
        let feat = self.features[idx].clone();
        let k = self.obj_built.entry(feat.kind.clone()).or_insert(0);
        let source = format!("{}{}", feat.kind, *k);
        *k += 1;
        // The VISUAL buffer holds a `VISUAL-CHUNK{n}` copy; the trace shows
        // the source object name (`TEXT{n}`).
        self.obj_feature
            .insert(source.clone(), feat.name.clone());
        self.obj_feature
            .insert("VISUAL-CHUNK0".to_string(), feat.name.clone());
        let inst = ChunkInst {
            name: Some("VISUAL-CHUNK0".to_string()),
            ctype: "VISUAL-OBJECT".to_string(),
            slots: Self::object_slots(&feat, &loc),
            bare: false,
        };
        if let Ok(idx) = self.resolve_loc(&loc) {
            self.features[idx].attended = true;
        }
        self.visual_busy = false;
        // An encoded object is not a copy (no `chunk-copied-from-fct`).
        self.vl_copied_from.remove("VISUAL-CHUNK0");
        self.now(
            Prio::Num(10),
            Out::Low,
            Event::SetBuffer {
                buffer: "VISUAL".to_string(),
                chunk: NewChunk::Instance(inst),
                requested,
                shown: Some(source),
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
        let mut v = vec![
            ("KIND".to_string(), Value::Sym(self.kind.clone())),
            ("VALUE".to_string(), Value::Sym(self.kind.clone())),
            ("COLOR".to_string(), Value::Sym("BLACK".to_string())),
            ("HEIGHT".to_string(), Value::Int(self.height)),
            ("WIDTH".to_string(), Value::Int(self.width)),
            ("SCREEN-X".to_string(), Value::Int(self.x)),
            ("SCREEN-Y".to_string(), Value::Int(self.y)),
            ("DISTANCE".to_string(), Value::Int(1080)),
            // `simple-size` (`vision.lisp:3698`): pixels→angle product,
            // rounded to 0.01.
            ("SIZE".to_string(), Value::Num(self.size())),
        ];
        v.extend(self.extra.iter().cloned());
        v
    }

    /// `simple-size` (`vision.lisp:3698`): `0.01 * round(angle(w) *
    /// angle(h) / 0.01)`, with `angle(px) = 2*atan(px/2, 1080)` in degrees
    /// and single-float arithmetic (`round` returns the integer quotient).
    fn size(&self) -> f64 {
        let ang = |px: i64| (2.0 * ((px as f64) / 2.0 / 1080.0).atan()).to_degrees();
        let prod = ang(self.width) * ang(self.height);
        let q = (prod / 0.01).round();
        (0.01f32 * q as f32) as f64
    }
}

/// Visual-location request matching over features. Unconstrained
/// requests match everything; unknown slots error.
fn loc_matches(feat: &Feature, want: &[(String, Value)]) -> Result<bool, EngineError> {
    for (s, v) in want {
        // Request parameters are written with a leading colon (`:attended`,
        // `:nearest`, ...); treat them as the plain slot for matching.
        let s = s.trim_start_matches(':');
        match (s, v) {
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
            ("ATTENDED", Value::Nil) => {
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
/// Euclidean distance between two 3-vectors (`dist`, `misc-utils.lisp:1156`).
fn vec_dist(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
}

/// `angle-between-vectors` (`vision.lisp:2386`) in degrees.
fn angle_between(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let num = b.2;
    let den = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2) + b.2.powi(2)).sqrt();
    (num / den).clamp(-1.0, 1.0).acos().to_degrees()
}

/// `add-gaussian-noise` (`emma.lisp:730`): `x` plus a logistic sample of
/// scale `sqrt(3·stddev²)/π`.
fn add_gaussian_noise(rng: &mut Mt, x: f32, stddev: f32) -> f32 {
    let s = (3.0f32 * stddev * stddev).sqrt() / std::f32::consts::PI;
    x + if s == 0.0 { 0.0 } else { rng.noise(s) }
}

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
    format!("{}{}.{:03}", if ms < 0 { "-" } else { "" }, a / 1000, a % 1000)
}

/// `~s` of an `:ol` value in the base-level trace.
fn ol_str(ol: &Ol) -> String {
    match ol {
        Ol::On => "T".to_string(),
        Ol::Off => "NIL".to_string(),
        Ol::Num(n) => lisp_f(*n as f32),
    }
}

/// SBCL `~S`/`~A` for a single-float: shortest round-tripping decimal,
/// rendered fixed for decimal exponents in `-3..=6` and in `m.mmmmmmeN`
/// form otherwise (e.g. `3.7122225e-6`, `1.0e7`). The blending trace prints
/// its floats with this form (the refs are the arbiter).
/// A symbol/string value's name (act-touch slot helpers).
fn name_of(v: &Value) -> String {
    match v {
        Value::Sym(s) | Value::Str(s) => s.clone(),
        _ => String::new(),
    }
}

/// A numeric value as `f64`, if it is one.
fn num_of(v: &Value) -> Option<f64> {
    match v {
        Value::Int(i) => Some(*i as f64),
        Value::Num(x) => Some(*x),
        _ => None,
    }
}

/// A numeric value as Lisp `~a` would print it: an integer stays an integer
/// (`r 1` -> `1`) while a float gets the single-float spelling (`1.0`).
fn lisp_num_str(v: &Value) -> String {
    match v {
        Value::Int(i) => i.to_string(),
        Value::Num(x) => lisp_s_f32(*x as f32),
        _ => String::new(),
    }
}

/// A numeric value, as an integer when integral (ACT-R preserves the
/// `generate-tracker-choices` integer choices).
fn num_value(x: f64) -> Value {
    if x.fract() == 0.0 && x.abs() < 9e15 {
        Value::Int(x as i64)
    } else {
        Value::Num(x)
    }
}

/// `~f`/`~a` single-float display for the touch signals.
/// `direction=` (`motor.lisp:894`): two directions are equal if within π/4.
fn direction_eq(d1: f32, d2: f32) -> bool {
    let delta = (d1 - d2).abs();
    (std::f32::consts::FRAC_PI_4 > delta) || (delta > 7.0 * std::f32::consts::PI / 4.0)
}

/// `minjerk-dist` (`motor.lisp:1256`): distance moved at time `tm`, given max
/// distance `a` and total move time `d`.
fn minjerk_dist(tm: f32, a: f32, d: f32) -> f32 {
    if tm >= d {
        a
    } else if tm <= 0.0 || d <= 0.0 {
        0.0
    } else {
        let td = tm / d;
        a * (10.0 * td.powi(3) - 15.0 * td.powi(4) + 6.0 * td.powi(5))
    }
}

fn lisp_s_f32(x: f32) -> String {    if !x.is_finite() {
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
            // `expt-coerced` (`misc-utils.lisp:515`): SBCL's `expt` on singles
            // goes through the double `(exp (* y (log x)))` and coerces, which
            // differs from Rust's `f32::powf` by an ulp (the refs adjudicate).
            let p = ((-decay as f64) * (t as f64).ln()).exp() as f32;
            (((count * p / denom) as f64).ln()) as f32
        }
    }
}

/// `exp-coerced` (`misc-utils.lisp:554`): SBCL's `exp` on a single-float
/// computes in double and rounds, which differs from Rust's `f32::exp` by an
/// ulp (the associative-learning refs adjudicate).
fn exp_coerced(x: f32) -> f32 {
    ((x as f64).exp()) as f32
}

/// `log-coerced` (`misc-utils.lisp:580`): SBCL's `log` on a single-float
/// computes in double and rounds.
fn log_coerced(x: f32) -> f32 {
    ((x as f64).ln()) as f32
}

/// `needed-times` (`associative-learning.lisp:704`): with `:ol t` the history
/// lists are never populated (they stay `nil`), which is why every
/// `… with history NIL` line is nil.
fn al_needed_refs(ol: &Ol, refs: &mut Vec<i64>, ct: i64) {
    if matches!(ol, Ol::On) {
        return;
    }
    refs.insert(0, ct);
    if let Ol::Num(n) = ol {
        let keep = (*n as usize).min(refs.len());
        refs.truncate(keep);
    }
}

/// `~s` of a reference history list (`NIL` when empty).
fn al_history(refs: &[i64]) -> String {
    if refs.is_empty() {
        "NIL".to_string()
    } else {
        format!(
            "({})",
            refs.iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        )
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

    fn concretize(
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
fn implicit_clears(prod: &Production, exempt: &[String], allow_dup: bool) -> Vec<String> {
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
            LhsItem::Eval { .. }
            | LhsItem::Bind { .. }
            | LhsItem::MvBind { .. } => {}
        }
    }
    let mut targeted: Vec<(char, String)> = Vec::new();
    for a in &prod.rhs {
        match a {
            RhsAction::Modify { buffer, .. } => targeted.push(('=', buffer.clone())),
            RhsAction::ModifyRequest { buffer, .. } => {
                targeted.push(('=', buffer.clone()))
            }
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
        if *op == '+' && !targeted.iter().any(|(o, b)| *o == '-' && b == t) && (allow_dup || !clears.contains(t)) {
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
            |n: &str| implicit_clears(m.productions.iter().find(|p| p.name == n).unwrap(), &exempt, false);
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
        assert_eq!(implicit_clears(dv, &exempt, false), vec!["RETRIEVAL".to_string()]);
        // Goal is harvest-exempt even when tested-but-untargeted.
        use crate::model::Pattern as Pat;
        use crate::model::{LhsItem, Production, RhsAction, SlotTest};
        let probe = Production {
            name: "PROBE".to_string(),
            reward: None,
            at: None,
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
            implicit_clears(&probe, &no_exempt, false),
            vec!["GOAL".to_string(), "VISUAL".to_string()]
        );
        assert_eq!(implicit_clears(&probe, &exempt, false), vec!["VISUAL".to_string()]);
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

    /// The bounded `!eval!`/`!bind!` evaluator: pure operators, variables,
    /// quotes, `mp-time`, and loud failure for unknown heads.
    #[test]
    fn eval_evaluator() {
        use crate::sexpr::parse_all;
        let e = Engine::new(m0("count.lisp"), (100, 0), 10.0, DeviceSetup::default()).unwrap();
        let mut b = Bindings::new();
        b.insert("X".to_string(), Value::Int(3));
        let one = |s: &str| parse_all(s).unwrap().pop().unwrap();
        let t = Value::Sym("T".to_string());
        assert_eq!(e.eval_expr(&one("(> (mp-time) .4)"), &b).unwrap(), Value::Nil);
        assert_eq!(e.eval_expr(&one("(> (mp-time) -1)"), &b).unwrap(), t);
        assert_eq!(
            e.eval_expr(&one("(not (equal =x 'blue))"), &b).unwrap(),
            t
        );
        assert_eq!(e.eval_expr(&one("(+ =x 1)"), &b).unwrap(), Value::Int(4));
        assert_eq!(e.eval_expr(&one("(null =x)"), &b).unwrap(), Value::Nil);
        assert_eq!(
            e.eval_expr(&one("(and (> =x 2) (< =x 4))"), &b).unwrap(),
            t
        );
        assert!(e.eval_expr(&one("(this-needs-a-real-lisp)"), &b).is_err());
    }

    /// Task-file `defun`s are callable by symbol or string head, with bare
    /// parameters bound from the call scope.
    #[test]
    fn eval_user_defun() {
        use crate::model::UserDefun;
        use crate::sexpr::parse_all;
        let mut e = Engine::new(m0("count.lisp"), (100, 0), 10.0, DeviceSetup::default()).unwrap();
        let expr = |s: &str| parse_all(s).unwrap().pop().unwrap();
        let mut fns = HashMap::new();
        fns.insert(
            "SINGLE-BIND".to_string(),
            UserDefun {
                params: vec!["VAL".to_string()],
                body: vec![expr("(1+ val)")],
            },
        );
        e.set_user_fns(fns);
        let b = Bindings::new();
        assert_eq!(e.eval_expr(&expr("(single-bind 3)"), &b).unwrap(), Value::Int(4));
        assert_eq!(
            e.eval_expr(&expr("(\"single-bind\" 4)"), &b).unwrap(),
            Value::Int(5)
        );
    }

    /// `render_production` reproduces the `:cst` instantiation block
    /// (`tutorial-unit8-simple-ppm-model-initial-run.ref`).
    #[test]
    fn render_production_instantiated() {
        use crate::model::{LhsItem, Pattern as Pat, Production, RhsAction, SlotTest};
        let prod = Production {
            name: "SMALL".to_string(),
            reward: None,
            at: None,
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

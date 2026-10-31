//! `sgp` parameter classification.
//!
//! ACT-R's central parameters are a large, open set; the port implements a
//! subset. Before this table existed, `apply_sgp` ended in `_ => {}`, so an
//! unimplemented parameter (or a typo) was accepted and silently ignored.
//! `sgp_support` is the single source of truth used by `Config::from_model`,
//! `apply_sgp`, and the corpus parameter-coverage test.
//!
//! `Both` and `RuntimeOnly` entries must have an `apply_sgp` arm (the default
//! arm reports drift if one is missing). `ModelOnly` entries are read while
//! loading a model but have no runtime setter yet. `Ignored` entries are
//! accepted for corpus compatibility and carry the reason they do nothing.

/// How the engine handles one `sgp` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SgpSupport {
    /// `Config::from_model` reads it and `apply_sgp` has a runtime setter.
    Both,
    /// `Config::from_model` reads it, but there is no runtime setter.
    ModelOnly,
    /// `apply_sgp` has a runtime setter; a model-body value is not read.
    RuntimeOnly,
    /// Accepted without an engine effect (cosmetic, driver-managed, or a
    /// known parity gap). The reason is documentation for the audit.
    Ignored(&'static str),
}

/// Classify an `sgp` key. `None` means the engine does not know the
/// parameter at all; callers reject it.
pub fn sgp_support(key: &str) -> Option<SgpSupport> {
    Some(match key.to_ascii_uppercase().as_str() {
        // Read by `Config::from_model` and settable at run time.
        ":ACT" | ":ALPHA" | ":ANS" | ":AUDIO-MS-TIMES" => SgpSupport::Both,
        ":BLC" | ":BLL" | ":CIA" | ":CRT" => SgpSupport::Both,
        ":CST" | ":DAT" | ":DCNN" | ":DECAY-TIME" => SgpSupport::Both,
        ":DO-NOT-HARVEST" | ":DO-NOT-QUERY" | ":EBLSE" | ":EGS" => SgpSupport::Both,
        ":ENABLE-INHIBITION" | ":EPL" | ":ER" | ":ESC" => SgpSupport::Both,
        ":GA" | ":HEAR-SELF" | ":INHIBITION-DECAY" | ":INHIBITION-SCALE" => SgpSupport::Both,
        ":IU" | ":LF" | ":MAS" | ":MD" => SgpSupport::Both,
        ":MODEL-WARNINGS" | ":MP" | ":MS" | ":NCNAR" => SgpSupport::Both,
        ":NSJI" | ":OL" | ":OVERSTUFF-AURAL-LOCATION" | ":OVERSTUFF-VISUAL-LOCATION" => {
            SgpSupport::Both
        }
        ":PCT" | ":PPM" | ":PPM-HOOK" | ":RANDOMIZE-TIME" => SgpSupport::Both,
        ":RECORD-TICKS" | ":RIR" | ":RT" | ":SACT" => SgpSupport::Both,
        ":SE-INTERCEPT" | ":SE-SCALE" | ":SEED" | ":SIM-HOOK" => SgpSupport::Both,
        ":TOUCH-DRAG-DELAY" | ":UL" | ":ULT" | ":UNSTUFF-AURAL-LOCATION" => SgpSupport::Both,
        ":UNSTUFF-VISUAL-LOCATION" | ":VIDT" | ":VISUAL-ONSET-SPAN" => SgpSupport::Both,
        ":VPFT" => SgpSupport::Both,

        // `apply_sgp`-only: a model-body value is not read yet.
        ":EMMA" | ":LEFT-START" | ":RIGHT-START" | ":STARTING-PARAMETERS" | ":V" => {
            SgpSupport::RuntimeOnly
        }

        // `Config::from_model`-only: no runtime setter yet.
        ":AL-PRIOR-P" | ":AL-PRIOR-W" | ":AL-SHOW" | ":AUTO-ATTEND" => SgpSupport::ModelOnly,
        ":BLEND-ALL-SLOTS" | ":BLT" | ":CURSOR-FITTS-COEFF" | ":DECLARATIVE-FINST-SPAN" => {
            SgpSupport::ModelOnly
        }
        ":DECLARATIVE-NUM-FINSTS" | ":EAL" | ":EYE-SACCADE-RATE" | ":FINGER-HEIGHT" => {
            SgpSupport::ModelOnly
        }
        ":FINGER-SPACING" | ":IMAGINAL-ACTIVATION" | ":IMAGINAL-DELAY" | ":KEY-CLOSURE-TIME" => {
            SgpSupport::ModelOnly
        }
        ":LE" | ":MIN-BL" | ":MIN-FITTS-TIME" | ":MOTOR-BURST-TIME" => SgpSupport::ModelOnly,
        ":MOTOR-FEATURE-PREP-TIME" | ":MOTOR-INITIATION-TIME" | ":OPTIMIZE-VISUAL" | ":PAS" => {
            SgpSupport::ModelOnly
        }
        ":PECK-FITTS-COEFF" | ":PIXELS-PER-INCH" | ":RHST" | ":SACCADE-BASE-TIME" => {
            SgpSupport::ModelOnly
        }
        ":SACCADE-FEAT-TIME" | ":SACCADE-INIT-TIME" | ":STYLE-WARNINGS" | ":SWIPE-SPEED" => {
            SgpSupport::ModelOnly
        }
        ":THUMB-WIDTH" | ":TIME-MASTER-START-INCREMENT" | ":TIME-MULT" | ":TIME-NOISE" => {
            SgpSupport::ModelOnly
        }
        ":TMP" | ":TOUCH-ROTATE-WIDTH" | ":TRACE-DETAIL" | ":TRACE-TRACKER" => {
            SgpSupport::ModelOnly
        }
        ":TT" | ":VIS-OBJ-FREQ" | ":VISUAL-ATTENTION-LATENCY" | ":VISUAL-ENCODING-EXPONENT" => {
            SgpSupport::ModelOnly
        }
        ":VISUAL-ENCODING-FACTOR" | ":VISUAL-FINST-SPAN" | ":VISUAL-NUM-FINSTS" => {
            SgpSupport::ModelOnly
        }

        // Known no-ops. See `PARITY-AUDIT.md` for the ones that are real gaps.
        ":UAN" | ":BL-HOOK" | ":NOISE-HOOK" => {
            SgpSupport::Ignored("adaptive-noise hooks not implemented (parity audit item 2)")
        }
        ":CHUNK-MERGE-HOOK" | ":RETRIEVAL-SET-HOOK" => {
            SgpSupport::Ignored("task data hooks not implemented; drivers compute the same data")
        }
        ":RETRIEVAL-REQUEST-HOOK" => {
            SgpSupport::Ignored("retrieval-history driver records requests natively")
        }
        ":UTILITY-OFFSETS" => SgpSupport::Ignored("the PPM offset is represented by :PPM"),
        ":CACHE-SIM-HOOK-RESULTS" => {
            SgpSupport::Ignored("similarity-result caching has no observable effect")
        }
        ":USE-TREE" => SgpSupport::Ignored("conflict-set data structure choice has no effect"),
        ":CMDT" => SgpSupport::Ignored("command trace is emitted by the command layer"),
        ":TESTING-FUNCTION" => {
            SgpSupport::Ignored("session-level :starting-parameters hook (invoke_testing_fn)")
        }
        ":SHOW-FOCUS" | ":SHOW-FINGERS" | ":NEEDS-MOUSE" => {
            SgpSupport::Ignored("GUI/display flag only")
        }
        ":CURSOR-DRAG-DELAY"
        | ":DEFAULT-PUNCH-DELAY"
        | ":FAST-PUNCH-DELAY"
        | ":SLOW-PUNCH-DELAY"
        | ":PECK-STRIKE-DISTANCE"
        | ":KEY-RELEASE-TIME"
        | ":DUAL-EXECUTION-STAGES"
        | ":DUAL-PROCESSOR-STAGES" => {
            SgpSupport::Ignored("extended-motor-actions driver sets the equivalent config")
        }

        _ => return None,
    })
}

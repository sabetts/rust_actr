//! Trace formatting: byte-exact ACT-R event lines.
//!
//! Format mirrored from `+format-event-event-string+`
//! (`actr7.x/framework/events.lisp:262`) for the single-model case:
//! `~6d` seconds, `.`, zero-padded 3-digit ms, space, module name
//! left-justified to the longest module name, space, details text.
//!
//! Module width is 22 (`PRODUCTION-COMPILATION`, the longest loaded
//! module name). Hardcoded rather than computed: the golden traces pin
//! the value, so any drift from loading a different module set fails
//! loudly in `tests/golden.rs` instead of silently reformatting.

/// Event output level (`:output` on `schedule-event`).
/// Visibility rule from `filter-test`
/// (`actr7.x/framework/printing.lisp:436`): `low` shows at any detail,
/// `medium` at medium/high, `t` (Always) only at high.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Out {
    Low,
    Medium,
    /// `:output t` — shown only at `:trace-detail high`.
    Always,
    /// `:output nil` — never shown (still executes).
    Never,
}

/// Model `:trace-detail` (`:default-value 'medium`, printing.lisp:411).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    Low,
    Medium,
    High,
}

impl Detail {
    pub fn shows(self, out: Out) -> bool {
        match self {
            Detail::Low => out == Out::Low,
            Detail::Medium => matches!(out, Out::Low | Out::Medium),
            Detail::High => !matches!(out, Out::Never),
        }
    }
}

/// Longest loaded ACT-R module name (`PRODUCTION-COMPILATION`).
pub const MODULE_WIDTH: usize = 22;
/// One trace line: `     0.050   PROCEDURAL             MOD-BUFFER-CHUNK GOAL`.
///
/// The three spaces after the time are the separators of the always-empty
/// (single-model, no-mp) model/mp fields in `+format-event-event-string+`.
///
/// `width` is the module column width. ACT-R computes it from
/// `max-module-name-length` (`framework/modules.lisp:386`); stubs may
/// override that function to shrink the column (e.g.
/// `tutorial-unit1-count-trace` pins 13), so the spec carries the value.
pub fn format_line(time_ms: i64, module: &str, details: &str, width: usize) -> String {
    debug_assert!(time_ms >= 0);
    let sec = time_ms / 1000;
    let ms = time_ms % 1000;
    format!("{sec:>6}.{ms:03}   {module:<width$} {details}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_shape() {
        assert_eq!(
            format_line(50, "PROCEDURAL", "MOD-BUFFER-CHUNK GOAL", MODULE_WIDTH),
            "     0.050   PROCEDURAL             MOD-BUFFER-CHUNK GOAL"
        );
        assert_eq!(
            format_line(0, "GOAL", "SET-BUFFER-CHUNK GOAL (ISA X) NIL", MODULE_WIDTH),
            "     0.000   GOAL                   SET-BUFFER-CHUNK GOAL (ISA X) NIL"
        );
        assert_eq!(
            format_line(
                350,
                "------",
                "Stopped because no events left to process",
                MODULE_WIDTH
            ),
            "     0.350   ------                 Stopped because no events left to process"
        );
        // Stub-overridden width (`max-module-name-length`), e.g. 13.
        assert_eq!(
            format_line(0, "GOAL", "SET-BUFFER-CHUNK GOAL (ISA X) NIL", 13),
            "     0.000   GOAL          SET-BUFFER-CHUNK GOAL (ISA X) NIL"
        );
    }

    #[test]
    fn visibility() {
        assert!(Detail::Low.shows(Out::Low));
        assert!(!Detail::Low.shows(Out::Medium));
        assert!(Detail::Medium.shows(Out::Medium));
        assert!(!Detail::Medium.shows(Out::Always));
        assert!(Detail::High.shows(Out::Always));
    }
}

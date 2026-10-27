//! case: test-ppm
//! ref:  test-ppm.ref
//! lisp: actr7.x/test-models/test-ppm.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: `:PPM-HOOK SHOW-PPM-VALUES`.
// The first two sections run, but the ref's third section is produced by a
// task-defined Lisp callback.
#[ignore = "missing-feature: PPM hook"]
fn test_ppm() -> Result<(), String> {
    Case::new("test-ppm")
        .models(&["test-models;models;test-ppm.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":PPM".into(), crate::sexp(r#"NIL"#)),
            Step::ModelOutput("\n******************************\nWithout ppm (default symbolic matching):\n******************************\n".into()),
            Step::Command { name: "whynot".into(), args: crate::sexps(r#"()"#) },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run { secs: 10.0, trace: true, width: 22 },
            Step::Reset,
            Step::Sgp(":PPM".into(), crate::sexp(r#"1"#)),
            Step::Sgp(":UTILITY-OFFSETS".into(), crate::sexp(r#"PPM-OFFSET"#)),
            Step::ModelOutput("\n******************************\nWith ppm enabled:\n******************************\n".into()),
            Step::Command { name: "whynot".into(), args: crate::sexps(r#"()"#) },
            Step::Run { secs: 10.0, trace: true, width: 22 },
            Step::Reset,
            Step::Sgp(":PPM".into(), crate::sexp(r#"1"#)),
            Step::Sgp(":PPM-HOOK".into(), crate::sexp(r#"SHOW-PPM-VALUES"#)),
            Step::Sgp(":UTILITY-OFFSETS".into(), crate::sexp(r#"PPM-OFFSET"#)),
            Step::ModelOutput("\n******************************\nWith ppm enabled and using the hook to show values as it goes:\n******************************\n".into()),
            Step::Run { secs: 10.0, trace: true, width: 22 },
        ])
        .check()
}

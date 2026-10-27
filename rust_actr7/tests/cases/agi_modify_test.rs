//! case: agi-modify-test
//! ref:  agi-modify-test.ref
//! lisp: actr7.x/test-models/agi-modify-test.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: exact add/modify-visicon trace.
// The ref checks visual-location renaming and slot updates across three
// `print-visicon` / `pprint-chunks` passes.
#[ignore = "engine-divergence: visicon modify trace"]
fn agi_modify_test() -> Result<(), String> {
    Case::new("agi-modify-test")
        .models(&["test-models;models;agi-modify-test.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::RunNEvents { count: 4, trace: true, width: 22 },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::Command { name: "pprint-chunks".into(), args: crate::sexps(r#"(#:VISUAL-LOCATION0 VISUAL-LOCATION3 VISUAL-LOCATION4 #:VISUAL-LOCATION9 VISUAL-LOCATION1 VISUAL-LOCATION10 #:VISUAL-LOCATION5 #:VISUAL-LOCATION6 #:VISUAL-LOCATION2 #:VISUAL-LOCATION11 #:VISUAL-LOCATION7 #:VISUAL-LOCATION8)"#) },
            Step::Run { secs: 10.0, trace: true, width: 22 },
            Step::RunNEvents { count: 4, trace: true, width: 22 },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::Command { name: "pprint-chunks".into(), args: crate::sexps(r#"(VISUAL-LOCATION12 #:VISUAL-LOCATION9 VISUAL-LOCATION3 VISUAL-LOCATION4 VISUAL-LOCATION1 VISUAL-LOCATION10 VISUAL-LOCATION13 VISUAL-LOCATION14 #:VISUAL-LOCATION2 VISUAL-LOCATION15 #:VISUAL-LOCATION7 #:VISUAL-LOCATION8)"#) },
            Step::Run { secs: 10.0, trace: true, width: 22 },
            Step::RunNEvents { count: 4, trace: true, width: 22 },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::Command { name: "pprint-chunks".into(), args: crate::sexps(r#"(VISUAL-LOCATION12 VISUAL-LOCATION19 VISUAL-LOCATION3 VISUAL-LOCATION4 VISUAL-LOCATION13 VISUAL-LOCATION14 VISUAL-LOCATION1 VISUAL-LOCATION10 VISUAL-LOCATION16 VISUAL-LOCATION17 VISUAL-LOCATION18 VISUAL-LOCATION15)"#) },
            Step::Run { secs: 10.0, trace: true, width: 22 },
        ])
        .check()
}

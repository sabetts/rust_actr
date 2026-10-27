//! case: hand-move-test
//! ref:  hand-move-test.ref
//! lisp: actr7.x/test-models/hand-move-test.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: hand move/key-event trace.
// The ref records cursor movement and handler output for a manual move; the
// engine's motor event sequence still differs.
#[ignore = "engine-divergence: motor move trace"]
fn hand_move_test() -> Result<(), String> {
    Case::new("hand-move-test")
        .models(&["test-models;models;hand-move-test.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Command { name: "chunk-type".into(), args: crate::sexps(r#"((CURSOR (:INCLUDE VISUAL-OBJECT)) (CURSOR T) SCREEN-POS VALUE STATUS COLOR HEIGHT WIDTH)"#) },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run { secs: 10.0, trace: true, width: 22 },
        ])
        .check()
}

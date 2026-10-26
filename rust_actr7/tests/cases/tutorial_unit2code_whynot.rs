//! case: tutorial_unit2code_whynot
//! ref:  tutorial_unit2code_whynot.ref
//! lisp: actr7.x/test-models/tutorial_unit2code_whynot.py

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit2code_whynot() -> Result<(), String> {
    Case::new("tutorial_unit2code_whynot")
        .models(&["tutorial;unit1;addition.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":V".into(), crate::sexp("NIL")),
            Step::Run {
                secs: 0.3,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp("T")),
            Step::ModelOutput(">>> actr.whynot('initialize-addition','increment-count')".into()),
            Step::Command {
                name: "whynot".into(),
                args: crate::sexps(r#"(INITIALIZE-ADDITION INCREMENT-COUNT)"#),
            },
            Step::ModelOutput("['INCREMENT-SUM']".into()),
        ])
        .check()
}

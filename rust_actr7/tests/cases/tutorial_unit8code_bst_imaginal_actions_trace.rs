//! case: tutorial-unit8code-bst-imaginal-actions-trace
//! ref:  tutorial-unit8code-bst-imaginal-actions-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit8code-bst-imaginal-actions-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook Lisp (pending filter parsing capabilities)"]
fn tutorial_unit8code_bst_imaginal_actions_trace() -> Result<(), String> {
    Case::new("tutorial-unit8code-bst-imaginal-actions-trace")
        .models(&[
            "tutorial;lisp;bst-ppm.lisp",
            "tutorial;unit8;bst-ppm-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 60.0,
                trace: true,
                width: 22,
            },
            Step::RunNEvents {
                count: 15,
                trace: false,
                width: 22,
            },
        ])
        .check()
}

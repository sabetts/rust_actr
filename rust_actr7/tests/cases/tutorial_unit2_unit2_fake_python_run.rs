//! case: tutorial-unit2-unit2-fake-python-run
//! ref:  tutorial-unit2-unit2-fake-python-run.ref
//! lisp: actr7.x/test-models/tutorial-unit2-unit2-fake-python-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit2_unit2_fake_python_run() -> Result<(), String> {
    Case::new("tutorial-unit2-unit2-fake-python-run")
        .models(&[
            "tutorial;lisp;unit2.lisp",
            "tutorial;unit2;unit2-assignment-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput(">>> unit2.experiment()".into()),
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(22),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("False".into()),
        ])
        .check()
}

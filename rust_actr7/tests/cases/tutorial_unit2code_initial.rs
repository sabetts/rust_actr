//! case: tutorial_unit2code_initial
//! ref:  tutorial_unit2code_initial.ref
//! lisp: actr7.x/test-models/tutorial_unit2code_initial.py

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit2code_initial() -> Result<(), String> {
    Case::new("tutorial_unit2code_initial")
        .models(&["tutorial;unit1;addition.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput(">>> import actr".into()),
            Step::ModelOutput("ACT-R connection has been started.".into()),
            Step::ModelOutput(
                ">>> actr.load_act_r_model(\"ACT-R:tutorial;unit1;addition.lisp\")".into(),
            ),
            Step::ModelOutput("True".into()),
            Step::ModelOutput(">>> actr.run(1)".into()),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("[0.55, 77, None]".into()),
            Step::ModelOutput(">>> actr.reset()".into()),
            Step::Reset,
            Step::ModelOutput("True".into()),
            Step::ModelOutput(">>> actr.run(.1)".into()),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("[0.1, 20, None]".into()),
            Step::ModelOutput(">>> actr.reload()".into()),
            Step::ModelOutput("True".into()),
        ])
        .check()
}

//! case: tutorial-unit5modeling-v9-p7-8-run-g1-chunks
//! ref:  tutorial-unit5modeling-v9-p7-8-run-g1-chunks.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v9-p7-8-run-g1-chunks.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: merge/naming of the goal copy.
// The ref prints G1 and CHUNK0 with the post-merge references; the engine's
// DM merge/rename order still differs (#206).
#[ignore = "engine-divergence: DM merge naming"]
fn tutorial_unit5modeling_v9_p7_8_run_g1_chunks() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v9-p7-8-run-g1-chunks")
        .models(&["test-models;models;declarative-issues-v9.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":NCNAR".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 28.0,
                trace: false,
                width: 22,
            },
            Step::ModelOutput("? (pprint-chunks g1 chunk0)".into()),
            Step::Command {
                name: "pprint-chunks".into(),
                args: crate::sexps(r#"(G1 CHUNK0)"#),
            },
        ])
        .check()
}

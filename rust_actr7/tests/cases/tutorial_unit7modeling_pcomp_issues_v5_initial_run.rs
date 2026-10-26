//! case: tutorial-unit7modeling-pcomp-issues-v5-initial-run
//! ref:  tutorial-unit7modeling-pcomp-issues-v5-initial-run.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v5-initial-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit7modeling_pcomp_issues_v5_initial_run() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v5-initial-run")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "tutorial;unit7;production-compilation-issues-model.lisp",
            "test-models;models;production-compilation-issues-model-v5.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Seed((301, 0)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
            Step::Reset,
            Step::Seed((302, 0)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
            Step::Reset,
            Step::Seed((303, 0)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
            Step::Reset,
            Step::Seed((304, 0)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
            Step::Reset,
            Step::Seed((305, 0)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
            Step::Reset,
            Step::Seed((306, 0)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
            Step::Reset,
            Step::Seed((307, 0)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
            Step::Reset,
            Step::Seed((308, 0)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
            Step::Reset,
            Step::Seed((309, 0)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
            Step::Reset,
            Step::Seed((310, 0)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
        ])
        .check()
}

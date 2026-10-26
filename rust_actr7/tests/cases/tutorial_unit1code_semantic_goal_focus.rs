//! case: tutorial-unit1code-semantic-goal-focus
//! ref:  tutorial-unit1code-semantic-goal-focus.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-semantic-goal-focus.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1code_semantic_goal_focus() -> Result<(), String> {
    Case::new("tutorial-unit1code-semantic-goal-focus")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (run 1)".into()),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("0.15".into()),
            Step::ModelOutput("24".into()),
            Step::ModelOutput("NIL".into()),
            Step::ModelOutput(
                "? (goal-focus (ISA is-member object canary category animal))".into(),
            ),
            Step::GoalFocus(crate::chunk_desc(
                r#"(ISA IS-MEMBER OBJECT CANARY CATEGORY ANIMAL)"#,
            )),
            Step::ModelOutput("(ISA IS-MEMBER OBJECT CANARY CATEGORY ANIMAL)".into()),
            Step::ModelOutput("? (run 1)".into()),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("0.25".into()),
            Step::ModelOutput("36".into()),
            Step::ModelOutput("NIL".into()),
            Step::ModelOutput("? (goal-focus (ISA is-member object canary category fish))".into()),
            Step::GoalFocus(crate::chunk_desc(
                r#"(ISA IS-MEMBER OBJECT CANARY CATEGORY FISH)"#,
            )),
            Step::ModelOutput("(ISA IS-MEMBER OBJECT CANARY CATEGORY FISH)".into()),
            Step::ModelOutput("? (run 1)".into()),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("0.35".into()),
            Step::ModelOutput("48".into()),
            Step::ModelOutput("NIL".into()),
        ])
        .check()
}

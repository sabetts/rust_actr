//! case: tutorial-unit1-count-goal-focus
//! lisp: actr7.x/test-models/tutorial-unit1-count-goal-focus.lisp
//! source: tutorial;unit1;count.lisp

#[test]
fn tutorial_unit1_count_goal_focus() -> Result<(), String> {
    let actual = crate::echo_forms("tutorial;unit1;count.lisp", "GOAL-FOCUS", None)?;
    crate::check_echo("tutorial-unit1-count-goal-focus", actual)
}

//! case: tutorial-unit7-paired-learning-alpha-1
//! ref:  tutorial-unit7-paired-learning-alpha-1.ref
//! lisp: actr7.x/test-models/tutorial-unit7-paired-learning-alpha-1.lisp

use crate::hooks::{self, action};
use actr_engine::{EventHook, EventView, HookEffect, PreResult, Step, Verdict};

#[derive(Default)]
struct Filter {
    started: bool,
    effects: Vec<HookEffect>,
}

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        if !self.started {
            self.started = true;
            self.effects.push(HookEffect::Sgp {
                key: ":SEED".into(),
                value: crate::sexp("(101 1)"),
            });
            self.effects.push(HookEffect::Sgp {
                key: ":ALPHA".into(),
                value: crate::sexp("1"),
            });
            self.effects.push(HookEffect::Sgp {
                key: ":PCT".into(),
                value: crate::sexp("T"),
            });
        }
        if ev.time_ms == 25435 && action(ev, "production-fired") {
            return PreResult {
                verdict: Verdict::Break,
                trace: Some(false),
            };
        }
        PreResult {
            verdict: Verdict::Continue,
            trace: Some(false),
        }
    }

    fn take_effects(&mut self) -> Vec<HookEffect> {
        std::mem::take(&mut self.effects)
    }
}

#[derive(Default)]
struct Filter2 {
    output: u32,
}

impl EventHook for Filter2 {
    fn pre(&mut self, _ev: &EventView) -> PreResult {
        if self.output == 1 {
            self.output += 1;
            PreResult::trace(false)
        } else {
            self.output += 1;
            PreResult::CONTINUE
        }
    }
}

#[test]
#[ignore = "engine-divergence: conflict resolution picks PRODUCTION8 instead of PRODUCTION7 at 25.435"]
fn tutorial_unit7_paired_learning_alpha_1() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit7-paired-learning-alpha-1")
        .models(&[
            "tutorial;lisp;paired.lisp",
            "tutorial;unit4;paired-model.lisp",
            "tutorial;unit7;paired-learning-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"(("zinc" "9"))"#),
            },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"(("zinc" "9"))"#),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"(("zinc" "9"))"#),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"(("zinc" "9"))"#),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
        ])
        .swap_hook_at(21, Filter2::default())
        .check_segment(Filter::default())
}

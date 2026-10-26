//! case: dynamic-production-compilation-tests
//! ref:  dynamic-production-compilation-tests.ref
//! lisp: actr7.x/test-models/dynamic-production-compilation-tests.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn dynamic_production_compilation_tests() -> Result<(), String> {
    Case::new("dynamic-production-compilation-tests")
        .models(&["test-models;models;dynamic-production-compilation-tests.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::PEnable(vec!["G1".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["G2".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["I1".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["I2".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["P1".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["P2".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["GD".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["ID".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["PD".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["GI".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["GP".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["IG".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["IP".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["PG".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["PI".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
        ])
        .check()
}

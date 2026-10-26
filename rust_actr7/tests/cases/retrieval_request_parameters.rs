//! case: retrieval-request-parameters
//! ref:  retrieval-request-parameters.ref
//! lisp: actr7.x/test-models/retrieval-request-parameters.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn retrieval_request_parameters() -> Result<(), String> {
    Case::new("retrieval-request-parameters")
        .models(&["test-models;models;retrieval-request-parameters.lisp"])
        .seed((100, 2))
        .steps(vec![
            Step::Reset,
            Step::PEnable(vec!["NORMAL".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["CHANGE-THRESHOLD".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["NO-BLL".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["NO-MAS".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["NO-MP".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PEnable(vec!["NO-ANS".into()]),
            Step::Run {
                secs: 0.1,
                trace: true,
                width: 22,
            },
        ])
        .check()
}

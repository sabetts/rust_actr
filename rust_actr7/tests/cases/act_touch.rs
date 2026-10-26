//! case: act-touch
//! ref:  act-touch.ref
//! lisp: actr7.x/test-models/act-touch.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn act_touch() -> Result<(), String> {
    Case::new("act-touch")
        .models(&["extras;act-touch;touch-demo-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":LEFT-START".into(), crate::sexp(r#"(100 300)"#)),
            Step::Sgp(":RIGHT-START".into(), crate::sexp(r#"(600 300)"#)),
            Step::EmitWarning("Installing a default touch device for start-hand-at-touch.".into()),
            Step::ModFocus(vec![("DEVICE".into(), crate::sexp(r#"T"#))]),
            Step::StartHandAtTouch("RIGHT".into()),
            Step::AddVisicon(vec![
                ("SCREEN-X".into(), crate::sexp(r#"350"#)),
                ("SCREEN-Y".into(), crate::sexp(r#"175"#)),
                ("WIDTH".into(), crate::sexp(r#"100"#)),
                ("HEIGHT".into(), crate::sexp(r#"50"#)),
            ]),
            Step::ModifyVisicon {
                at_ms: 3000,
                name: "CHUNK0".into(),
                x: 450,
                y: 300,
            },
            Step::Run {
                secs: 20.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}

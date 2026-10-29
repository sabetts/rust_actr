//! case: creating-an-image
//! ref:  creating-an-image.ref
//! lisp: actr7.x/test-models/creating-an-image.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn creating_an_image() -> Result<(), String> {
    Case::new("creating-an-image")
        .models(&["test-models;models;creating-an-image.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command { name: "chunk-type".into(), args: crate::sexps(r#"((CURSOR (:INCLUDE VISUAL-OBJECT)) (CURSOR T) SCREEN-POS VALUE STATUS COLOR HEIGHT WIDTH)"#) },
            Step::Command { name: "chunk-type".into(), args: crate::sexps(r#"((IMAGE (:INCLUDE VISUAL-OBJECT)) (IMAGE T) SCREEN-POS VALUE STATUS COLOR HEIGHT WIDTH)"#) },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::CreatingImageSetup,
            Step::RunNEvents { count: 2, trace: true, width: 22 },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::Run { secs: 5.0, trace: true, width: 22 },
        ])
        .check()
}

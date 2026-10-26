//! case: tutorial-unit1-count-add-dm
//! lisp: actr7.x/test-models/tutorial-unit1-count-add-dm.lisp
//! source: tutorial;unit1;count.lisp

use actr_engine::sexpr::Sexp;

#[test]
fn tutorial_unit1_count_add_dm() -> Result<(), String> {
    let forms = crate::define_model_forms("tutorial;unit1;count.lisp")?;
    let add_dm = forms
        .iter()
        .find_map(|form| match form {
            Sexp::List(items) if items.first().and_then(Sexp::as_sym) == Some("ADD-DM") => {
                Some(items)
            }
            _ => None,
        })
        .ok_or("no add-dm form")?;

    let mut actual = vec!["(add-dm".to_string()];
    for chunk in &add_dm[1..] {
        let Sexp::List(items) = chunk else {
            continue;
        };
        let parts = items
            .iter()
            .map(|item| match item {
                Sexp::Sym(sym) if sym == "ISA" => "ISA".to_string(),
                other => crate::sexp_show(other),
            })
            .collect::<Vec<_>>();
        actual.push(format!(" ({})", parts.join(" ")));
    }
    if let Some(last) = actual.last_mut() {
        last.push(')');
    }
    crate::check_echo("tutorial-unit1-count-add-dm", actual)
}

use std::path::PathBuf;
use actr_engine::sexpr::parse_all;
use actr_engine::{run_model_steps, Step};

fn run(models: &[&str], steps: Vec<Step>) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../actr7.x");
    let models: Vec<String> = models.iter().map(|s| s.to_string()).collect();
    match run_model_steps(&root, &models, (100, 0), &steps, &Default::default()) {
        Ok(out) => println!("{out}"),
        Err(e) => println!("ERR: {e}"),
    }
}

fn sgp(s: &str) -> Step {
    let forms = parse_all(s).unwrap();
    if let actr_engine::sexpr::Sexp::List(v) = &forms[0] {
        Step::Sgp(v[0].as_sym().unwrap().to_string(), v[1].clone())
    } else {
        unreachable!()
    }
}

#[test]
fn dump_indirect() {
    run(
        &["test-models;models;indirect-action-compilation.lisp"],
        vec![
            Step::Reset,
            sgp("(:CIA NIL)"),
            Step::Run { secs: 0.2, trace: true, width: 22 },
            Step::Reset,
            sgp("(:CIA T)"),
            Step::Run { secs: 0.2, trace: true, width: 22 },
        ],
    );
}

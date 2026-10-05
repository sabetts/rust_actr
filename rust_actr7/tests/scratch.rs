use std::path::PathBuf;
use actr_engine::{run_model_steps, Step};

fn run(models: &[&str], steps: Vec<Step>) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../actr7.x");
    let models: Vec<String> = models.iter().map(|s| s.to_string()).collect();
    match run_model_steps(&root, &models, (100, 0), &steps, &Default::default()) {
        Ok(out) => eprintln!("----\n{out}"),
        Err(e) => eprintln!("ERR: {e}"),
    }
}

#[test]
fn dump() {
    run(
        &["test-models;models;dynamic-slot-compilation.lisp"],
        vec![Step::Run { secs: 10.0, trace: true, width: 22 }],
    );
}

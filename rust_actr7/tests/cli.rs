//! `actr` binary: filesystem paths work without an ACT-R checkout.

use std::path::PathBuf;
use std::process::Command;

fn count_model() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("models")
        .join("tutorial")
        .join("unit1")
        .join("count.lisp")
        .to_string_lossy()
        .into_owned()
}

fn actr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_actr"));
    command.env_remove("ACTR_DIR");
    command
}

#[test]
fn run_accepts_an_absolute_model_path_without_actr_dir() {
    let output = actr()
        .args(["run", "--run", "0.2", &count_model()])
        .output()
        .expect("run actr");
    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PRODUCTION-FIRED INCREMENT"), "{stdout}");
}

#[test]
fn run_rejects_actr_logical_paths() {
    let output = actr()
        .args(["run", "ACT-R:tutorial;unit1;count.lisp"])
        .output()
        .expect("run actr");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ACT-R logical paths are not supported"),
        "{stderr}"
    );
}

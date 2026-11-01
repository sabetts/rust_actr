//! Project-authored surrogates for solution-source cases that cannot be run
//! upstream because the model/task lives in a password-protected archive.
//!
//! These are **not** upstream refs. Each trace is captured from SBCL with the
//! project-authored fixture models in `fixtures/models/extras/surrogates/`,
//! using the same fixed seed and `:trace-detail high` settings. The goal is to
//! exercise the combined mechanisms the blocked cases would have exercised,
//! not to reproduce the protected models byte-for-byte.

use std::fs;
use std::path::PathBuf;

use actr_engine::json::Json;
use actr_engine::rpc::session::{Remote, Session};
use actr_engine::{DeviceSetup, DeviceText};

mod common;
use common::{crate_dir, first_diff, fixture_model, normalize_eol};

struct NoRemote;

impl Remote for NoRemote {
    fn call(&mut self, _n: &str, _m: Option<&str>, _a: Vec<Json>) -> Result<Vec<Json>, String> {
        Err("no remote client".into())
    }
}

fn traces_dir() -> PathBuf {
    crate_dir()
        .join("fixtures")
        .join("surrogates")
        .join("traces")
}

fn eval(
    session: &mut Session,
    remote: &mut dyn Remote,
    name: &str,
    args: &[Json],
) -> Result<Vec<Json>, String> {
    session.evaluate(name, None, args, remote)
}

fn load(session: &mut Session, remote: &mut dyn Remote, path: &str) -> Result<(), String> {
    eval(
        session,
        remote,
        "load-act-r-model",
        &[Json::Str(fixture_model(path))],
    )?;
    Ok(())
}

fn goal(
    session: &mut Session,
    remote: &mut dyn Remote,
    pairs: &[(&str, Json)],
) -> Result<(), String> {
    let mut args = Vec::with_capacity(pairs.len() * 2);
    for (k, v) in pairs {
        args.push(Json::Str((*k).into()));
        args.push(v.clone());
    }
    eval(session, remote, "goal-focus", &[Json::Arr(args)])?;
    Ok(())
}

fn run(session: &mut Session, remote: &mut dyn Remote, secs: f64) -> Result<(), String> {
    eval(session, remote, "run", &[Json::Float(secs)])?;
    Ok(())
}

fn sgp(
    session: &mut Session,
    remote: &mut dyn Remote,
    key: &str,
    value: Json,
) -> Result<(), String> {
    eval(session, remote, "sgp", &[Json::Str(key.into()), value])?;
    Ok(())
}

fn run_case(
    name: &str,
    f: impl FnOnce(&mut Session, &mut dyn Remote) -> Result<(), String>,
) -> Result<(), String> {
    run_case_with_device(name, DeviceSetup::default(), f)
}

fn run_case_with_device(
    name: &str,
    device: DeviceSetup,
    f: impl FnOnce(&mut Session, &mut dyn Remote) -> Result<(), String>,
) -> Result<(), String> {
    let trace_path = traces_dir().join(format!("{name}.trace"));
    let expected =
        fs::read_to_string(&trace_path).map_err(|e| format!("{}: {e}", trace_path.display()))?;
    let mut session = Session::new((100, 0));
    session.set_device(device)?;
    let mut remote = NoRemote;
    f(&mut session, &mut remote)?;
    let actual = session.take_trace_output();
    if normalize_eol(&actual) == normalize_eol(&expected) {
        Ok(())
    } else {
        let e: Vec<&str> = expected.lines().collect();
        let a: Vec<&str> = actual.lines().collect();
        Err(format!(
            "{name}: trace mismatch vs {}\n{}",
            trace_path.display(),
            first_diff(&e, &a)
        ))
    }
}

#[test]
fn categorize_surrogate() {
    if let Err(e) = run_case("categorize", |s, r| {
        load(s, r, "extras;surrogates;categorize-task.lisp")?;
        load(s, r, "extras;surrogates;categorize-model.lisp")?;
        eval(s, r, "reset", &[])?;
        sgp(s, r, ":MP", Json::Int(1))?;
        sgp(s, r, ":PPM", Json::Int(1))?;
        sgp(s, r, ":ANS", Json::Null)?;
        sgp(s, r, ":EGS", Json::Float(0.0))?;
        sgp(s, r, ":TRACE-DETAIL", Json::Str("HIGH".into()))?;
        sgp(s, r, ":ACT", Json::Str("HIGH".into()))?;
        sgp(s, r, ":PCT", Json::Bool(true))?;
        for (name, value) in [("EH", 1), ("ES", -1), ("NL", -1), ("MH", -1)] {
            goal(
                s,
                r,
                &[
                    ("ISA", Json::Str("GOAL".into())),
                    ("STATE", Json::Str("ADD-ATTRIBUTE".into())),
                    ("NAME", Json::Str(name.into())),
                    ("VALUE", Json::Int(value)),
                ],
            )?;
            run(s, r, 20.0)?;
        }
        goal(
            s,
            r,
            &[
                ("ISA", Json::Str("GOAL".into())),
                ("STATE", Json::Str("CATEGORIZE".into())),
            ],
        )?;
        run(s, r, 20.0)?;
        Ok(())
    }) {
        panic!("{e}");
    }
}

fn zbrodoff_device() -> DeviceSetup {
    let parts = [("a", 100), ("+", 125), ("2", 150), ("=", 175), ("c", 200)];
    DeviceSetup {
        texts: parts
            .iter()
            .enumerate()
            .map(|(i, (text, x))| DeviceText {
                at_ms: 0,
                text: (*text).into(),
                x: *x,
                y: 150,
                run: None,
                seq: i as u32,
            })
            .collect(),
        ..DeviceSetup::default()
    }
}

fn zbrodoff_settings(s: &mut Session, r: &mut dyn Remote) -> Result<(), String> {
    sgp(s, r, ":MP", Json::Int(1))?;
    sgp(s, r, ":PPM", Json::Int(1))?;
    sgp(s, r, ":EGS", Json::Float(0.0))?;
    sgp(s, r, ":ANS", Json::Null)?;
    sgp(s, r, ":TRACE-DETAIL", Json::Str("LOW".into()))?;
    Ok(())
}

#[test]
fn zbrodoff_retrieval_hit() {
    if let Err(e) = run_case_with_device("zbrodoff-retrieval-hit", zbrodoff_device(), |s, r| {
        load(s, r, "extras;surrogates;zbrodoff-retrieval-hit.lisp")?;
        eval(s, r, "reset", &[])?;
        zbrodoff_settings(s, r)?;
        run(s, r, 10.0)?;
        Ok(())
    }) {
        panic!("{e}");
    }
}

#[test]
fn zbrodoff_retrieval_miss() {
    if let Err(e) = run_case_with_device("zbrodoff-retrieval-miss", zbrodoff_device(), |s, r| {
        load(s, r, "extras;surrogates;zbrodoff-retrieval-miss.lisp")?;
        eval(s, r, "reset", &[])?;
        zbrodoff_settings(s, r)?;
        run(s, r, 10.0)?;
        Ok(())
    }) {
        panic!("{e}");
    }
}

#[test]
fn choice_surrogate() {
    if let Err(e) = run_case("choice-surrogate", |s, r| {
        load(s, r, "extras;surrogates;choice-task.lisp")?;
        load(s, r, "extras;surrogates;choice-model.lisp")?;
        eval(s, r, "reset", &[])?;
        sgp(s, r, ":TRACE-DETAIL", Json::Str("MEDIUM".into()))?;
        for reward in [
            -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0,
        ] {
            goal(
                s,
                r,
                &[
                    ("ISA", Json::Str("CHOOSE".into())),
                    ("STATE", Json::Str("CHOOSE".into())),
                ],
            )?;
            run(s, r, 2.0)?;
            eval(s, r, "trigger-reward", &[Json::Float(reward)])?;
        }
        Ok(())
    }) {
        panic!("{e}");
    }
}

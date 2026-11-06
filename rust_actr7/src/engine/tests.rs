//! Engine unit tests.
use super::compilation::{latency, remove_dup_keep_last, render_production};
use super::dm::bl_equation;
use super::rhs::implicit_clears;
use super::*;
use std::path::PathBuf;

fn tutorial_model(name: &str) -> Model {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../actr7.x/tutorial/unit1")
        .join(name);
    let ms = crate::model::load_models(&p).unwrap();
    assert_eq!(ms.len(), 1);
    ms.into_iter().next().unwrap()
}

fn unit5_model(name: &str) -> Model {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../actr7.x/tutorial/unit5")
        .join(name);
    let ms = crate::model::load_models(&p).unwrap();
    assert_eq!(ms.len(), 1);
    ms.into_iter().next().unwrap()
}

fn inst(ctype: &str, slots: Vec<(&str, Value)>) -> ChunkInst {
    ChunkInst {
        name: None,
        ctype: ctype.into(),
        slots: slots.into_iter().map(|(s, v)| (s.into(), v)).collect(),
        bare: false,
    }
}

fn sym(s: &str) -> Value {
    Value::Sym(s.into())
}

/// `chunk-list` comes from `dm-chunks`, not raw insertion order
/// (`declarative-memory.lisp:1125`). Signature groups are newest-first and
/// chunks within a group are newest-first; partial matching then reverses
/// the filtered list. `declarative-issues`'s first request exercises several
/// interleaved signature groups.
#[test]
fn retrieval_candidate_order_uses_dm_signature_groups() {
    let mut e = Engine::new(
        unit5_model("declarative-issues.lisp"),
        (100, 1),
        0.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.trace_on = true;
    e.cfg.act = Some(Act::T);
    e.start_retrieval(
        NewChunk::Spec {
            ctype: "SIMPLE-VALUE".to_string(),
            slots: Vec::new(),
            neg: vec![("RESULT".to_string(), Value::Nil)],
            rels: Vec::new(),
            bare: false,
        },
        &[],
    )
    .unwrap();
    let out = e.output();
    let order: Vec<String> = out
        .lines()
        .filter_map(|line| line.strip_prefix("Computing activation for chunk "))
        .map(str::to_string)
        .collect();
    assert_eq!(
        order,
        vec!["1-1=0", "2-1=1", "3-2=1", "1+1=2", "1+2=3", "V1", "V2",]
    );
}

/// The unconstrained `isa simple-value` request in `declarative-issues`
/// matches every DM chunk. SBCL walks the signature groups newest-first,
/// reverses the filtered `chunk-list` for partial matching, and therefore
/// places `V3` (which shares the empty-signature group) before the math
/// facts. Raw insertion order puts `V3` after them.
#[test]
fn retrieval_candidate_order_follows_extended_signature_groups() {
    let mut e = Engine::new(
        unit5_model("declarative-issues.lisp"),
        (100, 1),
        0.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.trace_on = true;
    e.cfg.act = Some(Act::T);
    e.start_retrieval(
        NewChunk::Spec {
            ctype: String::new(),
            slots: Vec::new(),
            neg: Vec::new(),
            rels: Vec::new(),
            bare: false,
        },
        &[],
    )
    .unwrap();
    let order: Vec<String> = e
        .output()
        .lines()
        .filter_map(|line| line.strip_prefix("Computing activation for chunk "))
        .map(str::to_string)
        .collect();
    assert_eq!(
        order,
        vec![
            "ZERO",
            "ONE",
            "TWO",
            "THREE",
            "ADD",
            "SUBTRACT",
            "SKY",
            "GRASS",
            "ROSE",
            "V3",
            "1-1=0",
            "2-1=1",
            "3-2=1",
            "1+1=2",
            "1+2=3",
            "V1",
            "V2",
            "F1",
            "F2",
            "F3",
            "G1",
            "OLD-CONTEXT",
        ]
    );
}

#[test]
fn scripted_reset_is_idempotent() {
    // Construction performs a reset; an explicit `(reset)` at t=0 must
    // reproduce the same run (same DM, goal, CR) exactly.
    let a = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    let out_a = a.run().unwrap();
    let mut b = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    b.reset().unwrap();
    b.run_secs(10.0).unwrap();
    assert_eq!(out_a, b.output());
}

#[test]
fn queue_orders_time_then_priority_then_fifo() {
    let m = tutorial_model("count.lisp");
    let mut e = Engine::new(m, (100, 0), 10.0, DeviceSetup::default()).unwrap();
    e.queue.clear();
    e.seq = 0;
    let rd = |b: &str| Event::BufferRead { buffer: b.into() };
    e.push(100, Prio::Num(0), Out::Low, rd("A"));
    e.push(50, Prio::Min, Out::Low, rd("B"));
    e.push(50, Prio::Num(10), Out::Low, rd("C"));
    e.push(50, Prio::Num(10), Out::Low, rd("D"));
    e.push(50, Prio::Max, Out::Low, rd("E"));
    e.push(50, Prio::Num(0), Out::Low, rd("F"));
    let mut order = Vec::new();
    while let Some(q) = e.queue.pop() {
        match q.event {
            Event::BufferRead { buffer } => order.push(buffer),
            _ => unreachable!(),
        }
    }
    assert_eq!(order, vec!["E", "C", "D", "F", "B", "A"]);
}

#[test]
fn harvest_rules() {
    let exempt: Vec<String> = vec!["GOAL".to_string()];
    let m = tutorial_model("count.lisp");
    let clears = |n: &str| {
        implicit_clears(
            m.productions.iter().find(|p| p.name == n).unwrap(),
            &exempt,
            false,
        )
    };
    // START tests only =goal> but requests retrieval: request rule.
    assert_eq!(clears("START"), vec!["RETRIEVAL".to_string()]);
    // INCREMENT tests both, requests retrieval: request rule (no double).
    assert_eq!(clears("INCREMENT"), vec!["RETRIEVAL".to_string()]);
    // STOP has explicit -goal>: only the untested-by-rhs retrieval.
    assert_eq!(clears("STOP"), vec!["RETRIEVAL".to_string()]);
    let s = tutorial_model("semantic.lisp");
    let dv = s
        .productions
        .iter()
        .find(|p| p.name == "DIRECT-VERIFY")
        .unwrap();
    assert_eq!(
        implicit_clears(dv, &exempt, false),
        vec!["RETRIEVAL".to_string()]
    );
    // Goal is harvest-exempt even when tested-but-untargeted.
    use crate::model::Pattern as Pat;
    use crate::model::{LhsItem, Production, RhsAction, SlotTest};
    let probe = Production {
        name: "PROBE".to_string(),
        documentation: None,
        reward: None,
        at: None,
        utility: None,
        lhs: vec![LhsItem::Match {
            buffer: "GOAL".to_string(),
            tests: vec![SlotTest {
                slot: "ISA".to_string(),
                value: Pat::Sym("GOAL".to_string()),
                op: SlotOp::Eq,
            }],
        }],
        rhs: vec![RhsAction::Request {
            buffer: "VISUAL".to_string(),
            spec: crate::model::ChunkDesc::Spec {
                ctype: String::new(),
                slots: vec![],
                auto_cmd: false,
                neg: vec![],
                rels: vec![],
            },
        }],
    };
    let no_exempt: Vec<String> = vec![];
    assert_eq!(
        implicit_clears(&probe, &no_exempt, false),
        vec!["GOAL".to_string(), "VISUAL".to_string()]
    );
    assert_eq!(
        implicit_clears(&probe, &exempt, false),
        vec!["VISUAL".to_string()]
    );
}

#[test]
fn matcher_vars_nil_negation() {
    let m = tutorial_model("count.lisp");
    let mut e = Engine::new(m, (100, 0), 10.0, DeviceSetup::default()).unwrap();
    // goal-focus state: COUNT slot absent (= nil for tests).
    e.buffers.get_mut("GOAL").unwrap().chunk = Some(inst(
        "COUNT-FROM",
        vec![("START", sym("TWO")), ("END", sym("FOUR"))],
    ));
    let start = e.model.productions[0].clone();
    let b = e
        .match_prod(&start, &[], &mut Vec::new())
        .unwrap()
        .expect("START should match");
    assert_eq!(b.get("NUM1"), Some(&sym("TWO")));
    // INCREMENT needs =retrieval>: absent, no match.
    let inc = e.model.productions[1].clone();
    assert!(e.match_prod(&inc, &[], &mut Vec::new()).unwrap().is_none());
    // Retrieval TWO (number TWO next THREE): full match, NUM2=THREE.
    e.buffers.get_mut("RETRIEVAL").unwrap().chunk = Some(inst(
        "NUMBER",
        vec![("NUMBER", sym("TWO")), ("NEXT", sym("THREE"))],
    ));
    // Goal still count-less: `count =num1` fails on nil... START's
    // `count nil` passes but increment needs a real count.
    assert!(e.match_prod(&inc, &[], &mut Vec::new()).unwrap().is_none());
    // Advance the goal as START's firing would: count=TWO.
    e.buffers.get_mut("GOAL").unwrap().chunk = Some(inst(
        "COUNT-FROM",
        vec![
            ("START", sym("TWO")),
            ("END", sym("FOUR")),
            ("COUNT", sym("TWO")),
        ],
    ));
    let b = e
        .match_prod(&inc, &[], &mut Vec::new())
        .unwrap()
        .expect("INCREMENT matches");
    assert_eq!(b.get("NUM1"), Some(&sym("TWO")));
    assert_eq!(b.get("NUM2"), Some(&sym("THREE")));
    // STOP needs count=end: not yet.
    let stop = e.model.productions[2].clone();
    assert!(e.match_prod(&stop, &[], &mut Vec::new()).unwrap().is_none());
}

#[test]
fn matcher_failure_query() {
    let m = tutorial_model("semantic.lisp");
    let mut e = Engine::new(m, (100, 0), 10.0, DeviceSetup::default()).unwrap();
    e.buffers.get_mut("GOAL").unwrap().chunk = Some(inst(
        "IS-MEMBER",
        vec![
            ("OBJECT", sym("CANARY")),
            ("CATEGORY", sym("BIRD")),
            ("JUDGMENT", sym("PENDING")),
        ],
    ));
    let fail = e.model.productions[3].clone();
    // Retrieval merely empty is not failure.
    assert!(e.match_prod(&fail, &[], &mut Vec::new()).unwrap().is_none());
    e.buffers.get_mut("RETRIEVAL").unwrap().failed = true;
    assert!(e.match_prod(&fail, &[], &mut Vec::new()).unwrap().is_some());
}

/// SBCL `~S` single-float rendering (the blending trace's arbiter):
/// shortest digits, fixed for exponents -3..=6, exponent form outside.
#[test]
fn lisp_s_f32_matches_sbcl() {
    assert_eq!(lisp_s_f32(0.35355338), "0.35355338");
    assert_eq!(lisp_s_f32(2.5325232), "2.5325232");
    assert_eq!(lisp_s_f32(3.7122225e-6), "3.7122225e-6");
    assert_eq!(lisp_s_f32(1.1136667e-5), "1.1136667e-5");
    assert_eq!(lisp_s_f32(0.001), "0.001");
    assert_eq!(lisp_s_f32(1.0e7), "1.0e7");
    assert_eq!(lisp_s_f32(9999999.0), "9999999.0");
    assert_eq!(lisp_s_f32(-0.9), "-0.9");
    assert_eq!(lisp_s_f32(0.0), "0.0");
    assert_eq!(lisp_s_f32(2.0), "2.0");
}

/// CL `remove-duplicates` keeps the later copy (production
/// compilation's condition union depends on it).
#[test]
fn remove_dup_keeps_last() {
    assert_eq!(remove_dup_keep_last(vec![1, 2, 1, 3]), vec![2, 1, 3]);
    let a = ("SLOT".to_string(), SlotVal::Lit("SMALL".to_string()));
    let b = ("STATE".to_string(), SlotVal::Lit("P2".to_string()));
    assert_eq!(
        remove_dup_keep_last(vec![a.clone(), b.clone(), a]),
        vec![b, ("SLOT".to_string(), SlotVal::Lit("SMALL".to_string()))]
    );
}

/// The bounded `!eval!`/`!bind!` evaluator: pure operators, variables,
/// quotes, `mp-time`, and loud failure for unknown heads.
#[test]
fn eval_evaluator() {
    use crate::sexpr::parse_all;
    let e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    let mut b = Bindings::new();
    b.insert("X".to_string(), Value::Int(3));
    let one = |s: &str| parse_all(s).unwrap().pop().unwrap();
    let t = Value::Sym("T".to_string());
    assert_eq!(
        e.eval_expr(&one("(> (mp-time) .4)"), &b).unwrap(),
        Value::Nil
    );
    assert_eq!(e.eval_expr(&one("(> (mp-time) -1)"), &b).unwrap(), t);
    assert_eq!(e.eval_expr(&one("(not (equal =x 'blue))"), &b).unwrap(), t);
    assert_eq!(e.eval_expr(&one("(+ =x 1)"), &b).unwrap(), Value::Int(4));
    assert_eq!(e.eval_expr(&one("(null =x)"), &b).unwrap(), Value::Nil);
    assert_eq!(e.eval_expr(&one("(and (> =x 2) (< =x 4))"), &b).unwrap(), t);
    assert!(e.eval_expr(&one("(this-needs-a-real-lisp)"), &b).is_err());
}

/// Task-file `defun`s are callable by symbol or string head, with bare
/// parameters bound from the call scope.
#[test]
fn sgp_rejects_unknown_keys() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    let t = crate::sexpr::Sexp::Sym("T".into());
    assert!(e.apply_sgp(":NOT-A-REAL-PARAM", &t).is_err());
    // Documented no-ops stay accepted.
    assert!(e.apply_sgp(":UAN", &t).is_ok());
    assert!(e.apply_sgp(":SHOW-FOCUS", &t).is_ok());
    // Model-load-only keys are accepted but have no runtime effect.
    assert!(e.apply_sgp(":TRACE-TRACKER", &t).is_ok());
}

#[test]
fn runtime_production_redefinition_warns() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.apply_sgp(":MODEL-WARNINGS", &crate::sexpr::Sexp::Sym("T".into()))
        .unwrap();
    e.define_production_text("(p test =goal> ==> )").unwrap();
    e.define_production_text("(p test =goal> ==> )").unwrap();
    let out = e.output();
    assert!(
        out.contains("#|Warning: Production TEST already exists and it is being redefined. |#"),
        "missing redefinition warning in output:\n{out}"
    );
}

#[test]
fn eval_user_defun() {
    use crate::model::UserDefun;
    use crate::sexpr::parse_all;
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    let expr = |s: &str| parse_all(s).unwrap().pop().unwrap();
    let mut fns = HashMap::new();
    fns.insert(
        "SINGLE-BIND".to_string(),
        UserDefun {
            params: vec!["VAL".to_string()],
            body: vec![expr("(1+ val)")],
        },
    );
    e.set_user_fns(fns);
    let b = Bindings::new();
    assert_eq!(
        e.eval_expr(&expr("(single-bind 3)"), &b).unwrap(),
        Value::Int(4)
    );
    assert_eq!(
        e.eval_expr(&expr("(\"single-bind\" 4)"), &b).unwrap(),
        Value::Int(5)
    );
}

/// `render_production` reproduces the `:cst` instantiation block
/// (`tutorial-unit8-simple-ppm-model-initial-run.ref`).
#[test]
fn render_production_instantiated() {
    use crate::model::{LhsItem, Pattern as Pat, Production, RhsAction, SlotTest};
    let prod = Production {
        name: "SMALL".to_string(),
        documentation: None,
        reward: None,
        at: None,
        utility: None,
        lhs: vec![LhsItem::Match {
            buffer: "GOAL".to_string(),
            tests: vec![
                SlotTest {
                    slot: "ISA".to_string(),
                    value: Pat::Sym("TEST".to_string()),
                    op: SlotOp::Eq,
                },
                SlotTest {
                    slot: "VALUE".to_string(),
                    value: Pat::Sym("SMALL".to_string()),
                    op: SlotOp::Eq,
                },
                SlotTest {
                    slot: "VALUE".to_string(),
                    value: Pat::Var("VAL".to_string()),
                    op: SlotOp::Eq,
                },
            ],
        }],
        rhs: vec![
            RhsAction::Output {
                args: vec![Sexp::Sym("=VAL".to_string())],
            },
            RhsAction::Clear {
                buffer: "GOAL".to_string(),
            },
        ],
    };
    let mut b = Bindings::new();
    b.insert("VAL".to_string(), Value::Sym("SMALL".to_string()));
    assert_eq!(
            render_production(&prod, &b, &[], &[], &[]),
            "(P SMALL\n   =GOAL>\n       VALUE SMALL\n       VALUE SMALL\n ==>\n   !OUTPUT! SMALL\n   -GOAL>\n)"
        );
}

/// Row-by-row against `fixtures/units/tables/latency.sexp`: 802
/// `compute-activation-latency` points plus CL half-even ROUND probes.
#[test]
fn latency_matches_lisp_table() {
    use crate::sexpr::{parse_all, Sexp};
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/units/tables/latency.sexp");
    let src = std::fs::read_to_string(&path).unwrap();
    let forms = parse_all(&src).unwrap();
    let (mut lat, mut rnd) = (0, 0);
    for f in &forms {
        let items = match f {
            Sexp::List(it) => it,
            _ => panic!("table row not a list: {f:?}"),
        };
        match items.as_slice() {
            [Sexp::Sym(h), Sexp::Float(a), Sexp::Int(lf), Sexp::Float(le), Sexp::Int(want)]
                if h == "LAT" =>
            {
                assert_eq!(latency(*lf, *le as f32, *a as f32), *want, "row {f:?}");
                lat += 1;
            }
            [Sexp::Sym(h), Sexp::Float(x), Sexp::Int(want)] if h == "ROUND" => {
                assert_eq!(lisp_round(*x as f32), *want, "row {f:?}");
                rnd += 1;
            }
            row => panic!("bad table row: {row:?}"),
        }
    }
    assert!(lat >= 800 && rnd >= 7, "table shrank: lat={lat} rnd={rnd}");
}

/// Row-by-row against `fixtures/units/tables/base-level.sexp`:
/// act-scale derivation plus the decay equation over three OL
/// modes. Compared with epsilon (f32 `powf`/`ln` may differ 1ulp
/// across libms); the table pins the equation, not the last bit.
#[test]
fn base_level_matches_lisp_table() {
    use crate::sexpr::{parse_all, Sexp};
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/units/tables/base-level.sexp");
    let src = std::fs::read_to_string(&path).unwrap();
    let forms = parse_all(&src).unwrap();
    let (mut acts, mut rows, mut now_ms) = (0, 0, 0i64);
    for f in &forms {
        let items = match f {
            Sexp::List(it) => it,
            _ => panic!("table row not a list: {f:?}"),
        };
        match items.as_slice() {
            [Sexp::Sym(h), Sexp::Int(t)] if h == "NOW" => now_ms = *t,
            [Sexp::Sym(h), Sexp::Float(bll), Sexp::Float(want)] if h == "ACTSCALE" => {
                // (log (expt 1/1000 (- bll))): ratio coerced to single.
                let got = (0.001f32).powf(-(*bll as f32)).ln();
                assert!(
                    (got - *want as f32).abs() <= 1e-6,
                    "ACTSCALE {bll}: got {got}, want {want}"
                );
                acts += 1;
            }
            [Sexp::Sym(h), Sexp::Float(bll), ol, Sexp::Float(scale), Sexp::Int(n), Sexp::Int(ct), rest @ ..]
                if h == "BL" =>
            {
                let ol = match ol {
                    Sexp::Sym(s) if s == "NIL" => Ol::Off,
                    Sexp::Int(i) => Ol::Num(*i as f64),
                    Sexp::Sym(s) if s == "T" => Ol::On,
                    got => panic!("bad OL flag: {got:?}"),
                };
                assert!(rest.len() >= 1, "BL row missing RESULT: {f:?}");
                let (refs_s, res_s) = rest.split_at(rest.len() - 1);
                let refs: Vec<i64> = refs_s
                    .iter()
                    .map(|r| match r {
                        Sexp::Int(t) => *t,
                        got => panic!("bad ref time: {got:?}"),
                    })
                    .collect();
                let want = match &res_s[0] {
                    Sexp::Float(x) => *x as f32,
                    got => panic!("bad RESULT: {got:?}"),
                };
                assert_eq!(refs.len(), *n as usize, "row {f:?}");
                let got = bl_equation(
                    now_ms,
                    &refs,
                    *n as f32,
                    *ct,
                    *bll as f32,
                    &ol,
                    *scale as f32,
                );
                let tol = 1e-5 * want.abs().max(1.0);
                assert!(
                    (got - want).abs() <= tol,
                    "BL row {f:?}: got {got}, want {want}"
                );
                rows += 1;
            }
            row => panic!("bad table row: {row:?}"),
        }
    }
    assert!(
        acts >= 2 && rows >= 24,
        "table shrank: acts={acts} rows={rows}"
    );
}

#[test]
fn hook_effects_apply_sgp_and_output() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.out_lines.clear();
    e.apply_hook_effect(HookEffect::Sgp {
        key: ":V".into(),
        value: Sexp::Sym("NIL".into()),
    })
    .unwrap();
    assert!(!e.trace_on);
    e.apply_hook_effect(HookEffect::ModelOutput("hook output".into()))
        .unwrap();
    assert_eq!(e.out_lines, vec!["hook output".to_string()]);
}

#[test]
fn hook_effect_captures_buffer_without_trace() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    let mut chunk = inst(
        "COUNT-FROM",
        vec![("START", sym("TWO")), ("END", sym("FOUR"))],
    );
    chunk.name = Some("COUNT-FROM-0".into());
    e.buffers.get_mut("GOAL").unwrap().chunk = Some(chunk);
    let mark = e.out_lines.len();
    let sink = Rc::new(RefCell::new(String::new()));
    e.apply_hook_effect(HookEffect::CaptureBufferChunk {
        buffers: vec!["GOAL".into()],
        sink: sink.clone(),
    })
    .unwrap();
    assert!(sink.borrow().contains("GOAL: COUNT-FROM-0"));
    assert_eq!(e.out_lines.len(), mark);
}

#[test]
fn hook_break_still_applies_pre_effects() {
    struct BreakHook;
    impl EventHook for BreakHook {
        fn pre(&mut self, _ev: &EventView) -> PreResult {
            PreResult::BREAK
        }
        fn take_effects(&mut self) -> Vec<HookEffect> {
            vec![HookEffect::Sgp {
                key: ":V".into(),
                value: Sexp::Sym("NIL".into()),
            }]
        }
    }
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.schedule_dummy(1);
    e.set_event_hook(Some(Box::new(BreakHook)));
    e.run_n_events(1).unwrap();
    assert!(!e.trace_on);
    assert!(
        e.queue.peek().is_some(),
        "break must leave the event queued"
    );
}

#[test]
fn run_n_events_reports_the_processed_count() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.reset().unwrap();
    e.run_n_events(2).unwrap();
    assert_eq!(e.run_events(), 2);
}

#[test]
fn run_full_time_processes_events_at_the_limit() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.reset().unwrap();
    e.run_full_time(0.05).unwrap();
    assert!(
        e.out_lines
            .lines
            .iter()
            .any(|line| line.contains("PRODUCTION-FIRED START")),
        "event at the full-time limit was left queued: {:?}",
        e.out_lines
    );
}

#[test]
fn reset_silent_suppresses_setup_trace() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    let before = e.out_lines.len();
    e.reset_silent().unwrap();
    assert_eq!(e.out_lines.len(), before);
    assert!(!e.trace_on);
}

#[test]
fn hook_command_records_history_and_modifies_buffer() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.apply_hook_command("record-history", &[Sexp::Str("retrieval-history".into())])
        .unwrap();
    assert!(e.record_retrieval_history);
    assert!(e.cfg.sact.is_some());

    let mut chunk = inst("COUNT-FROM", vec![("START", sym("TWO"))]);
    chunk.name = Some("COUNT-FROM-0".into());
    e.buffers.get_mut("GOAL").unwrap().chunk = Some(chunk);
    e.apply_hook_command(
        "mod-buffer-chunk",
        &[
            Sexp::Sym("GOAL".into()),
            Sexp::List(vec![
                Sexp::Sym("STATE".into()),
                Sexp::Sym("CATEGORIZE".into()),
            ]),
        ],
    )
    .unwrap();
    assert_eq!(
        e.buffers["GOAL"].chunk.as_ref().unwrap().get("STATE"),
        Value::Sym("CATEGORIZE".into())
    );
}

#[test]
fn event_view_reports_non_empty_buffers() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.buffers.get_mut("GOAL").unwrap().chunk = Some(inst("COUNT-FROM", vec![]));
    let view = e.event_view(&Event::ConflictResolution, Out::Low, true);
    assert!(view.buffer_present("GOAL"));
    assert!(!view.buffer_present("MISSING"));
}

fn probe_sound(onset: i64, offset: i64, duration: i64) -> Sound {
    Sound {
        sname: "WORD0".into(),
        ename: "AUDIO-EVENT0".into(),
        kind: "WORD".into(),
        content: Value::Str("probe".into()),
        location: "SELF".into(),
        onset,
        offset: Some(offset),
        duration: Some(duration),
        delay: 300,
        recode: 150,
        pitch: "MIDDLE".into(),
        attended: SoundAttended::Nil,
        other_features: Vec::new(),
    }
}

#[test]
fn aural_detection_installs_location_and_skips_finished_end() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.reset().unwrap();
    e.cfg.detail = Detail::High;
    e.trace_on = true;
    e.sounds.push(probe_sound(250, 350, 100));
    e.sync_audio_chunks(0);
    e.push(250, Prio::Num(0), Out::Medium, Event::NewSound);
    e.push(
        550,
        Prio::Num(10),
        Out::Medium,
        Event::AuralLocation { evt: 0 },
    );
    e.run_secs(1.0).unwrap();

    assert!(
        e.aural_location_holds(0),
        "detection did not set the buffer"
    );
    assert_eq!(
        e.buffers["AURAL-LOCATION"]
            .chunk
            .as_ref()
            .and_then(|c| c.name.as_deref()),
        Some("AUDIO-EVENT0-0")
    );
    assert!(
        !e.out_lines
            .lines
            .iter()
            .any(|l| l.contains("AUDIO-EVENT-ENDED")),
        "a sound already finished at detection must not schedule the end check: {:?}",
        e.out_lines
    );
}

#[test]
fn aural_detection_schedules_end_for_unfinished_sound() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.reset().unwrap();
    e.cfg.detail = Detail::High;
    e.trace_on = true;
    e.sounds.push(probe_sound(250, 750, 500));
    e.sync_audio_chunks(0);
    e.push(250, Prio::Num(0), Out::Medium, Event::NewSound);
    e.push(
        550,
        Prio::Num(10),
        Out::Medium,
        Event::AuralLocation { evt: 0 },
    );
    e.run_secs(1.0).unwrap();

    assert!(
        e.aural_location_holds(0),
        "detection did not set the buffer"
    );
    assert!(
        e.out_lines
            .lines
            .iter()
            .any(|l| l.contains("AUDIO-EVENT-ENDED AUDIO-EVENT0")),
        "unfinished sound should report its end: {:?}",
        e.out_lines
    );
}

#[test]
fn indirect_request_sees_same_fire_imaginal_mod() {
    let mut e = Engine::new(
        tutorial_model("count.lisp"),
        (100, 0),
        10.0,
        DeviceSetup::default(),
    )
    .unwrap();
    e.reset().unwrap();
    let mut chunk = inst("RESULT", vec![("SLOT1", sym("RED"))]);
    chunk.name = Some("IMAGINAL-CHUNK0".into());
    e.buffer_mut("IMAGINAL").chunk = Some(chunk);
    e.push(
        0,
        Prio::Num(100),
        Out::Medium,
        Event::ModBuffer {
            buffer: "IMAGINAL".into(),
            slots: vec![("SLOT2".into(), sym("TWO"))],
            module: "PROCEDURAL".into(),
        },
    );

    e.apply_pending_imaginal_mod_peek();

    assert_eq!(
        e.buffers["IMAGINAL"].chunk.as_ref().unwrap().get("SLOT2"),
        Value::Sym("TWO".into())
    );
    assert!(
        e.queue.iter().any(|q| matches!(
            &q.event,
            Event::ModBuffer { buffer, .. } if buffer == "IMAGINAL"
        )),
        "visible ModBuffer event must remain queued"
    );
}

#[test]
fn extended_motor_dual_stage_params_survive_reset() {
    let src = r#"
(define-model probe
  (sgp :dual-processor-stages t :dual-execution-stages t))
"#;
    let model = crate::model::load_models_from_str(src)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let mut e = Engine::new(model, (100, 0), 10.0, DeviceSetup::default()).unwrap();
    assert_eq!(e.get_param(":DUAL-PROCESSOR-STAGES").as_deref(), Some("T"));
    assert_eq!(e.get_param(":DUAL-EXECUTION-STAGES").as_deref(), Some("T"));
    e.reset().unwrap();
    assert_eq!(e.get_param(":DUAL-PROCESSOR-STAGES").as_deref(), Some("T"));
    assert_eq!(e.get_param(":DUAL-EXECUTION-STAGES").as_deref(), Some("T"));
}

#[test]
fn visual_location_request_matches_device_feature_slots() {
    let f = Feature {
        name: "VISUAL-LOCATION0".into(),
        kind: "TEXT".into(),
        value: "beta".into(),
        x: 75,
        y: 36,
        width: 28,
        height: 10,
        extra: vec![("TEXT".into(), sym("T"))],
        spec: vec![
            ("COLOR".into(), sym("BLACK")),
            ("VALUE".into(), Value::Str("beta".into())),
            ("SCREEN-X".into(), Value::Int(75)),
            ("SCREEN-Y".into(), Value::Int(36)),
            ("WIDTH".into(), Value::Int(28)),
            ("HEIGHT".into(), Value::Int(10)),
            ("KIND".into(), sym("TEXT")),
            ("TEXT".into(), sym("T")),
        ],
        spec_pre: Vec::new(),
        tstamp_ms: 0,
        new_p: true,
        finst_ms: None,
    };
    let matches = |want: Vec<(&str, Value)>| {
        super::perception::loc_matches(
            &f,
            &want
                .into_iter()
                .map(|(s, v)| (s.to_string(), v))
                .collect::<Vec<_>>(),
            0,
            500,
        )
        .unwrap()
    };

    assert!(matches(vec![("COLOR", sym("black"))]));
    assert!(matches(vec![("VALUE", Value::Str("BETA".into()))]));
    assert!(matches(vec![("WIDTH", Value::Int(28))]));
    assert!(matches(vec![("TEXT", sym("T"))]));
    assert!(!matches(vec![("COLOR", sym("RED"))]));
    assert!(!matches(vec![("WIDTH", Value::Int(27))]));
}

#[test]
fn same_time_window_ops_after_display_queue_a_new_wave() {
    let src = r#"
(define-model probe)
"#;
    let model = crate::model::load_models_from_str(src)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let mut e = Engine::new(model, (100, 0), 10.0, DeviceSetup::default()).unwrap();
    e.add_window_text("alpha", 10, 10, 75, 20, 12);
    e.proc_display().unwrap();

    e.clear_window();
    e.add_window_text("beta", 20, 20, 75, 20, 12);

    assert_eq!(e.display_queued_ms, Some(e.time_ms));
    assert!(
        e.queue
            .iter()
            .any(|q| matches!(q.event, Event::ProcDisplay)),
        "same-time clear/add after a display must queue a proc-display"
    );
}

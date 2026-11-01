//! Speech articulation times, the syllable-count extension, and word sounds.

use std::path::PathBuf;

use actr_engine::json::Json;
use actr_engine::rpc::session::{Remote, Session};

struct NoRemote;

impl Remote for NoRemote {
    fn call(
        &mut self,
        _name: &str,
        _model: Option<&str>,
        _args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        Ok(vec![Json::Null])
    }
}

fn session() -> Session {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("actr7.x");
    let mut session = Session::new(root, (100, 0));
    session
        .load_source(
            "(require-extra \"syllable-count\")\n\
             (define-model speech-test\n\
             (chunk-type goal state)\n\
             (add-dm (start isa goal state start))\n\
             (goal-focus start))",
        )
        .expect("load model");
    session
}

fn evaluate(session: &mut Session, name: &str, args: &[Json]) -> Vec<Json> {
    session
        .evaluate(name, None, args, &mut NoRemote)
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn text(value: &str) -> Json {
    Json::Str(value.to_string())
}

#[test]
fn articulation_time_uses_the_speech_parameters() {
    let mut session = session();

    assert_eq!(
        session
            .engine_mut()
            .unwrap()
            .articulation_time_ms("through"),
        150
    );
    assert_eq!(
        session
            .engine_mut()
            .unwrap()
            .articulation_time_ms("xxxxxxx"),
        350
    );

    evaluate(
        &mut session,
        "set-parameter-value",
        &[text(":syllable-rate"), Json::Float(0.1)],
    );
    assert_eq!(
        session.engine_mut().unwrap().articulation_time_ms("zzz"),
        100
    );
    evaluate(
        &mut session,
        "set-parameter-value",
        &[text(":syllable-rate"), Json::Float(0.15)],
    );

    evaluate(
        &mut session,
        "register-articulation-time",
        &[text("special"), Json::Float(0.333)],
    );
    assert_eq!(
        session
            .engine_mut()
            .unwrap()
            .articulation_time_ms("special"),
        333
    );
    assert_eq!(
        evaluate(
            &mut session,
            "get-articulation-time",
            &[text("special"), Json::Bool(true)],
        ),
        vec![Json::Int(333)]
    );
    assert_eq!(
        evaluate(
            &mut session,
            "get-articulation-time",
            &[text("special"), Json::Bool(false)],
        ),
        vec![Json::Float(0.333)]
    );
}

#[test]
fn syllable_table_and_split_match_the_extension() {
    let mut session = session();

    assert_eq!(
        session.engine_mut().unwrap().get_syllable_count("through"),
        Some(1)
    );
    assert_eq!(
        session.engine_mut().unwrap().get_syllable_count("zzzzzzz"),
        None
    );
    assert_eq!(
        session
            .engine_mut()
            .unwrap()
            .articulation_time_ms("through zzzzzzz"),
        750
    );

    evaluate(
        &mut session,
        "set-parameter-value",
        &[text(":split-vocal-strings"), Json::Bool(true)],
    );
    assert_eq!(
        session
            .engine_mut()
            .unwrap()
            .articulation_time_ms("through zzzzzzz"),
        650
    );

    assert_eq!(
        evaluate(
            &mut session,
            "register-syllable-count",
            &[text("zzzzzzz"), Json::Int(1)],
        ),
        vec![text("zzzzzzz")]
    );
    assert_eq!(
        session
            .engine_mut()
            .unwrap()
            .articulation_time_ms("through zzzzzzz"),
        450
    );
    assert_eq!(
        evaluate(&mut session, "get-syllable-count", &[text("zzzzzzz")]),
        vec![Json::Int(1)]
    );
    assert_eq!(
        evaluate(&mut session, "remove-syllable-count", &[text("zzzzzzz")]),
        vec![text("zzzzzzz")]
    );
    assert_eq!(
        session
            .engine_mut()
            .unwrap()
            .articulation_time_ms("through zzzzzzz"),
        650
    );
}

fn time(session: &mut Session, text: &str) -> i64 {
    session.engine_mut().unwrap().articulation_time_ms(text)
}

#[test]
fn syllable_count_matrix_matches_the_reference() {
    let mut session = session();
    evaluate(
        &mut session,
        "register-articulation-time",
        &[text("special"), Json::Float(0.333)],
    );
    let words = [
        "a",
        "special",
        "through",
        "qqqqqqq",
        "through qqqqqqq",
        "special qqqqqqq",
    ];
    let expected = [
        [150, 150, 150, 150],
        [333, 333, 333, 333],
        [350, 150, 350, 150],
        [350, 350, 350, 350],
        [750, 750, 850, 650],
        [750, 750, 833, 833],
    ];
    for (word, row) in words.iter().zip(expected) {
        let mut index = 0;
        for split in [false, true] {
            for table in [false, true] {
                evaluate(
                    &mut session,
                    "set-parameter-value",
                    &[text(":split-vocal-strings"), Json::Bool(split)],
                );
                evaluate(
                    &mut session,
                    "set-parameter-value",
                    &[text(":use-syllable-table"), Json::Bool(table)],
                );
                assert_eq!(
                    time(&mut session, word),
                    row[index],
                    "{word:?} split={split} table={table}"
                );
                index += 1;
            }
        }
    }
}

#[test]
fn word_sound_uses_articulation_time() {
    let mut session = session();
    evaluate(
        &mut session,
        "new-word-sound",
        &[
            text("hello"),
            Json::Int(0),
            text("external"),
            Json::Bool(true),
        ],
    );
    session.engine_mut().expect("engine").run_secs(1.0).unwrap();

    let engine = session.engine_mut().expect("engine");
    let event = engine.buffer_read("aural-location").expect("event");
    assert!(event.starts_with("AUDIO-EVENT0"), "{event}");
    assert_eq!(
        engine.chunk_slot("AUDIO-EVENT0", "kind").as_deref(),
        Some("WORD")
    );
    assert_eq!(
        engine.chunk_slot("WORD0", "content").as_deref(),
        Some("\"hello\"")
    );
}

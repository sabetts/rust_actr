//! Loader: `define-model` files → [`Model`] AST.
//!
//! Handles the forms appearing in tutorial models: `clear-all`,
//! `define-model`, `sgp`, `chunk-type`, `add-dm`, `goal-focus`, and `p`
//! productions with `=buffer>` match / `?buffer?`... precisely:
//! `=x>` match patterns, `?x>` state queries, `+x>` requests,
//! `-x>` clears, `==>` separator, `- slot value` negation, `!output!`.
//! Anything outside that errors structurally — strict now, while the
//! executor's needs are still unknown, beats silent mis-parses.
//!
//! Symbol case: the reader folds to UPPERCASE, so `isa`/`ISA`,
//! `p`/`P` compare equal here.

use std::fmt;
use std::fs;
use std::path::Path;

use crate::sexpr::{parse_all, Sexp};

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// `=foo` variable reference (stored without the `=`).
    Var(String),
    Sym(String),
    /// Integer literal. Kept distinct from [`Pattern::Num`] because Lisp
    /// prints `3` for an integer and `3.0` for a float (`~S`), and traces
    /// compare byte-for-byte (GOTCHAS #13).
    Int(i64),
    Num(f64),
    Str(String),
    Nil,
}

impl Pattern {
    fn from_sexp(s: &Sexp) -> Result<Pattern, LoadError> {
        match s {
            Sexp::Sym(v) if v == "NIL" => Ok(Pattern::Nil),
            Sexp::Sym(v) if v.starts_with('=') => Ok(Pattern::Var(v[1..].to_string())),
            Sexp::Sym(v) => Ok(Pattern::Sym(v.clone())),
            Sexp::Int(i) => Ok(Pattern::Int(*i)),
            Sexp::Float(x) => Ok(Pattern::Num(*x)),
            Sexp::Str(v) => Ok(Pattern::Str(v.clone())),
            Sexp::List(_) => Err(LoadError::Structural(format!(
                "list where atomic pattern expected: {s:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlotTest {
    pub slot: String,
    pub value: Pattern,
    pub negated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LhsItem {
    /// `=goal> ISA count-from start =num1 ...`
    Match {
        buffer: String,
        tests: Vec<SlotTest>,
    },
    /// `?retrieval> buffer failure`
    Query {
        buffer: String,
        queries: Vec<(String, Pattern)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChunkDesc {
    /// `ISA <type> <slot value>...`, or bare pairs (empty type).
    Spec {
        ctype: String,
        slots: Vec<(String, Pattern)>,
    },
    /// A bare chunk name.
    Named(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RhsAction {
    /// `=goal> <slot value>...`
    Modify {
        buffer: String,
        slots: Vec<(String, Pattern)>,
    },
    /// `+retrieval> ISA number number =num2`
    Request { buffer: String, spec: ChunkDesc },
    /// `-goal>`
    Clear { buffer: String },
    /// `!output! (...)` — args kept raw for the M2c evaluator.
    Output { args: Vec<Sexp> },
    /// Any other `!head! ...` — preserved, executor decides later.
    Other { head: String, args: Vec<Sexp> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Production {
    pub name: String,
    pub lhs: Vec<LhsItem>,
    pub rhs: Vec<RhsAction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkType {
    pub name: String,
    pub slots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub name: String,
    pub ctype: Option<String>,
    /// Bare `(shark)` entries have no type and no slots.
    pub slots: Vec<(String, Pattern)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub name: String,
    /// `(sgp k v ...)` pairs, values raw.
    pub params: Vec<(String, Sexp)>,
    pub chunk_types: Vec<ChunkType>,
    /// Declarative memory from `add-dm`.
    pub chunks: Vec<Chunk>,
    pub initial_goal: Option<ChunkDesc>,
    pub productions: Vec<Production>,
}

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Parse(crate::sexpr::ParseError),
    Structural(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "io: {e}"),
            LoadError::Parse(e) => write!(f, "{e}"),
            LoadError::Structural(m) => write!(f, "structural: {m}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        LoadError::Io(e)
    }
}

impl From<crate::sexpr::ParseError> for LoadError {
    fn from(e: crate::sexpr::ParseError) -> Self {
        LoadError::Parse(e)
    }
}

fn structural<T>(msg: impl Into<String>) -> Result<T, LoadError> {
    Err(LoadError::Structural(msg.into()))
}

/// `=goal>` / `+retrieval>` / `?x>` / `-x>`, but not `==>` or `=var`.
fn buffer_ref(s: &str) -> Option<(char, &str)> {
    let mut c = s.chars();
    let op = c.next()?;
    if !"=+-?".contains(op) || !s.ends_with('>') || s == "==>" {
        return None;
    }
    let inner = &s[op.len_utf8()..s.len() - 1];
    if inner.is_empty() {
        return None;
    }
    Some((op, inner))
}

fn is_action_head(s: &Sexp) -> bool {
    match s {
        Sexp::Sym(t) => buffer_ref(t).is_some() || (t.starts_with('!') && t.ends_with('!')),
        _ => false,
    }
}

/// Parse `slot value` pairs with optional `-` negation prefixes.
fn parse_pairs(forms: &[Sexp]) -> Result<Vec<SlotTest>, LoadError> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < forms.len() {
        let negated = matches!(&forms[i], Sexp::Sym(t) if t == "-");
        if negated {
            i += 1;
        }
        let slot = match forms.get(i) {
            Some(Sexp::Sym(t)) => t.clone(),
            got => return structural(format!("slot name expected, got {got:?}")),
        };
        // Inequality modifiers (>, <, >=, <=) are valid ACT-R but
        // unimplemented: refuse loudly instead of treating the
        // modifier as a slot name and silently never matching.
        if [">", "<", ">=", "<="].contains(&slot.as_str()) {
            return structural(format!("inequality tests not implemented: {slot}"));
        }
        let value = match forms.get(i + 1) {
            Some(v) => Pattern::from_sexp(v)?,
            None => return structural(format!("value missing for slot {slot}")),
        };
        out.push(SlotTest {
            slot,
            value,
            negated,
        });
        i += 2;
    }
    Ok(out)
}

fn parse_pairs_plain(forms: &[Sexp]) -> Result<Vec<(String, Pattern)>, LoadError> {
    let mut out = Vec::new();
    for t in parse_pairs(forms)? {
        if t.negated {
            return structural(format!(
                "`-` negation of slot {} only valid on production LHS",
                t.slot
            ));
        }
        out.push((t.slot, t.value));
    }
    Ok(out)
}

/// `ISA <type> pairs...`, bare pairs (empty type), or a lone chunk name.
pub fn parse_chunk_desc(forms: &[Sexp]) -> Result<ChunkDesc, LoadError> {
    if forms.len() == 1 {
        if let Some(name) = forms[0].as_sym() {
            if name != "ISA" {
                return Ok(ChunkDesc::Named(name.to_string()));
            }
        }
    }
    let (ctype, rest) = match forms {
        [Sexp::Sym(isa), Sexp::Sym(t), rest @ ..] if isa == "ISA" => (t.clone(), rest),
        _ => (String::new(), forms),
    };
    // Negation is meaningless in a construction context.
    for f in rest {
        if matches!(f, Sexp::Sym(t) if t == "-") {
            return structural("`-` negation not allowed in chunk construction");
        }
    }
    Ok(ChunkDesc::Spec {
        ctype,
        slots: parse_pairs_plain(rest)?,
    })
}

fn parse_lhs(forms: &[Sexp]) -> Result<Vec<LhsItem>, LoadError> {
    let mut items = Vec::new();
    let mut i = 0;
    while i < forms.len() {
        let (op, buffer) = match &forms[i] {
            Sexp::Sym(t) => match buffer_ref(t) {
                Some(b) => b,
                None => return structural(format!("LHS: buffer ref expected, got {t}")),
            },
            got => return structural(format!("LHS: buffer ref expected, got {got:?}")),
        };
        i += 1;
        let mut j = i;
        while j < forms.len() && !is_action_head(&forms[j]) {
            j += 1;
        }
        let body = &forms[i..j];
        match op {
            '=' | '+' => {
                // `+` on the LHS never occurs in M0 models; accept like `=`
                // so the loader stays total over match-shaped groups.
                let tests = parse_pairs(body)?;
                items.push(LhsItem::Match {
                    buffer: buffer.to_string(),
                    tests,
                });
            }
            '?' => {
                let qs = parse_pairs(body)?;
                for q in &qs {
                    if q.negated {
                        return structural("`-` negation not allowed in `?` queries");
                    }
                }
                items.push(LhsItem::Query {
                    buffer: buffer.to_string(),
                    queries: qs.into_iter().map(|q| (q.slot, q.value)).collect(),
                });
            }
            _ => return structural(format!("LHS: unexpected `{op}` buffer ref")),
        }
        i = j;
    }
    Ok(items)
}

fn parse_rhs(forms: &[Sexp]) -> Result<Vec<RhsAction>, LoadError> {
    let mut actions = Vec::new();
    let mut i = 0;
    while i < forms.len() {
        let head = match &forms[i] {
            Sexp::Sym(t) => t.clone(),
            got => return structural(format!("RHS: action head expected, got {got:?}")),
        };
        i += 1;
        let mut j = i;
        while j < forms.len() && !is_action_head(&forms[j]) {
            j += 1;
        }
        let body = &forms[i..j];
        if let Some((op, buffer)) = buffer_ref(&head) {
            let buffer = buffer.to_string();
            match op {
                '=' => actions.push(RhsAction::Modify {
                    buffer,
                    slots: parse_pairs_plain(body)?,
                }),
                '+' => actions.push(RhsAction::Request {
                    buffer,
                    spec: parse_chunk_desc(body)?,
                }),
                '-' => {
                    if !body.is_empty() {
                        return structural(format!("clear `-{buffer}>` takes no args"));
                    }
                    actions.push(RhsAction::Clear { buffer });
                }
                _ => return structural(format!("RHS: unexpected `{op}` buffer ref")),
            }
        } else if head.starts_with('!') && head.ends_with('!') {
            if head == "!OUTPUT!" {
                actions.push(RhsAction::Output {
                    args: body.to_vec(),
                });
            } else {
                actions.push(RhsAction::Other {
                    head,
                    args: body.to_vec(),
                });
            }
        } else {
            return structural(format!("RHS: action head expected, got {head}"));
        }
        i = j;
    }
    Ok(actions)
}

fn parse_production(form: &Sexp) -> Result<Production, LoadError> {
    let items = match form {
        Sexp::List(items) => items,
        _ => return structural("production must be a list"),
    };
    let (head, name, body) = match items.as_slice() {
        [Sexp::Sym(h), Sexp::Sym(n), rest @ ..] if h == "P" => (h, n, rest),
        got => return structural(format!("`(p name ...)` expected, got {got:?}")),
    };
    let _ = head;
    let split = body
        .iter()
        .position(|s| matches!(s, Sexp::Sym(t) if t == "==>"))
        .ok_or_else(|| LoadError::Structural(format!("production {name} has no `==>`")))?;
    Ok(Production {
        name: name.clone(),
        lhs: parse_lhs(&body[..split])?,
        rhs: parse_rhs(&body[split + 1..])?,
    })
}

fn parse_chunk_type(form: &Sexp) -> Result<ChunkType, LoadError> {
    match form {
        Sexp::List(items) => match items.as_slice() {
            [Sexp::Sym(h), Sexp::Sym(name), slots @ ..] if h == "CHUNK-TYPE" => {
                let mut out = Vec::new();
                for s in slots {
                    match s {
                        Sexp::Sym(t) => out.push(t.clone()),
                        got => {
                            return structural(format!("chunk-type slot must be a symbol: {got:?}"))
                        }
                    }
                }
                Ok(ChunkType {
                    name: name.clone(),
                    slots: out,
                })
            }
            got => structural(format!("bad chunk-type form: {got:?}")),
        },
        got => structural(format!("bad chunk-type form: {got:?}")),
    }
}

fn parse_dm_entry(form: &Sexp) -> Result<Chunk, LoadError> {
    match form {
        Sexp::List(items) => match items.as_slice() {
            [Sexp::Sym(name)] => Ok(Chunk {
                name: name.clone(),
                ctype: None,
                slots: Vec::new(),
            }),
            [Sexp::Sym(name), Sexp::Sym(isa), Sexp::Sym(t), rest @ ..] if isa == "ISA" => {
                Ok(Chunk {
                    name: name.clone(),
                    ctype: Some(t.clone()),
                    slots: parse_pairs_plain(rest)?,
                })
            }
            got => structural(format!("bad add-dm entry: {got:?}")),
        },
        got => structural(format!("bad add-dm entry: {got:?}")),
    }
}

fn parse_model(form: &Sexp) -> Result<Model, LoadError> {
    let body = match form {
        Sexp::List(items) => match items.as_slice() {
            [Sexp::Sym(h), Sexp::Sym(name), rest @ ..] if h == "DEFINE-MODEL" => {
                (name.clone(), rest)
            }
            got => return structural(format!("define-model expected, got {got:?}")),
        },
        got => return structural(format!("define-model expected, got {got:?}")),
    };
    let (name, forms) = body;
    let mut model = Model {
        name,
        params: Vec::new(),
        chunk_types: Vec::new(),
        chunks: Vec::new(),
        initial_goal: None,
        productions: Vec::new(),
    };
    for f in forms {
        let items = match f {
            Sexp::List(items) => items,
            got => return structural(format!("model body form must be a list: {got:?}")),
        };
        let head = match items.first() {
            Some(Sexp::Sym(h)) => h.as_str(),
            got => return structural(format!("model body head must be a symbol: {got:?}")),
        };
        match head {
            "SGP" => {
                let rest = &items[1..];
                if rest.len() % 2 != 0 {
                    return structural("sgp needs even key/value count");
                }
                for kv in rest.as_chunks::<2>().0 {
                    let key = match &kv[0] {
                        Sexp::Sym(k) => k.clone(),
                        got => return structural(format!("sgp key must be a symbol: {got:?}")),
                    };
                    model.params.push((key, kv[1].clone()));
                }
            }
            "CHUNK-TYPE" => model.chunk_types.push(parse_chunk_type(f)?),
            "ADD-DM" => {
                for e in &items[1..] {
                    model.chunks.push(parse_dm_entry(e)?);
                }
            }
            "GOAL-FOCUS" => {
                if items.len() != 2 {
                    return structural("goal-focus takes exactly one argument");
                }
                let desc = match &items[1] {
                    Sexp::List(spec) => parse_chunk_desc(spec)?,
                    Sexp::Sym(n) => ChunkDesc::Named(n.clone()),
                    got => return structural(format!("bad goal-focus arg: {got:?}")),
                };
                model.initial_goal = Some(desc);
            }
            "P" => model.productions.push(parse_production(f)?),
            got => return structural(format!("unknown model body form: {got}")),
        }
    }
    Ok(model)
}

/// Load every `define-model` in a `.lisp` model file (M0 files hold one).
/// `(clear-all)` is acknowledged and skipped; anything else top-level errors.
pub fn load_models(path: &Path) -> Result<Vec<Model>, LoadError> {
    let src = fs::read_to_string(path).map_err(LoadError::Io)?;
    let mut models = Vec::new();
    for form in parse_all(&src)? {
        match &form {
            Sexp::List(items) => match items.first() {
                Some(Sexp::Sym(h)) if h == "CLEAR-ALL" => {}
                Some(Sexp::Sym(h)) if h == "DEFINE-MODEL" => models.push(parse_model(&form)?),
                got => return structural(format!("unexpected top-level form: {got:?}")),
            },
            got => return structural(format!("unexpected top-level form: {got:?}")),
        }
    }
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn m0(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("actr7.x")
            .join("tutorial")
            .join("unit1")
            .join(name)
    }

    fn the_only(path: &str) -> Model {
        let models = load_models(&m0(path)).unwrap();
        assert_eq!(models.len(), 1);
        models.into_iter().next().unwrap()
    }

    fn slots_of(chunk: &Chunk) -> Vec<(&str, String)> {
        chunk
            .slots
            .iter()
            .map(|(s, v)| (s.as_str(), format!("{v:?}")))
            .collect()
    }

    #[test]
    fn count_loads() {
        let m = the_only("count.lisp");
        assert_eq!(m.name, "COUNT");
        assert_eq!(
            m.chunk_types,
            vec![
                ChunkType {
                    name: "NUMBER".into(),
                    slots: vec!["NUMBER".into(), "NEXT".into()],
                },
                ChunkType {
                    name: "COUNT-FROM".into(),
                    slots: vec!["START".into(), "END".into(), "COUNT".into()],
                },
            ]
        );
        assert_eq!(m.chunks.len(), 5);
        let one = &m.chunks[0];
        assert_eq!(one.name, "ONE");
        assert_eq!(one.ctype.as_deref(), Some("NUMBER"));
        assert_eq!(
            slots_of(one),
            vec![
                ("NUMBER", "Sym(\"ONE\")".into()),
                ("NEXT", "Sym(\"TWO\")".into())
            ]
        );
        // `(five ISA number number five)` — type with a single slot.
        let five = &m.chunks[4];
        assert_eq!(five.ctype.as_deref(), Some("NUMBER"));
        assert_eq!(slots_of(five), vec![("NUMBER", "Sym(\"FIVE\")".into())]);

        match m.initial_goal.as_ref().unwrap() {
            ChunkDesc::Spec { ctype, slots } => {
                assert_eq!(ctype, "COUNT-FROM");
                let get = |s: &str| slots.iter().find(|(k, _)| k == s).map(|(_, v)| v.clone());
                assert_eq!(get("START"), Some(Pattern::Sym("TWO".into())));
                assert_eq!(get("END"), Some(Pattern::Sym("FOUR".into())));
            }
            got => panic!("goal should be a spec: {got:?}"),
        }

        let names: Vec<&str> = m.productions.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["START", "INCREMENT", "STOP"]);

        // `start` LHS: =goal> with ISA + var + nil tests.
        let start = &m.productions[0];
        assert_eq!(start.lhs.len(), 1);
        match &start.lhs[0] {
            LhsItem::Match { buffer, tests } => {
                assert_eq!(buffer, "GOAL");
                assert_eq!(tests.len(), 3);
                assert_eq!(tests[0].slot, "ISA");
                assert_eq!(tests[0].value, Pattern::Sym("COUNT-FROM".into()));
                assert_eq!(tests[1].value, Pattern::Var("NUM1".into()));
                assert!(!tests[1].negated);
                assert_eq!(tests[2].slot, "COUNT");
                assert_eq!(tests[2].value, Pattern::Nil);
            }
            got => panic!("{got:?}"),
        }
        // `increment` has a `- end =num1` negated test.
        let inc = &m.productions[1];
        let neg: Vec<_> = inc
            .lhs
            .iter()
            .flat_map(|item| match item {
                LhsItem::Match { tests, .. } => tests.clone(),
                _ => vec![],
            })
            .filter(|t| t.negated)
            .collect();
        assert_eq!(neg.len(), 1);
        assert_eq!(neg[0].slot, "END");
        assert_eq!(neg[0].value, Pattern::Var("NUM1".into()));
        // `increment` RHS requests a retrieval and prints.
        assert!(matches!(
            &inc.rhs[1],
            RhsAction::Request { buffer, .. } if buffer == "RETRIEVAL"
        ));
        assert!(matches!(inc.rhs[2], RhsAction::Output { .. }));
        // `stop` RHS clears goal then prints.
        let stop = &m.productions[2];
        assert_eq!(stop.rhs.len(), 2);
        assert!(matches!(
            &stop.rhs[0],
            RhsAction::Clear { buffer } if buffer == "GOAL"
        ));

        let param = |k: &str| m.params.iter().find(|(key, _)| key == k).map(|(_, v)| v);
        assert_eq!(param(":ESC"), Some(&Sexp::Sym("T".into())));
        assert!(matches!(param(":LF"), Some(Sexp::Float(x)) if (*x - 0.05).abs() < 1e-12));
    }

    #[test]
    fn semantic_loads_bare_chunks_and_queries() {
        let m = the_only("semantic.lisp");
        assert_eq!(m.name, "SEMANTIC");
        // `(shark)` — chunk with no slots and no type.
        let shark = m.chunks.iter().find(|c| c.name == "SHARK").unwrap();
        assert_eq!(shark.ctype, None);
        assert!(shark.slots.is_empty());
        let p14 = m.chunks.iter().find(|c| c.name == "P14").unwrap();
        assert_eq!(p14.ctype.as_deref(), Some("PROPERTY"));

        let names: Vec<&str> = m.productions.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "INITIAL-RETRIEVE",
                "DIRECT-VERIFY",
                "CHAIN-CATEGORY",
                "FAIL"
            ]
        );
        // `fail` LHS ends with `?retrieval> buffer failure`.
        let fail = &m.productions[3];
        match fail.lhs.last().unwrap() {
            LhsItem::Query { buffer, queries } => {
                assert_eq!(buffer, "RETRIEVAL");
                assert_eq!(
                    queries,
                    &vec![("BUFFER".to_string(), Pattern::Sym("FAILURE".into()))]
                );
            }
            got => panic!("{got:?}"),
        }
    }

    #[test]
    fn addition_loads() {
        let m = the_only("addition.lisp");
        assert_eq!(m.name, "ADDITION");
        assert_eq!(m.chunks.len(), 11);
        let names: Vec<&str> = m.productions.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "INITIALIZE-ADDITION",
                "TERMINATE-ADDITION",
                "INCREMENT-COUNT",
                "INCREMENT-SUM"
            ]
        );
        match m.initial_goal.as_ref().unwrap() {
            ChunkDesc::Spec { ctype, slots } => {
                assert_eq!(ctype, "ADD");
                assert!(slots.contains(&("ARG1".to_string(), Pattern::Sym("FIVE".into()))));
            }
            got => panic!("{got:?}"),
        }
    }

    #[test]
    fn structural_errors_are_loud() {
        // `==>` missing.
        assert!(parse_production(&parse_all("(p x =goal> ISA y)").unwrap()[0]).is_err());
        // Odd slot/value count.
        assert!(parse_pairs(&[Sexp::Sym("A".into())]).is_err());
        // Clear with args.
        assert!(parse_rhs(&parse_all("-goal> ISA x").unwrap())
            .unwrap_err()
            .to_string()
            .contains("takes no args"));
    }
}

//! Minimal S-expression reader for ACT-R model files.
//!
//! Covers the subset used by `define-model` files: nested lists, symbols
//! (case-folded to UPPERCASE like Lisp reader defaults — `isa` and `ISA`
//! are the same symbol), integers, floats (`.05`, `-2`, `0.050`, `1e3`),
//! double-quoted strings with `\"`/`\\` escapes, `'x` quote sugar,
//! `;` line comments and nestable `#| ... |#` block comments.
//! Anything else (reader macros, `|...|` beyond simple quoting) errors
//! loudly rather than mis-parsing.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Sexp {
    Sym(String),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Sexp>),
}

impl Sexp {
    /// Symbol text, if this is a symbol.
    pub fn as_sym(&self) -> Option<&str> {
        match self {
            Sexp::Sym(s) => Some(s),
            _ => None,
        }
    }

    /// `true` for the symbol `NIL` (any case in source; we fold).
    pub fn is_nil(&self) -> bool {
        matches!(self, Sexp::Sym(s) if s == "NIL")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub msg: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at line {}: {}", self.line, self.msg)
    }
}

impl std::error::Error for ParseError {}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    line: usize,
}

impl Parser {
    fn new(src: &str) -> Self {
        Parser {
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError {
            line: self.line,
            msg: msg.into(),
        })
    }

    /// Whitespace, `;` line comments, `#| |#` block comments (nestable).
    fn skip_trivia(&mut self) -> Result<(), ParseError> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some(';') => {
                    while !matches!(self.peek(), None | Some('\n')) {
                        self.bump();
                    }
                }
                Some('#') if self.chars.get(self.pos + 1) == Some(&'|') => {
                    self.pos += 2;
                    let mut depth = 1;
                    while depth > 0 {
                        match (self.peek(), self.chars.get(self.pos + 1).copied()) {
                            (None, _) => {
                                return self.err("unterminated #| comment");
                            }
                            (Some('#'), Some('|')) => {
                                self.pos += 2;
                                depth += 1;
                            }
                            (Some('|'), Some('#')) => {
                                self.pos += 2;
                                depth -= 1;
                            }
                            (Some(_), _) => {
                                self.bump();
                            }
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn parse_all(&mut self) -> Result<Vec<Sexp>, ParseError> {
        let mut out = Vec::new();
        loop {
            self.skip_trivia()?;
            if self.peek().is_none() {
                return Ok(out);
            }
            out.push(self.parse_expr()?);
        }
    }

    fn parse_expr(&mut self) -> Result<Sexp, ParseError> {
        self.parse_expr_in(false)
    }

    fn parse_expr_in(&mut self, quasiquote: bool) -> Result<Sexp, ParseError> {
        self.skip_trivia()?;
        match self.peek() {
            None => self.err("unexpected end of input"),
            Some('(') => {
                self.bump();
                let mut items = Vec::new();
                loop {
                    self.skip_trivia()?;
                    match self.peek() {
                        None => return self.err("unterminated list"),
                        Some(')') => {
                            self.bump();
                            return Ok(Sexp::List(items));
                        }
                        Some(_) => items.push(self.parse_expr_in(quasiquote)?),
                    }
                }
            }
            Some(')') => self.err("unexpected `)`"),
            Some('"') => self.parse_string(),
            Some('\'') => {
                self.bump();
                let q = self.parse_expr_in(quasiquote)?;
                Ok(Sexp::List(vec![Sexp::Sym("QUOTE".into()), q]))
            }
            Some('`') => {
                self.bump();
                let q = self.parse_expr_in(true)?;
                Ok(Sexp::List(vec![Sexp::Sym("QUASIQUOTE".into()), q]))
            }
            Some(',') if quasiquote => {
                self.bump();
                let splicing = self.peek() == Some('@');
                if splicing {
                    self.bump();
                }
                let q = self.parse_expr_in(false)?;
                let tag = if splicing {
                    "UNQUOTE-SPLICING"
                } else {
                    "UNQUOTE"
                };
                Ok(Sexp::List(vec![Sexp::Sym(tag.into()), q]))
            }
            Some(_) => self.parse_atom(),
        }
    }

    fn parse_string(&mut self) -> Result<Sexp, ParseError> {
        assert_eq!(self.bump(), Some('"'));
        let mut s = String::new();
        loop {
            match self.bump() {
                None => return self.err("unterminated string"),
                Some('"') => return Ok(Sexp::Str(s)),
                Some('\\') => match self.bump() {
                    None => return self.err("unterminated string escape"),
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some(c) => s.push(c),
                },
                Some(c) => s.push(c),
            }
        }
    }

    fn parse_atom(&mut self) -> Result<Sexp, ParseError> {
        // `|...|` quoted symbols: content kept verbatim, no case fold.
        if self.peek() == Some('|') {
            self.bump();
            let mut s = String::new();
            loop {
                match self.bump() {
                    None => return self.err("unterminated | symbol"),
                    Some('|') => return Ok(Sexp::Sym(s)),
                    Some(c) => s.push(c),
                }
            }
        }
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace() || "()\"';".contains(c) {
                break;
            }
            s.push(c);
            self.bump();
        }
        if s.is_empty() {
            return self.err("unexpected character");
        }
        if let Ok(i) = s.parse::<i64>() {
            return Ok(Sexp::Int(i));
        }
        if let Ok(x) = s.parse::<f64>() {
            // Guard: Rust parses "inf"/"nan" as floats; ACT-R files never
            // mean those, and a bare "." must stay a symbol.
            if s.eq_ignore_ascii_case("inf")
                || s.eq_ignore_ascii_case("+inf")
                || s.eq_ignore_ascii_case("-inf")
                || s.eq_ignore_ascii_case("nan")
                || !s.chars().any(|c| c.is_ascii_digit())
            {
                return Ok(Sexp::Sym(fold(s)));
            }
            return Ok(Sexp::Float(x));
        }
        Ok(Sexp::Sym(fold(s)))
    }
}

/// Lisp reader default: unescaped symbols fold to uppercase.
fn fold(s: String) -> String {
    s.to_ascii_uppercase()
}

/// Parse every top-level form in `src`.
pub fn parse_all(src: &str) -> Result<Vec<Sexp>, ParseError> {
    Parser::new(src).parse_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(src: &str) -> Sexp {
        let forms = parse_all(src).unwrap();
        assert_eq!(forms.len(), 1, "want exactly one form in {src:?}");
        forms.into_iter().next().unwrap()
    }

    #[test]
    fn nesting_and_atoms() {
        assert_eq!(
            one("(p start =goal> ISA count (run 10))"),
            Sexp::List(vec![
                Sexp::Sym("P".into()),
                Sexp::Sym("START".into()),
                Sexp::Sym("=GOAL>".into()),
                Sexp::Sym("ISA".into()),
                Sexp::Sym("COUNT".into()),
                Sexp::List(vec![Sexp::Sym("RUN".into()), Sexp::Int(10)]),
            ])
        );
    }

    #[test]
    fn case_folds_but_strings_do_not() {
        assert_eq!(one("isa"), Sexp::Sym("ISA".into()));
        assert_eq!(one("(goal-focus \"Letter Difference\")"), {
            Sexp::List(vec![
                Sexp::Sym("GOAL-FOCUS".into()),
                Sexp::Str("Letter Difference".into()),
            ])
        });
    }

    #[test]
    fn numbers() {
        assert_eq!(one(".05"), Sexp::Float(0.05));
        assert_eq!(one("-2"), Sexp::Int(-2));
        assert_eq!(one("0.050"), Sexp::Float(0.05));
        assert_eq!(one("10"), Sexp::Int(10));
        assert_eq!(one("-"), Sexp::Sym("-".into()));
        assert_eq!(one("=num1"), Sexp::Sym("=NUM1".into()));
        assert_eq!(one("nil"), Sexp::Sym("NIL".into()));
    }

    #[test]
    fn quasiquote_and_unquote() {
        assert_eq!(
            one("`((,x isa chunk))"),
            Sexp::List(vec![
                Sexp::Sym("QUASIQUOTE".into()),
                Sexp::List(vec![Sexp::List(vec![
                    Sexp::List(vec![Sexp::Sym("UNQUOTE".into()), Sexp::Sym("X".into())]),
                    Sexp::Sym("ISA".into()),
                    Sexp::Sym("CHUNK".into()),
                ])]),
            ])
        );
        assert_eq!(
            one("`(isa example ,@(mapcan 'list *slots* x))"),
            Sexp::List(vec![
                Sexp::Sym("QUASIQUOTE".into()),
                Sexp::List(vec![
                    Sexp::Sym("ISA".into()),
                    Sexp::Sym("EXAMPLE".into()),
                    Sexp::List(vec![
                        Sexp::Sym("UNQUOTE-SPLICING".into()),
                        Sexp::List(vec![
                            Sexp::Sym("MAPCAN".into()),
                            Sexp::List(vec![Sexp::Sym("QUOTE".into()), Sexp::Sym("LIST".into())]),
                            Sexp::Sym("*SLOTS*".into()),
                            Sexp::Sym("X".into()),
                        ]),
                    ]),
                ]),
            ])
        );
    }

    #[test]
    fn comments_and_quote() {
        let forms =
            parse_all("; leading comment\n(a 1) #| block #| nested |# still |# (b 2) ; trail")
                .unwrap();
        assert_eq!(forms.len(), 2);
        assert_eq!(
            one("'x"),
            Sexp::List(vec![Sexp::Sym("QUOTE".into()), Sexp::Sym("X".into())])
        );
    }

    #[test]
    fn errors_have_lines() {
        let e = parse_all("(a 1)\n(b .").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(parse_all("(a").is_err());
        assert!(parse_all("#| never closed").is_err());
    }
}

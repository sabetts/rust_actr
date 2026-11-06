//! Minimal JSON value/parser/serializer for the ACT-R RPC bridge.
//!
//! The crate has no external dependencies, and the RPC protocol
//! (`framework/dispatcher.lisp`, `tutorial/python/actr.py`) only needs plain
//! JSON: strings, numbers, booleans, `null`, arrays and objects. Python's
//! `json.dumps` produces this subset; the server's replies use the same
//! shapes (`{"id":..,"result":..,"error":..}`).

use std::fmt::Write as _;

/// A parsed JSON value. Object key order is preserved (Lisp's `json` package
/// emits alists in order and replies are echoed back).
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Default for Json {
    fn default() -> Self {
        Json::Null
    }
}

impl Json {
    /// A JSON array of `items`.
    pub fn arr(items: Vec<Json>) -> Json {
        Json::Arr(items)
    }

    /// The value at `key` of an object (or `None`).
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// A JSON string value, or `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Any numeric value as an `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Int(i) => Some(*i as f64),
            Json::Float(x) => Some(*x),
            _ => None,
        }
    }

    /// Serialize to a compact JSON string (no pretty printing, matching the
    /// wire format the dispatcher writes).
    pub fn write(&self) -> String {
        let mut s = String::new();
        self.write_into(&mut s);
        s
    }

    fn write_into(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(true) => out.push_str("true"),
            Json::Bool(false) => out.push_str("false"),
            Json::Int(i) => {
                let _ = write!(out, "{i}");
            }
            Json::Float(x) => write_float(*x, out),
            Json::Str(s) => write_string(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write_into(out);
                }
                out.push(']');
            }
            Json::Obj(kv) => {
                out.push('{');
                for (i, (k, v)) in kv.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(k, out);
                    out.push(':');
                    v.write_into(out);
                }
                out.push('}');
            }
        }
    }
}

/// JSON floats keep a decimal point even when integral, matching Python's
/// `json.dumps` (`0.0` stays a float, not the integer `0`).
fn write_float(x: f64, out: &mut String) {
    // ACT-R returns single floats for model values. When this f64 is exactly
    // representable as that single float, print its shortest round-trip form
    // (`0.1899999976158142` -> `0.19`), matching the Lisp JSON bridge.
    let text = if x != 0.0 && (x as f32) as f64 == x {
        format!("{}", x as f32)
    } else {
        format!("{x}")
    };
    out.push_str(&text);
    if !text.contains('.') && !text.contains('e') && !text.contains('E') {
        out.push_str(".0");
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Parse a complete JSON document. Trailing whitespace is allowed; trailing
/// non-whitespace is an error.
pub fn parse(s: &str) -> Result<Json, String> {
    let mut p = Parser {
        bytes: s.as_bytes(),
        pos: 0,
    };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.pos != p.bytes.len() {
        return Err(format!("trailing data at byte {}", p.pos));
    }
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while let Some(&c) = self.bytes.get(self.pos) {
            if matches!(c, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn value(&mut self) -> Result<Json, String> {
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            Some(c) => Err(format!("unexpected byte {c:#x} at {}", self.pos)),
            None => Err("unexpected end of input".into()),
        }
    }

    fn literal(&mut self, lit: &str, v: Json) -> Result<Json, String> {
        if self.bytes[self.pos..].starts_with(lit.as_bytes()) {
            self.pos += lit.len();
            Ok(v)
        } else {
            Err(format!("expected {lit} at {}", self.pos))
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.pos += 1; // {
        let mut kv = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Obj(kv));
        }
        loop {
            self.ws();
            let key = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return Err(format!("expected ':' at {}", self.pos));
            }
            self.pos += 1;
            self.ws();
            let val = self.value()?;
            kv.push((key, val));
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or '}}' at {}", self.pos)),
            }
        }
        Ok(Json::Obj(kv))
    }

    fn array(&mut self) -> Result<Json, String> {
        self.pos += 1; // [
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            self.ws();
            items.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or ']' at {}", self.pos)),
            }
        }
        Ok(Json::Arr(items))
    }

    fn string(&mut self) -> Result<String, String> {
        if self.peek() != Some(b'"') {
            return Err(format!("expected string at {}", self.pos));
        }
        self.pos += 1;
        let mut out = String::new();
        loop {
            let c = self.peek().ok_or("unterminated string")?;
            match c {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    let esc = self.peek().ok_or("unterminated escape")?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{08}'),
                        b'f' => out.push('\u{0c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let cp = self.hex4()?;
                            // Surrogate pair.
                            if (0xd800..0xdc00).contains(&cp) {
                                if self.peek() == Some(b'\\') {
                                    self.pos += 1;
                                    if self.peek() == Some(b'u') {
                                        self.pos += 1;
                                        let lo = self.hex4()?;
                                        let c = 0x10000
                                            + ((cp - 0xd800) << 10)
                                            + (lo.wrapping_sub(0xdc00));
                                        out.push(char::from_u32(c).unwrap_or('\u{fffd}'));
                                        continue;
                                    }
                                }
                                out.push('\u{fffd}');
                            } else {
                                out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
                            }
                        }
                        other => return Err(format!("bad escape \\{other}")),
                    }
                }
                _ => {
                    // Copy one UTF-8 scalar.
                    let start = self.pos;
                    let len = utf8_len(c);
                    if start + len > self.bytes.len() {
                        return Err("truncated UTF-8".into());
                    }
                    let chunk = std::str::from_utf8(&self.bytes[start..start + len])
                        .map_err(|e| e.to_string())?;
                    out.push_str(chunk);
                    self.pos += len;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.bytes.len() {
            return Err("truncated \\u escape".into());
        }
        let s =
            std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]).map_err(|e| e.to_string())?;
        self.pos += 4;
        u32::from_str_radix(s, 16).map_err(|e| e.to_string())
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| e.to_string())?;
        if is_float {
            text.parse::<f64>()
                .map(Json::Float)
                .map_err(|e| format!("bad number {text}: {e}"))
        } else {
            text.parse::<i64>()
                .map(Json::Int)
                .or_else(|_| text.parse::<f64>().map(Json::Float))
                .map_err(|e| format!("bad number {text}: {e}"))
        }
    }
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_request() {
        let j = parse(r#"{"method":"evaluate","id":1,"params":["run",null,10]}"#).unwrap();
        assert_eq!(j.get("method").unwrap().as_str(), Some("evaluate"));
        assert_eq!(j.get("params").unwrap().write(), r#"["run",null,10]"#);
    }

    #[test]
    fn nested_and_escapes() {
        let src = r#"{"a":[1,"x\"y",true,false,null,{"n":-2.5}]}"#;
        let j = parse(src).unwrap();
        assert_eq!(j.write(), src);
    }

    #[test]
    fn integral_floats_print_like_python() {
        assert_eq!(Json::Float(10.0).write(), "10.0");
        assert_eq!(Json::Float(0.0).write(), "0.0");
        assert_eq!(Json::Float(1.5).write(), "1.5");
    }

    #[test]
    fn f32_values_print_shortest_single_float() {
        assert_eq!(Json::Float(0.1899999976158142).write(), "0.19");
        assert_eq!(Json::Float(0.5900000333786011).write(), "0.59000003");
    }

    #[test]
    fn strings_and_lists() {
        // Python sends chunk specs as flat string lists.
        let j = parse(r#"["ISA","is-member","object","canary"]"#).unwrap();
        assert_eq!(j.write(), r#"["ISA","is-member","object","canary"]"#);
    }

    #[test]
    fn rejects_trailing() {
        assert!(parse("1 2").is_err());
    }
}

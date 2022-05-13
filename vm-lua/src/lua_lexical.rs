use smallstr::SmallString;
use smallvec::SmallVec;
use std::{iter::Iterator, str::{from_utf8, Chars}};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LitStringPrefix {
    /// '\''
    Apostrophe,
    /// '"'
    DoubleQuotes,
    /// '[['
    LongBracket(usize),
}
impl LitStringPrefix {
    pub fn match_end(&self, iter: &mut Chars, c: char) -> bool {
        match self {
            Self::Apostrophe => c == '\'',
            Self::DoubleQuotes => c == '\"',
            Self::LongBracket(l) => {
                let mut iter_clone = iter.clone();
                if c != ']' {
                    return false;
                }
                for _i in 0..*l {
                    if iter_clone.next() != Some('=') {
                        return false;
                    }
                }
                if iter_clone.next() != Some(']') {
                    return false;
                }
                *iter = iter_clone;
                true
            }
        }
    }
}
fn parse_string(iter: &mut Chars) -> Option<String> {
    let mut lit = Vec::<u8>::new();
    let prefix = match iter.next()? {
        '\'' => LitStringPrefix::Apostrophe,
        '"' => LitStringPrefix::DoubleQuotes,
        '[' => {
            let mut level = 0;
            loop {
                match iter.next()? {
                    '[' => break,
                    '=' => {
                        level += 1;
                    }
                    _ => return None,
                }
            }
            LitStringPrefix::LongBracket(level)
        }
        _ => return None,
    };
    while let Some(c) = iter.next() {
        match c {
            '\'' | '\"' | ']' => {
                if prefix.match_end(iter, c) {
                    return String::from_utf8(lit).ok();
                }
            }
            '\n' => {
                if prefix == LitStringPrefix::Apostrophe || prefix == LitStringPrefix::DoubleQuotes {
                    return None;
                } else {
                    lit.push(b'\n');
                }
            }
            '\\' => {
                match iter.next()? {
                    '\\' => lit.extend_from_slice(&[b'\\']),
                    'a' => lit.extend_from_slice(&[b'\x07']),
                    'b' => lit.extend_from_slice(&[b'\x08']),
                    'f' => lit.extend_from_slice(&[b'\x0c']),
                    'n' => lit.extend_from_slice(&[b'\n']),
                    'r' => lit.extend_from_slice(&[b'\r']),
                    't' => lit.extend_from_slice(&[b'\t']),
                    'v' => lit.extend_from_slice(&[b'\x0b']),
                    '\"' => lit.extend_from_slice(&[b'\"']),
                    '\'' => lit.extend_from_slice(&[b'\'']),
                    'z' => lit.extend_from_slice(&[b'1']),
                    'x' => {
                        let c2 = iter.next()?;
                        let c3 = iter.next()?;
                        let n2 = match c2 {
                            '0'..='9' => c2 as u8 - b'0',
                            'a'..='f' => c2 as u8 - b'a' + 10,
                            'A'..='F' => c2 as u8 - b'A' + 10,
                            _ => return None,
                        };
                        let n3 = match c3 {
                            '0'..='9' => c3 as u8 - b'0',
                            'a'..='f' => c3 as u8 - b'a' + 10,
                            'A'..='F' => c3 as u8 - b'A' + 10,
                            _ => return None,
                        };
                        lit.extend_from_slice(&[n2 << 4 | n3]);
                    }
                    'u' => {
                        let mut buffer = SmallVec::<[u8; 8]>::new();
                        loop {
                            let mut char_buffer = [0u8; 8];
                            buffer.extend_from_slice(iter.next()?.encode_utf8(&mut char_buffer).as_bytes());
                            if let Ok(_r) = from_utf8(&*buffer) {
                                break;
                            }
                        }
                        lit.extend_from_slice(&*buffer);
                    }
                    '0'..='9' => {
                        let c2 = iter.next()?;
                        let c3 = iter.next()?;
                        if !matches!(c2, '0'..='9') || !matches!(c3, '0'..='9') {
                            return None;
                        }
                        lit.extend_from_slice(&[((c as u8 - b'0') << 6)
                            | ((c2 as u8 - b'0') << 3)
                            | (c3 as u8 - b'0')]);
                    }
                    _ => return None,
                };
            }
            o => {
                if o.is_ascii() {
                    lit.push(o as u8);
                } else {
                    let mut temp = SmallString::<[u8; 8]>::new();
                    temp.push(o);
                    lit.extend_from_slice(temp.as_bytes());
                }
            }
        }
    }
    None
}
fn parse_single_line_annotation(_iter: &mut Chars) -> Option<String> {
    todo!();
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LuaNumberLit {
    Integer(i64),
    Float(f64),
}

impl LuaNumberLit {
    pub fn push(&mut self, i: i64, base: i64) {
        match self {
            LuaNumberLit::Integer(f) => {
                if let Some(t) = f.checked_mul(base).and_then(|f| f.checked_add(i)) {
                    *f = t;
                } else {
                    *self = Self::Float(*f as f64 * base as f64 + i as f64);
                }
            }
            LuaNumberLit::Float(f) => *f = *f * base as f64 + i as f64,
        }
    }
    pub fn push_after_point(&mut self, i: i64, base: i64, d: i32) {
        self.to_float();
        match self {
            LuaNumberLit::Integer(_f) => unreachable!(),

            LuaNumberLit::Float(f) => *f += i as f64 * (1.0 / base as f64).powi(d),
        }
    }
    pub fn to_float(&mut self) {
        match self {
            LuaNumberLit::Integer(i) => *self = Self::Float(*i as f64),
            LuaNumberLit::Float(_) => {}
        }
    }
    pub fn push_e(&mut self, i: i32, base: f64) {
        match self {
            LuaNumberLit::Integer(f) => *self = Self::Float((*f as f64) * base.powi(i)),
            LuaNumberLit::Float(f) => *f *= base.powi(i),
        }
    }
}
fn parse_number(iter: &mut Chars) -> Option<LuaNumberLit> {
    let mut number = LuaNumberLit::Integer(0);
    if iter.as_str().starts_with("0x") || iter.as_str().starts_with("0X") {
        iter.next();
        iter.next();
        let mut point_distance = 0;
        while let Some(c) = iter.as_str().chars().next() {
            if let Some(n) = c.to_digit(16) {
                iter.next();
                if point_distance == 0 {
                    number.push(n as i64, 16);
                } else {
                    number.push_after_point(n as i64, 16, point_distance);
                    point_distance += 1;
                }
            } else if c == '.' {
                iter.next();
                if point_distance == 0 {
                    point_distance = 1;
                } else {
                    return None;
                }
            } else if c == 'p' {
                iter.next();
                let mut p_number: i32 = 0;
                let mut neg_p = false;
                if '-' == iter.as_str().chars().next()? {
                    iter.next();
                    neg_p = true;
                } else {
                }
                while let Some(c) = iter.as_str().chars().next() {
                    if let Some(p) = c.to_digit(16) {
                        iter.next();
                        p_number = p_number.saturating_mul(10i32).saturating_add(p as i32);
                    } else {
                        break;
                    }
                }
                if neg_p {
                    p_number = p_number.saturating_neg();
                };
                number.push_e(p_number, 2.0);
                break;
            } else {
                break;
            }
        }
        Some(number)
    } else if iter.as_str().chars().next()?.is_digit(10) {
        let mut point_distance = 0;
        while let Some(c) = iter.as_str().chars().next() {
            if let Some(n) = c.to_digit(10) {
                iter.next();
                if point_distance == 0 {
                    number.push(n as i64, 10);
                } else {
                    number.push_after_point(n as i64, 10, point_distance);
                    point_distance += 1;
                }
            } else if c == '.' {
                iter.next();
                if point_distance == 0 {
                    point_distance = 1;
                } else {
                    return None;
                }
            } else if c == 'e' {
                iter.next();
                let mut e_number: i32 = 0;
                let mut neg_e = false;
                if '-' == iter.as_str().chars().next()? {
                    iter.next();
                    neg_e = true;
                } else {
                }
                while let Some(c) = iter.as_str().chars().next() {
                    if let Some(p) = c.to_digit(10) {
                        iter.next();
                        e_number = e_number.saturating_mul(10i32).saturating_add(p as i32);
                    } else {
                        break;
                    }
                }
                if neg_e {
                    e_number = e_number.saturating_neg();
                };
                number.push_e(e_number, 10.0);
                break;
            } else {
                break;
            }
        }
        Some(number)
    } else {
        None
    }
}
#[lexical(["and","break","do","else","elseif","end","false","for","function","if","in","local","nil","not","or","repeat","return","then","true","until","while","goto"],["=",",",";",".",":","::","(",")","[","]","{","}","+","-","*","/","//","%","^","<<",">>","==",">","<",">=","<=","..","...","#","|","&","~"])]
#[derive(Debug, Clone, PartialEq)]
pub enum LuaLexical {
    #[lexical(fn = "parse_number")]
    Number(LuaNumberLit),
    #[lexical(regex = r"[a-zA-Z_][a-zA-Z_0-9]*")]
    Name(String),
    #[lexical(fn = "parse_string")]
    String(String),
    #[lexical(ignore, regex = r#"--[^\n]*\n"#)]
    SingleLineAnnotation,
    #[lexical(ignore, regex = r#"--\[\[[*--]]\n"#)]
    MultiLineAnnotation,
    #[lexical(string = "~=")]
    NotEqual,
}

extern crate failure;
extern crate failure_derive;
use failure::Fallible;
pub use failure::{format_err as _format_err, Fallible as _Fallible};
pub use lazy_static as _lazy_static;
pub use regex as _regex;

pub trait Lexical: Sized {
    fn parse(source: &str) -> Fallible<Vec<Self>>;
}
pub fn to_ident(token: &str) -> String {
    match token {
        "" => "Empty".to_string(),
        "=" => "Assign".to_string(),
        "," => "Comma".to_string(),
        ";" => "Semicolon".to_string(),
        "." => "Dot".to_string(),
        ".." => "DoubleDot".to_string(),
        "..." => "TripleDot".to_string(),
        ":" => "Colon".to_string(),
        "::" => "DoubleColon".to_string(),
        "(" => "LeftParen".to_string(),
        ")" => "RightParen".to_string(),
        "[" => "LeftBracket".to_string(),
        "]" => "RightBracket".to_string(),
        "{" => "LeftBrace".to_string(),
        "}" => "RightBrace".to_string(),
        "+" => "Add".to_string(),
        "-" => "Sub".to_string(),
        "*" => "Mul".to_string(),
        "/" => "Div".to_string(),
        "//" => "DoubleSlash".to_string(),
        "%" => "Rem".to_string(),
        "^" => "Caret".to_string(),
        "#" => "Sharp".to_string(),
        "@" => "At".to_string(),
        "~" => "Tilde".to_string(),
        "<<" => "LeftShift".to_string(),
        "<<<" => "UnsignedLeftShift".to_string(),
        ">>" => "RightShift".to_string(),
        "==" => "Equal".to_string(),
        "!=" => "NotEqual".to_string(),
        ">" => "Large".to_string(),
        "<" => "Less".to_string(),
        ">=" => "LargeOrEqual".to_string(),
        "<=" => "LessOrEqual".to_string(),
        "|" => "BitOr".to_string(),
        "&" => "BitAnd".to_string(),
        "||" => "LogicalAnd".to_string(),
        "&&" => "LogicalOr".to_string(),
        "!" => "LogicalNot".to_string(),
        o => {
            let mut ident_name = String::new();
            let mut chars = o.chars();
            let first = chars.next().unwrap();
            ident_name.push(first.to_uppercase().next().unwrap_or(first));
            for c in chars {
                ident_name.push(c);
            }
            ident_name.to_string()
        }
    }
}

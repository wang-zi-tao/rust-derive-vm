#[cfg(test)]
#[macro_use]
extern crate lexical_derive;
#[cfg(test)]
mod tests {
    use lexical::Lexical;
    #[derive(Debug, PartialEq, Lexical)]
    enum LexicalImpl {
        #[lexical(word = "if")]
        If,
        #[lexical(word = "else")]
        Else,
        #[lexical(string = "=")]
        PutVariable,
        #[lexical(regex = r"\d+")]
        Int(usize),
        #[lexical(string = "+")]
        Add,
        #[lexical(string = "+=")]
        AddAssign,
        #[lexical(string = "-")]
        Sub,
        #[lexical(regex = r"\p{L}+")]
        Identify(String),
        #[lexical(indentation = "increase")]
        IndentationIncrease(usize, String),
        #[lexical(indentation = "decrease")]
        IndentationDecrease(usize, String),
        #[lexical(whitespace, ignore)]
        #[allow(dead_code)]
        Writespace(String),
        #[lexical(newline)]
        Newline,
    }
    #[allow(unused_imports)]
    pub use LexicalImpl::*;
    #[test]
    fn lexical_parse_0() {
        assert_eq!(&*LexicalImpl::parse("").unwrap(), &[]);
    }
    #[test]
    fn lexical_parse_1() {
        assert_eq!(&*LexicalImpl::parse("+").unwrap(), &[Add]);
    }
    #[test]
    fn lexical_parse_2() {
        assert_eq!(&*LexicalImpl::parse("+=").unwrap(), &[AddAssign]);
    }
    #[test]
    fn lexical_parse_3() {
        assert_eq!(&*LexicalImpl::parse("+=\n").unwrap(), &[AddAssign, Newline]);
    }
    #[test]
    fn lexical_parse_4() {
        assert_eq!(&*LexicalImpl::parse("+=\n -").unwrap(), &[AddAssign, Newline, IndentationIncrease(1, " ".to_string()), Sub]);
    }
    #[test]
    fn lexical_parse_5() {
        assert_eq!(&*LexicalImpl::parse("+=\n\n").unwrap(), &[AddAssign, Newline, Newline]);
    }
    #[test]
    fn lexical_parse_6() {
        assert_eq!(
            &*LexicalImpl::parse(
                "+=
                -
              +"
            )
            .unwrap(),
            &[AddAssign, Newline, IndentationIncrease(16, "                ".into()), Sub, Newline, IndentationDecrease(14, "              ".into()), Add]
        );
    }
    #[test]
    fn lexical_parse_7() {
        assert_eq!(&*LexicalImpl::parse("123456").unwrap(), &[Int(123456)]);
    }
    #[test]
    fn lexical_parse_8() {
        assert_eq!(&*LexicalImpl::parse("abc").unwrap(), &[Identify("abc".into())]);
    }
    #[test]
    fn lexical_parse_9() {
        assert_eq!(&*LexicalImpl::parse("abc中文 123").unwrap(), &[Identify("abc中文".into()), Int(123)]);
    }
    #[test]
    fn lexical_parse_10() {
        assert_eq!(&*LexicalImpl::parse("if").unwrap(), &[If]);
    }
    #[test]
    fn lexical_parse_11() {
        assert_eq!(&*LexicalImpl::parse("if 123").unwrap(), &[If, Int(123)]);
    }
    #[derive(Lexical)]
    pub enum PL0 {
        #[lexical(word = "begin")]
        Begin,
        #[lexical(word = "call")]
        Call,
        #[lexical(word = "const")]
        Const,
        #[lexical(word = "do")]
        Do,
        #[lexical(word = "end")]
        End,
        #[lexical(word = "if")]
        IF,
        #[lexical(word = "odd")]
        Odd,
        #[lexical(word = "procedure")]
        Procedure,
        #[lexical(word = "read")]
        Read,
        #[lexical(word = "then")]
        Then,
        #[lexical(word = "var")]
        Var,
        #[lexical(word = "while")]
        While,
        #[lexical(word = "write")]
        Write,
        #[lexical(regex = r"[a-zA-Z]\w{0,9}")]
        Ident(String),
        #[lexical(regex = r"[0-9]{0,14}")]
        Int(i64),
        #[lexical(string = "+")]
        Add,
        #[lexical(string = "+")]
        Sub,
        #[lexical(string = "*")]
        Mul,
        #[lexical(string = "/")]
        Div,
        #[lexical(string = ":=")]
        Assign,
        #[lexical(string = "<")]
        Less,
        #[lexical(string = "<=")]
        LessOrEqual,
        #[lexical(string = ">")]
        Large,
        #[lexical(string = ">=")]
        LargeOrEqual,
        #[lexical(string = "#")]
        Sharp,
    }
}

use logos::Logos;
use snafu::Snafu;
use std::num::ParseIntError;

#[derive(Snafu, Debug, PartialEq, Clone, Default)]
pub enum Error {
    #[default]
    #[snafu(display("unexpected token"))]
    UnexpectedToken,
    #[snafu(display("not an int: {source}"))]
    ParseInt { source: ParseIntError },
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Self::ParseInt { source: value }
    }
}

#[derive(Logos, Debug, PartialEq, Eq, Hash, Clone)]
#[logos(skip r"[ \t\f]+")]
#[logos(error = Error)]
pub(crate) enum Token {
    #[token("push")]
    Push,
    #[token("pop")]
    Pop,

    #[token("constant")]
    Constant,
    #[token("local")]
    Local,
    #[token("argument")]
    Argument,
    #[token("this")]
    This,
    #[token("that")]
    That,
    #[token("static")]
    Static,
    #[token("temp")]
    Temp,
    #[token("pointer")]
    Pointer,

    #[token("add")]
    Add,
    #[token("sub")]
    Subtract,
    #[token("neg")]
    Negate,
    #[token("eq")]
    Equal,
    #[token("gt")]
    Greater,
    #[token("lt")]
    Less,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("not")]
    Not,

    #[regex("[0-9]+", |lex| lex.slice().parse())]
    LitInt(u32),
    #[regex("\n")]
    Newline,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StackInstr {
    Push { segment: StackSegment, literal: u32 },
    Pop { segment: StackSegment, literal: u32 },
    Add,
    Subtract,
    Negate,
    Equal,
    Greater,
    Less,
    And,
    Or,
    Not,
}

impl StackInstr {
    pub(crate) fn push(segment: StackSegment, literal: u32) -> Self {
        Self::Push { segment, literal }
    }

    pub(crate) fn pop(segment: StackSegment, literal: u32) -> Self {
        Self::Pop { segment, literal }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StackSegment {
    Constant,
    Local,
    Argument,
    This,
    That,
    Static,
    Temp,
    Pointer,
}

#[cfg(test)]
mod tests {
    use crate::models::Error::ParseInt;
    use crate::models::Token;
    use logos::Logos;

    #[test]
    fn test_lit_not_int() {
        let testing = format!("push constant {}", u64::MAX);
        let mut lexer = Token::lexer(&testing);
        assert_eq!(lexer.next(), Some(Ok(Token::Push)));
        assert_eq!(lexer.next(), Some(Ok(Token::Constant)));
        assert!(matches!(lexer.next(), Some(Err(ParseInt { .. }))));
    }
}

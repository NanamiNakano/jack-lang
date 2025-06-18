use logos::Logos;
use snafu::Snafu;
use std::borrow::ToOwned;

#[derive(Snafu, Debug, PartialEq, Clone, Default)]
pub enum Error {
    #[default]
    #[snafu(display("unexpected token"))]
    UnexpectedToken,
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

    #[regex("[0-9]+", |lex| lex.slice().to_owned())]
    Literal(String),
    #[regex("\n")]
    Newline,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StackInstr {
    Push {
        segment: StackSegment,
        literal: String,
    },
    Pop {
        segment: StackSegment,
        literal: String,
    },
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
    pub(crate) fn push(segment: StackSegment, literal: &str) -> Self {
        Self::Push {
            segment,
            literal: literal.to_owned(),
        }
    }

    pub(crate) fn pop(segment: StackSegment, literal: &str) -> Self {
        Self::Pop {
            segment,
            literal: literal.to_owned(),
        }
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

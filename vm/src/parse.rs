use chumsky::input::Input;
use chumsky::IterParser;
use chumsky::error::Rich;
use chumsky::input::{Stream, ValueInput};
use chumsky::prelude::{SimpleSpan, choice, just};
use chumsky::{Parser, extra, select};
use logos::Logos;
use snafu::Snafu;
use std::num::ParseIntError;

#[derive(Snafu, Debug, PartialEq, Clone, Default)]
pub enum Error {
    #[default]
    #[snafu(display("unexpected token"))]
    UnexpectedToken,
    #[snafu(display("not an int"))]
    ParseInt { source: ParseIntError },
    #[snafu(display("syntax error"))]
    Syntax
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

pub(crate) fn parser<'tokens, I>() -> impl Parser<'tokens, I, Vec<StackInstr>, extra::Err<Rich<'tokens, Token>>>
where
    I: ValueInput<'tokens, Token = Token, Span = SimpleSpan>,
{
    let parse_segment = select! {
        Token::Constant => StackSegment::Constant,
        Token::Local => StackSegment::Local,
        Token::Argument => StackSegment::Argument,
        Token::This => StackSegment::This,
        Token::That => StackSegment::That,
        Token::Static => StackSegment::Static,
        Token::Temp => StackSegment::Temp,
        Token::Pointer => StackSegment::Pointer,
    };

    let parse_literal = select! {
        Token::LitInt(lit) => lit
    };

    choice((
        just(Token::Add).to(StackInstr::Add),
        just(Token::Subtract).to(StackInstr::Subtract),
        just(Token::Negate).to(StackInstr::Negate),
        just(Token::Equal).to(StackInstr::Equal),
        just(Token::Greater).to(StackInstr::Greater),
        just(Token::Less).to(StackInstr::Less),
        just(Token::And).to(StackInstr::And),
        just(Token::Or).to(StackInstr::Or),
        just(Token::Not).to(StackInstr::Not),
        just(Token::Push)
            .ignore_then(parse_segment)
            .then(parse_literal)
            .map(|(seg, lit)| StackInstr::push(seg, lit)),
        just(Token::Pop)
            .ignore_then(parse_segment)
            .then(parse_literal)
            .map(|(seg, lit)| StackInstr::pop(seg, lit)),
    ))
    .separated_by(just(Token::Newline))
    .allow_leading()
    .allow_trailing()
    .collect()
}

pub fn parse(input: impl AsRef<str>) -> Result<Vec<StackInstr>, Error> {
    let tokens = Token::lexer(input.as_ref())
        .spanned()
        .map(|(tok, span)| match tok {
            Ok(tok) => Ok((tok, span.into())),
            Err(source) => Err(source),
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let token_stream =
        Stream::from_iter(tokens).map((0..input.as_ref().len()).into(), |(t, s): (_, _)| (t, s));
    let result = parser().parse(token_stream);
    result.into_result().map_err(|_| Error::Syntax)
}

#[cfg(test)]
mod tests {
    use crate::parse::Error::ParseInt;
    use crate::parse::{parse, StackInstr, Token};
    use logos::Logos;
    use crate::parse::StackSegment::Constant;

    #[test]
    fn test_lit_not_int() {
        let testing = format!("push constant {}", u64::MAX);
        let mut lexer = Token::lexer(&testing);
        assert_eq!(lexer.next(), Some(Ok(Token::Push)));
        assert_eq!(lexer.next(), Some(Ok(Token::Constant)));
        assert!(matches!(lexer.next(), Some(Err(ParseInt { .. }))));
    }

    const TESTING_VM: &str = "push constant 1
    push constant 2
    add";
    #[test]
    fn test_parse() {
        let instr = parse(TESTING_VM).expect("expect ok");
        let parsed = vec![
            StackInstr::push(Constant, 1),
            StackInstr::push(Constant, 2),
            StackInstr::Add,
        ];
        assert_eq!(instr, parsed)
    }
}

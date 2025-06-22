use chumsky::error::Rich;
use chumsky::prelude::{choice, just};
use chumsky::{IterParser, extra};
use chumsky::{Parser, select};
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
    Syntax,
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

    #[token("function")]
    Function,
    #[token("call")]
    Call,
    #[token("return")]
    Return,

    #[regex("[0-9]+", |lex| lex.slice().parse())]
    LitInt(u32),
    #[regex("[a-zA-Z_.]+", |lex| lex.slice().to_owned())]
    Ident(String),
    #[token("\n")]
    Newline,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Instr {
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
    Call { name: String, args: u32 },
}

impl Instr {
    pub(crate) fn push(segment: StackSegment, literal: u32) -> Self {
        Self::Push { segment, literal }
    }

    pub(crate) fn pop(segment: StackSegment, literal: u32) -> Self {
        Self::Pop { segment, literal }
    }

    pub(crate) fn call(name: String, args: u32) -> Self {
        Self::Call { name, args }
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

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub(crate) instr: Vec<Instr>,
    pub(crate) name: String,
    pub(crate) args: u32,
}

impl Function {
    fn new(instr: Vec<Instr>, name: impl AsRef<str>, args: u32) -> Self {
        Self {
            instr,
            name: name.as_ref().to_owned(),
            args,
        }
    }
}

fn instr_parser<'tokens>()
-> impl Parser<'tokens, &'tokens [Token], Vec<Instr>, extra::Err<Rich<'tokens, Token>>> {
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

    let parse_ident = select! {
        Token::Ident(ident) => ident
    };

    choice((
        just(Token::Add).to(Instr::Add),
        just(Token::Subtract).to(Instr::Subtract),
        just(Token::Negate).to(Instr::Negate),
        just(Token::Equal).to(Instr::Equal),
        just(Token::Greater).to(Instr::Greater),
        just(Token::Less).to(Instr::Less),
        just(Token::And).to(Instr::And),
        just(Token::Or).to(Instr::Or),
        just(Token::Not).to(Instr::Not),
        just(Token::Push)
            .ignore_then(parse_segment)
            .then(parse_literal)
            .map(|(seg, lit)| Instr::push(seg, lit)),
        just(Token::Pop)
            .ignore_then(parse_segment)
            .then(parse_literal)
            .map(|(seg, lit)| Instr::pop(seg, lit)),
        just(Token::Call)
            .ignore_then(parse_ident)
            .then(parse_literal)
            .map(|(name, args)| Instr::call(name, args)),
    ))
    .separated_by(just(Token::Newline))
    .allow_leading()
    .allow_trailing()
    .collect()
}

fn parser<'tokens>()
-> impl Parser<'tokens, &'tokens [Token], Vec<Function>, extra::Err<Rich<'tokens, Token>>> { 
    let parse_literal = select! {
        Token::LitInt(lit) => lit
    };

    let parse_ident = select! {
        Token::Ident(ident) => ident
    };

    just(Token::Function)
            .ignore_then(parse_ident)
            .then(parse_literal)
            .then(instr_parser())
            .map(|((name, args), instr)| Function::new(instr, name, args))
            .then_ignore(just(Token::Return))
            .separated_by(just(Token::Newline))
            .allow_leading()
            .allow_trailing()
            .collect()
}

pub fn parse(input: impl AsRef<str>) -> Result<Vec<Function>, Error> {
    let tokens = Token::lexer(input.as_ref()).collect::<Result<Vec<_>, Error>>()?;
    let result = parser().parse(&tokens).into_result();
    result.map_err(|_| Error::Syntax)
}

pub fn parse_instr(input: impl AsRef<str>) -> Result<Vec<Instr>, Error> {
    let tokens = Token::lexer(input.as_ref()).collect::<Result<Vec<_>, Error>>()?;
    let result = instr_parser().parse(&tokens).into_result();
    result.map_err(|_| Error::Syntax)
}

#[cfg(test)]
mod tests {
    use crate::parse::Error::ParseInt;
    use crate::parse::StackSegment::Constant;
    use crate::parse::{Function, Instr, Token, parse, parse_instr};
    use logos::Logos;

    #[test]
    fn test_lit_not_int() {
        let testing = format!("push constant {}", u64::MAX);
        let mut lexer = Token::lexer(&testing);
        assert_eq!(lexer.next(), Some(Ok(Token::Push)));
        assert_eq!(lexer.next(), Some(Ok(Token::Constant)));
        assert!(matches!(lexer.next(), Some(Err(ParseInt { .. }))));
    }

    const TESTING_VM: &str = "function Test 0
    push constant 1
    push constant 2
    add
    return";
    #[test]
    fn test_parse_program() {
        let parsed = parse(TESTING_VM).expect("expect ok");
        let instr = vec![
            Instr::push(Constant, 1),
            Instr::push(Constant, 2),
            Instr::Add,
        ];
        let program = vec![Function::new(instr, "Test", 0)];
        assert_eq!(program, parsed)
    }

    const TESTING_VM_INSTR: &str = "push constant 1
    push constant 2
    add";
    #[test]
    fn test_parse_instr() {
        let parsed = parse_instr(TESTING_VM_INSTR).expect("expect ok");
        let instr = vec![
            Instr::push(Constant, 1),
            Instr::push(Constant, 2),
            Instr::Add,
        ];
        assert_eq!(instr, parsed)
    }
}

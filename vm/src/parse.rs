use chumsky::error::Rich;
use chumsky::prelude::{choice, just};
use chumsky::{IterParser, extra};
use chumsky::{Parser, select};
use derive_more::Display;
use logos::Logos;
use snafu::{ResultExt, Snafu};
use std::fmt::{Debug, Display, Formatter};
use std::num::ParseIntError;

#[derive(Snafu, Debug, PartialEq, Clone)]
pub enum Error {
    #[snafu(display("syntax error: {reasons}"))]
    Syntax { reasons: Reasons },
    #[snafu(display("error while lexing"))]
    Lexing { source: LexingError },
}

#[derive(Snafu, Debug, PartialEq, Clone, Default)]
pub enum LexingError {
    #[default]
    #[snafu(display("unexpected token"))]
    UnexpectedToken,
    #[snafu(display("not an int"))]
    ParseInt { source: ParseIntError },
}

#[derive(Debug, PartialEq, Clone)]
pub struct Reasons(Vec<String>);

impl Display for Reasons {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (index, reason) in self.0.iter().enumerate() {
            write!(f, "{index}: {reason}")?
        }
        Ok(())
    }
}

impl From<ParseIntError> for LexingError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseInt { source: value }
    }
}

#[derive(Logos, Debug, PartialEq, Eq, Hash, Clone, Display)]
#[logos(skip r"[ \t\f]+")]
#[logos(error = LexingError)]
pub(crate) enum Token {
    #[display("push")]
    #[token("push")]
    Push,
    #[display("pop")]
    #[token("pop")]
    Pop,

    #[display("constant")]
    #[token("constant")]
    Constant,
    #[display("local")]
    #[token("local")]
    Local,
    #[display("argument")]
    #[token("argument")]
    Argument,
    #[display("this")]
    #[token("this")]
    This,
    #[display("that")]
    #[token("that")]
    That,
    #[display("static")]
    #[token("static")]
    Static,
    #[display("temp")]
    #[token("temp")]
    Temp,
    #[display("pointer")]
    #[token("pointer")]
    Pointer,

    #[display("add")]
    #[token("add")]
    Add,
    #[display("sub")]
    #[token("sub")]
    Subtract,
    #[display("neg")]
    #[token("neg")]
    Negate,
    #[display("eq")]
    #[token("eq")]
    Equal,
    #[display("gt")]
    #[token("gt")]
    Greater,
    #[display("lt")]
    #[token("lt")]
    Less,
    #[display("and")]
    #[token("and")]
    And,
    #[display("or")]
    #[token("or")]
    Or,
    #[display("not")]
    #[token("not")]
    Not,

    #[display("function")]
    #[token("function")]
    Function,
    #[display("call")]
    #[token("call")]
    Call,
    #[display("return")]
    #[token("return")]
    Return,

    #[display("label")]
    #[token("label")]
    Label,
    #[display("goto")]
    #[token("goto")]
    Goto,
    #[display("if-goto")]
    #[token("if-goto")]
    CondGoto,

    #[regex("[0-9]+", |lex| lex.slice().parse())]
    LitInt(u32),
    #[regex("[a-zA-Z][a-zA-Z0-9_.]*", |lex| lex.slice().to_owned())]
    Ident(String),
    #[token("\n")]
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

#[derive(Clone, Debug, PartialEq)]
pub struct CallInstr {
    pub ident: String,
    pub args: u32,
}

impl CallInstr {
    pub fn new(ident: &str, args: u32) -> Self {
        Self {
            ident: ident.to_owned(),
            args,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BranchInstr {
    Label { ident: String },
    Goto { ident: String },
    CondGoto { ident: String },
}

impl BranchInstr {
    pub fn label(ident: &str) -> Self {
        Self::Label {
            ident: ident.to_owned(),
        }
    }
    pub fn goto(ident: &str) -> Self {
        Self::Goto {
            ident: ident.to_owned(),
        }
    }
    pub fn cond_goto(ident: &str) -> Self {
        Self::CondGoto {
            ident: ident.to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Instr {
    Stack { data: StackInstr },
    Call { data: CallInstr },
    Branch { data: BranchInstr },
}

impl From<StackInstr> for Instr {
    fn from(value: StackInstr) -> Self {
        Self::Stack { data: value }
    }
}

impl From<CallInstr> for Instr {
    fn from(value: CallInstr) -> Self {
        Self::Call { data: value }
    }
}

impl From<BranchInstr> for Instr {
    fn from(value: BranchInstr) -> Self {
        Self::Branch { data: value }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub(crate) instr: Vec<Instr>,
    pub(crate) name: String,
    pub(crate) vars: u32,
}

impl Function {
    pub fn new(instr: Vec<Instr>, name: &str, vars: u32) -> Self {
        Self {
            instr,
            name: name.to_owned(),
            vars,
        }
    }
}

fn stack_instr_parser<'tokens>()
-> impl Parser<'tokens, &'tokens [Token], StackInstr, extra::Err<Rich<'tokens, Token>>> {
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
}

fn branch_instr_parser<'tokens>()
-> impl Parser<'tokens, &'tokens [Token], BranchInstr, extra::Err<Rich<'tokens, Token>>> {
    let parse_ident = select! {
        Token::Ident(ident) => ident
    };

    choice((
        just(Token::Label)
            .ignore_then(parse_ident)
            .map(|ident| BranchInstr::label(&ident)),
        just(Token::Goto)
            .ignore_then(parse_ident)
            .map(|ident| BranchInstr::goto(&ident)),
        just(Token::CondGoto)
            .ignore_then(parse_ident)
            .map(|ident| BranchInstr::cond_goto(&ident)),
    ))
}

fn instr_parser<'tokens>()
-> impl Parser<'tokens, &'tokens [Token], Instr, extra::Err<Rich<'tokens, Token>>> {
    let parse_literal = select! {
        Token::LitInt(lit) => lit
    };

    let parse_ident = select! {
        Token::Ident(ident) => ident
    };

    choice((
        stack_instr_parser().map(|instr| instr.into()),
        just(Token::Call)
            .ignore_then(parse_ident)
            .then(parse_literal)
            .map(|(ident, args)| CallInstr::new(&ident, args).into()),
        branch_instr_parser().map(|instr| instr.into()),
    ))
}

fn parser<'tokens>()
-> impl Parser<'tokens, &'tokens [Token], Vec<Function>, extra::Err<Rich<'tokens, Token>>> {
    let parse_literal = select! {
        Token::LitInt(lit) => lit
    };

    let parse_ident = select! {
        Token::Ident(ident) => ident
    };

    let parse_instr = instr_parser()
        .separated_by(just(Token::Newline))
        .allow_leading()
        .allow_trailing()
        .collect();

    just(Token::Function)
        .ignore_then(parse_ident)
        .then(parse_literal)
        .then(parse_instr)
        .map(|((name, args), instr)| Function::new(instr, &name, args))
        .then_ignore(just(Token::Return))
        .separated_by(just(Token::Newline))
        .allow_leading()
        .allow_trailing()
        .collect()
}

pub fn parse(input: &str) -> Result<Vec<Function>, Error> {
    let tokens = Token::lexer(input)
        .collect::<Result<Vec<_>, _>>()
        .context(LexingSnafu)?;
    let result = parser().parse(&tokens).into_result();
    result.map_err(|errors| {
        let reasons = errors
            .clone()
            .iter()
            .map(|reason| reason.clone().into_reason().to_string())
            .collect::<Vec<_>>();
        Error::Syntax {
            reasons: Reasons(reasons),
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::parse::LexingError::ParseInt;
    use crate::parse::StackSegment::Constant;
    use crate::parse::{CallInstr, BranchInstr, Function, StackInstr, Token, parse};
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
    call Label 0
    return
    function Label 0
    label LABEL
    goto LABEL
    return";
    #[test]
    fn parse_program() {
        let parsed = parse(TESTING_VM).expect("expect ok");
        let test_instr = vec![
            StackInstr::push(Constant, 1).into(),
            StackInstr::push(Constant, 2).into(),
            StackInstr::Add.into(),
            CallInstr::new("Label", 0).into(),
        ];
        let label_instr = vec![
            BranchInstr::label("LABEL").into(),
            BranchInstr::goto("LABEL").into(),
        ];
        let program = vec![
            Function::new(test_instr, "Test", 0),
            Function::new(label_instr, "Label", 0),
        ];
        assert_eq!(program, parsed)
    }
}

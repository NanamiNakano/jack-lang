use chumsky::input::Input;
mod generate;
mod models;

use crate::Error::Parsing;
use crate::generate::Error as GeneratingError;
use crate::models::{Error as LexingError, StackInstr, StackSegment, Token};
use chumsky::error::Rich;
use chumsky::input::{Stream, ValueInput};
use chumsky::prelude::{choice, just};
use chumsky::span::SimpleSpan;
use chumsky::{IterParser, Parser};
use chumsky::{extra, select};
use logos::Logos;
use snafu::{ResultExt, Snafu};

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("error while lexing: {source}"))]
    Lexing { source: LexingError },
    #[snafu(display("error while parsing"))]
    Parsing,
    #[snafu(display("error while generating: {source}"))]
    Generating { source: GeneratingError },
}

fn parser<'tokens, I>() -> impl Parser<'tokens, I, Vec<StackInstr>, extra::Err<Rich<'tokens, Token>>>
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
            Err(source) => Err(Error::Lexing { source }),
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let token_stream =
        Stream::from_iter(tokens).map((0..input.as_ref().len()).into(), |(t, s): (_, _)| (t, s));
    let result = parser().parse(token_stream);
    result.into_result().map_err(|_| Parsing)
}

pub fn generate(instr: Vec<StackInstr>, scope: impl AsRef<str>) -> Result<String, Error> {
    instr
        .iter()
        .enumerate()
        .map(|(index, instr)| instr.generate(&scope, index))
        .collect::<Result<String, _>>()
        .context(GeneratingSnafu)
}

#[cfg(test)]
mod tests {
    use crate::models::StackInstr;
    use crate::models::StackSegment::Constant;
    use crate::{generate, parse};

    const TESTING_VM: &str = "push constant 1
    push constant 2
    add";
    const TESTING_ASM: &str = "@1\n\
    D=A\n\
    @SP\n\
    A=M\n\
    M=D\n\
    @SP\n\
    M=M+1\n\
    @2\n\
    D=A\n\
    @SP\n\
    A=M\n\
    M=D\n\
    @SP\n\
    M=M+1\n\
    @SP\n\
    AM=M-1\n\
    D=M\n\
    @SP\n\
    A=M-1\n\
    M=M+D\n";

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

    #[test]
    fn test_generate() {
        let instr = parse(TESTING_VM).expect("expect ok");
        let generated = generate(instr, "Test").expect("expect ok");
        assert_eq!(TESTING_ASM, generated)
    }
}

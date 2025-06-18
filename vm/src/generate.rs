use crate::models::{StackInstr, StackSegment};
use snafu::{ResultExt, Snafu};
use std::num::ParseIntError;
use crate::generate::Error::{SegmentOverflow, Syntax};

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("syntax error: {message}"))]
    Syntax { message: String },
    #[snafu(display("not an int: {source}"))]
    NotInt { source: ParseIntError },
    #[snafu(display("trying to access outside of a segment"))]
    SegmentOverflow
}

const PUSH_D: &'static str = "@SP\n\
    A=M\n\
    M=D\n\
    @SP\n\
    M=M+1\n";
const POP_TO_D: &'static str = "@SP\n\
    AM=M-1\n\
    D=M\n";
const LOAD_TOP: &'static str = "@SP\n\
    A=M-1\n";

impl StackSegment {
    fn generate_addr(&self, scope: &str, literal: &str) -> Result<String, Error> {
        match self {
            StackSegment::Constant => Err(Syntax { message: "constant has no address".to_owned() }),
            StackSegment::Local => Ok(format!("@LCL\n\
            D=M\n\
            @{literal}\n\
            A=D+A\n")),
            StackSegment::Argument => Ok(format!("@ARG\n\
            D=M\n\
            @{literal}\n\
            A=D+A\n")),
            StackSegment::This => Ok(format!("@THIS\n\
            D=M\n\
            @{literal}\n\
            A=D+A\n")),
            StackSegment::That => Ok(format!("@THAT\n\
            D=M\n\
            @{literal}\n\
            A=D+A\n")),
            StackSegment::Static => Ok(format!("@{scope}.{literal}\n")),
            StackSegment::Temp => {
                let index = literal.parse::<u32>().context(NotIntSnafu)?;
                if index > 7 {
                    Err(SegmentOverflow)
                } else { 
                    Ok(format!("@{}\n", 5 + index))
                }
            },
            StackSegment::Pointer => {
                match literal { 
                    "0" => Ok(String::from("@THIS\n")),
                    "1" => Ok(String::from("@THAT\n")),
                    _ => Err(Syntax { message: "no such pointer".to_owned() })
                }
            }
        }
    }
    
    fn generate_load_to_d(&self, scope: &str, literal: &str) -> Result<String, Error> {
        match self {
            StackSegment::Constant => Ok(format!("@{literal}\n\
            D=A\n")),
            _ => Ok(format!("{}\
            D=M\n", self.generate_addr(scope, literal)?)),
        }
    }
}

impl StackInstr {
    pub fn generate(&self, scope: &str, count: usize) -> Result<String, Error> {
        let label = format!("{scope}.{count}");
        match self {
            Self::Push { segment, literal } => {
                let load = segment.generate_load_to_d(scope, literal)?;
                Ok(format!("{load}{PUSH_D}"))
            },
            Self::Pop { segment, literal } => {
                let addr = segment.generate_addr(scope, literal)?;
                Ok(format!("{POP_TO_D}{addr}\
                M=D\n"))
            },
            Self::Add => Ok(format!(
                "{POP_TO_D}{LOAD_TOP}\
            M=M+D\n")),
            Self::Subtract => Ok(format!(
                "{POP_TO_D}{LOAD_TOP}\
            M=M-D\n")),
            Self::Negate => Ok(format!(
                "{LOAD_TOP}\
            M=-M\n")),
            Self::Equal => Ok(format!(
                "{POP_TO_D}{LOAD_TOP}\
            D=M-D\n\
            @TRUE.{label}\n\
            D;JEQ\n\
            {LOAD_TOP}\
            M=-1\n\
            @END.{label}\n\
            0;JMP\n\
            (TRUE.{label})\n\
            {LOAD_TOP}\
            M=0\n\
            (END.{label})\n")),
            Self::Greater => Ok(format!(
                "{POP_TO_D}{LOAD_TOP}\
            D=M-D\n\
            @TRUE.{label}\n\
            D;JGE\n\
            {LOAD_TOP}\
            M=-1\n\
            @END.{label}\n\
            0;JMP\n\
            (TRUE.{label})\n\
            {LOAD_TOP}\
            M=0\n\
            (END.{label})\n")),
            Self::Less => Ok(format!(
                "{POP_TO_D}{LOAD_TOP}\
            D=M-D\n\
            @TRUE.{label}\n\
            D;JLE\n\
            {LOAD_TOP}\
            M=-1\n\
            @END.{label}\n\
            0;JMP\n\
            (TRUE.{label})\n\
            {LOAD_TOP}\
            M=0\n\
            (END.{label})\n")),
            Self::And => Ok(format!(
                "{POP_TO_D}{LOAD_TOP}\
            M=M&D\n")),
            Self::Or => Ok(format!(
                "{POP_TO_D}{LOAD_TOP}\
            M=M|D\n",)),
            Self::Not => Ok(format!(
                "{POP_TO_D}{LOAD_TOP}\
            M=!M\n")),
        }
    }
}

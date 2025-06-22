use crate::generate::Error::{SegmentOverflow, Syntax};
use crate::parse::{StackInstr, StackSegment};
use snafu::Snafu;

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("syntax error: {message}"))]
    Syntax { message: String },
    #[snafu(display("trying to access outside of a segment"))]
    SegmentOverflow,
}

const PUSH_D: &'static str = "@SP\n\
    A=M\n\
    M=D\n\
    @SP\n\
    M=M+1\n";
const POP_TO_D: &'static str = "@SP\n\
    AM=M-1\n\
    D=M\n";
const LOAD_TOP_TO_M: &'static str = "@SP\n\
    A=M-1\n";

impl StackSegment {
    fn generate_addr(&self, scope: impl AsRef<str>, literal: &u32) -> Result<String, Error> {
        let scope = scope.as_ref();
        match self {
            StackSegment::Constant => Err(Syntax {
                message: "constant has no address".to_owned(),
            }),
            StackSegment::Local => Ok(format!(
                "@LCL\n\
            D=M\n\
            @{literal}\n\
            A=D+A\n"
            )),
            StackSegment::Argument => Ok(format!(
                "@ARG\n\
            D=M\n\
            @{literal}\n\
            A=D+A\n"
            )),
            StackSegment::This => Ok(format!(
                "@THIS\n\
            D=M\n\
            @{literal}\n\
            A=D+A\n"
            )),
            StackSegment::That => Ok(format!(
                "@THAT\n\
            D=M\n\
            @{literal}\n\
            A=D+A\n"
            )),
            StackSegment::Static => Ok(format!("@{scope}.{literal}\n")),
            StackSegment::Temp => {
                let index = literal;
                if *index > 7 {
                    Err(SegmentOverflow)
                } else {
                    Ok(format!("@{}\n", 5 + index))
                }
            }
            StackSegment::Pointer => match literal {
                0 => Ok(String::from("@THIS\n")),
                1 => Ok(String::from("@THAT\n")),
                _ => Err(Syntax {
                    message: "no such pointer".to_owned(),
                }),
            },
        }
    }

    fn generate_load_to_d(
        &self,
        scope: impl AsRef<str>,
        literal: &u32,
    ) -> Result<String, Error> {
        match self {
            StackSegment::Constant => Ok(format!(
                "@{literal}\n\
            D=A\n"
            )),
            _ => Ok(format!(
                "{}\
            D=M\n",
                self.generate_addr(scope, literal)?
            )),
        }
    }
}

impl Instr {
    pub fn generate(
        &self,
        scope: impl AsRef<str>,
        count: usize,
    ) -> Result<String, Error> {
        let boolean_label = format!("{}.{count}", scope.as_ref());
        let return_label = format!("{}$ret.{count}", scope.as_ref());
        match self {
            Self::Push { segment, literal } => {
                let load = segment.generate_load_to_d(scope, literal)?;
                Ok(format!("{load}{PUSH_D}"))
            }
            Self::Pop { segment, literal } => {
                let addr = segment.generate_addr(scope, literal)?;
                Ok(format!(
                    "{POP_TO_D}{addr}\
                M=D\n"
                ))
            }
            Self::Add => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            M=D+M\n"
            )),
            Self::Subtract => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            M=M-D\n"
            )),
            Self::Negate => Ok(format!(
                "{LOAD_TOP_TO_M}\
            M=-M\n"
            )),
            Self::Equal => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            D=M-D\n\
            @TRUE.{boolean_label}\n\
            D;JEQ\n\
            {LOAD_TOP_TO_M}\
            M=-1\n\
            @END.{boolean_label}\n\
            0;JMP\n\
            (TRUE.{boolean_label})\n\
            {LOAD_TOP_TO_M}\
            M=0\n\
            (END.{boolean_label})\n"
            )),
            Self::Greater => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            D=M-D\n\
            @TRUE.{boolean_label}\n\
            D;JGT\n\
            {LOAD_TOP_TO_M}\
            M=-1\n\
            @END.{boolean_label}\n\
            0;JMP\n\
            (TRUE.{boolean_label})\n\
            {LOAD_TOP_TO_M}\
            M=0\n\
            (END.{boolean_label})\n"
            )),
            Self::Less => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            D=M-D\n\
            @TRUE.{boolean_label}\n\
            D;JLT\n\
            {LOAD_TOP_TO_M}\
            M=-1\n\
            @END.{boolean_label}\n\
            0;JMP\n\
            (TRUE.{boolean_label})\n\
            {LOAD_TOP_TO_M}\
            M=0\n\
            (END.{boolean_label})\n"
            )),
            Self::And => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            M=M&D\n"
            )),
            Self::Or => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            M=M|D\n",
            )),
            Self::Not => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            M=!M\n"
            )),
        }
    }
}

pub fn generate(instr: Vec<StackInstr>, scope: impl AsRef<str>) -> Result<String, Error> {
    instr
        .iter()
        .enumerate()
        .map(|(index, instr)| instr.generate(&scope, index))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::generate::generate;
    use crate::parse::parse;

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
    fn test_generate() {
        let instr = parse(TESTING_VM).expect("expect ok");
        let generated = generate(instr, "Test").expect("expect ok");
        assert_eq!(TESTING_ASM, generated)
    }
}

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

trait Generate {
    type Error;
    fn generate(&self) -> Result<String, Self::Error>;
}

impl<T: Generate> Generate for Vec<T> {
    type Error = <T as Generate>::Error;

    fn generate(&self) -> Result<String, Self::Error> {
        self.iter().map(|item| item.generate()).collect()
    }
}

pub struct Scoped<T: ?Sized> {
    scope: String,
    value: T,
}

impl<T> Scoped<T> {
    pub fn new(value: T, scope: &str) -> Self {
        Self {
            scope: scope.to_owned(),
            value,
        }
    }
}

impl StackSegment {
    fn generate_addr(&self, scope: &str, literal: &u32) -> Result<String, Error> {
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

    fn generate_load_to_d(&self, scope: &str, literal: &u32) -> Result<String, Error> {
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

impl Generate for Scoped<StackInstr> {
    type Error = Error;
    fn generate(&self) -> Result<String, Self::Error> {
        let boolean_label = &self.scope;
        match &self.value {
            StackInstr::Push { segment, literal } => {
                let load = segment.generate_load_to_d(&self.scope, literal)?;
                Ok(format!("{load}{PUSH_D}"))
            }
            StackInstr::Pop { segment, literal } => {
                let addr = segment.generate_addr(&self.scope, literal)?;
                Ok(format!(
                    "{POP_TO_D}{addr}\
                M=D\n"
                ))
            }
            StackInstr::Add => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            M=D+M\n"
            )),
            StackInstr::Subtract => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            M=M-D\n"
            )),
            StackInstr::Negate => Ok(format!(
                "{LOAD_TOP_TO_M}\
            M=-M\n"
            )),
            StackInstr::Equal => Ok(format!(
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
            StackInstr::Greater => Ok(format!(
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
            StackInstr::Less => Ok(format!(
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
            StackInstr::And => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            M=M&D\n"
            )),
            StackInstr::Or => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            M=M|D\n",
            )),
            StackInstr::Not => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            M=!M\n"
            )),
        }
    }
}

impl StackInstr {
    pub fn scoped(self, scope: &str) -> Scoped<Self> {
        Scoped::new(self, scope)
    }
}

#[cfg(test)]
mod tests {
    use crate::generate::Generate;
    use crate::parse::StackInstr;
    use crate::parse::StackSegment::Constant;

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
    M=D+M\n";

    #[test]
    fn test_generate() {
        let instr = vec![
            StackInstr::push(Constant, 1).scoped("test"),
            StackInstr::push(Constant, 2).scoped("test"),
            StackInstr::Add.scoped("test"),
        ];
        let generated = instr.generate().expect("expect ok");
        assert_eq!(TESTING_ASM, generated)
    }
}

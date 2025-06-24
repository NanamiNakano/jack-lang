use crate::generate::Error::{SegmentOverflow, Syntax};
use crate::parse::{CallInstr, StackInstr, StackSegment};
use crate::scoped::Scoped;
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

trait ScopedGenerate {
    type Error;
    fn scoped_generate(&self, scope: &str) -> Result<String, Self::Error>;
}

impl<T: ScopedGenerate> ScopedGenerate for Vec<T> {
    type Error = <T as ScopedGenerate>::Error;

    fn scoped_generate(&self, scope: &str) -> Result<String, Self::Error> {
        self.iter()
            .map(|item| item.scoped_generate(scope))
            .collect()
    }
}

impl<T: ScopedGenerate> Generate for Scoped<T> {
    type Error = <T as ScopedGenerate>::Error;

    fn generate(&self) -> Result<String, Self::Error> {
        self.value.scoped_generate(&self.scope)
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

impl ScopedGenerate for StackInstr {
    type Error = Error;
    fn scoped_generate(&self, scope: &str) -> Result<String, Self::Error> {
        match &self {
            StackInstr::Push { segment, literal } => {
                let load = segment.generate_load_to_d(scope, literal)?;
                Ok(format!("{load}{PUSH_D}"))
            }
            StackInstr::Pop { segment, literal } => {
                let addr = segment.generate_addr(scope, literal)?;
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
            @TRUE.{scope}\n\
            D;JEQ\n\
            {LOAD_TOP_TO_M}\
            M=-1\n\
            @END.{scope}\n\
            0;JMP\n\
            (TRUE.{scope})\n\
            {LOAD_TOP_TO_M}\
            M=0\n\
            (END.{scope})\n"
            )),
            StackInstr::Greater => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            D=M-D\n\
            @TRUE.{scope}\n\
            D;JGT\n\
            {LOAD_TOP_TO_M}\
            M=-1\n\
            @END.{scope}\n\
            0;JMP\n\
            (TRUE.{scope})\n\
            {LOAD_TOP_TO_M}\
            M=0\n\
            (END.{scope})\n"
            )),
            StackInstr::Less => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
            D=M-D\n\
            @TRUE.{scope}\n\
            D;JLT\n\
            {LOAD_TOP_TO_M}\
            M=-1\n\
            @END.{scope}\n\
            0;JMP\n\
            (TRUE.{scope})\n\
            {LOAD_TOP_TO_M}\
            M=0\n\
            (END.{scope})\n"
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

impl ScopedGenerate for CallInstr {
    type Error = Error;

    fn scoped_generate(&self, scope: &str) -> Result<String, Self::Error> {
        let arg_offset = 5 + self.args;
        let callee = &self.ident;
        Ok(format!(
            "@{scope}\n\
        D=A\n\
        {PUSH_D}\
        @LCL\n\
        D=M\n\
        {PUSH_D}\
        @ARG\n\
        D=M\n\
        {PUSH_D}\
        @THIS\n\
        D=M\n\
        {PUSH_D}\
        @THAT\n\
        D=M\n\
        {PUSH_D}\
        @SP\n\
        D=M\n\
        @{arg_offset}\n\
        D=D-A\n\
        @LCL\n\
        M=D\n\
        @{callee}\n\
        0;JMP\n\
        ({scope})\n"
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::generate::ScopedGenerate;
    use crate::parse::StackSegment::Constant;
    use crate::parse::{CallInstr, StackInstr};

    const TEST_STACK_INSTR: &str = "@1\n\
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
    fn generate_stack_instr() {
        let instr = vec![
            StackInstr::push(Constant, 1),
            StackInstr::push(Constant, 2),
            StackInstr::Add,
        ];
        let generated = instr.scoped_generate("test").expect("expect ok");
        assert_eq!(TEST_STACK_INSTR, generated)
    }

    const TEST_CALL_INSTR: &str = "@Test.test$ret.0\n\
    D=A\n\
    @SP\n\
    A=M\n\
    M=D\n\
    @SP\n\
    M=M+1\n\
    @LCL\n\
    D=M\n\
    @SP\n\
    A=M\n\
    M=D\n\
    @SP\n\
    M=M+1\n\
    @ARG\n\
    D=M\n\
    @SP\n\
    A=M\n\
    M=D\n\
    @SP\n\
    M=M+1\n\
    @THIS\n\
    D=M\n\
    @SP\n\
    A=M\n\
    M=D\n\
    @SP\n\
    M=M+1\n\
    @THAT\n\
    D=M\n\
    @SP\n\
    A=M\n\
    M=D\n\
    @SP\n\
    M=M+1\n\
    @SP\n\
    D=M\n\
    @5\n\
    D=D-A\n\
    @LCL\n\
    M=D\n\
    @Callee\n\
    0;JMP\n\
    (Test.test$ret.0)\n";
    #[test]
    fn generate_call_instr() {
        let instr = CallInstr::new("Callee", 0);
        let generated = instr.scoped_generate("Test.test$ret.0").expect("expect ok");
        assert_eq!(TEST_CALL_INSTR, generated)
    }
}

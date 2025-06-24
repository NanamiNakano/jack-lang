use crate::generate::Error::{SegmentOverflow, Syntax};
use crate::parse::{BranchInstr, CallInstr, Function, Instr, StackInstr, StackSegment};
use crate::scoped::{Scoped, ToScoped};
use snafu::Snafu;

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("syntax error: {message}"))]
    Syntax { message: String },
    #[snafu(display("trying to access outside of a segment"))]
    SegmentOverflow,
}

const PUSH_D: &str = "@SP\n\
    A=M\n\
    M=D\n\
    @SP\n\
    M=M+1\n";
const POP_TO_D: &str = "@SP\n\
    AM=M-1\n\
    D=M\n";
const LOAD_TOP_TO_M: &str = "@SP\n\
    A=M-1\n";

pub trait Generate {
    type Error;
    fn generate(&self) -> Result<String, Self::Error>;
}

impl<T: Generate> Generate for Vec<T> {
    type Error = <T as Generate>::Error;

    fn generate(&self) -> Result<String, Self::Error> {
        self.iter().map(|item| item.generate()).collect()
    }
}

pub trait ScopedGenerate {
    type Error;
    fn scoped_generate(&self, scope: &str) -> Result<String, Self::Error>;
}

impl<T: ScopedGenerate + Clone> Generate for Scoped<T> {
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
                    "{addr}\
                    D=A\n\
                    @R15\n\
                    M=D\n\
                    {POP_TO_D}\
                    @R15\n\
                    A=M\n\
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
                M=0\n\
                @END.{scope}\n\
                0;JMP\n\
                (TRUE.{scope})\n\
                {LOAD_TOP_TO_M}\
                M=-1\n\
                (END.{scope})\n"
            )),
            StackInstr::Greater => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
                D=M-D\n\
                @TRUE.{scope}\n\
                D;JGT\n\
                {LOAD_TOP_TO_M}\
                M=0\n\
                @END.{scope}\n\
                0;JMP\n\
                (TRUE.{scope})\n\
                {LOAD_TOP_TO_M}\
                M=-1\n\
                (END.{scope})\n"
            )),
            StackInstr::Less => Ok(format!(
                "{POP_TO_D}{LOAD_TOP_TO_M}\
                D=M-D\n\
                @TRUE.{scope}\n\
                D;JLT\n\
                {LOAD_TOP_TO_M}\
                M=0\n\
                @END.{scope}\n\
                0;JMP\n\
                (TRUE.{scope})\n\
                {LOAD_TOP_TO_M}\
                M=-1\n\
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
            @ARG\n\
            M=D\n\
            @SP\n\
            D=M\n\
            @LCL\n\
            M=D\n\
            @{callee}\n\
            0;JMP\n\
            ({scope})\n"
        ))
    }
}

impl ScopedGenerate for BranchInstr {
    type Error = Error;

    fn scoped_generate(&self, scope: &str) -> Result<String, Self::Error> {
        match self {
            BranchInstr::Label { ident } => Ok(format!("({scope}.{ident})\n")),
            BranchInstr::Goto { ident } => Ok(format!(
                "@{scope}.{ident}\n\
                0;JMP\n"
            )),
            BranchInstr::CondGoto { ident } => Ok(format!(
                "{LOAD_TOP_TO_M}\
                D=M\n\
                @{scope}.{ident}\n\
                D;JLT"
            )),
        }
    }
}

impl ScopedGenerate for Function {
    type Error = Error;

    fn scoped_generate(&self, scope: &str) -> Result<String, Self::Error> {
        let fn_scope = &self.name;
        let body = self
            .instr
            .iter()
            .enumerate()
            .map(|(index, item)| match item {
                Instr::Stack { data } => {
                    match data {
                        StackInstr::Push { segment: StackSegment::Static, .. } => data.scoped_generate(scope),
                        StackInstr::Pop { segment: StackSegment::Static, .. } => data.scoped_generate(scope),
                        _ => data.scoped_generate(&format!("{fn_scope}.{index}"))
                    }
                },
                Instr::Call { data } => data.scoped_generate(&format!("{scope}$ret.{index}")),
                Instr::Branch { data } => data.scoped_generate(scope),
            })
            .collect::<Result<String, _>>()?;
        let init_local_vars =
            vec![StackInstr::push(StackSegment::Constant, 0).to_scoped(scope); self.vars as usize]
                .generate()?;
        Ok(format!(
            "({fn_scope})\n\
            {init_local_vars}\
            {body}\
            @5\n\
            D=A\n\
            @LCL\n\
            A=M-D\n\
            D=M\n\
            @R14\n\
            M=D\n\
            {LOAD_TOP_TO_M}\
            D=M\n\
            @ARG\n\
            A=M\n\
            M=D\n\
            D=A+1\n\
            @SP\n\
            M=D\n\
            @LCL\n\
            AM=M-1\n\
            D=M\n\
            @THAT\n\
            M=D\n\
            @LCL\n\
            AM=M-1\n\
            D=M\n\
            @THIS\n\
            M=D\n\
            @LCL\n\
            AM=M-1\n\
            D=M\n\
            @ARG\n\
            M=D\n\
            @LCL\n\
            A=M-1\n\
            D=M\n\
            @LCL\n\
            M=D\n\
            @R14\n\
            A=M\n\
            0;JMP\n"
        ))
    }
}

pub struct Class {
    pub functions: Vec<Function>,
    pub name: String,
}

impl Class {
    pub fn new(functions: Vec<Function>, name: &str) -> Self {
        Self {
            functions,
            name: name.to_owned()
        }
    }
}

impl Generate for Class {
    type Error = Error;

    fn generate(&self) -> Result<String, Self::Error> {
        self.functions.iter().map(|fun| fun.scoped_generate(&self.name)).collect()
    }
}

pub const BOOTSTRAP: &'static str = "@256\n\
    D=A\n\
    @SP\n\
    M=D\n\
    @Sys.init\n\
    0;JMP\n";

#[cfg(test)]
mod tests {
    use crate::generate::{Generate, ScopedGenerate};
    use crate::parse::StackSegment::Constant;
    use crate::parse::{BranchInstr, CallInstr, Function, StackInstr};
    use crate::scoped::ToScoped;

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
    M=D+M\n\
    @3\n\
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
    D=M-D\n\
    @TRUE.Test.test.4\n\
    D;JEQ\n\
    @SP\n\
    A=M-1\n\
    M=0\n\
    @END.Test.test.4\n\
    0;JMP\n\
    (TRUE.Test.test.4)\n\
    @SP\n\
    A=M-1\n\
    M=-1\n\
    (END.Test.test.4)\n";
    #[test]
    fn generate_stack_instr() {
        let instr = vec![
            StackInstr::push(Constant, 1).to_scoped("Test.test.0"),
            StackInstr::push(Constant, 2).to_scoped("Test.test.1"),
            StackInstr::Add.to_scoped("Test.test.2"),
            StackInstr::push(Constant, 3).to_scoped("Test.test.3"),
            StackInstr::Equal.to_scoped("Test.test.4"),
        ];
        let generated = instr.generate().expect("expect ok");
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

    const TEST_BRANCH_INSTR: &str = "(Test.test.Test)\n\
    @Test.test.Test\n\
    0;JMP\n";
    #[test]
    fn generate_branch_instr() {
        let instr = vec![
            BranchInstr::label("Test").to_scoped("Test.test"),
            BranchInstr::goto("Test").to_scoped("Test.test"),
        ];
        let generated = instr.generate().expect("expect ok");
        assert_eq!(TEST_BRANCH_INSTR, generated)
    }
    
    const TEST_FUNCTION: &str = "(Test.test)\n\
    @0\n\
    D=A\n\
    @SP\n\
    A=M\n\
    M=D\n\
    @SP\n\
    M=M+1\n\
    @SP\n\
    A=M-1\n\
    D=M\n\
    @ARG\n\
    A=M\n\
    M=D\n\
    D=A+1\n\
    @SP\n\
    M=D\n\
    @LCL\n\
    AM=M-1\n\
    D=M\n\
    @THAT\n\
    M=D\n\
    @LCL\n\
    AM=M-1\n\
    D=M\n\
    @THIS\n\
    M=D\n\
    @LCL\n\
    AM=M-1\n\
    D=M\n\
    @ARG\n\
    M=D\n\
    @2\n\
    D=A\n\
    @LCL\n\
    A=M-D\n\
    D=M\n\
    @R14\n\
    M=D\n\
    @LCL\n\
    A=M-1\n\
    D=M\n\
    @LCL\n\
    M=D\n\
    @R14\n\
    0;JMP\n";
    #[test]
    fn generate_function() {
        let instr = vec![
            StackInstr::push(Constant, 0).into()
        ];
        let function = Function::new(instr, "Test.test", 0);
        let generated = function.scoped_generate("Test").expect("expect ok");
        assert_eq!(TEST_FUNCTION, generated)
    }
}

use crate::Error::NotAFile;
use clap::Parser;
use clio::{Input, Output};
use snafu::{ResultExt, Snafu};
use std::io;
use std::io::{Read, Write};
use vm;

#[derive(Snafu, Debug)]
enum Error {
    #[snafu(display("io error"))]
    IO {
        source: io::Error
    },
    #[snafu(display("error when parsing"))]
    Parsing {
        source: vm::parse::Error
    },
    #[snafu(display("error when generating"))]
    Generating {
        source: vm::generate::Error
    },
    #[snafu(display("no such file: {path}"))]
    NoSuchFile {
        path: String
    },
    #[snafu(display("input is not a file"))]
    NotAFile
}
#[derive(Parser)]
struct Opts {
    #[clap(long, short, value_parser, default_value="-")]
    input: Input,
    #[clap(long, short, value_parser, default_value="-")]
    output: Output,
}

#[snafu::report]
fn main() -> Result<(), Error> {
    let mut opt = Opts::parse();

    let mut input = String::new();
    opt.input.read_to_string(&mut input).context(IOSnafu)?;
    let file_name = if opt.input.is_local() {
        opt.input.path().file_name().ok_or(NotAFile)?.to_string_lossy().to_string()
    } else {
        String::from("IO")
    };
    let parsed_program = vm::parse(input).context(ParsingSnafu)?;
    let generated = vm::generate(parsed_program, file_name).context(GeneratingSnafu)?;
    opt.output.write_all(generated.as_bytes()).context(IOSnafu)
}

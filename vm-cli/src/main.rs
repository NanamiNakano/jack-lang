use crate::Error::{EmptySource, Whatever};
use clap::Parser;
use clio::{has_extension, ClioPath};
use snafu::{ResultExt, Snafu};
use std::env::temp_dir;
use std::fs::File;
use std::io::{copy, read_to_string, BufReader, BufWriter, Write};
use std::path::Path;
use std::{fs, io};
use vm::generate::{bootstrap, Class, Generate};
use vm::parse::parse;

#[derive(Snafu, Debug)]
enum Error {
    #[snafu(display("io error"))]
    IO { source: io::Error },
    #[snafu(display("input is empty: {message}"))]
    EmptySource { message: String },
    #[snafu(display("error when parsing {path}"))]
    Parsing { source: vm::parse::Error, path: String },
    #[snafu(display("error when generating"))]
    Generating { source: vm::generate::Error },
    #[snafu(whatever)]
    Whatever {
        message: String
    }
}

impl From<clio::Error> for Error {
    fn from(value: clio::Error) -> Self {
        let clio::Error::Io(error) = value;
        Error::IO { source: error }
    }
}

#[derive(Parser)]
struct Opts {
    #[clap(long, short, value_parser = clap::value_parser!(ClioPath).exists(), default_value=".")]
    input: ClioPath,
    #[clap(long, short, value_parser = clap::value_parser!(ClioPath).is_file(), default_value="./out.asm")]
    output: ClioPath,
    #[clap(long, action, default_value_t = false)]
    no_boot: bool,
}

#[snafu::report]
fn main() -> Result<(), Error> {
    let opt = Opts::parse();


    let temp = temp_dir().canonicalize().context(IOSnafu)?;
    let temp = temp.join("jack-vm");
    if temp.exists() { 
        fs::remove_dir_all(&temp).context(IOSnafu)?;
    }
    fs::create_dir(&temp).context(IOSnafu)?;
    compile(opt.input, temp.as_path())?;
    link(temp.as_path(), opt.output.path(), !opt.no_boot)
}

fn compile(input_path: ClioPath, out_path: &Path) -> Result<(), Error> {
    if input_path.is_dir() {
        let vm_files = input_path
            .files(has_extension("vm"))?;
        if vm_files.is_empty() {
            return Err(EmptySource {
                message: "directory does not contain any vm file".to_owned(),
            });
        }
        for file_path in vm_files {
            let file_name = file_path.file_stem().expect("expect file name").to_owned();
            let path = file_path.to_string();

            let cached = file_path.read_all()?;
            let input = read_to_string(cached).context(IOSnafu)?;
            let parsed_fn = parse(&input).context(ParsingSnafu { path })?;
            let class = Class::new(parsed_fn, file_name.to_str().ok_or(Whatever { message: "invalid file name".to_owned() })?);
            let generated = class.generate().context(GeneratingSnafu)?;

            let out_file_path = out_path.join(file_name).with_extension("asm");
            let mut out_file = File::create(out_file_path).context(IOSnafu)?;
            out_file.write(generated.as_bytes()).context(IOSnafu)?;
        }
        return Ok(());
    }
    if input_path.is_file() {
        let file_name = input_path.file_name().expect("expect file name").to_owned();
        let path = input_path.to_string();

        let cached = input_path.read_all()?;
        let input = read_to_string(cached).context(IOSnafu)?;
        let parsed_fn = parse(&input).context(ParsingSnafu { path })?;
        let class = Class::new(parsed_fn, file_name.to_str().ok_or(Whatever { message: "invalid file name".to_owned() })?);
        let generated = class.generate().context(GeneratingSnafu)?;

        let out_file_path = out_path.join(file_name).with_extension("asm");
        let mut out_file = File::create(out_file_path).context(IOSnafu)?;
        out_file.write(generated.as_bytes()).context(IOSnafu)?;
    }
    Err(EmptySource {
        message: "invalid input".to_owned(),
    })
}

fn link(path: &Path, out_path: &Path, boot: bool) -> Result<(), Error> {
    let read_dir = path.read_dir().context(IOSnafu)?;
    let mut asm_files = vec![];
    for entry in read_dir {
        let entry = entry.context(IOSnafu)?.path();
        if entry.is_dir() {
            continue
        }
        let Some(ext) = entry.extension() else {
            continue
        };
        if ext != "asm" {
            continue
        }
        asm_files.push(entry)
    }
    if asm_files.is_empty() {
        return Err(EmptySource { message: "directory does not contain any asm file".to_owned() })
    }
    
    let out_file = File::create(out_path).context(IOSnafu)?;
    let mut writer = BufWriter::new(out_file);
    if boot {
        writer.write(bootstrap().as_bytes()).context(IOSnafu)?;
    }
    for file_path in asm_files {
        let file = File::open(file_path).context(IOSnafu)?;
        let mut reader = BufReader::new(file);
        copy(&mut reader, &mut writer).context(IOSnafu)?;
    }
    Ok(())
}

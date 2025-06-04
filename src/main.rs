use clap::Parser;
use error::CompilerError;
use std::path::PathBuf;

use crate::driver::CompilerDriver;

mod codegen;
mod driver;
mod error;
mod lexer;
mod parser;
mod types;

#[derive(Parser)]
#[command(name = "ccompiler")]
struct Args {
    /// Input C source file
    input: PathBuf,

    /// Stop after lexing
    #[arg(long)]
    lex: bool,

    /// Stop after parsing
    #[arg(long)]
    parse: bool,

    /// Stop after code generation
    #[arg(long)]
    codegen: bool,
}

// --- Main Function ---

fn main() -> Result<(), CompilerError> {
    let args = Args::parse();
    CompilerDriver::run(&args)?;
    std::process::exit(0);
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::path::Path;
    const TEST_FILE: &str = "./target/debug/hello.c"; // Path to the test file

    fn test_args(lex: bool, parse: bool, codegen: bool) -> Args {
        Args {
            input: Path::new(TEST_FILE).to_path_buf(),
            lex,
            parse,
            codegen,
        }
    }
    #[test]
    fn test_all() -> Result<(), CompilerError> {
        CompilerDriver::run(&test_args(false, false, true))
    }
}

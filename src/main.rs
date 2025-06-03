use clap::Parser;
use error::CompilerError;
use std::path::PathBuf;

use crate::driver::CompilerDriver;

mod codegen;
mod driver;
mod error;
mod expr;
mod lexer;
mod parser;
mod preprocessor;
mod token;
mod value;

#[derive(Parser)]
#[command(name = "rust_c_compiler")]
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
    CompilerDriver::run(&args)
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::path::Path;
    const TEST_FILE: &str = "./target/debug/hello.c"; // Path to the test file

    fn test_args(lex: bool, parse: bool) -> Args {
        Args {
            input: Path::new(TEST_FILE).to_path_buf(),
            lex,
            parse,
            codegen: false, // Set to false for lexer and parser tests
        }
    }
    #[test]
    fn test_all() -> Result<(), CompilerError> {
        CompilerDriver::run(&test_args(false, false))
    }

    #[test]
    fn test_lexer() -> Result<(), CompilerError> {
        CompilerDriver::run(&test_args(true, false))
    }

    #[test]
    fn test_parser() -> Result<(), CompilerError> {
        CompilerDriver::run(&test_args(false, true))
    }
}

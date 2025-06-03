use clap::Parser;
use error::CompilerError;
use std::path::{Path, PathBuf};

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

const TEST_FILE: &str = "./target/debug/hello.c"; // Path to the test file

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
impl Default for Args {
    fn default() -> Self {
        Args {
            input: Path::new(TEST_FILE).to_path_buf(), // 默认空路径，测试时覆盖
            lex: false,
            parse: false,
            codegen: false,
        }
    }
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

    fn test_args(lex: bool, parse: bool) -> Args {
        Args {
            input: Path::new(TEST_FILE).to_path_buf(),
            lex,
            parse,
            ..Args::default()
        }
    }

    #[test]
    fn test_lexer() -> Result<(), CompilerError> {
        CompilerDriver::run(&test_args(true, false))
    }

    #[test]
    fn test_parser() -> Result<(), CompilerError> {
        CompilerDriver::run(&test_args(false, true))
    }
    #[test]
    fn test_all() -> Result<(), CompilerError> {
        CompilerDriver::run(&test_args(false, false))
    }
}

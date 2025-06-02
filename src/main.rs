use clap::Parser;
use error::CompilerError;
use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

mod codegen;
mod error;
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
    let args = Args::parse(); // 命令行解析

    // 1. 预处理
    let preprocessed_path = preprocessor::get_preprocessed_path(&args.input);
    if let Err(e) = preprocessor::preprocess(&args.input, &preprocessed_path) {
        eprintln!("Preprocessing error: {:?}", e);
        cleanup(&preprocessed_path);
        process::exit(1);
    }
    //开始编译
    // 2. 词法分析
    let source = fs::read_to_string(&preprocessed_path)
        .map_err(|e| CompilerError::Io(format!("Failed to read file: {}", e)))?;
    let mut lexer = lexer::Lexer::new(source);
    lexer.tokenize()?;
    if args.lex {
        println!("{:?}", lexer.tokens);
        cleanup(&preprocessed_path); // 清理预处理文件
        return Ok(());
    }

    // // 3. 语法分析 (需要实现 parser)
    // let ast = parser::parse(tokens)?;
    // if args.parse {
    //     println!("{:#?}", ast);
    //     cleanup(&preprocessed_path); // 清理预处理文件
    //     return Ok(());
    // }

    // // 4. 代码生成 (需要实现 codegen)
    // let asm = codegen::generate(ast)?;
    // if args.codegen {
    //     println!("{}", asm);
    //     cleanup(&preprocessed_path); // 清理预处理文件
    //     return Ok(());
    // }

    // 5. 汇编和链接
    let output_path = args.input.with_extension(""); // 输出到相同目录

    if let Err(e) = assemble_and_link(&preprocessed_path, &output_path) {
        eprintln!("Assembly/linking error: {}", e);
        cleanup(&preprocessed_path);
        cleanup(&output_path); // 确保删除可能生成的不完整文件
        process::exit(1);
    }
    // 清理中间文件
    cleanup(&preprocessed_path);
    println!("Successfully generated: {}", output_path.display());
    Ok(())
}

fn assemble_and_link(input: &Path, output: &Path) -> std::io::Result<()> {
    // 这里简化为直接调用gcc完成全流程
    // 实际实现应该分步调用as和ld
    let status = std::process::Command::new("gcc")
        .arg(input)
        .arg("-o")
        .arg(output)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Assembly/linking failed",
        ))
    }
}

fn cleanup(file: &Path) {
    if fs::metadata(file).is_ok() {
        if let Err(e) = fs::remove_file(file) {
            eprintln!("警告: 清理文件 {:?} 失败: {}", file, e);
        }
    }
}

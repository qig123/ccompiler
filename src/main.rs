// src/main.rs

use clap::Parser;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 一个C语言编译器驱动程序
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// [必须] 要编译的C源文件
    source_file: PathBuf,

    /// 运行词法分析器，然后停止
    #[arg(long)]
    lex: bool,

    /// 运行词法分析器和语法分析器，然后停止
    #[arg(long)]
    parse: bool,

    /// 运行到汇编代码生成，然后停止
    #[arg(long)]
    codegen: bool,

    /// 生成汇编文件 (.s) 但不进行汇编或链接
    #[arg(short = 'S', long = "save-assembly")]
    save_assembly: bool, // 使用更有描述性的字段名
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    // 将所有核心逻辑放入一个返回 Result 的函数中，方便使用 `?` 操作符
    run_compiler(cli)
}

fn run_compiler(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // --- 1. 路径和文件校验 ---
    if !cli.source_file.exists() {
        eprintln!("错误: 输入文件不存在: {}", cli.source_file.display());
        std::process::exit(1);
    }
    if cli.source_file.extension().unwrap_or_default() != "c" {
        eprintln!(
            "警告: 输入文件 '{}' 可能不是一个C源文件 (.c)",
            cli.source_file.display()
        );
    }

    // --- 2. 定义所有中间和最终文件路径 ---
    let input_path = &cli.source_file;
    // 输出的可执行文件路径，例如 "program.c" -> "program"
    let output_exe_path = input_path.with_extension("");
    // 预处理后的中间文件路径，例如 "program.c" -> "program.i"
    let preprocessed_path = input_path.with_extension("i");
    // 汇编文件的路径，例如 "program.c" -> "program.s"
    let assembly_path = input_path.with_extension("s");

    // --- 3. 编译流程 (Pipeline) ---

    // 步骤 A: 预处理 (总是执行)
    // gcc -E -P <input_path> -o <preprocessed_path>
    preprocess(input_path, &preprocessed_path)?;

    // 步骤 B: 词法分析 (你的编译器)
    // 在这里，你应该调用你的词法分析器函数
    lex(&preprocessed_path)?; // 这是一个占位符
    if cli.lex {
        println!("--lex: 词法分析完成，程序停止。");
        cleanup(&[&preprocessed_path])?;
        return Ok(());
    }

    // 步骤 C: 语法分析 (你的编译器)
    parse(&preprocessed_path)?; // 这是一个占位符
    if cli.parse {
        println!("--parse: 语法分析完成，程序停止。");
        cleanup(&[&preprocessed_path])?;
        return Ok(());
    }

    // 步骤 D: 代码生成 (你的编译器)
    // 这个函数应该返回生成的汇编代码字符串
    let assembly_code = codegen(&preprocessed_path)?; // 这是一个占位符
    if cli.codegen {
        println!("--codegen: 汇编代码生成完成，程序停止。");
        cleanup(&[&preprocessed_path])?;
        return Ok(());
    }

    // 步骤 E: 发射汇编代码
    // 将生成的汇编代码写入 .s 文件
    fs::write(&assembly_path, assembly_code)?;
    println!("✅ 汇编代码已生成到: {}", assembly_path.display());

    // 步骤 F: 处理 -S 选项
    if cli.save_assembly {
        println!("-S: 保留汇编文件，不进行链接。");
        cleanup(&[&preprocessed_path])?; // 只清理预处理文件
        return Ok(());
    }

    // 步骤 G: 汇编与链接 (默认行为)
    // gcc <assembly_path> -o <output_exe_path>
    assemble_and_link(&assembly_path, &output_exe_path)?;

    // --- 4. 清理所有中间文件 ---
    cleanup(&[&preprocessed_path, &assembly_path])?;
    println!("✅ 编译成功！可执行文件在: {}", output_exe_path.display());

    Ok(())
}

/// 步骤 A: 调用 gcc 进行预处理
fn preprocess(input: &Path, output: &Path) -> io::Result<()> {
    println!(
        "(1) 正在预处理: {} -> {}",
        input.display(),
        output.display()
    );
    let status = Command::new("gcc")
        .arg("-E")
        .arg("-P")
        .arg(input)
        .arg("-o")
        .arg(output)
        .status()?;

    if !status.success() {
        eprintln!("❌ gcc 预处理失败");
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "GCC preprocessing failed",
        ));
    }
    Ok(())
}

/// 步骤 G: 调用 gcc 进行汇编和链接
fn assemble_and_link(assembly_file: &Path, output_exe: &Path) -> io::Result<()> {
    println!(
        "(5) 正在汇编和链接: {} -> {}",
        assembly_file.display(),
        output_exe.display()
    );
    let status = Command::new("gcc")
        .arg(assembly_file)
        .arg("-o")
        .arg(output_exe)
        .status()?;

    if !status.success() {
        eprintln!("❌ gcc 汇编或链接失败");
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "GCC assembly/linking failed",
        ));
    }
    Ok(())
}

/// 清理中间文件
fn cleanup(files: &[&PathBuf]) -> io::Result<()> {
    for file in files {
        if file.exists() {
            println!("   清理临时文件: {}", file.display());
            fs::remove_file(file)?;
        }
    }
    Ok(())
}

// --- 以下是你的编译器核心逻辑的占位符 (Placeholder) ---

/// 步骤 B: 词法分析 (占位符)
fn lex(input: &Path) -> Result<(), io::Error> {
    println!("(2) 正在进行词法分析: {}", input.display());
    // 在这里实现你的词法分析逻辑
    // 如果发现词法错误，返回 Err
    Ok(())
}

/// 步骤 C: 语法分析 (占位符)
fn parse(input: &Path) -> Result<(), io::Error> {
    println!("(3) 正在进行语法分析: {}", input.display());
    // 在这里实现你的语法分析逻辑
    // 如果发现语法错误，返回 Err
    Ok(())
}

/// 步骤 D: 代码生成 (占位符)
fn codegen(input: &Path) -> Result<String, io::Error> {
    println!("(4) 正在生成汇编代码: {}", input.display());
    // 在这里实现你的代码生成逻辑r
    // 如果成功，返回一个包含完整汇编代码的 String
    // 如果失败，返回 Err
    // 使用 `\` 可以让字符串在代码中跨行书写，而不会在最终的字符串中引入换行或多余的空格。
    let placeholder_asm = format!(
        "\t.globl main\n\
         main:\n\
         \tmovl $0, %eax\n\
         \tret\n\
         # Generated from {}\n",
        input.display()
    );
    Ok(placeholder_asm)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_compilation() {
        let cli = Cli {
            source_file: PathBuf::from(r"./tests/input.c"),
            lex: true,
            parse: false,
            codegen: false,
            save_assembly: false,
        };
        let _s = run_compiler(cli);
    }
}

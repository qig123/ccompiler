// src/main.rs

use clap::Parser;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::backend::assembly_ast;
use crate::backend::assembly_ast_gen::AssemblyGenerator;
use crate::backend::code_gen::CodeGenerator;
use crate::frontend::c_ast::AstNode;
use crate::frontend::c_ast::PrettyPrinter;
use crate::frontend::c_ast::Program;
use crate::frontend::lexer;
use crate::frontend::parser;

mod backend;
mod frontend;
/// RAII Guard: 在其生命周期结束时自动清理指定的文件。
#[derive(Debug)]
struct FileJanitor {
    /// 需要被清理的文件路径列表
    files_to_clean: Vec<PathBuf>,
}

impl FileJanitor {
    fn new(files: Vec<PathBuf>) -> Self {
        FileJanitor {
            files_to_clean: files,
        }
    }

    /// "解除" 对某个文件的清理责任。
    /// 当我们希望在成功时保留某个文件（如最终的可执行文件或 .s 文件）时调用。
    fn keep(&mut self, path_to_keep: &Path) {
        self.files_to_clean.retain(|p| p != path_to_keep);
    }
}

/// 当 FileJanitor 实例离开作用域时，`drop` 方法会被自动调用。
impl Drop for FileJanitor {
    fn drop(&mut self) {
        if self.files_to_clean.is_empty() {
            return;
        }
        println!("--- 自动清理 ---");
        for file in &self.files_to_clean {
            if file.exists() {
                // 我们在这里忽略 remove_file 可能的错误，因为清理失败不应使整个程序崩溃。
                // 在更复杂的应用中，你可能会记录这个错误。
                if let Err(e) = fs::remove_file(file) {
                    eprintln!("警告: 清理临时文件 {} 失败: {}", file.display(), e);
                } else {
                    println!("   已清理: {}", file.display());
                }
            }
        }
    }
}

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
    save_assembly: bool,
}

fn main() {
    let cli = Cli::parse();
    let result = run_compiler(cli);
    //eprintln!("\n>>> FINAL COMPILER RESULT: {:?}\n", result);
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_compiler(cli: Cli) -> Result<(), String> {
    // --- 1. 路径和文件校验 ---
    if !cli.source_file.exists() {
        return Err(format!(
            "错误: 输入文件不存在: {}",
            cli.source_file.display()
        ));
    }
    if cli.source_file.extension().unwrap_or_default() != "c" {
        println!(
            "警告: 输入文件 '{}' 可能不是一个C源文件 (.c)",
            cli.source_file.display()
        );
    }

    // --- 2. 定义所有中间和最终文件路径 ---
    let input_path = &cli.source_file;
    let output_exe_path = input_path.with_extension("");
    let preprocessed_path = input_path.with_extension("i");
    let assembly_path = input_path.with_extension("s");

    // --- 2.5. 创建并"武装"我们的清理卫兵 ---
    // 将所有可能生成的都文件交给 Janitor 管理
    let mut janitor = FileJanitor::new(vec![
        preprocessed_path.clone(),
        assembly_path.clone(),
        output_exe_path.clone(),
    ]);

    // 在开始前，先清理一次上次可能遗留的文件
    // drop 会立即调用，执行一次性清理
    drop(FileJanitor::new(vec![
        preprocessed_path.clone(),
        assembly_path.clone(),
        output_exe_path.clone(),
    ]));

    // --- 3. 编译流程 (Pipeline) ---
    // 任何下面的 `?` 失败，都会导致 run_compiler 退出，
    // `janitor` 会被 drop，从而自动清理所有文件。

    // 步骤 A: 预处理
    preprocess(input_path, &preprocessed_path).map_err(|e| e.to_string())?;

    // 步骤 B: 词法分析
    let tokens = lex(&preprocessed_path)?;
    if cli.lex {
        println!("--lex: 词法分析完成，程序停止。");
        // 当函数在这里返回时，janitor 会自动清理 .i 文件
        return Ok(());
    }

    // 步骤 C: 语法分析
    let ast = parse(tokens)?; // parse 应该接收 tokens
    if cli.parse {
        println!("--parse: 语法分析完成，程序停止。");
        return Ok(());
    }

    // 步骤 D: 代码生成
    let assembly_code_ast = codegen(ast)?;
    if cli.codegen {
        println!("--codegen: 汇编代码生成完成，程序停止。");
        return Ok(());
    }

    // 步骤 E: 发射汇编代码 (Code Emission)
    println!("\n(4) 正在发射汇编代码: -> {}", assembly_path.display());
    let code_generator = CodeGenerator::new();
    code_generator
        .generate_program_to_file(&assembly_code_ast, &assembly_path.to_string_lossy())?;
    // 上面这一行调用就完成了所有工作！
    // 它接收汇编 AST 的引用，以及目标文件路径的引用。
    // 如果写入失败，`?` 会自动将错误传递出去。

    // (原来的 fs::write 调用就不再需要了)
    // println! 已经被我们的 CodeGenerator 内部调用了，但在这里再确认一次也很好。
    println!("✅ 汇编代码已生成到: {}", assembly_path.display());

    // 步骤 F: 处理 -S 选项
    if cli.save_assembly {
        // 我们想保留 .s 文件，所以告诉 janitor 不要清理它
        janitor.keep(&assembly_path);
        println!("-S: 保留汇编文件，不进行链接。");
        // janitor 在这里 drop，会清理 .i 文件，但保留 .s 文件
        return Ok(());
    }

    // 步骤 G: 汇编与链接
    assemble_and_link(&assembly_path, &output_exe_path).map_err(|e| e.to_string())?;

    // --- 4. 【核心改动】成功完成，"解除"对最终文件的清理 ---
    // 如果程序运行到这里，说明一切顺利。我们想保留可执行文件。
    janitor.keep(&output_exe_path);
    println!("✅ 编译成功！可执行文件在: {}", output_exe_path.display());

    // 当函数在这里正常结束时，janitor 会被 drop。
    // 由于我们调用了 janitor.keep(&output_exe_path)，
    // 它只会清理 .i 和 .s 文件，而保留最终的可执行文件。
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

// 步骤 B: 词法分析 (占位符)
fn lex(input: &Path) -> Result<Vec<lexer::Token>, String> {
    println!("(2) 正在进行词法分析: {}", input.display());
    let lexer = lexer::Lexer::new();
    let content = fs::read_to_string(input).map_err(|e| e.to_string())?;
    let tokens = lexer.lex(&content)?;
    println!("✅ 词法分析完成，生成 {} 个 token", tokens.len());

    Ok(tokens)
}

/// 步骤 C: 语法分析 (占位符)
fn parse(tokens: Vec<lexer::Token>) -> Result<Program, String> {
    println!("\n(3) 正在进行语法分析 (输入 {} 个 token)", tokens.len());
    let parser = parser::Parser::new(tokens);
    let program = parser.parse()?;

    println!("✅ 语法分析完成，开始打印 AST：");
    // 创建并使用 PrettyPrinter
    println!();
    let mut printer = PrettyPrinter::new();
    program.pretty_print(&mut printer);
    println!();
    Ok(program)
}

/// 步骤 D: 代码生成 (占位符)
fn codegen(c_ast: Program) -> Result<assembly_ast::Program, String> {
    //先要生成汇编Ast
    let mut ass_gen = AssemblyGenerator::new();
    let ass_ast = ass_gen.generate(&c_ast)?;
    println!("{:?}", ass_ast);

    Ok(ass_ast)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_compilation() -> Result<(), String> {
        let cli = Cli {
            source_file: PathBuf::from(r"./tests/program.c"),
            lex: false,
            parse: false,
            codegen: false,
            save_assembly: false,
        };
        run_compiler(cli)
    }
}

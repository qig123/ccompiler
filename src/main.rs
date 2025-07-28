// src/main.rs

use clap::Parser;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::backend::assembly_ast;
use crate::backend::assembly_ast_gen::AssemblyGenerator;
use crate::backend::code_gen::CodeGenerator;
use crate::common::AstNode;
use crate::common::PrettyPrinter;
use crate::frontend::c_ast::Program;
use crate::frontend::lexer;
use crate::frontend::parser;

mod backend;
mod common;
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
        // println!("--- 自动清理 ---"); //
        let mut cleaned_any = false;
        for file in &self.files_to_clean {
            if file.exists() {
                if !cleaned_any {
                    println!("--- 自动清理 ---");
                    cleaned_any = true;
                }
                if let Err(e) = fs::remove_file(file) {
                    eprintln!("   警告: 清理临时文件 {} 失败: {}", file.display(), e);
                } else {
                    println!("   ✅ 已清理: {}", file.display());
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

    // 生成ir
    #[arg(long)]
    tacky: bool,

    /// 运行到汇编代码生成，然后停止
    #[arg(long)]
    codegen: bool,

    /// 生成汇编文件 (.s) 但不进行汇编或链接
    #[arg(short = 'S', long = "save-assembly")]
    save_assembly: bool,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run_compiler(cli) {
        // 在 main 函数中统一处理最终的错误打印
        eprintln!("\n❌ 编译失败: {}", e);
        std::process::exit(1);
    }
}

fn run_compiler(cli: Cli) -> Result<(), String> {
    // --- 1. 路径和文件校验 ---
    if !cli.source_file.exists() {
        return Err(format!("输入文件不存在: {}", cli.source_file.display()));
    }
    if cli.source_file.extension().unwrap_or_default() != "c" {
        println!(
            "   警告: 输入文件 '{}' 可能不是一个C源文件 (.c)",
            cli.source_file.display()
        );
    }

    // --- 2. 定义所有中间和最终文件路径 ---
    let input_path = &cli.source_file;
    let output_exe_path = input_path.with_extension("");
    let preprocessed_path = input_path.with_extension("i");
    let assembly_path = input_path.with_extension("s");

    let mut janitor = FileJanitor::new(vec![
        preprocessed_path.clone(),
        assembly_path.clone(),
        output_exe_path.clone(),
    ]);

    // 在开始前，先清理一次上次可能遗留的文件
    drop(FileJanitor::new(vec![
        preprocessed_path.clone(),
        assembly_path.clone(),
        output_exe_path.clone(),
    ]));

    println!("\n--- 开始编译: {} ---", input_path.display());

    // --- 3. 编译流程 (Pipeline) ---

    // 步骤 A -> (1)
    let tokens = preprocess_and_lex(input_path, &preprocessed_path)?;
    if cli.lex {
        println!("\n--lex: 词法分析完成，程序停止。");
        return Ok(());
    }

    // 步骤 B -> (2)
    let ast = parse(tokens)?;
    if cli.parse {
        println!("\n--parse: 语法分析完成，程序停止。");
        return Ok(());
    }

    // 步骤 C -> (3)
    let ir_ast = gen_ir(&ast)?;
    if cli.tacky {
        println!("\n--tacky: IR 生成完成, 程序停止。");
        return Ok(());
    }

    // 步骤 D -> (4)
    let assembly_code_ast = codegen(ir_ast)?;
    if cli.codegen {
        println!("\n--codegen: 汇编 AST 生成完成, 程序停止。");
        return Ok(());
    }

    // 步骤 E -> (5)
    emit_assembly(&assembly_code_ast, &assembly_path)?;
    if cli.save_assembly {
        janitor.keep(&assembly_path);
        println!("\n-S: 保留汇编文件，不进行链接。编译成功！");
        return Ok(());
    }

    // 步骤 F -> (6)
    assemble_and_link(&assembly_path, &output_exe_path)?;
    janitor.keep(&output_exe_path); // 成功后，解除对可执行文件的清理

    // 步骤 G -> (7)
    run_and_report_exit_code(&output_exe_path)?;

    println!("\n✅ 编译并运行成功！");

    Ok(())
}

// --- 分解后的编译阶段函数 ---

fn preprocess_and_lex(
    input: &Path,
    preprocessed_output: &Path,
) -> Result<Vec<lexer::Token>, String> {
    // (1) 预处理
    println!(
        "(1) 正在预处理: {} -> {}",
        input.display(),
        preprocessed_output.display()
    );
    let status = Command::new("gcc")
        .args(["-E", "-P"])
        .arg(input)
        .args(["-o", preprocessed_output.to_str().unwrap()])
        .status()
        .map_err(|e| format!("无法执行 gcc: {}", e))?;

    if !status.success() {
        return Err("gcc 预处理失败".to_string());
    }
    println!("   ✅ 预处理成功。");

    // (2) 词法分析
    println!("(2) 正在进行词法分析: {}", preprocessed_output.display());
    let lexer = lexer::Lexer::new();
    let content = fs::read_to_string(preprocessed_output).map_err(|e| e.to_string())?;
    let tokens = lexer.lex(&content)?;
    println!("   ✅ 词法分析完成，生成 {} 个 token。", tokens.len());
    Ok(tokens)
}

fn parse(tokens: Vec<lexer::Token>) -> Result<Program, String> {
    println!("(3) 正在进行语法分析 (输入 {} 个 token)...", tokens.len());
    let parser = parser::Parser::new(tokens);
    let program = parser.parse()?;
    println!("   ✅ 语法分析完成。打印 AST:");
    let mut stdout = io::stdout();
    let mut printer = PrettyPrinter::new(&mut stdout);
    program.pretty_print(&mut printer);
    Ok(program)
}

fn gen_ir(c_ast: &Program) -> Result<crate::backend::tacky_ir::Program, String> {
    println!("(4) 正在生成 Tacky IR...");
    let mut ir_gen = backend::tacky_gen::TackyGenerator::new();
    let ir_ast = ir_gen.generate_tacky(c_ast)?;
    println!("   ✅ IR 生成完成。打印 Tacky IR:");
    let mut stdout = io::stdout();
    let mut printer = PrettyPrinter::new(&mut stdout);
    ir_ast.pretty_print(&mut printer);
    Ok(ir_ast)
}

fn codegen(ir_ast: crate::backend::tacky_ir::Program) -> Result<assembly_ast::Program, String> {
    println!("(5) 正在生成汇编 AST...");
    let mut ass_gen = AssemblyGenerator::new();
    let ass_ast = ass_gen.generate(ir_ast)?;
    println!("   ✅ 汇编 AST 生成完成。打印汇编 AST:");
    let mut stdout = io::stdout();
    let mut printer = PrettyPrinter::new(&mut stdout);
    ass_ast.pretty_print(&mut printer);
    Ok(ass_ast)
}

fn emit_assembly(asm_ast: &assembly_ast::Program, output_path: &Path) -> Result<(), String> {
    println!("(6) 正在发射汇编代码 -> {}", output_path.display());
    let code_generator = CodeGenerator::new();
    code_generator.generate_program_to_file(asm_ast, &output_path.to_string_lossy())?;
    println!("   ✅ 汇编代码已生成。");
    Ok(())
}

fn assemble_and_link(assembly_file: &Path, output_exe: &Path) -> Result<(), String> {
    println!(
        "(7) 正在汇编和链接: {} -> {}",
        assembly_file.display(),
        output_exe.display()
    );
    let status = Command::new("gcc")
        .arg(assembly_file)
        .args(["-o", output_exe.to_str().unwrap()])
        .status()
        .map_err(|e| format!("无法执行 gcc: {}", e))?;

    if !status.success() {
        return Err("gcc 汇编或链接失败".to_string());
    }
    println!("   ✅ 汇编与链接成功。");
    Ok(())
}

fn run_and_report_exit_code(executable: &Path) -> Result<(), String> {
    println!("(8) 正在运行生成的可执行文件: {}", executable.display());
    let status = Command::new(executable)
        .status()
        .map_err(|e| format!("无法运行生成的文件 '{}': {}", executable.display(), e))?;

    match status.code() {
        Some(code) => {
            println!("   ✅ 程序执行完毕，返回值为: {}", code);
            Ok(())
        }
        None => Err("程序被信号终止，没有返回码。".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_compilation() -> Result<(), String> {
        let cli = Cli {
            source_file: PathBuf::from(r"./tests/program.c"),
            lex: true,
            parse: false,
            tacky: false,
            codegen: true,
            save_assembly: false,
        };
        run_compiler(cli)
    }
}

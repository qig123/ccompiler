// src/main.rs

use clap::Parser;
use std::collections::HashMap;
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
use crate::frontend::loop_labeling::LoopLabeling;
use crate::frontend::parser;
use crate::frontend::resolve_ident::IdentifierResolver;
use crate::frontend::type_checking::SymbolInfo;
use crate::frontend::type_checking::TypeChecker;

mod backend;
mod common;
mod frontend;

/// RAII Guard: 在其生命周期结束时自动清理指定的文件。
#[derive(Debug)]
struct FileJanitor {
    files_to_clean: Vec<PathBuf>,
}

impl FileJanitor {
    fn new(files: Vec<PathBuf>) -> Self {
        FileJanitor {
            files_to_clean: files,
        }
    }
    fn keep(&mut self, path_to_keep: &Path) {
        self.files_to_clean.retain(|p| p != path_to_keep);
    }
}

impl Drop for FileJanitor {
    fn drop(&mut self) {
        if self.files_to_clean.is_empty() {
            return;
        }
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

/// 全局计数器，用于生成唯一的名称和标签。
#[derive(Debug, Default)]
pub struct UniqueNameGenerator {
    counter: u32,
}
impl UniqueNameGenerator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn new_temp_var(&mut self) -> String {
        let current_value = self.counter;
        self.counter += 1;
        format!("tmp{}", current_value)
    }
    pub fn new_label(&mut self, name: &str) -> String {
        let current_value = self.counter;
        self.counter += 1;
        format!("{}.{}", name, current_value)
    }
    pub fn new_loop_label(&mut self, name: &str) -> String {
        self.new_label(name)
    }
    pub fn new_variable_name(&mut self, name: String) -> String {
        let current_value = self.counter;
        self.counter += 1;
        format!("{}.{}", name, current_value)
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

    /// 运行到语义分析完成，然后停止
    #[arg(long)]
    validate: bool,

    /// 运行到Tacky IR生成，然后停止
    #[arg(long)]
    tacky: bool,

    /// 运行到汇编代码生成，然后停止
    #[arg(long)]
    codegen: bool,

    /// 生成汇编文件 (.s) 并保留它
    #[arg(short = 'S', long = "save-assembly")]
    save_assembly: bool,

    /// 【只编译到目标文件 (.o)，不进行链接
    #[arg(short = 'c', long = "compile-only")]
    compile_only: bool,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run_compiler(cli) {
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
    let output_obj_path = input_path.with_extension("o");
    let output_exe_path = input_path.with_extension("");
    let preprocessed_path = input_path.with_extension("i");
    let assembly_path = input_path.with_extension("s");

    // 设置自动清理器，确保临时文件在程序结束时被删除
    let mut janitor = FileJanitor::new(vec![
        preprocessed_path.clone(),
        assembly_path.clone(),
        output_obj_path.clone(),
        output_exe_path.clone(),
    ]);

    // 在开始前，先清理一次上次可能遗留的文件
    drop(FileJanitor::new(vec![
        preprocessed_path.clone(),
        assembly_path.clone(),
        output_obj_path.clone(),
        output_exe_path.clone(),
    ]));

    // 初始化唯一名称生成器
    let mut name_gen = UniqueNameGenerator::new();

    println!("\n--- 开始编译: {} ---", input_path.display());

    // --- 3. 编译流程 (Pipeline) ---

    // (1) 预处理和词法分析
    let tokens = preprocess_and_lex(input_path, &preprocessed_path)?;
    if cli.lex {
        println!("\n--lex: 词法分析完成，程序停止。");
        return Ok(());
    }

    // (2) 语法分析
    let ast = parse(tokens)?;
    if cli.parse {
        println!("\n--parse: 语法分析完成，程序停止。");
        return Ok(());
    }

    // (3) 语义分析
    let resolved_ast = resolve_idents(&ast, &mut name_gen)?;
    let labeled_ast = label_loops(&resolved_ast, &mut name_gen)?;
    let tables = typecheck(&labeled_ast)?;
    if cli.validate {
        println!("\n--validate: 语义分析完成, 程序停止。");
        return Ok(());
    }

    // (4) 中间代码(IR)生成
    let ir_ast = gen_ir(&labeled_ast, &mut name_gen)?;
    if cli.tacky {
        println!("\n--tacky: IR 生成完成, 程序停止。");
        return Ok(());
    }

    // (5) 汇编AST生成
    let assembly_code_ast = codegen(ir_ast)?;
    if cli.codegen {
        println!("\n--codegen: 汇编 AST 生成完成, 程序停止。");
        return Ok(());
    }

    // (6) 发射汇编代码
    emit_assembly(&assembly_code_ast, &assembly_path, &tables)?;
    if cli.save_assembly {
        janitor.keep(&assembly_path); // 保留汇编文件
        println!("\n-S: 保留汇编文件。");
    }

    // --- 根据 -c 标志决定下一步 ---

    if cli.compile_only {
        // (7a) 只汇编，不链接
        assemble_only(&assembly_path, &output_obj_path)?;
        janitor.keep(&output_obj_path); // 保留 .o 文件
        println!("\n✅ 编译完成，生成目标文件: {}", output_obj_path.display());
    } else {
        // (7b) 汇编并链接
        assemble_and_link(&assembly_path, &output_exe_path)?;
        janitor.keep(&output_exe_path); // 保留可执行文件

        // (8) 运行并报告退出码
        run_and_report_exit_code(&output_exe_path)?;
        println!("\n✅ 编译并运行成功！");
    }

    Ok(())
}

// --- 分解后的编译阶段函数 ---

fn preprocess_and_lex(
    input: &Path,
    preprocessed_output: &Path,
) -> Result<Vec<lexer::Token>, String> {
    println!(
        "(1) 预处理: {} -> {}",
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

    println!("(1) 词法分析: {}", preprocessed_output.display());
    let lexer = lexer::Lexer::new();
    let content = fs::read_to_string(preprocessed_output).map_err(|e| e.to_string())?;
    let tokens = lexer.lex(&content)?;
    println!(
        "   ✅ 预处理与词法分析完成，生成 {} 个 token。",
        tokens.len()
    );
    Ok(tokens)
}
fn parse(tokens: Vec<lexer::Token>) -> Result<Program, String> {
    println!("(2) 语法分析 (输入 {} 个 token)...", tokens.len());
    let parser = parser::Parser::new(tokens);
    let program = parser.parse()?;
    println!("   ✅ 语法分析完成。打印 AST:");
    let mut stdout = io::stdout();
    let mut printer = PrettyPrinter::new(&mut stdout);
    program.pretty_print(&mut printer);
    Ok(program)
}
fn resolve_idents(c_ast: &Program, g: &mut UniqueNameGenerator) -> Result<Program, String> {
    println!("(3.1) 语义分析：标识符解析...");
    let mut resolver = IdentifierResolver::new(g);
    let ast = resolver.resolve_program(c_ast)?;
    println!("   ✅ 标识符解析完成, 打印解析后的 AST:");
    let mut stdout = io::stdout();
    let mut printer = PrettyPrinter::new(&mut stdout);
    ast.pretty_print(&mut printer);
    Ok(ast)
}
fn label_loops(c_ast: &Program, g: &mut UniqueNameGenerator) -> Result<Program, String> {
    println!("(3.2) 语义分析：循环标记...");
    let mut v = LoopLabeling::new(g);
    let ast = v.label_loops_in_program(c_ast)?;
    println!("   ✅ 循环标记完成, 打印标记后的 AST:");
    let mut stdout = io::stdout();
    let mut printer = PrettyPrinter::new(&mut stdout);
    ast.pretty_print(&mut printer);
    Ok(ast)
}
fn typecheck(c_ast: &Program) -> Result<HashMap<String, SymbolInfo>, String> {
    println!("(3.3) 类型检查：...");
    let resolver = TypeChecker::new();
    let tables = resolver.typecheck_program(c_ast)?;
    println!("   ✅ 类型检查完成,打印符号表");
    println!("{:?}", tables);
    Ok(tables)
}
fn gen_ir(
    c_ast: &Program,
    g: &mut UniqueNameGenerator,
) -> Result<crate::backend::tacky_ir::Program, String> {
    println!("(4) Tacky IR 生成...");
    let mut ir_gen = backend::tacky_gen::TackyGenerator::new(g);
    let ir_ast = ir_gen.generate_tacky(c_ast)?;
    println!("   ✅ IR 生成完成。打印 Tacky IR:");
    let mut stdout = io::stdout();
    let mut printer = PrettyPrinter::new(&mut stdout);
    ir_ast.pretty_print(&mut printer);
    Ok(ir_ast)
}
fn codegen(ir_ast: crate::backend::tacky_ir::Program) -> Result<assembly_ast::Program, String> {
    println!("(5) 汇编 AST 生成...");
    let mut ass_gen = AssemblyGenerator::new();
    let ass_ast = ass_gen.generate(ir_ast)?;
    println!("   ✅ 汇编 AST 生成完成。打印汇编 AST:");
    let mut stdout = io::stdout();
    let mut printer = PrettyPrinter::new(&mut stdout);
    ass_ast.pretty_print(&mut printer);
    Ok(ass_ast)
}
fn emit_assembly(
    asm_ast: &assembly_ast::Program,
    output_path: &Path,
    tables: &HashMap<String, SymbolInfo>,
) -> Result<(), String> {
    println!("(6) 汇编代码发射 -> {}", output_path.display());
    let code_generator = CodeGenerator::new(tables);
    code_generator.generate_program_to_file(asm_ast, &output_path.to_string_lossy())?;
    println!("   ✅ 汇编代码已生成。");
    Ok(())
}

/// 只将汇编文件编译成目标文件。
fn assemble_only(assembly_file: &Path, output_obj: &Path) -> Result<(), String> {
    println!(
        "(7a) 仅汇编: {} -> {}",
        assembly_file.display(),
        output_obj.display()
    );
    let status = Command::new("gcc")
        .arg("-c") // 关键标志
        .arg(assembly_file)
        .args(["-o", output_obj.to_str().unwrap()])
        .status()
        .map_err(|e| format!("无法执行 gcc: {}", e))?;

    if !status.success() {
        return Err("gcc 汇编失败".to_string());
    }
    println!("   ✅ 汇编成功。");
    Ok(())
}

fn assemble_and_link(assembly_file: &Path, output_exe: &Path) -> Result<(), String> {
    println!(
        "(7b) 汇编与链接: {} -> {}",
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
    println!("(8) 运行生成的可执行文件: {}", executable.display());
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
            lex: false,
            parse: false,
            validate: false,
            tacky: false,
            codegen: false,
            save_assembly: false,
            compile_only: false,
        };
        run_compiler(cli)
    }
}

use crate::{
    codegen::{
        assembly_emitter::CodeEmitter, assembly_ir::Assemble, codegen::TackyToAssemblyTranslator,
    },
    error::{CodegenError, CompilerError, ParserError, TackyError},
    lexer::{self, Token},
    parser::{c_ast::Program, parser::Parser},
    tacky::gen_tacky::AstToTackyTranslator,
};
use std::{fs, path::Path};

pub struct CompilerDriver;

impl CompilerDriver {
    pub fn run(args: &crate::Args) -> Result<(), CompilerError> {
        // 1. 预处理
        let preprocessed_path = &args.input.with_extension("i");
        Self::preprocess(&args.input, &preprocessed_path)?;

        let source = fs::read_to_string(&preprocessed_path)
            .map_err(|e| CompilerError::Io(format!("Failed to read file: {}", e)))?;

        // 2. 词法分析
        let tokens = Self::lex(&source)?;
        if args.lex {
            println!("{:?}", tokens);
            Self::cleanup(&preprocessed_path);
            return Ok(());
        }
        // 3. 语法分析
        let ast = Self::parse(tokens, &source)?;
        if args.parse {
            println!("{:#?}", ast);
            Self::cleanup(&preprocessed_path);
            return Ok(());
        }
        let tacky_ast = Self::translate_tacky(ast, &source)?;
        if args.tacky {
            println!("{:#?}", tacky_ast);
            Self::cleanup(&preprocessed_path);
            return Ok(());
        }

        // 4. 代码生成
        let asm = Self::codegen(tacky_ast)?;
        if args.codegen {
            // 打印AST信息
            println!("");
            println!("[AST Debug]");
            println!("");
            println!("{:#?}", asm); // 使用 {:#?} 美化输出
            Self::cleanup(&preprocessed_path);
            return Ok(());
        }
        // 生成汇编文件
        let asm_output_path = args.input.with_extension("s");
        CodeEmitter::emit(&asm, &asm_output_path)?;
        //打印生成的汇编代码
        println!("\n[Generated Assembly]");
        println!("");
        println!("Output file: {}", asm_output_path.display());
        match fs::read_to_string(&asm_output_path) {
            Ok(asm_content) => {
                println!("");
                println!("{}", asm_content);
            }
            Err(e) => {
                println!("\n[Error] Failed to read assembly file:");
                println!("File: {}", asm_output_path.display());
                println!("Error: {}", e);
            }
        }
        // 5. 汇编和链接
        let output_path = args.input.with_extension("");

        println!("Assembling and linking to: {}", output_path.display());
        Self::assemble_and_link(&asm_output_path, &output_path)?;
        Self::cleanup(&preprocessed_path);
        println!("Successfully generated: {}", output_path.display());
        Ok(())
    }

    fn preprocess(input_file: &Path, output_file: &Path) -> Result<(), CompilerError> {
        let mut command = std::process::Command::new("gcc");
        let status = command
            .args(&[
                "-E",
                "-P",
                input_file.to_str().unwrap(),
                "-o",
                output_file.to_str().unwrap(),
            ])
            .status()
            .map_err(|e| {
                CompilerError::ExternalToolError(format!(
                    "Failed to execute preprocessor ({}): {}",
                    command.get_program().display(),
                    e
                ))
            })?;
        if status.success() {
            Ok(())
        } else {
            Err(CompilerError::ExternalToolError(format!(
                "Preprocess failed. {} exited with status: {:?}",
                command.get_program().display(),
                status.code()
            )))
        }
    }

    fn lex<'a>(source: &'a str) -> Result<Vec<Token>, CompilerError> {
        let mut lexer = lexer::Lexer::new(&source);
        lexer.tokenize()?;
        Ok(lexer.tokens)
    }

    fn parse<'a>(tokens: Vec<Token>, source: &'a str) -> Result<Program, ParserError> {
        let mut parser = Parser::new(tokens, source);
        parser.parse()
    }
    fn translate_tacky<'a>(
        ast: Program,
        source: &'a str,
    ) -> Result<crate::tacky::tacky::Program, TackyError> {
        let mut t = AstToTackyTranslator::new(source);
        t.translate_program(ast)
    }
    fn codegen<'a>(ast: crate::tacky::tacky::Program) -> Result<Assemble, CodegenError> {
        let mut codegen = TackyToAssemblyTranslator::new();
        codegen.translate(ast)
    }

    fn assemble_and_link(input: &Path, output: &Path) -> Result<(), CompilerError> {
        let mut command = std::process::Command::new("gcc");
        command.arg("-o").arg(output).arg(input);

        let status = command.status().map_err(|e| {
            CompilerError::ExternalToolError(format!(
                "Failed to execute linker ({}): {}",
                command.get_program().display(), // 获取命令名
                e
            ))
        })?;

        if status.success() {
            Ok(())
        } else {
            // 命令执行成功，但返回了非零状态码
            // 这也是一种外部工具错误
            Err(CompilerError::ExternalToolError(format!(
                "Linking failed. {} exited with status: {:?}",
                command.get_program().display(),
                status.code() // 获取退出码
            )))
        }
    }
    fn cleanup(file: &Path) {
        if fs::metadata(file).is_ok() {
            let _ = fs::remove_file(file);
        }
    }
}

use crate::{
    codegen::{ast::Assemble, codegen::AssemblyGenerator},
    error::{CodegenError, CompilerError, ParserError},
    expr::Program,
    lexer::{self, Token},
    parser,
    preprocessor::{self},
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
        //  println!("{:#?}", ast);

        if args.parse {
            println!("{:#?}", ast);
            Self::cleanup(&preprocessed_path);
            return Ok(());
        }

        // 4. 代码生成（示例）
        let asm = Self::codegen(ast, &source)?;
        if args.codegen {
            println!("{:?}", asm);
            Self::cleanup(&preprocessed_path);
            return Ok(());
        }

        // 5. 汇编和链接
        let output_path = args.input.with_extension("");
        // println!("Output path: {}", output_path.display());
        Self::assemble_and_link(&args.input, &output_path)?;
        Self::cleanup(&preprocessed_path);
        println!("Successfully generated: {}", output_path.display());
        Ok(())
    }

    fn preprocess(input: &Path, output: &Path) -> Result<(), CompilerError> {
        preprocessor::preprocess(input, output).map_err(|e| {
            Self::cleanup(output);
            e
        })
    }

    fn lex<'a>(source: &'a str) -> Result<Vec<Token>, CompilerError> {
        let mut lexer = lexer::Lexer::new(&source);
        lexer.tokenize()?;
        Ok(lexer.tokens)
    }

    fn parse<'a>(tokens: Vec<Token>, source: &'a str) -> Result<Program, ParserError> {
        let mut parser = parser::Parser::new(tokens, source);
        parser.parse()
    }
    fn codegen<'a>(ast: Program, source: &'a str) -> Result<Assemble, CodegenError> {
        let mut codegen = AssemblyGenerator::new(source);
        codegen.generate(ast)
    }

    fn assemble_and_link(input: &Path, output: &Path) -> Result<(), CompilerError> {
        let status = std::process::Command::new("gcc")
            .arg(input)
            .arg("-o")
            .arg(output)
            .status()
            .map_err(|e| CompilerError::Io(e.to_string()))?;

        if status.success() {
            Ok(())
        } else {
            Err(CompilerError::Io("Linking failed".into()))
        }
    }

    fn cleanup(file: &Path) {
        if fs::metadata(file).is_ok() {
            let _ = fs::remove_file(file);
        }
    }
}

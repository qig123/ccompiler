use crate::{
    error::{CompilerError, ParserError},
    expr::Function,
    lexer::{self, Token},
    parser,
    preprocessor::{self},
};
use std::{fs, path::Path};

pub struct CompilerDriver;

impl CompilerDriver {
    pub fn run(args: &crate::Args) -> Result<(), CompilerError> {
        // 1. 预处理
        // println!("Preprocessing input file: {}", args.input.display());
        let preprocessed_path = &args.input.with_extension("i");
        Self::preprocess(&args.input, &preprocessed_path)?;
        // 2. 词法分析
        let tokens = Self::lex(&preprocessed_path)?;
        if args.lex {
            println!("{:?}", tokens);
            Self::cleanup(&preprocessed_path);
            return Ok(());
        }

        // 3. 语法分析
        let ast = Self::parse(tokens)?;
        if args.parse {
            println!("{:#?}", ast);
            Self::cleanup(&preprocessed_path);
            return Ok(());
        }

        // 4. 代码生成（示例）
        // let asm = Self::codegen(ast)?;
        // if args.codegen {
        //     println!("{}", asm);
        //     return Ok(());
        // }

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

    fn lex(path: &Path) -> Result<Vec<Token>, CompilerError> {
        let source = fs::read_to_string(path)
            .map_err(|e| CompilerError::Io(format!("Failed to read file: {}", e)))?;
        let mut lexer = lexer::Lexer::new(source);
        lexer.tokenize()?;
        Ok(lexer.tokens)
    }

    fn parse(tokens: Vec<Token>) -> Result<Vec<Function>, ParserError> {
        let mut parser = parser::Parser::new(tokens);
        parser.parse()
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

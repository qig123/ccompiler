use std::io; // 如果需要处理文件I/O错误

// Lexer 阶段的错误
#[derive(Debug, PartialEq, Clone)]
pub struct LexerError {
    pub message: String,
}

// Parser 阶段的错误
#[derive(Debug, PartialEq, Clone)]
pub struct ParserError {
    pub message: String,
}

// Codegen 阶段的错误
#[derive(Debug, PartialEq, Clone)]
pub struct CodegenError {
    pub message: String,
    // Codegen 错误可能与源码位置无关，也可能有关联（例如，某个表达式无法生成代码）
    // pub line: Option<usize>,
    // pub column: Option<usize>,
}

// 顶层编译器错误，包装了所有可能的错误类型
#[derive(Debug, PartialEq, Clone)]
pub enum CompilerError {
    Lexer(LexerError),
    Parser(ParserError),
    Codegen(CodegenError),
    // 其他可能的错误，例如文件读取错误
    Io(String), // Simplistic wrapper for IO errors
                // Internal compiler errors (should not happen if code is correct, but good for debugging)
                // InternalError(String),
}

// 实现 From trait 方便自动转换错误
impl From<LexerError> for CompilerError {
    fn from(err: LexerError) -> Self {
        CompilerError::Lexer(err)
    }
}

impl From<ParserError> for CompilerError {
    fn from(err: ParserError) -> Self {
        CompilerError::Parser(err)
    }
}

impl From<CodegenError> for CompilerError {
    fn from(err: CodegenError) -> Self {
        CompilerError::Codegen(err)
    }
}

// 如果需要处理 std::io::Error
impl From<io::Error> for CompilerError {
    fn from(err: io::Error) -> Self {
        CompilerError::Io(err.to_string())
    }
}

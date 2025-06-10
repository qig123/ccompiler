use std::{fmt, io}; // 如果需要处理文件I/O错误

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
#[derive(Debug, Clone, PartialEq)] // 添加 Clone 和 PartialEq 以便测试
pub enum SemanticError {
    DuplicateDeclaration {
        name: String,
        // 如果可能，可以尝试传递 Token 本身或其近似位置信息，
        // 但如果不行，至少有 name
    },
    UndeclaredVariable {
        name: String,
    },
    InvalidLvalue {
        // 可以尝试描述是什么样的非法左值，
        // 例如 "assignment to a literal" 或 "assignment to a binary expression"
        // 但这需要 analyze_expression 在检测到错误时能提供这种上下文
        // 简单起见，可以先不加额外描述，或者只加一个通用的
        description: String, // 例如 "Cannot assign to this expression"
    },
    Internal(String),
    MisplacedBreak,
    MisplacedContinue,
}
// Codegen 阶段的错误
#[derive(Debug, PartialEq, Clone)]
pub struct CodegenError {
    pub message: String,
}
#[derive(Debug, PartialEq, Clone)]
pub struct TackyError {
    pub message: String,
}
#[derive(Debug, PartialEq, Clone)]

pub struct CodeEmitterError {
    pub message: String,
}

// 顶层编译器错误，包装了所有可能的错误类型
#[derive(Debug, PartialEq, Clone)]
pub enum CompilerError {
    Lexer(LexerError),
    Parser(ParserError),
    Semantic(SemanticError),
    Codegen(CodegenError),
    CodeEmitter(CodeEmitterError),
    Tacky(TackyError),
    Io(String), // 可以保留这个用于其他一般的文件I/O错误
    // 添加一个新的变体用于外部工具执行错误
    ExternalToolError(String),
    // 或者更具体一些，例如
    // LinkingError(String),
    // AssemblyError(String), // 如果汇编和链接分开调用
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
impl From<SemanticError> for CompilerError {
    fn from(err: SemanticError) -> Self {
        CompilerError::Semantic(err)
    }
}

impl From<CodegenError> for CompilerError {
    fn from(err: CodegenError) -> Self {
        CompilerError::Codegen(err)
    }
}

impl From<CodeEmitterError> for CompilerError {
    fn from(err: CodeEmitterError) -> Self {
        CompilerError::CodeEmitter(err)
    }
}
impl From<TackyError> for CompilerError {
    fn from(err: TackyError) -> Self {
        CompilerError::Tacky(err)
    }
}

// 如果需要处理 std::io::Error
impl From<io::Error> for CompilerError {
    fn from(err: io::Error) -> Self {
        CompilerError::Io(err.to_string())
    }
}

// 确保 LexerError, ParserError, CodegenError, CodeEmitterError
// 也实现了 Display 或至少 Debug
impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
// 对 ParserError, CodegenError, CodeEmitterError 做类似实现
impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemanticError::DuplicateDeclaration { name } => {
                write!(
                    f,
                    "Semantic Error: Variable '{}' has already been declared in this scope.",
                    name
                )
            }
            SemanticError::UndeclaredVariable { name } => {
                write!(
                    f,
                    "Semantic Error: Variable '{}' has not been declared yet.",
                    name
                )
            }
            SemanticError::InvalidLvalue { description } => {
                write!(
                    f,
                    "Semantic Error: Invalid assignment target. {}.",
                    description
                )
            }
            SemanticError::MisplacedBreak => {
                write!(f, "Semantic Error: 'break' statement is misplaced.")
            }
            SemanticError::MisplacedContinue => {
                write!(f, "Semantic Error: 'continue' statement is misplaced.")
            }

            Self::Internal(msg) => {
                write!(f, "Semantic Error: Internal error: {}", msg)
            }
        }
    }
}
impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl fmt::Display for TackyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl fmt::Display for CodeEmitterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerError::Lexer(err) => write!(f, "Lexer Error: {}", err),
            CompilerError::Parser(err) => write!(f, "Parser Error: {}", err),
            CompilerError::Semantic(err) => write!(f, "Semantic Error: {}", err),
            CompilerError::Codegen(err) => write!(f, "Codegen Error: {}", err),
            CompilerError::Tacky(err) => write!(f, "Tacky Error: {}", err),
            CompilerError::CodeEmitter(err) => write!(f, "Code Emitter Error: {}", err),
            CompilerError::Io(msg) => write!(f, "IO Error: {}", msg),
            CompilerError::ExternalToolError(msg) => write!(f, "External Tool Error: {}", msg), // 添加这行
        }
    }
}

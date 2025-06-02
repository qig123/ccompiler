use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    KeywordInt,    // Keyword: int
    KeywordReturn, // Keyword: return
    KeywrodVoid,   // Keyword: void
    Identifer,     // Identifier: variable or function name
    LeftParen,     // Symbol: (
    RightParen,    // Symbol: )
    LeftBrace,     // Symbol: {
    RightBrace,    // Symbol: }
    Comma,         // Symbol: ,
    Semicolon,     // Symbol: ;
    LiteralInt,    // Literal: integer value
}
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    token_type: TokenType,
    line: usize,
    column: usize,
    lexeme: String, // 简单定义为String,后面考虑像C一样，用指针？
    literal: Option<Value>,
}
impl Token {
    pub fn new(
        token_type: TokenType,
        line: usize,
        column: usize,
        lexeme: String,
        literal: Option<Value>,
    ) -> Self {
        Token {
            token_type,
            line,
            column,
            lexeme,
            literal,
        }
    }
}

use std::ops::Range;

use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    KeywordInt,    // Keyword: int
    KeywordReturn, // Keyword: return
    KeywordVoid,   // Keyword: void
    Identifier,    // Identifier: variable or function name
    LeftParen,     // Symbol: (
    RightParen,    // Symbol: )
    LeftBrace,     // Symbol: {
    RightBrace,    // Symbol: }
    Comma,         // Symbol: ,
    Semicolon,     // Symbol: ;
    LiteralInt,    // Literal: integer value
    Eof,           // End of file
}
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub column: usize,
    pub lexeme_range: Range<usize>,
    pub literal: Option<Value>,
}
impl Token {
    pub fn new(
        token_type: TokenType,
        line: usize,
        column: usize,
        lexeme: Range<usize>,
        literal: Option<Value>,
    ) -> Self {
        Token {
            token_type,
            line,
            column,
            lexeme_range: lexeme,
            literal,
        }
    }
    pub fn get_lexeme<'a>(&self, source: &'a str) -> &'a str {
        &source[self.lexeme_range.clone()] // clone() 因为 Range 是 Copy
    }
}

use std::ops::Range;

use crate::types::types::Value;

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
    BitwiseNot,    // ~
    Minus,         // -
    Decrement,     // --
    Add,           //+
    Mul,           //*
    Div,
    Remainder,    //%
    Bang,         // !
    And,          // &&
    Or,           // ||
    EqualEqual,   // ==
    BangEqual,    // !=
    Less,         // <
    LessEqual,    //<=,
    Greater,      // >
    GreaterEqual, // >=
    Equal,

    Eof, // End of file
}
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme_range: Range<usize>,
    pub literal: Option<Value>,
}
impl Token {
    pub fn new(token_type: TokenType, lexeme: Range<usize>, literal: Option<Value>) -> Self {
        Token {
            token_type,
            lexeme_range: lexeme,
            literal,
        }
    }
    pub fn get_lexeme<'a>(&self, source: &'a str) -> &'a str {
        &source[self.lexeme_range.clone()] // clone() 因为 Range 是 Copy
    }
}

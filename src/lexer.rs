use crate::{
    error::LexerError,
    token::{Token, TokenType},
    value::Value,
};

pub struct Lexer {
    source: String,
    current: usize,
    start: usize,
    line: usize,
    column: usize,
    pub tokens: Vec<Token>,
}
impl Lexer {
    pub fn new(source: String) -> Self {
        Lexer {
            source,
            current: 0,
            start: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
        }
    }

    pub fn tokenize(&mut self) -> Result<(), LexerError> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token()?;
        }
        self.tokens.push(Token::new(
            crate::token::TokenType::Semicolon, // EOF token
            self.line,
            self.column,
            String::from("EOF"),
            None,
        ));
        Ok(())
    }
    fn scan_token(&mut self) -> Result<(), LexerError> {
        self.start = self.current;
        let c = self.advance();
        match c {
            '(' => {
                self.add_token(crate::token::TokenType::LeftParen, None);
                Ok(())
            }
            ')' => {
                self.add_token(crate::token::TokenType::RightParen, None);
                Ok(())
            }
            '{' => {
                self.add_token(crate::token::TokenType::LeftBrace, None);
                Ok(())
            }
            '}' => {
                self.add_token(crate::token::TokenType::RightBrace, None);
                Ok(())
            }
            ',' => {
                self.add_token(crate::token::TokenType::Comma, None);
                Ok(())
            }
            ';' => {
                self.add_token(crate::token::TokenType::Semicolon, None);
                Ok(())
            }
            '0'..='9' => {
                // 1. 处理以数字开头的 token
                let mut is_valid_number = true; // 假设当前是有效的数字
                while let Some(next_char) = self.peek() {
                    if next_char.is_digit(10) {
                        self.advance(); // 继续扫描数字
                    } else if next_char.is_alphabetic() || next_char == '_' {
                        // 2. 发现数字后面跟着字母或下划线，说明是非法的标识符
                        is_valid_number = false;
                        break; // 退出循环，停止继续读取字符
                    } else {
                        // 3. 遇到数字字面量的结束字符（例如空格、运算符等）
                        break;
                    }
                }
                if is_valid_number {
                    // 4. 如果是有效的数字字面量
                    let lexeme = &self.source[self.start..self.current];
                    if let Ok(value) = lexeme.parse::<i64>() {
                        self.add_token(TokenType::LiteralInt, Some(Value::Int(value))); // 添加数字字面量 token (假设 Value::Int 存在)
                    } else {
                        // 数字太大，无法解析为 i64
                        return Err(LexerError {
                            message: "Integer literal is too large".to_string(),
                            line: self.line,
                            column: self.column - (self.current - self.start),
                        });
                    }
                    Ok(())
                } else {
                    // 5. 如果是非法的标识符
                    return Err(LexerError {
                        message: "标识符不能以数字开头".to_string(),
                        line: self.line,
                        column: self.column - (self.current - self.start), // 使用正确的列号
                    });
                }
            }
            _ if c.is_alphabetic() || c == '_' => {
                // Handle identifiers and keywords
                while let Some(next_char) = self.peek() {
                    if next_char.is_alphanumeric() || next_char == '_' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let lexeme = &self.source[self.start..self.current];
                match lexeme {
                    "int" => self.add_token(crate::token::TokenType::KeywordInt, None),
                    "return" => self.add_token(crate::token::TokenType::KeywordReturn, None),
                    "void" => self.add_token(crate::token::TokenType::KeywrodVoid, None),
                    _ => self.add_token(crate::token::TokenType::Identifer, None),
                }
                Ok(())
            }
            _ if c.is_whitespace() => {
                // Handle whitespace
                if c == '\n' {
                    self.line += 1;
                    self.column = 1; // Reset column on new line
                } else {
                    self.column += 1; // Increment column for other whitespace
                }
                Ok(())
            }
            _ => {
                // Handle unexpected characters
                return Err(LexerError {
                    message: format!("Unexpected character: '{}'", c),
                    line: self.line,
                    column: self.column,
                });
            }
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }
    fn advance(&mut self) -> char {
        //这里用unwrap()是因为我们假设在调用时，current不超过source的长度
        let c = self.source[self.current..].chars().next().unwrap();
        self.current += c.len_utf8();
        self.column += 1;
        c
    }
    fn add_token(
        &mut self,
        token_type: crate::token::TokenType,
        literal: Option<crate::value::Value>,
    ) {
        let lexeme = self.source[self.start..self.current].to_string();
        let token = Token::new(token_type, self.line, self.column, lexeme, literal);
        self.tokens.push(token);
    }
    fn peek(&self) -> Option<char> {
        self.source[self.current..].chars().next()
    }
    // fn peek_next(&self) -> Option<char> {
    //     let next_index = self.current + self.peek()?.len_utf8();
    //     self.source[next_index..].chars().next()
    // }
    // fn match_char(&mut self, expected: char) -> bool {
    //     if self.is_at_end() || self.peek() != Some(expected) {
    //         return false;
    //     }
    //     self.advance();
    //     true
    // }
}

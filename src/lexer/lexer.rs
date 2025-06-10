use crate::{
    error::LexerError,
    lexer::token::{Token, TokenType},
    types::types::Value,
};

pub struct Lexer<'a> {
    source: &'a str,
    current: usize, // 当前正在处理的字符的字节索引 (Exclusive)
    start: usize,   // 当前 token 的起始字节索引 (Inclusive)
    pub tokens: Vec<Token>,
}
impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            source,
            current: 0,
            start: 0,
            tokens: Vec::new(),
        }
    }

    pub fn tokenize(&mut self) -> Result<(), LexerError> {
        while !self.is_at_end() {
            // 在开始扫描下一个 token 之前，记录当前 token 的起始位置 (字节索引和列号)
            self.start = self.current;
            self.scan_token()?;
        }
        let eof_range = self.start..self.current;
        self.tokens.push(Token::new(
            TokenType::Eof, // EOF token
            eof_range,
            None,
        ));
        Ok(())
    }
    fn scan_token(&mut self) -> Result<(), LexerError> {
        self.start = self.current;
        let c = self.advance();
        match c {
            '(' => {
                self.add_token(TokenType::LeftParen, None);
                Ok(())
            }
            ')' => {
                self.add_token(TokenType::RightParen, None);
                Ok(())
            }
            '{' => {
                self.add_token(TokenType::LeftBrace, None);
                Ok(())
            }
            '}' => {
                self.add_token(TokenType::RightBrace, None);
                Ok(())
            }
            ',' => {
                self.add_token(TokenType::Comma, None);
                Ok(())
            }
            ';' => {
                self.add_token(TokenType::Semicolon, None);
                Ok(())
            }
            '~' => {
                self.add_token(TokenType::BitwiseNot, None);
                Ok(())
            }
            '+' => {
                self.add_token(TokenType::Add, None);
                Ok(())
            }
            '*' => {
                self.add_token(TokenType::Mul, None);
                Ok(())
            }
            '/' => {
                self.add_token(TokenType::Div, None);
                Ok(())
            }
            '%' => {
                self.add_token(TokenType::Remainder, None);
                Ok(())
            }
            '?' => {
                self.add_token(TokenType::Question, None);
                Ok(())
            }
            ':' => {
                self.add_token(TokenType::Colon, None);
                Ok(())
            }
            '&' => {
                if let Some(c) = self.peek() {
                    if c == '&' {
                        self.add_token(TokenType::And, None);
                        self.advance();
                    }
                }
                Ok(())
            }
            '|' => {
                if let Some(c) = self.peek() {
                    if c == '|' {
                        self.add_token(TokenType::Or, None);
                        self.advance();
                    }
                }
                Ok(())
            }

            '!' => {
                if self.match_char('=') {
                    self.add_token(TokenType::BangEqual, None);
                } else {
                    self.add_token(TokenType::Bang, None);
                }
                Ok(())
            }
            '<' => {
                if self.match_char('=') {
                    self.add_token(TokenType::LessEqual, None);
                } else {
                    self.add_token(TokenType::Less, None);
                }
                Ok(())
            }
            '>' => {
                if self.match_char('=') {
                    self.add_token(TokenType::GreaterEqual, None);
                } else {
                    self.add_token(TokenType::Greater, None);
                }
                Ok(())
            }

            '-' => {
                if self.match_char('-') {
                    self.add_token(TokenType::Decrement, None);
                } else {
                    self.add_token(TokenType::Minus, None);
                }
                Ok(())
            }
            '=' => {
                if self.match_char('=') {
                    self.add_token(TokenType::EqualEqual, None);
                } else {
                    self.add_token(TokenType::Equal, None);
                }
                Ok(())
            }

            '0'..='9' => {
                let mut is_valid_number = true;
                while let Some(next_char) = self.peek() {
                    if next_char.is_digit(10) {
                        self.advance(); // 继续扫描数字
                    } else if next_char.is_alphabetic() || next_char == '_' {
                        // 数字后面跟着字母或下划线是非法的，记录并停止扫描
                        is_valid_number = false;
                        break;
                    } else {
                        // 遇到非数字、非字母、非下划线的字符，数字字面量结束
                        break;
                    }
                }
                let lexeme_range = self.start..self.current; // 获取数字字面量的范围

                if is_valid_number {
                    let lexeme = &self.source[lexeme_range.clone()];
                    if let Ok(value) = lexeme.parse::<i64>() {
                        self.add_token(TokenType::LiteralInt, Some(Value::Int(value)));
                    } else {
                        return Err(LexerError {
                            message: format!(
                                "Integer literal '{}' is too large or invalid",
                                lexeme
                            ),
                        });
                    }
                    Ok(())
                } else {
                    // 5. 如果是非法的标识符（如 123a）
                    return Err(LexerError {
                        message: format!(
                            " '{}' , this identifier cannot start with a digit",
                            &self.source[lexeme_range]
                        ),
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
                let lexeme_range = self.start..self.current; // 获取标识符或关键字的范围
                let lexeme_str = &self.source[lexeme_range.clone()]; // 使用范围从源字符串中获取切片
                match lexeme_str {
                    "int" => self.add_token(TokenType::KeywordInt, None),
                    "return" => self.add_token(TokenType::KeywordReturn, None),
                    "void" => self.add_token(TokenType::KeywordVoid, None),
                    "if" => self.add_token(TokenType::KeywordIf, None),
                    "else" => self.add_token(TokenType::KeywordElse, None),
                    "for" => self.add_token(TokenType::KeywordFor, None),
                    "do" => self.add_token(TokenType::KeywordDo, None),
                    "while" => self.add_token(TokenType::KeywordWhile, None),
                    "break" => self.add_token(TokenType::KeywordBreak, None),
                    "continue" => self.add_token(TokenType::KeywordContinue, None),
                    _ => self.add_token(TokenType::Identifier, None),
                }
                Ok(())
            }
            _ if c.is_whitespace() => Ok(()),
            _ => {
                return Err(LexerError {
                    message: format!("Unexpected character: '{}'", c),
                });
            }
        }
    }
    fn add_token(&mut self, token_type: TokenType, literal: Option<Value>) {
        // 计算当前 token 的范围
        let lexeme_range = self.start..self.current;
        // 使用记录的起始列来创建 Token
        let token = Token::new(token_type, lexeme_range, literal);
        self.tokens.push(token);
    }

    //辅助方法
    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }
    // 移动到下一个字符，更新 current 和 column
    fn advance(&mut self) -> char {
        let c = self.source[self.current..].chars().next().unwrap();
        self.current += c.len_utf8(); // 按字节长度前进
        c
    }
    // 查看下一个字符但不移动
    fn peek(&self) -> Option<char> {
        self.source.chars().nth(self.current)
    }
    // fn peek_next(&self) -> Option<char> {
    //     let next_index = self.current + self.peek()?.len_utf8();
    //     self.source[next_index..].chars().next()
    // }
    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.peek() != Some(expected) {
            return false;
        }
        self.advance();
        true
    }
}

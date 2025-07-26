#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    Identifier,
    Number,
    // Keywords
    Int,
    Void,
    Return,
    // Single-character tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Semicolon,
    // End of File
    Eof,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub lexeme: String,
    pub type_: TokenType,
    // `value` 字段现在将被用来存储字面量的值
    pub value: Option<String>,
}

#[derive(Debug)]
pub struct Lexer {}

impl Lexer {
    pub fn new() -> Self {
        Lexer {}
    }

    pub fn lex(&self, input: &str) -> Result<Vec<Token>, String> {
        // 使用 Vec::with_capacity 可以略微提高性能，因为我们大概知道会有多少个 token
        let mut tokens = Vec::with_capacity(input.len() / 2);
        let mut chars = input.chars().peekable();

        while let Some(&c) = chars.peek() {
            match c {
                '(' | ')' | '{' | '}' | ';' => {
                    let type_ = match c {
                        '(' => TokenType::LeftParen,
                        ')' => TokenType::RightParen,
                        '{' => TokenType::LeftBrace,
                        '}' => TokenType::RightBrace,
                        ';' => TokenType::Semicolon,
                        _ => unreachable!(),
                    };
                    tokens.push(Token {
                        lexeme: c.to_string(),
                        type_,
                        value: None,
                    });
                    chars.next();
                }
                '0'..='9' => {
                    tokens.push(self.lex_number(&mut chars)?);
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    tokens.push(self.lex_identifier(&mut chars));
                }
                c if c.is_whitespace() => {
                    chars.next();
                }
                _ => {
                    return Err(format!("Unexpected character: {}", c));
                }
            }
        }

        tokens.push(Token {
            lexeme: "".to_string(),
            type_: TokenType::Eof,
            value: None,
        });

        Ok(tokens)
    }

    fn lex_number(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> Result<Token, String> {
        let mut number_str = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_digit(10) {
                number_str.push(c);
                chars.next();
            } else {
                break;
            }
        }

        // 检查数字后面的字符
        if let Some(&next_char) = chars.peek() {
            if next_char.is_alphabetic() {
                return Err(format!(
                    "Identifier cannot start with a number: '{}{}'",
                    number_str, next_char
                ));
            }
        }

        Ok(Token {
            lexeme: number_str.clone(),
            type_: TokenType::Number,
            value: Some(number_str),
        })
    }

    /// 解析一个标识符或关键字
    fn lex_identifier(&self, chars: &mut std::iter::Peekable<std::str::Chars>) -> Token {
        let mut identifier = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                identifier.push(c);
                chars.next();
            } else {
                break;
            }
        }

        let type_ = match identifier.as_str() {
            "int" => TokenType::Int,
            "void" => TokenType::Void,
            "return" => TokenType::Return,
            _ => TokenType::Identifier,
        };

        // 根据类型决定如何构造 Token
        if type_ == TokenType::Identifier {
            Token {
                type_,
                lexeme: identifier.clone(),
                value: Some(identifier),
            }
        } else {
            Token {
                type_,
                lexeme: identifier,
                value: None,
            }
        }
    }
}

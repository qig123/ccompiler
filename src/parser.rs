use crate::{
    error::ParserError,
    expr::{Expr, Function, LiteralExpr, Stmt},
    lexer::{Token, token::TokenType},
    value::Value,
};

pub struct Parser<'a> {
    tokens: Vec<Token>,
    current: usize,
    source: &'a str,
}
impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, source: &'a str) -> Self {
        Parser {
            tokens,
            current: 0,
            source,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Function>, ParserError> {
        let mut functions = Vec::new();
        while !self.is_at_end() {
            let f = self.parse_function();
            match f {
                Ok(function) => functions.push(function),
                Err(e) => return Err(e),
            }
        }
        Ok(functions)
    }
    fn parse_function(&mut self) -> Result<Function, ParserError> {
        let _name = self.consume(TokenType::KeywordInt, "expected int")?;
        if self.match_token(&[TokenType::Identifier]) {
            let identifier = self.previous().clone();
            self.consume(TokenType::LeftParen, "Expected '(' after function name")?;
            self.consume(TokenType::KeywordVoid, "Expected 'void' ")?;
            self.consume(
                TokenType::RightParen,
                "Expected ')' to end function parameters",
            )?;
            let body = self.parse_body()?;
            Ok(Function {
                name: identifier,
                body,
            })
        } else {
            Err(ParserError {
                message: "Expected function name after 'int'".to_string(),
                line: self.peek().line,
                column: self.peek().column,
            })
        }
    }
    fn parse_body(&mut self) -> Result<Vec<Stmt>, ParserError> {
        let mut body = Vec::new();
        self.consume(TokenType::LeftBrace, "Expected '{' to start function body")?;
        let stmt = self.parse_statement()?;
        body.push(stmt); // 这里可以扩展为解析多个语句
        self.consume(TokenType::RightBrace, "Expected '}' to end function body")?;
        Ok(body)
    }
    fn parse_statement(&mut self) -> Result<Stmt, ParserError> {
        let keyword = self
            .consume(TokenType::KeywordReturn, "Expected 'return' keyword")
            .cloned()?;
        let value = self.parse_expression()?;
        self.consume(TokenType::Semicolon, "Expected ';' after return statement")?;
        Ok(Stmt::Return {
            keyword: keyword,
            value: Some(value),
        })
    }
    fn parse_expression(&mut self) -> Result<Expr, ParserError> {
        //这里表达式暂时只有一种, 就是数字常量int
        let token = self.previous().clone();
        // println!("Parsing expression: {:?}", token);
        if self.match_token(&[TokenType::LiteralInt]) {
            let prev = self.previous();
            match &prev.literal {
                Some(value) => {
                    let Value::Int(i) = value;
                    Ok(Expr::Literal(LiteralExpr::Integer(i.clone())))
                }
                None => {
                    unreachable!("Literal token should have a value");
                }
            }
        } else {
            Err(ParserError {
                message: "Expected an integer literal".to_string(),
                line: token.line,
                column: token.column,
            })
        }
    }

    fn match_token(&mut self, types: &[TokenType]) -> bool {
        // println!("Matching tokens: {:?}", types);
        for t in types {
            // println!("Matched token1111: {:?}", t);
            if self.check(t.clone()) {
                // println!("Matched token: {:?}", t);
                self.advance();
                return true;
            }
        }
        false
    }

    fn consume(&mut self, expected: TokenType, message: &str) -> Result<&Token, ParserError> {
        let current_token = self.peek();
        if self.check(expected) {
            Ok(self.advance())
        } else {
            // 获取上一个token的位置，这个位置更准确地表示了错误发生的地方
            let prev_token = self.previous();
            // 构造更有信息量的错误消息
            let error_message = format!(
                "{} (found '{}' instead)",
                message,
                current_token.get_lexeme(self.source)
            );
            Err(ParserError {
                message: error_message,
                line: prev_token.line,
                column: prev_token.column,
            })
        }
    }

    //基础方法
    fn check(&self, t: TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.peek().token_type == t
    }
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
    fn is_at_end(&self) -> bool {
        let p = self.peek();
        p.token_type == TokenType::Eof
    }
}

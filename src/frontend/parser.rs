use std::iter::Peekable;
use std::vec::IntoIter;

use crate::frontend::c_ast::{Expression, Function, Program, Statement, UnaryOp};
use crate::frontend::lexer::{Token, TokenType};

#[derive(Debug)]
pub struct Parser {
    tokens: Peekable<IntoIter<Token>>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens: tokens.into_iter().peekable(),
        }
    }

    /// 主入口点。它解析整个记号流。
    pub fn parse(mut self) -> Result<Program, String> {
        let program = self.parse_program()?;

        // 解析完程序后，我们期望流中只剩下 EOF 记号。
        match self.tokens.next() {
            Some(token) if token.type_ == TokenType::Eof => Ok(program), // 找到了预期的 EOF，解析成功！
            Some(token) => Err(format!(
                "Unexpected token {:?} found after the program has been parsed.",
                token.type_
            )),
            None => Err("Expected EOF token, but the token stream ended prematurely.".to_string()),
        }
    }

    /// <program> ::= <function>
    fn parse_program(&mut self) -> Result<Program, String> {
        // 在未来的扩展中，这里可以是一个循环来解析多个函数
        let function = self.parse_function()?;
        Ok(Program {
            functions: vec![function],
        })
    }

    /// <function> ::= "int" <identifier> "(" "void" ")" "{" <statement> "}"
    fn parse_function(&mut self) -> Result<Function, String> {
        self.consume(TokenType::Int)?;
        let name_token = self.consume(TokenType::Identifier)?;
        self.consume(TokenType::LeftParen)?;
        self.consume(TokenType::Void)?;
        self.consume(TokenType::RightParen)?;
        self.consume(TokenType::LeftBrace)?;
        let statement = self.parse_statement()?;
        self.consume(TokenType::RightBrace)?;

        let name = name_token
            .value
            .ok_or_else(|| "Identifier token is missing a value".to_string())?;

        Ok(Function {
            name,
            parameters: Vec::new(),
            body: vec![statement],
        })
    }

    /// <statement> ::= "return" <exp> ";"
    fn parse_statement(&mut self) -> Result<Statement, String> {
        self.consume(TokenType::Return)?;
        let expression = self.parse_expression()?;
        self.consume(TokenType::Semicolon)?;
        Ok(Statement::Return(expression))
    }

    /// <exp> ::= <int> | <unop> <exp> | "(" <exp> ")"
    fn parse_expression(&mut self) -> Result<Expression, String> {
        // 使用 peek_type 来决定应用哪条语法规则
        let next_token_type = self.peek_type()?.clone();
        match next_token_type {
            // 规则 1: <int>
            TokenType::Number => {
                let num_token = self.consume(TokenType::Number)?;
                let value = num_token
                    .lexeme
                    .parse::<i64>()
                    .map_err(|e| format!("Failed to parse number '{}': {}", num_token.lexeme, e))?;
                Ok(Expression::Constant(value))
            }

            // 规则 2: <unop> <exp>
            TokenType::Negate | TokenType::Complement => {
                let op_token = self.tokens.next().unwrap();
                let op = match op_token.type_ {
                    TokenType::Negate => UnaryOp::Negate,
                    TokenType::Complement => UnaryOp::Complement,
                    _ => unreachable!(),
                };
                let right_exp = self.parse_expression()?;
                Ok(Expression::Unary {
                    op,
                    exp: Box::new(right_exp),
                })
            }

            // 规则 3: "(" <exp> ")"
            TokenType::LeftParen => {
                self.consume(TokenType::LeftParen)?;
                let inner_exp = self.parse_expression()?;
                self.consume(TokenType::RightParen)?;
                Ok(inner_exp)
            }
            _ => Err(format!(
                "Unexpected token {:?}, expected an expression (number, unary operator, or '(').",
                next_token_type
            )),
        }
    }

    fn consume(&mut self, expected: TokenType) -> Result<Token, String> {
        match self.tokens.next() {
            Some(token) if token.type_ == TokenType::Eof => Err(format!(
                "Expected token {:?}, but found end of file instead.",
                expected
            )),
            Some(token) if token.type_ == expected => Ok(token),
            Some(token) => Err(format!(
                "Expected token {:?}, but found {:?}",
                expected, token.type_
            )),
            None => Err(format!(
                "Expected token {:?}, but the token stream was empty.",
                expected
            )),
        }
    }
    fn peek_type(&mut self) -> Result<&TokenType, String> {
        match self.tokens.peek() {
            Some(token) => Ok(&token.type_),
            None => Err("Expected a token but found end of file.".to_string()),
        }
    }
    #[allow(dead_code)]
    fn check(&mut self, expected: &TokenType) -> bool {
        match self.tokens.peek() {
            Some(token) if &token.type_ == expected => true,
            _ => false,
        }
    }
    #[allow(dead_code)]
    fn match_any(&mut self, types: &[TokenType]) -> bool {
        match self.tokens.peek() {
            Some(token) if types.contains(&token.type_) => {
                self.tokens.next(); // 匹配成功，消耗 token
                true
            }
            _ => false,
        }
    }
}

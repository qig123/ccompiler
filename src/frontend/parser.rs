use std::iter::Peekable;
use std::vec::IntoIter;

use crate::frontend::c_ast::{Expression, Function, Program, Statement};
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

    /// <exp> ::= <int>
    fn parse_expression(&mut self) -> Result<Expression, String> {
        let int_token = self.consume(TokenType::Number)?;

        let value_str = int_token
            .value
            .ok_or_else(|| "Number token is missing a value".to_string())?;

        let value = value_str
            .parse::<i64>()
            .map_err(|e| format!("Failed to parse number '{}': {}", value_str, e))?;

        Ok(Expression::Constant(value))
    }

    /// 消耗并返回一个期望类型的记号，否则返回错误。
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
}

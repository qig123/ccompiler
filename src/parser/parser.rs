use crate::{
    error::ParserError,
    lexer::{Token, token::TokenType},
    parser::c_ast::{BinaryOperator, Expr, Function, LiteralExpr, Program, Stmt},
    types::types::Value,
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

    pub fn parse(&mut self) -> Result<Program, ParserError> {
        let mut functions = Vec::new();
        while !self.is_at_end() {
            let f = self.parse_function();
            match f {
                Ok(function) => functions.push(function),
                Err(e) => return Err(e),
            }
        }
        Ok(Program {
            functions: functions,
        })
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
        self.parse_precedence(0) // 从最低优先级开始
    }
    //<exp>       ::= <factor> | <exp> <binop> <exp>  // 二元表达式采用优先级爬升
    fn parse_precedence(&mut self, min_precedence: u8) -> Result<Expr, ParserError> {
        let mut left = self.parser_factor()?; // 先解析基本表达式（数字、括号、一元操作等）

        loop {
            let token = self.peek();

            // 检查是否是二元运算符
            let op = match token.token_type {
                TokenType::Add => BinaryOperator::Add,
                TokenType::Minus => BinaryOperator::Subtract,
                TokenType::Mul => BinaryOperator::Multiply,
                TokenType::Div => BinaryOperator::Divide,
                TokenType::Remainder => BinaryOperator::Remainder,
                TokenType::And => BinaryOperator::And,
                TokenType::Or => BinaryOperator::Or,
                TokenType::EqualEqual => BinaryOperator::EqualEqual,
                TokenType::BangEqual => BinaryOperator::BangEqual,
                TokenType::Less => BinaryOperator::Less,
                TokenType::LessEqual => BinaryOperator::LessEqual,
                TokenType::Greater => BinaryOperator::Greater,
                TokenType::GreaterEqual => BinaryOperator::GreaterEqual,
                _ => break, // 不是二元运算符，结束循环
            };

            // 检查当前运算符优先级是否足够高
            if op.precedence() < min_precedence {
                break;
            }

            self.advance(); // 消耗运算符token

            // 递归解析右侧表达式，处理更高优先级的运算符
            let right = self.parse_precedence(op.precedence() + 1)?;

            left = Expr::Binary {
                operator: op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    //<factor>    ::= <int> | <unop> <factor> | "(" <exp> ")"  // 一元/括号表达式仍用递归下降
    fn parser_factor(&mut self) -> Result<Expr, ParserError> {
        if self.match_token(&[TokenType::LiteralInt]) {
            let prev = self.previous();
            match &prev.literal {
                Some(Value::Int(i)) => Ok(Expr::Literal(LiteralExpr::Integer(*i))),
                None => {
                    Err(ParserError {
                        message: format!("Internal error: LiteralInt token missing value at "), // Assuming Token has position
                    })
                }
            }
        } else if self.match_token(&[TokenType::Minus, TokenType::BitwiseNot, TokenType::Bang]) {
            // Assuming TokenType::Minus and TokenType::Tilde exist
            let operator_token = self.previous().clone(); // Keep the operator token

            // Recursively parse the <exp> that follows the operator
            let right_expr = self.parser_factor()?;

            Ok(Expr::Unary {
                operator: operator_token,
                right: Box::new(right_expr), // Box the child expression
            })

        // Check for grouping "(" <exp> ")"
        } else if self.match_token(&[TokenType::LeftParen]) {
            // Recursively parse the <exp> inside the parentheses
            let inner_expr = self.parse_expression()?;

            // Consume the closing parenthesis
            self.consume(TokenType::RightParen, "Expected ')' after expression")?;

            Ok(Expr::Grouping {
                expression: Box::new(inner_expr), // Box the child expression
            })

        // If none of the above match, it's a syntax error according to this grammar
        } else {
            Err(ParserError {
                message: format!(
                    "Expected an expression (integer, unary op, or grouping), but found '{}' ",
                    self.peek().get_lexeme(self.source), // Get lexeme for error message
                ),
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
            // 构造更有信息量的错误消息
            let error_message = format!(
                "{} (found '{}' instead)",
                message,
                current_token.get_lexeme(self.source)
            );
            Err(ParserError {
                message: error_message,
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

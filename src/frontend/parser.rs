// src/frontend/parser.rs

use std::iter::Peekable;
use std::vec::IntoIter;

use crate::frontend::c_ast::{
    BinaryOp, BlockItem, Declaration, Expression, Function, Program, Statement, UnaryOp,
};
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

    // --- 主入口和顶层解析函数 (这部分和您原来的一样，保持不变) ---

    /// 主入口点。它解析整个记号流。
    pub fn parse(mut self) -> Result<Program, String> {
        let program = self.parse_program()?;
        self.consume(TokenType::Eof)?;
        Ok(program)
    }

    /// <program> ::= <function>
    fn parse_program(&mut self) -> Result<Program, String> {
        let function = self.parse_function()?;
        Ok(Program {
            functions: vec![function],
        })
    }

    /// <function> ::= "int" <identifier> "(" "void" ")" "{" {<block-item>} "}"
    fn parse_function(&mut self) -> Result<Function, String> {
        self.consume(TokenType::Int)?;
        let name_token = self.consume(TokenType::Identifier)?;
        let name = name_token
            .value
            .ok_or_else(|| "标识符记号缺少值".to_string())?;

        self.consume(TokenType::LeftParen)?;
        self.consume(TokenType::Void)?;
        self.consume(TokenType::RightParen)?;
        self.consume(TokenType::LeftBrace)?;

        let mut body = Vec::new();
        while !self.check(TokenType::RightBrace) {
            body.push(self.parse_block_item()?);
        }

        self.consume(TokenType::RightBrace)?;

        Ok(Function {
            name,
            parameters: Vec::new(),
            body,
        })
    }

    //<block-item> ::= <statement> | <declaration>
    fn parse_block_item(&mut self) -> Result<BlockItem, String> {
        if self.check(TokenType::Int) {
            self.parse_declaration().map(BlockItem::D)
        } else {
            self.parse_statement().map(BlockItem::S)
        }
    }

    //<declaration> ::= "int" <identifier> ["=" <exp>] ";"
    fn parse_declaration(&mut self) -> Result<Declaration, String> {
        self.consume(TokenType::Int)?;
        let id = self.consume(TokenType::Identifier)?;
        let name = id.value.ok_or("标识符缺少名称")?;

        let init = if self.match_token(TokenType::Assignment) {
            Some(Box::new(self.parse_exp(0)?))
        } else {
            None
        };

        self.consume(TokenType::Semicolon)?;
        Ok(Declaration { name, init })
    }

    /// <statement> ::= "return" <exp> ";" | <exp> ";" | ";"|"if" "(" <exp> ")" <statement> ["else" <statement>]
    fn parse_statement(&mut self) -> Result<Statement, String> {
        if self.match_token(TokenType::Return) {
            let expr = self.parse_exp(0)?;
            self.consume(TokenType::Semicolon)?;
            Ok(Statement::Return(expr))
        } else if self.match_token(TokenType::Semicolon) {
            Ok(Statement::Null)
        } else if self.match_token(TokenType::If) {
            self.consume(TokenType::LeftParen)?;
            let c = self.parse_exp(0)?;
            self.consume(TokenType::RightParen)?;
            let then_stmt = self.parse_statement()?;
            let suc = self.match_token(TokenType::Else);
            let else_stmt;
            if suc {
                else_stmt = Some(Box::new(self.parse_statement()?));
            } else {
                else_stmt = None;
            }
            Ok(Statement::If {
                condition: c,
                then_stmt: Box::new(then_stmt),
                else_stmt: else_stmt,
            })
        } else {
            let expr = self.parse_exp(0)?;
            self.consume(TokenType::Semicolon)?;
            Ok(Statement::Expression(expr))
        }
    }

    // --- 表达式解析 (Pratt Parser) ---
    /// min_prec: 当前上下文的最小优先级。
    fn parse_exp(&mut self, min_prec: i32) -> Result<Expression, String> {
        let mut left = self.parse_prefix()?;

        loop {
            let next_token = match self.tokens.peek().cloned() {
                Some(tok) => tok,
                None => break, // Token 流结束
            };

            // 获取中缀运算符的优先级，如果不是运算符或优先级太低，则停止循环。
            let op_prec = match self.get_infix_precedence(&next_token.type_) {
                Some(prec) if prec >= min_prec => prec,
                _ => break,
            };

            // 消耗掉运算符 token
            let op_token = self.tokens.next().unwrap();

            left = match op_token.type_ {
                // 特殊情况：三元运算符
                TokenType::QuestionMark => {
                    // 'left' 是我们的 condition
                    // 解析 then 分支
                    let then_exp = self.parse_exp(0)?; // 在 '?' 和 ':' 之间，优先级重置
                    // 消耗 ':'
                    self.consume(TokenType::Colon)?;
                    // 解析 else 分支。三元运算是右结合的，所以右侧的优先级是 op_prec
                    let else_exp = self.parse_exp(op_prec)?;

                    Expression::Conditional {
                        condition: Box::new(left),
                        left: Box::new(then_exp),
                        right: Box::new(else_exp),
                    }
                }

                // 特殊情况：赋值运算符 (右结合)
                TokenType::Assignment => {
                    // 右结合运算符的右侧表达式应该以其自身的优先级来解析
                    let right = self.parse_exp(op_prec)?;
                    Expression::Assignment {
                        left: Box::new(left),
                        right: Box::new(right),
                    }
                }

                // 通用情况：所有左结合的二元运算符
                _ => {
                    let bin_op = match op_token.type_ {
                        TokenType::Add => BinaryOp::Add,
                        TokenType::Negate => BinaryOp::Subtract, // 中缀 '-'
                        TokenType::Mul => BinaryOp::Multiply,
                        TokenType::Div => BinaryOp::Divide,
                        TokenType::Remainder => BinaryOp::Remainder,
                        TokenType::And => BinaryOp::And,
                        TokenType::Or => BinaryOp::Or,
                        TokenType::BangEqual => BinaryOp::BangEqual,
                        TokenType::EqualEqual => BinaryOp::EqualEqual,
                        TokenType::Greater => BinaryOp::Greater,
                        TokenType::GreaterEqual => BinaryOp::GreaterEqual,
                        TokenType::Less => BinaryOp::Less,
                        TokenType::LessEqual => BinaryOp::LessEqual,
                        _ => unreachable!("已在 get_infix_precedence 中过滤"),
                    };

                    // 左结合运算符的右侧表达式优先级要高一级
                    let right = self.parse_exp(op_prec + 1)?;
                    Expression::Binary {
                        op: bin_op,
                        left: Box::new(left),
                        right: Box::new(right),
                    }
                }
            };
        }

        Ok(left)
    }

    /// 解析前缀部分：数字、变量、括号表达式或一元运算符。
    fn parse_prefix(&mut self) -> Result<Expression, String> {
        let next_token = self.tokens.next().ok_or("预期有表达式，但输入已结束")?;

        match next_token.type_ {
            TokenType::Number => {
                let value = next_token
                    .lexeme
                    .parse::<i64>()
                    .map_err(|e| e.to_string())?;
                Ok(Expression::Constant(value))
            }
            TokenType::Identifier => {
                let name = next_token.value.ok_or("标识符缺少名称")?;
                Ok(Expression::Var(name))
            }
            TokenType::LeftParen => {
                let exp = self.parse_exp(0)?; // 括号内重置优先级。
                self.consume(TokenType::RightParen)?;
                Ok(exp)
            }
            // 处理一元运算符
            TokenType::Negate | TokenType::Complement | TokenType::Bang => {
                let op = match next_token.type_ {
                    TokenType::Negate => UnaryOp::Negate,
                    TokenType::Complement => UnaryOp::Complement,
                    TokenType::Bang => UnaryOp::Not,
                    _ => unreachable!(),
                };
                // 对于一元运算符，它的右侧表达式应该以其自身的优先级来解析。
                let ((), op_prec) = self.get_prefix_precedence(&next_token.type_).unwrap();
                let right_exp = self.parse_exp(op_prec)?;
                Ok(Expression::Unary {
                    op,
                    exp: Box::new(right_exp),
                })
            }
            _ => Err(format!(
                "预期是表达式的前缀部分，但得到 {:?}",
                next_token.type_
            )),
        }
    }

    /// 获取中缀运算符的优先级。如果 token 不是中缀运算符，返回 None。
    fn get_infix_precedence(&self, typ: &TokenType) -> Option<i32> {
        match typ {
            TokenType::Assignment => Some(10),
            TokenType::QuestionMark => Some(15),
            TokenType::Or => Some(20),
            TokenType::And => Some(30),
            TokenType::EqualEqual | TokenType::BangEqual => Some(40),
            TokenType::Greater
            | TokenType::GreaterEqual
            | TokenType::Less
            | TokenType::LessEqual => Some(50),
            TokenType::Add | TokenType::Negate => Some(60), // Negate 在中缀位置代表减法
            TokenType::Mul | TokenType::Div | TokenType::Remainder => Some(70),
            _ => None,
        }
    }

    /// 获取前缀（一元）运算符的优先级。
    fn get_prefix_precedence(&self, typ: &TokenType) -> Option<((), i32)> {
        match typ {
            TokenType::Negate | TokenType::Complement | TokenType::Bang => Some(((), 80)), // 一元运算符优先级很高
            _ => None,
        }
    }

    // --- 工具函数 (保持不变) ---

    fn consume(&mut self, expected: TokenType) -> Result<Token, String> {
        match self.tokens.next() {
            Some(token) if token.type_ == expected => Ok(token),
            Some(token) => Err(format!("预期是 {:?}，但得到 {:?}", expected, token.type_)),
            None => Err(format!("预期是 {:?}，但输入已结束", expected)),
        }
    }

    fn check(&mut self, expected: TokenType) -> bool {
        self.tokens.peek().map_or(false, |t| t.type_ == expected)
    }

    fn match_token(&mut self, expected: TokenType) -> bool {
        if self.check(expected) {
            self.tokens.next();
            true
        } else {
            false
        }
    }
}

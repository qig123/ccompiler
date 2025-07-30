// src/frontend/parser.rs

use std::iter::Peekable;
use std::vec::IntoIter;

use crate::frontend::c_ast::{
    BinaryOp, Block, BlockItem, Declaration, Expression, ForInit, FunDecl, Program, Statement,
    UnaryOp, VarDecl,
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

    // --- 主入口和顶层解析函数 ---

    /// 主入口点。它解析整个记号流。
    pub fn parse(mut self) -> Result<Program, String> {
        self.parse_program()
    }

    /// <program> ::= {<function-declaration>}
    /// 根据语法，一个程序由一系列函数声明/定义组成。
    fn parse_program(&mut self) -> Result<Program, String> {
        let mut functions = Vec::new();
        while !self.match_token(TokenType::Eof) {
            // 解析一个完整的函数声明/定义
            // 注意：这里的语法假设顶层只有函数，没有全局变量。
            self.consume(TokenType::Int)?;
            let name_token = self.consume(TokenType::Identifier)?;
            let name = name_token
                .value
                .ok_or_else(|| "Identifier token is missing a value".to_string())?;
            // 已经消耗了 "int <identifier>"，现在解析函数的剩余部分
            let func_decl = self.parse_function_remainder(name)?;
            functions.push(func_decl);
        }
        Ok(Program { functions })
    }

    /// <declaration> ::= <variable-declaration> | <function-declaration>
    /// 解析一个声明。它会消耗掉 "int" 和标识符，然后根据下一个记号决定是变量还是函数。
    fn parse_declaration(&mut self) -> Result<Declaration, String> {
        self.consume(TokenType::Int)?;
        let name_token = self.consume(TokenType::Identifier)?;
        let name = name_token
            .value
            .ok_or_else(|| "Identifier token is missing a value".to_string())?;

        // 向前查看一个记号来决定路径
        // 如果是 '('，则为函数声明
        // 否则，为变量声明
        if self.check(TokenType::LeftParen) {
            let func_decl = self.parse_function_remainder(name)?;
            Ok(Declaration::Fun(func_decl))
        } else {
            let var_decl = self.parse_var_remainder(name)?;
            Ok(Declaration::Variable(var_decl))
        }
    }

    /// <variable-declaration> ::= "int" <identifier> ["=" <exp>] ";"
    /// 注意: "int" <identifier> 已被调用者消耗。此函数解析变量声明的剩余部分。
    fn parse_var_remainder(&mut self, name: String) -> Result<VarDecl, String> {
        let init = if self.match_token(TokenType::Assignment) {
            Some(self.parse_exp(0)?)
        } else {
            None
        };
        self.consume(TokenType::Semicolon)?;
        Ok(VarDecl { name, init })
    }

    /// <function-declaration> ::= "int" <identifier> "(" <param-list> ")" (<block> | ";")
    /// 注意: "int" <identifier> 已被调用者消耗。此函数解析函数声明的剩余部分。
    fn parse_function_remainder(&mut self, name: String) -> Result<FunDecl, String> {
        self.consume(TokenType::LeftParen)?;
        let params = self.parse_func_params()?;
        self.consume(TokenType::RightParen)?;

        // 如果下一个是分号，则是函数原型声明；否则应该是函数体。
        if self.match_token(TokenType::Semicolon) {
            Ok(FunDecl {
                name,
                parameters: params,
                body: None,
            })
        } else {
            // 期待一个代码块作为函数体
            let body = self.parse_block()?;
            Ok(FunDecl {
                name,
                parameters: params, // BUG修复：之前这里是 Vec::new()
                body: Some(body),
            })
        }
    }

    /// <param-list> ::= "void" | "int" <identifier> {"," "int" <identifier>}
    fn parse_func_params(&mut self) -> Result<Vec<String>, String> {
        if self.match_token(TokenType::Void) {
            return Ok(Vec::new());
        }

        // 如果不是 void，那么必须至少有一个 "int <identifier>"
        // 检查是否是 ')'，如果是，说明是 int foo() 这种情况，参数列表为空
        if self.check(TokenType::RightParen) {
            return Ok(Vec::new());
        }

        self.consume(TokenType::Int)?;
        let first_param = self.consume(TokenType::Identifier)?;
        let mut params = vec![first_param.value.unwrap()];

        while self.match_token(TokenType::Comma) {
            self.consume(TokenType::Int)?;
            let next_param = self.consume(TokenType::Identifier)?;
            params.push(next_param.value.unwrap());
        }

        Ok(params)
    }

    /// <block> ::= "{" {<block-item>} "}"
    /// 解析一个完整代码块，包括 '{' 和 '}'。
    fn parse_block(&mut self) -> Result<Block, String> {
        self.consume(TokenType::LeftBrace)?;
        let mut items = Vec::new();
        while !self.check(TokenType::RightBrace) {
            items.push(self.parse_block_item()?);
        }
        self.consume(TokenType::RightBrace)?;
        Ok(Block(items))
    }

    /// <block-item> ::= <statement> | <declaration>
    fn parse_block_item(&mut self) -> Result<BlockItem, String> {
        if self.check(TokenType::Int) {
            self.parse_declaration().map(BlockItem::D)
        } else {
            self.parse_statement().map(BlockItem::S)
        }
    }

    /// <for-init> ::= <variable-declaration> | [<exp>] ";"
    fn parse_forinit(&mut self) -> Result<ForInit, String> {
        if self.check(TokenType::Int) {
            // 这是一个内联变量声明
            let decl = self.parse_declaration()?;
            match decl {
                Declaration::Variable(var_decl) => Ok(ForInit::InitDecl(var_decl)),
                // for循环的init部分不能是函数声明
                Declaration::Fun(_) => {
                    Err("Function declaration is not allowed in for-init".to_string())
                }
            }
        } else if self.match_token(TokenType::Semicolon) {
            Ok(ForInit::InitExp(None))
        } else {
            let e = self.parse_exp(0)?;
            self.consume(TokenType::Semicolon)?;
            Ok(ForInit::InitExp(Some(e)))
        }
    }

    /// <statement> ::= ...
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
            let else_stmt = if self.match_token(TokenType::Else) {
                Some(Box::new(self.parse_statement()?))
            } else {
                None
            };
            Ok(Statement::If {
                condition: c,
                then_stmt: Box::new(then_stmt),
                else_stmt,
            })
        } else if self.check(TokenType::LeftBrace) {
            // 修正: parse_block 现在自己处理 '{'，所以这里只检查，不消耗
            let b = self.parse_block()?;
            Ok(Statement::Compound(b))
        } else if self.match_token(TokenType::Break) {
            self.consume(TokenType::Semicolon)?;
            Ok(Statement::Break("fakelabel".to_string()))
        } else if self.match_token(TokenType::Continue) {
            self.consume(TokenType::Semicolon)?;
            Ok(Statement::Continue("fakelabel".to_string()))
        } else if self.match_token(TokenType::While) {
            self.consume(TokenType::LeftParen)?;
            let c = self.parse_exp(0)?;
            self.consume(TokenType::RightParen)?;
            let body = self.parse_statement()?;
            Ok(Statement::While {
                condition: c,
                body: Box::new(body),
                label: None,
            })
        } else if self.match_token(TokenType::Do) {
            let body = self.parse_statement()?;
            self.consume(TokenType::While)?;
            self.consume(TokenType::LeftParen)?;
            let c = self.parse_exp(0)?;
            self.consume(TokenType::RightParen)?;
            self.consume(TokenType::Semicolon)?;
            Ok(Statement::DoWhile {
                body: Box::new(body),
                condition: c,
                label: None,
            })
        } else if self.match_token(TokenType::For) {
            self.consume(TokenType::LeftParen)?;
            let init = self.parse_forinit()?;
            let condition = if self.match_token(TokenType::Semicolon) {
                None
            } else {
                let cond = self.parse_exp(0)?;
                self.consume(TokenType::Semicolon)?;
                Some(cond)
            };
            let post = if self.match_token(TokenType::RightParen) {
                None
            } else {
                let p = self.parse_exp(0)?;
                self.consume(TokenType::RightParen)?;
                Some(p)
            };
            let body = self.parse_statement()?;

            Ok(Statement::For {
                init,
                condition,
                post,
                body: Box::new(body),
                label: None,
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
            // 通过 peek 获取下一个 token 的类型，但**不**持有对它的引用，
            // 以避免在调用 self 的其他方法时出现借用冲突。
            // TokenType 应该是 Copy 的，所以这里只是一个廉价的拷贝。
            let next_token_type = if let Some(token) = self.tokens.peek() {
                token.type_.clone()
            } else {
                break; // Token 流结束
            };

            // 获取中缀运算符的优先级，如果不是运算符或优先级太低，则停止循环。
            let op_prec = match self.get_infix_precedence(&next_token_type) {
                Some(prec) if prec >= min_prec => prec,
                _ => break,
            };

            // 消耗掉运算符 token
            let op_token = self.tokens.next().unwrap();

            left = match op_token.type_ {
                // 特殊情况：三元运算符
                TokenType::QuestionMark => {
                    let then_exp = self.parse_exp(0)?;
                    self.consume(TokenType::Colon)?;
                    let else_exp = self.parse_exp(op_prec)?;
                    Expression::Conditional {
                        condition: Box::new(left),
                        left: Box::new(then_exp),
                        right: Box::new(else_exp),
                    }
                }

                // 特殊情况：赋值运算符 (右结合)
                TokenType::Assignment => {
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
                        TokenType::Negate => BinaryOp::Subtract,
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
                        _ => unreachable!("Already filtered by get_infix_precedence"),
                    };

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

    /// <argument-list> ::= <exp> {"," <exp>}
    fn parse_argument(&mut self) -> Result<Vec<Expression>, String> {
        let mut argument_list = Vec::new();
        if self.check(TokenType::RightParen) {
            return Ok(Vec::new());
        }

        loop {
            let e = self.parse_exp(0)?;
            argument_list.push(e);
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(argument_list)
    }

    /// <factor> ::= <int> | <identifier> | <unop> <factor> | "(" <exp> ")" | <identifier> "(" [<argument-list>] ")"
    fn parse_prefix(&mut self) -> Result<Expression, String> {
        let next_token = self
            .tokens
            .next()
            .ok_or("Expected an expression, but found end of input")?;

        match next_token.type_ {
            TokenType::Number => {
                let value = next_token
                    .lexeme
                    .parse::<i64>()
                    .map_err(|e| e.to_string())?;
                Ok(Expression::Constant(value))
            }
            TokenType::Identifier => {
                let name = next_token.value.ok_or("Identifier is missing a name")?;
                if self.match_token(TokenType::LeftParen) {
                    let exp_vec = self.parse_argument()?;
                    self.consume(TokenType::RightParen)?;
                    Ok(Expression::FuncCall {
                        name,
                        args: exp_vec,
                    })
                } else {
                    Ok(Expression::Var(name))
                }
            }
            TokenType::LeftParen => {
                let exp = self.parse_exp(0)?;
                self.consume(TokenType::RightParen)?;
                Ok(exp)
            }
            TokenType::Negate | TokenType::Complement | TokenType::Bang => {
                let op = match next_token.type_ {
                    TokenType::Negate => UnaryOp::Negate,
                    TokenType::Complement => UnaryOp::Complement,
                    TokenType::Bang => UnaryOp::Not,
                    _ => unreachable!(),
                };
                let ((), op_prec) = self.get_prefix_precedence(&next_token.type_).unwrap();
                let right_exp = self.parse_exp(op_prec)?;
                Ok(Expression::Unary {
                    op,
                    exp: Box::new(right_exp),
                })
            }
            _ => Err(format!(
                "Expected expression prefix, but got {:?}",
                next_token.type_
            )),
        }
    }

    /// 获取中缀运算符的优先级。
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
            TokenType::Add | TokenType::Negate => Some(60),
            TokenType::Mul | TokenType::Div | TokenType::Remainder => Some(70),
            _ => None,
        }
    }

    /// 获取前缀（一元）运算符的优先级。
    fn get_prefix_precedence(&self, typ: &TokenType) -> Option<((), i32)> {
        match typ {
            TokenType::Negate | TokenType::Complement | TokenType::Bang => Some(((), 80)),
            _ => None,
        }
    }

    // --- 工具函数 ---

    fn consume(&mut self, expected: TokenType) -> Result<Token, String> {
        match self.tokens.next() {
            Some(token) if token.type_ == expected => Ok(token),
            Some(token) => Err(format!(
                "Expected {:?}, but got {:?}",
                expected, token.type_
            )),
            None => Err(format!("Expected {:?}, but input ended", expected)),
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

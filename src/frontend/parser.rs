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

    // --- 主入口和顶层解析函数 (这部分和您原来的一样，保持不变) ---

    /// 主入口点。它解析整个记号流。
    pub fn parse(mut self) -> Result<Program, String> {
        let program = self.parse_program()?;
        Ok(program)
    }

    /// <program> ::= {<function-declaration>}
    fn parse_program(&mut self) -> Result<Program, String> {
        let mut fs = Vec::new();
        while !self.match_token(TokenType::Eof) {
            self.consume(TokenType::Int)?;
            let name_token = self.consume(TokenType::Identifier)?;
            let name = name_token
                .value
                .ok_or_else(|| "标识符记号缺少值".to_string())?;
            let func_decl = self.parse_function_decl(name)?;
            fs.push(func_decl);
        }
        Ok(Program { functions: fs })
    }
    //<declaration> ::= <variable-declaration> | <function-declaration>
    fn parse_declaration(&mut self) -> Result<Declaration, String> {
        self.consume(TokenType::Int)?;
        let name_token = self.consume(TokenType::Identifier)?;
        let name = name_token
            .value
            .ok_or_else(|| "标识符记号缺少值".to_string())?;
        let next = self.tokens.peek().cloned();
        if let Some(t) = next {
            if t.type_ == TokenType::LeftParen {
                let func_decl = self.parse_function_decl(name)?;
                return Ok(Declaration::Fun(func_decl));
            } else {
                let var_decl = self.parse_var_decl(name)?;
                return Ok(Declaration::Variable(var_decl));
            }
        } else {
            return Err("期待一个声明".to_string());
        }
    }
    // <variable-declaration> ::= "int" <identifier> ["=" <exp>] ";"
    // 这里必须注意，"int" <identifier> 已经在调用方解析了
    fn parse_var_decl(&mut self, name: String) -> Result<VarDecl, String> {
        let init = if self.match_token(TokenType::Assignment) {
            Some(self.parse_exp(0)?)
        } else {
            None
        };
        self.consume(TokenType::Semicolon)?;
        Ok(VarDecl { name, init })
    }

    /// <function-declaration> ::= "int" <identifier> "(" <param-list> ")" (<block> | ";")
    /// 这里必须注意，"int" <identifier> 已经在调用方解析了
    fn parse_function_decl(&mut self, name: String) -> Result<FunDecl, String> {
        self.consume(TokenType::LeftParen)?;
        let params = self.parse_func_params()?;
        self.consume(TokenType::RightParen)?;
        if self.match_token(TokenType::Semicolon) {
            return Ok(FunDecl {
                name: name,
                parameters: params,
                body: None,
            });
        } else {
            self.consume(TokenType::LeftBrace)?; //代码不严谨的地方，这里应该在block里面 consume lef {.
            let body = self.parse_block()?;
            Ok(FunDecl {
                name,
                parameters: Vec::new(),
                body: Some(body),
            })
        }
    }
    // <param-list> ::= "void" | "int" <identifier> {"," "int" <identifier>}
    fn parse_func_params(&mut self) -> Result<Vec<String>, String> {
        // 根据语法规则，参数列表要么以 'void' 开头，要么以 'int' 开头。
        // 1. 尝试匹配 "void" 分支
        if self.match_token(TokenType::Void) {
            // 匹配成功，例如 int foo(void)
            // 返回一个空 Vec，表示没有参数名
            return Ok(Vec::new());
        }
        // 2. 如果不是 "void"，则必须匹配 "int <identifier> ..." 分支
        // 解析第一个强制性参数
        self.consume(TokenType::Int)?;
        let first_param = self.consume(TokenType::Identifier)?;

        let mut params = vec![first_param.value.unwrap()];

        // 循环解析 {"," "int" <identifier>} 部分
        while self.match_token(TokenType::Comma) {
            self.consume(TokenType::Int)?;
            let next_param = self.consume(TokenType::Identifier)?;
            params.push(next_param.value.unwrap());
        }

        Ok(params)
    }
    //<block> ::= "{" {<block-item>} "}"
    fn parse_block(&mut self) -> Result<Block, String> {
        let mut items = Vec::new();
        while !self.check(TokenType::RightBrace) {
            items.push(self.parse_block_item()?);
        }
        self.consume(TokenType::RightBrace)?;
        Ok(Block(items))
    }

    //<block-item> ::= <statement> | <declaration>
    fn parse_block_item(&mut self) -> Result<BlockItem, String> {
        let next = self.tokens.peek().cloned();
        if let Some(n) = next {
            if n.type_ == TokenType::Int {
                self.parse_declaration().map(BlockItem::D)
            } else {
                self.parse_statement().map(BlockItem::S)
            }
        } else {
            return Err("期待一个Stmt或者声明".to_string());
        }
    }

    //<for-init> ::= <variable-declaration> | [<exp>] ";"
    fn parse_forinit(&mut self) -> Result<ForInit, String> {
        let t = self.tokens.peek().cloned().unwrap();
        if self.match_token(TokenType::Int) {
            let name_token = self.consume(TokenType::Identifier)?;
            let name = name_token.value.unwrap();
            let var_decl = self.parse_var_decl(name.clone())?;
            Ok(ForInit::InitDecl(var_decl))
        } else if self.match_token(TokenType::Semicolon) {
            Ok(ForInit::InitExp(None))
        } else {
            let e = self.parse_exp(0)?;
            self.consume(TokenType::Semicolon)?;
            Ok(ForInit::InitExp(Some(e)))
        }
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
        } else if self.match_token(TokenType::LeftBrace) {
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
            let condition;
            let post;
            if self.match_token(TokenType::Semicolon) {
                condition = None;
            } else {
                condition = Some(self.parse_exp(0)?);
                self.consume(TokenType::Semicolon)?;
            }
            if self.match_token(TokenType::RightParen) {
                post = None;
            } else {
                post = Some(self.parse_exp(0)?);
                self.consume(TokenType::RightParen)?;
            }
            let body = self.parse_statement()?;

            Ok(Statement::For {
                init: init,
                condition: condition,
                post: post,
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
    //<argument-list> ::= <exp> {"," <exp>}
    fn parse_argument(&mut self) -> Result<Vec<Expression>, String> {
        let mut argument_list = Vec::new();
        if self.check(TokenType::RightParen) {
            Ok(Vec::new())
        } else {
            let e = self.parse_exp(0)?;
            argument_list.push(e);
            while self.match_token(TokenType::Comma) {
                let e = self.parse_exp(0)?;
                argument_list.push(e);
            }
            Ok(argument_list)
        }
    }

    /// 解析前缀部分：数字、变量、括号表达式或一元运算符。
    /// //<factor> ::= <int> | <identifier> | <unop> <factor> | "(" <exp> ")" | <identifier> "(" [<argument-list>] ")"
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
                if self.match_token(TokenType::LeftParen) {
                    let exp_vec = self.parse_argument()?;
                    self.consume(TokenType::RightParen)?;
                    Ok(Expression::FuncCall {
                        name: name,
                        args: exp_vec,
                    })
                } else {
                    Ok(Expression::Var(name))
                }
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

// src/frontend/parser.rs

//! **语法分析器 (Parser)**
//!
//! 该模块负责将词法分析器生成的 Token 流转换为抽象语法树 (AST)。
//! 它实现了一个递归下降解析器，并特别采用了普拉特解析（Pratt Parsing）技术来优雅地处理不同优先级的二元和一元运算符。
//!
//! ## 主要职责
//!
//! 1.  **顶层结构解析**:
//!     -   将整个程序（`<program>`）解析为一系列函数声明/定义的列表。
//!     -   能够区分函数声明（只有原型）和函数定义（包含函数体）。
//!
//! 2.  **声明解析**:
//!     -   解析变量声明 (`<variable-declaration>`) 和函数声明 (`<function-declaration>`)。
//!     -   通过向前查看（lookahead）一个 Token 来决定当前声明是变量还是函数。
//!
//! 3.  **语句解析**:
//!     -   解析C语言中的各种语句，包括：
//!         -   条件语句 (`if-else`)
//!         -   循环语句 (`while`, `do-while`, `for`)
//!         -   控制流语句 (`return`, `break`, `continue`)
//!         -   复合语句（代码块 `{...}`)
//!         -   表达式语句
//!
//! 4.  **表达式解析 (Pratt Parser)**:
//!     -   这是解析器最核心和复杂的部分。通过为每个运算符分配优先级（precedence），它能够正确地处理复杂的表达式，如 `a + b * c` 或 `-a * (b + c)`。
//!     -   `parse_exp` 是 Pratt 解析器的主要驱动函数。
//!     -   `parse_prefix` 用于处理前缀表达式，如常量、变量、括号表达式和一元运算符。
//!     -   `get_infix_precedence` 和 `get_prefix_precedence` 定义了运算符的优先级规则。
//!
//! ## 错误处理
//!
//! -   当 Token 流不符合预期的语法规则时，解析器会返回一个 `Err(String)`。
//! -   错误信息被格式化为 `"Syntax Error: ..."`，以明确指出错误的性质和位置。

use std::iter::Peekable;
use std::vec::IntoIter;

use crate::frontend::c_ast::{
    BinaryOp, Block, BlockItem, Declaration, Expression, ForInit, FunDecl, Program, Statement,
    UnaryOp, VarDecl,
};
use crate::frontend::lexer::{Token, TokenType};

/// 语法分析器结构体，持有 Token 流的迭代器。
#[derive(Debug)]
pub struct Parser {
    /// 一个可向前查看的 (peekable) Token 迭代器。
    /// `Peekable` 允许我们在不消耗 Token 的情况下查看下一个 Token，这对于语法分析至关重要。
    tokens: Peekable<IntoIter<Token>>,
}

impl Parser {
    /// 创建一个新的解析器实例。
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens: tokens.into_iter().peekable(),
        }
    }

    // --- 主入口和顶层解析函数 ---

    /// 解析器的主入口点。它消耗自身并尝试解析整个 Token 流。
    pub fn parse(mut self) -> Result<Program, String> {
        self.parse_program()
    }

    /// 解析整个程序。
    ///
    /// 文法规则: `<program> ::= {<function-declaration> | <variable-declaration>}`
    ///
    /// 在我们的C语言子集中，顶层可以包含函数和全局变量的声明。
    fn parse_program(&mut self) -> Result<Program, String> {
        let mut functions = Vec::new();
        // 持续解析，直到遇到文件结束符 (Eof)。
        while !self.match_token(TokenType::Eof) {
            // 顶层声明必须以 'int' 或 'void' 开头。
            let decl = self.parse_declaration()?;
            // 目前的简化实现只将函数定义添加到程序中。
            // 一个更完整的编译器需要处理全局变量和函数原型。
            if let Declaration::Fun(func_decl) = decl {
                functions.push(func_decl);
            } else {
                // 如果需要支持全局变量，可以在这里处理 `Declaration::Variable`
                return Err("Syntax Error: Global variable declarations are not yet supported.".to_string());
            }
        }
        Ok(Program { functions })
    }

    // --- 声明解析 ---

    /// 解析一个声明（变量或函数）。
    ///
    /// 文法规则: `<declaration> ::= "int" <identifier> (";" | "=" ... | "(" ...)`
    fn parse_declaration(&mut self) -> Result<Declaration, String> {
        // 所有声明都以类型说明符开始，这里我们只支持 "int"。
        self.consume(TokenType::Int)?;
        let name_token = self.consume(TokenType::Identifier)?;
        let name = name_token.value.ok_or_else(|| {
            "Syntax Error: Expected a name for the identifier, but it was missing.".to_string()
        })?;

        // 通过查看下一个 Token 来判断是函数还是变量。
        if self.check(TokenType::LeftParen) {
            // 如果是 '(', 那么这是一个函数声明或定义。
            let func_decl = self.parse_function_remainder(name)?;
            Ok(Declaration::Fun(func_decl))
        } else {
            // 否则，它是一个变量声明。
            let var_decl = self.parse_var_remainder(name)?;
            Ok(Declaration::Variable(var_decl))
        }
    }

    /// 解析变量声明的剩余部分。
    ///
    /// 调用此函数时，`"int" <identifier>` 已经被消耗。
    /// 文法规则: `<var-decl-remainder> ::= ["=" <exp>] ";"`
    fn parse_var_remainder(&mut self, name: String) -> Result<VarDecl, String> {
        let init = if self.match_token(TokenType::Assignment) {
            Some(self.parse_exp(0)?)
        } else {
            None
        };
        self.consume(TokenType::Semicolon)?;
        Ok(VarDecl { name, init })
    }

    /// 解析函数声明或定义的剩余部分。
    ///
    /// 调用此函数时，`"int" <identifier>` 已经被消耗。
    /// 文法规则: `<func-decl-remainder> ::= "(" <param-list> ")" (";" | <block>)`
    fn parse_function_remainder(&mut self, name: String) -> Result<FunDecl, String> {
        self.consume(TokenType::LeftParen)?;
        let params = self.parse_func_params()?;
        self.consume(TokenType::RightParen)?;

        if self.match_token(TokenType::Semicolon) {
            // 如果是分号，这是一个函数原型声明 (e.g., `int add(int a, int b);`)
            Ok(FunDecl {
                name,
                parameters: params,
                body: None,
            })
        } else {
            // 否则，必须是一个函数体代码块。
            let body = self.parse_block()?;
            Ok(FunDecl {
                name,
                parameters: params,
                body: Some(body),
            })
        }
    }

    /// 解析函数参数列表。
    ///
    /// 文法规则: `<param-list> ::= "void" | <param> {"," <param>} | <empty>`
    /// `<param> ::= "int" <identifier>`
    fn parse_func_params(&mut self) -> Result<Vec<String>, String> {
        // 处理 `void` 参数或空参数列表 `()` 的情况。
        if self.match_token(TokenType::Void) || self.check(TokenType::RightParen) {
            return Ok(Vec::new());
        }

        let mut params = Vec::new();
        // 解析第一个参数。
        self.consume(TokenType::Int)?;
        let first_param = self.consume(TokenType::Identifier)?;
        params.push(first_param.value.unwrap()); // `unwrap` 在这里是安全的，因为标识符 Token 总是有值。

        // 循环解析后续由逗号分隔的参数。
        while self.match_token(TokenType::Comma) {
            self.consume(TokenType::Int)?;
            let next_param = self.consume(TokenType::Identifier)?;
            params.push(next_param.value.unwrap());
        }

        Ok(params)
    }

    // --- 语句和块解析 ---

    /// 解析一个代码块。
    ///
    /// 文法规则: `<block> ::= "{" {<block-item>} "}"`
    fn parse_block(&mut self) -> Result<Block, String> {
        self.consume(TokenType::LeftBrace)?;
        let mut items = Vec::new();
        while !self.check(TokenType::RightBrace) {
            items.push(self.parse_block_item()?);
        }
        self.consume(TokenType::RightBrace)?;
        Ok(Block(items))
    }

    /// 解析代码块中的一个条目，它可以是一个声明或一个语句。
    ///
    /// 文法规则: `<block-item> ::= <declaration> | <statement>`
    fn parse_block_item(&mut self) -> Result<BlockItem, String> {
        // 通过检查下一个 Token 是否为 "int" 来区分声明和语句。
        // 这是一个简化的假设，一个完整的C编译器需要更复杂的 lookahead。
        if self.check(TokenType::Int) {
            self.parse_declaration().map(BlockItem::D)
        } else {
            self.parse_statement().map(BlockItem::S)
        }
    }

    /// 解析 `for` 循环的初始化部分。
    ///
    /// 文法规则: `<for-init> ::= <variable-declaration> | [<exp>] ";"`
    fn parse_for_init(&mut self) -> Result<ForInit, String> {
        if self.check(TokenType::Int) {
            // 情况 1: `for (int i = 0; ...)`
            let decl = self.parse_declaration()?;
            match decl {
                Declaration::Variable(var_decl) => Ok(ForInit::InitDecl(var_decl)),
                Declaration::Fun(_) => Err(
                    "Syntax Error: Function declaration is not allowed in a for-loop initializer.".to_string(),
                ),
            }
        } else if self.match_token(TokenType::Semicolon) {
            // 情况 2: `for (; ...)` (无初始化表达式)
            Ok(ForInit::InitExp(None))
        } else {
            // 情况 3: `for (i = 0; ...)`
            let e = self.parse_exp(0)?;
            self.consume(TokenType::Semicolon)?;
            Ok(ForInit::InitExp(Some(e)))
        }
    }

    /// 解析一条语句。
    ///
    /// 文法规则:
    /// `<statement> ::= "return" <exp> ";"
    ///              |  <exp> ";"
    ///              |  "if" "(" <exp> ")" <statement> ["else" <statement>]
    ///              |  <block>
    ///              |  "while" "(" <exp> ")" <statement>
    ///              |  "do" <statement> "while" "(" <exp> ")" ";"
    ///              |  "for" "(" <for-init> [<exp>] ";" [<exp>] ")" <statement>
    ///              |  "break" ";"
    ///              |  "continue" ";"
    ///              |  ";"`
    fn parse_statement(&mut self) -> Result<Statement, String> {
        if self.match_token(TokenType::Return) {
            let expr = self.parse_exp(0)?;
            self.consume(TokenType::Semicolon)?;
            Ok(Statement::Return(expr))
        } else if self.match_token(TokenType::If) {
            self.consume(TokenType::LeftParen)?;
            let condition = self.parse_exp(0)?;
            self.consume(TokenType::RightParen)?;
            let then_stmt = self.parse_statement()?;
            let else_stmt = if self.match_token(TokenType::Else) {
                Some(Box::new(self.parse_statement()?))
            } else {
                None
            };
            Ok(Statement::If {
                condition,
                then_stmt: Box::new(then_stmt),
                else_stmt,
            })
        } else if self.check(TokenType::LeftBrace) {
            let block = self.parse_block()?;
            Ok(Statement::Compound(block))
        } else if self.match_token(TokenType::While) {
            self.consume(TokenType::LeftParen)?;
            let condition = self.parse_exp(0)?;
            self.consume(TokenType::RightParen)?;
            let body = self.parse_statement()?;
            Ok(Statement::While {
                condition,
                body: Box::new(body),
                label: None, // 标签在后续阶段处理
            })
        } else if self.match_token(TokenType::Do) {
            let body = self.parse_statement()?;
            self.consume(TokenType::While)?;
            self.consume(TokenType::LeftParen)?;
            let condition = self.parse_exp(0)?;
            self.consume(TokenType::RightParen)?;
            self.consume(TokenType::Semicolon)?;
            Ok(Statement::DoWhile {
                body: Box::new(body),
                condition,
                label: None,
            })
        } else if self.match_token(TokenType::For) {
            self.consume(TokenType::LeftParen)?;
            let init = self.parse_for_init()?;
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
        } else if self.match_token(TokenType::Break) {
            self.consume(TokenType::Semicolon)?;
            Ok(Statement::Break("fakelabel".to_string())) // 标签在后续阶段处理
        } else if self.match_token(TokenType::Continue) {
            self.consume(TokenType::Semicolon)?;
            Ok(Statement::Continue("fakelabel".to_string())) // 标签在后续阶段处理
        } else if self.match_token(TokenType::Semicolon) {
            Ok(Statement::Null)
        } else {
            // 如果以上都不是，则它必须是一个表达式语句。
            let expr = self.parse_exp(0)?;
            self.consume(TokenType::Semicolon)?;
            Ok(Statement::Expression(expr))
        }
    }

    // --- 表达式解析 (Pratt Parser) ---

    /// 使用 Pratt 解析法解析表达式。
    ///
    /// `min_prec` 参数指定了当前解析上下文的最小运算符优先级。
    /// 这是 Pratt 解析算法的核心，用于正确处理运算符的结合性和优先级。
    fn parse_exp(&mut self, min_prec: i32) -> Result<Expression, String> {
        // 表达式总是以前缀部分开始（例如，一个数字、一个变量、一个括号表达式或一个一元运算符）。
        let mut left = self.parse_prefix()?;

        // 循环处理中缀运算符。
        loop {
            let next_token_type = match self.tokens.peek() {
                Some(token) => token.type_.clone(),
                None => break, // Token 流结束
            };

            // 获取该 Token 作为中缀运算符的优先级。
            // 如果它不是一个有效的运算符，或者其优先级低于当前上下文的最小优先级，则停止循环。
            let op_prec = match self.get_infix_precedence(&next_token_type) {
                Some(prec) if prec >= min_prec => prec,
                _ => break,
            };

            // 消耗掉运算符 Token。
            let op_token = self.tokens.next().unwrap();

            // 根据运算符的类型，构建相应的表达式节点。
            left = match op_token.type_ {
                // 特殊情况：三元条件运算符 `?:`
                TokenType::QuestionMark => {
                    let then_exp = self.parse_exp(0)?; // `then` 分支的优先级最低
                    self.consume(TokenType::Colon)?;
                    // `else` 分支的优先级与 `?:` 相同，以处理 `a ? b : c ? d : e`
                    let else_exp = self.parse_exp(op_prec)?;
                    Expression::Conditional {
                        condition: Box::new(left),
                        left: Box::new(then_exp),
                        right: Box::new(else_exp),
                    }
                }
                // 特殊情况：赋值运算符 `=` (右结合)
                TokenType::Assignment => {
                    // 对于右结合运算符，递归调用 `parse_exp` 时传入与当前运算符相同的优先级。
                    let right = self.parse_exp(op_prec)?;
                    Expression::Assignment {
                        left: Box::new(left),
                        right: Box::new(right),
                    }
                }
                // 通用情况：所有左结合的二元运算符
                _ => {
                    let bin_op = self.to_binary_op(&op_token.type_)?;
                    // 对于左结合运算符，递归调用 `parse_exp` 时传入更高的优先级 (`op_prec + 1`)。
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

    /// 解析函数调用的参数列表。
    ///
    /// 文法规则: `<argument-list> ::= <exp> {"," <exp>} | <empty>`
    fn parse_argument_list(&mut self) -> Result<Vec<Expression>, String> {
        if self.check(TokenType::RightParen) {
            return Ok(Vec::new()); // 空参数列表
        }

        let mut argument_list = Vec::new();
        loop {
            let e = self.parse_exp(0)?;
            argument_list.push(e);
            if !self.match_token(TokenType::Comma) {
                break; // 没有更多参数
            }
        }
        Ok(argument_list)
    }

    /// 解析表达式的前缀部分。
    ///
    /// 文法规则:
    /// `<prefix> ::= <int-literal>
    ///            |  <identifier>
    ///            |  <identifier> "(" [<argument-list>] ")"
    ///            |  <unary-op> <prefix>
    ///            |  "(" <exp> ")"`
    fn parse_prefix(&mut self) -> Result<Expression, String> {
        let next_token = self.tokens.next().ok_or_else(|| {
            "Syntax Error: Expected an expression, but found end of input.".to_string()
        })?;

        match next_token.type_ {
            TokenType::Number => {
                let value = next_token.lexeme.parse::<i64>().map_err(|e| {
                    format!("Syntax Error: Invalid number format: {}", e)
                })?;
                Ok(Expression::Constant(value))
            }
            TokenType::Identifier => {
                let name = next_token.value.ok_or("Internal Error: Identifier token is missing a name")?;
                if self.match_token(TokenType::LeftParen) {
                    // 这是一个函数调用
                    let args = self.parse_argument_list()?;
                    self.consume(TokenType::RightParen)?;
                    Ok(Expression::FuncCall { name, args })
                } else {
                    // 这是一个变量
                    Ok(Expression::Var(name))
                }
            }
            TokenType::LeftParen => {
                // 这是一个括号表达式
                let exp = self.parse_exp(0)?;
                self.consume(TokenType::RightParen)?;
                Ok(exp)
            }
            // 处理所有一元前缀运算符
            TokenType::Negate | TokenType::Complement | TokenType::Bang => {
                let op = self.to_unary_op(&next_token.type_)?;
                let ((), op_prec) = self.get_prefix_precedence(&next_token.type_).unwrap();
                let right_exp = self.parse_exp(op_prec)?;
                Ok(Expression::Unary {
                    op,
                    exp: Box::new(right_exp),
                })
            }
            _ => Err(format!(
                "Syntax Error: Expected an expression prefix (like a number, variable, or '('), but found {:?}.",
                next_token.type_
            )),
        }
    }

    // --- 优先级和工具函数 ---

    /// 获取中缀（二元）运算符的优先级。返回 `None` 表示该 Token 不是一个有效的中缀运算符。
    fn get_infix_precedence(&self, typ: &TokenType) -> Option<i32> {
        match typ {
            TokenType::Assignment => Some(10),
            TokenType::QuestionMark => Some(15), // 三元运算符
            TokenType::Or => Some(20),
            TokenType::And => Some(30),
            TokenType::EqualEqual | TokenType::BangEqual => Some(40),
            TokenType::Greater | TokenType::GreaterEqual | TokenType::Less | TokenType::LessEqual => Some(50),
            TokenType::Add | TokenType::Negate => Some(60), // 在中缀位置，'-' 是减法
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

    /// 将 `TokenType` 转换为 `BinaryOp`。
    fn to_binary_op(&self, typ: &TokenType) -> Result<BinaryOp, String> {
        match typ {
            TokenType::Add => Ok(BinaryOp::Add),
            TokenType::Negate => Ok(BinaryOp::Subtract), // 在中缀位置，'-' 是减法
            TokenType::Mul => Ok(BinaryOp::Multiply),
            TokenType::Div => Ok(BinaryOp::Divide),
            TokenType::Remainder => Ok(BinaryOp::Remainder),
            TokenType::And => Ok(BinaryOp::And),
            TokenType::Or => Ok(BinaryOp::Or),
            TokenType::BangEqual => Ok(BinaryOp::BangEqual),
            TokenType::EqualEqual => Ok(BinaryOp::EqualEqual),
            TokenType::Greater => Ok(BinaryOp::Greater),
            TokenType::GreaterEqual => Ok(BinaryOp::GreaterEqual),
            TokenType::Less => Ok(BinaryOp::Less),
            TokenType::LessEqual => Ok(BinaryOp::LessEqual),
            _ => Err(format!("Internal Error: Cannot convert {:?} to a binary operator.", typ)),
        }
    }

    /// 将 `TokenType` 转换为 `UnaryOp`。
    fn to_unary_op(&self, typ: &TokenType) -> Result<UnaryOp, String> {
        match typ {
            TokenType::Negate => Ok(UnaryOp::Negate),
            TokenType::Complement => Ok(UnaryOp::Complement),
            TokenType::Bang => Ok(UnaryOp::Not),
            _ => Err(format!("Internal Error: Cannot convert {:?} to a unary operator.", typ)),
        }
    }

    /// 消耗一个期望的 Token。如果下一个 Token 不是期望的类型，则返回错误。
    fn consume(&mut self, expected: TokenType) -> Result<Token, String> {
        match self.tokens.next() {
            Some(token) if token.type_ == expected => Ok(token),
            Some(token) => Err(format!(
                "Syntax Error: Expected token {:?}, but got {:?}.",
                expected, token.type_
            )),
            None => Err(format!(
                "Syntax Error: Expected token {:?}, but the input stream ended.",
                expected
            )),
        }
    }

    /// 检查下一个 Token 是否是期望的类型，但不消耗它。
    fn check(&mut self, expected: TokenType) -> bool {
        self.tokens.peek().map_or(false, |t| t.type_ == expected)
    }

    /// 如果下一个 Token 是期望的类型，则消耗它并返回 `true`。否则，不消耗任何东西并返回 `false`。
    fn match_token(&mut self, expected: TokenType) -> bool {
        if self.check(expected) {
            self.tokens.next();
            true
        } else {
            false
        }
    }
}

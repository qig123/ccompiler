use crate::{
    error::ParserError,
    lexer::{Token, token::TokenType},
    parser::c_ast::{
        BinaryOperator, Block, Declaration, Expr, Function, LiteralExpr, Program, Stmt,
    },
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
            // 确保每个函数都被完整解析或报告错误
            match self.parse_function() {
                Ok(function) => functions.push(function),
                Err(e) => {
                    // 遇到错误时，可以尝试同步（跳过一些 token 直到可能的恢复点）
                    // 但对于简单编译器，直接返回错误通常更安全
                    return Err(e);
                }
            }
        }
        Ok(Program {
            functions: functions,
        })
    }
    fn parse_function(&mut self) -> Result<Function, ParserError> {
        // 期望函数返回类型 int
        self.consume(
            TokenType::KeywordInt,
            "Expected 'int' for function return type",
        )?;

        // 期望函数名称 (Identifier)
        let identifier_token = self
            .consume(TokenType::Identifier, "Expected function name after 'int'")?
            .clone();

        // 期望参数列表开始 '('
        self.consume(TokenType::LeftParen, "Expected '(' after function name")?;

        // 期望参数列表是 void
        // TODO: 未来需要扩展以支持实际参数
        self.consume(
            TokenType::KeywordVoid,
            "Expected 'void' as parameter (currently only void is supported)",
        )?;

        // 期望参数列表结束 ')'
        self.consume(
            TokenType::RightParen,
            "Expected ')' to end function parameters",
        )?;

        // 解析函数体
        let body = self.parse_body()?;

        Ok(Function {
            name: identifier_token,
            body,
        })
    }
    fn parse_body(&mut self) -> Result<Vec<Block>, ParserError> {
        let mut body = Vec::new();
        // 期望函数体开始 '{'
        self.consume(TokenType::LeftBrace, "Expected '{' to start function body")?;

        // 循环解析多个 block item (声明或语句)，直到遇到 '}' 或文件结束
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            let block_item = self.parse_block()?;
            body.push(block_item);
        }
        // 循环结束

        // 期望函数体结束 '}'
        self.consume(TokenType::RightBrace, "Expected '}' to end function body")?;
        Ok(body)
    }
    fn parse_block(&mut self) -> Result<Block, ParserError> {
        // 检查是否是变量声明 (以 'int' 开头)
        if self.check(TokenType::KeywordInt) {
            // 这是一个声明
            self.advance(); // 消耗 'int'
            let name_token = self
                .consume(TokenType::Identifier, "Expected variable name after 'int'")?
                .clone();

            let init = if self.match_token(&[TokenType::Equal]) {
                // 如果有初始化部分 '='
                Some(self.parse_expression()?)
            } else {
                // 没有初始化部分
                None
            };

            // 声明以 ';' 结束
            self.consume(
                TokenType::Semicolon,
                "Expected ';' after variable declaration",
            )?;
            let name = name_token.get_lexeme(self.source);
            Ok(Block::Declaration(Declaration {
                name: name_token,
                init: init.map(Box::new), // 如果有初始化表达式，则装箱
                unique_name: format!("{}", name), // 生成唯一名称
            }))
        } else {
            // 否则，这是一个语句
            let stmt = self.parse_statement()?;
            Ok(Block::Stmt(stmt))
        }
    }
    fn parse_statement(&mut self) -> Result<Stmt, ParserError> {
        if self.match_token(&[TokenType::KeywordReturn]) {
            // 如果是 return 语句
            return self.parse_return_statement();
        } else if self.match_token(&[TokenType::Semicolon]) {
            // 如果是空语句 ';'
            return Ok(Stmt::Null);
        } else {
            // 否则，解析为表达式语句
            let expr = self.parse_expression()?;
            // 表达式语句以 ';' 结束
            self.consume(TokenType::Semicolon, "Expected ';' after expression")?;
            return Ok(Stmt::Expression {
                exp: Box::new(expr),
            });
        }
    }
    fn parse_return_statement(&mut self) -> Result<Stmt, ParserError> {
        // return 关键字已经被 match_token 消耗了，获取它
        let keyword = self.previous().clone();

        // 检查 ';' 是否紧随其后，决定是否有返回值
        let value = if self.check(TokenType::Semicolon) {
            None // 没有返回值
        } else {
            Some(self.parse_expression()?) // 解析返回值表达式
        };

        // 期望 ';' 结束 return 语句
        self.consume(TokenType::Semicolon, "Expected ';' after return statement")?;

        Ok(Stmt::Return {
            keyword,
            value: value.map(Box::new),
        })
    }
    fn parse_expression(&mut self) -> Result<Expr, ParserError> {
        // 从最低优先级开始解析表达式
        self.parse_precedence(0)
    }

    // 使用 Pratt 解析算法解析二元表达式，处理优先级和结合性
    fn parse_precedence(&mut self, min_precedence: u8) -> Result<Expr, ParserError> {
        // 首先解析左侧操作数，它必须是一个 factor (字面量、变量、一元表达式、括号表达式等)
        let mut left = self.parser_factor()?;

        // 进入循环，尝试将后续的二元运算符绑定到当前的 left 表达式
        loop {
            let token = self.peek();

            // 识别下一个可能的二元运算符，并获取其优先级
            let op_info = match token.token_type {
                TokenType::Add => Some((BinaryOperator::Add, BinaryOperator::Add.precedence())),
                TokenType::Minus => Some((
                    BinaryOperator::Subtract,
                    BinaryOperator::Subtract.precedence(),
                )),
                TokenType::Mul => Some((
                    BinaryOperator::Multiply,
                    BinaryOperator::Multiply.precedence(),
                )),
                TokenType::Div => {
                    Some((BinaryOperator::Divide, BinaryOperator::Divide.precedence()))
                }
                TokenType::Remainder => Some((
                    BinaryOperator::Remainder,
                    BinaryOperator::Remainder.precedence(),
                )),
                TokenType::And => Some((BinaryOperator::And, BinaryOperator::And.precedence())), // &&
                TokenType::Or => Some((BinaryOperator::Or, BinaryOperator::Or.precedence())), // ||
                TokenType::EqualEqual => Some((
                    BinaryOperator::EqualEqual,
                    BinaryOperator::EqualEqual.precedence(),
                )), // ==
                TokenType::BangEqual => Some((
                    BinaryOperator::BangEqual,
                    BinaryOperator::BangEqual.precedence(),
                )), // !=
                TokenType::Less => Some((BinaryOperator::Less, BinaryOperator::Less.precedence())),
                TokenType::LessEqual => Some((
                    BinaryOperator::LessEqual,
                    BinaryOperator::LessEqual.precedence(),
                )),
                TokenType::Greater => Some((
                    BinaryOperator::Greater,
                    BinaryOperator::Greater.precedence(),
                )),
                TokenType::GreaterEqual => Some((
                    BinaryOperator::GreaterEqual,
                    BinaryOperator::GreaterEqual.precedence(),
                )),
                TokenType::Equal => {
                    Some((BinaryOperator::Equal, BinaryOperator::Equal.precedence()))
                } // = (赋值)
                _ => None, // 不是支持的二元运算符，退出循环
            };

            // 如果不是二元运算符，或者运算符的优先级不足以与当前的 left 结合，则退出循环
            let (op, op_precedence) = match op_info {
                Some(info) => info,
                None => break, // 不是二元运算符，停止扩展
            };

            // Pratt 解析规则核心：如果当前运算符的优先级小于传入的最低优先级，则不能绑定
            if op_precedence < min_precedence {
                break; // 优先级太低，停止扩展
            }

            // *** 消耗掉当前的二元运算符 Token ***
            self.advance(); // 现在已经确定要使用这个运算符了，消耗它

            // 确定解析右侧表达式所需的最低优先级
            // 左结合运算符 (+, -, *, /, %, &&, ||, 比较运算符)：右侧必须解析优先级更高的表达式 (p + 1)
            // 右结合运算符 (=)：右侧可以解析优先级相同或更高的表达式 (p)
            let next_min_precedence = if op == BinaryOperator::Equal {
                // 赋值运算符 '=' 是右结合的
                op_precedence
            } else {
                // 大多数其他运算符是左结合的
                op_precedence + 1
            };

            // 递归解析右侧操作数，传入计算出的下一级最低优先级
            let right = self.parse_precedence(next_min_precedence)?;

            // 根据运算符类型构建新的 AST 节点
            left = match op {
                BinaryOperator::Equal => Expr::Assignment {
                    // 赋值的左侧理论上必须是可赋值的（如变量 Var），但 AST 生成阶段可以先允许任何 Expr，
                    // 语义分析阶段再检查是否合法。
                    left: Box::new(left),   // 现在的 left 是赋值的左侧
                    right: Box::new(right), // 刚解析的 right 是赋值的右侧
                },
                // 所有其他二元运算符
                _ => Expr::Binary {
                    operator: op,           // 存储二元运算符类型
                    left: Box::new(left),   // 之前的 left 是二元操作的左侧
                    right: Box::new(right), // 刚解析的 right 是二元操作的右侧
                },
            };

            // 将新构建的二元表达式或赋值表达式作为新的 left，循环继续，尝试绑定更高优先级的运算符
        }

        // 循环结束，left 中存放的是最终解析出的表达式的根节点
        Ok(left)
    }

    // <factor> ::= <int> | <identifier> | <unop> <factor> | "(" <exp> ")"
    // 解析最高优先级的表达式单元：字面量、变量、一元操作、括号表达式
    fn parser_factor(&mut self) -> Result<Expr, ParserError> {
        // 解析整数常量
        if self.match_token(&[TokenType::LiteralInt]) {
            let prev = self.previous();
            match &prev.literal {
                Some(Value::Int(i)) => Ok(Expr::Literal(LiteralExpr::Integer(*i))),
                None => {
                    // 内部错误：LiteralInt Token 应该带有 Value
                    Err(ParserError {
                        message: format!("Internal error: LiteralInt token missing value at "), // 假设 Token 有 span 字段
                    })
                }
            }
        // 解析一元运算符
        } else if self.match_token(&[TokenType::Minus, TokenType::BitwiseNot, TokenType::Bang]) {
            let operator_token = self.previous().clone(); // 获取并消耗一元运算符 Token

            // 递归解析一元运算符后面的操作数
            let right_expr = self.parser_factor()?; // 一元运算符通常结合最高

            Ok(Expr::Unary {
                operator: operator_token,
                right: Box::new(right_expr), // 将子表达式装箱
            })

        // 解析括号表达式 "(" <exp> ")"
        } else if self.match_token(&[TokenType::LeftParen]) {
            // 消耗左括号 '('

            // 递归解析括号内的完整表达式
            let inner_expr = self.parse_expression()?;

            // 期望匹配并消耗右括号 ')'
            self.consume(TokenType::RightParen, "Expected ')' after expression")?;

            // 括号表达式在 AST 中表示为 Grouping
            Ok(Expr::Grouping {
                expression: Box::new(inner_expr), // 将内部表达式装箱
            })
        // 解析标识符 (变量引用)
        } else if self.match_token(&[TokenType::Identifier]) {
            let identifier = self.previous().clone(); // 获取并消耗标识符 Token
            let name = identifier.get_lexeme(self.source).to_string(); // 获取标识符的文本
            Ok(Expr::Var {
                name: identifier,  // 使用标识符的 Token
                unique_name: name, // 生成唯一名称（目前直接使用标识符文本）
            })
        // 遇到无法识别的 Token，报告错误
        } else {
            Err(ParserError {
                message: format!(
                    "Expected an expression (integer, variable, unary op, or grouping), but found '{}' ",
                    self.peek().get_lexeme(self.source), // 获取当前 Token 的文本用于错误信息
                ),
            })
        }
    }

    // 检查当前 Token 是否在给定的类型列表中，如果是，则消耗并返回 true
    fn match_token(&mut self, types: &[TokenType]) -> bool {
        for t in types {
            if self.check(t.clone()) {
                self.advance();
                return true;
            }
        }
        false // 没有匹配到任何类型
    }

    // 检查当前 Token 是否是期望的类型，如果是，则消耗并返回该 Token 的引用，否则返回错误
    fn consume(&mut self, expected: TokenType, message: &str) -> Result<&Token, ParserError> {
        let current_token = self.peek();
        if self.check(expected) {
            Ok(self.advance()) // 匹配成功，消耗 Token 并返回
        } else {
            // 构造更有信息量的错误消息，包含期待的 Token 类型和实际遇到的 Token 文本
            let error_message = format!(
                "{} (found '{}' instead at position )",
                message,
                current_token.get_lexeme(self.source),
            );
            Err(ParserError {
                message: error_message,
            })
        }
    }

    // 基础方法：检查当前 Token 是否是给定类型，不消耗
    fn check(&self, t: TokenType) -> bool {
        if self.is_at_end() {
            return false; // 文件末尾无法匹配任何类型
        }
        self.peek().token_type == t
    }

    // 基础方法：消耗当前 Token，并将当前位置向前移动一位，返回被消耗的 Token 的引用
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        // advance 总是返回 current 移动之前位置的 Token
        self.previous()
    }

    // 基础方法：查看当前位置的 Token，不消耗
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    // 基础方法：查看前一个被消耗的 Token
    fn previous(&self) -> &Token {
        // 确保 current > 0，否则会越界
        debug_assert!(self.current > 0, "Cannot call previous() at the beginning");
        &self.tokens[self.current - 1]
    }

    // 基础方法：检查是否到达文件末尾
    fn is_at_end(&self) -> bool {
        self.peek().token_type == TokenType::Eof
    }
}

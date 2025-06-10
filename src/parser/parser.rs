use crate::{
    error::ParserError,
    lexer::{Token, token::TokenType},
    parser::c_ast::{
        BinaryOperator, Block, Declaration, Expr, Function, LiteralExpr, Program, Stmt,
    },
    types::types::Value,
};

// --- 辅助数据结构和函数 (用于表达式解析) ---
#[derive(Debug, Clone)]
enum OperatorType {
    Binary(BinaryOperator),
    TernaryQuestion, // 特指三元运算符的 '?' 部分
}

#[derive(Debug, Clone)]
struct OperatorDetails {
    op_type: OperatorType,
    precedence: u8,
}

fn get_operator_info(token_type: TokenType) -> Option<OperatorDetails> {
    match token_type {
        TokenType::Add => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::Add),
            precedence: BinaryOperator::Add.precedence(),
        }),
        TokenType::Minus => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::Subtract),
            precedence: BinaryOperator::Subtract.precedence(),
        }),
        TokenType::Mul => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::Multiply),
            precedence: BinaryOperator::Multiply.precedence(),
        }),
        TokenType::Div => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::Divide),
            precedence: BinaryOperator::Divide.precedence(),
        }),
        TokenType::Remainder => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::Remainder),
            precedence: BinaryOperator::Remainder.precedence(),
        }),
        TokenType::And => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::And),
            precedence: BinaryOperator::And.precedence(),
        }),
        TokenType::Or => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::Or),
            precedence: BinaryOperator::Or.precedence(),
        }),
        TokenType::EqualEqual => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::EqualEqual),
            precedence: BinaryOperator::EqualEqual.precedence(),
        }),
        TokenType::BangEqual => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::BangEqual),
            precedence: BinaryOperator::BangEqual.precedence(),
        }),
        TokenType::Less => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::Less),
            precedence: BinaryOperator::Less.precedence(),
        }),
        TokenType::LessEqual => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::LessEqual),
            precedence: BinaryOperator::LessEqual.precedence(),
        }),
        TokenType::Greater => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::Greater),
            precedence: BinaryOperator::Greater.precedence(),
        }),
        TokenType::GreaterEqual => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::GreaterEqual),
            precedence: BinaryOperator::GreaterEqual.precedence(),
        }),
        TokenType::Equal => Some(OperatorDetails {
            op_type: OperatorType::Binary(BinaryOperator::Equal),
            precedence: BinaryOperator::Equal.precedence(),
        }),
        TokenType::Question => Some(OperatorDetails {
            op_type: OperatorType::TernaryQuestion,
            precedence: BinaryOperator::Question.precedence(),
        }),
        _ => None,
    }
}

// --- Parser 结构体定义 ---
pub struct Parser<'a> {
    tokens: Vec<Token>,
    current: usize,
    source: &'a str,
}

impl<'a> Parser<'a> {
    // --- 公共 API ---
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
            match self.parse_function() {
                Ok(function) => functions.push(function),
                Err(e) => return Err(e), // 简单起见，遇到错误直接返回
            }
        }
        Ok(Program { functions })
    }

    // --- 高层级解析方法 (函数、块、语句) ---
    fn parse_function(&mut self) -> Result<Function, ParserError> {
        self.consume(
            TokenType::KeywordInt,
            "Expected 'int' for function return type",
        )?;
        let identifier_token = self
            .consume(TokenType::Identifier, "Expected function name after 'int'")?
            .clone();
        self.consume(TokenType::LeftParen, "Expected '(' after function name")?;
        // TODO: 未来需要扩展以支持实际参数
        self.consume(
            TokenType::KeywordVoid,
            "Expected 'void' as parameter (currently only void is supported)",
        )?;
        self.consume(
            TokenType::RightParen,
            "Expected ')' to end function parameters",
        )?;
        let body = self.parse_body()?;

        Ok(Function {
            name: identifier_token,
            body,
        })
    }

    fn parse_body(&mut self) -> Result<Vec<Block>, ParserError> {
        let mut body = Vec::new();
        self.consume(TokenType::LeftBrace, "Expected '{' to start function body")?;
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            body.push(self.parse_block()?);
        }
        self.consume(TokenType::RightBrace, "Expected '}' to end function body")?;
        Ok(body)
    }

    fn parse_block(&mut self) -> Result<Block, ParserError> {
        if self.check(TokenType::KeywordInt) {
            self.advance(); // 消耗 'int'
            let name_token = self
                .consume(TokenType::Identifier, "Expected variable name after 'int'")?
                .clone();
            let init = if self.match_token(&[TokenType::Equal]) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.consume(
                TokenType::Semicolon,
                "Expected ';' after variable declaration",
            )?;
            let name = name_token.get_lexeme(self.source);
            Ok(Block::Declaration(Declaration {
                name: name_token,
                init: init.map(Box::new),
                unique_name: format!("{}", name),
            }))
        } else {
            Ok(Block::Stmt(self.parse_statement()?))
        }
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParserError> {
        if self.match_token(&[TokenType::KeywordReturn]) {
            self.parse_return_statement()
        } else if self.match_token(&[TokenType::Semicolon]) {
            Ok(Stmt::Null)
        } else if self.match_token(&[TokenType::KeywordIf]) {
            self.consume(TokenType::LeftParen, "Expected '(' after if")?;
            let condition = self.parse_expression()?;
            self.consume(TokenType::RightParen, "Expected ')' after if condition")?;
            let then_branch = self.parse_statement()?;
            let else_branch = if self.match_token(&[TokenType::KeywordElse]) {
                Some(self.parse_statement()?)
            } else {
                None
            };
            Ok(Stmt::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: else_branch.map(Box::new),
            })
        } else {
            let expr = self.parse_expression()?;
            self.consume(
                TokenType::Semicolon,
                "Expected ';' after expression statement",
            )?;
            Ok(Stmt::Expression {
                exp: Box::new(expr),
            })
        }
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, ParserError> {
        let keyword = self.previous().clone(); // 'return' 已被消耗
        let value = if self.check(TokenType::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.consume(TokenType::Semicolon, "Expected ';' after return statement")?;
        Ok(Stmt::Return {
            keyword,
            value: value.map(Box::new),
        })
    }

    // --- 表达式解析 (Pratt Parser) ---
    fn parse_expression(&mut self) -> Result<Expr, ParserError> {
        self.parse_precedence(0) // 从最低优先级开始
    }

    fn parse_precedence(&mut self, min_precedence: u8) -> Result<Expr, ParserError> {
        // 1. 解析前缀部分 (factor) 作为左操作数
        let mut left = self.parse_factor()?;

        loop {
            // 2. 查看下一个 token, 获取操作符信息
            let peeked_token = self.peek();
            let op_details = match get_operator_info(peeked_token.token_type.clone()) {
                // 3. 如果是可识别的操作符且优先级足够高，则处理
                Some(details) if details.precedence >= min_precedence => details,
                _ => break, // 否则，停止扩展当前表达式
            };

            // 4. 消耗操作符
            self.advance();

            // 5. 根据操作符类型处理
            match op_details.op_type {
                OperatorType::TernaryQuestion => {
                    // 'left' 是条件部分
                    // 'then' 表达式允许任何优先级
                    let then_expr = self.parse_precedence(0)?;
                    self.consume(TokenType::Colon, "Expected ':' in ternary operator")?;
                    // 'else' 表达式使用 '?' 的优先级，以支持右结合
                    let else_expr = self.parse_precedence(op_details.precedence)?;
                    left = Expr::Condtional {
                        condition: Box::new(left),
                        left: Box::new(then_expr),
                        right: Box::new(else_expr),
                    };
                }
                OperatorType::Binary(binary_op) => {
                    let next_min_precedence = if binary_op == BinaryOperator::Equal {
                        op_details.precedence // 右结合 (如赋值)
                    } else {
                        op_details.precedence + 1 // 左结合 (其他二元操作)
                    };
                    let right = self.parse_precedence(next_min_precedence)?;

                    if binary_op == BinaryOperator::Equal {
                        left = Expr::Assignment {
                            left: Box::new(left),
                            right: Box::new(right),
                        };
                    } else {
                        left = Expr::Binary {
                            operator: binary_op,
                            left: Box::new(left),
                            right: Box::new(right),
                        };
                    }
                }
            }
        }
        Ok(left)
    }

    // 解析原子表达式: 字面量, 变量, 一元操作, 括号表达式
    fn parse_factor(&mut self) -> Result<Expr, ParserError> {
        if self.match_token(&[TokenType::LiteralInt]) {
            let prev = self.previous();
            match &prev.literal {
                Some(Value::Int(i)) => Ok(Expr::Literal(LiteralExpr::Integer(*i))),
                None => Err(ParserError {
                    message: format!("Internal error: LiteralInt token missing value"),
                }),
            }
        } else if self.match_token(&[TokenType::Minus, TokenType::BitwiseNot, TokenType::Bang]) {
            let operator_token = self.previous().clone();
            // 一元操作符通常紧密绑定其后的因子
            let right_expr = self.parse_factor()?;
            Ok(Expr::Unary {
                operator: operator_token,
                right: Box::new(right_expr),
            })
        } else if self.match_token(&[TokenType::LeftParen]) {
            let inner_expr = self.parse_expression()?;
            self.consume(
                TokenType::RightParen,
                "Expected ')' after expression in parentheses",
            )?;
            Ok(Expr::Grouping {
                expression: Box::new(inner_expr),
            })
        } else if self.match_token(&[TokenType::Identifier]) {
            let identifier = self.previous().clone();
            let name = identifier.get_lexeme(self.source).to_string();
            Ok(Expr::Var {
                name: identifier,
                unique_name: name,
            })
        } else {
            Err(ParserError {
                message: format!(
                    "Expected an expression factor (integer, variable, unary, or grouping), but found '{}'",
                    self.peek().get_lexeme(self.source),
                ),
            })
        }
    }

    // --- Token 处理辅助方法 ---
    fn match_token(&mut self, types: &[TokenType]) -> bool {
        for t in types {
            if self.check(t.clone()) {
                // TokenType 可能需要 #[derive(Clone)]
                self.advance();
                return true;
            }
        }
        false
    }

    fn consume(&mut self, expected: TokenType, message: &str) -> Result<&Token, ParserError> {
        if self.check(expected.clone()) {
            // TokenType 可能需要 #[derive(Clone)]
            Ok(self.advance())
        } else {
            let current_token_lexeme = self.peek().get_lexeme(self.source);
            Err(ParserError {
                message: format!("{} (found '{}')", message, current_token_lexeme),
            })
        }
    }

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
        debug_assert!(
            self.current > 0,
            "Cannot call previous() at the beginning of tokens"
        );
        &self.tokens[self.current - 1]
    }

    fn is_at_end(&self) -> bool {
        // 确保 current 不会越界，如果 tokens 为空或 current 已经是末尾的 EOF
        if self.tokens.is_empty() || self.current >= self.tokens.len() {
            // 这种情况下，最好 lexer 保证最后一个 token 是 EOF
            // 如果 tokens 可能为空，或者最后一个不是 EOF，这里需要更健壮的处理
            // 假设 lexer 总是产生至少一个 EOF token
            return true; // 或者 panic!，取决于你的错误处理策略
        }
        self.peek().token_type == TokenType::Eof
    }
}

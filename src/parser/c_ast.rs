use crate::lexer::Token;
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: Token,
    pub body: Vec<Stmt>,
}
#[derive(Debug, Clone, PartialEq)]

pub enum Expr {
    Literal(LiteralExpr),
    Unary {
        operator: Token,
        right: Box<Expr>,
    },
    Grouping {
        expression: Box<Expr>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}
impl BinaryOperator {
    pub fn precedence(&self) -> u8 {
        match self {
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Remainder => 50,
            BinaryOperator::Add | BinaryOperator::Subtract => 45,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralExpr {
    Integer(i64),
}
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Return { keyword: Token, value: Option<Expr> },
}

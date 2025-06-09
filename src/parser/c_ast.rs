use crate::lexer::Token;
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: Token,
    pub body: Vec<Block>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Stmt(Stmt),
    Declaration(Declaration),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: Token,
    pub init: Option<Box<Expr>>,
    pub unique_name: String, // 新增字段，存储生成的唯一名称
}
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Return {
        keyword: Token,
        value: Option<Box<Expr>>,
    },
    Expression {
        exp: Box<Expr>,
    },
    Null,
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
    Var {
        name: Token,
        unique_name: String, // 新增字段，存储对应的唯一名称
    },
    Assignment {
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
    And,
    Or,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
}
impl BinaryOperator {
    pub fn precedence(&self) -> u8 {
        match self {
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Remainder => 50,
            BinaryOperator::Add | BinaryOperator::Subtract => 45,
            BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => 35,
            BinaryOperator::EqualEqual | BinaryOperator::BangEqual => 30,
            BinaryOperator::And => 10,
            BinaryOperator::Or => 5,
            BinaryOperator::Equal => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralExpr {
    Integer(i64),
}

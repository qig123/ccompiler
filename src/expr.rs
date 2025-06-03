use crate::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: Token,
    pub body: Vec<Stmt>,
}
#[derive(Debug, Clone, PartialEq)]

pub enum Expr {
    Literal(LiteralExpr),
}
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralExpr {
    Integer(i64),
}
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Return { keyword: Token, value: Option<Expr> },
}

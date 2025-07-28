// src/frontend/c_ast.rs

use crate::common::{AstNode, PrettyPrinter};
use std::fmt;

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
}
#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Return(Expression),
}

#[derive(Debug)]
pub enum Expression {
    Constant(i64),
    Unary {
        op: UnaryOp,
        exp: Box<Expression>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

#[derive(Debug)]
pub enum UnaryOp {
    Complement,
    Negate,
    Not,
}

#[derive(Debug)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    And,
    Or,
    EqualEqual,
    BangEqual,
    LessEqual,
    GreaterEqual,
    Less,
    Greater,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnaryOp::Complement => write!(f, "~"),
            UnaryOp::Negate => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Subtract => write!(f, "-"),
            BinaryOp::Multiply => write!(f, "*"),
            BinaryOp::Divide => write!(f, "/"),
            BinaryOp::Remainder => write!(f, "%"),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
            BinaryOp::BangEqual => write!(f, "!="),
            BinaryOp::EqualEqual => write!(f, "=="),
            BinaryOp::Greater => write!(f, ">"),
            BinaryOp::GreaterEqual => write!(f, ">="),
            BinaryOp::Less => write!(f, "<"),
            BinaryOp::LessEqual => write!(f, "<="),
        }
    }
}

impl AstNode for Program {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln("Program").unwrap();
        printer.indent();
        for function in &self.functions {
            function.pretty_print(printer);
        }
        printer.unindent();
    }
}

impl AstNode for Function {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        let params_str = if self.parameters.is_empty() {
            "void".to_string()
        } else {
            self.parameters.join(", ")
        };
        printer
            .writeln(&format!(
                "Function(name: \"{}\", params: [{}])",
                self.name, params_str
            ))
            .unwrap();

        printer.indent();
        // 如果函数体不为空，可以打印一个 "Body" 标签来分隔
        if !self.body.is_empty() {
            printer.writeln("Body").unwrap();
            printer.indent();
            for statement in &self.body {
                statement.pretty_print(printer);
            }
            printer.unindent();
        }
        printer.unindent();
    }
}

impl AstNode for Statement {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        match self {
            Statement::Return(expr) => {
                // Return 节点后面紧跟其表达式子树
                printer.writeln("Return").unwrap();
                printer.indent();
                expr.pretty_print(printer);
                printer.unindent();
            }
        }
    }
}

impl AstNode for Expression {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        match self {
            Expression::Constant(value) => {
                // 叶子节点，直接打印
                printer.writeln(&format!("Constant({})", value)).unwrap();
            }
            Expression::Unary { op, exp } => {
                // 打印节点信息，然后缩进，打印子节点，最后取消缩进
                printer.writeln(&format!("Unary({})", op)).unwrap();
                printer.indent();
                exp.pretty_print(printer);
                printer.unindent();
            }
            Expression::Binary { op, left, right } => {
                // 同样，打印节点信息，然后处理子节点
                printer.writeln(&format!("Binary({})", op)).unwrap();
                printer.indent();
                left.pretty_print(printer);
                right.pretty_print(printer);
                printer.unindent();
            }
        }
    }
}

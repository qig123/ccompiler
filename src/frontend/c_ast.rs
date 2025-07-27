use std::fmt;

use crate::common::{AstNode, PrettyPrinter};

//src/frontend/c_ast.rs

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<String>, // Will be empty for "void"
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
}
#[derive(Debug)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}
impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnaryOp::Complement => write!(f, "Complement (~)"),
            UnaryOp::Negate => write!(f, "Negate (-)"),
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
        let params = if self.parameters.is_empty() {
            "void".to_string()
        } else {
            self.parameters.join(", ")
        };
        printer
            .writeln(&format!(
                "Function(name: {}, params: [{}])",
                self.name, params
            ))
            .unwrap();

        printer.indent();
        printer.writeln("Body").unwrap();
        printer.indent();
        for statement in &self.body {
            statement.pretty_print(printer);
        }
        printer.unindent();
        printer.unindent();
    }
}

impl AstNode for Statement {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        match self {
            Statement::Return(expr) => {
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
                printer
                    .writeln(&format!("Constant(value: {})", value))
                    .unwrap();
            }
            Expression::Unary { op, exp } => {
                printer.writeln(&format!("Unary(op: {})", op)).unwrap();
                printer.indent();
                exp.pretty_print(printer);
                printer.unindent();
            }
            Expression::Binary { op, left, right } => {
                printer.writeln(&format!("Binary({}", op)).unwrap();
                printer.indent();
                left.pretty_print(printer);
                right.pretty_print(printer);
                printer.writeln(&format!(")")).unwrap();
                printer.unindent();
            }
        }
    }
}

use std::fmt;

use crate::common::{AstNode, PrettyPrinter};

//src/backend/tacky_ir.rs
#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
}
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub body: Vec<Instruction>,
}
#[derive(Debug, Clone)]
pub enum Instruction {
    Return(Value),
    Unary {
        op: UnaryOp,
        src: Value,
        dst: Value,
    },
    Binary {
        op: BinaryOp,
        src1: Value,
        src2: Value,
        dst: Value,
    },
}
#[derive(Debug, Clone)]
pub enum Value {
    Constant(i64),
    Var(String),
}
#[derive(Debug, Clone)]
pub enum UnaryOp {
    Complement,
    Negate,
}
#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Constant(i) => write!(f, "{}", i),
            Value::Var(name) => write!(f, "{}", name),
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Complement => write!(f, "COMPLEMENT"),
            UnaryOp::Negate => write!(f, "NEG"),
        }
    }
}
impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "Add"),
            BinaryOp::Subtract => write!(f, "Subtract"),
            BinaryOp::Multiply => write!(f, "Multiply"),
            BinaryOp::Divide => write!(f, "Divide"),
            BinaryOp::Remainder => write!(f, "Remainder"),
        }
    }
}

impl AstNode for Program {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln("TackyIR_Program").unwrap();
        printer.indent();
        for function in &self.functions {
            function.pretty_print(printer);
            printer.writeln("").unwrap();
        }
        printer.unindent();
    }
}

impl AstNode for Function {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln(&format!("{}:", self.name)).unwrap();

        printer.indent();
        for instruction in &self.body {
            instruction.pretty_print(printer);
        }
        printer.unindent();
    }
}

impl AstNode for Instruction {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        let line = match self {
            Instruction::Return(val) => {
                format!("return {}", val)
            }
            Instruction::Unary { op, src, dst } => {
                format!("{} = {} {}", dst, op, src)
            }
            Instruction::Binary {
                op,
                src1,
                src2,
                dst,
            } => {
                format!("{} = {} {} {}", dst, op, src1, src2)
            }
        };
        printer.writeln(&line).unwrap();
    }
}

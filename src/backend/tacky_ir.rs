// src/backend/tacky_ir.rs

use crate::common::{AstNode, PrettyPrinter};
use std::fmt;

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
    Copy {
        src: Value,
        dst: Value,
    },
    Jump(String),
    JumpIfZero {
        condition: Value,
        target: String,
    },
    JumpIfNotZero {
        condition: Value,
        target: String,
    },
    Label(String),
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
    Not,
}
#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    EqualEqual,
    BangEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
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
            // ~ 用于按位取反
            UnaryOp::Complement => write!(f, "~"),
            // - 用于算术取负
            UnaryOp::Negate => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Subtract => write!(f, "-"),
            BinaryOp::Multiply => write!(f, "*"),
            BinaryOp::Divide => write!(f, "/"),
            BinaryOp::Remainder => write!(f, "%"),
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
                format!("{} = {} {} {}", dst, src1, op, src2)
            }
            Instruction::Copy { src, dst } => {
                format!("Copy {} {}", src, dst)
            }
            Instruction::Jump(s) => {
                format!("Jump {}", s)
            }
            Instruction::JumpIfZero { condition, target } => {
                format!("JumpIfZero {} {}", condition, target)
            }
            Instruction::JumpIfNotZero { condition, target } => {
                format!("JumpIfNotZero {} {}", condition, target)
            }
            Instruction::Label(t) => {
                format!("{}:", t)
            }
        };
        // Labels shouldn't be indented like other instructions
        if let Instruction::Label(_) = self {
            printer.unindent();
            printer.writeln(&line).unwrap();
            printer.indent();
        } else {
            printer.writeln(&line).unwrap();
        }
    }
}

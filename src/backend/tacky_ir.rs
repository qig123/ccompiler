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
    Unary { op: UnaryOp, src: Value, dst: Value },
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

impl AstNode for Program {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln("TackyIR_Program");
        printer.indent();
        for function in &self.functions {
            function.pretty_print(printer);
            printer.writeln("");
        }
        printer.unindent();
    }
}

impl AstNode for Function {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln(&format!("{}:", self.name));

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
        };
        printer.writeln(&line);
    }
}

// 注意：我们不需要为 Value 和 UnaryOp 实现 AstNode，
// 因为它们不是独立的树节点，而是作为指令的一部分被打印。
// 为它们实现 Display trait 是更合适的选择。

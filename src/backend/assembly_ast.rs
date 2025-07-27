use crate::common::{AstNode, PrettyPrinter};

// src/backend/assembly_ast.rs
#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Mov { src: Operand, dst: Operand },
    Unary { op: UnaryOp, operand: Operand },
    AllocateStack(i64),
    Ret,
}
#[derive(Debug, Clone)]
pub enum UnaryOp {
    Not, //按位取反
    Neg,
}

#[derive(Debug, Clone)]
pub enum Operand {
    Imm(i64),
    Register(Reg),
    Pseudo(String),
    Stack(i64),
}
#[derive(Debug, Clone)]
pub enum Reg {
    AX,
    R10,
}
impl AstNode for Program {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln("AssemblyProgram");
        printer.indent();
        for function in &self.functions {
            function.pretty_print(printer);
        }
        printer.unindent();
    }
}

impl AstNode for Function {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln(&format!("Function(name: .{})", self.name));
        printer.indent();
        for instruction in &self.instructions {
            instruction.pretty_print(printer);
        }
        printer.unindent();
    }
}

impl AstNode for Instruction {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        match self {
            Instruction::Mov { src, dst } => {
                printer.writeln(&format!("mov {}, {}", src.to_string(), dst.to_string()));
            }
            Instruction::Ret => {
                printer.writeln("ret");
            }
            _ => {
                panic!()
            }
        }
    }
}

impl ToString for Operand {
    fn to_string(&self) -> String {
        match self {
            Operand::Imm(val) => format!("${}", val),
            Operand::Register(_r) => "%eax".to_string(),
            _ => {
                panic!()
            }
        }
    }
}

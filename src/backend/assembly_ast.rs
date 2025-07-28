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
    Mov {
        src: Operand,
        dst: Operand,
    },
    Unary {
        op: UnaryOp,
        operand: Operand,
    },
    Binary {
        op: BinaryOp,
        left_operand: Operand,
        right_operand: Operand,
    },
    Cmp {
        operand1: Operand,
        operand2: Operand,
    },
    Idiv(Operand),
    Cdq,
    Jmp(String),
    JmpCC {
        condtion: ConditionCode,
        target: String,
    },
    SetCC {
        conditin: ConditionCode,
        operand: Operand,
    },
    Label(String),
    AllocateStack(i64),
    Ret,
}
#[derive(Debug, Clone)]
pub enum ConditionCode {
    E,
    NE,
    G,
    GE,
    L,
    LE,
}
#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
}
#[derive(Debug, Clone)]
pub enum UnaryOp {
    Complement, //按位取反
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
    DX,
    R10,
    R11,
}
//--------------打印逻辑

impl AstNode for Program {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln("AssemblyProgram").unwrap();
        printer.indent();
        for function in &self.functions {
            function.pretty_print(printer);
        }
        printer.unindent();
    }
}

impl AstNode for Function {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer
            .writeln(&format!("Function(name: {})", self.name))
            .unwrap();
        printer.indent();
        for instruction in &self.instructions {
            instruction.pretty_print(printer);
        }
        printer.unindent();
    }
}

// in impl AstNode for Instruction
impl AstNode for Instruction {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln(&format!("{:?}", self)).unwrap();
    }
}

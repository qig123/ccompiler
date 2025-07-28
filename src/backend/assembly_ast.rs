use std::fmt::{self};

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
    Idiv(Operand),
    Cdq, //拓展eax
    AllocateStack(i64),
    Ret,
}
#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
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

impl AstNode for Instruction {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        let line = self.to_string();
        printer.writeln(&line).unwrap();
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // movl src, dst
            Instruction::Mov { src, dst } => write!(f, "movl {}, {}", src, dst),
            // negl operand
            Instruction::Unary { op, operand } => write!(f, "{} {}", op, operand),
            // subq $N, %rsp
            Instruction::AllocateStack(size) => write!(f, "subq ${}, %rsp", size),
            // ret
            Instruction::Ret => write!(f, "ret"),
            Instruction::Binary {
                op,
                left_operand,
                right_operand,
            } => write!(f, "{} {} {}", op, left_operand, right_operand),

            Instruction::Cdq => write!(f, "cdq"),
            Instruction::Idiv(operand) => write!(f, "idivl {}", operand),
        }
    }
}
impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 根据上下文，AX 可以是 rax, eax, ax, al
        // R10 可以是 r10, r10d, r10w, r10b
        // 为了简单和与32位兼容，我们这里使用 `e` 和 `d` 后缀
        match self {
            Reg::AX => write!(f, "%eax"),
            Reg::R10 => write!(f, "%r10d"),
            Reg::DX => write!(f, "%edx"),
            Reg::R11 => write!(f, "%r11d"),
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Not => write!(f, "notl"), // 'l' 后缀表示 long (32-bit)
            UnaryOp::Neg => write!(f, "negl"),
        }
    }
}
impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "addl"),
            BinaryOp::Subtract => write!(f, "subl"),
            BinaryOp::Multiply => write!(f, "imul"),
        }
    }
}
impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // 立即数: $5
            Operand::Imm(val) => write!(f, "${}", val),
            // 寄存器: %eax
            Operand::Register(reg) => write!(f, "{}", reg),
            // 伪寄存器 (用于调试，通常不出现在最终代码)
            Operand::Pseudo(name) => write!(f, "%{}", name),
            // 栈操作数: -4(%rbp)
            Operand::Stack(offset) => write!(f, "{}(%rbp)", offset),
        }
    }
}

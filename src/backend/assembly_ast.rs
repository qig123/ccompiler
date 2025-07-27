use std::fmt;

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
        printer.writeln(&format!("Function(name: {})", self.name));
        printer.indent();
        for instruction in &self.instructions {
            instruction.pretty_print(printer);
        }
        printer.unindent();
    }
}

impl AstNode for Instruction {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        // 使用一个 let 绑定来构建字符串，让 match 更干净
        let line = match self {
            // movl src, dst
            Instruction::Mov { src, dst } => {
                format!("movl {}, {}", src, dst) // 直接使用，会自动调用 .to_string()
            }
            // negl operand
            Instruction::Unary { op, operand } => {
                format!("{} {}", op, operand)
            }
            // subq $N, %rsp
            Instruction::AllocateStack(size) => {
                format!("subq ${}, %rsp", size)
            }
            // ret
            Instruction::Ret => "ret".to_string(),
        };

        // 使用 printer 打印带缩进的行
        printer.writeln(&line);
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
impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // 立即数: $5
            Operand::Imm(val) => write!(f, "${}", val),
            // 寄存器: %eax
            Operand::Register(reg) => write!(f, "{}", reg), // 直接调用 Reg 的 Display
            // 伪寄存器 (用于调试，通常不出现在最终代码)
            Operand::Pseudo(name) => write!(f, "%{}", name),
            // 栈操作数: -4(%rbp)
            Operand::Stack(offset) => write!(f, "{}(%rbp)", offset),
        }
    }
}

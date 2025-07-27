use crate::common::{AstNode, PrettyPrinter};

// src/backend/assembly_ast.rs
#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub enum Instruction {
    Mov { src: Operand, dst: Operand },
    Ret,
}

#[derive(Debug, Clone)]
pub enum Operand {
    Imm(i64),
    Register,
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
                // 对于指令，我们不想增加缩进，而是直接打印
                printer.writeln(&format!("mov {}, {}", src.to_string(), dst.to_string()));
            }
            Instruction::Ret => {
                printer.writeln("ret");
            }
        }
    }
}

// 为 Operand 实现 Display trait 会让打印更方便
// 或者直接写一个 to_string 方法
impl ToString for Operand {
    fn to_string(&self) -> String {
        match self {
            Operand::Imm(val) => format!("${}", val),
            Operand::Register => "%eax".to_string(),
        }
    }
}

// 注意：我们不需要为 Operand 实现 AstNode，因为它不是一个独立的树节点，
// 而是指令的一部分。直接在指令的打印逻辑中处理它更简单。
// 如果你想，当然也可以为它实现 AstNode。

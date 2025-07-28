// backend/code_gen.rs

use crate::backend::assembly_ast::{
    BinaryOp, ConditionCode, Function, Instruction, Operand, Program, Reg, UnaryOp,
};
use std::fs::File;
use std::io::{self, BufWriter, Write};
/// x86-64 指令后缀（表示操作数大小）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionSuffix {
    Byte,
    Long,
}
pub struct CodeGenerator {}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {}
    }

    pub fn generate_program_to_file(
        &self,
        program: &Program,
        file_name: &str,
    ) -> Result<(), String> {
        let file = File::create(file_name).map_err(|e| format!("无法创建文件: {}", e))?;
        let mut writer = BufWriter::new(file);
        self.emit_program(program, &mut writer)
            .map_err(|e| e.to_string())
    }

    fn emit_program(&self, program: &Program, writer: &mut impl Write) -> io::Result<()> {
        for function in &program.functions {
            self.emit_function(function, writer)?;
            writeln!(writer)?; // 函数间空行
        }
        writeln!(writer, ".section .note.GNU-stack,\"\",@progbits")?;
        Ok(())
    }

    fn emit_function(&self, function: &Function, writer: &mut impl Write) -> io::Result<()> {
        // --- 函数元信息 ---
        writeln!(writer, "    .globl {}", function.name)?;
        writeln!(writer, "{}:", function.name)?;

        // --- 函数序言 ---
        self.emit_indented("pushq %rbp", writer)?;
        self.emit_indented("movq %rsp, %rbp", writer)?;

        // --- 函数体 ---
        for instruction in &function.instructions {
            self.emit_instruction(instruction, writer)?;
        }

        Ok(())
    }

    fn emit_instruction(
        &self,
        instruction: &Instruction,
        writer: &mut impl Write,
    ) -> io::Result<()> {
        match instruction {
            // 对于其他所有指令，我们都希望有缩进
            Instruction::Mov { src, dst } => {
                let line = format!(
                    "movl {}, {}",
                    self.format_operand(src, InstructionSuffix::Long),
                    self.format_operand(dst, InstructionSuffix::Long)
                );
                self.emit_indented(&line, writer)?;
            }
            Instruction::Unary { op, operand } => {
                let op_str = match op {
                    UnaryOp::Neg => "negl",
                    UnaryOp::Complement => "notl",
                };
                let line = format!(
                    "{} {}",
                    op_str,
                    self.format_operand(operand, InstructionSuffix::Long)
                );
                self.emit_indented(&line, writer)?;
            }
            Instruction::AllocateStack(size) => {
                self.emit_indented(&format!("subq ${}, %rsp", size), writer)?;
            }

            Instruction::Ret => {
                self.emit_indented("movq %rbp, %rsp", writer)?;
                self.emit_indented("popq %rbp", writer)?;
                self.emit_indented("ret", writer)?;
            }
            Instruction::Binary {
                op,
                left_operand,
                right_operand,
            } => {
                let op_str = match op {
                    BinaryOp::Add => "addl",
                    BinaryOp::Subtract => "subl",
                    BinaryOp::Multiply => "imull",
                };
                let src = self.format_operand(left_operand, InstructionSuffix::Long);
                let dst = self.format_operand(right_operand, InstructionSuffix::Long);
                self.emit_indented(&format!("{} {}, {}", op_str, src, dst), writer)?;
            }
            Instruction::Idiv(operand) => {
                let opr = self.format_operand(operand, InstructionSuffix::Long);
                self.emit_indented(&format!("idivl {}", opr), writer)?;
            }
            Instruction::Cdq => {
                self.emit_indented("cdq", writer)?;
            }
            Instruction::Cmp { operand1, operand2 } => {
                let opr1: String = self.format_operand(operand1, InstructionSuffix::Long);
                let opr2: String = self.format_operand(operand2, InstructionSuffix::Long);
                self.emit_indented(&format!("cmpl {}, {}", opr1, opr2), writer)?;
            }
            Instruction::Jmp(name) => {
                self.emit_indented(&format!("jmp .L{}", name), writer)?;
            }
            Instruction::JmpCC { condtion, target } => {
                let c = self.format_condition(condtion);
                self.emit_indented(&format!("j{} .L{}", c, target), writer)?;
            }
            Instruction::SetCC { conditin, operand } => {
                //当寄存器出现在 SetCC 中时，输出 1 字节的名称，在其他任何地方都输出 4 字节的名称
                let c = self.format_condition(conditin);
                let opr = self.format_operand(operand, InstructionSuffix::Byte);
                self.emit_indented(&format!("set{} {}", c, opr), writer)?;
            }
            Instruction::Label(t) => {
                self.emit_indented(&format!(".L{}:", t), writer)?;
            }
        }
        Ok(())
    }

    // --- 辅助函数 ---

    /// 统一处理带缩进的写入
    fn emit_indented(&self, line: &str, writer: &mut impl Write) -> io::Result<()> {
        writeln!(writer, "    {}", line)
    }

    /// 格式化操作数
    fn format_operand(&self, operand: &Operand, s: InstructionSuffix) -> String {
        match operand {
            Operand::Imm(val) => format!("${}", val),
            Operand::Register(reg) => self.format_reg(reg, s),
            Operand::Stack(offset) => format!("{}(%rbp)", offset),
            Operand::Pseudo(_) => {
                panic!("伪寄存器不应出现在最终代码生成阶段");
            }
        }
    }
    fn format_condition(&self, code: &ConditionCode) -> String {
        match code {
            ConditionCode::E => {
                format!("e")
            }
            ConditionCode::NE => {
                format!("ne")
            }
            ConditionCode::G => {
                format!("g")
            }
            ConditionCode::GE => {
                format!("ge")
            }
            ConditionCode::L => {
                format!("l")
            }
            ConditionCode::LE => {
                format!("le")
            }
        }
    }

    fn format_reg(&self, reg: &Reg, s: InstructionSuffix) -> String {
        match s {
            InstructionSuffix::Byte => match reg {
                Reg::AX => "%al".to_string(),
                Reg::DX => "%dl".to_string(),
                Reg::R10 => "%r10b".to_string(),
                Reg::R11 => "%r11b".to_string(),
            },

            InstructionSuffix::Long => match reg {
                Reg::AX => "%eax".to_string(),
                Reg::DX => "%edx".to_string(),
                Reg::R10 => "%r10d".to_string(),
                Reg::R11 => "%r11d".to_string(),
            },
        }
    }
}

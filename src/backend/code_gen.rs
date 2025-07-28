// backend/code_gen.rs

use crate::backend::assembly_ast::{Function, Instruction, Operand, Program, Reg, UnaryOp};
use std::fs::File;
use std::io::{self, BufWriter, Write};

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
                    self.format_operand(src),
                    self.format_operand(dst)
                );
                self.emit_indented(&line, writer)?;
            }
            Instruction::Unary { op, operand } => {
                let op_str = match op {
                    UnaryOp::Neg => "negl",
                    UnaryOp::Not => "notl",
                };
                let line = format!("{} {}", op_str, self.format_operand(operand));
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
            _ => {
                panic!()
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
    fn format_operand(&self, operand: &Operand) -> String {
        match operand {
            Operand::Imm(val) => format!("${}", val),
            Operand::Register(reg) => self.format_reg(reg),
            Operand::Stack(offset) => format!("{}(%rbp)", offset),
            Operand::Pseudo(_) => {
                // 在代码生成阶段，不应该再有伪寄存器
                // 如果出现，说明之前的编译趟有 bug
                panic!("伪寄存器不应出现在最终代码生成阶段");
            }
        }
    }

    /// 格式化寄存器
    fn format_reg(&self, reg: &Reg) -> String {
        match reg {
            Reg::AX => "%eax".to_string(),
            Reg::R10 => "%r10d".to_string(),
            // 如果未来添加了 RSP, RBP, 需要在这里处理
            _ => {
                panic!()
            }
        }
    }
}

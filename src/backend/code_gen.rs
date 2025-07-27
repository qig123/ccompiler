// backend/code_gen.rs

use crate::backend::assembly_ast::{Function, Instruction, Operand, Program, Reg, UnaryOp};
use std::fs::File;
use std::io::{self, BufWriter, Write};

pub struct CodeGenerator {}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {}
    }

    /// 主入口：将汇编 AST 生成到指定的文件中。
    pub fn generate_program_to_file(
        &self,
        program: &Program,
        file_name: &str,
    ) -> Result<(), String> {
        let file =
            File::create(file_name).map_err(|e| format!("无法创建文件 {}: {}", file_name, e))?;
        let mut writer = BufWriter::new(file);

        self.emit_program(program, &mut writer)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn emit_program(&self, program: &Program, writer: &mut impl Write) -> io::Result<()> {
        for function in &program.functions {
            self.emit_function(function, writer)?;
            writeln!(writer)?;
        }
        writeln!(writer, ".section .note.GNU-stack,\"\",@progbits")?;
        Ok(())
    }

    fn emit_function(&self, function: &Function, writer: &mut impl Write) -> io::Result<()> {
        let function_name = &function.name;
        writeln!(writer, ".globl {}", function_name)?;
        writeln!(writer, "{}:", function_name)?;
        //pushq %rbp  movq {@}%rsp, %rbp
        writeln!(writer, "    pushq %rbp",)?;
        writeln!(writer, "    movq %rsp, %rbp",)?;

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
            Instruction::Mov { src, dst } => {
                writeln!(
                    writer,
                    "    movl {}, {}",
                    self.format_operand(src),
                    self.format_operand(dst)
                )?;
            }
            Instruction::Ret => {
                writeln!(writer, "    movq %rbp, %rsp",)?;
                writeln!(writer, "    popq %rbp",)?;
                writeln!(writer, "    ret")?;
            }
            Instruction::AllocateStack(i) => {
                writeln!(writer, "    subq ${}, %rsp", i)?;
            }
            Instruction::Unary { op, operand } => {
                let ass_op = match op {
                    UnaryOp::Neg => "negl",
                    UnaryOp::Not => "notl",
                };
                writeln!(
                    writer,
                    "{}",
                    format!("    {}  {}", ass_op, self.format_operand(operand))
                )?;
            }
        };
        Ok(())
    }

    fn format_operand(&self, operand: &Operand) -> String {
        match operand {
            Operand::Imm(val) => format!("${}", val),
            Operand::Register(r) => self.format_reg(r),
            Operand::Stack(i) => format!("{}(%rbp)", i),
            _ => {
                unreachable!()
            }
        }
    }
    fn format_reg(&self, r: &Reg) -> String {
        match r {
            Reg::AX => "%eax".to_string(),
            Reg::R10 => "%r10d".to_string(),
        }
    }
}

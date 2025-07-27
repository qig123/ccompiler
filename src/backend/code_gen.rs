// backend/code_gen.rs

use crate::backend::assembly_ast::Program;
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

    // 这个函数现在是核心，它使用 Display trait 来生成代码
    fn emit_program(&self, program: &Program, writer: &mut impl Write) -> io::Result<()> {
        for function in &program.functions {
            // --- 函数序言 ---
            writeln!(writer, "    .globl {}", function.name)?;
            writeln!(writer, "{}:", function.name)?;
            writeln!(writer, "    pushq %rbp")?;
            writeln!(writer, "    movq %rsp, %rbp")?;

            // --- 函数体 ---
            for instruction in &function.instructions {
                // 直接打印 instruction！它会自动调用 Display::fmt
                // 我们添加缩进
                writeln!(writer, "    {}", instruction)?;
            }

            // --- 函数结尾 ---
            // 注意：结尾逻辑现在从 Instruction::Ret 的打印中分离出来了，
            // 因为它属于函数结构，而不是 ret 指令本身。
            writeln!(writer, "    movq %rbp, %rsp")?;
            writeln!(writer, "    popq %rbp")?;
            writeln!(writer, "    ret")?;

            writeln!(writer)?; // 函数间空行
        }

        writeln!(writer, ".section .note.GNU-stack,\"\",@progbits")?;
        Ok(())
    }
}

// **注意**: `emit_function`, `emit_instruction`, `format_operand`, `format_reg`
// 这些函数全都被删除了！它们的逻辑被 `Display` trait 的实现和新的 `emit_program` 吸收了。

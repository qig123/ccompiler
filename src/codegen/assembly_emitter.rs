// codegen/emitter.rs (或者你 CodeEmitter 所在的模块)

use crate::codegen::assembly_ir::{
    self as assembly_ir, // Re-export the module itself for clarity
    Assemble,
    Operand,
    Reg,
    UnaryOperator as AssUnaryOperator,
};
use crate::error::{CodeEmitterError, CompilerError};
use std::fs::File;
use std::io::Write;

pub struct CodeEmitter;

impl CodeEmitter {
    pub fn emit(
        assembly_ast: &Assemble,
        output_path: &std::path::Path,
    ) -> Result<(), CompilerError> {
        let mut file = File::create(output_path).map_err(|e| {
            CompilerError::Io(format!(
                "Failed to create output file '{}': {}",
                output_path.display(),
                e
            ))
        })?;

        // Write file header and section directives
        CodeEmitter::write_line(
            &mut file,
            &format!(
                "\t.file\t\"{}\"",
                output_path
                    .file_name()
                    .unwrap_or_else(|| output_path.as_ref())
                    .to_string_lossy()
            ),
            "Failed to write .file directive",
        )?;
        CodeEmitter::write_line(&mut file, "\t.text", "Failed to write .text directive")?;
        CodeEmitter::write_line(&mut file, "", "Failed to write newline")?; // Blank line for clarity

        // Your Assemble AST holds only one function according to your ASDL
        // We can iterate over a single-element slice or just access it directly.
        let function = &assembly_ast.function;

        // --- Emit Function Start ---
        CodeEmitter::write_line(
            &mut file,
            &format!("\t.globl\t{}", function.name),
            "Failed to write .globl directive",
        )?;
        CodeEmitter::write_line(
            &mut file,
            &format!("\t.type\t{}, @function", function.name),
            "Failed to write .type directive",
        )?;
        CodeEmitter::write_line(
            &mut file,
            &format!("{}:", function.name),
            "Failed to write function label",
        )?;
        CodeEmitter::write_line(&mut file, "", "Failed to write newline")?;

        // --- Emit Function Prologue (Stack Frame Setup) ---
        // Corresponds to: pushq %rbp; movq %rsp, %rbp
        CodeEmitter::write_line(&mut file, "\tpushq\t%rbp", "Failed to write pushq %rbp")?;
        CodeEmitter::write_line(
            &mut file,
            "\tmovq\t%rsp, %rbp",
            "Failed to write movq %rsp, %rbp",
        )?;

        // --- Emit Function Body Instructions ---
        // We iterate through the Assembly AST instructions
        // let mut allocate_stack_seen = false; // Track if AllocateStack was emitted
        for instruction in &function.instructions {
            match instruction {
                assembly_ir::Instruction::AllocateStack(size) => {
                    if *size > 0 {
                        // Corresponds to: subq $size, %rsp
                        // This instruction should appear after the frame setup if present.
                        // If the translator correctly put it at the beginning of the list,
                        // it will be emitted right after push/mov rbp.
                        CodeEmitter::write_line(
                            &mut file,
                            &format!("\tsubq\t${}, %rsp", size),
                            "Failed to write subq instruction",
                        )?;
                        // allocate_stack_seen = true;
                    } else {
                        // No stack space needed, skip emitting subq
                    }
                }
                assembly_ir::Instruction::Mov { src, dst } => {
                    // Translate operands and emit mov instruction
                    let src_str = CodeEmitter::operand_to_string(src)?;
                    let dst_str = CodeEmitter::operand_to_string(dst)?;
                    // Use movl for 32-bit moves, consistent with -offsets and typical int size
                    CodeEmitter::write_line(
                        &mut file,
                        &format!("\tmovl\t{}, {}", src_str, dst_str),
                        "Failed to write MOV instruction",
                    )?;
                }
                assembly_ir::Instruction::Unary { op, operand } => {
                    // Translate operand and emit unary instruction
                    let operand_str = CodeEmitter::operand_to_string(operand)?;
                    let op_str = match op {
                        AssUnaryOperator::Neg => "negl", // 32-bit negate
                        AssUnaryOperator::Not => "notl", // 32-bit bitwise not
                    };
                    CodeEmitter::write_line(
                        &mut file,
                        &format!("\t{}\t{}", op_str, operand_str),
                        "Failed to write Unary instruction",
                    )?;
                }
                assembly_ir::Instruction::Ret => {
                    // The Ret instruction in the AST marks the logical end of the function.
                    // Before the final 'ret' opcode, we must emit the stack frame teardown.

                    // --- Emit Function Epilogue (Stack Frame Teardown) ---
                    // Corresponds to: movq %rbp, %rsp; popq %rbp
                    // This recovers the stack space allocated by subq and restores the caller's rbp.
                    // The check for allocate_stack_seen is technically not needed if using rbp,
                    // as the movq %rbp, %rsp always brings the stack pointer back to the base,
                    // cleaning up space regardless of whether subq was used.
                    // However, explicitly adding back the allocated size (addq) can sometimes be
                    // an alternative way to clean up, but movq %rbp, %rsp is standard with RBP.
                    CodeEmitter::write_line(
                        &mut file,
                        "\tmovq\t%rbp, %rsp",
                        "Failed to write movq %rbp, %rsp",
                    )?;
                    CodeEmitter::write_line(
                        &mut file,
                        "\tpopq\t%rbp",
                        "Failed to write popq %rbp",
                    )?;

                    // --- Emit Final Return Opcode ---
                    CodeEmitter::write_line(&mut file, "\tret", "Failed to write RET instruction")?;

                    // According to the conversion rules, Ret is the last logical instruction.
                    // We might want to enforce that no more instructions follow this in the AST list.
                    // For now, we just emit and the loop will continue if there are more (which would be wrong).
                }
                _ => {
                    println!("unsupport"); //todo
                }
            }
        }

        // --- Emit Function End Directives ---
        CodeEmitter::write_line(&mut file, "", "Failed to write newline")?;
        CodeEmitter::write_line(
            &mut file,
            &format!("\t.size\t{}, .-{}", function.name, function.name),
            "Failed to write .size directive",
        )?;

        // --- Emit Other Sections (like stack protection) ---
        CodeEmitter::write_line(
            &mut file,
            "\t.section\t.note.GNU-stack,\"\",@progbits",
            "Failed to write stack section directive",
        )?;

        Ok(())
    }

    // Helper to translate an Operand AST node to AT&T assembly syntax string
    fn operand_to_string(operand: &Operand) -> Result<String, CodeEmitterError> {
        match operand {
            Operand::Imm(val) => Ok(format!("${}", val)), // Immediate: $value
            Operand::Reg(reg) => Ok(CodeEmitter::reg_to_string(reg)), // Register: %reg_name
            Operand::Stack(offset) => {
                // Stack address: offset(%rbp)
                // Offset is negative relative to %rbp.
                Ok(format!("{}(%rbp)", offset))
            }
            Operand::Pseudo(id) => {
                // Pseudoregisters should have been replaced by Stack operands in the previous pass.
                // Encountering one here is an error in the compiler's intermediate stages.
                Err(CodeEmitterError {
                    message: format!(
                        "Internal error: Pseudoregister '{}' found in final Assembly AST. Pseudo-to-Stack pass failed?",
                        id
                    ),
                })
            }
        }
    }

    // Helper to translate a Reg AST node to AT&T assembly syntax string (32-bit)
    fn reg_to_string(reg: &Reg) -> String {
        match reg {
            // Using 32-bit register names (eax, r10d) consistent with movl/negl/notl
            // and 4-byte stack allocation. If targeting 64-bit exclusively, use rax, r10.
            Reg::AX => "%eax".to_string(),
            Reg::R10 => "%r10d".to_string(),
            _ => {
                todo!()
            }
        }
    }

    // Helper to write a line to the file and handle potential IO errors
    fn write_line(file: &mut File, line: &str, context: &str) -> Result<(), CompilerError> {
        writeln!(file, "{}", line).map_err(|e| {
            CompilerError::Io(format!("{}: {}", context, e)) // Wrap IO error
            // Or wrap in CodeEmitterError if your CompilerError structure is different
            // CompilerError::CodeEmitter(CodeEmitterError {
            //     message: format!("{}: {}", context, e),
            // })
        })
    }
}

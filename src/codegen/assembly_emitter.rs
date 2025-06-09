// codegen/emitter.rs

use crate::codegen::assembly_ir::{
    self as assembly_ir, Assemble, BinaryOperator as AssBinaryOperator, Operand, Reg,
    UnaryOperator as AssUnaryOperator,
};
use crate::error::{CodeEmitterError, CompilerError}; // Ensure CodeEmitterError is accessible
use std::fs::File;
use std::io::Write;
use std::path::Path; // Use Path instead of std::path::Path

pub struct CodeEmitter;

#[derive(Clone, Copy)]
enum OperandSize {
    Byte, // 8-bit (e.g., %al)
    Long, // 32-bit (e.g., %eax)
          // Quad, // 64-bit (e.g., %rax) - if you need it later
}
impl CodeEmitter {
    pub fn emit(
        assembly_ast: &Assemble,
        output_path: &Path, // Use Path
    ) -> Result<(), CompilerError> {
        let mut file = File::create(output_path).map_err(|e| {
            CompilerError::Io(format!(
                "Failed to create output file '{}': {}",
                output_path.display(),
                e
            ))
        })?;

        // Write file header and section directives
        // Use path.file_name().and_then(|n| n.to_str()) for better error handling
        let filename_str = output_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown_file"); // Provide a fallback if file name is invalid Unicode

        CodeEmitter::write_line(
            &mut file,
            &format!("\t.file\t\"{}\"", filename_str),
            "Failed to write .file directive",
        )?;
        CodeEmitter::write_line(&mut file, "\t.text", "Failed to write .text directive")?;
        CodeEmitter::write_line(&mut file, "", "Failed to write newline")?;

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
        CodeEmitter::write_line(&mut file, "\tpushq\t%rbp", "Failed to write pushq %rbp")?;
        CodeEmitter::write_line(
            &mut file,
            "\tmovq\t%rsp, %rbp",
            "Failed to write movq %rsp, %rbp",
        )?;

        // --- Emit Function Body Instructions ---
        for instruction in &function.instructions {
            match instruction {
                assembly_ir::Instruction::AllocateStack(size) => {
                    if *size > 0 {
                        // This was already correct regarding the comma
                        CodeEmitter::write_line(
                            &mut file,
                            &format!("\tsubq\t${}, %rsp", size),
                            "Failed to write subq instruction",
                        )?;
                    }
                }
                assembly_ir::Instruction::Mov { src, dst } => {
                    let src_str = CodeEmitter::operand_to_string(src, OperandSize::Long)?;
                    let dst_str = CodeEmitter::operand_to_string(dst, OperandSize::Long)?;
                    // This was already correct regarding the comma
                    CodeEmitter::write_line(
                        &mut file,
                        &format!("\tmovl\t{}, {}", src_str, dst_str),
                        "Failed to write MOV instruction",
                    )?;
                }
                assembly_ir::Instruction::Unary { op, operand } => {
                    let operand_str = CodeEmitter::operand_to_string(operand, OperandSize::Long)?;
                    let op_str = match op {
                        AssUnaryOperator::Neg => "negl",
                        AssUnaryOperator::Not => "notl",
                    };
                    // Unary is single operand, no comma needed
                    CodeEmitter::write_line(
                        &mut file,
                        &format!("\t{}\t{}", op_str, operand_str),
                        "Failed to write Unary instruction",
                    )?;
                }
                assembly_ir::Instruction::Ret => {
                    // The Ret instruction is where we place the epilogue
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
                    CodeEmitter::write_line(&mut file, "\tret", "Failed to write RET instruction")?;
                    // After ret, no more instructions should logically follow in the AST
                }
                assembly_ir::Instruction::Binary {
                    op,
                    left_operand,
                    right_operand,
                } => {
                    let left_operand_str =
                        CodeEmitter::operand_to_string(left_operand, OperandSize::Long)?;
                    let right_operand_str =
                        CodeEmitter::operand_to_string(right_operand, OperandSize::Long)?;

                    let op_str = match op {
                        AssBinaryOperator::Add => "addl",
                        AssBinaryOperator::Mult => "imull",
                        AssBinaryOperator::Sub => "subl",
                    };
                    // *** FIX: Add comma between operands ***
                    CodeEmitter::write_line(
                        &mut file,
                        &format!("\t{}\t{}, {}", op_str, left_operand_str, right_operand_str), // Added ", "
                        "Failed to write Binary instruction",
                    )?;
                }
                assembly_ir::Instruction::Idiv(operand) => {
                    let operand_str = CodeEmitter::operand_to_string(operand, OperandSize::Long)?;
                    // Idiv is single operand, no comma needed
                    CodeEmitter::write_line(
                        &mut file,
                        &format!("\tidivl\t{}", operand_str),
                        "Failed to write Idiv instruction",
                    )?;
                }
                assembly_ir::Instruction::Cdq => {
                    // *** FIX: Remove trailing tab, though not an error cause ***
                    CodeEmitter::write_line(
                        &mut file,
                        "\tcdq", // Removed trailing "\t"
                        "Failed to write cdq instruction",
                    )?;
                }
                assembly_ir::Instruction::Cmp {
                    left_operand,
                    right_operand,
                } => {
                    let left_operand_str =
                        CodeEmitter::operand_to_string(left_operand, OperandSize::Long)?;
                    let right_operand_str =
                        CodeEmitter::operand_to_string(right_operand, OperandSize::Long)?;
                    // Cmp is single operand, no comma needed
                    CodeEmitter::write_line(
                        &mut file,
                        &format!("\tcmpl\t{}, {}", left_operand_str, right_operand_str),
                        "Failed to write Cmp instruction",
                    )?;
                }
                assembly_ir::Instruction::Jmp(target) => {
                    // Jmp is unconditional, no operands
                    CodeEmitter::write_line(
                        &mut file,
                        &format!("\tjmp\t.L{}", target),
                        "Failed to write Jmp instruction",
                    )?;
                }
                assembly_ir::Instruction::JmpCC { condition, target } => {
                    let condition_str = match condition {
                        assembly_ir::Condition::E => "je",
                        assembly_ir::Condition::NE => "jne",
                        assembly_ir::Condition::L => "jl",
                        assembly_ir::Condition::LE => "jle",
                        assembly_ir::Condition::G => "jg",
                        assembly_ir::Condition::GE => "jge",
                    };
                    // JmpCC is single operand, no comma needed
                    CodeEmitter::write_line(
                        &mut file,
                        &format!("\t{} .L{}", condition_str, target),
                        "Failed to write JmpCC instruction",
                    )?;
                }
                assembly_ir::Instruction::SetCC { condition, dst } => {
                    //Setcc use 8 bit registers, so we need to ensure dst is a valid 8-bit register
                    // let dst_str = CodeEmitter::operand_to_string(dst)?;
                    let dst_str = CodeEmitter::operand_to_string(dst, OperandSize::Byte)?;
                    let condition_str = match condition {
                        assembly_ir::Condition::E => "sete",
                        assembly_ir::Condition::NE => "setne",
                        assembly_ir::Condition::L => "setl",
                        assembly_ir::Condition::LE => "setle",
                        assembly_ir::Condition::G => "setg",
                        assembly_ir::Condition::GE => "setge",
                    };
                    // SetCC is single operand, no comma needed
                    CodeEmitter::write_line(
                        &mut file,
                        &format!("\t{} {}", condition_str, dst_str),
                        "Failed to write SetCC instruction",
                    )?;
                }
                assembly_ir::Instruction::Label(name) => {
                    // Labels are just written as is, no tab needed
                    CodeEmitter::write_line(
                        &mut file,
                        &format!(".L{}:", name),
                        "Failed to write Label",
                    )?;
                } // _ => {
                  //     // Handle other instructions like Cmp, Jmp, JmpCC, SetCC, Label
                  //     // For simplicity, we assume these are not present in the final Assembly AST
                  //     // since they are typically used for control flow and not in the final assembly.
                  //     // If they were present, we would need to handle them similarly to the above.
                  //     continue; // Skip unsupported instructions
                  // }
            }
        }

        // Note: If the last instruction was NOT Ret, the epilogue won't be emitted!
        // A more robust emitter would ensure epilogue is always emitted before EOF,
        // or rely on the translator ensuring Ret is the last instruction in the list.
        // Assuming Ret is always last for simplicity based on your rules.

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

    // Helper to translate a Reg AST node to AT&T assembly syntax string (32-bit)
    fn reg_to_string(reg: &Reg, size: OperandSize) -> String {
        match size {
            OperandSize::Byte => match reg {
                // 8-bit registers
                Reg::AX => "%al".to_string(),
                Reg::DX => "%dl".to_string(),
                Reg::R10 => "%r10b".to_string(),
                Reg::R11 => "%r11b".to_string(),
            },
            OperandSize::Long => match reg {
                // 32-bit registers
                Reg::AX => "%eax".to_string(),
                Reg::DX => "%edx".to_string(),
                Reg::R10 => "%r10d".to_string(),
                Reg::R11 => "%r11d".to_string(),
            },
        }
    }
    // Helper to translate an Operand AST node to AT&T assembly syntax string
    fn operand_to_string(operand: &Operand, size: OperandSize) -> Result<String, CodeEmitterError> {
        match operand {
            Operand::Imm(val) => Ok(format!("${}", val)),
            Operand::Reg(reg) => Ok(CodeEmitter::reg_to_string(reg, size)), // Pass the size down
            Operand::Stack(offset) => Ok(format!("{}(%rbp)", offset)),
            Operand::Pseudo(id) => Err(CodeEmitterError {
                message: format!(
                    "Internal error: Pseudoregister '{}' found in final Assembly AST.",
                    id
                ),
            }),
        }
    }

    // Helper to write a line to the file and handle potential IO errors
    fn write_line(file: &mut File, line: &str, context: &str) -> Result<(), CompilerError> {
        writeln!(file, "{}", line).map_err(|e| CompilerError::Io(format!("{}: {}", context, e)))
    }
}

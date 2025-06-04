use crate::{
    codegen::assembly_ir::Assemble,
    error::{CodeEmitterError, CompilerError},
};
use std::fs::File;
use std::io::Write;

pub struct CodeEmitter;

impl CodeEmitter {
    pub fn emit(ast: &Assemble, output_path: &std::path::Path) -> Result<(), CompilerError> {
        let mut file = File::create(output_path)
            .map_err(|e| CompilerError::Io(format!("Failed to create file: {}", e)))?;

        let emitter = CodeEmitter;

        // 写入文件头部
        emitter.write_line(
            &mut file,
            &format!("\t.file\t\"{}\"", output_path.display()),
            "Failed to write file header",
        )?;
        emitter.write_line(&mut file, "\t.text", "Failed to write .text directive")?;

        for function in &ast.function {
            // 函数声明
            emitter.write_line(
                &mut file,
                &format!("\t.globl\t{}", function.name),
                "Failed to write function name",
            )?;
            emitter.write_line(
                &mut file,
                &format!("\t.type\t{}, @function", function.name),
                "Failed to write function type",
            )?;
            emitter.write_line(
                &mut file,
                &format!("{}:", function.name),
                "Failed to write function label",
            )?;

            // 函数体
            for instruction in &function.instructions {
                match instruction {
                    crate::codegen::assembly_ir::Instruction::Mov { src, dst } => {
                        let src_str = match src {
                            crate::codegen::assembly_ir::Operand::Imm(val) => format!("${}", val),
                            _ => {
                                return Err(CompilerError::CodeEmitter(CodeEmitterError {
                                    message: "src must be immediate".to_string(),
                                }));
                            }
                        };
                        let dst_str = match dst {
                            crate::codegen::assembly_ir::Operand::Register(reg) => {
                                format!("%{}", reg)
                            }
                            _ => {
                                return Err(CompilerError::CodeEmitter(CodeEmitterError {
                                    message: "dst must be a register".to_string(),
                                }));
                            }
                        };
                        emitter.write_line(
                            &mut file,
                            &format!("\tmov\t{}, {}", src_str, dst_str),
                            "Failed to write MOV instruction",
                        )?;
                    }
                    crate::codegen::assembly_ir::Instruction::Ret => {
                        emitter.write_line(
                            &mut file,
                            "\tret",
                            "Failed to write RET instruction",
                        )?;
                    }
                }
            }

            // 函数结束标记
            emitter.write_line(
                &mut file,
                &format!("\t.size\t{}, .-{}", function.name, function.name),
                "Failed to write function size",
            )?;
        }

        // 栈保护段
        emitter.write_line(
            &mut file,
            "\t.section\t.note.GNU-stack,\"\",@progbits",
            "Failed to write stack section",
        )?;

        Ok(())
    }

    fn write_line(&self, file: &mut File, line: &str, context: &str) -> Result<(), CompilerError> {
        writeln!(file, "{}", line).map_err(|e| {
            CompilerError::CodeEmitter(CodeEmitterError {
                message: format!("{}: {}", context, e),
            })
        })
    }
}

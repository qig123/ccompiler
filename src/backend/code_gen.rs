// gener_code.rs

use std::fs;

use crate::backend::ass_ast::{Function, Instructions, Operand, Program};

pub struct CodeGenerator {}

impl CodeGenerator {
    /// 创建一个新的代码生成器实例。
    pub fn new() -> Self {
        CodeGenerator {}
    }

    /// 主入口：生成整个程序的汇编代码并写入文件。
    ///
    /// # Arguments
    ///
    /// * `program` - 要编译的汇编 AST。
    /// * `file_name` - 输出的 .s 文件名。
    pub fn generate_program_to_file(
        &self,
        program: &Program,
        file_name: &str,
    ) -> Result<(), String> {
        // 1. 调用内部方法生成汇编字符串
        let assembly_code = self.emit_program(program);

        // 2. (可选) 打印到控制台以供调试
        println!("--- Generated Assembly for Linux ---");
        println!("{}", assembly_code.trim_end());
        println!("------------------------------------");

        // 3. 将代码写入文件
        fs::write(file_name, assembly_code)
            .map_err(|e| format!("Failed to write to file '{}': {}", file_name, e))
    }

    /// 遍历 Program AST，生成完整的汇编代码字符串。
    fn emit_program(&self, program: &Program) -> String {
        let mut output = String::new();

        // 遍历并生成每个函数的代码
        for function in &program.functions {
            output.push_str(&self.emit_function(function));
            output.push('\n'); // 在函数之间加一个空行，提高可读性
        }

        // 在文件末尾添加 Linux 特定的 .note.GNU-stack section
        output.push_str(".section .note.GNU-stack,\"\",@progbits\n");

        output
    }

    /// 格式化单个函数定义。
    fn emit_function(&self, function: &Function) -> String {
        let mut output = String::new();

        // 函数名在 Linux 上不需要任何修饰
        let function_name = &function.name;

        // 输出 .globl <name>
        output.push_str(&format!(".globl {}\n", function_name));
        // 输出 <name>:
        output.push_str(&format!("{}:\n", function_name));

        // 遍历并格式化函数内的所有指令
        for instruction in &function.instructions {
            output.push_str(&self.emit_instruction(instruction));
        }

        output
    }

    /// 格式化单条汇编指令。
    fn emit_instruction(&self, instruction: &Instructions) -> String {
        let instruction_str = match instruction {
            Instructions::Mov { src, dst } => {
                format!(
                    "movl {}, {}",
                    self.emit_operand(src),
                    self.emit_operand(dst)
                )
            }
            Instructions::Ret => "ret".to_string(),
        };
        // 为指令添加缩进和换行符，使其更易读
        format!("    {}\n", instruction_str)
    }

    /// 格式化单个操作数。
    fn emit_operand(&self, operand: &Operand) -> String {
        match operand {
            Operand::Imm(val) => format!("${}", val),
            Operand::Register => "%eax".to_string(),
        }
    }
}

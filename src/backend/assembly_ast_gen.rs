// backend/ass_gen.rs

// [注意] 你的 use 语句中可能是 ass_ast，我这里按照之前的讨论改为 assembly_ast
use crate::backend::assembly_ast::{Function, Instruction, Operand, Program};
use crate::frontend::c_ast::{
    Expression, Function as C_Function, Program as C_Program, Statement as C_Statement,
};

/// 负责将 C AST 转换为汇编 AST。
pub struct AssemblyGenerator {}

impl AssemblyGenerator {
    pub fn new() -> Self {
        AssemblyGenerator {}
    }

    /// 主入口：生成整个程序的汇编 AST。
    pub fn generate(&mut self, c_program: &C_Program) -> Result<Program, String> {
        // 1. 初始化一个空的函数列表
        let mut functions: Vec<Function> = Vec::new();

        // 2. 遍历 C AST 中的每一个函数定义
        for c_func in &c_program.functions {
            // 3. 为每个 C 函数生成一个汇编函数
            //    如果生成失败，`?` 会立即让整个 generate 函数返回错误
            let assembly_function = self.generate_function(c_func)?;

            // 4. 将成功生成的汇编函数添加到列表中
            functions.push(assembly_function);
        }

        // 5. 用生成的函数列表构建最终的汇编 Program
        Ok(Program { functions })
    }

    /// 从 C 函数 AST 生成汇编函数 AST。
    fn generate_function(&mut self, c_func: &C_Function) -> Result<Function, String> {
        // 1. 初始化一个空的指令列表，用于收集该函数的所有汇编指令
        let mut instructions: Vec<Instruction> = Vec::new();

        // 2. 遍历 C 函数体中的每一条语句
        for statement in &c_func.body {
            // 3. 为每个 C 语句生成一个指令序列（可能包含多个指令）
            let generated_instructions = self.generate_statement(statement)?;

            // 4. 将生成的指令序列追加（extend）到总的指令列表中
            instructions.extend(generated_instructions);
        }

        // 5. 用函数名和收集到的指令列表构建汇编函数
        Ok(Function {
            name: c_func.name.clone(),
            instructions,
        })
    }

    /// 从单个 C 语句生成一个或多个汇编指令。
    fn generate_statement(&mut self, statement: &C_Statement) -> Result<Vec<Instruction>, String> {
        match statement {
            C_Statement::Return(expr) => {
                // 1. 先为表达式生成对应的操作数
                let return_value_operand = self.generate_expression(expr)?;

                // 2. 构造指令序列
                let instructions = vec![
                    Instruction::Mov {
                        src: return_value_operand,
                        dst: Operand::Register, // ABI 规定返回值放在 %eax/%rax
                    },
                    Instruction::Ret,
                ];
                Ok(instructions)
            } // 当你支持更多语句时，在这里添加 match 分支
              // 例如：
              // C_Statement::Declaration(...) => { /* ... */ }
              // C_Statement::Expression(...) => { /* ... */ }
        }
    }

    /// 从单个 C 表达式生成一个汇编操作数。
    fn generate_expression(&mut self, expr: &Expression) -> Result<Operand, String> {
        match expr {
            Expression::Constant(n) => Ok(Operand::Imm(*n)),
            // 当你支持变量或更复杂的表达式时，在这里添加 match 分支。
            // 对于不支持的表达式，返回一个描述性的错误。
            // _ => Err(format!("不支持的表达式类型: {:?}", expr)),
            _ => {
                panic!()
            }
        }
    }
}

// backend/ass_gen.rs

use crate::backend::assembly_ast::{Function, Instructions, Operand, Program};
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
    /// 接收一个 C 语言的 Program AST，返回一个汇编语言的 Program AST。
    pub fn generate(&mut self, c_program: &C_Program) -> Result<Program, String> {
        // 使用 .iter().map().collect() 的模式来转换函数列表。
        // `collect` 可以巧妙地将 `Iterator<Item=Result<T, E>>` 转换为 `Result<Vec<T>, E>`。
        // 如果任何一个 `generate_function` 调用失败，整个表达式会立即返回 Err。
        let functions = c_program
            .functions
            .iter()
            .map(|c_func| self.generate_function(c_func))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Program { functions })
    }

    /// 从 C 函数 AST 生成汇编函数 AST。
    fn generate_function(&mut self, c_func: &C_Function) -> Result<Function, String> {
        // 使用 flat_map 来处理一个语句列表（可能每个语句生成多个指令）的转换，
        // 并将结果“拍平”到一个单一的指令列表中。
        let instructions = c_func
            .body
            .iter()
            .map(|statement| self.generate_statement(statement)) // 每个 statement 生成 Result<Vec<Instructions>, String>
            .collect::<Result<Vec<Vec<_>>, _>>()? // 收集成 Result<Vec<Vec<Instructions>>, String>
            .into_iter() // 转换为迭代器
            .flatten() // 将 Vec<Vec<Instructions>> 拍平成 Vec<Instructions>
            .collect(); // 最终收集成 Vec<Instructions>

        Ok(Function {
            name: c_func.name.clone(),
            instructions,
        })
    }

    /// 从单个 C 语句生成一个或多个汇编指令。
    fn generate_statement(&mut self, statement: &C_Statement) -> Result<Vec<Instructions>, String> {
        match statement {
            C_Statement::Return(expr) => {
                // 1. 先为表达式生成对应的操作数
                let return_value_operand = self.generate_expression(expr)?;
                // 2. 构造指令序列
                let instructions = vec![
                    Instructions::Mov {
                        src: return_value_operand,
                        dst: Operand::Register, // ABI 规定返回值放在 %eax/%rax
                    },
                    Instructions::Ret,
                ];
                Ok(instructions)
            } // 当你支持更多语句时，在这里添加 case
              // 例如：C_Statement::Expression(...) => { ... }
        }
    }

    /// 从单个 C 表达式生成一个汇编操作数。
    /// 这是简化的版本，复杂的表达式可能需要生成指令序列并将结果放入临时位置（如寄存器）。
    fn generate_expression(&mut self, expr: &Expression) -> Result<Operand, String> {
        match expr {
            Expression::Constant(n) => Ok(Operand::Imm(*n)),
            // 当你支持变量或更复杂的表达式时，在这里添加 case。
            // 例如：Expression::Variable(name) => { ... }
            //      Expression::BinaryOp(...) => { ... }
            // 对于不支持的表达式，返回一个描述性的错误。
            // _ => Err(format!("不支持的表达式类型: {:?}", expr)),
        }
    }
}

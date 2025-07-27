use crate::backend::tacky_ir::*;
use crate::frontend::c_ast;

#[derive(Debug, Default)]
pub struct UniqueNameGenerator {
    counter: u32,
}

impl UniqueNameGenerator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn new_temp_var(&mut self) -> String {
        let current_value = self.counter;
        self.counter += 1;
        format!("tmp{}", current_value)
    }
}

#[derive(Debug)]
pub struct TackyGenerator {
    name_gen: UniqueNameGenerator,
}

impl TackyGenerator {
    pub fn new() -> Self {
        TackyGenerator {
            name_gen: UniqueNameGenerator::new(),
        }
    }

    pub fn generate_tacky(&mut self, c_ast: &c_ast::Program) -> Result<Program, String> {
        let mut fs = Vec::new();
        for item in &c_ast.functions {
            let mut all_instructions = Vec::new();
            for statement in &item.body {
                let instructions = self.generate_tacky_statement(statement)?;
                all_instructions.extend(instructions);
            }

            let f1 = Function {
                name: item.name.clone(),
                body: all_instructions,
            };
            fs.push(f1);
        }
        Ok(Program { functions: fs })
    }

    fn generate_tacky_statement(
        &mut self,
        c_stat: &c_ast::Statement,
    ) -> Result<Vec<Instruction>, String> {
        match c_stat {
            c_ast::Statement::Return(exp) => {
                let (mut instructions, result_value) = self.generate_tacky_exp(exp)?;
                instructions.push(Instruction::Return(result_value));
                Ok(instructions)
            }
        }
    }

    /// 修改后的核心函数
    /// 返回: (生成的指令列表, 表达式结果存放的 Value)
    fn generate_tacky_exp(
        &mut self,
        exp: &c_ast::Expression,
    ) -> Result<(Vec<Instruction>, Value), String> {
        match exp {
            c_ast::Expression::Constant(i) => Ok((Vec::new(), Value::Constant(*i))),

            // 递归情况：一元运算
            c_ast::Expression::Unary { op, exp } => {
                let (mut instructions, src_value) = self.generate_tacky_exp(exp)?;
                let dst_var_name = self.name_gen.new_temp_var();
                let dst_value = Value::Var(dst_var_name);
                let tacky_op = match op {
                    c_ast::UnaryOp::Complement => UnaryOp::Complement,
                    c_ast::UnaryOp::Negate => UnaryOp::Negate,
                };
                instructions.push(Instruction::Unary {
                    op: tacky_op,
                    src: src_value,
                    dst: dst_value.clone(),
                });
                Ok((instructions, dst_value))
            }
            c_ast::Expression::Binary { op, left, right } => {
                let (mut instructions1, src1_value) = self.generate_tacky_exp(left)?;
                let (instructions2, src2_value) = self.generate_tacky_exp(right)?;
                let dst_var_name = self.name_gen.new_temp_var();
                let dst_value = Value::Var(dst_var_name);
                let tacky_op = match op {
                    c_ast::BinaryOp::Add => BinaryOp::Add,
                    c_ast::BinaryOp::Subtract => BinaryOp::Subtract,
                    c_ast::BinaryOp::Multiply => BinaryOp::Multiply,
                    c_ast::BinaryOp::Divide => BinaryOp::Divide,
                    c_ast::BinaryOp::Remainder => BinaryOp::Remainder,
                };
                instructions1.extend(instructions2);
                instructions1.push(Instruction::Binary {
                    op: tacky_op,
                    src1: src1_value,
                    src2: src2_value,
                    dst: dst_value.clone(),
                });
                Ok((instructions1, dst_value))
            }
        }
    }
}

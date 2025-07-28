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
    pub fn new_temp_label(&mut self) -> String {
        let current_value = self.counter;
        self.counter += 1;
        format!("label{}", current_value)
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
                    c_ast::UnaryOp::Not => UnaryOp::Not,
                };
                instructions.push(Instruction::Unary {
                    op: tacky_op,
                    src: src_value,
                    dst: dst_value.clone(),
                });
                Ok((instructions, dst_value))
            }
            c_ast::Expression::Binary { op, left, right } => match op {
                c_ast::BinaryOp::And => {
                    // 1. 计算 e1 -> v1
                    let (mut instructions, v1) = self.generate_tacky_exp(left)?;
                    let label_false = self.name_gen.new_temp_label();
                    let label_end = self.name_gen.new_temp_label();

                    // 2. 短路跳转: if (v1 == 0) goto false_label
                    instructions.push(Instruction::JumpIfZero {
                        condition: v1,
                        target: label_false.clone(),
                    });

                    // 3. 计算 e2 -> v2
                    let (instructions2, v2) = self.generate_tacky_exp(right)?;
                    instructions.extend(instructions2);

                    // 4. 短路跳转: if (v2 == 0) goto false_label
                    instructions.push(Instruction::JumpIfZero {
                        condition: v2,
                        target: label_false.clone(),
                    });

                    // 5. 创建唯一的结果变量
                    let result_var = self.name_gen.new_temp_var();
                    let result = Value::Var(result_var.clone());

                    // 6. 真分支: result = 1
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(1),
                        dst: result.clone(),
                    });

                    // 7. 跳转到结束
                    instructions.push(Instruction::Jump(label_end.clone()));
                    // 8. 假分支标签: result = 0
                    instructions.push(Instruction::Label(label_false));
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(0),
                        dst: result.clone(),
                    });
                    // 9. 结束标签
                    instructions.push(Instruction::Label(label_end));
                    Ok((instructions, result))
                }
                c_ast::BinaryOp::Or => {
                    // 1. 计算 e1 -> v1
                    let (mut instructions, v1) = self.generate_tacky_exp(left)?;
                    let label_true = self.name_gen.new_temp_label(); // 真分支标签
                    let label_end = self.name_gen.new_temp_label();

                    // 2. 短路跳转: if (v1 != 0) goto true_label
                    instructions.push(Instruction::JumpIfNotZero {
                        condition: v1,
                        target: label_true.clone(),
                    });

                    // 3. 计算 e2 -> v2
                    let (instructions2, v2) = self.generate_tacky_exp(right)?;
                    instructions.extend(instructions2);

                    // 4. 短路跳转: if (v2 != 0) goto true_label
                    instructions.push(Instruction::JumpIfNotZero {
                        condition: v2,
                        target: label_true.clone(),
                    });

                    // 5. 创建结果变量
                    let result_var = self.name_gen.new_temp_var();
                    let result = Value::Var(result_var.clone());

                    // 6. 假分支: result = 0 (e1 和 e2 均为假)
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(0),
                        dst: result.clone(),
                    });

                    // 7. 跳转到结束 (避免进入真分支)
                    instructions.push(Instruction::Jump(label_end.clone()));

                    // 8. 真分支标签: result = 1
                    instructions.push(Instruction::Label(label_true));
                    instructions.push(Instruction::Copy {
                        src: Value::Constant(1),
                        dst: result.clone(),
                    });
                    // 9. 结束标签
                    instructions.push(Instruction::Label(label_end));

                    Ok((instructions, result))
                }
                _ => {
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
                        c_ast::BinaryOp::BangEqual => BinaryOp::BangEqual,
                        c_ast::BinaryOp::EqualEqual => BinaryOp::EqualEqual,
                        c_ast::BinaryOp::Greater => BinaryOp::Greater,
                        c_ast::BinaryOp::GreaterEqual => BinaryOp::GreaterEqual,
                        c_ast::BinaryOp::Less => BinaryOp::Less,
                        c_ast::BinaryOp::LessEqual => BinaryOp::LessEqual,
                        _ => unreachable!(),
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
            },
        }
    }
}

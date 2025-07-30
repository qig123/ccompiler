use crate::UniqueNameGenerator;
use crate::backend::tacky_ir::*;
use crate::frontend::c_ast::{self, BlockItem};

#[derive(Debug)]
pub struct TackyGenerator<'a> {
    name_gen: &'a mut UniqueNameGenerator,
}

// A helper enum to make the short-circuiting logic more readable.
enum ShortCircuitJump {
    OnZero,
    OnNotZero,
}

impl<'a> TackyGenerator<'a> {
    pub fn new(g: &'a mut UniqueNameGenerator) -> Self {
        TackyGenerator { name_gen: g }
    }

    pub fn generate_tacky(&mut self, c_ast: &c_ast::Program) -> Result<Program, String> {
        let mut fs = Vec::new();
        for item in &c_ast.functions {
            let mut all_instructions = Vec::new();
            let body_ins = self.generate_block(&item.body)?;
            all_instructions.extend(body_ins);
            //在每个函数体的末尾添加一条额外的 TACKY 指令：Return(Constant(0))
            all_instructions.push(Instruction::Return(Value::Constant(0)));
            let f1 = Function {
                name: item.name.clone(),
                body: all_instructions,
            };
            fs.push(f1);
        }
        Ok(Program { functions: fs })
    }
    fn generate_block(&mut self, b: &c_ast::Block) -> Result<Vec<Instruction>, String> {
        let mut all_instructions = Vec::new();
        for statement in &b.0 {
            match statement {
                BlockItem::D(d) => {
                    let ins = self.generate_tacky_decl(&d)?;
                    all_instructions.extend(ins);
                }
                BlockItem::S(s) => {
                    let instructions = self.generate_tacky_statement(&s)?;
                    all_instructions.extend(instructions)
                }
            }
        }
        Ok(all_instructions)
    }
    fn generate_tacky_decl(&mut self, d: &c_ast::Declaration) -> Result<Vec<Instruction>, String> {
        match &d.init {
            None => {
                let v: Vec<Instruction> = Vec::new();
                Ok(v)
            }
            Some(e) => {
                let (mut instructions, result_value) = self.generate_tacky_exp(&e)?;
                let ins_c = Instruction::Copy {
                    src: result_value,
                    dst: Value::Var(d.name.clone()),
                };
                instructions.push(ins_c);
                Ok(instructions)
            }
        }
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
            c_ast::Statement::Null => {
                let v: Vec<Instruction> = Vec::new();
                Ok(v)
            }
            c_ast::Statement::Expression(e) => {
                //丢弃表达式的值
                let (instructions, _) = self.generate_tacky_exp(e)?;
                Ok(instructions)
            }
            c_ast::Statement::Compound(b) => Ok(self.generate_block(b)?),
            c_ast::Statement::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                // 策略：统一处理公共部分（条件），然后根据是否存在 else 分支来构建不同的控制流。
                // 同样严格遵循 C 的求值顺序。

                let mut instructions = Vec::new();

                // --- 1. 条件部分 (公共逻辑) ---
                // 首先，且只生成并执行【条件】表达式的指令。
                let (cond_instrs, cond_val) = self.generate_tacky_exp(condition)?;
                instructions.extend(cond_instrs);

                // --- 2. 根据是否存在 else 分支，构建不同的控制流 ---
                match else_stmt {
                    // Case 1: if (condition) { then_stmt }
                    None => {
                        // 只需要一个标签，用于跳过 then_stmt。
                        let end_label = self.name_gen.new_temp_label();

                        // 如果条件为假(0)，则跳过整个 then 块。
                        instructions.push(Instruction::JumpIfZero {
                            condition: cond_val,
                            target: end_label.clone(),
                        });

                        // 生成并添加 then 块的指令。
                        let then_instrs = self.generate_tacky_statement(then_stmt)?;
                        instructions.extend(then_instrs);

                        // 放置结束标签。
                        instructions.push(Instruction::Label(end_label));
                    }

                    // Case 2: if (condition) { then_stmt } else { else_stmt }
                    Some(else_s) => {
                        // 需要两个标签：一个用于跳转到 else，一个用于跳到结尾。
                        let else_label = self.name_gen.new_temp_label();
                        let end_label = self.name_gen.new_temp_label();

                        // 如果条件为假(0)，则跳转到 else 块。
                        instructions.push(Instruction::JumpIfZero {
                            condition: cond_val,
                            target: else_label.clone(),
                        });

                        // [Then 分支]
                        // 生成并添加 then 块的指令。
                        let then_instrs = self.generate_tacky_statement(then_stmt)?;
                        instructions.extend(then_instrs);
                        // then 块执行完毕后，必须无条件跳过 else 块。
                        instructions.push(Instruction::Jump(end_label.clone()));

                        // [Else 分支]
                        // 放置 else 块的入口标签。
                        instructions.push(Instruction::Label(else_label));
                        // 生成并添加 else 块的指令。
                        let else_instrs = self.generate_tacky_statement(else_s)?;
                        instructions.extend(else_instrs);

                        // [结尾]
                        // 放置共同的结束标签。
                        instructions.push(Instruction::Label(end_label));
                    }
                }
                Ok(instructions)
            }
            _ => panic!(),
        }
    }

    /// Generates TACKY IR for short-circuiting binary operators like `&&` and `||`.
    ///
    /// # Arguments
    /// * `left`, `right` - The left and right hand side expressions.
    /// * `jump_type` - The condition on which to short-circuit.
    /// * `short_circuit_val` - The value to assign to the result if we short-circuit.
    /// * `fall_through_val` - The value to assign to the result if we don't short-circuit.
    fn generate_short_circuit_op(
        &mut self,
        left: &c_ast::Expression,
        right: &c_ast::Expression,
        jump_type: ShortCircuitJump,
        short_circuit_val: i64,
        fall_through_val: i64,
    ) -> Result<(Vec<Instruction>, Value), String> {
        // 1. Evaluate left expression
        let (mut instructions, v1) = self.generate_tacky_exp(left)?;

        // 2. Generate labels
        let short_circuit_label = self.name_gen.new_temp_label();
        let end_label = self.name_gen.new_temp_label();

        // 3. Helper function to create the correct jump instruction
        let make_jump = |condition, target| match jump_type {
            ShortCircuitJump::OnZero => Instruction::JumpIfZero { condition, target },
            ShortCircuitJump::OnNotZero => Instruction::JumpIfNotZero { condition, target },
        };

        // 4. Conditional jump for left expression
        instructions.push(make_jump(v1, short_circuit_label.clone()));

        // 5. Evaluate right expression
        let (instrs2, v2) = self.generate_tacky_exp(right)?;
        instructions.extend(instrs2);

        // 6. Conditional jump for right expression
        instructions.push(make_jump(v2, short_circuit_label.clone()));

        // 7. Create result variable
        let result_var = self.name_gen.new_temp_var();
        let result = Value::Var(result_var);

        // 8. Fall-through case (no short-circuit happened)
        instructions.push(Instruction::Copy {
            src: Value::Constant(fall_through_val),
            dst: result.clone(),
        });
        instructions.push(Instruction::Jump(end_label.clone()));

        // 9. Short-circuit case
        instructions.push(Instruction::Label(short_circuit_label));
        instructions.push(Instruction::Copy {
            src: Value::Constant(short_circuit_val),
            dst: result.clone(),
        });

        // 10. End label
        instructions.push(Instruction::Label(end_label));

        Ok((instructions, result))
    }

    /// 修改后的核心函数
    /// 返回: (生成的指令列表, 表达式结果存放的 Value)
    fn generate_tacky_exp(
        &mut self,
        exp: &c_ast::Expression,
    ) -> Result<(Vec<Instruction>, Value), String> {
        match exp {
            c_ast::Expression::Constant(i) => Ok((Vec::new(), Value::Constant(*i))),

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
                c_ast::BinaryOp::And => self.generate_short_circuit_op(
                    left,
                    right,
                    ShortCircuitJump::OnZero, // For &&, we short-circuit if a value is 0
                    0,                        // The result is 0 if we short-circuit
                    1,                        // The result is 1 if we don't (fall-through)
                ),
                c_ast::BinaryOp::Or => self.generate_short_circuit_op(
                    left,
                    right,
                    ShortCircuitJump::OnNotZero, // For ||, we short-circuit if a value is not 0
                    1,                           // The result is 1 if we short-circuit
                    0,                           // The result is 0 if we don't (fall-through)
                ),
                _ => {
                    // All other binary operators that don't short-circuit
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
                        _ => unreachable!("Handled by short-circuiting logic"),
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
            c_ast::Expression::Assignment { left, right } => {
                //  处理左侧表达式，得到目标位置,目前只能是Var
                let (mut instructions_for_dest, dest_value) = self.generate_tacky_exp(left)?;
                let (instructions_for_src, src_value) = self.generate_tacky_exp(right)?;
                instructions_for_dest.extend(instructions_for_src);
                let copy_ins = Instruction::Copy {
                    src: src_value,
                    dst: dest_value.clone(),
                };
                instructions_for_dest.push(copy_ins);
                Ok((instructions_for_dest, dest_value))
            }
            c_ast::Expression::Var(id) => Ok((Vec::new(), Value::Var(id.clone()))),
            c_ast::Expression::Conditional {
                condition,
                left,
                right,
            } => {
                // 策略：遵循 C 语言的短路求值规则，按执行顺序生成指令，
                // 同时通过代码结构化来提高可读性。

                // --- 1. 准备阶段 ---
                // 创建整个表达式所需的共享资源：最终结果的临时变量和跳转标签。
                // 这部分可以安全地提前完成。
                let result_val = Value::Var(self.name_gen.new_temp_var());
                let false_label = self.name_gen.new_temp_label();
                let end_label = self.name_gen.new_temp_label();

                let mut instructions = Vec::new();

                // --- 2. 条件部分 ---
                // 首先，且只生成并执行【条件】表达式的指令。
                let (cond_instrs, cond_val) = self.generate_tacky_exp(condition)?;
                instructions.extend(cond_instrs);

                // 根据条件结果进行跳转。如果为假(0)，则跳过 "then" 分支。
                instructions.push(Instruction::JumpIfZero {
                    condition: cond_val,
                    target: false_label.clone(),
                });

                // --- 3. Then 分支 (当条件为真时执行) ---
                // 只有在确定要执行 "then" 分支时，才为其生成指令。
                // 这保证了 `left` 表达式的副作用只在条件为真时发生。
                let (then_instrs, then_val) = self.generate_tacky_exp(left)?;
                instructions.extend(then_instrs);
                instructions.push(Instruction::Copy {
                    src: then_val,
                    dst: result_val.clone(),
                });
                // "then" 分支执行完毕后，必须无条件跳过 "else" 分支。
                instructions.push(Instruction::Jump(end_label.clone()));

                // --- 4. Else 分支 (当条件为假时执行) ---
                // 放置 "else" 分支的入口标签。
                instructions.push(Instruction::Label(false_label));

                // 只有在确定要执行 "else" 分支时，才为其生成指令。
                // 这保证了 `right` 表达式的副作用只在条件为假时发生。
                let (else_instrs, else_val) = self.generate_tacky_exp(right)?;
                instructions.extend(else_instrs);
                instructions.push(Instruction::Copy {
                    src: else_val,
                    dst: result_val.clone(),
                });

                // --- 5. 结尾 ---
                // 放置 "then" 和 "else" 分支汇合的最终标签。
                instructions.push(Instruction::Label(end_label));

                Ok((instructions, result_val))
            }
        }
    }
}

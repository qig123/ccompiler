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
            for statement in &item.body {
                match statement {
                    BlockItem::D(d) => {
                        let ins = self.generate_tacky_decl(d)?;
                        all_instructions.extend(ins);
                    }
                    BlockItem::S(s) => {
                        let instructions = self.generate_tacky_statement(s)?;
                        all_instructions.extend(instructions)
                    }
                }
            }
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
            _ => {
                panic!()
            }
        }
    }
}

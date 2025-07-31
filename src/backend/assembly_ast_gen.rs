// src/backend/assembly_ast_gen.rs

use std::collections::HashMap;
use std::vec;

use crate::backend::assembly_ast::{
    BinaryOp, ConditionCode, Function, Instruction, Operand, Program, Reg, UnaryOp,
};
use crate::backend::tacky_ir;

/// 负责将 IR AST 转换为汇编 AST。
pub struct AssemblyGenerator {}

// 为 Instruction 添加一个辅助方法，用于遍历和映射其所有操作数。
impl Instruction {
    /// 创建一个新指令，其中每个操作数都通过一个闭包进行映射。
    /// f: &mut impl FnMut(&Operand) -> Operand
    fn map_operands(&self, mut f: impl FnMut(&Operand) -> Operand) -> Instruction {
        match self {
            Instruction::Mov { src, dst } => Instruction::Mov {
                src: f(src),
                dst: f(dst),
            },
            Instruction::Unary { op, operand } => Instruction::Unary {
                op: op.clone(),
                operand: f(operand),
            },
            Instruction::Binary {
                op,
                left_operand,
                right_operand,
            } => Instruction::Binary {
                op: op.clone(),
                left_operand: f(left_operand),
                right_operand: f(right_operand),
            },
            Instruction::Idiv(operand) => Instruction::Idiv(f(operand)),
            Instruction::SetCC { conditin, operand } => Instruction::SetCC {
                conditin: conditin.clone(),
                operand: f(operand),
            },
            Instruction::Cmp { operand1, operand2 } => Instruction::Cmp {
                operand1: f(operand1),
                operand2: f(operand2),
            },
            Instruction::Push(opd) => Instruction::Push(f(opd)),
            // 其他没有操作数的指令直接克隆
            _ => self.clone(),
        }
    }
}

impl AssemblyGenerator {
    pub fn new() -> Self {
        AssemblyGenerator {}
    }

    pub fn generate(&mut self, ir_program: tacky_ir::Program) -> Result<Program, String> {
        let functions = ir_program
            .functions
            .into_iter()
            .map(|ir_func| self.process_function(&ir_func))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Program { functions })
    }

    fn process_function(&mut self, ir_func: &tacky_ir::Function) -> Result<Function, String> {
        // 第 1 步：将 IR 转换为初始汇编指令
        let mut initial_instructions = Vec::new();
        let ins_helper = self.generate_function_helper(ir_func)?;
        initial_instructions.extend(ins_helper);
        let ins = self.generate_initial_instructions(ir_func)?;
        initial_instructions.extend(ins);

        // 第 2 步：替换伪寄存器并计算栈大小
        let (instructions_with_stack, stack_size) =
            self.allocate_stack_slots(&initial_instructions);

        // 第 3 步：修复无效指令 (例如内存到内存的移动)
        let mut final_instructions = self.patch_instructions(&instructions_with_stack);

        // 第 4 步：插入栈分配指令
        if stack_size > 0 {
            // x86-64 要求栈是 16 字节对齐的
            let aligned_stack_size = (stack_size + 15) & !15;
            final_instructions.insert(0, Instruction::AllocateStack(aligned_stack_size));
        }

        Ok(Function {
            name: ir_func.name.clone(),
            instructions: final_instructions,
            stack_size,
        })
    }
    fn generate_function_helper(
        &mut self,
        ir_func: &tacky_ir::Function,
    ) -> Result<Vec<Instruction>, String> {
        let mut ins = Vec::new();

        for (i, param) in ir_func.params.iter().enumerate() {
            let destination = Operand::Pseudo(param.clone());
            let source = if i < 6 {
                // --- 情况1: 前6个参数，通过寄存器传递 ---
                // 使用 match 将索引映射到正确的寄存器
                let register = match i {
                    0 => Reg::DI,
                    1 => Reg::SI,
                    2 => Reg::DX,
                    3 => Reg::CX,
                    4 => Reg::R8,
                    5 => Reg::R9,
                    // 这个分支理论上不可能到达，因为我们有 i < 6 的检查
                    _ => unreachable!(),
                };
                Operand::Register(register)
            } else {
                // --- 情况2: 第7个及以后的参数，通过栈传递 ---
                // 计算相对于基址指针 %rbp 的偏移量
                // 第7个参数 (i=6) 的偏移量是 16
                // 第8个参数 (i=7) 的偏移量是 24 (16 + 8)
                // ...
                let offset = 16 + ((i - 6) * 8) as i64;
                Operand::Stack(offset)
            };
            ins.push(Instruction::Mov {
                src: source,
                dst: destination,
            });
        }
        Ok(ins)
    }

    fn generate_initial_instructions(
        &self,
        ir_func: &tacky_ir::Function,
    ) -> Result<Vec<Instruction>, String> {
        ir_func
            .body
            .iter()
            .map(|ins| self.generate_instruction(ins))
            .collect::<Result<Vec<_>, _>>()
            .map(|vecs| vecs.into_iter().flatten().collect())
    }

    /// (重构后的辅助函数) 为关系运算符和逻辑 NOT 生成指令序列。
    /// 该函数生成标准的 `cmp/setcc/movzbl` 模式。
    fn generate_relational_op_instructions(
        &self,
        op1: &Operand,
        op2: &Operand,
        dst: &Operand,
        cc: ConditionCode,
    ) -> Vec<Instruction> {
        vec![
            // 1. 比较两个操作数
            Instruction::Cmp {
                operand1: op2.clone(),
                operand2: op1.clone(),
            },
            // 2. 根据条件设置字节大小的 AL 寄存器
            Instruction::SetCC {
                conditin: cc,
                operand: Operand::Register(Reg::AX), // SetCC 将使用8位的 %al 部分
            },
            // 3. 将字节从 %al 移动到完整的 %eax 寄存器，并进行零扩展。
            //    我们通过一个从8位源到32位目标的移动来表示这一点。
            //    我们的代码生成器需要处理这个特殊情况。
            Instruction::Mov {
                src: Operand::Register(Reg::AX), // 暗示源是 %al
                dst: Operand::Register(Reg::AX), // 暗示目标是 %eax
            },
            // 4. 将最终结果（在 %eax 中的 0 或 1）移动到目标位置。
            Instruction::Mov {
                src: Operand::Register(Reg::AX),
                dst: dst.clone(),
            },
        ]
    }

    /// 从单个 ir instruction 生成一个或多个汇编指令。
    fn generate_instruction(
        &self,
        ir_incs: &tacky_ir::Instruction,
    ) -> Result<Vec<Instruction>, String> {
        match ir_incs {
            tacky_ir::Instruction::Return(val) => {
                let return_operand = self.generate_expression(val)?;
                Ok(vec![
                    Instruction::Mov {
                        src: return_operand,
                        dst: Operand::Register(Reg::AX),
                    },
                    Instruction::Ret,
                ])
            }
            tacky_ir::Instruction::Unary { op, src, dst } => {
                let src_operand = self.generate_expression(src)?;
                let dst_operand = self.generate_expression(dst)?;
                match op {
                    // 处理 ~ 和 -
                    tacky_ir::UnaryOp::Complement | tacky_ir::UnaryOp::Negate => {
                        let op_type = match op {
                            tacky_ir::UnaryOp::Complement => UnaryOp::Complement,
                            tacky_ir::UnaryOp::Negate => UnaryOp::Neg,
                            _ => unreachable!(),
                        };
                        Ok(vec![
                            Instruction::Mov {
                                src: src_operand,
                                dst: dst_operand.clone(),
                            },
                            Instruction::Unary {
                                op: op_type,
                                operand: dst_operand,
                            },
                        ])
                    }
                    // !x 等价于 x == 0
                    tacky_ir::UnaryOp::Not => Ok(self.generate_relational_op_instructions(
                        &src_operand,
                        &Operand::Imm(0),
                        &dst_operand,
                        ConditionCode::E,
                    )),
                }
            }
            tacky_ir::Instruction::Binary {
                op,
                src1,
                src2,
                dst,
            } => {
                let src1_operand = self.generate_expression(src1)?;
                let src2_operand = self.generate_expression(src2)?;
                let dst_operand = self.generate_expression(dst)?;

                match op {
                    // 除法和取余的特殊情况
                    tacky_ir::BinaryOp::Divide => Ok(vec![
                        Instruction::Mov {
                            src: src1_operand,
                            dst: Operand::Register(Reg::AX),
                        },
                        Instruction::Cdq,
                        Instruction::Idiv(src2_operand),
                        Instruction::Mov {
                            src: Operand::Register(Reg::AX),
                            dst: dst_operand,
                        },
                    ]),
                    tacky_ir::BinaryOp::Remainder => Ok(vec![
                        Instruction::Mov {
                            src: src1_operand,
                            dst: Operand::Register(Reg::AX),
                        },
                        Instruction::Cdq,
                        Instruction::Idiv(src2_operand),
                        Instruction::Mov {
                            src: Operand::Register(Reg::DX),
                            dst: dst_operand,
                        },
                    ]),
                    // 关系运算符现在使用辅助函数
                    tacky_ir::BinaryOp::EqualEqual
                    | tacky_ir::BinaryOp::BangEqual
                    | tacky_ir::BinaryOp::Greater
                    | tacky_ir::BinaryOp::GreaterEqual
                    | tacky_ir::BinaryOp::Less
                    | tacky_ir::BinaryOp::LessEqual => {
                        let cc = match op {
                            tacky_ir::BinaryOp::EqualEqual => ConditionCode::E,
                            tacky_ir::BinaryOp::BangEqual => ConditionCode::NE,
                            tacky_ir::BinaryOp::Greater => ConditionCode::G,
                            tacky_ir::BinaryOp::GreaterEqual => ConditionCode::GE,
                            tacky_ir::BinaryOp::Less => ConditionCode::L,
                            tacky_ir::BinaryOp::LessEqual => ConditionCode::LE,
                            _ => unreachable!(),
                        };
                        Ok(self.generate_relational_op_instructions(
                            &src1_operand,
                            &src2_operand,
                            &dst_operand,
                            cc,
                        ))
                    }
                    // 标准算术运算符
                    _ => {
                        let asm_op = match op {
                            tacky_ir::BinaryOp::Add => BinaryOp::Add,
                            tacky_ir::BinaryOp::Subtract => BinaryOp::Subtract,
                            tacky_ir::BinaryOp::Multiply => BinaryOp::Multiply,
                            _ => unreachable!("应在前面处理"),
                        };
                        Ok(vec![
                            Instruction::Mov {
                                src: src1_operand,
                                dst: dst_operand.clone(),
                            },
                            Instruction::Binary {
                                op: asm_op,
                                left_operand: src2_operand,
                                right_operand: dst_operand,
                            },
                        ])
                    }
                }
            }
            tacky_ir::Instruction::Jump(t) => Ok(vec![Instruction::Jmp(t.clone())]),
            tacky_ir::Instruction::JumpIfZero { condition, target } => {
                let condition_value = self.generate_expression(condition)?;
                Ok(vec![
                    Instruction::Cmp {
                        operand1: Operand::Imm(0),
                        operand2: condition_value,
                    },
                    Instruction::JmpCC {
                        condtion: ConditionCode::E,
                        target: target.clone(),
                    },
                ])
            }
            tacky_ir::Instruction::JumpIfNotZero { condition, target } => {
                let condition_value = self.generate_expression(condition)?;
                Ok(vec![
                    Instruction::Cmp {
                        operand1: Operand::Imm(0),
                        operand2: condition_value,
                    },
                    Instruction::JmpCC {
                        condtion: ConditionCode::NE,
                        target: target.clone(),
                    },
                ])
            }
            tacky_ir::Instruction::Copy { src, dst } => {
                let src_operand = self.generate_expression(src)?;
                let dst_operand = self.generate_expression(dst)?;
                Ok(vec![Instruction::Mov {
                    src: src_operand,
                    dst: dst_operand,
                }])
            }
            tacky_ir::Instruction::Label(t) => Ok(vec![Instruction::Label(t.clone())]),
            tacky_ir::Instruction::FunctionCall { name, args, dst } => {
                let mut ins = Vec::new();
                //对齐
                let num_stack_args = if args.len() > 6 { args.len() - 6 } else { 0 };
                let stack_padding = if num_stack_args % 2 != 0 { 8 } else { 0 };
                if stack_padding != 0 {
                    ins.push(Instruction::AllocateStack(stack_padding));
                }
                //  发射寄存器参数的指令
                let split_idx = std::cmp::min(args.len(), 6);
                let (register_args, stack_args) = args.split_at(split_idx);
                let arg_registers = [Reg::DI, Reg::SI, Reg::DX, Reg::CX, Reg::R8, Reg::R9];
                for (i, tacky_arg) in register_args.iter().enumerate() {
                    let assembly_arg = self.generate_expression(tacky_arg)?;
                    // 因为 register_args.len() <= 6，所以 i 不会越界
                    let target_register = arg_registers[i].clone();
                    ins.push(Instruction::Mov {
                        src: assembly_arg,
                        dst: Operand::Register(target_register),
                    });
                }
                // 4. 发射栈参数的指令
                // 关键：必须反向遍历！
                for tacky_arg in stack_args.iter().rev() {
                    let assembly_arg = self.generate_expression(tacky_arg)?;
                    match assembly_arg {
                        Operand::Register(_) | Operand::Imm(_) => {
                            ins.push(Instruction::Push(assembly_arg));
                        }
                        _ => {
                            ins.push(Instruction::Mov {
                                src: assembly_arg,
                                dst: Operand::Register(Reg::AX),
                            });
                            ins.push(Instruction::Push(Operand::Register(Reg::AX)));
                        }
                    }
                }
                // // 发出 call 指令
                ins.push(Instruction::Call(name.clone()));
                // 调整栈指针
                let stack_args_len_i64 = stack_args.len() as i64;
                let bytes_to_remove: i64 = 8 * stack_args_len_i64 + stack_padding;
                if bytes_to_remove > 0 {
                    ins.push(Instruction::DeallocateStack(bytes_to_remove));
                }
                // 获取返回值
                let assembly_dst = self.generate_expression(dst)?;
                ins.push(Instruction::Mov {
                    src: Operand::Register(Reg::AX),
                    dst: assembly_dst,
                });

                Ok(ins)
            }
        }
    }

    fn generate_expression(&self, v: &tacky_ir::Value) -> Result<Operand, String> {
        match v {
            tacky_ir::Value::Constant(i) => Ok(Operand::Imm(*i)),
            tacky_ir::Value::Var(name) => Ok(Operand::Pseudo(name.clone())),
        }
    }

    fn patch_instructions(&self, instructions: &[Instruction]) -> Vec<Instruction> {
        let mut new_ins = Vec::with_capacity(instructions.len());

        for item in instructions {
            match item {
                // 修复内存到内存的 mov
                Instruction::Mov {
                    src: Operand::Stack(s_off),
                    dst: Operand::Stack(d_off),
                } => {
                    new_ins.push(Instruction::Mov {
                        src: Operand::Stack(*s_off),
                        dst: Operand::Register(Reg::R10),
                    });
                    new_ins.push(Instruction::Mov {
                        src: Operand::Register(Reg::R10),
                        dst: Operand::Stack(*d_off),
                    });
                }
                // 修复 idiv 的立即数操作数
                Instruction::Idiv(Operand::Imm(val)) => {
                    new_ins.push(Instruction::Mov {
                        src: Operand::Imm(*val),
                        dst: Operand::Register(Reg::R10),
                    });
                    new_ins.push(Instruction::Idiv(Operand::Register(Reg::R10)));
                }
                Instruction::Binary {
                    op,
                    left_operand,
                    right_operand,
                } => {
                    match (op, left_operand, right_operand) {
                        // 修复 add/sub 的内存到内存操作
                        (
                            BinaryOp::Add | BinaryOp::Subtract,
                            Operand::Stack(l_off),
                            Operand::Stack(r_off),
                        ) => {
                            new_ins.push(Instruction::Mov {
                                src: Operand::Stack(*l_off),
                                dst: Operand::Register(Reg::R10),
                            });
                            new_ins.push(Instruction::Binary {
                                op: op.clone(),
                                left_operand: Operand::Register(Reg::R10),
                                right_operand: Operand::Stack(*r_off),
                            });
                        }
                        // 修复 imul 的内存目标操作数
                        (BinaryOp::Multiply, _, Operand::Stack(r_off)) => {
                            new_ins.push(Instruction::Mov {
                                src: Operand::Stack(*r_off),
                                dst: Operand::Register(Reg::R11),
                            });
                            new_ins.push(Instruction::Binary {
                                op: BinaryOp::Multiply,
                                left_operand: left_operand.clone(),
                                right_operand: Operand::Register(Reg::R11),
                            });
                            new_ins.push(Instruction::Mov {
                                src: Operand::Register(Reg::R11),
                                dst: Operand::Stack(*r_off),
                            });
                        }
                        // 其他二元操作都是有效的
                        _ => new_ins.push(item.clone()),
                    }
                }
                Instruction::Cmp {
                    operand1: Operand::Stack(s_off),
                    operand2: Operand::Stack(d_off),
                } => {
                    new_ins.push(Instruction::Mov {
                        src: Operand::Stack(*s_off),
                        dst: Operand::Register(Reg::R10),
                    });
                    new_ins.push(Instruction::Cmp {
                        operand1: Operand::Register(Reg::R10),
                        operand2: Operand::Stack(*d_off),
                    });
                }
                Instruction::Cmp {
                    operand1,
                    operand2: Operand::Imm(i),
                } => {
                    new_ins.push(Instruction::Mov {
                        src: Operand::Imm(*i),
                        dst: Operand::Register(Reg::R11),
                    });
                    new_ins.push(Instruction::Cmp {
                        operand1: operand1.clone(),
                        operand2: Operand::Register(Reg::R11),
                    });
                }
                // 其他所有指令都是有效的
                _ => new_ins.push(item.clone()),
            }
        }
        new_ins
    }

    /// 它接受一个指令列表，返回一个新的、替换好伪寄存器的列表和栈大小
    fn allocate_stack_slots(&self, instructions: &[Instruction]) -> (Vec<Instruction>, i64) {
        let mut pseudo_map: HashMap<String, i64> = HashMap::new();
        let mut next_stack_offset = -4; // 第一个变量在 -4(%rbp)

        let mut map_operand_logic = |operand: &Operand| {
            if let Operand::Pseudo(name) = operand {
                let offset = *pseudo_map.entry(name.clone()).or_insert_with(|| {
                    let offset = next_stack_offset;
                    next_stack_offset -= 4;
                    offset
                });
                Operand::Stack(offset)
            } else {
                operand.clone()
            }
        };

        let new_instructions = instructions
            .iter()
            .map(|inst| inst.map_operands(&mut map_operand_logic))
            .collect();

        // 栈大小是分配的变量数 * 4
        let stack_size = pseudo_map.len() as i64 * 4;
        (new_instructions, stack_size)
    }
}

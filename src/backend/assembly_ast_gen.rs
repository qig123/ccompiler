// backend/ass_gen.rs

use std::collections::HashMap;

use crate::backend::assembly_ast::{
    BinaryOp, Function, Instruction, Operand, Program, Reg, UnaryOp,
};
use crate::backend::tacky_ir;
/// 负责将 IR AST 转换为汇编 AST。
pub struct AssemblyGenerator {}

impl AssemblyGenerator {
    pub fn new() -> Self {
        AssemblyGenerator {}
    }

    pub fn generate(&mut self, ir_program: tacky_ir::Program) -> Result<Program, String> {
        let functions = ir_program
            .functions
            .iter()
            .map(|ir_func| self.process_function(ir_func))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Program { functions })
    }
    fn process_function(&mut self, ir_func: &tacky_ir::Function) -> Result<Function, String> {
        // Pass 1: IR -> 初始汇编指令
        let initial_instructions = self.generate_initial_instructions(ir_func)?;

        // Pass 2: 替换伪寄存器并计算栈大小
        let (instructions_with_stack, stack_size) =
            self.allocate_stack_slots(&initial_instructions);

        // Pass 3: 修复内存到内存的 mov 指令
        let mut final_instructions = self.fix_memory_moves(&instructions_with_stack);

        // Pass 4: 插入栈分配指令
        if stack_size > 0 {
            final_instructions.insert(0, Instruction::AllocateStack(stack_size));
        }

        Ok(Function {
            name: ir_func.name.clone(),
            instructions: final_instructions,
        })
    }
    fn generate_initial_instructions(
        &mut self,
        ir_func: &tacky_ir::Function,
    ) -> Result<Vec<Instruction>, String> {
        let mut instructions = Vec::new();
        for ins in &ir_func.body {
            instructions.extend(self.generate_instruction(ins)?);
        }
        Ok(instructions)
    }

    /// 从单个 ir instruction 生成一个或多个汇编指令。
    fn generate_instruction(
        &mut self,
        ir_incs: &tacky_ir::Instruction,
    ) -> Result<Vec<Instruction>, String> {
        let mut ins: Vec<Instruction> = Vec::new();
        match ir_incs {
            tacky_ir::Instruction::Return(expr) => {
                let return_value_operand = self.generate_expression(expr)?;

                let instructions = vec![
                    Instruction::Mov {
                        src: return_value_operand,
                        dst: Operand::Register(Reg::AX),
                    },
                    Instruction::Ret,
                ];
                ins.extend(instructions);
            }
            tacky_ir::Instruction::Unary { op, src, dst } => {
                let src_operand = self.generate_expression(src)?;
                let dst_operand = self.generate_expression(dst)?;
                let op_type = match op {
                    tacky_ir::UnaryOp::Complement => UnaryOp::Not,
                    tacky_ir::UnaryOp::Negate => UnaryOp::Neg,
                };
                let instructions = vec![
                    Instruction::Mov {
                        src: src_operand,
                        dst: dst_operand.clone(),
                    },
                    Instruction::Unary {
                        op: op_type,
                        operand: dst_operand.clone(),
                    },
                ];
                ins.extend(instructions);
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
                    tacky_ir::BinaryOp::Add => {
                        let instructions = vec![
                            Instruction::Mov {
                                src: src1_operand,
                                dst: dst_operand.clone(),
                            },
                            Instruction::Binary {
                                op: BinaryOp::Add,
                                left_operand: src2_operand,
                                right_operand: dst_operand,
                            },
                        ];
                        ins.extend(instructions);
                    }
                    tacky_ir::BinaryOp::Subtract => {
                        let instructions = vec![
                            Instruction::Mov {
                                src: src1_operand,
                                dst: dst_operand.clone(),
                            },
                            Instruction::Binary {
                                op: BinaryOp::Subtract,
                                left_operand: src2_operand,
                                right_operand: dst_operand,
                            },
                        ];
                        ins.extend(instructions);
                    }
                    tacky_ir::BinaryOp::Multiply => {
                        let instructions = vec![
                            Instruction::Mov {
                                src: src1_operand,
                                dst: dst_operand.clone(),
                            },
                            Instruction::Binary {
                                op: BinaryOp::Multiply,
                                left_operand: src2_operand,
                                right_operand: dst_operand,
                            },
                        ];
                        ins.extend(instructions);
                    }
                    tacky_ir::BinaryOp::Divide => {
                        let instructions = vec![
                            Instruction::Mov {
                                src: src1_operand,
                                dst: Operand::Register(Reg::AX),
                            },
                            Instruction::Cdq,
                            Instruction::Idiv(src2_operand),
                            Instruction::Mov {
                                src: Operand::Register(Reg::AX),
                                dst: dst_operand.clone(),
                            },
                        ];
                        ins.extend(instructions);
                    }
                    tacky_ir::BinaryOp::Remainder => {
                        let instructions = vec![
                            Instruction::Mov {
                                src: src1_operand,
                                dst: Operand::Register(Reg::AX),
                            },
                            Instruction::Cdq,
                            Instruction::Idiv(src2_operand),
                            Instruction::Mov {
                                src: Operand::Register(Reg::DX),
                                dst: dst_operand.clone(),
                            },
                        ];
                        ins.extend(instructions);
                    }
                };
            }
        }

        Ok(ins)
    }

    fn generate_expression(&mut self, v: &tacky_ir::Value) -> Result<Operand, String> {
        match &v {
            &tacky_ir::Value::Constant(i) => Ok(Operand::Imm(*i)),
            &tacky_ir::Value::Var(name) => Ok(Operand::Pseudo(name.clone())),
        }
    }
    fn fix_memory_moves(&self, instructions: &[Instruction]) -> Vec<Instruction> {
        let mut new_ins = Vec::with_capacity(instructions.len());
        let temp_reg = Operand::Register(Reg::R10);

        for item in instructions {
            if let Instruction::Mov { src, dst } = item {
                if matches!(src, Operand::Stack(_)) && matches!(dst, Operand::Stack(_)) {
                    // 1. mov src, %r10d
                    new_ins.push(Instruction::Mov {
                        src: src.clone(),
                        dst: temp_reg.clone(),
                    });
                    // 2. mov %r10d, dst
                    new_ins.push(Instruction::Mov {
                        src: temp_reg.clone(),
                        dst: dst.clone(),
                    });
                    continue;
                }
            }
            //修复接受常量操作数的 idiv 指令 movl $3, %r10d  idivl %r10d
            if let Instruction::Idiv(operand) = item {
                if matches!(operand, Operand::Imm(_)) {
                    new_ins.push(Instruction::Mov {
                        src: operand.clone(),
                        dst: Operand::Register(Reg::R10),
                    });
                    new_ins.push(Instruction::Idiv(Operand::Register(Reg::R10)));
                    continue;
                }
            }
            if let Instruction::Binary {
                op,
                left_operand,
                right_operand,
            } = item
            {
                match op {
                    BinaryOp::Multiply => {
                        if matches!(right_operand, Operand::Stack(_)) {
                            new_ins.push(Instruction::Mov {
                                src: right_operand.clone(),
                                dst: Operand::Register(Reg::R11),
                            });
                            new_ins.push(Instruction::Binary {
                                op: BinaryOp::Multiply,
                                left_operand: left_operand.clone(),
                                right_operand: Operand::Register(Reg::R11),
                            });
                            new_ins.push(Instruction::Mov {
                                src: Operand::Register(Reg::R11),
                                dst: right_operand.clone(),
                            });
                            continue;
                        }
                    }
                    BinaryOp::Add => {
                        if matches!(left_operand, Operand::Stack(_))
                            && matches!(right_operand, Operand::Stack(_))
                        {
                            new_ins.push(Instruction::Mov {
                                src: left_operand.clone(),
                                dst: temp_reg.clone(),
                            });
                            new_ins.push(Instruction::Binary {
                                op: BinaryOp::Add,
                                left_operand: temp_reg.clone(),
                                right_operand: right_operand.clone(),
                            });
                            continue;
                        }
                    }
                    BinaryOp::Subtract => {
                        if matches!(left_operand, Operand::Stack(_))
                            && matches!(right_operand, Operand::Stack(_))
                        {
                            new_ins.push(Instruction::Mov {
                                src: left_operand.clone(),
                                dst: temp_reg.clone(),
                            });
                            new_ins.push(Instruction::Binary {
                                op: BinaryOp::Subtract,
                                left_operand: temp_reg.clone(),
                                right_operand: right_operand.clone(),
                            });
                            continue;
                        }
                    }
                }
            }
            new_ins.push(item.clone());
        }
        new_ins
    }
    // 它接受一个指令列表，返回一个新的、替换好伪寄存器的列表和栈大小
    fn allocate_stack_slots(&self, instructions: &[Instruction]) -> (Vec<Instruction>, i64) {
        let mut map: HashMap<String, i64> = HashMap::new();
        let mut new_instructions = Vec::with_capacity(instructions.len());

        for item in instructions {
            let new_item = match item {
                Instruction::Mov { src, dst } => {
                    let new_src = self.map_operand(src, &mut map);
                    let new_dst = self.map_operand(dst, &mut map);
                    Instruction::Mov {
                        src: new_src,
                        dst: new_dst,
                    }
                }
                Instruction::Unary { op, operand } => {
                    let new_operand = self.map_operand(operand, &mut map);
                    Instruction::Unary {
                        op: op.clone(),
                        operand: new_operand,
                    }
                }
                Instruction::Binary {
                    op,
                    left_operand,
                    right_operand,
                } => {
                    let new_left = self.map_operand(left_operand, &mut map);
                    let new_right = self.map_operand(right_operand, &mut map);
                    Instruction::Binary {
                        op: op.clone(),
                        left_operand: new_left,
                        right_operand: new_right,
                    }
                }
                Instruction::Idiv(operand) => {
                    let new_operand = self.map_operand(operand, &mut map);
                    Instruction::Idiv(new_operand)
                }
                _ => item.clone(),
            };
            new_instructions.push(new_item);
        }

        let stack_size = map.len() as i64 * 4;
        (new_instructions, stack_size)
    }

    // 替换逻辑
    fn map_operand<'a>(&self, operand: &'a Operand, map: &mut HashMap<String, i64>) -> Operand {
        if let Operand::Pseudo(name) = operand {
            // 先检查 key 是否存在
            if let Some(offset) = map.get(name) {
                return Operand::Stack(*offset);
            }
            let new_offset = -(map.len() as i64 + 1) * 4;
            map.insert(name.clone(), new_offset);
            Operand::Stack(new_offset)
        } else {
            operand.clone()
        }
    }
}

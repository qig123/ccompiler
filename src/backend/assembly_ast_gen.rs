// src/backend/assembly_ast_gen.rs

use std::collections::HashMap;

use crate::backend::assembly_ast::{
    BinaryOp, Function, Instruction, Operand, Program, Reg, UnaryOp,
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
        // Pass 1: IR -> 初始汇编指令
        let initial_instructions = self.generate_initial_instructions(ir_func)?;

        // Pass 2: 替换伪寄存器并计算栈大小
        let (instructions_with_stack, stack_size) =
            self.allocate_stack_slots(&initial_instructions);

        // Pass 3: 修复无效指令 (例如内存到内存的移动)
        let mut final_instructions = self.patch_instructions(&instructions_with_stack);

        // Pass 4: 插入栈分配指令
        if stack_size > 0 {
            // x86-64 要求栈是 16 字节对齐的.
            let aligned_stack_size = (stack_size + 15) & !15;
            final_instructions.insert(0, Instruction::AllocateStack(aligned_stack_size));
        }

        Ok(Function {
            name: ir_func.name.clone(),
            instructions: final_instructions,
        })
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
                let op_type = match op {
                    tacky_ir::UnaryOp::Complement => UnaryOp::Not,
                    tacky_ir::UnaryOp::Negate => UnaryOp::Neg,
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
                    _ => {
                        let asm_op = match op {
                            tacky_ir::BinaryOp::Add => BinaryOp::Add,
                            tacky_ir::BinaryOp::Subtract => BinaryOp::Subtract,
                            tacky_ir::BinaryOp::Multiply => BinaryOp::Multiply,
                            // 前面的 match 已经处理了 Divide 和 Remainder，这里不会发生
                            _ => unreachable!(),
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

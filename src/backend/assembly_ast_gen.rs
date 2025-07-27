// backend/ass_gen.rs

use std::collections::HashMap;

use crate::backend::assembly_ast::{Function, Instruction, Operand, Program, Reg, UnaryOp};
use crate::backend::tacky_ir;
/// 负责将 IR AST 转换为汇编 AST。
pub struct AssemblyGenerator {}

impl AssemblyGenerator {
    pub fn new() -> Self {
        AssemblyGenerator {}
    }

    /// 主入口：生成整个程序的汇编 AST。
    pub fn generate(&mut self, c_program: tacky_ir::Program) -> Result<Program, String> {
        let mut functions: Vec<Function> = Vec::new();

        for ir_func in &c_program.functions {
            let mut assembly_function = self.generate_function(ir_func)?;
            //pass 2 指令替换
            let alloct = self.replace_fake_register(&mut assembly_function.instructions)?;
            assembly_function
                .instructions
                .insert(0, Instruction::AllocateStack(alloct));
            //pass 3指令修复
            let new_ins = self.fix_instruction(&assembly_function.instructions)?;
            let new_func = Function {
                name: assembly_function.name,
                instructions: new_ins,
            };
            functions.push(new_func);
        }

        Ok(Program { functions })
    }

    /// 从 IR 函数 AST 生成汇编函数 AST。
    fn generate_function(&mut self, c_func: &tacky_ir::Function) -> Result<Function, String> {
        let mut instructions: Vec<Instruction> = Vec::new();

        for ins in &c_func.body {
            let generated_instructions = self.generate_instruction(&ins)?;
            instructions.extend(generated_instructions);
        }
        Ok(Function {
            name: c_func.name.clone(),
            instructions,
        })
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
        }

        Ok(ins)
    }

    fn generate_expression(&mut self, v: &tacky_ir::Value) -> Result<Operand, String> {
        match &v {
            &tacky_ir::Value::Constant(i) => Ok(Operand::Imm(*i)),
            &tacky_ir::Value::Var(name) => Ok(Operand::Pseudo(name.clone())),
        }
    }
    //把每个伪寄存器操作数替换为内存地址
    fn replace_fake_register(&mut self, ins: &mut Vec<Instruction>) -> Result<i64, String> {
        let mut map: HashMap<String, i64> = HashMap::new();

        for item in ins {
            match item {
                Instruction::Mov { src, dst } => {
                    // 处理 src 操作数
                    let new_src = match src {
                        Operand::Pseudo(name) => {
                            if let Some(offset) = map.get(name) {
                                Operand::Stack(*offset)
                            } else {
                                let offset = -(map.len() as i64 + 1) * 4;
                                map.insert(name.clone(), offset);
                                Operand::Stack(offset)
                            }
                        }
                        _ => src.clone(),
                    };

                    // 处理 dst 操作数
                    let new_dst = match dst {
                        Operand::Pseudo(name) => {
                            if let Some(offset) = map.get(name) {
                                Operand::Stack(*offset)
                            } else {
                                let offset = -(map.len() as i64 + 1) * 4;
                                map.insert(name.clone(), offset);
                                Operand::Stack(offset)
                            }
                        }
                        _ => dst.clone(),
                    };

                    *item = Instruction::Mov {
                        src: new_src,
                        dst: new_dst,
                    };
                }
                Instruction::Unary { op, operand } => {
                    let new_operand = match operand {
                        Operand::Pseudo(name) => {
                            if let Some(offset) = map.get(name) {
                                Operand::Stack(*offset)
                            } else {
                                let offset = -(map.len() as i64 + 1) * 4;
                                map.insert(name.clone(), offset);
                                Operand::Stack(offset)
                            }
                        }
                        _ => operand.clone(),
                    };
                    *item = Instruction::Unary {
                        op: op.clone(),
                        operand: new_operand,
                    }
                }
                _ => {}
            }
        }

        // 返回栈空间大小
        Ok((map.len() as i64) * 4)
    }
    //fix mov two operand is memory operand
    fn fix_instruction(&mut self, ins: &Vec<Instruction>) -> Result<Vec<Instruction>, String> {
        let mut new_ins: Vec<Instruction> = Vec::new();
        let temp_reg = Operand::Register(Reg::R10); // 使用 R10D 作为临时寄存器

        for item in ins {
            match item {
                Instruction::Mov { src, dst } => {
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
                    } else {
                        new_ins.push(item.clone());
                    }
                }
                Instruction::AllocateStack(_) | Instruction::Ret | Instruction::Unary { .. } => {
                    new_ins.push(item.clone());
                }
            }
        }
        Ok(new_ins)
    }
}

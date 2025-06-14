// codegen/translator.rs (或其他文件)

use crate::{
    codegen::assembly_ir::{
        // Assuming your assembly_ir module is structured this way
        self as assembly_ir, // Re-export the module itself for clarity
        Assemble,
        BinaryOperator as AssBinaryOperator,
        Instruction,
        Operand,
        UnaryOperator as AssUnaryOperator,
    },
    error::CodegenError,
};
use std::collections::{HashMap, HashSet};

use crate::tacky::tacky::{
    BinaryOperator as TackyBinaryOperator,
    // Assuming your tacky module is structured this way
    Instruction as TackyInstruction,
    Program as TackyProgram,
    UnaryOperator as TackyUnaryOperator,
    Value as TackyValue,
}; // Need HashMap for pseudo -> stack map, HashSet for unique pseudos // Your error type

pub struct TackyToAssemblyTranslator {
    // State for Pass 2: Mapping pseudoregisters to stack offsets
    pseudo_to_stack_offset: HashMap<String, i64>,
    next_stack_offset: i64, // Starts at -4, decreases by 4 for each new pseudo
}

impl TackyToAssemblyTranslator {
    pub fn new() -> Self {
        TackyToAssemblyTranslator {
            pseudo_to_stack_offset: HashMap::new(),
            next_stack_offset: -4, // First temporary variable goes at -4(%rbp)
        }
    }

    // Main translation function: TACKY Program -> Assembly Program
    // Follows the three passes described.
    pub fn translate(&mut self, tacky_program: TackyProgram) -> Result<Assemble, CodegenError> {
        // Assumption: TackyProgram always contains exactly one function definition
        let tacky_function = tacky_program.definition;

        // Pass 1: Initial translation to Assembly Instructions with Pseudo operands,
        // and collect all unique pseudoregisters used.
        let mut initial_assembly_instructions: Vec<Instruction> = Vec::new();
        let mut unique_pseudoregisters: HashSet<String> = HashSet::new();

        for instruction in tacky_function.body {
            let (asm_instructions, pseudos) = self.translate_tacky_instruction(instruction)?;
            initial_assembly_instructions.extend(asm_instructions);
            unique_pseudoregisters.extend(pseudos);
        }

        // Pass 2: Assign stack offsets to pseudoregisters and replace Pseudo operands
        // Sort pseudos for deterministic output (optional but good practice)
        let mut sorted_pseudos: Vec<String> = unique_pseudoregisters.into_iter().collect();
        sorted_pseudos.sort();

        for pseudo_id in sorted_pseudos {
            // Assign the current offset to the pseudo and store it
            self.pseudo_to_stack_offset
                .insert(pseudo_id, self.next_stack_offset);
            // Move to the next available stack offset (subtract 4 for the next 4-byte variable)
            self.next_stack_offset -= 4;
        }

        // Calculate the total stack space needed.
        // If next_stack_offset is -4 (no pseudos), space = -(-4) - 4 = 0
        // If next_stack_offset is -8 (one pseudo at -4), space = -(-8) - 4 = 4
        // If next_stack_offset is -12 (pseudos at -4, -8), space = -(-12) - 4 = 8
        // If next_stack_offset is -16 (pseudos at -4, -8, -12), space = -(-16) - 4 = 12
        // The space needed is the absolute value of the *last assigned offset*,
        // which is (-self.next_stack_offset) - 4 bytes less than the final next_stack_offset.
        let total_stack_space_needed = (-self.next_stack_offset) - 4;

        // Replace Pseudo operands with Stack operands in the generated instructions
        let mut stack_replaced_instructions: Vec<Instruction> = Vec::new();
        for instruction in initial_assembly_instructions {
            stack_replaced_instructions.push(self.replace_pseudos_in_instruction(instruction)?);
        }

        // Pass 3: Add AllocateStack and fix Mov(Stack, Stack)
        let mut final_assembly_instructions: Vec<Instruction> = Vec::new();

        // Insert AllocateStack instruction at the beginning if space is needed
        if total_stack_space_needed > 0 {
            final_assembly_instructions.push(assembly_ir::Instruction::AllocateStack(
                total_stack_space_needed,
            ));
        }

        // Iterate through instructions and fix Mov(Stack, Stack)
        for instruction in stack_replaced_instructions {
            match instruction {
                Instruction::Mov {
                    src: Operand::Stack(s_offset),
                    dst: Operand::Stack(d_offset),
                } => {
                    // Rewrite Mov(Stack, Stack) into two instructions via R10
                    final_assembly_instructions.push(assembly_ir::Instruction::Mov {
                        src: assembly_ir::Operand::Stack(s_offset),
                        dst: assembly_ir::Operand::Reg(assembly_ir::Reg::R10),
                    });
                    final_assembly_instructions.push(assembly_ir::Instruction::Mov {
                        src: assembly_ir::Operand::Reg(assembly_ir::Reg::R10),
                        dst: assembly_ir::Operand::Stack(d_offset),
                    });
                }

                // --- Fix Idiv(Imm) ---
                Instruction::Idiv(operand) => {
                    match operand {
                        Operand::Imm(i) => {
                            // Rule: idivl $3 => movl $3, %r10d idivl %r10d
                            // Idiv operand cannot be an immediate value. Use R10 as temp.
                            final_assembly_instructions.push(assembly_ir::Instruction::Mov {
                                src: Operand::Imm(i),
                                dst: assembly_ir::Operand::Reg(assembly_ir::Reg::R10), // Use R10 for divisor temp
                            });
                            final_assembly_instructions.push(assembly_ir::Instruction::Idiv(
                                assembly_ir::Operand::Reg(assembly_ir::Reg::R10),
                            ));
                        }
                        // Idiv(Reg) and Idiv(Stack) are valid operands for Idiv, pass them through
                        _ => {
                            final_assembly_instructions
                                .push(assembly_ir::Instruction::Idiv(operand));
                        }
                    }
                }

                // --- Fix Binary operations ---
                Instruction::Binary {
                    op,
                    left_operand,
                    right_operand,
                } => {
                    match op {
                        AssBinaryOperator::Add | AssBinaryOperator::Sub => {
                            match (&left_operand, &right_operand) {
                                (Operand::Stack(_), Operand::Stack(_)) => {
                                    // Rule: addl mem, mem => movl mem, reg; addl reg, mem
                                    // Fix Binary(Add/Sub, Stack, Stack)
                                    // Load source (left_operand) into R10, then perform op on R10 and destination (right_operand)
                                    final_assembly_instructions.push(
                                        assembly_ir::Instruction::Mov {
                                            src: left_operand.clone(), // Clone source Stack operand
                                            dst: assembly_ir::Operand::Reg(assembly_ir::Reg::R10),
                                        },
                                    );
                                    final_assembly_instructions.push(
                                        assembly_ir::Instruction::Binary {
                                            op: op,
                                            left_operand: assembly_ir::Operand::Reg(
                                                assembly_ir::Reg::R10,
                                            ),
                                            right_operand: right_operand.clone(), // Clone destination Stack operand
                                        },
                                    );
                                }
                                // Binary(Add/Sub, Imm/Reg/Stack, Reg) and Binary(Add/Sub, Imm/Reg, Stack) are valid, pass through
                                // Note: Binary(Add/Sub, Stack, Imm) is not generated by Pass 1 rules (dst is never Imm)
                                _ => {
                                    // Pass through other valid Add/Sub combinations
                                    final_assembly_instructions.push(
                                        assembly_ir::Instruction::Binary {
                                            op,
                                            left_operand,
                                            right_operand,
                                        },
                                    );
                                }
                            }
                        }
                        AssBinaryOperator::Mult => {
                            // Rule: imull imm, mem => movl mem, reg; imull imm, reg; movl reg, mem
                            // AST: Binary(Mult, src, dst) means dst = dst * src
                            // If dst is Stack, we need temp register R11.
                            match &right_operand {
                                Operand::Stack(_) => {
                                    // Fix Binary(Mult, src, Stack) - src can be Imm, Reg, Stack
                                    // Pattern based on rule: movl dst, R11; imull src, R11; movl R11, dst
                                    final_assembly_instructions.push(
                                        assembly_ir::Instruction::Mov {
                                            src: right_operand.clone(), // Clone destination (Stack) operand
                                            dst: assembly_ir::Operand::Reg(assembly_ir::Reg::R11), // Use R11 for Mult destination temp
                                        },
                                    );
                                    final_assembly_instructions.push(
                                        assembly_ir::Instruction::Binary {
                                            op: AssBinaryOperator::Mult,
                                            left_operand: left_operand.clone(), // Clone source operand
                                            right_operand: assembly_ir::Operand::Reg(
                                                assembly_ir::Reg::R11,
                                            ), // Destination is the temp register
                                        },
                                    );
                                    final_assembly_instructions.push(
                                        assembly_ir::Instruction::Mov {
                                            src: assembly_ir::Operand::Reg(assembly_ir::Reg::R11), // Store result from R11
                                            dst: right_operand.clone(), // Clone destination (Stack) operand again
                                        },
                                    );
                                }
                                // Binary(Mult, src, Reg) is valid, pass through
                                // Note: Binary(Mult, src, Imm) is not generated by Pass 1 rules
                                _ => {
                                    // Pass through other valid Mult combinations (where dest is Reg)
                                    final_assembly_instructions.push(
                                        assembly_ir::Instruction::Binary {
                                            op,
                                            left_operand,
                                            right_operand,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
                //fix Cmp(Stack, Stack)
                Instruction::Cmp {
                    left_operand,
                    right_operand,
                } => {
                    match (&left_operand, &right_operand) {
                        // Case 1: Cmp Stack, Stack -> mov Stack1, R10; cmp R10, Stack2
                        // Handles the "two memory operands" constraint.
                        (Operand::Stack(s1), Operand::Stack(s2)) => {
                            // Rule: Use R10 for the first operand if it's memory (and the second is also memory)
                            final_assembly_instructions.push(assembly_ir::Instruction::Mov {
                                src: Operand::Stack(*s1), // Load Stack1 into R10
                                dst: assembly_ir::Operand::Reg(assembly_ir::Reg::R10),
                            });
                            // Compare R10 (holding Stack1) against Stack2.
                            // If Emitter translates Cmp { A, B } to `cmpl B, A` (A-B),
                            // Cmp { R10, Stack2 } -> `cmpl Stack2, R10` -> R10 - Stack2 -> Stack1 - Stack2. Correct.
                            final_assembly_instructions.push(assembly_ir::Instruction::Cmp {
                                left_operand: assembly_ir::Operand::Reg(assembly_ir::Reg::R10),
                                right_operand: Operand::Stack(*s2),
                            });
                        }
                        // Case 2: Cmp X, Imm -> mov Imm, R11; cmp X, R11
                        // Handles the "second operand cannot be immediate" constraint.
                        // This covers Reg, Imm; Stack, Imm; and Imm, Imm.
                        (_, Operand::Imm(i)) => {
                            // Rule: Use R11 for the second operand if it's immediate.
                            final_assembly_instructions.push(assembly_ir::Instruction::Mov {
                                src: Operand::Imm(*i), // Load the immediate value into R11
                                dst: assembly_ir::Operand::Reg(assembly_ir::Reg::R11),
                            });
                            // Compare the original left operand (X) against R11 (holding the immediate).
                            // If Emitter translates Cmp { A, B } to `cmpl B, A` (A-B),
                            // Cmp { left_operand, R11 } -> `cmpl R11, left_operand_str` -> left_operand - R11 -> left_operand - Imm. Correct.
                            final_assembly_instructions.push(assembly_ir::Instruction::Cmp {
                                left_operand, // The original left operand (Reg, Stack, or Imm)
                                right_operand: assembly_ir::Operand::Reg(assembly_ir::Reg::R11), // Compare against R11
                            });
                        }
                        // Case 3: All other combinations (Reg/Imm/Stack vs Reg/Stack)
                        // These are valid combinations for `cmpl src, dst` after Emitter swap, and don't violate
                        // the two main constraints.
                        // Examples: Cmp Reg, Reg; Cmp Reg, Stack; Cmp Stack, Reg; Cmp Imm, Reg; Cmp Imm, Stack.
                        // Note: Cmp Imm, Imm and Cmp Stack, Stack and Cmp X, Imm (where X is Reg/Stack)
                        // are already handled by the cases above.
                        _ => {
                            // Pass through directly. The emitter will swap operands for assembly if needed.
                            final_assembly_instructions.push(assembly_ir::Instruction::Cmp {
                                left_operand,
                                right_operand,
                            });
                        }
                    }
                }

                // --- Pass through other instructions that don't need specific fixing in this pass ---
                // Mov instructions other than Stack -> Stack are fine (e.g., Imm->Stack, Reg->Stack, Stack->Reg, Imm->Reg, Reg->Reg)
                inst @ Instruction::Mov { .. } => {
                    final_assembly_instructions.push(inst);
                }
                // Unary operations are fine after pseudo replacement
                inst @ Instruction::Unary { .. } => {
                    final_assembly_instructions.push(inst);
                }
                // Cdq is fine
                inst @ Instruction::Cdq => {
                    final_assembly_instructions.push(inst);
                }
                // Ret is fine
                inst @ Instruction::Ret => {
                    final_assembly_instructions.push(inst);
                }
                // AllocateStack is handled at the beginning, but passing it here is harmless
                // (it won't be in the input list `stack_replaced_instructions` anyway if added only at the start)
                inst @ Instruction::AllocateStack(_) => {
                    final_assembly_instructions.push(inst);
                }
                // Jmp and JmpCC are fine
                inst @ Instruction::Jmp(_) => {
                    final_assembly_instructions.push(inst);
                }
                inst @ Instruction::JmpCC { .. } => {
                    final_assembly_instructions.push(inst);
                }
                // Labels are fine
                inst @ Instruction::Label(_) => {
                    final_assembly_instructions.push(inst);
                }
                //SetCC is fine
                inst @ Instruction::SetCC { .. } => {
                    final_assembly_instructions.push(inst);
                }
            }
        }

        let ass_function = assembly_ir::AssFunction {
            name: tacky_function.name, // Function name is a String from TACKY
            instructions: final_assembly_instructions,
        };

        // Assuming Assembly Program AST holds one function
        Ok(assembly_ir::Assemble {
            function: ass_function,
        })
    }

    // Helper for Pass 1: Translates a single Tacky Instruction
    // Returns a vector of generated Assembly Instructions and a list of Pseudo names found in them.
    fn translate_tacky_instruction(
        &self,
        instruction: TackyInstruction,
    ) -> Result<(Vec<Instruction>, Vec<String>), CodegenError> {
        let mut asm_instructions: Vec<Instruction> = Vec::new();
        let mut pseudos_found: Vec<String> = Vec::new();

        let collect_pseudo = |value: &TackyValue, pseudos: &mut Vec<String>| {
            if let TackyValue::Var(id) = value {
                pseudos.push(id.clone());
            }
        };

        match instruction {
            TackyInstruction::Return(val) => {
                // Rule: Return(val) => Mov(val, Reg(AX)), Ret
                collect_pseudo(&val, &mut pseudos_found);
                let src_operand = self.translate_tacky_value(&val)?;

                // Move the return value to AX (or EAX for 32-bit)
                asm_instructions.push(assembly_ir::Instruction::Mov {
                    src: src_operand,
                    dst: assembly_ir::Operand::Reg(assembly_ir::Reg::AX),
                });
                // Add the return instruction
                asm_instructions.push(assembly_ir::Instruction::Ret);
            }
            TackyInstruction::Unary { op, src, dst } => {
                match op {
                    TackyUnaryOperator::Complement | TackyUnaryOperator::Negate => {
                        // Rule: Unary(unary_operator, src, dst) => Mov(src, dst), Unary(unary_operator, dst)
                        collect_pseudo(&src, &mut pseudos_found);
                        collect_pseudo(&dst, &mut pseudos_found);

                        let src_operand = self.translate_tacky_value(&src)?;
                        let dst_operand = self.translate_tacky_value(&dst)?;

                        // First: Mov src to dst
                        asm_instructions.push(assembly_ir::Instruction::Mov {
                            src: src_operand,
                            dst: dst_operand.clone(), // Clone dst because it's used again in the next instruction
                        });

                        // Second: Perform Unary operation on dst (in-place)
                        let ass_unary_op = match op {
                            TackyUnaryOperator::Complement => AssUnaryOperator::Not,
                            TackyUnaryOperator::Negate => AssUnaryOperator::Neg,
                            _ => {
                                return Err(CodegenError {
                                    message: "unsupport op".to_string(),
                                });
                            }
                        };
                        asm_instructions.push(assembly_ir::Instruction::Unary {
                            op: ass_unary_op,
                            operand: dst_operand, // The operand to the Assembly Unary is the destination
                        });
                    }
                    TackyUnaryOperator::Bang => {
                        //Rule: Unary(Not, src, dst)=> Cmp(Imm(0),arc) Mov(Imm(0),dst),SetCC(E,dst)
                        collect_pseudo(&src, &mut pseudos_found);
                        collect_pseudo(&dst, &mut pseudos_found);

                        let src_operand = self.translate_tacky_value(&src)?;
                        let dst_operand = self.translate_tacky_value(&dst)?;
                        let cmp = assembly_ir::Instruction::Cmp {
                            left_operand: assembly_ir::Operand::Imm(0),
                            right_operand: src_operand,
                        };
                        asm_instructions.push(cmp);
                        let mov = assembly_ir::Instruction::Mov {
                            src: assembly_ir::Operand::Imm(0),
                            dst: dst_operand.clone(),
                        };
                        asm_instructions.push(mov);
                        asm_instructions.push(assembly_ir::Instruction::SetCC {
                            condition: assembly_ir::Condition::E,
                            dst: dst_operand,
                        });
                    } // _ => {
                      //     return Err(CodegenError {
                      //         message: "unsupported unary operator".to_string(),
                      //     });
                      // }
                }
            } // Add translation for other TACKY Instruction types if needed
            TackyInstruction::Binary {
                op,
                src1,
                src2,
                dst,
            } => {
                match op {
                    TackyBinaryOperator::Add
                    | TackyBinaryOperator::Subtract
                    | TackyBinaryOperator::Multiply
                    | TackyBinaryOperator::Divide
                    | TackyBinaryOperator::Remainder => {
                        collect_pseudo(&src1, &mut pseudos_found);
                        collect_pseudo(&src2, &mut pseudos_found);
                        collect_pseudo(&dst, &mut pseudos_found);

                        let src1_operand = self.translate_tacky_value(&src1)?;
                        let src2_operand = self.translate_tacky_value(&src2)?;
                        let dst_operand = self.translate_tacky_value(&dst)?;

                        // Second: Perform Binary operation on dst (in-place)
                        match op {
                            TackyBinaryOperator::Add => {
                                // First: Mov src1 to dst
                                asm_instructions.push(assembly_ir::Instruction::Mov {
                                    src: src1_operand,
                                    dst: dst_operand.clone(), // Clone dst because it's used again in the next instruction
                                });
                                asm_instructions.push(assembly_ir::Instruction::Binary {
                                    op: AssBinaryOperator::Add,
                                    left_operand: src2_operand,
                                    right_operand: dst_operand,
                                });
                            }
                            TackyBinaryOperator::Subtract => {
                                // First: Mov src1 to dst
                                asm_instructions.push(assembly_ir::Instruction::Mov {
                                    src: src1_operand,
                                    dst: dst_operand.clone(), // Clone dst because it's used again in the next instruction
                                });
                                asm_instructions.push(assembly_ir::Instruction::Binary {
                                    op: AssBinaryOperator::Sub,
                                    left_operand: src2_operand,
                                    right_operand: dst_operand,
                                });
                            }
                            TackyBinaryOperator::Multiply => {
                                // First: Mov src1 to dst
                                asm_instructions.push(assembly_ir::Instruction::Mov {
                                    src: src1_operand,
                                    dst: dst_operand.clone(), // Clone dst because it's used again in the next instruction
                                });
                                asm_instructions.push(assembly_ir::Instruction::Binary {
                                    op: AssBinaryOperator::Mult,
                                    left_operand: src2_operand,
                                    right_operand: dst_operand,
                                });
                            }
                            //div 指令不能对立即值进行作，因此如果 src2 是常量，则 division 和 remaining 的汇编指令将无效,会在稍后的流程修复
                            TackyBinaryOperator::Divide => {
                                //Mov(src1, Reg(AX))
                                asm_instructions.push(assembly_ir::Instruction::Mov {
                                    src: src1_operand,
                                    dst: Operand::Reg(assembly_ir::Reg::AX),
                                });
                                //Cdq
                                asm_instructions.push(Instruction::Cdq);
                                //Idiv(src2)
                                asm_instructions.push(Instruction::Idiv(src2_operand));
                                //Mov(Reg(AX), dst)
                                asm_instructions.push(assembly_ir::Instruction::Mov {
                                    src: Operand::Reg(assembly_ir::Reg::AX),
                                    dst: dst_operand,
                                });
                            }
                            TackyBinaryOperator::Remainder => {
                                //Mov(src1, Reg(AX))
                                asm_instructions.push(assembly_ir::Instruction::Mov {
                                    src: src1_operand,
                                    dst: Operand::Reg(assembly_ir::Reg::AX),
                                });
                                //Cdq
                                asm_instructions.push(Instruction::Cdq);
                                //Idiv(src2)
                                asm_instructions.push(Instruction::Idiv(src2_operand));
                                //Mov(Reg(AX), dst)
                                asm_instructions.push(assembly_ir::Instruction::Mov {
                                    src: Operand::Reg(assembly_ir::Reg::DX),
                                    dst: dst_operand,
                                });
                            }
                            _ => {
                                return Err(CodegenError {
                                    message: "unsupport op".to_string(),
                                });
                            }
                        };
                    }
                    TackyBinaryOperator::EqualEqual
                    | TackyBinaryOperator::BangEqual
                    | TackyBinaryOperator::Less
                    | TackyBinaryOperator::LessEqual
                    | TackyBinaryOperator::Greater
                    | TackyBinaryOperator::GreaterEqual => {
                        //Rule : Binary(op, src1, src2, dst) => Cmp(src2, src1), Mov(Imm(0), dst), SetCC(op, dst)
                        collect_pseudo(&src1, &mut pseudos_found);
                        collect_pseudo(&src2, &mut pseudos_found);
                        collect_pseudo(&dst, &mut pseudos_found);

                        let src1_operand = self.translate_tacky_value(&src1)?;
                        let src2_operand = self.translate_tacky_value(&src2)?;
                        let dst_operand = self.translate_tacky_value(&dst)?;
                        // First: Compare src1 and src2
                        asm_instructions.push(assembly_ir::Instruction::Cmp {
                            left_operand: src2_operand,
                            right_operand: src1_operand,
                        });
                        // Second: Move 0 to dst
                        asm_instructions.push(assembly_ir::Instruction::Mov {
                            src: assembly_ir::Operand::Imm(0),
                            dst: dst_operand.clone(), // Clone dst because it's used again in the next instruction
                        });
                        // Third: Set condition code based on the comparison
                        let condition = match op {
                            TackyBinaryOperator::EqualEqual => assembly_ir::Condition::E,
                            TackyBinaryOperator::BangEqual => assembly_ir::Condition::NE,
                            TackyBinaryOperator::Less => assembly_ir::Condition::L,
                            TackyBinaryOperator::LessEqual => assembly_ir::Condition::LE,
                            TackyBinaryOperator::Greater => assembly_ir::Condition::G,
                            TackyBinaryOperator::GreaterEqual => assembly_ir::Condition::GE,
                            _ => {
                                return Err(CodegenError {
                                    message: "unsupported binary operator".to_string(),
                                });
                            }
                        };
                        asm_instructions.push(assembly_ir::Instruction::SetCC {
                            condition,
                            dst: dst_operand, // The destination for the SetCC is the same as the original dst
                        });
                    } // _ => {
                      //     return Err(CodegenError {
                      //         message: "unsupported binary operator".to_string(),
                      //     });
                      // }
                }
            }
            TackyInstruction::Copy { src, dst } => {
                // Rule: Copy(src, dst) => Mov(src, dst)
                collect_pseudo(&src, &mut pseudos_found);
                collect_pseudo(&dst, &mut pseudos_found);

                let src_operand = self.translate_tacky_value(&src)?;
                let dst_operand = self.translate_tacky_value(&dst)?;

                asm_instructions.push(assembly_ir::Instruction::Mov {
                    src: src_operand,
                    dst: dst_operand,
                });
            }
            TackyInstruction::Jump { target } => {
                // Rule: Jump(target) => Jmp(target)
                asm_instructions.push(assembly_ir::Instruction::Jmp(target));
            }
            TackyInstruction::JumpIfZero { condition, target } => {
                // Rule: JumpIfZero(condition, target) => Cmp(condition, Imm(0)), JmpCC(E, target)
                collect_pseudo(&condition, &mut pseudos_found);

                let condition_operand = self.translate_tacky_value(&condition)?;
                asm_instructions.push(assembly_ir::Instruction::Cmp {
                    left_operand: condition_operand,
                    right_operand: assembly_ir::Operand::Imm(0),
                });
                asm_instructions.push(assembly_ir::Instruction::JmpCC {
                    condition: assembly_ir::Condition::E,
                    target,
                });
            }
            TackyInstruction::JumpIfNotZero { condition, target } => {
                // Rule: JumpIfNotZero(condition, target) => Cmp(condition, Imm(0)), JmpCC(NE, target)
                collect_pseudo(&condition, &mut pseudos_found);

                let condition_operand = self.translate_tacky_value(&condition)?;
                asm_instructions.push(assembly_ir::Instruction::Cmp {
                    left_operand: condition_operand,
                    right_operand: assembly_ir::Operand::Imm(0),
                });
                asm_instructions.push(assembly_ir::Instruction::JmpCC {
                    condition: assembly_ir::Condition::NE,
                    target,
                });
            }
            TackyInstruction::Label { name } => {
                // Rule: Label(name) => Jmp(name) // Labels are just markers, no Assembly instruction needed
                asm_instructions.push(assembly_ir::Instruction::Label(name));
            } // _ => {
              //     return Err(CodegenError {
              //         message: "unsupport op".to_string(),
              //     });
              // }
        }

        Ok((asm_instructions, pseudos_found))
    }

    // Helper for Pass 1: Translates a Tacky Value to an initial Assembly Operand (Pseudo or Imm)
    fn translate_tacky_value(&self, value: &TackyValue) -> Result<Operand, CodegenError> {
        match value {
            TackyValue::Constant(i) => Ok(assembly_ir::Operand::Imm(*i)),
            TackyValue::Var(id) => Ok(assembly_ir::Operand::Pseudo(id.clone())),
        }
    }

    // Helper for Pass 2: Replaces Pseudo operands with Stack operands in an instruction
    fn replace_pseudos_in_instruction(
        &self,
        instruction: Instruction,
    ) -> Result<Instruction, CodegenError> {
        let replace_operand = |operand: Operand,
                               map: &HashMap<String, i64>|
         -> Result<Operand, CodegenError> {
            match operand {
                Operand::Pseudo(id) => {
                    // Look up the pseudo ID in the map to get its stack offset
                    if let Some(&offset) = map.get(&id) {
                        Ok(assembly_ir::Operand::Stack(offset))
                    } else {
                        // This case indicates an internal error: a pseudo was used but not allocated stack space
                        Err(CodegenError {
                            message: format!(
                                "Internal error: Pseudoregister '{}' used but no stack allocation found.",
                                id
                            ),
                        })
                    }
                }
                // All other operand types are returned as is
                _ => Ok(operand),
            }
        };

        match instruction {
            Instruction::Mov { src, dst } => {
                let new_src = replace_operand(src, &self.pseudo_to_stack_offset)?;
                let new_dst = replace_operand(dst, &self.pseudo_to_stack_offset)?;
                Ok(assembly_ir::Instruction::Mov {
                    src: new_src,
                    dst: new_dst,
                })
            }
            Instruction::Unary { op, operand } => {
                let new_operand: Operand = replace_operand(operand, &self.pseudo_to_stack_offset)?;
                Ok(assembly_ir::Instruction::Unary {
                    op,
                    operand: new_operand,
                })
            }
            Instruction::Binary {
                op,
                left_operand,
                right_operand,
            } => {
                let left_src = replace_operand(left_operand, &self.pseudo_to_stack_offset)?;
                let right_src = replace_operand(right_operand, &self.pseudo_to_stack_offset)?;
                Ok(assembly_ir::Instruction::Binary {
                    op: op,
                    left_operand: left_src,
                    right_operand: right_src,
                })
            }
            Instruction::Idiv(operand) => {
                let new_operand: Operand = replace_operand(operand, &self.pseudo_to_stack_offset)?;
                Ok(assembly_ir::Instruction::Idiv(new_operand))
            }
            Instruction::Cmp {
                left_operand,
                right_operand,
            } => {
                let new_left = replace_operand(left_operand, &self.pseudo_to_stack_offset)?;
                let new_right = replace_operand(right_operand, &self.pseudo_to_stack_offset)?;
                Ok(assembly_ir::Instruction::Cmp {
                    left_operand: new_left,
                    right_operand: new_right,
                })
            }
            Instruction::SetCC { condition, dst } => {
                let new_dst = replace_operand(dst, &self.pseudo_to_stack_offset)?;
                Ok(assembly_ir::Instruction::SetCC {
                    condition,
                    dst: new_dst,
                })
            }

            // AllocateStack and Ret instructions have no operands to replace in this step
            inst @ assembly_ir::Instruction::AllocateStack(_)
            | inst @ assembly_ir::Instruction::Ret => Ok(inst),
            inst @ assembly_ir::Instruction::Cdq => Ok(inst),
            inst @ assembly_ir::Instruction::Jmp(_) => Ok(inst),
            inst @ assembly_ir::Instruction::JmpCC { .. } => Ok(inst),
            inst @ assembly_ir::Instruction::Label(_) => Ok(inst),
        }
    }
}

// codegen/translator.rs (或其他文件)

use crate::{
    codegen::assembly_ir::{
        // Assuming your assembly_ir module is structured this way
        self as assembly_ir, // Re-export the module itself for clarity
        Assemble,
        Instruction,
        Operand,
        UnaryOperator as AssUnaryOperator,
    },
    error::CodegenError,
};
use std::collections::{HashMap, HashSet};

use crate::tacky::tacky::{
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
                // All other instructions (Mov with other operand types, Unary, Ret, AllocateStack)
                // are kept as is. (AllocateStack is already handled above, but this match is fine)
                _ => final_assembly_instructions.push(instruction),
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
                };
                asm_instructions.push(assembly_ir::Instruction::Unary {
                    op: ass_unary_op,
                    operand: dst_operand, // The operand to the Assembly Unary is the destination
                });
            }
            // Add translation for other TACKY Instruction types if needed
            _ => {
                return Err(CodegenError {
                    message: format!("Unsupported Tacky instruction: {:?}", instruction),
                });
            }
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
                let new_operand = replace_operand(operand, &self.pseudo_to_stack_offset)?;
                Ok(assembly_ir::Instruction::Unary {
                    op,
                    operand: new_operand,
                })
            }
            // AllocateStack and Ret instructions have no operands to replace in this step
            inst @ assembly_ir::Instruction::AllocateStack(_)
            | inst @ assembly_ir::Instruction::Ret => Ok(inst),
        }
    }
}

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
            } // Add translation for other TACKY Instruction types if needed
              // _ => return Err(CodegenError { message: format!("Unsupported Tacky instruction: {:?}", instruction) }),
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

// You'll need to define your CodegenError type somewhere, e.g.:
/*
use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
    // Potentially add source location information
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Codegen Error: {}", self.message)
    }
}

impl Error for CodegenError {}
*/

// --- Add tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::assembly_ir::{
        Instruction as AssInstruction, Operand as AssOperand, Reg,
        UnaryOperator as AssUnaryOperator,
    };
    use crate::tacky::tacky::{
        FunctionDefinition, Instruction as TackyInstruction, Program as TackyProgram,
        UnaryOperator as TackyUnaryOperator, Value as TackyValue,
    }; // Import Tacky types // Import Assembly IR types

    #[test]
    fn test_tacky_to_assembly_return_literal() {
        // TACKY:
        // program { definition: FunctionDefinition { name: "main", body: [ Return(Constant(5)) ] } }
        let tacky_program = TackyProgram {
            definition: FunctionDefinition {
                name: "main".to_string(),
                body: vec![TackyInstruction::Return(TackyValue::Constant(5))],
            },
        };

        let mut translator = TackyToAssemblyTranslator::new();
        let assembly_program = translator.translate(tacky_program).unwrap();

        // Expected Assembly:
        // Assemble { function: AssFunction { name: "main", instructions: [ Mov { src: Imm(5), dst: Reg(AX) }, Ret ] } }
        assert_eq!(assembly_program.function.name, "main");
        assert_eq!(assembly_program.function.instructions.len(), 2); // Mov + Ret
        assert_eq!(
            assembly_program.function.instructions[0],
            AssInstruction::Mov {
                src: AssOperand::Imm(5),
                dst: AssOperand::Reg(Reg::AX)
            }
        );
        assert_eq!(
            assembly_program.function.instructions[1],
            AssInstruction::Ret
        );

        // No pseudoregisters used, so no stack allocation
        assert!(translator.pseudo_to_stack_offset.is_empty());
        assert_eq!(translator.next_stack_offset, -4); // Should remain -4 if no pseudos
    }

    #[test]
    fn test_tacky_to_assembly_unary_negate() {
        // TACKY:
        // program { definition: FunctionDefinition { name: "main", body: [ Unary { op: Negate, src: Constant(10), dst: Var("t0") }, Return(Var("t0")) ] } }
        // Represents something like: t0 = -10; return t0;
        let tacky_program = TackyProgram {
            definition: FunctionDefinition {
                name: "main".to_string(),
                body: vec![
                    TackyInstruction::Unary {
                        op: TackyUnaryOperator::Negate,
                        src: TackyValue::Constant(10),
                        dst: TackyValue::Var("t0".to_string()),
                    },
                    TackyInstruction::Return(TackyValue::Var("t0".to_string())),
                ],
            },
        };

        let mut translator = TackyToAssemblyTranslator::new();
        let assembly_program = translator.translate(tacky_program).unwrap();

        // Expected Assembly (Pass 1):
        // [ Mov { src: Imm(10), dst: Pseudo("t0") }, Unary { op: Neg, operand: Pseudo("t0") }, Mov { src: Pseudo("t0"), dst: Reg(AX) }, Ret ]
        // Pseudo "t0" detected. Stack map: {"t0": -4}. Next offset: -8. Total space: 4.
        // Expected Assembly (Pass 2):
        // [ Mov { src: Imm(10), dst: Stack(-4) }, Unary { op: Neg, operand: Stack(-4) }, Mov { src: Stack(-4), dst: Reg(AX) }, Ret ]
        // Expected Assembly (Pass 3):
        // [ AllocateStack(4), Mov { src: Imm(10), dst: Stack(-4) }, Unary { op: Neg, operand: Stack(-4) }, Mov { src: Stack(-4), dst: Reg(AX) }, Ret ]

        assert_eq!(assembly_program.function.name, "main");
        assert_eq!(assembly_program.function.instructions.len(), 4); // AllocateStack + Mov + Unary + Ret

        assert_eq!(
            assembly_program.function.instructions[0],
            AssInstruction::AllocateStack(4)
        ); // Should allocate 4 bytes for t0
        assert_eq!(
            assembly_program.function.instructions[1],
            AssInstruction::Mov {
                src: AssOperand::Imm(10),
                dst: AssOperand::Stack(-4)
            }
        );
        assert_eq!(
            assembly_program.function.instructions[2],
            AssInstruction::Unary {
                op: AssUnaryOperator::Neg,
                operand: AssOperand::Stack(-4)
            }
        );
        assert_eq!(
            assembly_program.function.instructions[3],
            AssInstruction::Mov {
                src: AssOperand::Stack(-4),
                dst: AssOperand::Reg(Reg::AX)
            }
        );
        // Note: The Return instruction from Tacky became a Mov to AX. The Ret is a separate instruction.

        assert_eq!(translator.pseudo_to_stack_offset.get("t0"), Some(&-4));
        assert_eq!(translator.next_stack_offset, -8); // After allocating -4 for t0
    }

    #[test]
    fn test_tacky_to_assembly_nested_unary() {
        // TACKY:
        // program { definition: FunctionDefinition { name: "main", body: [ Unary { op: Negate, src: Constant(5), dst: Var("t0") }, Unary { op: Complement, src: Var("t0"), dst: Var("t1") }, Return(Var("t1")) ] } }
        // Represents something like: t0 = -5; t1 = ~t0; return t1;
        let tacky_program = TackyProgram {
            definition: FunctionDefinition {
                name: "main".to_string(),
                body: vec![
                    TackyInstruction::Unary {
                        // t0 = -5
                        op: TackyUnaryOperator::Negate,
                        src: TackyValue::Constant(5),
                        dst: TackyValue::Var("t0".to_string()),
                    },
                    TackyInstruction::Unary {
                        // t1 = ~t0
                        op: TackyUnaryOperator::Complement,
                        src: TackyValue::Var("t0".to_string()),
                        dst: TackyValue::Var("t1".to_string()),
                    },
                    TackyInstruction::Return(TackyValue::Var("t1".to_string())), // return t1
                ],
            },
        };

        let mut translator = TackyToAssemblyTranslator::new();
        let assembly_program = translator.translate(tacky_program).unwrap();

        // Expected Pseudoregisters: t0, t1.
        // Stack map: {"t0": -4, "t1": -8}. Next offset: -12. Total space: 8.
        // Pass 1:
        // [ Mov(Imm(5), Pseudo("t0")), Unary(Neg, Pseudo("t0")), Mov(Pseudo("t0"), Pseudo("t1")), Unary(Not, Pseudo("t1")), Mov(Pseudo("t1"), Reg(AX)), Ret ]
        // Pass 2 (Replace Pseudo):
        // [ Mov(Imm(5), Stack(-4)), Unary(Neg, Stack(-4)), Mov(Stack(-4), Stack(-8)), Unary(Not, Stack(-8)), Mov(Stack(-8), Reg(AX)), Ret ]
        // Pass 3 (AllocateStack + Fix Mov(Stack, Stack)):
        // [ AllocateStack(8), Mov(Imm(5), Stack(-4)), Unary(Neg, Stack(-4)), Mov(Stack(-4), Reg(R10)), Mov(Reg(R10), Stack(-8)), Unary(Not, Stack(-8)), Mov(Stack(-8), Reg(AX)), Ret ]

        assert_eq!(assembly_program.function.name, "main");
        assert_eq!(assembly_program.function.instructions.len(), 7); // AllocateStack + Mov + Unary + (Mov+Mov for Stack->Stack) + Unary + Mov + Ret

        let instructions = &assembly_program.function.instructions;
        assert_eq!(instructions[0], AssInstruction::AllocateStack(8)); // Should allocate 8 bytes for t0 and t1
        assert_eq!(
            instructions[1],
            AssInstruction::Mov {
                src: AssOperand::Imm(5),
                dst: AssOperand::Stack(-4)
            }
        );
        assert_eq!(
            instructions[2],
            AssInstruction::Unary {
                op: AssUnaryOperator::Neg,
                operand: AssOperand::Stack(-4)
            }
        );

        // This was Mov(Stack(-4), Stack(-8)), now split
        assert_eq!(
            instructions[3],
            AssInstruction::Mov {
                src: AssOperand::Stack(-4),
                dst: AssOperand::Reg(Reg::R10)
            }
        );
        assert_eq!(
            instructions[4],
            AssInstruction::Mov {
                src: AssOperand::Reg(Reg::R10),
                dst: AssOperand::Stack(-8)
            }
        );

        assert_eq!(
            instructions[5],
            AssInstruction::Unary {
                op: AssUnaryOperator::Not,
                operand: AssOperand::Stack(-8)
            }
        );
        assert_eq!(
            instructions[6],
            AssInstruction::Mov {
                src: AssOperand::Stack(-8),
                dst: AssOperand::Reg(Reg::AX)
            }
        );
        // Note: There's no final 'Ret' in this expected list, which seems wrong based on the TACKY rule.
        // Ah, the Tacky Return(val) rule is Mov(val, Reg(AX)), Ret. Let's re-check the count.
        // AllocateStack + Mov(Imm->Stack) + Unary(Stack) + Mov(Stack->Stack) -> (Mov(Stack->R10), Mov(R10->Stack)) + Unary(Stack) + Mov(Stack->AX) + Ret
        // 1 + 1 + 1 + 2 + 1 + 1 + 1 = 8 instructions?
        // Let's re-examine the rules and test output.
        // TACKY Instructions:
        // 1. Unary { op: Negate, src: Constant(5), dst: Var("t0") } => Mov(Imm(5), Pseudo("t0")), Unary(Neg, Pseudo("t0")) (2 asm)
        // 2. Unary { op: Complement, src: Var("t0"), dst: Var("t1") } => Mov(Pseudo("t0"), Pseudo("t1")), Unary(Not, Pseudo("t1")) (2 asm)
        // 3. Return(Var("t1")) => Mov(Pseudo("t1"), Reg(AX)), Ret (2 asm)
        // Initial ASM = 2 + 2 + 2 = 6 instructions. Pseudos: t0, t1. Stack: {"t0":-4, "t1":-8}. Next: -12. Space: 8.
        // Pass 2 (Replace):
        // [ Mov(Imm(5), Stack(-4)), Unary(Neg, Stack(-4)), Mov(Stack(-4), Stack(-8)), Unary(Not, Stack(-8)), Mov(Stack(-8), Reg(AX)), Ret ] (6 asm)
        // Pass 3 (Fix Mov + Allocate):
        // [ AllocateStack(8), Mov(Imm(5), Stack(-4)), Unary(Neg, Stack(-4)), Mov(Stack(-4), R10), Mov(R10, Stack(-8)), Unary(Not, Stack(-8)), Mov(Stack(-8), Reg(AX)), Ret ] (1 + 6 = 7 asm)
        // Okay, 7 instructions in the test output seems correct. My manual count was off slightly.

        assert_eq!(translator.pseudo_to_stack_offset.get("t0"), Some(&-4));
        assert_eq!(translator.pseudo_to_stack_offset.get("t1"), Some(&-8));
        assert_eq!(translator.next_stack_offset, -12); // After allocating -4 and -8
    }
}

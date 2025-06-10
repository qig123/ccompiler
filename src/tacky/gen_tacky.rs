// translator.rs

use crate::lexer::token::TokenType;
use crate::parser::c_ast::BlockItem;
use crate::{common_ids, parser};
use crate::{error::TackyError, tacky::tacky};
use parser::c_ast::{
    BinaryOperator as AstBinaryOperator, Expr as AstExpr, Function as AstFunction,
    LiteralExpr as AstLiteralExpr, Program as AstProgram, Stmt as AstStmt,
};
use std::vec::Vec;
use tacky::{
    BinaryOperator as TackyBinaryOperator, FunctionDefinition as TackyFunctionDefinition,
    Instruction as TackyInstruction, Program as TackyProgram, UnaryOperator as TackyUnaryOperator,
    Value as TackyValue,
};

// A helper to manage temporary variable names
struct TempGenerator {}

impl TempGenerator {
    fn new() -> Self {
        TempGenerator {}
    }

    fn next(&mut self) -> String {
        let name = common_ids::generate_translator_temp_name();
        name
    }
}
// A helper to manage label  names
struct LabelGenerator {}

impl LabelGenerator {
    fn new() -> Self {
        LabelGenerator {}
    }

    fn next(&mut self) -> String {
        common_ids::generate_translator_label_name()
    }
}
pub struct AstToTackyTranslator<'a> {
    temp_generator: TempGenerator,
    label_generator: LabelGenerator,
    source: &'a str,
    // Add other state if needed, like a symbol table
}

impl<'a> AstToTackyTranslator<'a> {
    pub fn new(source: &'a str) -> Self {
        AstToTackyTranslator {
            temp_generator: TempGenerator::new(),
            label_generator: LabelGenerator::new(),
            source: source,
        }
    }

    // Main entry point for translation
    // Translates the AST Program to a TACKY Program
    // Note: The TACKY Program only holds ONE function definition.
    // This translator will assume the AST Program has exactly one function
    // or only process the first one.
    pub fn translate_program(
        &mut self,
        ast_program: AstProgram,
    ) -> Result<TackyProgram, TackyError> {
        if ast_program.functions.is_empty() {
            // TACKY Program must have a function definition
            return Err(TackyError {
                message: "AST Program must contain at least one function for this TACKY target."
                    .to_string(),
            });
        }
        if ast_program.functions.len() > 1 {
            eprintln!(
                "Warning: AST Program contains more than one function, only the first will be translated to TACKY."
            );
        }

        let ast_function = ast_program.functions.into_iter().next().unwrap(); // Take the first function

        let tacky_function = self.translate_function(ast_function)?;
        Ok(TackyProgram {
            definition: tacky_function,
        })
    }

    // Translates an AST Function to a TACKY FunctionDefinition
    fn translate_function(
        &mut self,
        ast_function: AstFunction,
    ) -> Result<TackyFunctionDefinition, TackyError> {
        let function_name = ast_function.name.get_lexeme(self.source);
        let mut tacky_body: Vec<TackyInstruction> = Vec::new();

        // Translate each statement in the function body
        for stmt in ast_function.body.items {
            let instructions = self.translate_function_blockitem(stmt)?;
            tacky_body.extend(instructions);
        }
        //自动插入 constant 0 返回,后续可以优化
        tacky_body.push(TackyInstruction::Return(TackyValue::Constant(0)));
        Ok(TackyFunctionDefinition {
            name: function_name.to_string(),
            body: tacky_body,
        })
    }

    fn translate_function_blockitem(
        &mut self,
        ast_function: BlockItem,
    ) -> Result<Vec<TackyInstruction>, TackyError> {
        let mut instructions: Vec<TackyInstruction> = Vec::new();

        match ast_function {
            BlockItem::Stmt(ast_stmt) => {
                // Translate the AST statement to TACKY instructions
                let stmt_instructions = self.translate_stmt(ast_stmt)?;
                instructions.extend(stmt_instructions);
            }
            BlockItem::Declaration(decl) => {
                // Handle variable declaration
                // For TACKY, we assume declarations are just ignored as they don't affect the flow.
                // If the declaration has an initializer, we can translate it to a Copy instruction.
                if let Some(init_expr) = decl.init {
                    let (init_instructions, init_value) = self.translate_expr(*init_expr)?;
                    instructions.extend(init_instructions);
                    // Create a Copy instruction to assign the initial value to the variable
                    let dest_value = TackyValue::Var(decl.unique_name.clone());
                    instructions.push(TackyInstruction::Copy {
                        src: init_value,
                        dst: dest_value,
                    });
                }
            }
        }

        Ok(instructions)
    }

    // Translates an AST Statement to a sequence of TACKY Instructions
    fn translate_stmt(&mut self, ast_stmt: AstStmt) -> Result<Vec<TackyInstruction>, TackyError> {
        let mut instructions: Vec<TackyInstruction> = Vec::new();

        match ast_stmt {
            AstStmt::Return { keyword: _, value } => {
                // The provided TACKY ASDL is Return(val), implying a value is always returned.
                // We'll require a value in the AST Return statement for this translation.
                let expr = value.ok_or(TackyError {
                    message: "AST Return statement must have a value for this TACKY target."
                        .to_string(),
                })?;

                // Translate the expression. This will generate instructions to compute the expression
                // and return the TACKY Value representing the result.
                let (expr_instructions, result_value) = self.translate_expr(*expr)?;

                // Add the instructions generated by the expression
                instructions.extend(expr_instructions);

                // Add the final Return instruction
                instructions.push(TackyInstruction::Return(result_value));
            } // Handle other statement types if the AST had them (e.g., assignments)
            AstStmt::Expression { exp } => {
                // Translate the expression statement
                let (expr_instructions, _result_value) = self.translate_expr(*exp)?;
                instructions.extend(expr_instructions);
                // Note: Expression statements in TACKY do not produce a value, so we don't return anything.
            }
            AstStmt::Null => {
                // Null statements do nothing, so we can just return an empty instruction list.
                // This is a no-op in TACKY.
            }
            AstStmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                // Translate the condition expression
                let (cond_instructions, cond_value) = self.translate_expr(*condition)?;
                instructions.extend(cond_instructions);

                // Generate labels for the true and false branches
                let else_label_name = self.label_generator.next();
                let end_label_name = self.label_generator.next();

                instructions.push(TackyInstruction::JumpIfZero {
                    condition: cond_value,
                    target: else_label_name.clone(),
                });
                let then_instructions = self.translate_stmt(*then_branch)?;
                instructions.extend(then_instructions);

                // 跳过else分支
                instructions.push(TackyInstruction::Jump {
                    target: end_label_name.clone(),
                });

                // else分支标签
                instructions.push(TackyInstruction::Label {
                    name: else_label_name,
                });

                // 翻译else分支
                if let Some(else_branch) = else_branch {
                    let else_instructions = self.translate_stmt(*else_branch)?;
                    instructions.extend(else_instructions);
                }

                // 结束标签
                instructions.push(TackyInstruction::Label {
                    name: end_label_name,
                });
            }
            _ => {
                return Err(TackyError {
                    message: format!(
                        "Unsupported AST statement type for translation: {:?}",
                        ast_stmt
                    ),
                });
            }
        }

        Ok(instructions)
    }

    // Translates an AST Expression into a sequence of TACKY Instructions
    // and returns the TACKY Value that holds the expression's result.
    // Complex expressions are broken down using temporary variables.
    fn translate_expr(
        &mut self,
        ast_expr: AstExpr,
    ) -> Result<(Vec<TackyInstruction>, TackyValue), TackyError> {
        let mut instructions: Vec<TackyInstruction> = Vec::new();
        let result_value: TackyValue;

        match ast_expr {
            AstExpr::Literal(AstLiteralExpr::Integer(i)) => {
                result_value = TackyValue::Constant(i);
            }
            AstExpr::Unary { operator, right } => {
                // Translate the right-hand side expression first
                let (right_instructions, right_value) = self.translate_expr(*right)?;
                instructions.extend(right_instructions); // Add instructions from the right sub-expression

                // Determine the TACKY unary operator
                let tacky_op = match operator.token_type {
                    TokenType::Minus => TackyUnaryOperator::Negate,
                    TokenType::BitwiseNot => TackyUnaryOperator::Complement, // Assuming ~ maps to Complement
                    TokenType::Bang => TackyUnaryOperator::Bang, // Assuming ! maps to Bang (logical NOT)
                    _ => {
                        return Err(TackyError {
                            message: format!(
                                "Unsupported AST unary operator token: {:?}",
                                operator.token_type
                            ),
                        });
                    }
                };

                // The result of the unary operation must be stored in a temporary variable
                let temp_var_name = self.temp_generator.next();
                let dest_value = TackyValue::Var(temp_var_name);

                // Create the TACKY Unary instruction: dest_value = tacky_op right_value
                let unary_instruction = TackyInstruction::Unary {
                    op: tacky_op,
                    src: right_value,
                    dst: dest_value.clone(), // Clone dest_value as it's used in the instruction and as the result
                };
                instructions.push(unary_instruction);

                // The result of this unary expression is the temporary variable
                result_value = dest_value;
            }
            AstExpr::Grouping { expression } => {
                // Grouping is just for parsing precedence, it doesn't create new instructions or change the value.
                // Just translate the inner expression.
                return self.translate_expr(*expression); // Directly return the result of the inner expression
            }
            AstExpr::Binary {
                operator,
                left,
                right,
            } => {
                // We need a temporary variable to store the final result of the binary operation.
                // This variable will be assigned in different branches (e.g., true/false for boolean ops)
                // or hold the direct result of arithmetic/comparison ops.
                let result_var_name = self.temp_generator.next();
                let result_temp_value = TackyValue::Var(result_var_name); // This will be the returned TackyValue

                if operator == AstBinaryOperator::And {
                    // Translate left side
                    let (left_instructions, left_value) = self.translate_expr(*left)?;
                    instructions.extend(left_instructions);

                    // --- Short-circuiting for AND (A && B) ---
                    // If left is 0 (false), jump directly to the end and set result to 0.
                    // Otherwise, continue to evaluate the right side.
                    let false_label_name = self.label_generator.next();
                    let end_label_name = self.label_generator.next();

                    // If left_value is 0, jump to the false case label
                    instructions.push(TackyInstruction::JumpIfZero {
                        condition: left_value,
                        target: false_label_name.clone(),
                    });

                    // Translate right side (only executed if left_value is non-zero)
                    let (right_instructions, right_value) = self.translate_expr(*right)?;
                    instructions.extend(right_instructions);

                    // If right_value is 0, jump to the false case label
                    instructions.push(TackyInstruction::JumpIfZero {
                        condition: right_value,
                        target: false_label_name.clone(),
                    });

                    // If we reach here, both left and right were non-zero (true).
                    // Set the result variable to 1.
                    instructions.push(TackyInstruction::Copy {
                        src: TackyValue::Constant(1),
                        dst: result_temp_value.clone(),
                    });

                    // Jump to the end label to skip the false case assignment.
                    instructions.push(TackyInstruction::Jump {
                        target: end_label_name.clone(),
                    });

                    // --- False Case Label ---
                    // This is where execution jumps if either left or right was 0.
                    instructions.push(TackyInstruction::Label {
                        name: false_label_name,
                    });

                    // Set the result variable to 0.
                    instructions.push(TackyInstruction::Copy {
                        src: TackyValue::Constant(0),
                        dst: result_temp_value.clone(),
                    });

                    // --- End Label ---
                    // This is the merge point after the boolean logic.
                    instructions.push(TackyInstruction::Label {
                        name: end_label_name,
                    });

                    // The result of the AND expression is stored in result_temp_value.
                    result_value = result_temp_value;
                } else if operator == AstBinaryOperator::Or {
                    // Translate left side
                    let (left_instructions, left_value) = self.translate_expr(*left)?;
                    instructions.extend(left_instructions);

                    // --- Short-circuiting for OR (A || B) ---
                    // If left is non-zero (true), jump directly to the end and set result to 1.
                    // Otherwise, continue to evaluate the right side.
                    let true_label_name = self.label_generator.next();
                    let end_label_name = self.label_generator.next();

                    // If left_value is non-zero, jump to the true case label
                    instructions.push(TackyInstruction::JumpIfNotZero {
                        condition: left_value,
                        target: true_label_name.clone(),
                    });

                    // Translate right side (only executed if left_value is 0)
                    let (right_instructions, right_value) = self.translate_expr(*right)?;
                    instructions.extend(right_instructions);

                    // If right_value is non-zero, jump to the true case label
                    instructions.push(TackyInstruction::JumpIfNotZero {
                        condition: right_value,
                        target: true_label_name.clone(),
                    });

                    // If we reach here, both left and right were 0 (false).
                    // Set the result variable to 0.
                    instructions.push(TackyInstruction::Copy {
                        src: TackyValue::Constant(0),
                        dst: result_temp_value.clone(),
                    });

                    // Jump to the end label to skip the true case assignment.
                    instructions.push(TackyInstruction::Jump {
                        target: end_label_name.clone(),
                    });

                    // --- True Case Label ---
                    // This is where execution jumps if either left or right was non-zero.
                    instructions.push(TackyInstruction::Label {
                        name: true_label_name,
                    });

                    // Set the result variable to 1.
                    instructions.push(TackyInstruction::Copy {
                        src: TackyValue::Constant(1),
                        dst: result_temp_value.clone(),
                    });

                    // --- End Label ---
                    // This is the merge point after the boolean logic.
                    instructions.push(TackyInstruction::Label {
                        name: end_label_name,
                    });

                    // The result of the OR expression is stored in result_temp_value.
                    result_value = result_temp_value;
                } else {
                    // Handle other binary operators (arithmetic, comparison)

                    // Translate both sides first
                    let (left_instructions, left_value) = self.translate_expr(*left)?;
                    let (right_instructions, right_value) = self.translate_expr(*right)?;
                    instructions.extend(left_instructions);
                    instructions.extend(right_instructions);

                    // Determine the TACKY binary operator
                    let tacky_op = match operator {
                        AstBinaryOperator::Add => TackyBinaryOperator::Add,
                        AstBinaryOperator::Multiply => TackyBinaryOperator::Multiply,
                        AstBinaryOperator::Divide => TackyBinaryOperator::Divide,
                        AstBinaryOperator::Subtract => TackyBinaryOperator::Subtract,
                        AstBinaryOperator::Remainder => TackyBinaryOperator::Remainder,
                        AstBinaryOperator::EqualEqual => TackyBinaryOperator::EqualEqual,
                        AstBinaryOperator::Less => TackyBinaryOperator::Less,
                        AstBinaryOperator::LessEqual => TackyBinaryOperator::LessEqual,
                        AstBinaryOperator::Greater => TackyBinaryOperator::Greater,
                        AstBinaryOperator::GreaterEqual => TackyBinaryOperator::GreaterEqual,
                        AstBinaryOperator::BangEqual => TackyBinaryOperator::BangEqual,
                        // Logical AND/OR are handled above
                        AstBinaryOperator::And | AstBinaryOperator::Or => {
                            // This should not be reached because And/Or are handled above,
                            // but good to have a fallback or error here.
                            return Err(TackyError {
                                message: format!(
                                    "Logical AND/OR should have been handled by short-circuit logic, but fell through here: {:?}",
                                    operator
                                ),
                            });
                        }
                        _ => {
                            return Err(TackyError {
                                message: format!("Unsupported AST binary operator: {:?}", operator),
                            });
                        }
                    };

                    // Generate the TACKY Binary instruction: result_temp_value = src1 op src2
                    let binary_instruction = TackyInstruction::Binary {
                        op: tacky_op,
                        src1: left_value,
                        src2: right_value,
                        dst: result_temp_value.clone(), // Store result in our temp variable
                    };
                    instructions.push(binary_instruction);

                    // The result of this expression is the temporary variable
                    result_value = result_temp_value;
                }
            }
            AstExpr::Var {
                name: _,
                unique_name,
            } => {
                // For a variable, we just need to return its value
                // The unique_name is the TACKY variable name
                result_value = TackyValue::Var(unique_name);
            }
            AstExpr::Assignment { left, right } => {
                // Handle variable assignment: left = right
                // Translate the right-hand side first
                let (right_instructions, right_value) = self.translate_expr(*right)?;
                instructions.extend(right_instructions);

                // The left side must be a variable (for TACKY)
                if let AstExpr::Var {
                    name: _,
                    unique_name,
                } = *left
                {
                    // Create a Copy instruction to assign the value to the variable
                    let dest_value = TackyValue::Var(unique_name);
                    instructions.push(TackyInstruction::Copy {
                        src: right_value,
                        dst: dest_value.clone(),
                    });
                    result_value = dest_value; // The result of the assignment is the variable itself
                } else {
                    return Err(TackyError {
                        message: "Left-hand side of assignment must be a variable.".to_string(),
                    });
                }
            }
            AstExpr::Condtional {
                condition,
                left,
                right,
            } => {
                // Translate the condition first
                let (cond_instructions, cond_value) = self.translate_expr(*condition)?;
                instructions.extend(cond_instructions);

                // Generate labels for the true and false branches
                let false_label_name = self.label_generator.next();
                let end_label_name = self.label_generator.next();
                // if condtion is false ,we will jump to false_label_name
                instructions.push(TackyInstruction::JumpIfZero {
                    condition: cond_value,
                    target: false_label_name.clone(),
                });
                // Translate the left side (true branch)
                let (left_instructions, left_value) = self.translate_expr(*left)?;
                instructions.extend(left_instructions);
                let result_temp = self.temp_generator.next(); // 显式分配临时变量存储结果
                instructions.push(TackyInstruction::Copy {
                    src: left_value,
                    dst: TackyValue::Var(result_temp.clone()),
                }); //jump to end_label_name
                instructions.push(TackyInstruction::Jump {
                    target: end_label_name.clone(),
                });
                instructions.push(TackyInstruction::Label {
                    name: false_label_name,
                });
                // Translate the right side (false branch)
                let (right_instructions, right_value) = self.translate_expr(*right)?;
                instructions.extend(right_instructions);
                instructions.push(TackyInstruction::Copy {
                    src: right_value,
                    dst: TackyValue::Var(result_temp.clone()),
                });
                // End label for the conditional
                instructions.push(TackyInstruction::Label {
                    name: end_label_name,
                });
                result_value = TackyValue::Var(result_temp);
            } // _ => {
              //     return Err(TackyError {
              //         message: format!(
              //             "Unsupported AST expression type for translation: {:?}",
              //             ast_expr
              //         ),
              //     });
              // }
        }
        Ok((instructions, result_value))
    }
}

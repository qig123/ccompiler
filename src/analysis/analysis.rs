use crate::{common_ids, error::SemanticError, parser::c_ast::*};
use std::collections::HashMap;

fn generate_unique_variable_name_internal(ori: String) -> String {
    // Renamed to avoid conflict
    common_ids::generate_analysis_variable_name(ori)
}

pub struct SemanticAnalyzer<'a> {
    source: &'a str,
    scopes: Vec<HashMap<String, String>>, // 作用域栈
                                          // scopes[0] 是全局作用域 (如果支持的话), scopes.last() 是当前作用域
}

impl<'a> SemanticAnalyzer<'a> {
    pub fn new(source: &'a str) -> Self {
        SemanticAnalyzer {
            source,
            scopes: Vec::new(), // 初始化为空栈
        }
    }

    // --- 作用域管理辅助方法 ---
    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn leave_scope(&mut self) {
        // 确保不会 pop 空栈，尽管在正确使用时这不应该发生
        if !self.scopes.is_empty() {
            self.scopes.pop();
        } else {
            // 这是一个内部逻辑错误，可能需要 panic 或记录
            eprintln!("Error: Attempted to leave scope from an empty scope stack.");
        }
    }

    // 查找变量的唯一名称
    fn lookup_variable(&self, user_name: &str) -> Option<&String> {
        // 从当前作用域 (栈顶) 开始向上查找
        for scope_map in self.scopes.iter().rev() {
            if let Some(unique_name) = scope_map.get(user_name) {
                return Some(unique_name);
            }
        }
        None
    }

    // --- 主要分析方法 ---
    pub fn analyze(&mut self, program: Program) -> Result<Program, SemanticError> {
        // 如果有全局作用域的概念，可以在这里 enter_scope()
        // self.enter_scope(); // 全局作用域 (可选)

        let mut analyzed_functions = Vec::new();
        for func in program.functions {
            let analyzed_func = self.analyze_function(func)?;
            analyzed_functions.push(analyzed_func);
        }

        // self.leave_scope(); // 全局作用域 (可选)
        Ok(Program {
            functions: analyzed_functions,
        })
    }

    fn analyze_function(&mut self, function: Function) -> Result<Function, SemanticError> {
        self.enter_scope(); // 每个函数体开始一个新的作用域

        let mut analyzed_body_items = Vec::new();
        for block_item_enum in function.body.items {
            // function.body 是 Vec<Block (as BlockItem)>
            let analyzed_item = self.analyze_block_item(block_item_enum)?;
            analyzed_body_items.push(analyzed_item);
        }

        // println!("Leaving scope for function: {}", function_name_str);
        self.leave_scope(); // 函数体结束，离开作用域

        Ok(Function {
            name: function.name,
            body: Block {
                items: analyzed_body_items,
            },
        })
    }

    // 分析一个由声明和语句组成的列表 (例如函数体或复合语句的内部)
    // 这个函数不再直接管理作用域的进入和退出，调用者负责
    fn analyze_block_items_list(
        &mut self,
        items: Vec<BlockItem>,
    ) -> Result<Vec<BlockItem>, SemanticError> {
        let mut analyzed_items = Vec::new();
        for item in items {
            analyzed_items.push(self.analyze_block_item(item)?);
        }
        Ok(analyzed_items)
    }

    // 分析单个 BlockItem (Declaration 或 Stmt)
    // 这个函数也不直接管理作用域，它在已建立的作用域内操作
    fn analyze_block_item(
        &mut self,
        block_item_enum: BlockItem,
    ) -> Result<BlockItem, SemanticError> {
        match block_item_enum {
            BlockItem::Declaration(decl) => {
                let analyzed_decl = self.analyze_declaration(decl)?;
                Ok(BlockItem::Declaration(analyzed_decl))
            }
            BlockItem::Stmt(stmt) => {
                let analyzed_stmt = self.analyze_statement(stmt)?;
                Ok(BlockItem::Stmt(analyzed_stmt))
            }
        }
    }

    fn analyze_declaration(
        &mut self,
        declaration: Declaration,
    ) -> Result<Declaration, SemanticError> {
        let user_name = declaration.name.get_lexeme(self.source).to_string();

        // 获取当前作用域的 map (必须可变)
        let current_scope_map = self.scopes.last_mut().ok_or_else(|| {
            SemanticError::Internal("No current scope active for declaration.".to_string())
        })?;

        // 在当前作用域检查重复声明
        if current_scope_map.contains_key(&user_name) {
            return Err(SemanticError::DuplicateDeclaration {
                name: user_name,
                // token: declaration.name.clone() // 如果 SemanticError 可以携带 Token
            });
        }

        let unique_name = generate_unique_variable_name_internal(user_name.clone());
        current_scope_map.insert(user_name, unique_name.clone());

        let analyzed_init = if let Some(init_expr) = declaration.init {
            let analyzed_expr = self.analyze_expression(*init_expr)?;
            Some(Box::new(analyzed_expr))
        } else {
            None
        };

        Ok(Declaration {
            name: declaration.name,
            unique_name,
            init: analyzed_init,
        })
    }

    fn analyze_statement(&mut self, statement: Stmt) -> Result<Stmt, SemanticError> {
        match statement {
            Stmt::Return { keyword, value } => {
                let analyzed_value = if let Some(return_expr) = value {
                    let analyzed_expr = self.analyze_expression(*return_expr)?;
                    Some(Box::new(analyzed_expr))
                } else {
                    None
                };
                Ok(Stmt::Return {
                    keyword,
                    value: analyzed_value,
                })
            }
            Stmt::Expression { exp } => {
                let analyzed_exp = self.analyze_expression(*exp)?;
                Ok(Stmt::Expression {
                    exp: Box::new(analyzed_exp),
                })
            }
            Stmt::Null => Ok(Stmt::Null),
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let analyzed_condition = self.analyze_expression(*condition)?;

                // if 的 then 分支可以是单个语句或一个块，它们都会创建自己的（逻辑上的）作用域
                // 对于简单的语句，我们不需要显式 enter/leave scope，因为它们不引入新声明
                // 但如果 then_branch 是 Stmt::Compound，它内部会处理自己的作用域
                let analyzed_then_branch = self.analyze_statement_scoped(*then_branch)?;

                let analyzed_else_branch = if let Some(else_stmt) = else_branch {
                    Some(Box::new(self.analyze_statement_scoped(*else_stmt)?))
                } else {
                    None
                };
                Ok(Stmt::If {
                    condition: Box::new(analyzed_condition),
                    then_branch: Box::new(analyzed_then_branch),
                    else_branch: analyzed_else_branch,
                })
            }
            Stmt::Compound(block) => {
                // 假设你的 Stmt 枚举有这个变体
                self.enter_scope();
                // println!("Entering scope for compound statement.");
                let analyzed_items = self.analyze_block_items_list(block.items)?;
                // println!("Leaving scope for compound statement.");
                self.leave_scope();
                Ok(Stmt::Compound(Block {
                    items: analyzed_items,
                }))
            } // 你之前的 _ => Err(...) 可能需要移除或调整，因为 Stmt::Compound 也是一个有效的语句
        }
    }

    // 辅助函数，用于分析可能引入新作用域的语句（如复合语句）
    // 或者只是为了代码结构清晰
    fn analyze_statement_scoped(&mut self, statement: Stmt) -> Result<Stmt, SemanticError> {
        // 如果 statement 本身是 Stmt::Compound，它会在 analyze_statement 中处理自己的作用域
        // 其他类型的语句通常不直接创建新的变量作用域 (除非它们是块)
        self.analyze_statement(statement)
    }

    fn analyze_expression(&mut self, expression: Expr) -> Result<Expr, SemanticError> {
        match expression {
            Expr::Literal(lit) => Ok(Expr::Literal(lit)),
            Expr::Unary { operator, right } => {
                let analyzed_right = self.analyze_expression(*right)?;
                Ok(Expr::Unary {
                    operator,
                    right: Box::new(analyzed_right),
                })
            }
            Expr::Grouping { expression } => {
                let analyzed_expression = self.analyze_expression(*expression)?;
                Ok(Expr::Grouping {
                    expression: Box::new(analyzed_expression),
                })
            }
            Expr::Binary {
                operator,
                left,
                right,
            } => {
                let analyzed_left = self.analyze_expression(*left)?;
                let analyzed_right = self.analyze_expression(*right)?;
                Ok(Expr::Binary {
                    operator,
                    left: Box::new(analyzed_left),
                    right: Box::new(analyzed_right),
                })
            }
            Expr::Var {
                name,
                unique_name: _old_unique_name,
            } => {
                let user_name = name.get_lexeme(self.source).to_string();
                match self.lookup_variable(&user_name) {
                    Some(found_unique_name) => Ok(Expr::Var {
                        name,
                        unique_name: found_unique_name.clone(),
                    }),
                    None => Err(SemanticError::UndeclaredVariable {
                        name: user_name,
                        // token: name.clone() // 如果 SemanticError 可以携带 Token
                    }),
                }
            }
            Expr::Assignment { left, right } => {
                // 首先分析右侧，因为它可能使用旧的变量值
                let analyzed_right = self.analyze_expression(*right)?;
                // 然后分析左侧，确保它是有效的左值，并获取其 unique_name
                let analyzed_left = self.analyze_expression(*left)?;

                // 检查左值是否为 Var (或其他允许的左值类型，如果你的语言更复杂)
                match &analyzed_left {
                    Expr::Var { .. } => { /* Var is a valid LValue */ }
                    _ => {
                        return Err(SemanticError::InvalidLvalue {
                            // 最好能提供表达式的文本或位置
                            description: format!(
                                "Expression {:?} is not a valid LValue.",
                                analyzed_left
                            ),
                        });
                    }
                }

                Ok(Expr::Assignment {
                    left: Box::new(analyzed_left), // 已经填充了正确的 unique_name
                    right: Box::new(analyzed_right),
                })
            }
            Expr::Condtional {
                condition,
                left,
                right,
            } => {
                let analyzed_condition = self.analyze_expression(*condition)?;
                let analyzed_left = self.analyze_expression(*left)?;
                let analyzed_right = self.analyze_expression(*right)?;
                Ok(Expr::Condtional {
                    condition: Box::new(analyzed_condition),
                    left: Box::new(analyzed_left),
                    right: Box::new(analyzed_right),
                })
            }
        }
    }
}

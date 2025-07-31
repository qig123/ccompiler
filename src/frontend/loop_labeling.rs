// src/frontend/loop_labeling.rs

//! **循环标签解析 (Loop Labeling)**
//!
//! 该模块是语义分析的第二阶段，在标识符解析之后运行。
//! 它的核心任务是为循环语句（`while`, `do-while`, `for`）生成唯一的标签，
//! 并将这些标签与相应的 `break` 和 `continue` 语句关联起来。
//!
//! ## 主要职责
//!
//! 1.  **遍历AST**:
//!     -   通过深度优先搜索（DFS）的方式遍历整个抽象语法树。
//!
//! 2.  **循环栈管理**:
//!     -   维护一个 `loop_stack`，用于跟踪当前嵌套的循环层级。
//!     -   当进入一个新的循环语句时，会生成一个唯一的循环标签（例如，`loop.0`, `loop.1`），并将其压入栈顶。
//!     -   当完成对该循环体的遍历后，将其标签从栈中弹出。
//!
//! 3.  **标签关联**:
//!     -   在遍历过程中，如果遇到 `break` 或 `continue` 语句，它会从 `loop_stack` 的栈顶取出当前最内层循环的标签。
//!     -   然后，它将这个标签填充到 `break` 或 `continue` 语句的AST节点中。
//!     -   这个标签将在后续的代码生成阶段用于实现正确的跳转逻辑（例如，`break` 跳转到循环结束点，`continue` 跳转到循环开始点）。
//!
//! 4.  **错误处理**:
//!     -   捕捉与循环控制相关的语义错误，例如：
//!         -   在任何循环之外使用 `break` 语句。
//!         -   在任何循环之外使用 `continue` 语句。

use crate::{
    frontend::c_ast::{Block, BlockItem, FunDecl, Program, Statement},
    UniqueNameGenerator,
};

/// 循环标签解析器的状态机。
pub struct LoopLabeling<'a> {
    /// 循环标签栈，用于跟踪当前所在的循环。
    /// 每当进入一个循环，就将新生成的唯一循环标签压入此栈。
    loop_stack: Vec<String>,
    /// 用于生成唯一标签名的工具。
    name_gen: &'a mut UniqueNameGenerator,
}

impl<'a> LoopLabeling<'a> {
    /// 创建一个新的循环标签解析器。
    pub fn new(g: &'a mut UniqueNameGenerator) -> Self {
        LoopLabeling {
            loop_stack: Vec::new(),
            name_gen: g,
        }
    }

    /// 解析器的主入口点，负责遍历并标记整个程序中的所有循环。
    pub fn label_loops_in_program(&mut self, ast: &Program) -> Result<Program, String> {
        let mut labeled_functions = Vec::new();
        for f in &ast.functions {
            let new_f = self.label_loops_in_function_decl(f)?;
            labeled_functions.push(new_f);
        }
        Ok(Program {
            functions: labeled_functions,
        })
    }

    /// 遍历函数声明，主要处理其函数体。
    fn label_loops_in_function_decl(&mut self, f: &FunDecl) -> Result<FunDecl, String> {
        let new_body = if let Some(b) = &f.body {
            Some(self.label_loops_in_block(b)?)
        } else {
            None
        };

        Ok(FunDecl {
            name: f.name.clone(),
            parameters: f.parameters.clone(),
            body: new_body,
        })
    }

    /// 遍历代码块中的每一个条目。
    fn label_loops_in_block(&mut self, block: &Block) -> Result<Block, String> {
        let mut new_items = Vec::new();
        for item in &block.0 {
            new_items.push(self.label_loops_in_block_item(item)?);
        }
        Ok(Block(new_items))
    }

    /// 遍历块内条目，区分声明和语句。
    fn label_loops_in_block_item(&mut self, item: &BlockItem) -> Result<BlockItem, String> {
        match item {
            // 声明本身不包含循环控制，因此我们直接克隆它。
            // 一个更完备的实现可能需要递归检查初始化表达式，但在这里我们简化处理。
            BlockItem::D(d) => Ok(BlockItem::D(d.clone())),
            BlockItem::S(s) => {
                let new_s = self.label_loops_in_statement(s)?;
                Ok(BlockItem::S(new_s))
            }
        }
    }

    /// 这是核心的遍历函数，处理各种语句类型。
    fn label_loops_in_statement(&mut self, stmt: &Statement) -> Result<Statement, String> {
        match stmt {
            // --- 循环语句处理 ---

            Statement::While { condition, body, .. } => {
                // 1. 为此循环生成一个新的、唯一的标签。
                let loop_label = self.name_gen.new_loop_label("loop");
                // 2. 将标签压入栈中，表示我们进入了一个新的循环层级。
                self.loop_stack.push(loop_label.clone());

                // 3. 递归地处理循环体。在循环体中遇到的任何 `break` 或 `continue`
                //    都将使用我们刚刚压入栈的标签。
                let new_body = self.label_loops_in_statement(body)?;

                // 4. 循环体处理完毕，将此循环的标签弹出栈。
                self.loop_stack.pop();

                // 5. 返回一个新的、已填充标签的 `While` 语句节点。
                Ok(Statement::While {
                    condition: condition.clone(),
                    body: Box::new(new_body),
                    label: Some(loop_label),
                })
            }

            Statement::DoWhile { body, condition, .. } => {
                let loop_label = self.name_gen.new_loop_label("loop");
                self.loop_stack.push(loop_label.clone());
                let new_body = self.label_loops_in_statement(body)?;
                self.loop_stack.pop();
                Ok(Statement::DoWhile {
                    body: Box::new(new_body),
                    condition: condition.clone(),
                    label: Some(loop_label),
                })
            }

            Statement::For { init, condition, post, body, .. } => {
                let loop_label = self.name_gen.new_loop_label("loop");
                self.loop_stack.push(loop_label.clone());
                let new_body = self.label_loops_in_statement(body)?;
                self.loop_stack.pop();
                Ok(Statement::For {
                    init: init.clone(),
                    condition: condition.clone(),
                    post: post.clone(),
                    body: Box::new(new_body),
                    label: Some(loop_label),
                })
            }

            // --- Break/Continue 处理 ---

            Statement::Break(_) => {
                // 检查循环栈是否为空。如果为空，说明 `break` 不在任何循环内。
                if let Some(current_loop_label) = self.loop_stack.last() {
                    // 如果不为空，则使用栈顶的标签。
                    Ok(Statement::Break(current_loop_label.clone()))
                } else {
                    Err("Semantic Error: 'break' statement not in a loop or switch statement.".to_string())
                }
            }

            Statement::Continue(_) => {
                if let Some(current_loop_label) = self.loop_stack.last() {
                    Ok(Statement::Continue(current_loop_label.clone()))
                } else {
                    Err("Semantic Error: 'continue' statement not in a loop.".to_string())
                }
            }

            // --- 其他语句的递归处理 ---

            Statement::Compound(b) => {
                let new_b = self.label_loops_in_block(b)?;
                Ok(Statement::Compound(new_b))
            }

            Statement::If { condition, then_stmt, else_stmt } => {
                let new_then = self.label_loops_in_statement(then_stmt)?;
                let new_else = else_stmt
                    .as_ref()
                    .map(|s| self.label_loops_in_statement(s))
                    .transpose()?;
                Ok(Statement::If {
                    condition: condition.clone(),
                    then_stmt: Box::new(new_then),
                    else_stmt: new_else.map(Box::new),
                })
            }

            // 对于不包含控制流的简单语句，直接克隆即可。
            Statement::Return(e) => Ok(Statement::Return(e.clone())),
            Statement::Expression(e) => Ok(Statement::Expression(e.clone())),
            Statement::Null => Ok(Statement::Null),
        }
    }
}
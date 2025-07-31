// src/frontend/loop_labeling.rs

use crate::{
    UniqueNameGenerator,
    frontend::c_ast::{Block, BlockItem, FunDecl, Program, Statement},
};

pub struct LoopLabeling<'a> {
    loop_stack: Vec<String>, // 只存储循环 ID 的栈
    name_gen: &'a mut UniqueNameGenerator,
}

impl<'a> LoopLabeling<'a> {
    pub fn new(g: &'a mut UniqueNameGenerator) -> Self {
        LoopLabeling {
            loop_stack: Vec::new(),
            name_gen: g,
        }
    }

    // 主入口函数
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

    fn label_loops_in_function_decl(&mut self, f: &FunDecl) -> Result<FunDecl, String> {
        // 函数本身不创建循环，但我们需要遍历它的 body
        if let Some(b) = &f.body {
            let new_body = self.label_loops_in_block(&b)?;
            Ok(FunDecl {
                name: f.name.clone(),
                parameters: f.parameters.clone(),
                body: Some(new_body),
            })
        } else {
            Ok(FunDecl {
                name: f.name.clone(),
                parameters: f.parameters.clone(),
                body: None,
            })
        }
    }

    fn label_loops_in_block(&mut self, block: &Block) -> Result<Block, String> {
        let mut new_items = Vec::new();
        for item in &block.0 {
            let new_item = self.label_loops_in_block_item(item)?;
            new_items.push(new_item);
        }
        Ok(Block(new_items))
    }

    fn label_loops_in_block_item(&mut self, item: &BlockItem) -> Result<BlockItem, String> {
        match item {
            // 声明不包含循环控制语句，但其初始化表达式可能包含（虽然在C中非法）
            // 这里我们简化，假设声明的初始化中没有 break/continue
            BlockItem::D(d) => Ok(BlockItem::D(d.clone())),
            BlockItem::S(s) => {
                let new_s = self.label_loops_in_statement(s)?;
                Ok(BlockItem::S(new_s))
            }
        }
    }

    fn label_loops_in_statement(&mut self, stmt: &Statement) -> Result<Statement, String> {
        match stmt {
            // 对于非循环、非break/continue的语句，只需递归遍历其子语句
            Statement::Return(e) => Ok(Statement::Return(e.clone())),
            Statement::Expression(e) => Ok(Statement::Expression(e.clone())),
            Statement::Null => Ok(Statement::Null),
            Statement::Compound(b) => {
                let new_b = self.label_loops_in_block(b)?;
                Ok(Statement::Compound(new_b))
            }
            Statement::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
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

            Statement::Break(_) => {
                // 忽略原来的占位符
                if let Some(current_loop_label) = self.loop_stack.last() {
                    Ok(Statement::Break(current_loop_label.clone()))
                } else {
                    Err("'break' statement not in loop or switch statement".to_string())
                }
            }
            Statement::Continue(_) => {
                // 忽略原来的占位符
                if let Some(current_loop_label) = self.loop_stack.last() {
                    Ok(Statement::Continue(current_loop_label.clone()))
                } else {
                    Err("'continue' statement not in a loop".to_string())
                }
            }

            Statement::While {
                condition, body, ..
            } => {
                let loop_label = self.name_gen.new_loop_label("loop");
                self.loop_stack.push(loop_label.clone());

                let new_body = self.label_loops_in_statement(body)?;

                self.loop_stack.pop();

                Ok(Statement::While {
                    condition: condition.clone(),
                    body: Box::new(new_body),
                    label: Some(loop_label),
                })
            }

            Statement::DoWhile {
                body, condition, ..
            } => {
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

            Statement::For {
                init,
                condition,
                post,
                body,
                ..
            } => {
                let loop_label = self.name_gen.new_loop_label("loop");
                self.loop_stack.push(loop_label.clone());

                let new_body = self.label_loops_in_statement(body)?;

                self.loop_stack.pop();

                Ok(Statement::For {
                    init: init.clone(), // init, condition, post不包含循环控制
                    condition: condition.clone(),
                    post: post.clone(),
                    body: Box::new(new_body),
                    label: Some(loop_label),
                })
            }
        }
    }
}

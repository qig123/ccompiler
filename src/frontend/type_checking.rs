use std::{collections::HashMap, thread::sleep};

use crate::frontend::c_ast::{
    Block, BlockItem, Declaration, Expression, ForInit, FunDecl, Program, Statement, VarDecl,
};
#[derive(Debug, PartialEq)]
pub struct SymbolInfo {
    tpye: CType,
    defined: bool,
}

#[derive(Debug, PartialEq)]
pub enum CType {
    Int,
    FunType { param_count: usize },
}

#[derive(Debug, PartialEq)]
pub struct TypeChecker<'a> {
    env_stack: &'a mut Vec<HashMap<String, SymbolInfo>>,
}

impl<'a> TypeChecker<'a> {
    pub fn new(env_stack: &'a mut Vec<HashMap<String, SymbolInfo>>) -> Self {
        TypeChecker { env_stack }
    }

    pub fn typecheck_program(&mut self, ast: &Program) -> Result<(), String> {
        self.env_stack.push(HashMap::new());

        for f in &ast.functions {
            self.typecheck_function_decl(f)?;
        }

        self.env_stack.pop();
        Ok(())
    }

    fn typecheck_function_decl(&mut self, f: &FunDecl) -> Result<(), String> {
        let fun_type = CType::FunType {
            param_count: f.parameters.len(),
        };
        let has_body = !f.body.is_none();
        let mut already_def = false;
        println!("typecheck_function_decl{:?}", self.env_stack);
        let (existing_entry, _) = self.find_identifier_in_all_scopes(&f.name);
        if let Some(old_decl) = existing_entry {
            if old_decl.tpye != fun_type {
                return Err(format!(
                    "Semantic Error: Incompatible function declarations.",
                ));
            }
            already_def = old_decl.defined;
            if already_def && has_body {
                return Err(format!(
                    "Semantic Error: Function is defined more than once.",
                ));
            }
        }
        self.insert_identifier(
            f.name.clone(),
            SymbolInfo {
                tpye: fun_type,
                defined: (already_def || has_body),
            },
        );

        self.env_stack.push(HashMap::new());

        for p_name in &f.parameters {
            self.insert_identifier(
                p_name.clone(),
                SymbolInfo {
                    tpye: CType::Int,
                    defined: false, //这是变量，这个值好像无所谓
                },
            );
        }
        if let Some(body_block) = &f.body {
            for item in &body_block.0 {
                self.typecheck_block_item(item)?;
            }
        }
        self.env_stack.pop();
        Ok(())
    }

    /// 解析代码块（Block）。
    /// 一个块会引入一个新的作用域。
    fn typecheck_block(&mut self, block: &Block) -> Result<(), String> {
        self.env_stack.push(HashMap::new()); // 进入新作用域

        for item in &block.0 {
            self.typecheck_block_item(item)?;
        }

        self.env_stack.pop(); // 退出作用域
        Ok(())
    }

    fn typecheck_block_item(&mut self, item: &BlockItem) -> Result<(), String> {
        match item {
            BlockItem::D(d) => {
                self.typecheck_declaration(d)?;
                Ok(())
            }
            BlockItem::S(s) => {
                self.typecheck_statement(s)?;
                Ok(())
            }
        }
    }

    fn typecheck_declaration(&mut self, d: &Declaration) -> Result<(), String> {
        match d {
            Declaration::Variable(v) => {
                self.typecheck_variable_declaration(v)?;
                Ok(())
            }
            Declaration::Fun(f) => {
                self.typecheck_function_decl(f)?;
                Ok(())
            }
        }
    }

    fn typecheck_variable_declaration(&mut self, v: &VarDecl) -> Result<(), String> {
        self.insert_identifier(
            v.name.clone(),
            SymbolInfo {
                tpye: CType::Int,
                defined: false,
            },
        );
        match &v.init {
            Some(e) => self.typecheck_expression(e)?,
            None => {}
        }
        Ok(())
    }

    /// 解析语句。
    fn typecheck_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Expression(e) => {
                self.typecheck_expression(e)?;
                Ok(())
            }
            Statement::Return(e) => {
                self.typecheck_expression(e)?;
                Ok(())
            }
            Statement::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                self.typecheck_expression(condition)?;
                self.typecheck_statement(then_stmt)?;
                if let Some(es) = else_stmt {
                    self.typecheck_statement(es)?
                }
                Ok(())
            }
            Statement::Compound(b) => {
                self.typecheck_block(b)?;
                Ok(())
            }
            Statement::While {
                condition, body, ..
            } => {
                self.typecheck_expression(condition)?;
                self.typecheck_statement(body)?;
                Ok(())
            }
            Statement::DoWhile {
                body, condition, ..
            } => {
                self.typecheck_statement(body)?;
                self.typecheck_expression(condition)?;
                Ok(())
            }
            Statement::For {
                init,
                condition,
                post,
                body,
                ..
            } => {
                self.env_stack.push(HashMap::new());
                self.resolve_for_init(init)?;
                match condition {
                    Some(c) => Some(self.typecheck_expression(c)?),
                    None => None,
                };
                match post {
                    Some(p) => Some(self.typecheck_expression(p)?),
                    None => None,
                };
                self.typecheck_statement(body)?;
                self.env_stack.pop();

                Ok(())
            }
            Statement::Null => Ok(()),
            Statement::Break(_) => Ok(()),
            Statement::Continue(_) => Ok(()),
        }
    }

    /// 解析 `for` 循环的初始化部分。
    fn resolve_for_init(&mut self, init: &ForInit) -> Result<(), String> {
        match init {
            ForInit::InitDecl(d) => {
                self.typecheck_variable_declaration(d)?;
                Ok(())
            }
            ForInit::InitExp(Some(e)) => {
                self.typecheck_expression(e)?;
                Ok(())
            }
            ForInit::InitExp(None) => Ok(()),
        }
    }

    /// 解析表达式。
    fn typecheck_expression(&mut self, e: &Expression) -> Result<(), String> {
        match e {
            Expression::Assignment { left, right } => {
                self.typecheck_expression(left)?;
                self.typecheck_expression(right)?;
                Ok(())
            }
            Expression::Var(id) => {
                let (info, _) = self.find_identifier_in_all_scopes(id);
                if let Some(r) = info {
                    if r.tpye != CType::Int {
                        return Err(format!("Semantic Error: Function name used as variable",));
                    }
                }
                Ok(())
            }
            Expression::FuncCall { name, args } => {
                let (info, _) = self.find_identifier_in_all_scopes(name);
                if let Some(r) = info {
                    match r.tpye {
                        CType::Int => {
                            return Err(format!("Semantic Error: Variable used as function name",));
                        }
                        CType::FunType { param_count } => {
                            if param_count != args.len() {
                                return Err(format!(
                                    "Semantic Error: Function called with the wrong number of arguments",
                                ));
                            }
                        }
                    }
                    for arg in args {
                        self.typecheck_expression(arg)?;
                    }
                }

                Ok(())
            }
            Expression::Binary { op: _, left, right } => {
                self.typecheck_expression(left)?;
                self.typecheck_expression(right)?;
                Ok(())
            }
            Expression::Unary { op: _, exp } => {
                self.typecheck_expression(exp)?;
                Ok(())
            }
            Expression::Conditional {
                condition,
                left,
                right,
            } => {
                self.typecheck_expression(condition)?;
                self.typecheck_expression(left)?;
                self.typecheck_expression(right)?;
                Ok(())
            }
            Expression::Constant(_) => Ok(()),
        }
    }

    // --- 作用域和符号表辅助函数 ---

    /// 从内到外查找所有作用域中的标识符。
    /// 返回找到的标识符信息以及一个布尔值，该值指示是否在最内层作用域找到。
    fn find_identifier_in_all_scopes(&self, name: &str) -> (Option<&SymbolInfo>, bool) {
        if let Some(current_scope) = self.env_stack.last() {
            if let Some(info) = current_scope.get(name) {
                return (Some(info), true); // 在当前作用域找到
            }
        }
        for scope in self.env_stack.iter().rev().skip(1) {
            if let Some(info) = scope.get(name) {
                return (Some(info), false); // 在外部作用域找到
            }
        }
        (None, false) // 未找到
    }

    /// 在当前作用域中插入一个新的标识符。
    fn insert_identifier(&mut self, name: String, info: SymbolInfo) {
        if let Some(current_scope) = self.env_stack.last_mut() {
            current_scope.insert(name, info);
        }
    }
}

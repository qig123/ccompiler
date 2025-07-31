use std::collections::HashMap;

use crate::frontend::c_ast::{
    Block, BlockItem, Declaration, Expression, ForInit, FunDecl, Program, Statement, VarDecl,
};
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolInfo {
    tpye: CType,
    defined: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CType {
    Int,
    FunType { param_count: usize },
}

#[derive(Debug)]
pub struct TypeChecker {
    /// 全局函数表，生命周期贯穿整个检查过程
    symbol_tables: HashMap<String, SymbolInfo>,
    /// 局部作用域栈，只用于变量和参数
    scopes: Vec<HashMap<String, SymbolInfo>>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            symbol_tables: HashMap::new(),
            scopes: Vec::new(),
        }
    }

    /// 公开的入口方法，返回最终的全局函数表
    pub fn typecheck_program(
        mut self,
        ast: &Program,
    ) -> Result<HashMap<String, SymbolInfo>, String> {
        // 全局作用域是最外层，我们为它 push 一个 scope
        // 虽然C语言不允许在顶层声明变量，但这是个好习惯
        self.push_scope();

        for decl in &ast.functions {
            // 现在Program还没有全局变量
            self.typecheck_function_decl(decl)?;
        }

        self.pop_scope();
        Ok(self.symbol_tables) // 返回包含了所有函数信息的表
    }

    // --- 主要检查逻辑 ---

    fn typecheck_function_decl(&mut self, f: &FunDecl) -> Result<(), String> {
        let fun_type = CType::FunType {
            param_count: f.parameters.len(),
        };
        let has_body = f.body.is_some();

        if let Some(old_decl) = self.symbol_tables.get(&f.name) {
            // 检查类型兼容性
            if old_decl.tpye != fun_type {
                return Err(format!(
                    "Semantic Error: Incompatible function declarations for '{}'.",
                    f.name
                ));
            }
            // 检查重复定义
            if old_decl.defined && has_body {
                return Err(format!(
                    "Semantic Error: Redefinition of function '{}'.",
                    f.name
                ));
            }
        }

        // 更新或插入函数信息到全局表
        // 使用 entry API 更高效
        let defined_status = self
            .symbol_tables
            .get(&f.name)
            .map_or(has_body, |d| d.defined || has_body);
        self.symbol_tables.insert(
            f.name.clone(),
            SymbolInfo {
                tpye: fun_type,
                defined: defined_status,
            },
        );

        // 如果有函数体，才需要处理参数和局部作用域
        if let Some(body_block) = &f.body {
            self.push_scope(); // 为函数体创建一个新的作用域

            // 将参数作为局部变量添加到新作用域
            for p_name in &f.parameters {
                self.insert_variable(
                    p_name.clone(),
                    SymbolInfo {
                        tpye: CType::Int,
                        defined: true, // 参数可以认为是被“定义”的
                    },
                )?;
            }
            // 检查函数体
            self.typecheck_block_body(body_block)?;

            self.pop_scope(); // 退出函数作用域
        }
        Ok(())
    }

    fn typecheck_variable_declaration(&mut self, v: &VarDecl) -> Result<(), String> {
        // 关键改动：变量总是被添加到当前的局部作用域
        self.insert_variable(
            v.name.clone(),
            SymbolInfo {
                tpye: CType::Int,
                defined: v.init.is_some(), // 如果有初始化，可以认为它被定义了
            },
        )?;
        if let Some(e) = &v.init {
            self.typecheck_expression(e)?
        }
        Ok(())
    }

    fn typecheck_expression(&mut self, e: &Expression) -> Result<(), String> {
        match e {
            Expression::Var(id) => {
                // 关键改动：使用新的查找函数
                match self.find_identifier(id) {
                    Some(info) => {
                        if info.tpye != CType::Int {
                            Err(format!(
                                "Semantic Error: Function '{}' used as a variable.",
                                id
                            ))
                        } else {
                            Ok(())
                        }
                    }
                    None => Err(format!(
                        "Semantic Error: Use of undeclared identifier '{}'.",
                        id
                    )),
                }
            }
            Expression::FuncCall { name, args } => {
                // 关键改动：使用新的查找函数
                match self.find_identifier(name) {
                    Some(info) => match info.tpye {
                        CType::Int => Err(format!(
                            "Semantic Error: Variable '{}' used as a function.",
                            name
                        )),
                        CType::FunType { param_count } => {
                            if param_count != args.len() {
                                Err(format!(
                                    "Semantic Error: Function '{}' called with wrong number of arguments. Expected {}, got {}.",
                                    name,
                                    param_count,
                                    args.len()
                                ))
                            } else {
                                for arg in args {
                                    self.typecheck_expression(arg)?;
                                }
                                Ok(())
                            }
                        }
                    },
                    None => Err(format!(
                        "Semantic Error: Call to undeclared function '{}'.",
                        name
                    )),
                }
            }
            // ... 其他表达式分支保持不变 ...
            Expression::Assignment { left, right } => {
                self.typecheck_expression(left)?;
                self.typecheck_expression(right)?;
                Ok(())
            }
            Expression::Binary { op: _, left, right } => {
                self.typecheck_expression(left)?;
                self.typecheck_expression(right)?;
                Ok(())
            }
            // ... etc
            _ => Ok(()), // 简化处理
        }
    }

    // --- 作用域和声明辅助函数 ---

    /// 查找标识符。
    /// 规则：先从内到外查找局部作用域，如果没找到，再查找全局函数表。
    fn find_identifier(&self, name: &str) -> Option<SymbolInfo> {
        // 1. 遍历 scopes 栈，从内向外 (rev)
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info.clone()); // 找到局部变量/参数，立即返回
            }
        }
        // 2. 如果局部没找到，查找全局函数表
        if let Some(info) = self.symbol_tables.get(name) {
            return Some(info.clone());
        }
        // 3. 彻底没找到
        None
    }

    /// 向当前作用域插入一个变量。
    /// C语言允许在内层作用域隐藏外层作用域的同名变量，所以我们只检查当前作用域。
    fn insert_variable(&mut self, name: String, info: SymbolInfo) -> Result<(), String> {
        let current_scope = self
            .scopes
            .last_mut()
            .expect("Cannot insert variable without a scope. This is a compiler bug.");

        if current_scope.contains_key(&name) {
            Err(format!(
                "Semantic Error: Redefinition of variable '{}' in the same scope.",
                name
            ))
        } else {
            current_scope.insert(name, info);
            Ok(())
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn typecheck_block_body(&mut self, block: &Block) -> Result<(), String> {
        for item in &block.0 {
            self.typecheck_block_item(item)?;
        }
        Ok(())
    }

    fn typecheck_block_item(&mut self, item: &BlockItem) -> Result<(), String> {
        match item {
            BlockItem::D(d) => self.typecheck_declaration(d),
            BlockItem::S(s) => self.typecheck_statement(s),
        }
    }

    fn typecheck_declaration(&mut self, d: &Declaration) -> Result<(), String> {
        match d {
            Declaration::Variable(v) => self.typecheck_variable_declaration(v),
            Declaration::Fun(f) => self.typecheck_function_decl(f),
        }
    }

    fn typecheck_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Compound(b) => {
                self.push_scope();
                self.typecheck_block_body(b)?;
                self.pop_scope();
                Ok(())
            }
            Statement::For {
                init,
                condition,
                post,
                body,
                ..
            } => {
                self.push_scope();
                self.resolve_for_init(init)?;
                if let Some(c) = condition {
                    self.typecheck_expression(c)?;
                }
                if let Some(p) = post {
                    self.typecheck_expression(p)?;
                }
                self.typecheck_statement(body)?;
                self.pop_scope();
                Ok(())
            }
            // 其他语句...
            Statement::Expression(e) => self.typecheck_expression(e),
            Statement::Return(e) => self.typecheck_expression(e),
            Statement::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                self.typecheck_expression(condition)?;
                self.typecheck_statement(then_stmt)?;
                if let Some(es) = else_stmt {
                    self.typecheck_statement(es)?;
                }
                Ok(())
            }
            // ...
            _ => Ok(()),
        }
    }

    fn resolve_for_init(&mut self, init: &ForInit) -> Result<(), String> {
        match init {
            ForInit::InitDecl(d) => self.typecheck_variable_declaration(d),
            ForInit::InitExp(Some(e)) => self.typecheck_expression(e),
            ForInit::InitExp(None) => Ok(()),
        }
    }
}

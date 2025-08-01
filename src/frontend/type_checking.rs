use std::collections::HashMap;

use crate::frontend::c_ast::{
    Block, BlockItem, Declaration, Expression, ForInit, FunDecl, Program, Statement, StorageClass,
    VarDecl,
};

#[derive(Debug, Clone, PartialEq)]
pub enum InitValue {
    Tentative,    // 暂定定义，如 `int a;`
    Initial(i64), // 带有初始值，如 `int a = 5;`
    NoInitalizer, // 无初始值，如 `extern int a;`
}

#[derive(Debug, Clone, PartialEq)]
pub enum IdentifierAttrs {
    // 函数属性：是否已定义，是否全局可见
    FunAttr { defined: bool, global: bool },
    // 静态存储期变量属性：初始值，是否全局可见
    StaticAttr { init_value: InitValue, global: bool },
    // 自动存储期变量（局部变量）
    LocalAttr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolInfo {
    pub tpye: CType,
    pub identifier_attrs: IdentifierAttrs,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CType {
    Int,
    FunType { param_count: usize },
}

#[derive(Debug)]
pub struct TypeChecker {
    /// 全局符号表：函数和文件作用域变量
    symbol_tables: HashMap<String, SymbolInfo>,
    /// 局部作用域栈：用于块作用域变量和参数
    scopes: Vec<HashMap<String, SymbolInfo>>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            symbol_tables: HashMap::new(),
            scopes: Vec::new(),
        }
    }

    pub fn typecheck_program(
        mut self,
        ast: &Program,
    ) -> Result<HashMap<String, SymbolInfo>, String> {
        self.push_scope(); // 全局作用域

        for decl in &ast.declarations {
            self.typecheck_declaration(decl, true)?; // true 表示文件作用域
        }

        self.pop_scope();
        Ok(self.symbol_tables)
    }

    // --- 声明检查 ---

    fn typecheck_declaration(&mut self, d: &Declaration, is_file_scope: bool) -> Result<(), String> {
        match d {
            Declaration::Fun(f) => {
                // 函数定义（带函数体）只允许在文件作用域。
                if !is_file_scope && f.body.is_some() {
                    return Err("函数定义不允许在块作用域内。".to_string());
                }
                // 函数声明（无论在文件还是块作用域）都针对全局符号表进行检查。
                self.typecheck_function_declaration(f)
            }
            Declaration::Variable(v) => {
                if is_file_scope {
                    self.typecheck_file_scope_variable_declaration(v)
                } else {
                    self.typecheck_block_scope_variable_declaration(v)
                }
            }
        }
    }

    fn typecheck_function_declaration(&mut self, decl: &FunDecl) -> Result<(), String> {
        let fun_type = CType::FunType {
            param_count: decl.parameters.len(),
        };
        let has_body = decl.body.is_some();
        let mut already_defined = false;

        // 默认是全局可见的，除非显式声明为 static
        let mut global = !matches!(decl.storage_class, Some(StorageClass::Static));

        if let Some(old_decl_info) = self.symbol_tables.get(&decl.name).cloned() {
            if old_decl_info.tpye != fun_type {
                return Err(format!("函数 '{}' 的声明不兼容", decl.name));
            }

            if let IdentifierAttrs::FunAttr {
                defined,
                global: old_global,
            } = old_decl_info.identifier_attrs
            {
                already_defined = defined;
                if already_defined && has_body {
                    return Err(format!("函数 '{}' 被多次定义", decl.name));
                }

                if old_global && matches!(decl.storage_class, Some(StorageClass::Static)) {
                    return Err("静态函数声明跟在非静态函数声明之后".to_string());
                }

                // 链接性保持不变
                global = old_global;
            } else {
                return Err(format!("'{}' 被重新声明为不同类型的符号", decl.name));
            }
        }

        let attrs = IdentifierAttrs::FunAttr {
            defined: already_defined || has_body,
            global,
        };
        self.symbol_tables.insert(
            decl.name.clone(),
            SymbolInfo {
                tpye: fun_type.clone(),
                identifier_attrs: attrs,
            },
        );

        if let Some(body_block) = &decl.body {
            self.push_scope();

            for p_name in &decl.parameters {
                self.insert_variable(
                    p_name.clone(),
                    SymbolInfo {
                        tpye: CType::Int,
                        identifier_attrs: IdentifierAttrs::LocalAttr,
                    },
                )?;
            }
            self.typecheck_block_body(body_block)?;

            self.pop_scope();
        }
        Ok(())
    }

    fn typecheck_file_scope_variable_declaration(&mut self, decl: &VarDecl) -> Result<(), String> {
        let mut initial_value = if let Some(init_expr) = &decl.init {
            let const_val = self.eval_const_expr(init_expr)?;
            InitValue::Initial(const_val)
        } else {
            if matches!(decl.storage_class, Some(StorageClass::Extern)) {
                InitValue::NoInitalizer
            } else {
                InitValue::Tentative
            }
        };

        let mut global = !matches!(decl.storage_class, Some(StorageClass::Static));

        if let Some(old_decl_info) = self.symbol_tables.get(&decl.name).cloned() {
            if old_decl_info.tpye != CType::Int {
                return Err(format!("函数 '{}' 被重新声明为变量", decl.name));
            }

            if let IdentifierAttrs::StaticAttr {
                init_value: old_init,
                global: old_global,
            } = old_decl_info.identifier_attrs
            {
                if matches!(decl.storage_class, Some(StorageClass::Extern)) {
                    global = old_global;
                } else if old_global != global {
                    return Err("变量链接冲突".to_string());
                }

                initial_value = match (old_init, initial_value) {
                    (InitValue::Initial(_), InitValue::Initial(_)) => {
                        return Err("文件作用域变量定义冲突".to_string());
                    }
                    (init @ InitValue::Initial(_), _) => init,
                    (_, init @ InitValue::Initial(_)) => init,
                    (InitValue::Tentative, _) | (_, InitValue::Tentative) => InitValue::Tentative,
                    (InitValue::NoInitalizer, InitValue::NoInitalizer) => InitValue::NoInitalizer,
                };
            } else {
                return Err(format!("'{}' 被重新声明为不同类型的符号", decl.name));
            }
        }

        let attrs = IdentifierAttrs::StaticAttr {
            init_value: initial_value,
            global,
        };
        self.symbol_tables.insert(
            decl.name.clone(),
            SymbolInfo {
                tpye: CType::Int,
                identifier_attrs: attrs,
            },
        );

        Ok(())
    }

    fn typecheck_block_scope_variable_declaration(&mut self, decl: &VarDecl) -> Result<(), String> {
        match &decl.storage_class {
            Some(StorageClass::Extern) => {
                if decl.init.is_some() {
                    return Err("局部 extern 变量声明带有初始值".to_string());
                }

                if let Some(old_decl_info) = self.find_identifier(&decl.name) {
                    if old_decl_info.tpye != CType::Int {
                        return Err(format!("函数 '{}' 被重新声明为变量", decl.name));
                    }
                } else {
                    let attrs = IdentifierAttrs::StaticAttr {
                        init_value: InitValue::NoInitalizer,
                        global: true,
                    };
                    self.symbol_tables.insert(
                        decl.name.clone(),
                        SymbolInfo {
                            tpye: CType::Int,
                            identifier_attrs: attrs,
                        },
                    );
                }
                Ok(())
            }
            Some(StorageClass::Static) => {
                let initial_value = if let Some(init_expr) = &decl.init {
                    let const_val = self
                        .eval_const_expr(init_expr)
                        .map_err(|_| "局部静态变量的初始值不是常量".to_string())?;
                    InitValue::Initial(const_val)
                } else {
                    InitValue::Initial(0)
                };

                let attrs = IdentifierAttrs::StaticAttr {
                    init_value: initial_value,
                    global: false,
                };
                self.insert_variable(
                    decl.name.clone(),
                    SymbolInfo {
                        tpye: CType::Int,
                        identifier_attrs: attrs,
                    },
                )
            }
            None => {
                // 自动变量
                let attrs = IdentifierAttrs::LocalAttr;
                self.insert_variable(
                    decl.name.clone(),
                    SymbolInfo {
                        tpye: CType::Int,
                        identifier_attrs: attrs,
                    },
                )?;
                if let Some(e) = &decl.init {
                    self.typecheck_expression(e)?;
                }
                Ok(())
            }
        }
    }

    // --- 语句和表达式检查 ---

    fn typecheck_block_body(&mut self, block: &Block) -> Result<(), String> {
        for item in &block.0 {
            self.typecheck_block_item(item)?;
        }
        Ok(())
    }

    fn typecheck_block_item(&mut self, item: &BlockItem) -> Result<(), String> {
        match item {
            BlockItem::D(d) => self.typecheck_declaration(d, false), // false 表示块作用域
            BlockItem::S(s) => self.typecheck_statement(s),
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
            _ => Ok(()), // while, dowhile, break, continue, null 等语句
        }
    }

    fn resolve_for_init(&mut self, init: &ForInit) -> Result<(), String> {
        match init {
            ForInit::InitDecl(d) => {
                if d.storage_class.is_some() {
                    return Err("for 循环初始值设定项中不允许使用存储类说明符".to_string());
                }
                self.typecheck_block_scope_variable_declaration(d)
            }
            ForInit::InitExp(Some(e)) => self.typecheck_expression(e),
            ForInit::InitExp(None) => Ok(()),
        }
    }

    fn typecheck_expression(&mut self, e: &Expression) -> Result<(), String> {
        match e {
            Expression::Var(id) => match self.find_identifier(id) {
                Some(info) => {
                    if info.tpye != CType::Int {
                        Err(format!("语义错误：函数 '{}' 被用作变量。", id))
                    } else {
                        Ok(())
                    }
                }
                None => Err(format!("语义错误：使用了未声明的标识符 '{}'。", id)),
            },
            Expression::FuncCall { name, args } => match self.find_identifier(name) {
                Some(info) => match info.tpye {
                    CType::Int => Err(format!("语义错误：变量 '{}' 被用作函数。", name)),
                    CType::FunType { param_count } => {
                        if param_count != args.len() {
                            Err(format!(
                                "语义错误：函数 '{}' 调用时参数数量错误。预期 {} 个，实际 {} 个。",
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
                None => Err(format!("语义错误：调用了未声明的函数 '{}'。", name)),
            },
            Expression::Assignment { left, right } => {
                self.typecheck_expression(left)?;
                self.typecheck_expression(right)?;
                Ok(())
            }
            Expression::Binary { left, right, .. } => {
                self.typecheck_expression(left)?;
                self.typecheck_expression(right)?;
                Ok(())
            }
            Expression::Unary { exp, .. } => {
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

    // --- 辅助函数 ---

    fn eval_const_expr(&self, expr: &Expression) -> Result<i64, String> {
        match expr {
            Expression::Constant(i) => Ok(*i),
            _ => Err("初始值不是常量表达式！".to_string()),
        }
    }

    fn find_identifier(&self, name: &str) -> Option<SymbolInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info.clone());
            }
        }
        self.symbol_tables.get(name).cloned()
    }

    fn insert_variable(&mut self, name: String, info: SymbolInfo) -> Result<(), String> {
        let current_scope = self
            .scopes
            .last_mut()
            .expect("没有作用域时无法插入变量。这是一个编译器错误。");

        if current_scope.contains_key(&name) {
            Err(format!("语义错误：在同一作用域中重定义了变量 '{}'。", name))
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
}
